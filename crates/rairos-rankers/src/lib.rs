//! Rairos Rankers — Paper ranking strategies
#![allow(dead_code)]
//!
//! Implements semantic similarity ranking, composite scoring, and research momentum.
//! Reference: Python rankers/ and scoring/ modules.

use chrono::{DateTime, Utc};
use rairos_core::{cosine_similarity, Database, Paper, ParseStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum RankerError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Paper not found: {0}")]
    PaperNotFound(String),
    #[error("Embedding not found for paper: {0}")]
    NoEmbedding(String),
    #[error("Invalid embedding vector: {0}")]
    InvalidEmbedding(String),
}

pub type Result<T> = std::result::Result<T, RankerError>;

// ============================================================================
// RankedResult
// ============================================================================

/// A paper paired with its ranking score (higher = better).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedResult {
    pub paper: Paper,
    pub score: f32,
}

// ============================================================================
// Ranker Trait
// ============================================================================

/// Abstract base for paper ranking strategies.
pub trait Ranker {
    /// Rank papers similar/related to paper_id.
    ///
    /// - `paper_id`: Query paper ID
    /// - `threshold`: Minimum score to include (default 0.0 = no filter)
    /// - `limit`: Maximum number of results to return
    fn rank(&self, paper_id: &str, threshold: f32, limit: usize) -> Result<Vec<RankedResult>>;
}

// ============================================================================
// CosineSimilarityRanker
// ============================================================================

/// Rank papers by cosine similarity of their embedding vectors.
///
/// Uses ndarray for batch vector operations — O(n) scan with all similarity
/// computation happening in a single vectorized pass.
pub struct CosineSimilarityRanker {
    db: Database,
}

impl CosineSimilarityRanker {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Compute cosine similarity between two vectors.
    fn cosine_similarity(q: &[f32], e: &[f32]) -> f32 {
        cosine_similarity(q, e)
    }
}

impl Ranker for CosineSimilarityRanker {
    fn rank(&self, paper_id: &str, threshold: f32, limit: usize) -> Result<Vec<RankedResult>> {
        let query_emb = self
            .db
            .get_embedding(paper_id)
            .map_err(|_e| RankerError::NoEmbedding(paper_id.to_string()))?;

        let query_emb = match query_emb {
            Some(e) => e,
            None => return Ok(vec![]),
        };

        let paper_ids = self
            .db
            .list_papers_with_embeddings()
            .map_err(|e| RankerError::NoEmbedding(e.to_string()))?;

        let paper_ids: Vec<String> = paper_ids.into_iter().filter(|id| id != paper_id).collect();

        if paper_ids.is_empty() {
            return Ok(vec![]);
        }

        // Compute cosine similarities using ndarray
        let mut scored: Vec<(String, f32)> = Vec::new();
        for pid in &paper_ids {
            if let Ok(Some(emb)) = self.db.get_embedding(pid) {
                let sim = Self::cosine_similarity(&query_emb, &emb);
                if sim >= threshold {
                    scored.push((pid.clone(), sim));
                }
            }
        }

        // Sort by score descending
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        // Build results
        let results: Vec<RankedResult> = scored
            .into_iter()
            .map(|(pid, sim)| {
                let paper = self.db.get_paper(&pid).unwrap_or_else(|_| {
                    Paper::with_metadata(
                        None,
                        format!("Paper {}", pid),
                        String::new(),
                        vec![],
                        vec![],
                        Default::default(),
                    )
                });
                RankedResult { paper, score: sim }
            })
            .collect();

        Ok(results)
    }
}

// ============================================================================
// CompositeScorer
// ============================================================================

/// Rank papers by a weighted combination of:
/// - Cosine similarity of embeddings (semantic overlap)
/// - Recency bonus (newer papers rank higher)
/// - Parse quality bonus (papers with better parse_status rank higher)
pub struct CompositeScorer {
    db: Database,
    sim_weight: f32,
    recency_weight: f32,
    parse_weight: f32,
    year_boost_range: i32,
}

impl CompositeScorer {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            sim_weight: 0.7,
            recency_weight: 0.2,
            parse_weight: 0.1,
            year_boost_range: 5,
        }
    }

    pub fn with_weights(
        db: Database,
        sim_weight: f32,
        recency_weight: f32,
        parse_weight: f32,
    ) -> Self {
        Self {
            db,
            sim_weight,
            recency_weight,
            parse_weight,
            year_boost_range: 5,
        }
    }

    /// Map parse_status to 0-1 quality score.
    fn parse_quality_score(status: &ParseStatus) -> f32 {
        match status {
            ParseStatus::Done => 1.0,
            ParseStatus::Parsing => 0.5,
            ParseStatus::Pending => 0.3,
            ParseStatus::Failed => 0.1,
        }
    }

    /// Get year from DateTime
    fn get_year(dt: &DateTime<Utc>) -> i32 {
        dt.format("%Y").to_string().parse().unwrap_or(2020)
    }
}

impl Ranker for CompositeScorer {
    fn rank(&self, paper_id: &str, threshold: f32, limit: usize) -> Result<Vec<RankedResult>> {
        let cosine_ranker = CosineSimilarityRanker::new(self.db.clone());
        let sim_results = cosine_ranker.rank(paper_id, 0.0, 100)?;

        if sim_results.is_empty() {
            return Ok(vec![]);
        }

        // Determine reference year (most recent paper in results)
        let mut ref_year: i32 = Utc::now().format("%Y").to_string().parse().unwrap_or(2024);
        for result in &sim_results {
            let year = Self::get_year(&result.paper.published);
            if year > ref_year {
                ref_year = year;
            }
        }

        let mut scored: Vec<RankedResult> = Vec::new();
        for result in sim_results {
            // Normalize similarity to [0, 1]
            let sim_norm = result.score.min(1.0);

            // Recency: 0-1 based on distance from ref_year
            let year = Self::get_year(&result.paper.published);
            let year_dist = (ref_year - year).max(0).min(self.year_boost_range) as f32;
            let recency_norm = 1.0 - (year_dist / self.year_boost_range as f32);

            // Parse quality
            let parse_norm = Self::parse_quality_score(&result.paper.parse_status);

            let composite = self.sim_weight * sim_norm
                + self.recency_weight * recency_norm
                + self.parse_weight * parse_norm;

            if composite >= threshold {
                scored.push(RankedResult {
                    paper: result.paper,
                    score: composite,
                });
            }
        }

        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(limit);
        Ok(scored)
    }
}

// ============================================================================
// ResearchMomentum
// ============================================================================

/// Research momentum / paper importance scoring.
///
/// Formula:
///   score = citation_score * 0.3
///         + tag_popularity * 0.25
///         + recency_boost * 0.2
///         + novelty_factor * 0.15
///         + radar_heat * 0.1
/// All components are normalised to [0, 100].
pub struct ResearchMomentum {
    db: Database,
    scores_cache: HashMap<String, f32>,
    radar_data: HashMap<String, RadarEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct RadarEntry {
    score: f32,
}

impl ResearchMomentum {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            scores_cache: HashMap::new(),
            radar_data: HashMap::new(),
        }
    }

    /// Load radar data from file (stub implementation).
    fn load_radar(&mut self) {
        // In a real implementation, this would load from data/radar.json
        // For now, we use an empty radar data structure
        self.radar_data = HashMap::new();
    }

    /// Compute momentum score for a paper (cached).
    pub fn score_paper(&mut self, paper_uid: &str) -> f32 {
        if let Some(&score) = self.scores_cache.get(paper_uid) {
            return score;
        }

        let paper = match self.db.get_paper(paper_uid) {
            Ok(p) => p,
            Err(_) => return 0.0,
        };

        let score = self.compute_paper_score(&paper);
        self.scores_cache.insert(paper_uid.to_string(), score);
        score
    }

    /// Compute the full momentum score for a paper.
    fn compute_paper_score(&mut self, paper: &Paper) -> f32 {
        self.load_radar();

        let year = paper
            .published
            .format("%Y")
            .to_string()
            .parse::<i32>()
            .unwrap_or(2020);
        let now = Utc::now()
            .format("%Y")
            .to_string()
            .parse::<i32>()
            .unwrap_or(2024);
        let age = (now - year).max(0);

        // 1. Citation score (30%) — log scale
        let cited_by = paper.metadata.cited_by as f32;
        let citation_score = if cited_by > 0.0 {
            ((cited_by.ln_1p() / 5.0f32.ln()) * 100.0).min(100.0)
        } else {
            0.0
        };

        // 2. Tag popularity (25%) — we use categories as proxy
        let tag_popularity = 50.0; // default middle
        let _tag_count = paper.categories.len() as f32;

        // 3. Recency boost (20%) — exponential decay
        let recency_boost = (if age > 0 {
            (-(age as f32) / 5.0).exp()
        } else {
            1.0
        }) * 20.0;

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

    /// Get top papers by momentum score.
    pub fn get_top_papers(&mut self, top_n: usize) -> Vec<(String, f32)> {
        let papers = self.db.list_papers(None, 1000, 0).unwrap_or_default();
        let mut scored: Vec<(String, f32)> = papers
            .iter()
            .map(|p| {
                let score = self.score_paper(&p.id);
                (p.id.clone(), score)
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_n);
        scored
    }

    /// Score a tag (category) by aggregate paper momentum.
    pub fn score_tag(&mut self, tag: &str) -> TagScore {
        let papers = self.db.list_papers(None, 1000, 0).unwrap_or_default();
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

        let papers_scores: Vec<f32> = tag_papers.iter().map(|p| self.score_paper(&p.id)).collect();
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
            raw_score,
            papers_count: tag_papers.len(),
            avg_citation: avg_cite,
            heat_trend: heat_trend.to_string(),
            momentum_label: momentum_label.to_string(),
        }
    }

    /// Get tag leaderboard — all tags ranked by momentum score.
    pub fn get_tag_leaderboard(&mut self) -> Vec<(String, f32)> {
        let papers = self.db.list_papers(None, 1000, 0).unwrap_or_default();
        let mut tags: HashMap<String, Vec<f32>> = HashMap::new();

        for paper in &papers {
            for cat in &paper.categories {
                let score = self.score_paper(&paper.id);
                tags.entry(cat.clone()).or_default().push(score);
            }
        }

        let mut scored: Vec<(String, f32)> = tags
            .into_iter()
            .map(|(tag, scores)| {
                let avg = scores.iter().sum::<f32>() / scores.len() as f32;
                (tag, avg)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored
    }

    /// Recompute all scores from scratch.
    pub fn refresh_all(&mut self) {
        self.scores_cache.clear();
        let papers = self.db.list_papers(None, 10000, 0).unwrap_or_default();
        for paper in papers {
            self.score_paper(&paper.id);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagScore {
    pub raw_score: f32,
    pub papers_count: usize,
    pub avg_citation: f32,
    pub heat_trend: String,
    pub momentum_label: String,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let v1 = &[1.0, 0.0, 0.0];
        let v2 = &[1.0, 0.0, 0.0];
        let v3 = &[0.0, 1.0, 0.0];

        assert!((CosineSimilarityRanker::cosine_similarity(v1, v2) - 1.0).abs() < 1e-6);
        assert!((CosineSimilarityRanker::cosine_similarity(v1, v3)).abs() < 1e-6);
    }

    #[test]
    fn test_parse_quality_score() {
        assert!((CompositeScorer::parse_quality_score(&ParseStatus::Done) - 1.0).abs() < 1e-6);
        assert!((CompositeScorer::parse_quality_score(&ParseStatus::Parsing) - 0.5).abs() < 1e-6);
        assert!((CompositeScorer::parse_quality_score(&ParseStatus::Pending) - 0.3).abs() < 1e-6);
        assert!((CompositeScorer::parse_quality_score(&ParseStatus::Failed) - 0.1).abs() < 1e-6);
    }
}
