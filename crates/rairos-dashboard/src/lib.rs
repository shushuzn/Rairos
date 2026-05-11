//! rairos-dashboard — Research Dashboard
//!
//! Aggregated view of research progress.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionSummary {
    pub id: String,
    pub question: String,
    pub status: String,
    pub priority: String,
    pub hypotheses_count: usize,
    pub roadmap_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentSummary {
    pub id: String,
    pub name: String,
    pub status: String,
    pub milestone: String,
    pub metrics_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PaperStats {
    pub total_papers: usize,
    pub recent_papers: usize,
    pub by_year: HashMap<String, usize>,
    pub by_tag: HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotPaper {
    pub paper_id: String,
    pub title: String,
    pub year: i32,
    pub velocity: f64,
    pub forward_cites: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendKeyword {
    pub keyword: String,
    pub direction: String,
    pub paper_count: i32,
    pub growth: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapPreferenceStats {
    pub total_events: usize,
    pub preferred_types: Vec<(String, f64)>,
    pub disliked_types: Vec<(String, f64)>,
    pub preferred_keywords: Vec<(String, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    #[serde(rename = "generated_at")]
    pub generated_at: String,
    pub questions: Vec<QuestionSummary>,
    pub experiments: Vec<ExperimentSummary>,
    pub papers: Option<PaperStats>,
    pub hot_papers: Vec<HotPaper>,
    pub trends: Vec<TrendKeyword>,
    pub gap_preferences: Option<GapPreferenceStats>,
    pub summary: HashMap<String, serde_json::Value>,
}

impl Default for DashboardData {
    fn default() -> Self {
        Self {
            generated_at: Utc::now().to_rfc3339(),
            questions: Vec::new(),
            experiments: Vec::new(),
            papers: None,
            hot_papers: Vec::new(),
            trends: Vec::new(),
            gap_preferences: None,
            summary: HashMap::new(),
        }
    }
}

impl DashboardData {
    pub fn new() -> Self {
        Self::default()
    }
}

pub struct Dashboard {
    papers: Vec<PaperRecord>,
    questions: Vec<QuestionSummary>,
    experiments: Vec<ExperimentSummary>,
}

struct PaperRecord {
    id: String,
    title: String,
    year: Option<i32>,
    created_at: String,
    tags: Vec<String>,
}

impl Dashboard {
    pub fn new() -> Self {
        Self {
            papers: Vec::new(),
            questions: Vec::new(),
            experiments: Vec::new(),
        }
    }

    pub fn add_paper(&mut self, id: &str, title: &str, year: Option<i32>, created_at: &str, tags: Vec<String>) {
        self.papers.push(PaperRecord {
            id: id.to_string(),
            title: title.to_string(),
            year,
            created_at: created_at.to_string(),
            tags,
        });
    }

    pub fn add_question(&mut self, question: QuestionSummary) {
        self.questions.push(question);
    }

    pub fn add_experiment(&mut self, experiment: ExperimentSummary) {
        self.experiments.push(experiment);
    }

    pub fn collect(&mut self) -> DashboardData {
        let mut data = DashboardData::new();

        data.questions = self.questions.clone();
        data.experiments = self.experiments.clone();
        data.papers = Some(self.collect_paper_stats());
        data.hot_papers = self.collect_hot_papers();
        data.summary = self.build_summary(&data);

        data
    }

    fn collect_paper_stats(&self) -> PaperStats {
        let mut stats = PaperStats {
            total_papers: self.papers.len(),
            ..Default::default()
        };

        let thirty_days_ago = Utc::now() - chrono::Duration::days(30);

        for p in &self.papers {
            if let Some(year) = p.year {
                let year_str = year.to_string();
                *stats.by_year.entry(year_str).or_insert(0) += 1;
            }

            if let Ok(created) = DateTime::parse_from_rfc3339(&p.created_at) {
                if created >= thirty_days_ago {
                    stats.recent_papers += 1;
                }
            }

            for tag in &p.tags {
                *stats.by_tag.entry(tag.clone()).or_insert(0) += 1;
            }
        }

        stats
    }

    fn collect_hot_papers(&self) -> Vec<HotPaper> {
        let mut scored: Vec<(f64, i32, String, String, i32)> = self.papers.iter()
            .filter_map(|p| {
                let year = p.year?;
                if !(2000..=2026).contains(&year) {
                    return None;
                }
                let age = 2026 - year + 1;
                let velocity = 1.0 / age as f64;
                Some((velocity, 1, p.id.clone(), p.title.clone(), year))
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        scored.into_iter().take(10).map(|(velocity, fwd, pid, title, year)| {
            HotPaper {
                paper_id: pid,
                title: if title.len() > 60 { format!("{}...", &title[..60]) } else { title },
                year,
                velocity: (velocity * 10.0).round() / 10.0,
                forward_cites: fwd,
            }
        }).collect()
    }

    fn build_summary(&self, data: &DashboardData) -> HashMap<String, serde_json::Value> {
        let mut summary = HashMap::new();

        let mut questions_by_status: HashMap<&str, usize> = HashMap::new();
        for q in &data.questions {
            *questions_by_status.entry(&q.status).or_insert(0) += 1;
        }

        let mut experiments_by_status: HashMap<&str, usize> = HashMap::new();
        for e in &data.experiments {
            *experiments_by_status.entry(&e.status).or_insert(0) += 1;
        }

        summary.insert("total_questions".to_string(), serde_json::json!(data.questions.len()));
        summary.insert("questions_by_status".to_string(), serde_json::json!(questions_by_status));
        summary.insert("total_experiments".to_string(), serde_json::json!(data.experiments.len()));
        summary.insert("experiments_by_status".to_string(), serde_json::json!(experiments_by_status));

        if let Some(ref papers) = data.papers {
            summary.insert("total_papers".to_string(), serde_json::json!(papers.total_papers));
            summary.insert("papers_this_month".to_string(), serde_json::json!(papers.recent_papers));
        }

        summary.insert("hot_papers_count".to_string(), serde_json::json!(data.hot_papers.len()));
        summary.insert("trends_count".to_string(), serde_json::json!(data.trends.len()));

        summary
    }

    pub fn render_text(&self, data: &DashboardData) -> String {
        let mut lines = Vec::new();

        lines.push("=".repeat(60));
        lines.push("Research Dashboard".to_string());
        lines.push(format!("Generated: {}", &data.generated_at[..19]));
        lines.push("=".repeat(60));
        lines.push(String::new());

        lines.push("## Summary".to_string());
        let s = &data.summary;
        lines.push(format!("  Questions: {}", s.get("total_questions").and_then(|v| v.as_u64()).unwrap_or(0)));
        lines.push(format!("  Experiments: {}", s.get("total_experiments").and_then(|v| v.as_u64()).unwrap_or(0)));
        if let Some(papers) = &data.papers {
            lines.push(format!("  Papers: {} (this month: {})", papers.total_papers, papers.recent_papers));
        }

        lines.push(String::new());

        if !data.hot_papers.is_empty() {
            lines.push("## Hot Papers (by Citation Velocity)".to_string());
            for (i, p) in data.hot_papers.iter().take(5).enumerate() {
                lines.push(format!("  {}. {}/y  {} ({}))", i + 1, p.velocity, p.title, p.year));
            }
            lines.push(String::new());
        }

        if !data.questions.is_empty() {
            lines.push("## Questions".to_string());
            let mut by_status: HashMap<&str, Vec<&QuestionSummary>> = HashMap::new();
            for q in &data.questions {
                by_status.entry(&q.status).or_default().push(q);
            }
            let mut status_keys: Vec<_> = by_status.keys().collect();
            status_keys.sort();
            for status in status_keys {
                let questions = &by_status[status];
                lines.push(format!("  {} ({})", status.to_uppercase(), questions.len()));
                for q in questions.iter().take(3) {
                    let q_short = if q.question.len() > 50 { format!("{}...", &q.question[..50]) } else { q.question.clone() };
                    lines.push(format!("    - [{}] {}", q.id, q_short));
                }
            }
            lines.push(String::new());
        }

        if !data.experiments.is_empty() {
            lines.push("## Experiments".to_string());
            let mut by_status: HashMap<&str, Vec<&ExperimentSummary>> = HashMap::new();
            for e in &data.experiments {
                by_status.entry(&e.status).or_default().push(e);
            }
            let mut status_keys: Vec<_> = by_status.keys().collect();
            status_keys.sort();
            for status in status_keys {
                let experiments = &by_status[status];
                lines.push(format!("  {} ({})", status.to_uppercase(), experiments.len()));
                for e in experiments.iter().take(3) {
                    lines.push(format!("    - [{}] {}", e.id, e.name));
                }
            }
            lines.push(String::new());
        }

        if let Some(papers) = &data.papers {
            if papers.total_papers > 0 {
                lines.push("## Papers".to_string());
                lines.push(format!("  Total: {}", papers.total_papers));
                lines.push(format!("  This month: {}", papers.recent_papers));
                lines.push(String::new());
            }
        }

        lines.push("=".repeat(60));

        lines.join("\n")
    }

    pub fn render_json(&self, data: &DashboardData) -> String {
        serde_json::to_string_pretty(data).unwrap_or_default()
    }
}

impl Default for Dashboard {
    fn default() -> Self {
        Self::new()
    }
}

use std::iter::Iterator;
use std::cmp::PartialOrd;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_new() {
        let dashboard = Dashboard::new();
        assert!(dashboard.papers.is_empty());
    }

    #[test]
    fn test_dashboard_add_paper() {
        let mut dashboard = Dashboard::new();
        dashboard.add_paper("p1", "Test Paper", Some(2024), "2024-01-01T00:00:00Z", vec!["tag1".to_string()]);
        assert_eq!(dashboard.papers.len(), 1);
    }

    #[test]
    fn test_collect_empty() {
        let mut dashboard = Dashboard::new();
        let data = dashboard.collect();
        assert_eq!(data.questions.len(), 0);
        assert_eq!(data.experiments.len(), 0);
    }

    #[test]
    fn test_collect_paper_stats() {
        let mut dashboard = Dashboard::new();
        dashboard.add_paper("p1", "Paper 1", Some(2024), &Utc::now().to_rfc3339(), vec!["cs.AI".to_string()]);
        let data = dashboard.collect();
        assert!(data.papers.is_some());
        if let Some(ref stats) = data.papers {
            assert_eq!(stats.total_papers, 1);
        }
    }

    #[test]
    fn test_render_text() {
        let mut dashboard = Dashboard::new();
        let data = dashboard.collect();
        let text = dashboard.render_text(&data);
        assert!(text.contains("Research Dashboard"));
    }

    #[test]
    fn test_render_json() {
        let mut dashboard = Dashboard::new();
        let data = dashboard.collect();
        let json = dashboard.render_json(&data);
        assert!(json.contains("generated_at"));
    }
}
