//! Error types for rairos-vector

use thiserror::Error;

#[derive(Error, Debug)]
pub enum VectorError {
    #[error("Embedding generation failed: {0}")]
    EmbeddingFailed(String),

    #[error("Vector store error: {0}")]
    StoreError(String),

    #[error("API request failed: {0}")]
    ApiError(String),

    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("LLM generation failed: {0}")]
    LlmError(String),
}
