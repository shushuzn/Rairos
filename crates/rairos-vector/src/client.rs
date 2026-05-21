//! VectorStore trait for storing and searching embeddings.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use crate::error::VectorError;

/// Configuration for vector store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStoreConfig {
    pub collection_name: String,
    pub dimensions: usize,
    pub distance_metric: DistanceMetric,
}

impl VectorStoreConfig {
    pub fn new(collection_name: impl Into<String>, dimensions: usize) -> Self {
        Self {
            collection_name: collection_name.into(),
            dimensions,
            distance_metric: DistanceMetric::Cosine,
        }
    }

    pub fn with_distance_metric(mut self, metric: DistanceMetric) -> Self {
        self.distance_metric = metric;
        self
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DistanceMetric {
    Cosine,
    Euclidean,
    DotProduct,
}

impl Default for DistanceMetric {
    fn default() -> Self {
        DistanceMetric::Cosine
    }
}

/// A search result hit with score and optional payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    /// Unique identifier for the stored vector
    pub id: String,
    /// Similarity score (higher = more similar for cosine/dot, lower = more similar for euclidean)
    pub score: f32,
    /// Optional payload/metadata stored with the vector
    pub payload: Option<serde_json::Value>,
    /// The embedding vector (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
}

/// Trait for vector storage and retrieval.
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Initialize the vector store (create collection, etc.)
    async fn initialize(&self, config: &VectorStoreConfig) -> Result<(), VectorError>;

    /// Upsert a single vector with payload
    async fn upsert_one(
        &self,
        id: &str,
        embedding: &[f32],
        payload: Option<serde_json::Value>,
    ) -> Result<(), VectorError>;

    /// Upsert multiple vectors at once
    async fn upsert_batch(
        &self,
        vectors: Vec<(String, Vec<f32>, Option<serde_json::Value>)>,
    ) -> Result<(), VectorError>;

    /// Search for similar vectors
    async fn search(
        &self,
        query: &[f32],
        top_k: usize,
        filter: Option<&str>,
    ) -> Result<Vec<SearchHit>, VectorError>;

    /// Delete a vector by ID
    async fn delete(&self, id: &str) -> Result<(), VectorError>;

    /// Get a vector by ID
    async fn get(&self, id: &str) -> Result<Option<SearchHit>, VectorError>;

    /// Get the number of vectors in the store
    async fn count(&self) -> Result<usize, VectorError>;

    /// Check if a vector exists
    async fn exists(&self, id: &str) -> Result<bool, VectorError>;

    /// Drop the collection
    async fn drop_collection(&self) -> Result<(), VectorError>;
}

/// Simple in-memory vector store for testing
pub struct InMemoryVectorStore {
    vectors: RwLock<HashMap<String, (Vec<f32>, Option<serde_json::Value>)>>,
    dimensions: RwLock<usize>,
    distance_metric: RwLock<DistanceMetric>,
}

impl InMemoryVectorStore {
    pub fn new() -> Self {
        Self {
            vectors: RwLock::new(HashMap::new()),
            dimensions: RwLock::new(0),
            distance_metric: RwLock::new(DistanceMetric::Cosine),
        }
    }

    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot / (norm_a * norm_b)
        }
    }

    fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f32>()
            .sqrt()
    }
}

impl Default for InMemoryVectorStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VectorStore for InMemoryVectorStore {
    async fn initialize(&self, config: &VectorStoreConfig) -> Result<(), VectorError> {
        *self.dimensions.write().unwrap() = config.dimensions;
        *self.distance_metric.write().unwrap() = config.distance_metric;
        Ok(())
    }

    async fn upsert_one(
        &self,
        id: &str,
        embedding: &[f32],
        payload: Option<serde_json::Value>,
    ) -> Result<(), VectorError> {
        let dims = *self.dimensions.read().unwrap();
        if dims == 0 {
            *self.dimensions.write().unwrap() = embedding.len();
        } else if embedding.len() != dims {
            return Err(VectorError::DimensionMismatch {
                expected: dims,
                got: embedding.len(),
            });
        }
        self.vectors.write().unwrap().insert(id.to_string(), (embedding.to_vec(), payload));
        Ok(())
    }

    async fn upsert_batch(
        &self,
        vectors: Vec<(String, Vec<f32>, Option<serde_json::Value>)>,
    ) -> Result<(), VectorError> {
        for (id, embedding, payload) in vectors {
            self.upsert_one(&id, &embedding, payload).await?;
        }
        Ok(())
    }

    async fn search(
        &self,
        query: &[f32],
        top_k: usize,
        _filter: Option<&str>,
    ) -> Result<Vec<SearchHit>, VectorError> {
        let dims = *self.dimensions.read().unwrap();
        if dims == 0 {
            return Ok(vec![]);
        }

        let vectors = self.vectors.read().unwrap();
        let metric = *self.distance_metric.read().unwrap();

        let mut results: Vec<SearchHit> = vectors
            .iter()
            .map(|(id, (embedding, payload))| {
                let score = match metric {
                    DistanceMetric::Cosine => Self::cosine_similarity(query, embedding),
                    DistanceMetric::Euclidean => -Self::euclidean_distance(query, embedding), // Negative for sorting
                    DistanceMetric::DotProduct => {
                        query.iter().zip(embedding.iter()).map(|(x, y)| x * y).sum()
                    }
                };
                SearchHit {
                    id: id.clone(),
                    score,
                    payload: payload.clone(),
                    embedding: None,
                }
            })
            .collect();

        // Sort by score descending (higher is better for cosine/dot, and we negated euclidean)
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
            score: 1.0, // Self-similarity
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
        *self.dimensions.write().unwrap() = 0;
        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_vector(dim: usize) -> Vec<f32> {
        (0..dim).map(|i| (i as f32) * 0.1).collect()
    }

    fn create_payload(text: &str) -> serde_json::Value {
        serde_json::json!({ "text": text, "source": "test" })
    }

    #[tokio::test]
    async fn test_in_memory_vector_store_new() {
        let store = InMemoryVectorStore::new();
        let count = store.count().await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_in_memory_vector_store_initialize() {
        let store = InMemoryVectorStore::new();
        let config = VectorStoreConfig::new("test-collection", 128);
        store.initialize(&config).await.unwrap();

        // After init, dimensions should be set
        let result = store.search(&vec![0.0; 128], 10, None).await.unwrap();
        assert_eq!(result.len(), 0);
    }

    #[tokio::test]
    async fn test_upsert_one_basic() {
        let store = InMemoryVectorStore::new();
        let vector = create_test_vector(4);

        store.upsert_one("doc1", &vector, None).await.unwrap();

        let count = store.count().await.unwrap();
        assert_eq!(count, 1);

        let exists = store.exists("doc1").await.unwrap();
        assert!(exists);
    }

    #[tokio::test]
    async fn test_upsert_one_with_payload() {
        let store = InMemoryVectorStore::new();
        let vector = create_test_vector(4);
        let payload = create_payload("Hello world");

        store.upsert_one("doc1", &vector, Some(payload.clone())).await.unwrap();

        let hit = store.get("doc1").await.unwrap();
        assert!(hit.is_some());
        let hit = hit.unwrap();
        assert_eq!(hit.id, "doc1");
        assert!(hit.embedding.is_some());
        assert_eq!(hit.payload, Some(payload));
    }

    #[tokio::test]
    async fn test_upsert_one_dimension_mismatch() {
        let store = InMemoryVectorStore::new();
        let vector1 = create_test_vector(4);
        let vector2 = create_test_vector(8);

        store.upsert_one("doc1", &vector1, None).await.unwrap();
        let result = store.upsert_one("doc2", &vector2, None).await;

        assert!(result.is_err());
        if let Err(VectorError::DimensionMismatch { expected, got }) = result {
            assert_eq!(expected, 4);
            assert_eq!(got, 8);
        } else {
            panic!("Expected DimensionMismatch error");
        }
    }

    #[tokio::test]
    async fn test_upsert_one_auto_set_dimensions() {
        let store = InMemoryVectorStore::new();
        let vector = create_test_vector(4);

        // First upsert should auto-set dimensions
        store.upsert_one("doc1", &vector, None).await.unwrap();

        // Second upsert with same dimensions should work
        store.upsert_one("doc2", &vector, None).await.unwrap();

        let count = store.count().await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_upsert_batch_basic() {
        let store = InMemoryVectorStore::new();
        let vectors = vec![
            ("doc1".to_string(), create_test_vector(4), None),
            ("doc2".to_string(), create_test_vector(4), None),
            ("doc3".to_string(), create_test_vector(4), None),
        ];

        store.upsert_batch(vectors).await.unwrap();

        let count = store.count().await.unwrap();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_upsert_batch_with_payloads() {
        let store = InMemoryVectorStore::new();
        let vectors = vec![
            ("doc1".to_string(), create_test_vector(4), Some(create_payload("Text 1"))),
            ("doc2".to_string(), create_test_vector(4), Some(create_payload("Text 2"))),
        ];

        store.upsert_batch(vectors).await.unwrap();

        let hit1 = store.get("doc1").await.unwrap().unwrap();
        let hit2 = store.get("doc2").await.unwrap().unwrap();

        assert!(hit1.payload.is_some());
        assert!(hit2.payload.is_some());
    }

    #[tokio::test]
    async fn test_upsert_batch_overwrites_existing() {
        let store = InMemoryVectorStore::new();
        let vector1 = create_test_vector(4);
        let vector2 = vec![0.9, 0.8, 0.7, 0.6];

        store.upsert_one("doc1", &vector1, None).await.unwrap();
        store.upsert_batch(vec![("doc1".to_string(), vector2.clone(), None)])
            .await
            .unwrap();

        let hit = store.get("doc1").await.unwrap().unwrap();
        assert_eq!(hit.embedding, Some(vector2));
    }

    #[tokio::test]
    async fn test_search_basic() {
        let store = InMemoryVectorStore::new();
        let vector = create_test_vector(4);

        store.upsert_one("doc1", &vector, None).await.unwrap();
        store.upsert_one("doc2", &vec![0.0, 0.0, 0.0, 0.0], None).await.unwrap();

        let results = store.search(&vector, 10, None).await.unwrap();

        assert_eq!(results.len(), 2);
        // First result should be doc1 (self-similarity = 1.0)
        assert_eq!(results[0].id, "doc1");
        assert!((results[0].score - 1.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_search_top_k() {
        let store = InMemoryVectorStore::new();

        for i in 0..10 {
            let vector: Vec<f32> = (0..4).map(|j| (i as f32) * 0.1 + (j as f32) * 0.01).collect();
            store.upsert_one(&format!("doc{}", i), &vector, None).await.unwrap();
        }

        let results = store.search(&create_test_vector(4), 3, None).await.unwrap();
        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn test_search_empty_store() {
        let store = InMemoryVectorStore::new();
        let results = store.search(&vec![0.0; 4], 10, None).await.unwrap();
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_search_uninitialized() {
        let store = InMemoryVectorStore::new();
        // Without initialize, dimensions is 0
        let results = store.search(&vec![0.0; 4], 10, None).await.unwrap();
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_search_cosine_similarity() {
        let store = InMemoryVectorStore::new();

        // Identical vectors should have score 1.0
        let vector = vec![1.0, 0.0, 0.0, 0.0];
        store.upsert_one("doc1", &vector, None).await.unwrap();

        let results = store.search(&vector, 1, None).await.unwrap();
        assert!((results[0].score - 1.0).abs() < 0.001);

        // Opposite vectors should have score -1.0
        let opposite = vec![-1.0, 0.0, 0.0, 0.0];
        let results = store.search(&opposite, 1, None).await.unwrap();
        assert!((results[0].score - (-1.0)).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_delete_basic() {
        let store = InMemoryVectorStore::new();
        store.upsert_one("doc1", &create_test_vector(4), None).await.unwrap();

        let exists = store.exists("doc1").await.unwrap();
        assert!(exists);

        store.delete("doc1").await.unwrap();

        let exists = store.exists("doc1").await.unwrap();
        assert!(!exists);
    }

    #[tokio::test]
    async fn test_delete_nonexistent() {
        let store = InMemoryVectorStore::new();
        // Should not error when deleting non-existent
        store.delete("nonexistent").await.unwrap();
    }

    #[tokio::test]
    async fn test_get_basic() {
        let store = InMemoryVectorStore::new();
        let vector = create_test_vector(4);
        let payload = create_payload("Test content");

        store.upsert_one("doc1", &vector, Some(payload.clone())).await.unwrap();

        let hit = store.get("doc1").await.unwrap();
        assert!(hit.is_some());

        let hit = hit.unwrap();
        assert_eq!(hit.id, "doc1");
        assert_eq!(hit.embedding, Some(vector));
        assert_eq!(hit.payload, Some(payload));
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let store = InMemoryVectorStore::new();
        let hit = store.get("nonexistent").await.unwrap();
        assert!(hit.is_none());
    }

    #[tokio::test]
    async fn test_count_empty() {
        let store = InMemoryVectorStore::new();
        let count = store.count().await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_count_after_upsert() {
        let store = InMemoryVectorStore::new();
        store.upsert_batch(vec![
            ("doc1".to_string(), create_test_vector(4), None),
            ("doc2".to_string(), create_test_vector(4), None),
        ]).await.unwrap();

        let count = store.count().await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_exists() {
        let store = InMemoryVectorStore::new();
        assert!(!store.exists("doc1").await.unwrap());

        store.upsert_one("doc1", &create_test_vector(4), None).await.unwrap();
        assert!(store.exists("doc1").await.unwrap());
    }

    #[tokio::test]
    async fn test_drop_collection() {
        let store = InMemoryVectorStore::new();
        store.upsert_batch(vec![
            ("doc1".to_string(), create_test_vector(4), None),
            ("doc2".to_string(), create_test_vector(4), None),
        ]).await.unwrap();

        assert_eq!(store.count().await.unwrap(), 2);

        store.drop_collection().await.unwrap();

        assert_eq!(store.count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_cosine_similarity() {
        // Test the static method directly
        let a = vec![1.0, 0.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0, 0.0];
        assert!((InMemoryVectorStore::cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let a = vec![1.0, 0.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0, 0.0];
        assert!((InMemoryVectorStore::cosine_similarity(&a, &b) - 0.0).abs() < 0.001);

        let a = vec![1.0, 0.0, 0.0, 0.0];
        let b = vec![-1.0, 0.0, 0.0, 0.0];
        assert!((InMemoryVectorStore::cosine_similarity(&a, &b) - (-1.0)).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_euclidean_distance() {
        let a = vec![0.0, 0.0, 0.0, 0.0];
        let b = vec![3.0, 4.0, 0.0, 0.0];
        let dist = InMemoryVectorStore::euclidean_distance(&a, &b);
        assert!((dist - 5.0).abs() < 0.001); // sqrt(3^2 + 4^2) = 5
    }

    #[tokio::test]
    async fn test_vector_store_config_new() {
        let config = VectorStoreConfig::new("my-collection", 128);
        assert_eq!(config.collection_name, "my-collection");
        assert_eq!(config.dimensions, 128);
        assert_eq!(config.distance_metric, DistanceMetric::Cosine);
    }

    #[tokio::test]
    async fn test_vector_store_config_with_distance_metric() {
        let config = VectorStoreConfig::new("my-collection", 128)
            .with_distance_metric(DistanceMetric::Euclidean);
        assert_eq!(config.distance_metric, DistanceMetric::Euclidean);
    }

    #[tokio::test]
    async fn test_distance_metric_default() {
        let metric = DistanceMetric::default();
        assert_eq!(metric, DistanceMetric::Cosine);
    }

    #[tokio::test]
    async fn test_search_hit_serialization() {
        let hit = SearchHit {
            id: "doc1".to_string(),
            score: 0.95,
            payload: Some(serde_json::json!({"text": "hello"})),
            embedding: None,
        };

        let json = serde_json::to_string(&hit).unwrap();
        let deserialized: SearchHit = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, "doc1");
        assert!((deserialized.score - 0.95).abs() < f32::EPSILON);
    }
}
