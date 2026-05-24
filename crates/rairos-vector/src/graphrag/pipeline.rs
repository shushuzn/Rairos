//! GraphRAG pipeline combining retrieval, reasoning, and generation.

use std::sync::Arc;

use crate::embedding::Embedder;
use crate::client::VectorStore;
use rairos_kg_neo4j::Neo4jKgClient;

use crate::graphrag::community::{CommunitySummarizer, CommunitySummary};
use crate::graphrag::error::GraphRagError;
use crate::graphrag::retrieval::{HybridRetriever, HybridSearchResult};
use crate::graphrag::reasoning::{PathFinder, ReasoningPath};

/// Configuration for GraphRAG pipeline
#[derive(Debug, Clone)]
pub struct GraphRagConfig {
    /// Number of documents to retrieve
    pub top_k: usize,
    /// Vector similarity weight (0.0 to 1.0)
    pub vector_weight: f32,
    /// Graph proximity weight (0.0 to 1.0)
    pub graph_weight: f32,
    /// Minimum similarity score
    pub min_score: f32,
    /// Maximum reasoning hops
    pub max_hops: usize,
    /// Enable community summarization
    pub enable_communities: bool,
    /// System prompt for LLM
    pub system_prompt: String,
    /// User prompt template
    pub user_prompt_template: String,
}

impl Default for GraphRagConfig {
    fn default() -> Self {
        Self {
            top_k: 10,
            vector_weight: 0.5,
            graph_weight: 0.5,
            min_score: 0.0,
            max_hops: 3,
            enable_communities: true,
            system_prompt: "You are a helpful AI assistant specialized in materials science and machine learning. \
Use the provided context to answer the user's question. If the context doesn't contain \
enough information to answer, say so. Cite your sources when possible."
                .to_string(),
            user_prompt_template: "Context from knowledge graph:\n{context}\n\nSources:\n{sources}\n\nQuestion: {question}\n\nAnswer:".to_string(),
        }
    }
}

/// Answer returned by GraphRAG pipeline
#[derive(Debug)]
pub struct GraphRagAnswer {
    /// The generated answer
    pub answer: String,
    /// Source entities used
    pub sources: Vec<Source>,
    /// Communities used in reasoning
    pub communities: Vec<CommunitySummary>,
    /// Reasoning paths used
    pub paths: Vec<ReasoningPath>,
    /// Retrieved entities
    pub retrieved_entities: Vec<HybridSearchResult>,
}

/// A source used in the answer
#[derive(Debug, Clone)]
pub struct Source {
    pub entity_id: String,
    pub label: String,
    pub source_type: String,
    pub score: f32,
}

/// GraphRAG pipeline
pub struct GraphRagPipeline<E: Embedder, V: VectorStore> {
    embedder: Arc<E>,
    vector_store: Arc<V>,
    kg_client: Arc<Neo4jKgClient>,
    retriever: HybridRetriever,
    community_summarizer: CommunitySummarizer,
    path_finder: PathFinder,
    config: GraphRagConfig,
}

impl<E: Embedder, V: VectorStore> GraphRagPipeline<E, V> {
    /// Create a new GraphRAG pipeline
    pub fn new(
        embedder: Arc<E>,
        vector_store: Arc<V>,
        kg_client: Arc<Neo4jKgClient>,
        config: GraphRagConfig,
    ) -> Self {
        let retriever = HybridRetriever::with_config(&config);
        let community_summarizer = CommunitySummarizer::new();
        let path_finder = PathFinder::with_config(config.max_hops);

        Self {
            embedder,
            vector_store,
            kg_client,
            retriever,
            community_summarizer,
            path_finder,
            config,
        }
    }

    /// Query the GraphRAG pipeline
    pub async fn query(&self, question: &str) -> Result<GraphRagAnswer, GraphRagError> {
        // 1. Embed the question
        let query_embedding = self
            .embedder
            .embed(vec![question.to_string()])
            .await
            .map_err(|e| GraphRagError::EmbeddingFailed(e.to_string()))?
            .into_iter()
            .next()
            .ok_or_else(|| GraphRagError::EmbeddingFailed("No embedding returned".to_string()))?;

        // 2. Vector similarity search
        let vector_results = self
            .vector_store
            .search(&query_embedding, self.config.top_k, None)
            .await
            .map_err(|e| GraphRagError::VectorStoreError(e.to_string()))?;

        // 3. Convert to hybrid results
        let hybrid_results = self.vector_to_hybrid_results(vector_results);

        // 4. Expand with graph context
        let expanded_results = self.expand_with_graph_context(hybrid_results).await?;

        // 5. Community detection (optional)
        let communities = if self.config.enable_communities {
            self.detect_communities(&expanded_results).await?
        } else {
            vec![]
        };

        // 6. Multi-hop reasoning paths
        let paths = self.find_reasoning_paths(&expanded_results).await?;

        // 7. Build context for LLM
        let context = self.build_context(&expanded_results, &communities, &paths)?;
        let sources = self.extract_sources(&expanded_results);

        // 8. Generate answer (placeholder - requires LLM integration)
        let answer = self.generate_answer(&context, question, &sources).await?;

        Ok(GraphRagAnswer {
            answer,
            sources,
            communities,
            paths,
            retrieved_entities: expanded_results,
        })
    }

    /// Convert vector search results to hybrid results
    fn vector_to_hybrid_results(
        &self,
        hits: Vec<crate::client::SearchHit>,
    ) -> Vec<HybridSearchResult> {
        hits
            .into_iter()
            .map(|hit| HybridSearchResult {
                entity_id: hit.id.clone(),
                score: hit.score,
                vector_score: hit.score,
                graph_score: 0.0,
                node_type: "Paper".to_string(),
                label: hit
                    .payload
                    .as_ref()
                    .and_then(|p| p.get("title").or_else(|| p.get("label")))
                    .and_then(|v| v.as_str())
                    .unwrap_or(&hit.id)
                    .to_string(),
                metadata: hit.payload.unwrap_or(serde_json::json!({})),
                related_entities: vec![],
            })
            .collect()
    }

    /// Expand results with graph context
    async fn expand_with_graph_context(
        &self,
        mut results: Vec<HybridSearchResult>,
    ) -> Result<Vec<HybridSearchResult>, GraphRagError> {
        for result in &mut results {
            // Get related papers from KG
            if let Ok(related) = self
                .kg_client
                .get_related_papers(&result.entity_id, 5)
                .await
            {
                result.related_entities = related
                    .into_iter()
                    .take(5)
                    .map(|node| crate::graphrag::retrieval::RelatedEntity {
                        entity_id: node.entity_id.clone(),
                        relation: "RELATED".to_string(),
                        label: node.label,
                    })
                    .collect();
            }

            // Calculate graph score based on connections
            let connection_score = (result.related_entities.len() as f32 * 0.1).min(0.5);
            result.graph_score = connection_score;
            result.score = self.retriever.combine_scores(result.vector_score, result.graph_score);
        }

        Ok(results)
    }

    /// Detect communities among retrieved entities
    async fn detect_communities(
        &self,
        _results: &[HybridSearchResult],
    ) -> Result<Vec<CommunitySummary>, GraphRagError> {
        let communities = rairos_kg_neo4j::algorithms::detect_communities(
            &self.kg_client,
            "Paper",
        )
        .await
        .map_err(|e| GraphRagError::KgError(e.to_string()))?;

        // Convert to CommunitySummary
        let summaries: Vec<CommunitySummary> = communities
            .into_iter()
            .take(5)
            .map(|c| CommunitySummary {
                community_id: c.community_id.to_string(),
                summary: format!("Community of {} papers", c.community_size),
                keywords: vec![],
                representatives: vec![c.entity_id],
                coverage_score: 0.5,
            })
            .collect();

        Ok(summaries)
    }

    /// Find reasoning paths through the graph
    async fn find_reasoning_paths(
        &self,
        results: &[HybridSearchResult],
    ) -> Result<Vec<ReasoningPath>, GraphRagError> {
        let entity_ids: Vec<String> = results.iter().map(|r| r.entity_id.clone()).collect();

        // Use simplified path finding
        let path_finder = PathFinder::new(self.config.max_hops, 5);
        let paths = path_finder.find_paths(
            entity_ids.first().map(|s| s.as_str()).unwrap_or(""),
            &entity_ids[1..],
        );

        Ok(paths)
    }

    /// Build context string for LLM
    fn build_context(
        &self,
        results: &[HybridSearchResult],
        communities: &[CommunitySummary],
        paths: &[ReasoningPath],
    ) -> Result<String, GraphRagError> {
        let mut context_parts = Vec::new();

        // Add entity context
        for result in results.iter().take(self.config.top_k) {
            let mut part = format!(
                "- {} ({}, score: {:.3})\n  {} related entities",
                result.label,
                result.entity_id,
                result.score,
                result.related_entities.len()
            );

            if !result.related_entities.is_empty() {
                let related_labels: Vec<_> = result
                    .related_entities
                    .iter()
                    .take(3)
                    .map(|e| e.label.as_str())
                    .collect();
                part.push_str(&format!(": {}", related_labels.join(", ")));
            }

            context_parts.push(part);
        }

        // Add community summaries
        if !communities.is_empty() {
            context_parts.push("\n## Research Communities\n".to_string());
            for community in communities {
                context_parts.push(format!(
                    "- {}: {} (keywords: {})",
                    community.community_id,
                    community.summary,
                    community.keywords.join(", ")
                ));
            }
        }

        // Add reasoning paths
        if !paths.is_empty() {
            context_parts.push("\n## Reasoning Paths\n".to_string());
            for (i, path) in paths.iter().take(3).enumerate() {
                let entities: Vec<_> = path
                    .entities
                    .iter()
                    .map(|e| e.label.as_str())
                    .collect();
                context_parts.push(format!(
                    "{}. {} [{} hops, score: {:.3}]",
                    i + 1,
                    entities.join(" -> "),
                    path.hops,
                    path.score
                ));
            }
        }

        Ok(context_parts.join("\n"))
    }

    /// Extract sources from results
    fn extract_sources(&self, results: &[HybridSearchResult]) -> Vec<Source> {
        results
            .iter()
            .map(|r| Source {
                entity_id: r.entity_id.clone(),
                label: r.label.clone(),
                source_type: r.node_type.clone(),
                score: r.score,
            })
            .collect()
    }

    /// Generate answer using LLM (placeholder)
    async fn generate_answer(
        &self,
        _context: &str,
        _question: &str,
        _sources: &[Source],
    ) -> Result<String, GraphRagError> {
        // In a full implementation, this would call the LLM
        // For now, return a placeholder
        Ok("LLM integration required to generate answer. Configure LLM client for full functionality.".to_string())
    }

    /// Add documents to both vector store and knowledge graph
    pub async fn index_document(
        &self,
        entity_id: &str,
        title: &str,
        content: &str,
        metadata: serde_json::Value,
    ) -> Result<(), GraphRagError> {
        // 1. Generate embedding
        let embedding = self
            .embedder
            .embed(vec![content.to_string()])
            .await
            .map_err(|e| GraphRagError::EmbeddingFailed(e.to_string()))?
            .into_iter()
            .next()
            .ok_or_else(|| GraphRagError::EmbeddingFailed("No embedding returned".to_string()))?;

        // 2. Store in vector database
        let mut payload = metadata;
        payload["title"] = serde_json::json!(title);
        payload["content"] = serde_json::json!(content);

        self.vector_store
            .upsert_one(entity_id, &embedding, Some(payload))
            .await
            .map_err(|e| GraphRagError::VectorStoreError(e.to_string()))?;

        // 3. Store in knowledge graph
        self.kg_client
            .upsert_paper(entity_id, title, serde_json::json!({}))
            .await
            .map_err(|e| GraphRagError::KgError(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphrag::retrieval::RelatedEntity;

    #[test]
    fn test_default_config() {
        let config = GraphRagConfig::default();
        assert_eq!(config.top_k, 10);
        assert_eq!(config.vector_weight, 0.5);
        assert_eq!(config.graph_weight, 0.5);
        assert_eq!(config.min_score, 0.0);
        assert_eq!(config.max_hops, 3);
        assert!(config.enable_communities);
        assert!(!config.system_prompt.is_empty());
        assert!(!config.user_prompt_template.is_empty());
    }

    #[test]
    fn test_default_config_prompts() {
        let config = GraphRagConfig::default();
        assert!(config.system_prompt.contains("AI assistant"));
        assert!(config.user_prompt_template.contains("{context}"));
        assert!(config.user_prompt_template.contains("{question}"));
    }

    #[test]
    fn test_hybrid_retriever() {
        let retriever = HybridRetriever::new(0.7, 0.3);
        let combined = retriever.combine_scores(0.8, 0.6);
        assert!((combined - 0.74).abs() < 0.01); // 0.7*0.8 + 0.3*0.6 = 0.56 + 0.18 = 0.74
    }

    #[test]
    fn test_hybrid_retriever_with_config() {
        let config = GraphRagConfig::default();
        let retriever = HybridRetriever::with_config(&config);
        let combined = retriever.combine_scores(1.0, 0.0);
        // 0.5 * 1.0 + 0.5 * 0.0 = 0.5
        assert!((combined - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_hybrid_search_result_creation() {
        let result = HybridSearchResult {
            entity_id: "paper1".to_string(),
            score: 0.85,
            vector_score: 0.9,
            graph_score: 0.7,
            node_type: "Paper".to_string(),
            label: "Attention Is All You Need".to_string(),
            metadata: serde_json::json!({"year": 2017}),
            related_entities: vec![
                RelatedEntity {
                    entity_id: "paper2".to_string(),
                    relation: "CITES".to_string(),
                    label: "Related Paper".to_string(),
                },
            ],
        };
        assert_eq!(result.entity_id, "paper1");
        assert!((result.score - 0.85).abs() < 0.001);
        assert!((result.vector_score - 0.9).abs() < 0.001);
        assert!((result.graph_score - 0.7).abs() < 0.001);
        assert_eq!(result.node_type, "Paper");
        assert_eq!(result.label, "Attention Is All You Need");
        assert_eq!(result.related_entities.len(), 1);
    }

    #[test]
    fn test_graph_rag_answer_debug() {
        let answer = GraphRagAnswer {
            answer: "Test answer".to_string(),
            sources: vec![
                Source {
                    entity_id: "p1".to_string(),
                    label: "Paper 1".to_string(),
                    source_type: "Paper".to_string(),
                    score: 0.9,
                },
            ],
            communities: vec![],
            paths: vec![],
            retrieved_entities: vec![],
        };
        assert_eq!(answer.answer, "Test answer");
        assert_eq!(answer.sources.len(), 1);
    }

    #[test]
    fn test_source_clone() {
        let source = Source {
            entity_id: "p1".to_string(),
            label: "Paper 1".to_string(),
            source_type: "Paper".to_string(),
            score: 0.9,
        };
        let cloned = source.clone();
        assert_eq!(cloned.entity_id, source.entity_id);
        assert_eq!(cloned.label, source.label);
    }

    #[test]
    fn test_retrieval_config_default() {
        let config = crate::retrieval::RetrievalConfig::default();
        assert_eq!(config.vector_top_k, 10);
        assert_eq!(config.graph_top_k, 10);
        assert_eq!(config.max_related, 5);
        assert_eq!(config.min_score, 0.0);
    }
}
