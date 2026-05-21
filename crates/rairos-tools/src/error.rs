//! Tool execution errors.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Invalid input parameters: {0}")]
    InvalidInput(String),

    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Timeout after {0:?}")]
    Timeout(std::time::Duration),
}
