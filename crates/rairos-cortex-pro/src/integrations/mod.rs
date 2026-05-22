//! Integration module for connecting SparksCrew with other Rairos crates.
//!
//! This module provides bridges to:
//! - `rairos-vector` - Vector storage and retrieval
//! - `rairos-kg-neo4j` - Knowledge graph operations
//! - `rairos-graphrag` - RAG question answering
//! - `rairos-pdf-advanced` - PDF parsing and literature mining

/// Configuration for integrations with other Rairos crates.
#[derive(Debug, Clone)]
pub struct IntegrationConfig {
    /// Enable vector storage integration
    pub vector_enabled: bool,
    /// Enable knowledge graph integration
    pub kg_enabled: bool,
    /// Enable RAG integration
    pub graphrag_enabled: bool,
    /// Enable PDF/literature integration
    pub pdf_enabled: bool,
}

impl Default for IntegrationConfig {
    fn default() -> Self {
        Self {
            vector_enabled: true,
            kg_enabled: true,
            graphrag_enabled: true,
            pdf_enabled: true,
        }
    }
}

impl IntegrationConfig {
    /// Create a new config with all integrations disabled.
    pub fn minimal() -> Self {
        Self {
            vector_enabled: false,
            kg_enabled: false,
            graphrag_enabled: false,
            pdf_enabled: false,
        }
    }

    /// Enable all integrations.
    pub fn all() -> Self {
        Self {
            vector_enabled: true,
            kg_enabled: true,
            graphrag_enabled: true,
            pdf_enabled: true,
        }
    }

    /// Enable only vector storage.
    pub fn vector_only() -> Self {
        Self {
            vector_enabled: true,
            kg_enabled: false,
            graphrag_enabled: false,
            pdf_enabled: false,
        }
    }
}

pub mod vector_integration;
pub mod kg_integration;
pub mod graphrag_integration;
pub mod pdf_integration;

pub use vector_integration::*;
pub use kg_integration::*;
pub use graphrag_integration::*;
pub use pdf_integration::*;
