//! GraphRAG integration for rairos-cortex-pro.
//!
//! Provides RAG (Retrieval-Augmented Generation) question answering
//! over materials science literature.

use serde::{Deserialize, Serialize};

/// A question about materials science.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagQuestion {
    /// The question text
    pub question: String,
    /// Optional context or constraints
    pub context: Option<String>,
    /// Maximum number of sources to retrieve
    pub max_sources: usize,
}

impl Default for RagQuestion {
    fn default() -> Self {
        Self {
            question: String::new(),
            context: None,
            max_sources: 5,
        }
    }
}

/// An answer from the RAG system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagAnswer {
    /// The generated answer
    pub answer: String,
    /// Source documents used
    pub sources: Vec<RagSource>,
    /// Confidence score
    pub confidence: f32,
}

/// A source document used in RAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagSource {
    /// Document ID
    pub doc_id: String,
    /// Document title or description
    pub title: String,
    /// Relevant excerpt
    pub excerpt: String,
    /// Relevance score
    pub score: f32,
}

impl Default for RagSource {
    fn default() -> Self {
        Self {
            doc_id: String::new(),
            title: String::new(),
            excerpt: String::new(),
            score: 0.0,
        }
    }
}

/// RAG service for materials science question answering.
#[derive(Debug, Clone)]
pub struct RagService {
    enabled: bool,
}

impl RagService {
    /// Create a new RAG service.
    pub fn new() -> Self {
        Self { enabled: true }
    }

    /// Create a disabled service.
    pub fn disabled() -> Self {
        Self { enabled: false }
    }

    /// Check if this service is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Query the RAG system.
    pub async fn query(&self, question: &RagQuestion) -> Result<RagAnswer, RagServiceError> {
        if !self.enabled {
            return Err(RagServiceError::Disabled);
        }
        // In full implementation, would call rairos-graphrag
        Ok(RagAnswer {
            answer: "RAG query result placeholder".to_string(),
            sources: vec![],
            confidence: 0.8,
        })
    }

    /// Query with hybrid retrieval (vector + knowledge graph).
    pub async fn query_hybrid(
        &self,
        question: &RagQuestion,
    ) -> Result<RagAnswer, RagServiceError> {
        if !self.enabled {
            return Err(RagServiceError::Disabled);
        }
        Ok(RagAnswer {
            answer: "Hybrid RAG result placeholder".to_string(),
            sources: vec![],
            confidence: 0.85,
        })
    }

    /// Index a document for RAG retrieval.
    pub async fn index_document(
        &self,
        _doc_id: &str,
        _content: &str,
        _metadata: serde_json::Value,
    ) -> Result<(), RagServiceError> {
        if !self.enabled {
            return Err(RagServiceError::Disabled);
        }
        Ok(())
    }
}

impl Default for RagService {
    fn default() -> Self {
        Self::new()
    }
}

/// Error type for RAG operations.
#[derive(Debug, thiserror::Error)]
pub enum RagServiceError {
    #[error("RAG service is disabled")]
    Disabled,
    #[error("RAG error: {0}")]
    RagError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rag_question() {
        let q = RagQuestion {
            question: "What is the thermoelectric figure of merit of Bi2Te3?".to_string(),
            context: Some("thermoelectric materials".to_string()),
            max_sources: 3,
        };
        assert!(q.question.contains("Bi2Te3"));
    }

    #[test]
    fn test_rag_service_disabled() {
        let service = RagService::disabled();
        assert!(!service.is_enabled());
    }
}
