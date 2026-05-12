//! rairos-vector-store — Zilliz Cloud vector store for persistent code embeddings.
//!
//! Ported from `core/vector_store.py`.
//!
//! This module provides the core data structures and traits for vector storage.
//! The actual Zilliz/Milvus client integration would require the pymilvus crate
//! which has complex C bindings. This crate provides the types and local-only
//! utilities that can work without a remote connection.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur in vector store operations.
#[derive(Error, Debug)]
pub enum VectorStoreError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Collection not found: {0}")]
    CollectionNotFound(String),

    #[error("Insert failed: {0}")]
    InsertFailed(String),

    #[error("Search failed: {0}")]
    SearchFailed(String),

    #[error("Delete failed: {0}")]
    DeleteFailed(String),

    #[error("Not configured: ZILLIZ_URI must be set")]
    NotConfigured,

    #[error("Invalid vector dimension: expected {expected}, got {got}")]
    InvalidDimension { expected: usize, got: usize },
}

/// A search result with score and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Unique chunk ID
    pub id: String,
    /// Similarity score (0-1, higher is more similar)
    pub score: f32,
    /// Code content
    pub content: String,
    /// File path
    pub file: String,
    /// Line number
    pub line: i32,
}

impl SearchResult {
    /// Create a new search result.
    pub fn new(id: String, score: f32, content: String, file: String, line: i32) -> Self {
        Self {
            id,
            score,
            content,
            file,
            line,
        }
    }
}

/// Collection schema definition for vector storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionSchema {
    /// Collection name
    pub name: String,
    /// Vector dimension
    pub dim: usize,
    /// Description
    pub description: String,
}

impl CollectionSchema {
    /// Create a new collection schema.
    pub fn new(name: String, dim: usize) -> Self {
        Self {
            name,
            dim,
            description: String::new(),
        }
    }

    /// Create schema for code chunks collection.
    pub fn code_chunks(dim: usize) -> Self {
        Self {
            name: "code_chunks".to_string(),
            dim,
            description: "Code embedding chunks for semantic search".to_string(),
        }
    }
}

/// Vector store statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStoreStats {
    /// Number of chunks
    pub chunks: i64,
    /// Vector dimension
    pub dim: usize,
    /// Collection name
    pub collection: String,
}

impl VectorStoreStats {
    /// Create new stats.
    pub fn new(chunks: i64, dim: usize, collection: String) -> Self {
        Self {
            chunks,
            dim,
            collection,
        }
    }
}

/// Configuration for Zilliz connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZillizConfig {
    /// Zilliz Cloud URI
    pub uri: String,
    /// Zilliz API key
    pub token: String,
    /// Default vector dimension
    pub dim: usize,
}

impl ZillizConfig {
    /// Create config from environment variables.
    pub fn from_env() -> Result<Self, VectorStoreError> {
        let uri = std::env::var("ZILLIZ_URI").map_err(|_| VectorStoreError::NotConfigured)?;
        let token = std::env::var("ZILLIZ_TOKEN").unwrap_or_default();
        Ok(Self {
            uri,
            token,
            dim: 768, // nomic-embed-text default
        })
    }

    /// Check if Zilliz is configured.
    pub fn is_configured() -> bool {
        std::env::var("ZILLIZ_URI").is_ok()
    }
}

/// Upsert request for batch insert/update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertRequest {
    /// Chunk IDs
    pub ids: Vec<String>,
    /// Embedding vectors
    pub vectors: Vec<Vec<f32>>,
    /// Code contents
    pub contents: Vec<String>,
    /// File paths
    pub files: Vec<String>,
    /// Line numbers
    pub lines: Vec<i32>,
}

impl UpsertRequest {
    /// Validate the upsert request.
    pub fn validate(&self) -> Result<(), VectorStoreError> {
        let n = self.ids.len();
        if self.vectors.len() != n
            || self.contents.len() != n
            || self.files.len() != n
            || self.lines.len() != n
        {
            return Err(VectorStoreError::InsertFailed(
                "All input vectors must have the same length".to_string(),
            ));
        }
        Ok(())
    }
}

/// Search request parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    /// Query vector
    pub query_vector: Vec<f32>,
    /// Maximum results to return
    pub limit: usize,
    /// Optional filter expression
    pub filter_expr: Option<String>,
}

impl SearchRequest {
    /// Create a new search request.
    pub fn new(query_vector: Vec<f32>, limit: usize) -> Self {
        Self {
            query_vector,
            limit,
            filter_expr: None,
        }
    }

    /// Create with a filter expression.
    pub fn with_filter(mut self, filter: String) -> Self {
        self.filter_expr = Some(filter);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_result_new() {
        let result = SearchResult::new(
            "chunk1".to_string(),
            0.95,
            "fn main() {}".to_string(),
            "src/main.rs".to_string(),
            1,
        );
        assert_eq!(result.id, "chunk1");
        assert!((result.score - 0.95).abs() < 0.001);
    }

    #[test]
    fn test_collection_schema_code_chunks() {
        let schema = CollectionSchema::code_chunks(768);
        assert_eq!(schema.name, "code_chunks");
        assert_eq!(schema.dim, 768);
    }

    #[test]
    fn test_zilliz_config_not_configured() {
        // When ZILLIZ_URI is not set
        std::env::remove_var("ZILLIZ_URI");
        assert!(!ZillizConfig::is_configured());
    }

    #[test]
    fn test_upsert_request_validation() {
        let request = UpsertRequest {
            ids: vec!["1".to_string()],
            vectors: vec![vec![0.1, 0.2]],
            contents: vec!["content".to_string()],
            files: vec!["file.rs".to_string()],
            lines: vec![1],
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_upsert_request_validation_mismatch() {
        let request = UpsertRequest {
            ids: vec!["1".to_string(), "2".to_string()],
            vectors: vec![vec![0.1, 0.2]], // Only one vector
            contents: vec!["content".to_string()],
            files: vec!["file.rs".to_string()],
            lines: vec![1],
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_search_request_new() {
        let request = SearchRequest::new(vec![0.1, 0.2, 0.3], 10);
        assert_eq!(request.limit, 10);
        assert!(request.filter_expr.is_none());
    }

    #[test]
    fn test_search_request_with_filter() {
        let request = SearchRequest::new(vec![0.1, 0.2, 0.3], 10)
            .with_filter("file like '%test%'".to_string());
        assert!(request.filter_expr.is_some());
    }
}
