//! Error types for rairos-graphrag

use thiserror::Error;

#[derive(Error, Debug)]
pub enum GraphRagError {
    #[error("Embedding failed: {0}")]
    EmbeddingFailed(String),

    #[error("Vector store error: {0}")]
    VectorStoreError(String),

    #[error("Knowledge graph error: {0}")]
    KgError(String),

    #[error("LLM generation failed: {0}")]
    GenerationError(String),

    #[error("No relevant documents found for query: {0}")]
    NoResults(String),

    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    #[error("Community summarization failed: {0}")]
    SummarizationError(String),
}
