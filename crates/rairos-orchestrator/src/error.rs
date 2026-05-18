//! rairos-orchestrator — Autonomous Research Orchestrator (closed-loop research agent)
//!
//! Watches arXiv via subscriptions, triggers deep gap analysis on new papers,
//! scores results against Gene Pool preferences, and notifies when high-value
//! research opportunities are found.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum OrchestratorError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("State error: {0}")]
    State(String),

    #[error("Not initialized: {0}")]
    NotInitialized(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, OrchestratorError>;