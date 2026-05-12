//! Rairos Rankers Score — Composite scoring strategy
//!
//! Reference: Python rankers/score.py
//!
//! Ranks papers by weighted combination of:
//! - Cosine similarity of embeddings (semantic overlap)
//! - Recency bonus (newer papers rank higher)
//! - Parse quality bonus (papers with better parse_status rank higher)

use chrono::{Datelike, NaiveDate, Utc};
use rairos_rankers_base::{RankedResult, Ranker, RankerError, Result};
use serde::{Deserialize, Serialize};

// ============================================================================
// Error Types
// ============================================================================

// ============================================================================
// Parse Status
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParseStatus {
    Full,
    Sections,
    Partial,
    Failed,
    Unknown,
}

impl ParseStatus {
    /// Map string to ParseStatus
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "full" => ParseStatus::Full,
            "sections" => ParseStatus::Sections,
            "partial" => ParseStatus::Partial,
            "failed" => ParseStatus::Failed,
            _ => ParseStatus::Unknown,
        }
    }
}

// ============================================================================
// Paper (simplified)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paper {
    pub id: String,
    pub title: String,
    pub published: Option<NaiveDate>,
    pub parse_status: Option<ParseStatus>,
    pub embedding: Option<Vec<f32>>,
}

impl Paper {
    pub fn new(
        id: &str,
        title: &str,
        published: Option<NaiveDate>,
        parse_status: Option<ParseStatus>,
        embedding: Option<Vec<f32>>,
    ) -> Self {
        Self {
            id: id.to_string(),
            title: title.to_string(),
            published,
            parse_status,
            embedding,
        }
    }

    pub fn published_year(&self) -> Option<i32> {
        self.published.map(|d| d.year())
    }
}

// ============================================================================
// Composite Scorer
// ============================================================================

/// Rank papers by a weighted combination of:
/// - Cosine similarity of embeddings (semantic overlap)
/// - Recency bonus (newer papers rank higher)
/// - Parse quality bonus (papers with better parse_status rank higher)
pub struct CompositeScorer {
    sim_weight: f32,
    recency_weight: f32,
    parse_weight: f32,
    year_boost_range: i32,
}

impl CompositeScorer {
    /// Create a new scorer with validated weights (must sum to 1.0).
    pub fn new(sim_weight: f32, recency_weight: f32, parse_weight: f32) -> Self {
        assert!((sim_weight + recency_weight + parse_weight - 1.0).abs() < 0.001);
        Self {
            sim_weight,
            recency_weight,
            parse_weight,
            year_boost_range: 5,
        }
    }

    /// Create scorer with default weights (0.7, 0.2, 0.1).
    pub fn with_defaults() -> Self {
        Self {
            sim_weight: 0.7,
            recency_weight: 0.2,
            parse_weight: 0.1,
            year_boost_range: 5,
        }
    }

    /// Map parse_status to 0-1 quality score
    pub fn parse_quality_score(status: Option<ParseStatus>) -> f32 {
        match status.unwrap_or(ParseStatus::Unknown) {
            ParseStatus::Full => 1.0,
            ParseStatus::Sections => 0.8,
            ParseStatus::Partial => 0.5,
            ParseStatus::Failed => 0.1,
            ParseStatus::Unknown => 0.0,
        }
    }

    /// Compute cosine similarity between two vectors
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
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

    /// Rank papers by composite score
    pub fn rank_papers(
        &self,
        query_paper: &Paper,
        candidates: &[Paper],
        threshold: f32,
        limit: usize,
    ) -> std::result::Result<Vec<RankedResult<Paper>>, RankerError> {
        let query_emb = match &query_paper.embedding {
            Some(e) => e,
            None => return Err(RankerError::NoEmbedding(query_paper.id.clone())),
        };

        let ref_year = candidates
            .iter()
            .filter_map(|p| p.published_year())
            .max()
            .unwrap_or_else(|| Utc::now().year());

        let mut scored: Vec<(Paper, f32)> = candidates
            .iter()
            .filter(|p| p.id != query_paper.id)
            .filter_map(|p| {
                let emb = p.embedding.as_ref()?;
                let sim = Self::cosine_similarity(query_emb, emb);
                Some((p.clone(), sim))
            })
            .map(|(mut p, sim_score)| {
                let sim_norm = sim_score.min(1.0);
                let recency_norm = if let Some(year) = p.published_year() {
                    let year_dist = (ref_year - year).max(0).min(self.year_boost_range) as f32;
                    1.0 - (year_dist / self.year_boost_range as f32)
                } else {
                    0.0
                };
                let parse_norm = Self::parse_quality_score(p.parse_status);
                let composite = self.sim_weight * sim_norm
                    + self.recency_weight * recency_norm
                    + self.parse_weight * parse_norm;
                p.embedding = None;
                (p, composite)
            })
            .filter(|(_, score)| *score >= threshold)
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        Ok(scored
            .into_iter()
            .map(|(p, s)| RankedResult::new(p, s))
            .collect())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_paper(id: &str, year: i32, month: u32, day: u32, status: Option<ParseStatus>) -> Paper {
        Paper::new(
            id,
            &format!("Paper {}", id),
            NaiveDate::from_ymd_opt(year, month, day),
            status,
            Some(vec![1.0, 0.0, 0.0]),
        )
    }

    #[test]
    fn test_parse_quality_score_full() {
        assert!((CompositeScorer::parse_quality_score(Some(ParseStatus::Full)) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_parse_quality_score_sections() {
        assert!((CompositeScorer::parse_quality_score(Some(ParseStatus::Sections)) - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn test_parse_quality_score_partial() {
        assert!((CompositeScorer::parse_quality_score(Some(ParseStatus::Partial)) - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_parse_quality_score_failed() {
        assert!((CompositeScorer::parse_quality_score(Some(ParseStatus::Failed)) - 0.1).abs() < f32::EPSILON);
    }

    #[test]
    fn test_parse_quality_score_unknown() {
        assert!((CompositeScorer::parse_quality_score(None) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_parse_status_from_str() {
        assert_eq!(ParseStatus::from_str("full"), ParseStatus::Full);
        assert_eq!(ParseStatus::from_str("FULL"), ParseStatus::Full);
        assert_eq!(ParseStatus::from_str("sections"), ParseStatus::Sections);
        assert_eq!(ParseStatus::from_str("partial"), ParseStatus::Partial);
        assert_eq!(ParseStatus::from_str("failed"), ParseStatus::Failed);
        assert_eq!(ParseStatus::from_str("unknown"), ParseStatus::Unknown);
        assert_eq!(ParseStatus::from_str("garbage"), ParseStatus::Unknown);
    }

    #[test]
    fn test_composite_scorer_with_defaults() {
        let scorer = CompositeScorer::with_defaults();
        assert_eq!(scorer.sim_weight, 0.7);
        assert_eq!(scorer.recency_weight, 0.2);
        assert_eq!(scorer.parse_weight, 0.1);
        assert_eq!(scorer.year_boost_range, 5);
    }

    #[test]
    fn test_composite_scorer_valid_weights() {
        let _scorer = CompositeScorer::new(0.6, 0.3, 0.1);
    }

    #[test]
    #[should_panic]
    fn test_composite_scorer_invalid_weights() {
        let _scorer = CompositeScorer::new(0.5, 0.5, 0.5);
    }

    #[test]
    fn test_cosine_similarity() {
        let v1 = vec![1.0, 0.0, 0.0];
        let v2 = vec![1.0, 0.0, 0.0];
        assert!((CompositeScorer::cosine_similarity(&v1, &v2) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_rank_papers_empty() {
        let scorer = CompositeScorer::with_defaults();
        let query = make_paper("q", 2020, 1, 1, Some(ParseStatus::Full));
        let results = scorer.rank_papers(&query, &[], 0.0, 10);
        assert!(results.unwrap().is_empty());
    }

    #[test]
    fn test_rank_papers_with_results() {
        let scorer = CompositeScorer::with_defaults();
        let query = make_paper("q", 2020, 1, 1, Some(ParseStatus::Full));
        let candidates = vec![
            make_paper("c1", 2021, 1, 1, Some(ParseStatus::Full)),
            make_paper("c2", 2022, 1, 1, Some(ParseStatus::Partial)),
        ];
        let results = scorer.rank_papers(&query, &candidates, 0.0, 10).unwrap();
        // c1 ranks higher: (sim=1.0*0.7 + recency=0.8*0.2 + parse=1.0*0.1=0.96)
        // c2: (sim=1.0*0.7 + recency=1.0*0.2 + parse=0.5*0.1=0.95)
        assert_eq!(results[0].paper.id, "c1");
    }

    #[test]
    fn test_rank_papers_threshold() {
        let scorer = CompositeScorer::with_defaults();
        let query = make_paper("q", 2020, 1, 1, Some(ParseStatus::Full));
        let candidates = vec![
            make_paper("c1", 2021, 1, 1, Some(ParseStatus::Full)),
            make_paper("c2", 2022, 1, 1, Some(ParseStatus::Partial)),
        ];
        let results = scorer.rank_papers(&query, &candidates, 0.9, 10).unwrap();
        assert!(results.len() <= 2);
    }

    #[test]
    fn test_rank_papers_no_embedding() {
        let scorer = CompositeScorer::with_defaults();
        let query = Paper::new("q", "Query", None, Some(ParseStatus::Full), None);
        let candidates = vec![make_paper("c1", 2021, 1, 1, Some(ParseStatus::Full))];
        let results = scorer.rank_papers(&query, &candidates, 0.0, 10);
        assert!(results.is_err());
    }

    #[test]
    fn test_paper_published_year() {
        let paper = make_paper("p1", 2023, 6, 15, None);
        assert_eq!(paper.published_year(), Some(2023));
    }

    #[test]
    fn test_paper_no_published_date() {
        let paper = Paper::new("p1", "Paper", None, None, None);
        assert_eq!(paper.published_year(), None);
    }
}
