//! Rairos Rankers Base — Base abstractions for ranking strategies
//!
//! Reference: Python rankers/base.py
//!
//! Provides the Ranker trait and RankedResult type.

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum RankerError {
    #[error("Paper not found: {0}")]
    PaperNotFound(String),
    #[error("Embedding not found for paper: {0}")]
    NoEmbedding(String),
    #[error("Invalid threshold: {0}")]
    InvalidThreshold(String),
    #[error("Invalid limit: {0}")]
    InvalidLimit(String),
}

/// Result type alias
pub type Result<T> = std::result::Result<T, RankerError>;

// ============================================================================
// RankedResult
// ============================================================================

/// A paper record paired with its ranking score (higher = better).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedResult<P> {
    /// The paper record
    pub paper: P,
    /// The ranking score (higher = better)
    pub score: f32,
}

impl<P> RankedResult<P> {
    /// Create a new RankedResult
    pub fn new(paper: P, score: f32) -> Self {
        Self { paper, score }
    }

    /// Map the paper through a function
    pub fn map<Q, F>(self, f: F) -> RankedResult<Q>
    where
        F: FnOnce(P) -> Q,
    {
        RankedResult {
            paper: f(self.paper),
            score: self.score,
        }
    }
}

// ============================================================================
// Ranker Trait
// ============================================================================

/// Abstract base for paper ranking strategies.
pub trait Ranker<P> {
    /// Rank papers similar/related to paper_id.
    ///
    /// # Arguments
    /// * `paper_id` - Query paper ID
    /// * `threshold` - Minimum score to include (default 0.0 = no filter)
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    /// List of RankedResult sorted by score descending
    fn rank(&self, paper_id: &str, threshold: f32, limit: usize) -> Result<Vec<RankedResult<P>>>;

    /// Get the name of this ranker
    fn name(&self) -> &'static str {
        "base"
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ranked_result_new() {
        let result: RankedResult<String> = RankedResult::new("paper1".to_string(), 0.95);
        assert_eq!(result.paper, "paper1");
        assert_eq!(result.score, 0.95);
    }

    #[test]
    fn test_ranked_result_map() {
        let result: RankedResult<i32> = RankedResult::new(42, 0.8);
        let mapped = result.map(|n| n * 2);
        assert_eq!(mapped.paper, 84);
        assert_eq!(mapped.score, 0.8);
    }

    #[test]
    fn test_ranked_result_clone() {
        let result: RankedResult<String> = RankedResult::new("test".to_string(), 0.5);
        let cloned = result.clone();
        assert_eq!(cloned.paper, result.paper);
        assert_eq!(cloned.score, result.score);
    }

    #[test]
    fn test_ranker_error_paper_not_found() {
        let err = RankerError::PaperNotFound("p123".to_string());
        assert!(err.to_string().contains("p123"));
    }

    #[test]
    fn test_ranker_error_no_embedding() {
        let err = RankerError::NoEmbedding("p456".to_string());
        assert!(err.to_string().contains("p456"));
    }

    #[test]
    fn test_ranked_result_serialize() {
        let result: RankedResult<&str> = RankedResult::new("paper1", 0.85);
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("paper1"));
        assert!(json.contains("0.85"));
    }

    #[test]
    fn test_ranked_result_deserialize() {
        let json = r#"{"paper":"paper1","score":0.75}"#;
        let result: RankedResult<String> = serde_json::from_str(json).unwrap();
        assert_eq!(result.paper, "paper1");
        assert!((result.score - 0.75).abs() < f32::EPSILON);
    }
}

// ============================================================================
// Mock Ranker for Testing
// ============================================================================

/// A simple mock ranker for testing
pub struct MockRanker {
    pub name: &'static str,
    pub results: Vec<RankedResult<String>>,
}

impl MockRanker {
    pub fn new(name: &'static str, results: Vec<RankedResult<String>>) -> Self {
        Self { name, results }
    }
}

impl Ranker<String> for MockRanker {
    fn rank(
        &self,
        _paper_id: &str,
        threshold: f32,
        limit: usize,
    ) -> Result<Vec<RankedResult<String>>> {
        let filtered: Vec<_> = self
            .results
            .iter()
            .filter(|r| r.score >= threshold)
            .take(limit)
            .cloned()
            .collect();
        Ok(filtered)
    }

    fn name(&self) -> &'static str {
        self.name
    }
}

#[cfg(test)]
mod mock_tests {
    use super::*;

    #[test]
    fn test_mock_ranker() {
        let results = vec![
            RankedResult::new("paper1".to_string(), 0.9),
            RankedResult::new("paper2".to_string(), 0.8),
            RankedResult::new("paper3".to_string(), 0.7),
        ];
        let ranker = MockRanker::new("test", results);

        let found = ranker.rank("query", 0.75, 10).unwrap();
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].paper, "paper1");
    }

    #[test]
    fn test_mock_ranker_with_threshold() {
        let results = vec![
            RankedResult::new("high".to_string(), 0.95),
            RankedResult::new("low".to_string(), 0.5),
        ];
        let ranker = MockRanker::new("test", results);

        let found = ranker.rank("query", 0.9, 10).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].paper, "high");
    }

    #[test]
    fn test_mock_ranker_with_limit() {
        let results = vec![
            RankedResult::new("p1".to_string(), 0.9),
            RankedResult::new("p2".to_string(), 0.85),
            RankedResult::new("p3".to_string(), 0.8),
            RankedResult::new("p4".to_string(), 0.75),
        ];
        let ranker = MockRanker::new("test", results);

        let found = ranker.rank("query", 0.0, 2).unwrap();
        assert_eq!(found.len(), 2);
    }
}
