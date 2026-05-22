//! Error types for rairos-cortex-pro

use thiserror::Error;

/// Common result type for rairos-cortex-pro operations
pub type Result<T> = std::result::Result<T, CortexProError>;

#[derive(Error, Debug)]
pub enum CortexProError {
    #[error("Agent execution failed: {0}")]
    AgentError(String),

    #[error("Crew execution failed: {0}")]
    CrewError(String),

    #[error("Pipeline error: {0}")]
    PipelineError(String),

    #[error("State error: {0}")]
    StateError(String),

    #[error("LLM error: {0}")]
    LlmError(String),

    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Invalid pipeline: {0}")]
    InvalidPipeline(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Context limit exceeded: {0}")]
    ContextLimitExceeded(String),

    /// Wrapper for underlying errors with source chaining
    #[error("External error: {source}")]
    External {
        #[from]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}
