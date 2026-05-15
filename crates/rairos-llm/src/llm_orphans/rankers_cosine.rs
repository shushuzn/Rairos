//! Rairos Rankers Cosine — Cosine similarity ranking strategy
//!
//! Reference: Python rankers/cosine.py
//!
//! Ranks papers by cosine similarity of their embedding vectors.
//! Uses ndarray for batch vector operations.

use ndarray::arr1;
use crate::llm_orphans::rankers_base::{RankedResult, Ranker, RankerError, Result};
use serde::{Deserialize, Serialize};

// ============================================================================
// Paper (simplified for this crate)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paper {
    pub id: String,
    pub title: String,
    pub embedding: Option<Vec<f32>>,
}

impl Paper {
    pub fn new(id: &str, title: &str, embedding: Option<Vec<f32>>) -> Self {
        Self {
            id: id.to_string(),
            title: title.to_string(),
            embedding,
        }
    }
}

// ============================================================================
// Cosine Similarity Ranker
// ============================================================================

/// Rank papers by cosine similarity of their embedding vectors.
///
/// Uses ndarray for batch vector operations — O(n) scan with all similarity
/// computation happening in a single vectorized pass.
pub struct CosineSimilarityRanker {
    papers: Vec<Paper>,
}

impl CosineSimilarityRanker {
    pub fn new(papers: Vec<Paper>) -> Self {
        Self { papers }
    }

    /// Compute cosine similarity between two vectors
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let a_arr = arr1(a);
        let b_arr = arr1(b);

        let norm_a = a_arr.dot(&a_arr).sqrt();
        let norm_b = b_arr.dot(&b_arr).sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        a_arr.dot(&b_arr) / (norm_a * norm_b)
    }
}

impl Ranker<Paper> for CosineSimilarityRanker {
    fn rank(
        &self,
        paper_id: &str,
        threshold: f32,
        limit: usize,
    ) -> Result<Vec<RankedResult<Paper>>> {
        // Find the query paper
        let query_paper = self.papers.iter().find(|p| p.id == paper_id);

        let query_paper = match query_paper {
            Some(p) => p,
            None => return Err(RankerError::PaperNotFound(paper_id.to_string())),
        };

        let query_emb = match &query_paper.embedding {
            Some(e) => e,
            None => return Err(RankerError::NoEmbedding(paper_id.to_string())),
        };

        // Compute similarities to all other papers
        let mut scored: Vec<(Paper, f32)> = self
            .papers
            .iter()
            .filter(|p| p.id != paper_id)
            .filter_map(|p| {
                let emb = p.embedding.as_ref()?;
                let sim = Self::cosine_similarity(query_emb, emb);
                Some((p.clone(), sim))
            })
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Filter by threshold and limit
        let results: Vec<RankedResult<Paper>> = scored
            .into_iter()
            .filter(|(_, score)| *score >= threshold)
            .take(limit)
            .map(|(p, score)| RankedResult::new(p, score))
            .collect();

        Ok(results)
    }

    fn name(&self) -> &'static str {
        "cosine"
    }
}

// ============================================================================
// Simple Cosine Ranker (standalone)
// ============================================================================

/// Simple concrete implementation with Vec<f32> embeddings
pub struct SimpleCosineRanker {
    embeddings: std::collections::HashMap<String, Vec<f32>>,
    papers: std::collections::HashMap<String, String>, // id -> title
}

impl SimpleCosineRanker {
    pub fn new() -> Self {
        Self {
            embeddings: std::collections::HashMap::new(),
            papers: std::collections::HashMap::new(),
        }
    }

    pub fn add_paper(&mut self, id: &str, title: &str, embedding: Vec<f32>) {
        self.papers.insert(id.to_string(), title.to_string());
        self.embeddings.insert(id.to_string(), embedding);
    }

    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot / (norm_a * norm_b)
    }

    pub fn rank(&self, paper_id: &str, threshold: f32, limit: usize) -> Vec<(String, f32)> {
        let query_emb = match self.embeddings.get(paper_id) {
            Some(e) => e,
            None => return vec![],
        };

        let mut scored: Vec<(String, f32)> = self
            .embeddings
            .iter()
            .filter(|(id, _)| *id != paper_id)
            .map(|(id, emb)| {
                let sim = Self::cosine_similarity(query_emb, emb);
                (id.clone(), sim)
            })
            .filter(|(_, sim)| *sim >= threshold)
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        scored
    }
}

impl Default for SimpleCosineRanker {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let v = vec![1.0, 0.0, 0.0];
        assert!((CosineSimilarityRanker::cosine_similarity(&v, &v) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let v1 = vec![1.0, 0.0, 0.0];
        let v2 = vec![0.0, 1.0, 0.0];
        assert!((CosineSimilarityRanker::cosine_similarity(&v1, &v2)).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let v1 = vec![1.0, 0.0, 0.0];
        let v2 = vec![-1.0, 0.0, 0.0];
        assert!((CosineSimilarityRanker::cosine_similarity(&v1, &v2) - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_different_lengths() {
        let v1 = vec![1.0, 0.0];
        let v2 = vec![1.0, 0.0, 0.0];
        assert!((CosineSimilarityRanker::cosine_similarity(&v1, &v2)).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_zero_vector() {
        let zero = vec![0.0, 0.0, 0.0];
        let v = vec![1.0, 0.0, 0.0];
        assert!((CosineSimilarityRanker::cosine_similarity(&zero, &v)).abs() < 1e-6);
        assert!((CosineSimilarityRanker::cosine_similarity(&v, &zero)).abs() < 1e-6);
    }

    #[test]
    fn test_simple_cosine_ranker_new() {
        let ranker = SimpleCosineRanker::new();
        assert!(ranker.embeddings.is_empty());
    }

    #[test]
    fn test_simple_cosine_ranker_add_paper() {
        let mut ranker = SimpleCosineRanker::new();
        ranker.add_paper("p1", "Paper 1", vec![1.0, 0.0, 0.0]);
        assert_eq!(ranker.papers.len(), 1);
        assert_eq!(ranker.embeddings.len(), 1);
    }

    #[test]
    fn test_simple_cosine_ranker_rank() {
        let mut ranker = SimpleCosineRanker::new();
        ranker.add_paper("p1", "Paper 1", vec![1.0, 0.0, 0.0]);
        ranker.add_paper("p2", "Paper 2", vec![0.0, 1.0, 0.0]);
        ranker.add_paper("p3", "Paper 3", vec![0.9, 0.1, 0.0]);

        let results = ranker.rank("p1", 0.0, 10);
        assert_eq!(results.len(), 2);

        // p3 should be first (closer to p1 than p2)
        assert_eq!(results[0].0, "p3");
        assert!(results[0].1 > results[1].1);
    }

    #[test]
    fn test_simple_cosine_ranker_rank_with_threshold() {
        let mut ranker = SimpleCosineRanker::new();
        ranker.add_paper("p1", "Paper 1", vec![1.0, 0.0, 0.0]);
        ranker.add_paper("p2", "Paper 2", vec![0.0, 1.0, 0.0]);
        ranker.add_paper("p3", "Paper 3", vec![0.1, 0.0, 0.0]);

        let results = ranker.rank("p1", 0.5, 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "p3");
    }

    #[test]
    fn test_simple_cosine_ranker_rank_not_found() {
        let ranker = SimpleCosineRanker::new();
        let results = ranker.rank("nonexistent", 0.0, 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_simple_cosine_ranker_rank_limit() {
        let mut ranker = SimpleCosineRanker::new();
        for i in 0..5 {
            ranker.add_paper(
                &format!("p{}", i),
                &format!("Paper {}", i),
                vec![1.0 - i as f32 * 0.2, 0.0, 0.0],
            );
        }

        let results = ranker.rank("p0", 0.0, 2);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_cosine_similarity_ranker_rank() {
        let papers = vec![
            Paper::new("p1", "Paper 1", Some(vec![1.0, 0.0, 0.0])),
            Paper::new("p2", "Paper 2", Some(vec![0.0, 1.0, 0.0])),
            Paper::new("p3", "Paper 3", Some(vec![0.9, 0.1, 0.0])),
        ];
        let ranker = CosineSimilarityRanker::new(papers);

        let results = ranker.rank("p1", 0.0, 10).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].paper.id, "p3");
    }

    #[test]
    fn test_cosine_similarity_ranker_not_found() {
        let papers = vec![Paper::new("p1", "Paper 1", Some(vec![1.0, 0.0, 0.0]))];
        let ranker = CosineSimilarityRanker::new(papers);

        let results = ranker.rank("nonexistent", 0.0, 10);
        assert!(results.is_err());
    }
}
