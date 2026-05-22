//! Embedder trait and implementations for generating text embeddings.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::error::VectorError;

/// Embedding model identifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmbeddingModel {
    OpenAiAda002,
    OpenAiTextEmbedding3Small,
    OpenAiTextEmbedding3Large,
    Bgem3,
    Custom(String),
}

impl EmbeddingModel {
    pub fn dimensions(&self) -> usize {
        match self {
            EmbeddingModel::OpenAiAda002 => 1536,
            EmbeddingModel::OpenAiTextEmbedding3Small => 1536,
            EmbeddingModel::OpenAiTextEmbedding3Large => 3072,
            EmbeddingModel::Bgem3 => 1024,
            EmbeddingModel::Custom(_) => 0, // Unknown
        }
    }

    pub fn api_name(&self) -> &str {
        match self {
            EmbeddingModel::OpenAiAda002 => "text-embedding-ada-002",
            EmbeddingModel::OpenAiTextEmbedding3Small => "text-embedding-3-small",
            EmbeddingModel::OpenAiTextEmbedding3Large => "text-embedding-3-large",
            EmbeddingModel::Bgem3 => "bge-m3",
            EmbeddingModel::Custom(name) => name,
        }
    }
}

/// Trait for generating embeddings from text.
#[async_trait]
pub trait Embedder: Send + Sync {
    /// Generate embeddings for a list of texts.
    /// Returns a vector of embedding vectors, one per input text.
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, VectorError>;

    /// Get the embedding dimension for this embedder.
    fn dimensions(&self) -> usize;

    /// Get the model identifier.
    fn model(&self) -> &EmbeddingModel;
}

// ============================================================================
// OpenAI Embeddings
// ============================================================================

/// OpenAI Embeddings API client
pub struct OpenAiEmbedder {
    api_key: String,
    base_url: String,
    model: EmbeddingModel,
    client: reqwest::Client,
}

impl OpenAiEmbedder {
    pub fn new(api_key: String) -> Self {
        Self::with_model(api_key, EmbeddingModel::OpenAiTextEmbedding3Small)
    }

    pub fn with_model(api_key: String, model: EmbeddingModel) -> Self {
        Self {
            api_key,
            base_url: "https://api.openai.com/v1".to_string(),
            model,
            client: reqwest::Client::new(),
        }
    }

    pub fn with_base_url(api_key: String, base_url: String) -> Self {
        Self {
            api_key,
            base_url,
            model: EmbeddingModel::OpenAiTextEmbedding3Small,
            client: reqwest::Client::new(),
        }
    }

    async fn do_embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, VectorError> {
        #[derive(Serialize)]
        struct Request {
            input: Vec<String>,
            model: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            encoding_format: Option<String>,
        }

        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct Response {
            data: Vec<EmbeddingData>,
            usage: Usage,
        }

        #[derive(Deserialize)]
        struct EmbeddingData {
            embedding: Vec<f32>,
            index: usize,
        }

        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct Usage {
            prompt_tokens: usize,
            total_tokens: usize,
        }

        let url = format!("{}/embeddings", self.base_url);
        let request = Request {
            input: texts.clone(),
            model: self.model.api_name().to_string(),
            encoding_format: Some("float".to_string()),
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| VectorError::ApiError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(VectorError::ApiError(format!("OpenAI API error {}: {}", status, body)));
        }

        let resp: Response = response
            .json()
            .await
            .map_err(|e| VectorError::ApiError(e.to_string()))?;

        // Sort by index to maintain order
        let mut embeddings: Vec<Vec<f32>> = vec![vec![]; texts.len()];
        for data in resp.data {
            embeddings[data.index] = data.embedding;
        }

        Ok(embeddings)
    }
}

#[async_trait]
impl Embedder for OpenAiEmbedder {
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, VectorError> {
        // OpenAI has a limit of 8192 tokens per request, roughly 200-400 texts
        // Chunk to be safe
        const CHUNK_SIZE: usize = 100;

        let mut all_embeddings = Vec::with_capacity(texts.len());
        for chunk in texts.chunks(CHUNK_SIZE) {
            let chunk_embeddings = self.do_embed(chunk.to_vec()).await?;
            all_embeddings.extend(chunk_embeddings);
        }
        Ok(all_embeddings)
    }

    fn dimensions(&self) -> usize {
        self.model.dimensions()
    }

    fn model(&self) -> &EmbeddingModel {
        &self.model
    }
}

// ============================================================================
// BGE-M3 Embeddings (via MiniMax API)
// ============================================================================

/// BGE-M3 embedding client via MiniMax API
pub struct BgemEmbedder {
    api_key: String,
    base_url: String,
    dimensions: usize,
    client: reqwest::Client,
}

impl BgemEmbedder {
    pub fn new(api_key: String) -> Self {
        Self::with_base_url(
            api_key,
            "https://api.minimaxi.com/v1".to_string(),
        )
    }

    pub fn with_base_url(api_key: String, base_url: String) -> Self {
        Self {
            api_key,
            base_url,
            dimensions: 1024, // BGE-M3 default
            client: reqwest::Client::new(),
        }
    }

    async fn do_embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, VectorError> {
        #[derive(Serialize)]
        struct Request {
            model: String,
            input: Vec<String>,
        }

        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct Response {
            data: Vec<EmbeddingData>,
            usage: Usage,
        }

        #[derive(Deserialize)]
        struct EmbeddingData {
            embedding: Vec<f32>,
            index: usize,
        }

        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct Usage {
            prompt_tokens: usize,
            total_tokens: usize,
        }

        let url = format!("{}/embeddings", self.base_url);
        let request = Request {
            input: texts,
            model: "bge-m3".to_string(),
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| VectorError::ApiError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(VectorError::ApiError(format!("BGE-M3 API error {}: {}", status, body)));
        }

        let resp: Response = response
            .json()
            .await
            .map_err(|e| VectorError::ApiError(e.to_string()))?;

        let mut embeddings: Vec<Vec<f32>> = vec![vec![]; resp.data.len()];
        for data in resp.data {
            embeddings[data.index] = data.embedding;
        }

        Ok(embeddings)
    }
}

#[async_trait]
impl Embedder for BgemEmbedder {
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, VectorError> {
        const CHUNK_SIZE: usize = 100;
        let mut all_embeddings = Vec::with_capacity(texts.len());

        for chunk in texts.chunks(CHUNK_SIZE) {
            let chunk_embeddings = self.do_embed(chunk.to_vec()).await?;
            all_embeddings.extend(chunk_embeddings);
        }
        Ok(all_embeddings)
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn model(&self) -> &EmbeddingModel {
        &EmbeddingModel::Bgem3
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock embedder for testing that returns predefined embeddings
    struct MockEmbedder {
        dimensions: usize,
        embeddings: Vec<Vec<f32>>,
        model: EmbeddingModel,
    }

    impl MockEmbedder {
        fn new(dimensions: usize, embeddings: Vec<Vec<f32>>) -> Self {
            Self {
                dimensions,
                embeddings,
                model: EmbeddingModel::Custom("mock".to_string()),
            }
        }
    }

    #[async_trait]
    impl Embedder for MockEmbedder {
        async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, VectorError> {
            // Return the predefined embeddings, repeating if necessary
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

    #[tokio::test]
    async fn test_embedding_model_dimensions() {
        assert_eq!(EmbeddingModel::OpenAiAda002.dimensions(), 1536);
        assert_eq!(EmbeddingModel::OpenAiTextEmbedding3Small.dimensions(), 1536);
        assert_eq!(EmbeddingModel::OpenAiTextEmbedding3Large.dimensions(), 3072);
        assert_eq!(EmbeddingModel::Bgem3.dimensions(), 1024);
        assert_eq!(EmbeddingModel::Custom("test".to_string()).dimensions(), 0);
    }

    #[tokio::test]
    async fn test_embedding_model_api_name() {
        assert_eq!(EmbeddingModel::OpenAiAda002.api_name(), "text-embedding-ada-002");
        assert_eq!(EmbeddingModel::OpenAiTextEmbedding3Small.api_name(), "text-embedding-3-small");
        assert_eq!(EmbeddingModel::OpenAiTextEmbedding3Large.api_name(), "text-embedding-3-large");
        assert_eq!(EmbeddingModel::Bgem3.api_name(), "bge-m3");
        assert_eq!(EmbeddingModel::Custom("custom-model".to_string()).api_name(), "custom-model");
    }

    #[tokio::test]
    async fn test_mock_embedder_single_text() {
        let embedder = MockEmbedder::new(
            4,
            vec![vec![0.1, 0.2, 0.3, 0.4]],
        );
        let result = embedder.embed(vec!["hello".to_string()]).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], vec![0.1, 0.2, 0.3, 0.4]);
    }

    #[tokio::test]
    async fn test_mock_embedder_multiple_texts() {
        let embedder = MockEmbedder::new(
            4,
            vec![vec![0.1, 0.2, 0.3, 0.4], vec![0.5, 0.6, 0.7, 0.8]],
        );
        let result = embedder
            .embed(vec!["hello".to_string(), "world".to_string()])
            .await
            .unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], vec![0.1, 0.2, 0.3, 0.4]);
        assert_eq!(result[1], vec![0.5, 0.6, 0.7, 0.8]);
    }

    #[tokio::test]
    async fn test_mock_embedder_repeats_embeddings() {
        let embedder = MockEmbedder::new(
            4,
            vec![vec![0.1, 0.2, 0.3, 0.4]],
        );
        // More texts than embeddings - should cycle
        let result = embedder
            .embed(vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
            ])
            .await
            .unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], vec![0.1, 0.2, 0.3, 0.4]);
        assert_eq!(result[1], vec![0.1, 0.2, 0.3, 0.4]);
        assert_eq!(result[2], vec![0.1, 0.2, 0.3, 0.4]);
    }

    #[tokio::test]
    async fn test_mock_embedder_empty_texts() {
        let embedder = MockEmbedder::new(4, vec![]);
        let result = embedder.embed(vec![]).await.unwrap();
        assert_eq!(result.len(), 0);
    }

    #[tokio::test]
    async fn test_mock_embedder_dimensions() {
        let embedder = MockEmbedder::new(256, vec![vec![0.0; 256]]);
        assert_eq!(embedder.dimensions(), 256);
    }

    #[tokio::test]
    async fn test_openai_embedder_new() {
        let embedder = OpenAiEmbedder::new("test-key".to_string());
        assert_eq!(embedder.dimensions(), 1536); // default model
    }

    #[tokio::test]
    async fn test_openai_embedder_with_model() {
        let embedder = OpenAiEmbedder::with_model(
            "test-key".to_string(),
            EmbeddingModel::OpenAiTextEmbedding3Large,
        );
        assert_eq!(embedder.dimensions(), 3072);
    }

    #[tokio::test]
    async fn test_openai_embedder_with_base_url() {
        let embedder = OpenAiEmbedder::with_base_url(
            "test-key".to_string(),
            "https://custom.api.com/v1".to_string(),
        );
        assert_eq!(embedder.dimensions(), 1536);
    }

    #[tokio::test]
    async fn test_bgem_embedder_new() {
        let embedder = BgemEmbedder::new("test-key".to_string());
        assert_eq!(embedder.dimensions(), 1024);
    }

    #[tokio::test]
    async fn test_bgem_embedder_with_base_url() {
        let embedder = BgemEmbedder::with_base_url(
            "test-key".to_string(),
            "https://custom.api.com/v1".to_string(),
        );
        assert_eq!(embedder.dimensions(), 1024);
    }
}
