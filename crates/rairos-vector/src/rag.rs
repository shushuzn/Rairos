//! RAG (Retrieval-Augmented Generation) Pipeline
//!
//! Combines vector retrieval with LLM generation for question answering.

use std::sync::Arc;
use async_trait::async_trait;
use crate::client::{SearchHit, VectorStore};
use crate::embedding::Embedder;
use crate::error::VectorError;
use regex::Regex;

/// RAG configuration
#[derive(Debug, Clone)]
pub struct RagConfig {
    /// Number of documents to retrieve
    pub top_k: usize,
    /// Minimum similarity score threshold
    pub min_score: f32,
    /// Whether to include raw retrieved content in context
    pub include_raw_content: bool,
    /// System prompt for the LLM
    pub system_prompt: String,
    /// User prompt template with {context} and {question} placeholders
    pub user_prompt_template: String,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            top_k: 5,
            min_score: 0.0,
            include_raw_content: true,
            system_prompt: "You are a helpful AI assistant specialized in materials science. \
Use the provided context to answer the user's question. If the context doesn't \
contain enough information to answer, say so."
                .to_string(),
            user_prompt_template: "Context:\n{context}\n\nQuestion: {question}\n\nAnswer:".to_string(),
        }
    }
}

/// RAG Pipeline combining retrieval and generation
pub struct RagPipeline<E: Embedder, V: VectorStore, L: LlmClient> {
    embedder: Arc<E>,
    store: Arc<V>,
    llm: Arc<L>,
    config: RagConfig,
}

/// LLM client trait for RAG generation
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn generate(&self, prompt: &str) -> Result<String, VectorError>;
    fn name(&self) -> &'static str;
}

/// Simple RAG answer with sources
#[derive(Debug)]
pub struct RagAnswer {
    pub answer: String,
    pub sources: Vec<RagSource>,
    pub retrieved_docs: Vec<SearchHit>,
}

#[derive(Debug, Clone)]
pub struct RagSource {
    pub id: String,
    pub content: String,
    pub score: f32,
}

// ============================================================================
// Inline Citation RAG (Citation-aware generation)
// ============================================================================

/// Configuration for inline citation RAG.
/// Based on research showing that post-hoc citation has only 65-70% accuracy,
/// while inline citation during generation achieves >95% accuracy.
#[derive(Debug, Clone)]
pub struct InlineCitationConfig {
    /// Whether to require inline citations in generated answers
    pub require_citations: bool,
    /// Citation format template
    pub citation_format: String,
    /// Minimum claims before requiring citation
    pub min_claims_before_citation: usize,
    /// Source ID prefix for citations
    pub source_prefix: String,
}

impl Default for InlineCitationConfig {
    fn default() -> Self {
        Self {
            require_citations: true,
            citation_format: "[{id}]".to_string(),
            min_claims_before_citation: 2,
            source_prefix: "Source".to_string(),
        }
    }
}

/// A cited segment with source attribution.
#[derive(Debug, Clone)]
pub struct CitedSegment {
    /// The text content with inline citation markers
    pub text: String,
    /// Source IDs used in this segment
    pub sources: Vec<String>,
    /// Position in original answer
    pub start_char: usize,
    pub end_char: usize,
}

/// An answer with verified inline citations.
#[derive(Debug)]
pub struct CitationAnswer {
    /// The answer with inline citations
    pub answer: String,
    /// All sources used
    pub sources: Vec<RagSource>,
    /// Claim-level citations for verification
    pub claims: Vec<CitedClaim>,
    /// Unverified claims (potential hallucinations)
    pub unverified_claims: Vec<String>,
}

/// A single claim with its supporting source.
#[derive(Debug, Clone)]
pub struct CitedClaim {
    pub claim: String,
    pub source_id: Option<String>,
    pub verified: bool,
}

/// Inline Citation RAG pipeline that enforces source attribution during generation.
pub struct InlineCitationRag<E: Embedder, V: VectorStore, L: LlmClient> {
    base: RagPipeline<E, V, L>,
    config: InlineCitationConfig,
}

impl<E: Embedder, V: VectorStore, L: LlmClient> InlineCitationRag<E, V, L> {
    /// Create a new inline citation RAG pipeline.
    pub fn new(base: RagPipeline<E, V, L>) -> Self {
        Self::with_config(base, InlineCitationConfig::default())
    }

    /// Create with custom configuration.
    pub fn with_config(base: RagPipeline<E, V, L>, config: InlineCitationConfig) -> Self {
        Self { base, config }
    }

    /// Query with inline citations enforced.
    pub async fn query_with_citations(&self, question: &str) -> Result<CitationAnswer, VectorError> {
        // 1. Get base RAG answer
        let base_answer = self.base.query(question).await?;

        if base_answer.sources.is_empty() {
            return Ok(CitationAnswer {
                answer: base_answer.answer,
                sources: vec![],
                claims: vec![],
                unverified_claims: vec![],
            });
        }

        // 2. Build source-aware prompt that requires citations
        let source_list = base_answer
            .sources
            .iter()
            .enumerate()
            .map(|(i, s)| format!("{}[{}]: {}", self.config.source_prefix, i + 1, s.content))
            .collect::<Vec<_>>()
            .join("\n\n");

        let citation_prompt = format!(
            "You are a research assistant that ALWAYS cites sources for factual claims.\n\n\
            Available sources:\n{}\n\n\
            Question: {}\n\n\
            Instructions:\n\
            1. Answer the question based ONLY on the information in the sources above.\n\
            2. For EVERY factual statement, include a citation using [N] format where N is the source number.\n\
            3. If you cannot answer from the sources, say 'I cannot answer this from the provided sources.'\n\
            4. Do not make up information not in the sources.\n\n\
            Answer with inline citations:",
            source_list, question
        );

        // 3. Generate answer with citation requirement
        let cited_answer = self.base.llm.generate(&citation_prompt).await?;

        // 4. Parse claims and verify citations
        let claims = self.parse_claims_with_citations(&cited_answer, &base_answer.sources);

        // 5. Identify unverified claims
        let unverified_claims: Vec<String> = claims
            .iter()
            .filter(|c| !c.verified)
            .map(|c| c.claim.clone())
            .collect();

        // 6. Build source map for output
        let source_map: std::collections::HashMap<String, &RagSource> = base_answer
            .sources
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let key = format!("[{}]", i + 1);
                (key, s)
            })
            .collect();

        // 7. Rewrite answer with proper source IDs
        let rewritten_answer = self.rewrite_with_source_ids(&cited_answer, &source_map);

        Ok(CitationAnswer {
            answer: rewritten_answer,
            sources: base_answer.sources,
            claims,
            unverified_claims,
        })
    }

    /// Parse claims from generated text and match to sources.
    fn parse_claims_with_citations(
        &self,
        text: &str,
        sources: &[RagSource],
    ) -> Vec<CitedClaim> {
        let mut claims = Vec::new();
        let citation_pattern = regex::Regex::new(r"\[(\d+)\]").unwrap();

        // Split by sentences (simple approach)
        let sentences: Vec<&str> = text
            .split(|c| c == '.' || c == '!' || c == '?')
            .filter(|s| !s.trim().is_empty())
            .collect();

        for sentence in sentences {
            let sentence = sentence.trim();
            if sentence.len() < 10 {
                continue;
            }

            let caps = citation_pattern.captures(sentence);
            let source_id_opt = caps.map(|c| {
                let idx: usize = c.get(1).unwrap().as_str().parse().unwrap_or(0);
                if idx > 0 && idx <= sources.len() {
                    Some(sources[idx - 1].id.clone())
                } else {
                    None
                }
            }).flatten();

            let claim_text = citation_pattern.replace_all(sentence, "").trim().to_string();
            let verified = source_id_opt.is_some();

            claims.push(CitedClaim {
                claim: claim_text,
                source_id: source_id_opt,
                verified,
            });
        }

        claims
    }

    /// Rewrite citations to use actual source IDs instead of numbers.
    fn rewrite_with_source_ids(
        &self,
        text: &str,
        source_map: &std::collections::HashMap<String, &RagSource>,
    ) -> String {
        let citation_pattern = regex::Regex::new(r"\[(\d+)\]").unwrap();

        citation_pattern.replace_all(text, |caps: &regex::Captures| {
            let idx: usize = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
            if idx > 0 && idx <= source_map.len() {
                let key = format!("[{}]", idx);
                if let Some(source) = source_map.get(&key) {
                    return self.config.citation_format.replace("{id}", &source.id);
                }
            }
            caps.get(0).unwrap().as_str().to_string()
        }).to_string()
    }

    /// Check if a claim is supported by any source (NLI-based verification placeholder).
    pub fn verify_claim_support(&self, _claim: &str, _source: &str) -> bool {
        // In a full implementation, this would use NLI or LLM-as-judge
        // For now, return true if source is provided
        true
    }
}

impl<E: Embedder, V: VectorStore, L: LlmClient> RagPipeline<E, V, L> {
    pub fn new(embedder: Arc<E>, store: Arc<V>, llm: Arc<L>) -> Self {
        Self::with_config(embedder, store, llm, RagConfig::default())
    }

    pub fn with_config(embedder: Arc<E>, store: Arc<V>, llm: Arc<L>, config: RagConfig) -> Self {
        Self {
            embedder,
            store,
            llm,
            config,
        }
    }

    /// Query the RAG pipeline with a question
    pub async fn query(&self, question: &str) -> Result<RagAnswer, VectorError> {
        // 1. Embed the question
        let query_embedding = self
            .embedder
            .embed(vec![question.to_string()])
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| VectorError::EmbeddingFailed("No embedding returned".to_string()))?;

        // 2. Retrieve similar documents
        let hits = self
            .store
            .search(&query_embedding, self.config.top_k, None)
            .await?;

        // 3. Filter by minimum score
        let filtered_hits: Vec<SearchHit> = hits
            .into_iter()
            .filter(|h| h.score >= self.config.min_score)
            .collect();

        if filtered_hits.is_empty() {
            return Ok(RagAnswer {
                answer: "I couldn't find any relevant documents to answer your question.".to_string(),
                sources: vec![],
                retrieved_docs: vec![],
            });
        }

        // 4. Build context from retrieved documents
        let sources: Vec<RagSource> = filtered_hits
            .iter()
            .map(|hit| {
                let content = hit
                    .payload
                    .as_ref()
                    .and_then(|p| {
                        p.get("text")
                            .or_else(|| p.get("content"))
                            .or_else(|| p.get("document"))
                    })
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                RagSource {
                    id: hit.id.clone(),
                    content: content.clone(),
                    score: hit.score,
                }
            })
            .collect();

        let context = sources
            .iter()
            .map(|s| format!("[{}] (score: {:.3})\n{}", s.id, s.score, s.content))
            .collect::<Vec<_>>()
            .join("\n\n");

        // 5. Build prompts
        let user_prompt = self
            .config
            .user_prompt_template
            .replace("{context}", &context)
            .replace("{question}", question);

        let full_prompt = format!("{}\n\n{}", self.config.system_prompt, user_prompt);

        // 6. Generate answer
        let answer = self.llm.generate(&full_prompt).await?;

        Ok(RagAnswer {
            answer,
            sources,
            retrieved_docs: filtered_hits,
        })
    }

    /// Add a document to the RAG pipeline
    pub async fn add_document(
        &self,
        id: &str,
        text: &str,
        metadata: Option<serde_json::Value>,
    ) -> Result<(), VectorError> {
        let embedding = self
            .embedder
            .embed(vec![text.to_string()])
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| VectorError::EmbeddingFailed("No embedding returned".to_string()))?;

        let mut payload = metadata.unwrap_or(serde_json::Value::Null);
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("text".to_string(), serde_json::json!(text));
        } else {
            payload = serde_json::json!({ "text": text });
        }

        self.store
            .upsert_one(id, &embedding, Some(payload))
            .await?;

        Ok(())
    }

    /// Add multiple documents to the RAG pipeline
    pub async fn add_documents(
        &self,
        docs: Vec<(String, String, Option<serde_json::Value>)>,
    ) -> Result<(), VectorError> {
        if docs.is_empty() {
            return Ok(());
        }

        // Embed all texts
        let texts: Vec<String> = docs.iter().map(|(_, t, _)| t.clone()).collect();
        let embeddings = self.embedder.embed(texts).await?;

        // Build payloads
        let vectors: Vec<(String, Vec<f32>, Option<serde_json::Value>)> = docs
            .into_iter()
            .zip(embeddings)
            .map(|((id, text, metadata), embedding)| {
                let payload = match metadata {
                    Some(mut v) => {
                        if let Some(obj) = v.as_object_mut() {
                            obj.insert("text".to_string(), serde_json::json!(text));
                            Some(v)
                        } else {
                            Some(serde_json::json!({ "text": text }))
                        }
                    }
                    None => Some(serde_json::json!({ "text": text })),
                };
                (id, embedding, payload)
            })
            .collect();

        self.store.upsert_batch(vectors).await?;

        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::VectorStore;
    use crate::embedding::Embedder;
    use crate::EmbeddingModel;
    use std::collections::HashMap;
    use std::sync::RwLock;

    /// Mock LLM client for testing
    struct MockLlmClient {
        response: String,
    }

    #[async_trait]
    impl LlmClient for MockLlmClient {
        async fn generate(&self, _prompt: &str) -> Result<String, VectorError> {
            Ok(self.response.clone())
        }

        fn name(&self) -> &'static str {
            "mock-llm"
        }
    }

    /// Mock embedder for testing
    struct MockEmbedder {
        embeddings: Vec<Vec<f32>>,
        dimensions: usize,
        model: EmbeddingModel,
    }

    impl MockEmbedder {
        fn new(embeddings: Vec<Vec<f32>>) -> Self {
            let dimensions = embeddings.first().map(|e| e.len()).unwrap_or(0);
            Self {
                embeddings,
                dimensions,
                model: EmbeddingModel::Custom("mock".to_string()),
            }
        }
    }

    #[async_trait]
    impl Embedder for MockEmbedder {
        async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, VectorError> {
            let mut result = Vec::with_capacity(texts.len());
            for (i, _) in texts.iter().enumerate() {
                let idx = if self.embeddings.is_empty() {
                    0
                } else {
                    i % self.embeddings.len()
                };
                result.push(self.embeddings[idx].clone());
            }
            Ok(result)
        }

        fn dimensions(&self) -> usize {
            self.dimensions
        }

        fn model(&self) -> &EmbeddingModel {
            &self.model
        }
    }

    /// Mock vector store for testing
    struct MockVectorStore {
        vectors: RwLock<HashMap<String, (Vec<f32>, Option<serde_json::Value>)>>,
        dimensions: RwLock<usize>,
    }

    impl MockVectorStore {
        fn new() -> Self {
            Self {
                vectors: RwLock::new(HashMap::new()),
                dimensions: RwLock::new(4), // Default dimensions for tests
            }
        }

        fn with_dimensions(dimensions: usize) -> Self {
            Self {
                vectors: RwLock::new(HashMap::new()),
                dimensions: RwLock::new(dimensions),
            }
        }
    }

    impl Default for MockVectorStore {
        fn default() -> Self {
            Self::new()
        }
    }

    #[async_trait]
    impl VectorStore for MockVectorStore {
        async fn initialize(&self, config: &crate::client::VectorStoreConfig) -> Result<(), VectorError> {
            *self.dimensions.write().unwrap() = config.dimensions;
            Ok(())
        }

        async fn upsert_one(
            &self,
            id: &str,
            embedding: &[f32],
            payload: Option<serde_json::Value>,
        ) -> Result<(), VectorError> {
            self.vectors.write().unwrap().insert(
                id.to_string(),
                (embedding.to_vec(), payload),
            );
            Ok(())
        }

        async fn upsert_batch(
            &self,
            vectors: Vec<(String, Vec<f32>, Option<serde_json::Value>)>,
        ) -> Result<(), VectorError> {
            for (id, embedding, payload) in vectors {
                self.vectors.write().unwrap().insert(id, (embedding, payload));
            }
            Ok(())
        }

        async fn search(
            &self,
            query: &[f32],
            top_k: usize,
            _filter: Option<&str>,
        ) -> Result<Vec<SearchHit>, VectorError> {
            let vectors = self.vectors.read().unwrap();
            let mut results: Vec<SearchHit> = vectors
                .iter()
                .map(|(id, (embedding, payload))| {
                    let dot: f32 = query.iter().zip(embedding.iter()).map(|(x, y)| x * y).sum();
                    SearchHit {
                        id: id.clone(),
                        score: dot,
                        payload: payload.clone(),
                        embedding: None,
                    }
                })
                .collect();
            results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
            results.truncate(top_k);
            Ok(results)
        }

        async fn delete(&self, id: &str) -> Result<(), VectorError> {
            self.vectors.write().unwrap().remove(id);
            Ok(())
        }

        async fn get(&self, id: &str) -> Result<Option<SearchHit>, VectorError> {
            let vectors = self.vectors.read().unwrap();
            Ok(vectors.get(id).map(|(embedding, payload)| SearchHit {
                id: id.to_string(),
                score: 1.0,
                payload: payload.clone(),
                embedding: Some(embedding.clone()),
            }))
        }

        async fn count(&self) -> Result<usize, VectorError> {
            Ok(self.vectors.read().unwrap().len())
        }

        async fn exists(&self, id: &str) -> Result<bool, VectorError> {
            Ok(self.vectors.read().unwrap().contains_key(id))
        }

        async fn drop_collection(&self) -> Result<(), VectorError> {
            self.vectors.write().unwrap().clear();
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_rag_config_default() {
        let config = RagConfig::default();
        assert_eq!(config.top_k, 5);
        assert_eq!(config.min_score, 0.0);
        assert!(config.include_raw_content);
        assert!(!config.system_prompt.is_empty());
        assert!(config.user_prompt_template.contains("{context}"));
        assert!(config.user_prompt_template.contains("{question}"));
    }

    #[tokio::test]
    async fn test_rag_pipeline_new() {
        let embedder = Arc::new(MockEmbedder::new(vec![vec![0.1, 0.2, 0.3, 0.4]]));
        let store = Arc::new(MockVectorStore::with_dimensions(4));
        let llm = Arc::new(MockLlmClient { response: "Test response".to_string() });

        let pipeline = RagPipeline::new(embedder, store, llm);
        assert_eq!(pipeline.config.top_k, 5);
    }

    #[tokio::test]
    async fn test_rag_pipeline_with_config() {
        let embedder = Arc::new(MockEmbedder::new(vec![vec![0.1, 0.2, 0.3, 0.4]]));
        let store = Arc::new(MockVectorStore::with_dimensions(4));
        let llm = Arc::new(MockLlmClient { response: "Test".to_string() });

        let config = RagConfig {
            top_k: 10,
            min_score: 0.5,
            ..Default::default()
        };
        let pipeline = RagPipeline::with_config(embedder, store, llm, config);
        assert_eq!(pipeline.config.top_k, 10);
        assert_eq!(pipeline.config.min_score, 0.5);
    }

    #[tokio::test]
    async fn test_rag_add_document() {
        let embedder = Arc::new(MockEmbedder::new(vec![vec![0.1, 0.2, 0.3, 0.4]]));
        let store = Arc::new(MockVectorStore::with_dimensions(4));
        let llm = Arc::new(MockLlmClient { response: "Test".to_string() });

        let pipeline = RagPipeline::new(embedder.clone(), store.clone(), llm);

        pipeline
            .add_document("doc1", "Hello world", Some(serde_json::json!({"source": "test"})))
            .await
            .unwrap();

        let count = store.count().await.unwrap();
        assert_eq!(count, 1);

        let exists = store.exists("doc1").await.unwrap();
        assert!(exists);
    }

    #[tokio::test]
    async fn test_rag_add_documents() {
        let embedder = Arc::new(MockEmbedder::new(vec![
            vec![0.1, 0.2, 0.3, 0.4],
            vec![0.5, 0.6, 0.7, 0.8],
        ]));
        let store = Arc::new(MockVectorStore::with_dimensions(4));
        let llm = Arc::new(MockLlmClient { response: "Test".to_string() });

        let pipeline = RagPipeline::new(embedder.clone(), store.clone(), llm);

        let docs = vec![
            ("doc1".to_string(), "First document".to_string(), None),
            ("doc2".to_string(), "Second document".to_string(), None),
        ];
        pipeline.add_documents(docs).await.unwrap();

        let count = store.count().await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_rag_add_documents_empty() {
        let embedder = Arc::new(MockEmbedder::new(vec![vec![0.1, 0.2, 0.3, 0.4]]));
        let store = Arc::new(MockVectorStore::with_dimensions(4));
        let llm = Arc::new(MockLlmClient { response: "Test".to_string() });

        let pipeline = RagPipeline::new(embedder.clone(), store.clone(), llm);

        // Empty documents should not error
        pipeline.add_documents(vec![]).await.unwrap();

        let count = store.count().await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_rag_query_basic() {
        let embedder = Arc::new(MockEmbedder::new(vec![vec![0.1, 0.2, 0.3, 0.4]]));
        let store = Arc::new(MockVectorStore::with_dimensions(4));
        let llm = Arc::new(MockLlmClient {
            response: "This is the generated answer.".to_string(),
        });

        let pipeline = RagPipeline::new(embedder, store, llm);

        // Add a document first
        pipeline
            .add_document("doc1", "Test document content", None)
            .await
            .unwrap();

        // Query
        let answer = pipeline.query("What is the content?").await.unwrap();
        assert_eq!(answer.answer, "This is the generated answer.");
        assert!(!answer.sources.is_empty());
    }

    #[tokio::test]
    async fn test_rag_query_no_results() {
        let embedder = Arc::new(MockEmbedder::new(vec![vec![0.1, 0.2, 0.3, 0.4]]));
        let store = Arc::new(MockVectorStore::with_dimensions(4));
        let llm = Arc::new(MockLlmClient { response: "Test".to_string() });

        let pipeline = RagPipeline::new(embedder, store, llm);

        // Query without adding any documents
        let answer = pipeline.query("What is the content?").await.unwrap();
        assert!(answer.answer.contains("couldn't find"));
        assert!(answer.sources.is_empty());
        assert!(answer.retrieved_docs.is_empty());
    }

    #[tokio::test]
    async fn test_rag_query_filters_by_min_score() {
        let embedder = Arc::new(MockEmbedder::new(vec![vec![0.1, 0.2, 0.3, 0.4]]));
        let store = Arc::new(MockVectorStore::with_dimensions(4));
        let llm = Arc::new(MockLlmClient { response: "Test".to_string() });

        let config = RagConfig {
            min_score: 10.0, // High threshold
            ..Default::default()
        };
        let pipeline = RagPipeline::with_config(embedder, store, llm, config);

        // Add a document
        pipeline
            .add_document("doc1", "Test", None)
            .await
            .unwrap();

        // Query with high min_score - should return no results
        let answer = pipeline.query("Test").await.unwrap();
        assert!(answer.answer.contains("couldn't find"));
    }

    #[tokio::test]
    async fn test_rag_answer_sources() {
        let embedder = Arc::new(MockEmbedder::new(vec![vec![1.0, 0.0, 0.0, 0.0]]));
        let store = Arc::new(MockVectorStore::with_dimensions(4));
        let llm = Arc::new(MockLlmClient { response: "Answer".to_string() });

        let pipeline = RagPipeline::new(embedder, store, llm);

        pipeline
            .add_document(
                "doc1",
                "The answer is 42",
                Some(serde_json::json!({"source": "manual"})),
            )
            .await
            .unwrap();

        let answer = pipeline.query("What is the answer?").await.unwrap();

        assert!(!answer.sources.is_empty());
        let source = &answer.sources[0];
        assert_eq!(source.id, "doc1");
        assert!(!source.content.is_empty());
    }

    #[tokio::test]
    async fn test_rag_source_debug() {
        let source = RagSource {
            id: "test".to_string(),
            content: "content".to_string(),
            score: 0.95,
        };
        let debug = format!("{:?}", source);
        assert!(debug.contains("test"));
        assert!(debug.contains("0.95"));
    }

    #[tokio::test]
    async fn test_rag_answer_debug() {
        let answer = RagAnswer {
            answer: "test answer".to_string(),
            sources: vec![],
            retrieved_docs: vec![],
        };
        let debug = format!("{:?}", answer);
        assert!(debug.contains("test answer"));
    }

    #[tokio::test]
    async fn test_mock_llm_client() {
        let llm = MockLlmClient {
            response: "Generated text".to_string(),
        };
        assert_eq!(llm.name(), "mock-llm");

        let result = llm.generate("test prompt").await.unwrap();
        assert_eq!(result, "Generated text");
    }

    #[tokio::test]
    async fn test_mock_embedder() {
        let embedder = MockEmbedder::new(vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]]);
        assert_eq!(embedder.dimensions(), 3);

        let results = embedder.embed(vec!["a".to_string(), "b".to_string()]).await.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], vec![1.0, 2.0, 3.0]);
        assert_eq!(results[1], vec![4.0, 5.0, 6.0]);
    }
}

/// Implementation of LlmClient that wraps rairos-llm's LlmClient
/// This is optional and only available when the "llm" feature is enabled
#[cfg(feature = "llm")]
pub mod llm_integration {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Arc;

    /// Adapter for rairos-llm LlmClient
    pub struct RairosLlmClient {
        client: Arc<dyn rairos_llm::LlmClient>,
        model: String,
        temperature: f32,
        max_tokens: u32,
    }

    impl RairosLlmClient {
        pub fn new(client: Arc<dyn rairos_llm::LlmClient>) -> Self {
            Self::with_config(client, "gpt-4o".to_string(), 0.7, 2048)
        }

        pub fn with_model(client: Arc<dyn rairos_llm::LlmClient>, model: &str) -> Self {
            Self::with_config(client, model.to_string(), 0.7, 2048)
        }

        pub fn with_config(
            client: Arc<dyn rairos_llm::LlmClient>,
            model: String,
            temperature: f32,
            max_tokens: u32,
        ) -> Self {
            Self {
                client,
                model,
                temperature,
                max_tokens,
            }
        }
    }

    #[async_trait]
    impl LlmClient for RairosLlmClient {
        async fn generate(&self, prompt: &str) -> Result<String, VectorError> {
            use rairos_llm::{LlmResponse, Message};

            let messages = vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }];

            let response = self
                .client
                .complete(messages, &self.model, self.temperature, self.max_tokens)
                .await
                .map_err(|e| VectorError::LlmError(e.to_string()))?;

            match response {
                LlmResponse::NonStream(resp) => Ok(resp.content),
                LlmResponse::Stream(_) => {
                    Err(VectorError::LlmError(
                        "Streaming not supported for RAG".to_string(),
                    ))
                }
            }
        }

        fn name(&self) -> &'static str {
            "rairos-llm"
        }
    }
}
