//! Vector storage integration for rairos-cortex-pro.
//!
//! Provides functionality to store and retrieve research results as vectors
//! for similarity search and RAG applications.

use serde::{Deserialize, Serialize};

/// A research result that can be stored as a vector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchVector {
    /// The research hypothesis or finding
    pub content: String,
    /// Type of research result
    pub result_type: ResearchResultType,
    /// Associated metadata
    pub metadata: VectorMetadata,
}

/// Type of research result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResearchResultType {
    /// A generated hypothesis
    Hypothesis,
    /// A research plan
    Plan,
    /// An execution result
    ExecutionResult,
    /// A generated report section
    ReportSection,
    /// Literature finding
    LiteratureFinding,
}

impl Default for ResearchResultType {
    fn default() -> Self {
        ResearchResultType::Hypothesis
    }
}

/// Metadata for a research vector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorMetadata {
    /// Source query or topic
    pub topic: Option<String>,
    /// Confidence score (0.0 - 1.0)
    pub confidence: Option<f32>,
    /// Timestamp
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for VectorMetadata {
    fn default() -> Self {
        Self {
            topic: None,
            confidence: None,
            timestamp: Some(chrono::Utc::now()),
        }
    }
}

/// Vector storage service for research results.
#[derive(Debug, Clone)]
pub struct VectorStorageService {
    enabled: bool,
}

impl VectorStorageService {
    /// Create a new vector storage service.
    pub fn new() -> Self {
        Self { enabled: true }
    }

    /// Create a disabled service (for when rairos-vector is not available).
    pub fn disabled() -> Self {
        Self { enabled: false }
    }

    /// Check if this service is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Store a research result.
    pub async fn store(&self, _result: &ResearchVector) -> Result<String, VectorStorageError> {
        if !self.enabled {
            return Err(VectorStorageError::Disabled);
        }
        // In a full implementation, this would call rairos-vector
        Ok("vector-id-placeholder".to_string())
    }

    /// Search for similar research results.
    pub async fn search(
        &self,
        _query: &str,
        _limit: usize,
    ) -> Result<Vec<SearchHit>, VectorStorageError> {
        if !self.enabled {
            return Err(VectorStorageError::Disabled);
        }
        // In a full implementation, this would call rairos-vector
        Ok(vec![])
    }
}

impl Default for VectorStorageService {
    fn default() -> Self {
        Self::new()
    }
}

/// Error type for vector storage operations.
#[derive(Debug, thiserror::Error)]
pub enum VectorStorageError {
    #[error("Vector storage is disabled")]
    Disabled,
    #[error("Storage error: {0}")]
    StorageError(String),
}

/// A search hit result.
#[derive(Debug, Clone)]
pub struct SearchHit {
    /// The matched research vector
    pub vector: ResearchVector,
    /// Similarity score
    pub score: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_research_vector_creation() {
        let vector = ResearchVector {
            content: "Doping Bi2Te3 with Se improves thermoelectric performance".to_string(),
            result_type: ResearchResultType::Hypothesis,
            metadata: VectorMetadata::default(),
        };
        assert!(vector.content.contains("Bi2Te3"));
    }

    #[test]
    fn test_vector_storage_disabled() {
        let service = VectorStorageService::disabled();
        assert!(!service.is_enabled());
    }
}
