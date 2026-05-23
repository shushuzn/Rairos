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
use std::hash::Hash;
use std::sync::RwLock;
use std::borrow::Borrow;
use crate::client::{DistanceMetric, SearchHit, VectorStore, VectorStoreConfig};
use crate::error::VectorError;

/// Bounded cache with simple LRU eviction (FIFO - oldest entry removed first)
/// Used to prevent unbounded memory growth in HashMaps
struct BoundedCache<K, V> {
    map: HashMap<K, V>,
    order: Vec<K>, // Most recent at end (LRU: end = most recently used)
    capacity: usize,
}

impl<K: Eq + Hash + Clone, V> BoundedCache<K, V> {
    fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::with_capacity(capacity),
            order: Vec::with_capacity(capacity),
            capacity,
        }
    }

    fn insert(&mut self, k: K, v: V) {
        // If key exists, update and move to end (most recent)
        if self.map.contains_key(&k) {
            self.map.insert(k.clone(), v);
            // Remove old position and push to end
            if let Some(pos) = self.order.iter().position(|x| x == &k) {
                self.order.remove(pos);
            }
            self.order.push(k);
            return;
        }

        // Evict oldest if at capacity
        if self.map.len() >= self.capacity {
            if let Some(oldest) = self.order.first() {
                self.map.remove(oldest);
                self.order.remove(0);
            }
        }

        self.map.insert(k.clone(), v);
        self.order.push(k);
    }

    fn get<Q: ?Sized>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.map.get(k)
    }

    fn remove<Q: ?Sized>(&mut self, k: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        if let Some(v) = self.map.remove(k) {
            if let Some(pos) = self.order.iter().position(|x| x.borrow() == k) {
                self.order.remove(pos);
            }
            return Some(v);
        }
        None
    }

    fn len(&self) -> usize {
        self.map.len()
    }

    fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    fn contains_key<Q: ?Sized>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.map.contains_key(k)
    }

    fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
    }

    fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.map.iter()
    }
}

impl<K: Eq + Hash + Clone, V> Default for BoundedCache<K, V> {
    fn default() -> Self {
        Self::new(1000)
    }
}

/// FAISS index types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[derive(Default)]
pub enum FaissIndexType {
    /// Flat index - exact search, good for small datasets
    #[default]
    Flat,
    /// IVF index - approximate search with inverted file index
    Ivf,
    /// HNSW index - hierarchical navigable small world graph
    Hnsw,
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
#[allow(dead_code)]
pub struct FaissStore {
    vectors: RwLock<BoundedCache<String, Vec<f32>>>,
    payloads: RwLock<BoundedCache<String, serde_json::Value>>,
    dimensions: RwLock<usize>,
    distance_metric: RwLock<DistanceMetric>,
    config: FaissStoreConfig,
}

impl FaissStore {
    pub fn new() -> Self {
        Self {
            vectors: RwLock::new(BoundedCache::new(10000)), // Vector storage: 10000 entries max
            payloads: RwLock::new(BoundedCache::new(10000)), // Payload storage: 10000 entries max
            dimensions: RwLock::new(0),
            distance_metric: RwLock::new(DistanceMetric::Cosine),
            config: FaissStoreConfig::default(),
        }
    }

    pub fn with_config(config: FaissStoreConfig) -> Self {
        Self {
            vectors: RwLock::new(BoundedCache::new(10000)),
            payloads: RwLock::new(BoundedCache::new(10000)),
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
        if vectors.is_empty() {
            return Ok(());
        }

        // Batch processing: single lock acquisition for all operations
        let mut vectors_guard = self.vectors.write().unwrap();
        let mut payloads_guard = self.payloads.write().unwrap();

        // Check/update dimensions with write lock held
        let dims = *self.dimensions.read().unwrap();
        let first_dim = vectors.first().map(|(_, v, _)| v.len()).unwrap_or(0);

        if dims == 0 {
            *self.dimensions.write().unwrap() = first_dim;
        } else if first_dim != dims {
            return Err(VectorError::DimensionMismatch { expected: dims, got: first_dim });
        }

        // Insert all vectors and payloads under single lock scope
        for (id, embedding, payload) in vectors {
            if embedding.len() != dims {
                return Err(VectorError::DimensionMismatch { expected: dims, got: embedding.len() });
            }
            vectors_guard.insert(id.clone(), embedding);
            if let Some(p) = payload {
                payloads_guard.insert(id, p);
            }
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
        // Single scope for both write operations - shorter critical section
        let mut vectors = self.vectors.write().unwrap();
        let mut payloads = self.payloads.write().unwrap();
        vectors.remove(id);
        payloads.remove(id);
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
        // Single scope for all three write operations - reduces lock overhead
        let mut vectors = self.vectors.write().unwrap();
        let mut payloads = self.payloads.write().unwrap();
        let mut dimensions = self.dimensions.write().unwrap();
        vectors.clear();
        payloads.clear();
        *dimensions = 0;
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
