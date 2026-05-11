//! Rairos Paradigm — Paradigm Concentration Monitor
//!
//! Detects when >60% of citations in a domain cluster around ≤3 references.
//! Flags a generalization_gap risk alert.

use serde::{Deserialize, Serialize};

const ALERT_THRESHOLD: f64 = 0.60;
const TOP_N: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperStats {
    pub paper_id: String,
    pub title: String,
    pub citation_count: i32,
    pub share_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParadigmAlert {
    #[serde(rename = "type")]
    pub alert_type: String,
    pub severity: String,
    pub message: String,
    pub top_papers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParadigmResult {
    pub categories: Vec<PaperStats>,
    pub alerts: Vec<ParadigmAlert>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_papers: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_citations: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub struct ParadigmMonitor;

impl ParadigmMonitor {
    pub fn check(papers: &[[&str; 3]], citation_counts: &[i32]) -> ParadigmResult {
        if papers.len() != citation_counts.len() {
            return ParadigmResult {
                categories: Vec::new(),
                alerts: Vec::new(),
                total_papers: None,
                total_citations: None,
                error: Some("mismatched input lengths".to_string()),
            };
        }

        let total_citations: i32 = citation_counts.iter().sum();
        if total_citations == 0 {
            return ParadigmResult {
                categories: Vec::new(),
                alerts: Vec::new(),
                total_papers: Some(papers.len()),
                total_citations: Some(0),
                error: None,
            };
        }

        let mut indexed: Vec<(&str, i32)> = papers
            .iter()
            .zip(citation_counts.iter())
            .map(|(p, c)| (p[0], *c))
            .collect();
        indexed.sort_by(|a, b| b.1.cmp(&a.1));

        let top_n_slice = &indexed[..indexed.len().min(TOP_N)];
        let top_n_citations: i32 = top_n_slice.iter().map(|(_, c)| *c).sum();
        let concentration = top_n_citations as f64 / total_citations as f64;

        let categories: Vec<PaperStats> = top_n_slice
            .iter()
            .map(|(p, c)| {
                let share_pct = (*c as f64 / total_citations as f64) * 100.0;
                PaperStats {
                    paper_id: (*p).to_string(),
                    title: String::new(),
                    citation_count: *c,
                    share_pct: (share_pct * 10.0).round() / 10.0,
                }
            })
            .collect();

        let mut alerts = Vec::new();
        if concentration > ALERT_THRESHOLD {
            let threshold_pct = (ALERT_THRESHOLD * 100.0).round() as i32;
            let conc_pct = (concentration * 100.0).round() as i32;
            alerts.push(ParadigmAlert {
                alert_type: "paradigm_concentration".to_string(),
                severity: "high".to_string(),
                message: format!(
                    "{}% of citations cluster around {} papers (threshold: {}%). Consider diversifying reading to reduce generalization gap risk.",
                    conc_pct,
                    TOP_N,
                    threshold_pct
                ),
                top_papers: top_n_slice.iter().map(|(p, _)| (*p).to_string()).collect(),
            });
        }

        ParadigmResult {
            categories,
            alerts,
            total_papers: Some(papers.len()),
            total_citations: Some(total_citations),
            error: None,
        }
    }

    pub fn render_html(result: &ParadigmResult) -> String {
        if result.error.is_some() {
            return "<p>Paradigm concentration monitor temporarily unavailable</p>".to_string();
        }

        if result.categories.is_empty() && result.alerts.is_empty() {
            return "<p>No paradigm concentration detected.</p>".to_string();
        }

        let mut parts = Vec::new();

        for alert in &result.alerts {
            parts.push(format!(
                "<div style=\"background:#fff3cd;border:1px solid #ffc107;border-radius:6px;padding:12px;margin-bottom:12px;font-size:13px\">\
                 <strong>⚠️ Paradigm Concentration Alert</strong><br>{}</div>",
                alert.message
            ));
        }

        if !result.categories.is_empty() {
            let mut rows = Vec::new();
            for (i, cat) in result.categories.iter().enumerate() {
                rows.push(format!(
                    "<tr><td style=\"padding:6px 12px\">{}</td>\
                     <td style=\"padding:6px 12px\">{}</td>\
                     <td style=\"padding:6px 12px;text-align:center\">{}%</td></tr>",
                    i + 1,
                    cat.paper_id,
                    cat.share_pct
                ));
            }
            parts.push(format!(
                "<table style=\"width:100%;border-collapse:collapse;font-size:13px\">\
                 <tr style=\"background:#f5f5f5\">\
                 <th style=\"padding:8px 12px\">#</th>\
                 <th style=\"padding:8px 12px;text-align:left\">Paper</th>\
                 <th style=\"padding:8px 12px;text-align:center\">Citation Share</th>\
                 </tr>{}</table>",
                rows.join("")
            ));
        }

        parts.join("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_citations() {
        let papers: Vec<[&str; 3]> = vec![["p1", "Title 1", "cat1"]];
        let counts = vec![0];
        let result = ParadigmMonitor::check(&papers, &counts);
        assert!(result.categories.is_empty());
        assert!(result.alerts.is_empty());
        assert_eq!(result.total_citations, Some(0));
    }

    #[test]
    fn test_no_alert_below_threshold() {
        let papers: Vec<[&str; 3]> = vec![
            ["p1", "Title 1", "cat1"],
            ["p2", "Title 2", "cat1"],
            ["p3", "Title 3", "cat1"],
            ["p4", "Title 4", "cat1"],
            ["p5", "Title 5", "cat1"],
            ["p6", "Title 6", "cat1"],
            ["p7", "Title 7", "cat1"],
            ["p8", "Title 8", "cat1"],
            ["p9", "Title 9", "cat1"],
            ["p10", "Title 10", "cat1"],
        ];
        let counts = vec![3, 3, 3, 3, 3, 3, 3, 3, 3, 3];
        let result = ParadigmMonitor::check(&papers, &counts);
        assert!(result.categories.len() <= 3);
        assert!(result.alerts.is_empty());
    }

    #[test]
    fn test_alert_above_threshold() {
        let papers: Vec<[&str; 3]> = vec![
            ["p1", "Title 1", "cat1"],
            ["p2", "Title 2", "cat1"],
            ["p3", "Title 3", "cat1"],
        ];
        let counts = vec![80, 10, 10];
        let result = ParadigmMonitor::check(&papers, &counts);
        assert_eq!(result.alerts.len(), 1);
        assert_eq!(result.alerts[0].severity, "high");
    }

    #[test]
    fn test_render_html_empty() {
        let result = ParadigmResult {
            categories: Vec::new(),
            alerts: Vec::new(),
            total_papers: Some(0),
            total_citations: Some(0),
            error: None,
        };
        let html = ParadigmMonitor::render_html(&result);
        assert!(html.contains("No paradigm concentration detected"));
    }

    #[test]
    fn test_render_html_with_alert() {
        let result = ParadigmResult {
            categories: Vec::new(),
            alerts: vec![ParadigmAlert {
                alert_type: "test".to_string(),
                severity: "high".to_string(),
                message: "Test message".to_string(),
                top_papers: vec![],
            }],
            total_papers: Some(10),
            total_citations: Some(100),
            error: None,
        };
        let html = ParadigmMonitor::render_html(&result);
        assert!(html.contains("Paradigm Concentration Alert"));
    }
}
