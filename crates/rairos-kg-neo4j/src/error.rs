//! Error types for rairos-kg-neo4j

use thiserror::Error;

#[derive(Error, Debug)]
pub enum KgError {
    #[error("Neo4j connection failed: {0}")]
    ConnectionError(String),

    #[error("Cypher query failed: {0}")]
    QueryError(String),

    #[error("Node not found: {0}")]
    NodeNotFound(String),

    #[error("Edge not found: {0}")]
    EdgeNotFound(String),

    #[error("Invalid node type: {0}")]
    InvalidNodeType(String),

    #[error("Invalid edge type: {0}")]
    InvalidEdgeType(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Database error: {0}")]
    DatabaseError(String),
}
