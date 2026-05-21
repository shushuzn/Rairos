//! FAISS local vector index implementation
//!
//! This module provides a local FAISS index for vector storage and search.
//! FAISS is a library for efficient similarity search and clustering of dense vectors.
//!
//! Note: This is a simplified implementation. For production use with FAISS,
//! consider using the `faiss` crate bindings or calling Python FAISS via subprocess.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use crate::client::{DistanceMetric, SearchHit, VectorStore, VectorStoreConfig};
use crate::error::VectorError;

/// FAISS index types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FaissIndexType {
    /// Flat index - exact search, good for small datasets
    Flat,
    /// IVF index - approximate search with inverted file index
    Ivf,
    /// HNSW index - hierarchical navigable small world graph
    Hnsw,
}

impl Default for FaissIndexType {
    fn default() -> Self {
        FaissIndexType::Flat
    }
}

/// FAISS local vector store configuration
#[derive(Debug, Clone)]
pub struct FaissStoreConfig {
    pub index_type: FaissIndexType,
    pub nlist: usize,       // For IVF: number of clusters
    pub nprobe: usize,      // For IVF: number of clusters to search
    pub m: usize,           // For HNSW: number of connections
    pub ef_construction: usize, // For HNSW: construction parameter
}

impl Default for FaissStoreConfig {
    fn default() -> Self {
        Self {
            index_type: FaissIndexType::Flat,
            nlist: 100,
            nprobe: 10,
            m: 32,
            ef_construction: 40,
        }
    }
}

/// In-memory FAISS-like store (simplified implementation)
///
/// This provides a pure-Rust alternative to FAISS bindings for development
/// and testing. For production, consider using the `faiss` crate or
/// calling Python FAISS via subprocess.
pub struct FaissStore {
    vectors: RwLock<HashMap<String, Vec<f32>>>,
    payloads: RwLock<HashMap<String, serde_json::Value>>,
    dimensions: RwLock<usize>,
    distance_metric: RwLock<DistanceMetric>,
    config: FaissStoreConfig,
}

impl FaissStore {
    pub fn new() -> Self {
        Self {
            vectors: RwLock::new(HashMap::new()),
            payloads: RwLock::new(HashMap::new()),
            dimensions: RwLock::new(0),
            distance_metric: RwLock::new(DistanceMetric::Cosine),
            config: FaissStoreConfig::default(),
        }
    }

    pub fn with_config(config: FaissStoreConfig) -> Self {
        Self {
            vectors: RwLock::new(HashMap::new()),
            payloads: RwLock::new(HashMap::new()),
            dimensions: RwLock::new(0),
            distance_metric: RwLock::new(DistanceMetric::Cosine),
            config,
        }
    }

    fn compute_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        match *self.distance_metric.read().unwrap() {
            DistanceMetric::Cosine => {
                let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
                let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
                let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
                if norm_a == 0.0 || norm_b == 0.0 {
                    0.0
                } else {
                    dot / (norm_a * norm_b)
                }
            }
            DistanceMetric::Euclidean => {
                let dist: f32 = a
                    .iter()
                    .zip(b.iter())
                    .map(|(x, y)| (x - y).powi(2))
                    .sum::<f32>()
                    .sqrt();
                -dist // Negative so higher = more similar
            }
            DistanceMetric::DotProduct => {
                a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
            }
        }
    }

    fn compute_distances(&self, query: &[f32]) -> Vec<(String, f32)> {
        let vectors = self.vectors.read().unwrap();
        let mut results: Vec<(String, f32)> = vectors
            .iter()
            .map(|(id, vec)| {
                let score = self.compute_similarity(query, vec);
                (id.clone(), score)
            })
            .collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results
    }
}

impl Default for FaissStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VectorStore for FaissStore {
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

        self.vectors
            .write()
            .unwrap()
            .insert(id.to_string(), embedding.to_vec());
        if let Some(p) = payload {
            self.payloads.write().unwrap().insert(id.to_string(), p);
        }
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
        let distances = self.compute_distances(query);
        let payloads = self.payloads.read().unwrap();

        let hits: Vec<SearchHit> = distances
            .into_iter()
            .take(top_k)
            .map(|(id, score)| SearchHit {
                id: id.clone(),
                score,
                payload: payloads.get(&id).cloned(),
                embedding: None,
            })
            .collect();

        Ok(hits)
    }

    async fn delete(&self, id: &str) -> Result<(), VectorError> {
        self.vectors.write().unwrap().remove(id);
        self.payloads.write().unwrap().remove(id);
        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Option<SearchHit>, VectorError> {
        let vectors = self.vectors.read().unwrap();
        let payloads = self.payloads.read().unwrap();

        if let Some(vec) = vectors.get(id) {
            Ok(Some(SearchHit {
                id: id.to_string(),
                score: 1.0,
                payload: payloads.get(id).cloned(),
                embedding: Some(vec.clone()),
            }))
        } else {
            Ok(None)
        }
    }

    async fn count(&self) -> Result<usize, VectorError> {
        Ok(self.vectors.read().unwrap().len())
    }

    async fn exists(&self, id: &str) -> Result<bool, VectorError> {
        Ok(self.vectors.read().unwrap().contains_key(id))
    }

    async fn drop_collection(&self) -> Result<(), VectorError> {
        self.vectors.write().unwrap().clear();
        self.payloads.write().unwrap().clear();
        *self.dimensions.write().unwrap() = 0;
        Ok(())
    }
}

impl FaissStore {
    /// Train the index with provided vectors (for IVF/HNSW)
    /// This is a placeholder - real FAISS would do clustering
    pub async fn train(&self, _vectors: &[Vec<f32>]) -> Result<(), VectorError> {
        // In a real FAISS implementation, this would fit the index
        // For the simplified version, this is a no-op
        Ok(())
    }
}
