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

// ============================================================================
// Parallel Evaluation Metrics with Rayon
// ============================================================================

use rayon::prelude::*;

#[derive(Debug, Clone)]
struct ScoredDocument {
    doc_id: u64,
    score: f64,
}

#[derive(Debug, Clone)]
struct QueryResult {
    query_id: u64,
    documents: Vec<ScoredDocument>,
}

fn ndcg_at_k(query_result: &QueryResult, relevant_docs: &[u64], k: usize) -> f64 {
    let top_k_len = query_result.documents.len().min(k);
    let top_k = &query_result.documents[..top_k_len];

    let dcg: f64 = top_k
        .iter()
        .enumerate()
        .map(|(i, doc)| {
            let rel = if relevant_docs.contains(&doc.doc_id) { 1.0 } else { 0.0 };
            rel / (i as f64 + 2.0).log2()
        })
        .sum();

    let idcg: f64 = (0..top_k_len.min(relevant_docs.len()))
        .map(|i| 1.0 / (i as f64 + 2.0).log2())
        .sum();

    if idcg == 0.0 {
        0.0
    } else {
        dcg / idcg
    }
}

pub fn average_ndcg_parallel(
    query_results: &[QueryResult],
    relevant_per_query: &[&[u64]],
    k: usize,
) -> f64 {
    if query_results.is_empty() {
        return 0.0;
    }

    let sum: f64 = query_results
        .par_iter()
        .zip(relevant_per_query.par_iter())
        .map(|(qr, rels)| ndcg_at_k(qr, rels, k))
        .sum();

    sum / query_results.len() as f64
}

#[cfg(test)]
mod ndcg_tests {
    use super::*;

    #[test]
    fn test_ndcg_empty_results() {
        let qr = QueryResult {
            query_id: 1,
            documents: vec![],
        };
        let relevant = &[1u64, 2, 3];
        assert_eq!(ndcg_at_k(&qr, relevant, 10), 0.0);
    }

    #[test]
    fn test_ndcg_perfect_ranking() {
        let qr = QueryResult {
            query_id: 1,
            documents: vec![
                ScoredDocument { doc_id: 1, score: 0.9 },
                ScoredDocument { doc_id: 2, score: 0.8 },
                ScoredDocument { doc_id: 3, score: 0.7 },
            ],
        };
        let relevant = &[1u64, 2, 3];
        let ndcg = ndcg_at_k(&qr, relevant, 3);
        assert!((ndcg - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_ndcg_parallel_empty() {
        let results: &[QueryResult] = &[];
        let relevant: &[&[u64]] = &[];
        assert_eq!(average_ndcg_parallel(results, relevant, 10), 0.0);
    }

    #[test]
    fn test_ndcg_parallel_multiple_queries() {
        let results = vec![
            QueryResult {
                query_id: 1,
                documents: vec![
                    ScoredDocument { doc_id: 1, score: 0.9 },
                    ScoredDocument { doc_id: 2, score: 0.8 },
                ],
            },
            QueryResult {
                query_id: 2,
                documents: vec![
                    ScoredDocument { doc_id: 3, score: 0.95 },
                    ScoredDocument { doc_id: 4, score: 0.85 },
                ],
            },
        ];
        let relevant: &[&[u64]] = &[&[1u64, 2], &[3u64, 4]];
        let avg = average_ndcg_parallel(&results, relevant, 10);
        assert!((avg - 1.0).abs() < 1e-6);
    }
}

// ============================================================================
// Property-Based Testing for Ranker Correctness
// ============================================================================

#[cfg(test)]
mod ranker_property_tests {
    use super::*;
    use proptest::prelude::*;

    fn ranked_result_vec() -> impl Strategy<Value = Vec<RankedResult<String>>> {
        prop::collection::vec(
            (any::<String>(), any::<f32>()),
            1..50,
        )
        .prop_map(|v| {
            v.into_iter()
                .map(|(paper, score)| RankedResult { paper, score })
                .collect()
        })
    }

    proptest! {
        #[test]
        fn mock_ranker_respects_threshold(results in ranked_result_vec(), threshold: f32) {
            let ranker = MockRanker::new("prop_test", results.clone());
            let found = ranker.rank("query", threshold, 100).unwrap();

            prop_assert!(found.iter().all(|r| r.score >= threshold));
        }

        #[test]
        fn mock_ranker_respects_limit(results in ranked_result_vec(), limit: usize) {
            let limit = (limit % 100).max(1);
            let ranker = MockRanker::new("prop_test", results);
            let found = ranker.rank("query", 0.0, limit).unwrap();

            prop_assert!(found.len() <= limit);
        }

        #[test]
        fn ranked_results_sorted_by_score_descending(results in ranked_result_vec()) {
            if results.len() < 2 {
                return Ok(());
            }

            let mut sorted_results = results.clone();
            sorted_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

            let ranker = MockRanker::new("prop_test", sorted_results);
            let found = ranker.rank("query", 0.0, 100).unwrap();

            if found.len() >= 2 {
                for window in found.windows(2) {
                    prop_assert!(window[0].score >= window[1].score,
                        "Scores not decreasing: {:?} >= {:?}", window[0].score, window[1].score);
                }
            }
        }

        #[test]
        fn ranker_returns_subset_of_original(results in ranked_result_vec()) {
            let ranker = MockRanker::new("prop_test", results.clone());
            let original_ids: std::collections::HashSet<_> = results.iter().map(|r| r.paper.clone()).collect();

            let found = ranker.rank("query", 0.0, 100).unwrap();
            for r in &found {
                prop_assert!(original_ids.contains(&r.paper));
            }
        }
    }
}

// ========== Code Gene Implementation ==========
/// Compute weighted score from multiple ranking signals.
/// Each signal contributes with a specific weight to the final score.
pub fn weighted_score(signals: &[f64], weights: &[f64]) -> f64 {
    if signals.len() != weights.len() {
        panic!("signals and weights must have same length");
    }
    signals.iter()
        .zip(weights.iter())
        .map(|(s, w)| s * w)
        .sum()
}

// ========== Test Code ==========
    // From code gene test: test-gene-001
    #[test]
    fn test_weighted_score_basic() {
        let signals = vec![0.9, 0.8, 0.7];
        let weights = vec![0.5, 0.3, 0.2];
        let result = weighted_score(&signals, &weights);
        assert!((result - 0.83).abs() < 0.01);
    }

// ========== Code Gene Implementation ==========
/// LRU cache for ranker results
pub struct LruRankerCache<K, V> {
    cache: std::collections::HashMap<K, V>,
    order: Vec<K>,
    capacity: usize,
}

impl<K: std::hash::Hash + Eq + Clone, V: Clone> LruRankerCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        Self { cache: std::collections::HashMap::new(), order: Vec::new(), capacity }
    }
    
    pub fn get(&mut self, key: &K) -> Option<V> {
        if let Some(v) = self.cache.get(key) {
            // Move to end (most recently used)
            self.order.retain(|k| k != key);
            self.order.push(key.clone());
            Some(v.clone())
        } else {
            None
        }
    }
    
    pub fn insert(&mut self, key: K, value: V) {
        if self.cache.contains_key(&key) {
            self.order.retain(|k| k != &key);
        } else if self.cache.len() >= self.capacity {
            if let Some(oldest) = self.order.first() {
                self.cache.remove(oldest);
            }
            self.order.remove(0);
        }
        self.cache.insert(key.clone(), value);
        self.order.push(key);
    }
}

// ========== Code Gene: 18b0eb7d ==========
// 8. Precision@K with Threshold
/// Compute Precision@K with relevance threshold.
pub fn precision_at_k(relevant: &[u64], ranked: &[u64], k: usize, threshold: f32) -> f32 {
    if k == 0 || ranked.is_empty() {
        return 0.0;
    }
    
    let top_k = &ranked[..ranked.len().min(k)];
    let relevant_in_topk = top_k.iter()
        .filter(|&&doc| {
            // Simplified: assume relevant if in the relevant set
            relevant.contains(&doc)
        })
        .count() as f32;
    
    relevant_in_topk / k as f32
}

// ========== Test: 18b0eb7d ==========
    // 8. Precision@K with Threshold
    #[test]
    fn test_precision_at_k_basic() {
        let relevant = vec![1, 2, 3];
        let ranked = vec![1, 4, 2, 5];
        let p = precision_at_k(&relevant, &ranked, 3, 0.0);
        assert!((p - 0.666).abs() < 0.01);
    }