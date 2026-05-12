//! rairos-events — Event processing pipeline: news → capsules → related papers → insights.
//!
//! Ported from `llm/events.py`.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const HIGH_IMPACT_KEYWORDS: &[&str] = &[
    "导弹",
    "袭击",
    "无人机",
    "石油",
    "霍尔木兹",
    "制裁",
    "missile",
    "drone",
    "oil",
    "sanctions",
    "Strait of Hormuz",
    "利率",
    "通胀",
    "非农",
    "美联储",
    "加息",
    "rate",
    "inflation",
    "Fed",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsItem {
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub timestamp: String,
    #[serde(default)]
    pub url: String,
}

impl NewsItem {
    pub fn from_dict(map: &HashMap<String, serde_json::Value>) -> Self {
        Self {
            content: map
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            title: map
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            timestamp: map
                .get("timestamp")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            url: map
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSummary {
    pub primary_keyword: String,
    pub keywords: Vec<String>,
    pub capsule_title: String,
    pub brief: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperRef {
    pub paper_id: String,
    pub title: String,
    pub relevance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventResult {
    pub event_id: String,
    pub capsule_id: String,
    pub capsule_title: String,
    pub timestamp: String,
    pub keywords: Vec<String>,
    #[serde(default)]
    pub news_count: usize,
    pub related_papers: Vec<PaperRef>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub error: String,
}

pub fn build_summary(news_items: &[NewsItem], keyword: &str) -> EventSummary {
    let all_text: String = news_items
        .iter()
        .map(|item| item.content.clone())
        .collect::<Vec<_>>()
        .join(" ");

    let words: Vec<&str> = all_text
        .split_whitespace()
        .filter(|w| w.len() > 1)
        .collect();
    let mut word_counts: HashMap<&str, usize> = HashMap::new();
    for w in &words {
        *word_counts.entry(w).or_insert(0) += 1;
    }
    let mut sorted_words: Vec<(&str, usize)> = word_counts.into_iter().collect();
    sorted_words.sort_by_key(|b| std::cmp::Reverse(b.1));
    let top_kws: Vec<String> = sorted_words
        .iter()
        .take(10)
        .map(|(w, _)| (*w).to_string())
        .collect();

    let first_text = news_items
        .first()
        .map(|n| n.content.clone())
        .unwrap_or_default();

    EventSummary {
        primary_keyword: if keyword.is_empty() {
            top_kws
                .first()
                .cloned()
                .unwrap_or_else(|| "event".to_string())
        } else {
            keyword.to_string()
        },
        keywords: if !top_kws.is_empty() {
            top_kws.iter().take(8).cloned().collect()
        } else {
            vec![keyword.to_string()]
        },
        capsule_title: if !first_text.is_empty() {
            first_text.chars().take(120).collect()
        } else {
            format!("Event: {}", keyword)
        },
        brief: all_text.chars().take(300).collect(),
        timestamp: Utc::now().to_rfc3339(),
    }
}

pub fn infer_gap_type(summary: &EventSummary) -> &'static str {
    let text = summary.brief.to_lowercase();
    let military_kw = ["导弹", "袭击", "drone", "missile", "军事"];
    let oil_kw = ["石油", "油", "oil", "能源"];
    let rate_kw = ["利率", "通胀", "加息", "rate"];

    if military_kw.iter().any(|w| text.contains(w)) {
        return "scalability_issue";
    }
    if oil_kw.iter().any(|w| text.contains(w)) {
        return "evaluation_gap";
    }
    if rate_kw.iter().any(|w| text.contains(w)) {
        return "method_limitation";
    }
    "unexplored_application"
}

pub fn render_event_report(result: &EventResult) -> String {
    if !result.error.is_empty() {
        return format!("  Error: {}", result.error);
    }

    let mut lines = vec![
        "\n  ⚡ Event Processed".to_string(),
        format!("  ID: {}", result.event_id),
        format!(
            "  Time: {}",
            &result.timestamp[..19.min(result.timestamp.len())]
        ),
        format!("  Keywords: {}", result.keywords.join(", ")),
        String::new(),
        "  Capsule encoded:".to_string(),
        format!(
            "    {}",
            &result.capsule_title[..result.capsule_title.len().min(80)]
        ),
        String::new(),
        format!(
            "  Related academic papers ({}):",
            result.related_papers.len()
        ),
    ];

    for r in &result.related_papers {
        lines.push(format!("    {} relevance={:.2}", r.paper_id, r.relevance));
        lines.push(format!("    {}", &r.title[..r.title.len().min(70)]));
    }
    lines.push(String::new());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_summary_with_news() {
        let news = vec![
            NewsItem {
                content: "Iran launches missile attack on US base".to_string(),
                title: "Breaking News".to_string(),
                timestamp: "".to_string(),
                url: "".to_string(),
            },
            NewsItem {
                content: "Tensions rise in Gulf region".to_string(),
                title: "Update".to_string(),
                timestamp: "".to_string(),
                url: "".to_string(),
            },
        ];
        let summary = build_summary(&news, "iran");
        assert_eq!(summary.primary_keyword, "iran");
        assert!(!summary.keywords.is_empty());
        assert!(summary.capsule_title.contains("Iran"));
    }

    #[test]
    fn test_build_summary_empty_keyword() {
        let news = vec![NewsItem {
            content: "Test content with uniquewordxyz here".to_string(),
            title: "".to_string(),
            timestamp: "".to_string(),
            url: "".to_string(),
        }];
        let summary = build_summary(&news, "");
        assert!(!summary.primary_keyword.is_empty());
    }

    #[test]
    fn test_infer_gap_type_military() {
        let summary = EventSummary {
            primary_keyword: "test".to_string(),
            keywords: vec![],
            capsule_title: "".to_string(),
            brief: "Iranian missile attack on US interests".to_string(),
            timestamp: "".to_string(),
        };
        assert_eq!(infer_gap_type(&summary), "scalability_issue");
    }

    #[test]
    fn test_infer_gap_type_oil() {
        let summary = EventSummary {
            primary_keyword: "test".to_string(),
            keywords: vec![],
            capsule_title: "".to_string(),
            brief: "Oil prices surge after OPEC meeting".to_string(),
            timestamp: "".to_string(),
        };
        assert_eq!(infer_gap_type(&summary), "evaluation_gap");
    }

    #[test]
    fn test_infer_gap_type_rate() {
        let summary = EventSummary {
            primary_keyword: "test".to_string(),
            keywords: vec![],
            capsule_title: "".to_string(),
            brief: "Fed raises interest rates amid inflation".to_string(),
            timestamp: "".to_string(),
        };
        assert_eq!(infer_gap_type(&summary), "method_limitation");
    }

    #[test]
    fn test_infer_gap_type_default() {
        let summary = EventSummary {
            primary_keyword: "test".to_string(),
            keywords: vec![],
            capsule_title: "".to_string(),
            brief: "Some random research topic".to_string(),
            timestamp: "".to_string(),
        };
        assert_eq!(infer_gap_type(&summary), "unexplored_application");
    }

    #[test]
    fn test_render_event_report_error() {
        let result = EventResult {
            event_id: "".to_string(),
            capsule_id: "".to_string(),
            capsule_title: "".to_string(),
            timestamp: "".to_string(),
            keywords: vec![],
            news_count: 0,
            related_papers: vec![],
            summary: "".to_string(),
            error: "No news found".to_string(),
        };
        let report = render_event_report(&result);
        assert!(report.contains("Error"));
    }

    #[test]
    fn test_render_event_report_success() {
        let result = EventResult {
            event_id: "evt_123".to_string(),
            capsule_id: "cap_456".to_string(),
            capsule_title: "Test Event".to_string(),
            timestamp: "2024-01-15T10:30:00Z".to_string(),
            keywords: vec!["test".to_string(), "event".to_string()],
            news_count: 5,
            related_papers: vec![PaperRef {
                paper_id: "paper_1".to_string(),
                title: "Important Paper".to_string(),
                relevance: 0.85,
            }],
            summary: "Test summary".to_string(),
            error: "".to_string(),
        };
        let report = render_event_report(&result);
        assert!(report.contains("evt_123"));
        assert!(report.contains("Test Event"));
        assert!(report.contains("paper_1"));
    }

    #[test]
    fn test_high_impact_keywords_presence() {
        assert!(HIGH_IMPACT_KEYWORDS.contains(&"missile"));
        assert!(HIGH_IMPACT_KEYWORDS.contains(&"drone"));
        assert!(HIGH_IMPACT_KEYWORDS.contains(&"oil"));
        assert!(HIGH_IMPACT_KEYWORDS.contains(&"sanctions"));
        assert!(HIGH_IMPACT_KEYWORDS.contains(&"利率"));
    }
}
