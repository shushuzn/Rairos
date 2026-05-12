//! rairos-impact-scorer — Paper Impact Scorer for AI Research OS.
//!
//! Ported from `llm/impact_scorer.py` (243 LOC, pure stdlib).
//!
//! Composite influence scoring combining normalized citations, PageRank,
//! citation momentum, and author h-index.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Data Structures ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactScore {
    pub paper_id: String,
    pub title: String,
    pub year: i32,
    #[serde(default)]
    pub raw_citations: i32,
    #[serde(default)]
    pub normalized_score: f64,
    #[serde(default)]
    pub pagerank_score: f64,
    #[serde(default)]
    pub momentum_score: f64,
    #[serde(default)]
    pub author_h_index: f64,
    #[serde(default)]
    pub composite_score: f64,
    #[serde(default)]
    pub percentile: f64,
    #[serde(default)]
    pub tier: String,
}

impl ImpactScore {
    pub fn to_dict(&self) -> serde_json::Value {
        serde_json::json!({
            "paper_id": self.paper_id,
            "title": self.title,
            "year": self.year,
            "raw_citations": self.raw_citations,
            "normalized_score": (self.normalized_score * 1000.0).round() / 1000.0,
            "pagerank_score": (self.pagerank_score * 1000.0).round() / 1000.0,
            "momentum_score": (self.momentum_score * 1000.0).round() / 1000.0,
            "author_h_index": (self.author_h_index * 1000.0).round() / 1000.0,
            "composite_score": (self.composite_score * 1000.0).round() / 1000.0,
            "percentile": (self.percentile * 10.0).round() / 10.0,
            "tier": self.tier,
        })
    }
}

// ─── Impact Scorer ────────────────────────────────────────────────────────────

const WEIGHT_NORMALIZED: f64 = 0.30;
const WEIGHT_PAGERANK: f64 = 0.30;
const WEIGHT_MOMENTUM: f64 = 0.25;
const WEIGHT_AUTHOR: f64 = 0.15;

const TIER_THRESHOLDS: &[(&str, f64)] = &[("S", 0.8), ("A", 0.6), ("B", 0.4), ("C", 0.2)];

pub struct ImpactScorer {
    scores: HashMap<String, ImpactScore>,
    pagerank_iterations: usize,
}

impl Default for ImpactScorer {
    fn default() -> Self {
        Self::new()
    }
}

impl ImpactScorer {
    pub fn new() -> Self {
        Self {
            scores: HashMap::new(),
            pagerank_iterations: 4,
        }
    }

    fn normalize(&self, value: f64, baseline: f64) -> f64 {
        1.0 - (1.0 / (1.0 + value / baseline))
    }

    fn tier(&self, composite: f64) -> String {
        for (t, threshold) in TIER_THRESHOLDS {
            if composite >= *threshold {
                return t.to_string();
            }
        }
        "D".to_string()
    }

    fn compute_pagerank(&self, paper_id: &str, citing_papers: &[serde_json::Value]) -> f64 {
        if citing_papers.is_empty() {
            return 0.1;
        }

        let mut scores: HashMap<String, f64> = citing_papers
            .iter()
            .filter_map(|p| p.get("paper_id").and_then(|v| v.as_str()).map(|s| (s.to_string(), 1.0)))
            .collect();

        let damping = 0.85;

        for _ in 0..self.pagerank_iterations {
            let mut new_scores: HashMap<String, f64> = HashMap::new();
            for (pid, score) in &scores {
                if self.scores.contains_key(pid) {
                    let inherited = *score * damping;
                    *new_scores.entry(pid.clone()).or_insert(0.0) += inherited;
                }
            }
            let total: f64 = new_scores.values().sum();
            if total > 0.0 {
                for (k, v) in &mut new_scores {
                    *v /= total;
                }
                scores = new_scores;
            }
        }

        scores.values().sum()
    }

    pub fn score_paper(
        &mut self,
        paper_id: &str,
        title: &str,
        year: i32,
        raw_citations: i32,
        citing_papers: Option<Vec<serde_json::Value>>,
        author_h_index: f64,
    ) -> ImpactScore {
        let current_year = 2026;

        let age = (current_year - year).max(1);
        let normalized = raw_citations as f64 / age as f64;

        let citing = citing_papers.unwrap_or_default();
        let pagerank = self.compute_pagerank(paper_id, &citing);

        let momentum = raw_citations as f64 / (age as f64).powf(0.7);

        let composite = WEIGHT_NORMALIZED * self.normalize(normalized, 10.0)
            + WEIGHT_PAGERANK * pagerank
            + WEIGHT_MOMENTUM * self.normalize(momentum, 10.0)
            + WEIGHT_AUTHOR * (author_h_index / 50.0).min(1.0);

        let tier = self.tier(composite);

        let score = ImpactScore {
            paper_id: paper_id.to_string(),
            title: title.to_string(),
            year,
            raw_citations,
            normalized_score: (normalized * 1000.0).round() / 1000.0,
            pagerank_score: (pagerank * 1000.0).round() / 1000.0,
            momentum_score: (momentum * 1000.0).round() / 1000.0,
            author_h_index: (author_h_index * 1000.0).round() / 1000.0,
            composite_score: (composite * 1000.0).round() / 1000.0,
            percentile: 0.0,
            tier,
        };

        self.scores.insert(paper_id.to_string(), score.clone());
        score
    }

    pub fn score_batch(&mut self, papers: &[serde_json::Value]) -> Vec<ImpactScore> {
        let mut results = Vec::new();

        for p in papers {
            let paper_id = p.get("paper_id").and_then(|v| v.as_str()).unwrap_or("");
            let title = p.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let year = p.get("year").and_then(|v| v.as_i64()).unwrap_or(2020) as i32;
            let raw_citations = p.get("citation_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let author_h_index = p.get("author_h_index").and_then(|v| v.as_f64()).unwrap_or(0.0);

            let score = self.score_paper(paper_id, title, year, raw_citations, None, author_h_index);
            results.push(score);
        }

        self.assign_percentiles(&mut results);
        results
    }

    fn assign_percentiles(&self, scores: &mut [ImpactScore]) {
        if scores.is_empty() {
            return;
        }
        scores.sort_by(|a, b| {
            b.composite_score.partial_cmp(&a.composite_score).unwrap_or(std::cmp::Ordering::Equal)
        });
        let n = scores.len();
        for (i, s) in scores.iter_mut().enumerate() {
            s.percentile = ((n - i) as f64 / n as f64) * 100.0;
        }
    }

    pub fn get_top_papers(
        &mut self,
        papers: &[serde_json::Value],
        limit: usize,
        min_score: f64,
        year_filter: Option<i32>,
    ) -> Vec<ImpactScore> {
        let scored = self.score_batch(papers);
        let mut filtered: Vec<_> = scored
            .into_iter()
            .filter(|s| s.composite_score >= min_score)
            .collect();
        if let Some(yf) = year_filter {
            filtered.retain(|s| s.year >= yf);
        }
        filtered.sort_by(|a, b| {
            b.composite_score.partial_cmp(&a.composite_score).unwrap_or(std::cmp::Ordering::Equal)
        });
        filtered.truncate(limit);
        filtered
    }

    pub fn rank_papers(&mut self, papers: &[serde_json::Value], top_k: usize) -> Vec<serde_json::Value> {
        let scored = self.score_batch(papers);
        let mut sorted = scored;
        sorted.sort_by(|a, b| {
            b.composite_score.partial_cmp(&a.composite_score).unwrap_or(std::cmp::Ordering::Equal)
        });
        let top: Vec<_> = sorted
            .into_iter()
            .take(top_k)
            .enumerate()
            .map(|(i, s)| {
                let why = self.explain_score(&s);
                let d = s.to_dict();
                serde_json::json!({
                    "rank": i + 1,
                    "paper_id": d["paper_id"],
                    "title": d["title"],
                    "year": d["year"],
                    "raw_citations": d["raw_citations"],
                    "normalized_score": d["normalized_score"],
                    "pagerank_score": d["pagerank_score"],
                    "momentum_score": d["momentum_score"],
                    "author_h_index": d["author_h_index"],
                    "composite_score": d["composite_score"],
                    "percentile": d["percentile"],
                    "tier": d["tier"],
                    "why": why,
                })
            })
            .collect();
        top
    }

    fn explain_score(&self, s: &ImpactScore) -> String {
        let mut parts: Vec<String> = vec![];
        if s.normalized_score > 0.5 {
            parts.push(format!("高年化引用 ({:.1}/年)", s.normalized_score));
        }
        if s.pagerank_score > 0.3 {
            parts.push("被高影响力论文引用".to_string());
        }
        if s.momentum_score > 0.4 {
            parts.push("引用增长强劲".to_string());
        }
        if s.author_h_index > 30.0 {
            parts.push(format!("作者H指数高 ({:.0})", s.author_h_index));
        }
        if parts.is_empty() {
            "综合评分".to_string()
        } else {
            parts.join(", ")
        }
    }

    pub fn render_ranking(&self, ranking: &[serde_json::Value]) -> String {
        if ranking.is_empty() {
            return "No papers to rank.".to_string();
        }

        let mut lines = vec![
            "=".repeat(70),
            "📊 Paper Impact Ranking".to_string(),
            "=".repeat(70),
            "".to_string(),
        ];
        lines.push(format!(
            "{:<6}{:<6}{:<8}{:<12}{:<6} Title",
            "Rank", "Tier", "Score", "Citations", "Year"
        ));
        lines.push("-".repeat(70));

        for entry in ranking {
            let tier_emoji = HashMap::from([
                ("S", "⭐"),
                ("A", "🅰️"),
                ("B", "🅱️"),
                ("C", "⚙️"),
                ("D", "📄"),
            ]);
            let tier_str = entry.get("tier").and_then(|v| v.as_str()).unwrap_or("D");
            let emoji = tier_emoji.get(tier_str).unwrap_or(&"📄");
            let title = entry.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let title_short = if title.len() > 40 { &title[..40] } else { title };
            lines.push(format!(
                "{:<6}{:<6}{:<8.3}{:<12}{:<6}",
                entry.get("rank").and_then(|v| v.as_i64()).unwrap_or(0),
                emoji,
                entry.get("composite_score").and_then(|v| v.as_f64()).unwrap_or(0.0),
                entry.get("raw_citations").and_then(|v| v.as_i64()).unwrap_or(0),
                entry.get("year").and_then(|v| v.as_i64()).unwrap_or(0),
            ) + title_short);
        }

        lines.push("=".repeat(70));
        lines.join("\n")
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_impact_score_to_dict() {
        let s = ImpactScore {
            paper_id: "p1".to_string(),
            title: "Test paper".to_string(),
            year: 2023,
            raw_citations: 50,
            normalized_score: 0.8,
            pagerank_score: 0.5,
            momentum_score: 0.6,
            author_h_index: 40.0,
            composite_score: 0.7,
            percentile: 85.0,
            tier: "B".to_string(),
        };
        let d = s.to_dict();
        assert_eq!(d["paper_id"], "p1");
        assert_eq!(d["tier"], "B");
    }

    #[test]
    fn test_tier_assignment() {
        let mut scorer = ImpactScorer::new();
        assert_eq!(scorer.tier(0.9), "S");
        assert_eq!(scorer.tier(0.7), "A");
        assert_eq!(scorer.tier(0.5), "B");
        assert_eq!(scorer.tier(0.3), "C");
        assert_eq!(scorer.tier(0.1), "D");
    }

    #[test]
    fn test_normalize() {
        let scorer = ImpactScorer::new();
        let result = scorer.normalize(10.0, 10.0);
        assert!((result - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_score_paper_basic() {
        let mut scorer = ImpactScorer::new();
        let score = scorer.score_paper(
            "p1", "Test", 2023, 50, None, 30.0,
        );
        assert_eq!(score.paper_id, "p1");
        assert!(score.composite_score > 0.0);
    }

    #[test]
    fn test_score_batch_empty() {
        let mut scorer = ImpactScorer::new();
        let result = scorer.score_batch(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_render_ranking_empty() {
        let scorer = ImpactScorer::new();
        let result = scorer.render_ranking(&[]);
        assert_eq!(result, "No papers to rank.");
    }

    #[test]
    fn test_tier_thresholds() {
        let scorer = ImpactScorer::new();
        for (tier, threshold) in TIER_THRESHOLDS {
            let score = ImpactScore {
                paper_id: "".to_string(),
                title: "".to_string(),
                year: 2020,
                raw_citations: 0,
                normalized_score: 0.0,
                pagerank_score: 0.0,
                momentum_score: 0.0,
                author_h_index: 0.0,
                composite_score: *threshold,
                percentile: 0.0,
                tier: "".to_string(),
            };
            assert_eq!(scorer.tier(score.composite_score), *tier);
        }
    }
}
