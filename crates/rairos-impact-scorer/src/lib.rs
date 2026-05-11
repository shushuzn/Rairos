use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

impl ImpactScore {
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut m = HashMap::new();
        m.insert("paper_id".to_string(), serde_json::json!(self.paper_id));
        m.insert("title".to_string(), serde_json::json!(self.title));
        m.insert("year".to_string(), serde_json::json!(self.year));
        m.insert("raw_citations".to_string(), serde_json::json!(self.raw_citations));
        m.insert("normalized_score".to_string(), serde_json::json!(round(self.normalized_score, 3)));
        m.insert("pagerank_score".to_string(), serde_json::json!(round(self.pagerank_score, 3)));
        m.insert("momentum_score".to_string(), serde_json::json!(round(self.momentum_score, 3)));
        m.insert("author_h_index".to_string(), serde_json::json!(round(self.author_h_index, 3)));
        m.insert("composite_score".to_string(), serde_json::json!(round(self.composite_score, 3)));
        m.insert("percentile".to_string(), serde_json::json!(round(self.percentile, 1)));
        m.insert("tier".to_string(), serde_json::json!(self.tier));
        m
    }
}

pub struct ImpactScorer {
    db: Option<()>,
    scores: HashMap<String, ImpactScore>,
    pagerank_iterations: usize,
}

impl ImpactScorer {
    pub const WEIGHT_NORMALIZED: f64 = 0.30;
    pub const WEIGHT_PAGERANK: f64 = 0.30;
    pub const WEIGHT_MOMENTUM: f64 = 0.25;
    pub const WEIGHT_AUTHOR: f64 = 0.15;

    pub fn new() -> Self {
        Self {
            db: None,
            scores: HashMap::new(),
            pagerank_iterations: 4,
        }
    }

    pub fn score_paper(
        &mut self,
        paper_id: &str,
        title: &str,
        year: i32,
        raw_citations: i32,
        citing_papers: Option<Vec<HashMap<String, String>>>,
        author_h_index: f64,
    ) -> ImpactScore {
        let current_year = 2026;
        let age = (current_year - year).max(1);
        let normalized = raw_citations as f64 / age as f64;

        let pagerank = self.compute_pagerank(paper_id, citing_papers.unwrap_or_default());
        let momentum = raw_citations as f64 / (age as f64).powf(0.7);

        let composite = Self::WEIGHT_NORMALIZED * self.normalize(normalized)
            + Self::WEIGHT_PAGERANK * pagerank
            + Self::WEIGHT_MOMENTUM * self.normalize(momentum)
            + Self::WEIGHT_AUTHOR * (author_h_index / 50.0).min(1.0);

        let tier = self.tier(composite);

        let score = ImpactScore {
            paper_id: paper_id.to_string(),
            title: title.to_string(),
            year,
            raw_citations,
            normalized_score: normalized,
            pagerank_score: pagerank,
            momentum_score: momentum,
            author_h_index,
            composite_score: composite,
            percentile: 0.0,
            tier,
        };

        self.scores.insert(paper_id.to_string(), score.clone());
        score
    }

    fn compute_pagerank(&self, _paper_id: &str, citing_papers: Vec<HashMap<String, String>>) -> f64 {
        if citing_papers.is_empty() {
            return 0.1;
        }

        let mut scores: HashMap<String, f64> = citing_papers
            .iter()
            .filter_map(|p| p.get("paper_id").cloned())
            .map(|id| (id, 1.0))
            .collect();

        let damping = 0.85;

        for _ in 0..self.pagerank_iterations {
            let mut new_scores: HashMap<String, f64> = HashMap::new();
            for (pid, score) in &scores {
                if self.scores.contains_key(pid) {
                    let inherited = score * damping;
                    *new_scores.entry(pid.clone()).or_insert(0.0) += inherited;
                }
            }
            let total = new_scores.values().sum::<f64>();
            if total > 0.0 {
                for v in new_scores.values_mut() {
                    *v /= total;
                }
            }
            scores = new_scores;
        }

        scores.values().sum()
    }

    fn normalize(&self, value: f64) -> f64 {
        1.0 - (1.0 / (1.0 + value / 10.0))
    }

    fn tier(&self, composite: f64) -> String {
        if composite >= 0.8 { "S".to_string() }
        else if composite >= 0.6 { "A".to_string() }
        else if composite >= 0.4 { "B".to_string() }
        else if composite >= 0.2 { "C".to_string() }
        else { "D".to_string() }
    }

    pub fn score_batch(
        &mut self,
        papers: Vec<HashMap<String, serde_json::Value>>,
        citing_map: Option<HashMap<String, Vec<String>>>,
    ) -> Vec<ImpactScore> {
        let mut results = Vec::new();

        for p in &papers {
            let paper_id = p.get("paper_id").and_then(|v| v.as_str()).unwrap_or("");
            let title = p.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let year = p.get("year").and_then(|v| v.as_i64()).unwrap_or(2020) as i32;
            let raw_citations = p.get("citation_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let author_h_index = p.get("author_h_index")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            let citing = citing_map.as_ref()
                .and_then(|m| m.get(paper_id))
                .map(|ids| ids.iter().map(|id| {
                    let mut m = HashMap::new();
                    m.insert("paper_id".to_string(), id.clone());
                    m
                }).collect()).unwrap_or_default();

            let score = self.score_paper(paper_id, title, year, raw_citations, Some(citing), author_h_index);
            results.push(score);
        }

        self.assign_percentiles(&mut results);
        results
    }

    fn assign_percentiles(&self, scores: &mut [ImpactScore]) {
        if scores.is_empty() {
            return;
        }
        scores.sort_by(|a, b| b.composite_score.partial_cmp(&a.composite_score).unwrap());
        let n = scores.len();
        for (i, s) in scores.iter_mut().enumerate() {
            s.percentile = ((n - i) as f64 / n as f64) * 100.0;
        }
    }

    pub fn rank_papers(
        &mut self,
        papers: Vec<HashMap<String, serde_json::Value>>,
        top_k: usize,
    ) -> Vec<HashMap<String, serde_json::Value>> {
        let scored = self.score_batch(papers, None);
        let mut sorted = scored;
        sorted.sort_by(|a, b| b.composite_score.partial_cmp(&a.composite_score).unwrap());

        sorted.into_iter().take(top_k).enumerate().map(|(i, s)| {
            let mut m = s.to_dict();
            m.insert("rank".to_string(), serde_json::json!(i + 1));
            m.insert("why".to_string(), serde_json::json!(self.explain_score(&s)));
            m
        }).collect()
    }

    fn explain_score(&self, s: &ImpactScore) -> String {
        let mut parts = Vec::new();
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

    pub fn render_ranking(&self, ranking: &[HashMap<String, serde_json::Value>]) -> String {
        if ranking.is_empty() {
            return "No papers to rank.".to_string();
        }

        let mut lines = vec![
            "=".repeat(70),
            "📊 Paper Impact Ranking".to_string(),
            "=".repeat(70),
            "".to_string(),
            format!("{:<6}{:<6}{:<8}{:<12}{:<6} Title", "Rank", "Tier", "Score", "Citations", "Year"),
            "-".repeat(70),
        ];

        for entry in ranking {
            let tier_emoji = match entry.get("tier").and_then(|v| v.as_str()) {
                Some("S") => "⭐",
                Some("A") => "🅰️",
                Some("B") => "🅱️",
                Some("C") => "⚙️",
                _ => "📄",
            };
            let rank = entry.get("rank").and_then(|v| v.as_i64()).unwrap_or(0);
            let score = entry.get("composite_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let raw_citations = entry.get("raw_citations").and_then(|v| v.as_i64()).unwrap_or(0);
            let year = entry.get("year").and_then(|v| v.as_i64()).unwrap_or(0);
            let title = entry.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let title = &title[..title.len().min(40)];

            lines.push(format!(
                "{:<6}{:<6}{:<8.3}{:<12}{:<6}{}",
                rank, tier_emoji, score, raw_citations, year, title
            ));
        }

        lines.push("=".repeat(70));
        lines.join("\n")
    }
}

fn round(v: f64, decimals: usize) -> f64 {
    let mul = 10_f64.powi(decimals as i32);
    (v * mul).round() / mul
}

impl Default for ImpactScorer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_paper() {
        let mut scorer = ImpactScorer::new();
        let score = scorer.score_paper(
            "paper1",
            "Test Paper",
            2020,
            100,
            None,
            25.0,
        );
        assert!(score.composite_score > 0.0);
        assert!(!score.tier.is_empty());
    }

    #[test]
    fn test_tier_assignment() {
        let mut scorer = ImpactScorer::new();
        let s = scorer.score_paper("p1", "Title", 2020, 500, None, 50.0);
        assert!(["S", "A", "B", "C", "D"].contains(&s.tier.as_str()));

        let s = scorer.score_paper("p2", "Title", 2020, 10, None, 5.0);
        assert!(["S", "A", "B", "C", "D"].contains(&s.tier.as_str()));
    }

    #[test]
    fn test_score_batch() {
        let mut scorer = ImpactScorer::new();
        let papers = vec![
            {
                let mut m = HashMap::new();
                m.insert("paper_id".to_string(), serde_json::json!("p1"));
                m.insert("title".to_string(), serde_json::json!("Paper 1"));
                m.insert("year".to_string(), serde_json::json!(2020));
                m.insert("citation_count".to_string(), serde_json::json!(100));
                m
            },
            {
                let mut m = HashMap::new();
                m.insert("paper_id".to_string(), serde_json::json!("p2"));
                m.insert("title".to_string(), serde_json::json!("Paper 2"));
                m.insert("year".to_string(), serde_json::json!(2021));
                m.insert("citation_count".to_string(), serde_json::json!(50));
                m
            },
        ];
        let results = scorer.score_batch(papers, None);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_rank_papers() {
        let mut scorer = ImpactScorer::new();
        let papers = vec![
            {
                let mut m = HashMap::new();
                m.insert("paper_id".to_string(), serde_json::json!("p1"));
                m.insert("title".to_string(), serde_json::json!("Paper 1"));
                m.insert("year".to_string(), serde_json::json!(2020));
                m.insert("citation_count".to_string(), serde_json::json!(100));
                m
            },
            {
                let mut m = HashMap::new();
                m.insert("paper_id".to_string(), serde_json::json!("p2"));
                m.insert("title".to_string(), serde_json::json!("Paper 2"));
                m.insert("year".to_string(), serde_json::json!(2021));
                m.insert("citation_count".to_string(), serde_json::json!(200));
                m
            },
        ];
        let ranked = scorer.rank_papers(papers, 10);
        assert_eq!(ranked.len(), 2);
        assert_eq!(ranked[0].get("rank").and_then(|v| v.as_i64()), Some(1));
    }
}
