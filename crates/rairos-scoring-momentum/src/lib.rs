//! Rairos Scoring Momentum — Research momentum / paper importance scoring
//!
//! Reference: Python scoring/momentum.py
//!
//! Formula:
//!   score = citation_score * 0.3
//!         + tag_popularity * 0.25
//!         + recency_boost * 0.2
//!         + novelty_factor * 0.15
//!         + radar_heat * 0.1
//! All components are normalised to [0, 100].

use chrono::{Datelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum MomentumError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Paper not found: {0}")]
    PaperNotFound(String),
}

/// Result type alias
pub type Result<T> = std::result::Result<T, MomentumError>;

// ============================================================================
// Radar Entry
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
struct RadarEntry {
    score: f32,
}

// ============================================================================
// Paper Metadata (simplified)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct PaperMetadata {
    pub cited_by: usize,
    pub references: usize,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paper {
    pub id: String,
    pub title: String,
    pub year: i32,
    pub categories: Vec<String>,
    pub metadata: PaperMetadata,
}

impl Paper {
    pub fn new(id: &str, title: &str, year: i32, categories: Vec<String>) -> Self {
        Self {
            id: id.to_string(),
            title: title.to_string(),
            year,
            categories,
            metadata: Default::default(),
        }
    }
}

// ============================================================================
// Tag Score
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagScore {
    pub raw_score: f32,
    pub papers_count: usize,
    pub avg_citation: f32,
    pub heat_trend: String,
    pub momentum_label: String,
}

// ============================================================================
// Research Momentum
// ============================================================================

/// Compute research momentum scores for papers and tags.
pub struct ResearchMomentum {
    scores_cache: HashMap<String, f32>,
    radar_data: HashMap<String, RadarEntry>,
}

impl ResearchMomentum {
    /// Create a new ResearchMomentum scorer
    pub fn new() -> Self {
        Self {
            scores_cache: HashMap::new(),
            radar_data: HashMap::new(),
        }
    }

    /// Load radar data from file (searches data/radar.json and radar.json)
    pub fn load_radar(&mut self) {
        let candidates = vec!["data/radar.json", "radar.json"];
        for path in candidates {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(data) = serde_json::from_str::<HashMap<String, RadarEntry>>(&content) {
                    self.radar_data = data;
                    return;
                }
            }
        }
    }

    /// Compute momentum score for a paper (cached)
    pub fn score_paper(&mut self, paper: &Paper) -> f32 {
        let paper_id = &paper.id;
        if let Some(&score) = self.scores_cache.get(paper_id) {
            return score;
        }

        let score = self.compute_score(paper);
        self.scores_cache.insert(paper_id.clone(), score);
        score
    }

    /// Compute the full momentum score for a paper
    fn compute_score(&mut self, paper: &Paper) -> f32 {
        // Load radar if not already loaded
        if self.radar_data.is_empty() {
            self.load_radar();
        }

        let now = Utc::now().year();
        let age = (now - paper.year).max(0);

        // 1. Citation score (30%) — log scale
        let cited_by = paper.metadata.cited_by as f32;
        let citation_score = if cited_by > 0.0 {
            ((cited_by.ln_1p() / 5.0f32.ln()) * 100.0).min(100.0)
        } else {
            0.0
        };

        // 2. Tag popularity (25%) — based on category count
        let tag_popularity = if paper.categories.is_empty() {
            50.0
        } else {
            // Simplified: use category count as proxy for tag popularity
            (paper.categories.len() as f32 * 10.0).min(100.0)
        };

        // 3. Recency boost (20%) — exponential decay
        let recency_boost = if age > 0 {
            (-(age as f32) / 5.0).exp() * 20.0
        } else {
            20.0
        };

        // 4. Novelty factor (15%) — papers that are cited but cite few others
        let novelty_factor = if paper.metadata.references > 0 {
            (cited_by / paper.metadata.references.max(1) as f32 * 10.0).min(15.0)
        } else if cited_by > 0.0 {
            15.0 // originator paper — cited but cites no one
        } else {
            0.0
        };

        // 5. Radar heat (10%) — based on top category
        let mut radar_heat = 0.0f32;
        if let Some(top_cat) = paper.categories.first() {
            if let Some(entry) = self.radar_data.get(top_cat) {
                radar_heat = entry.score;
            }
        }

        let total = citation_score * 0.30
            + tag_popularity * 0.25
            + recency_boost * 0.20
            + novelty_factor * 0.15
            + radar_heat * 0.10;

        total.min(100.0)
    }

    /// Score a tag by aggregate paper momentum
    pub fn score_tag(&mut self, tag: &str, papers: &[Paper]) -> TagScore {
        let tag_papers: Vec<&Paper> = papers
            .iter()
            .filter(|p| p.categories.contains(&tag.to_string()))
            .collect();

        if tag_papers.is_empty() {
            return TagScore {
                raw_score: 0.0,
                papers_count: 0,
                avg_citation: 0.0,
                heat_trend: "unknown".to_string(),
                momentum_label: "niche".to_string(),
            };
        }

        let papers_scores: Vec<f32> = tag_papers.iter().map(|p| self.score_paper(p)).collect();
        let raw_score = papers_scores.iter().sum::<f32>() / papers_scores.len() as f32;
        let citations: usize = tag_papers.iter().map(|p| p.metadata.cited_by).sum();
        let avg_cite = citations as f32 / tag_papers.len() as f32;

        let heat = self.radar_data.get(tag).map(|e| e.score).unwrap_or(0.0);
        let (heat_trend, momentum_label) = if heat > 70.0 {
            ("rising", "hot")
        } else if heat > 40.0 {
            ("stable", "established")
        } else if heat > 0.0 {
            ("declining", "maturing")
        } else {
            ("unknown", "niche")
        };

        TagScore {
            raw_score: (raw_score * 100.0).round() / 100.0,
            papers_count: tag_papers.len(),
            avg_citation: (avg_cite * 100.0).round() / 100.0,
            heat_trend: heat_trend.to_string(),
            momentum_label: momentum_label.to_string(),
        }
    }

    /// Get tag leaderboard — all tags ranked by momentum score
    pub fn get_tag_leaderboard(&mut self, papers: &[Paper]) -> Vec<(String, f32)> {
        let mut tags: HashMap<String, Vec<f32>> = HashMap::new();

        for paper in papers {
            for cat in &paper.categories {
                let score = self.score_paper(paper);
                tags.entry(cat.clone()).or_default().push(score);
            }
        }

        let mut scored: Vec<(String, f32)> = tags
            .into_iter()
            .map(|(tag, scores)| {
                let avg = scores.iter().sum::<f32>() / scores.len() as f32;
                (tag, (avg * 100.0).round() / 100.0)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored
    }

    /// Get top papers by momentum score
    pub fn get_top_papers(&mut self, papers: &[Paper], top_n: usize) -> Vec<(String, f32)> {
        let mut scored: Vec<(String, f32)> = papers
            .iter()
            .map(|p| {
                let score = self.score_paper(p);
                (p.id.clone(), (score * 100.0).round() / 100.0)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_n);
        scored
    }

    /// Recompute all scores from scratch
    pub fn refresh_all(&mut self, papers: &[Paper]) {
        self.scores_cache.clear();
        for paper in papers {
            self.score_paper(paper);
        }
    }

    /// Clear the scores cache
    pub fn clear_cache(&mut self) {
        self.scores_cache.clear();
    }
}

impl Default for ResearchMomentum {
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

    fn make_paper(
        id: &str,
        year: i32,
        categories: Vec<&str>,
        cited_by: usize,
        references: usize,
    ) -> Paper {
        Paper {
            id: id.to_string(),
            title: format!("Paper {}", id),
            year,
            categories: categories.into_iter().map(|s| s.to_string()).collect(),
            metadata: PaperMetadata {
                cited_by,
                references,
            },
        }
    }

    #[test]
    fn test_score_paper_zero_citations() {
        let mut momentum = ResearchMomentum::new();
        let paper = make_paper("p1", 2020, vec!["cs.AI"], 0, 0);
        let score = momentum.score_paper(&paper);
        // Should have recency boost but no citation/novelty
        assert!(score >= 0.0);
        assert!(score <= 100.0);
    }

    #[test]
    fn test_score_paper_with_citations() {
        let mut momentum = ResearchMomentum::new();
        let paper = make_paper("p2", 2020, vec!["cs.AI"], 100, 50);
        let score = momentum.score_paper(&paper);
        assert!(score > 0.0);
        assert!(score <= 100.0);
    }

    #[test]
    fn test_score_paper_caching() {
        let mut momentum = ResearchMomentum::new();
        let paper = make_paper("p3", 2020, vec!["cs.AI"], 50, 20);

        let score1 = momentum.score_paper(&paper);
        let score2 = momentum.score_paper(&paper); // Should be cached

        assert_eq!(score1, score2);
        assert_eq!(momentum.scores_cache.len(), 1);
    }

    #[test]
    fn test_score_tag_empty() {
        let mut momentum = ResearchMomentum::new();
        let papers = vec![];
        let result = momentum.score_tag("cs.AI", &papers);

        assert_eq!(result.raw_score, 0.0);
        assert_eq!(result.papers_count, 0);
        assert_eq!(result.heat_trend, "unknown");
        assert_eq!(result.momentum_label, "niche");
    }

    #[test]
    fn test_score_tag_with_papers() {
        let mut momentum = ResearchMomentum::new();
        let papers = vec![
            make_paper("p1", 2020, vec!["cs.AI"], 10, 5),
            make_paper("p2", 2021, vec!["cs.AI"], 20, 10),
        ];
        let result = momentum.score_tag("cs.AI", &papers);

        assert_eq!(result.papers_count, 2);
        assert!(result.raw_score >= 0.0);
    }

    #[test]
    fn test_get_top_papers() {
        let mut momentum = ResearchMomentum::new();
        let papers = vec![
            make_paper("p1", 2020, vec!["cs.AI"], 5, 0),
            make_paper("p2", 2021, vec!["cs.AI"], 50, 0),
            make_paper("p3", 2022, vec!["cs.AI"], 100, 0),
        ];

        let top = momentum.get_top_papers(&papers, 2);
        assert_eq!(top.len(), 2);
        // Paper p3 has most citations, should be first
        assert_eq!(top[0].0, "p3");
    }

    #[test]
    fn test_get_tag_leaderboard() {
        let mut momentum = ResearchMomentum::new();
        let papers = vec![
            make_paper("p1", 2020, vec!["cs.AI", "cs.LG"], 10, 0),
            make_paper("p2", 2021, vec!["cs.AI"], 20, 0),
            make_paper("p3", 2022, vec!["cs.LG"], 30, 0),
        ];

        let leaderboard = momentum.get_tag_leaderboard(&papers);
        assert!(leaderboard.len() >= 2);

        // cs.LG should have higher score (papers with more citations)
        let cs_lg_score = leaderboard
            .iter()
            .find(|(t, _)| *t == "cs.LG")
            .map(|(_, s)| *s);
        let cs_ai_score = leaderboard
            .iter()
            .find(|(t, _)| *t == "cs.AI")
            .map(|(_, s)| *s);

        if let (Some(lg), Some(ai)) = (cs_lg_score, cs_ai_score) {
            assert!(lg >= ai);
        }
    }

    #[test]
    fn test_refresh_all_clears_cache() {
        let mut momentum = ResearchMomentum::new();
        let papers = vec![make_paper("p1", 2020, vec!["cs.AI"], 10, 0)];

        momentum.score_paper(&papers[0]); // Populate cache
        assert_eq!(momentum.scores_cache.len(), 1);

        momentum.refresh_all(&papers);
        assert_eq!(momentum.scores_cache.len(), 1); // Still 1, but recomputed
    }

    #[test]
    fn test_recency_boost_newer_papers() {
        let mut momentum = ResearchMomentum::new();
        let now = Utc::now().year();

        let paper_old = make_paper("old", now - 10, vec!["cs.AI"], 0, 0);
        let paper_new = make_paper("new", now, vec!["cs.AI"], 0, 0);

        let score_old = momentum.score_paper(&paper_old);
        let score_new = momentum.score_paper(&paper_new);

        // Newer papers should have higher recency boost
        assert!(score_new > score_old);
    }
}
