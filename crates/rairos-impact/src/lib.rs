//! Rairos Impact — Paper Impact Scorer
//!
//! Composite influence scoring: age-normalized citations, PageRank-style propagation,
//! citation velocity, and author h-index aggregation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const CURRENT_YEAR: i32 = 2026;
const PAGERANK_ITERATIONS: usize = 4;
const PAGERANK_DAMPING: f64 = 0.85;

const WEIGHT_NORMALIZED: f64 = 0.30;
const WEIGHT_PAGERANK: f64 = 0.30;
const WEIGHT_MOMENTUM: f64 = 0.25;
const WEIGHT_AUTHOR: f64 = 0.15;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactScore {
    pub paper_id: String,
    pub title: String,
    pub year: i32,
    pub raw_citations: i32,
    pub normalized_score: f64,
    pub pagerank_score: f64,
    pub momentum_score: f64,
    pub author_h_index: f64,
    pub composite_score: f64,
    pub percentile: f64,
    pub tier: String,
}

pub struct ImpactScorer {
    scores: HashMap<String, ImpactScore>,
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
        }
    }

    fn normalize(&self, value: f64, baseline: f64) -> f64 {
        1.0 - (1.0 / (1.0 + value / baseline))
    }

    fn tier(&self, composite: f64) -> String {
        if composite >= 0.8 {
            "S".to_string()
        } else if composite >= 0.6 {
            "A".to_string()
        } else if composite >= 0.4 {
            "B".to_string()
        } else if composite >= 0.2 {
            "C".to_string()
        } else {
            "D".to_string()
        }
    }

    fn compute_pagerank(&self, _paper_id: &str, citing_papers: &[String]) -> f64 {
        if citing_papers.is_empty() {
            return 0.1;
        }

        let mut scores: HashMap<String, f64> = citing_papers
            .iter()
            .map(|pid| (pid.clone(), 1.0))
            .collect();

        for _ in 0..PAGERANK_ITERATIONS {
            let mut new_scores: HashMap<String, f64> = HashMap::new();
            for (pid, score) in &scores {
                if self.scores.contains_key(pid) {
                    let inherited = score * PAGERANK_DAMPING;
                    *new_scores.entry(pid.clone()).or_insert(0.0) += inherited;
                }
            }
            let total: f64 = new_scores.values().sum();
            if total > 0.0 {
                for v in new_scores.values_mut() {
                    *v /= total;
                }
            }
            scores = new_scores;
        }

        scores.values().sum()
    }

    fn assign_percentiles(&self, results: &mut [ImpactScore]) {
        if results.is_empty() {
            return;
        }
        results.sort_by(|a, b| {
            b.composite_score
                .partial_cmp(&a.composite_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let n = results.len();
        for (i, s) in results.iter_mut().enumerate() {
            s.percentile = ((n - i) as f64 / n as f64) * 100.0;
        }
    }

    pub fn score_paper(
        &mut self,
        paper_id: &str,
        title: &str,
        year: i32,
        raw_citations: i32,
        citing_papers: &[String],
        author_h_index: f64,
    ) -> ImpactScore {
        let age = (CURRENT_YEAR - year).max(1);

        let normalized = raw_citations as f64 / age as f64;
        let pagerank = self.compute_pagerank(paper_id, citing_papers);
        let momentum = raw_citations as f64 / (age as f64).powf(0.7);

        let composite = WEIGHT_NORMALIZED * self.normalize(normalized, 10.0)
            + WEIGHT_PAGERANK * pagerank
            + WEIGHT_MOMENTUM * self.normalize(momentum, 10.0)
            + WEIGHT_AUTHOR * (author_h_index / 50.0).min(1.0);

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
            tier: self.tier(composite),
        };

        self.scores.insert(paper_id.to_string(), score.clone());
        score
    }

    pub fn score_batch(&mut self, papers: &[[&str; 3]], citations: i32, author_h_index: f64) -> Vec<ImpactScore> {
        let mut results: Vec<ImpactScore> = Vec::new();
        for p in papers {
            let score = self.score_paper(p[0], p[1], p[2].parse().unwrap_or(2020), citations, &[], author_h_index);
            results.push(score);
        }
        self.assign_percentiles(&mut results);
        results
    }

    pub fn render_ranking(&self, ranking: &[[i32; 6]]) -> String {
        if ranking.is_empty() {
            return "No papers to rank.".to_string();
        }

        let tier_emoji: HashMap<i32, &str> = [
            (0, "⭐"),
            (1, "🅰️"),
            (2, "🅱️"),
            (3, "⚙️"),
            (4, "📄"),
        ]
        .into_iter()
        .collect();

        let mut lines = Vec::new();
        lines.push("=".repeat(70));
        lines.push("📊 Paper Impact Ranking".to_string());
        lines.push("=".repeat(70));
        lines.push(String::new());
        lines.push(format!(
            "{:<6}{:<6}{:<8}{:<12}{:<6} Title",
            "Rank", "Tier", "Score", "Citations", "Year"
        ));
        lines.push("-".repeat(70));

        for (i, entry) in ranking.iter().enumerate() {
            let tier = entry[0];
            let emoji = tier_emoji.get(&tier).unwrap_or(&"📄");
            lines.push(format!(
                "{:<6}{:<6}{:<8.3}{:<12}{:<6}",
                i + 1,
                emoji,
                entry[1] as f64 / 1000.0,
                entry[2],
                entry[3]
            ));
        }

        lines.push("=".repeat(70));
        lines.join("\n")
    }

    pub fn to_dict(&self, score: &ImpactScore) -> HashMap<String, serde_json::Value> {
        let mut m = HashMap::new();
        m.insert("paper_id".to_string(), serde_json::json!(score.paper_id));
        m.insert("title".to_string(), serde_json::json!(score.title));
        m.insert("year".to_string(), serde_json::json!(score.year));
        m.insert("raw_citations".to_string(), serde_json::json!(score.raw_citations));
        m.insert("normalized_score".to_string(), serde_json::json!(score.normalized_score));
        m.insert("pagerank_score".to_string(), serde_json::json!(score.pagerank_score));
        m.insert("momentum_score".to_string(), serde_json::json!(score.momentum_score));
        m.insert("author_h_index".to_string(), serde_json::json!(score.author_h_index));
        m.insert("composite_score".to_string(), serde_json::json!(score.composite_score));
        m.insert("percentile".to_string(), serde_json::json!(score.percentile));
        m.insert("tier".to_string(), serde_json::json!(score.tier));
        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_assignment() {
        let scorer = ImpactScorer::new();
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
        assert!((result - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_score_paper() {
        let mut scorer = ImpactScorer::new();
        let score = scorer.score_paper("p1", "Test Paper", 2023, 50, &[], 25.0);
        assert_eq!(score.paper_id, "p1");
        assert_eq!(score.year, 2023);
        assert_eq!(score.raw_citations, 50);
        assert!(score.composite_score > 0.0);
    }

    #[test]
    fn test_score_batch() {
        let mut scorer = ImpactScorer::new();
        let papers = [
            ["p1", "Paper A", "2023"],
            ["p2", "Paper B", "2022"],
            ["p3", "Paper C", "2021"],
        ];
        let results = scorer.score_batch(&papers, 30, 20.0);
        assert_eq!(results.len(), 3);
        assert!(results[0].percentile >= results[1].percentile);
    }

    #[test]
    fn test_pagerank_no_citations() {
        let scorer = ImpactScorer::new();
        let score = scorer.compute_pagerank("p1", &[]);
        assert!((score - 0.1).abs() < 0.01);
    }

    #[test]
    fn test_empty_ranking() {
        let scorer = ImpactScorer::new();
        let ranking: [[i32; 6]; 0] = [];
        let rendered = scorer.render_ranking(&ranking);
        assert_eq!(rendered, "No papers to rank.");
    }
}
