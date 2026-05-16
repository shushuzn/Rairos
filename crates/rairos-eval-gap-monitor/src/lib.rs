//! rairos-eval-gap-monitor — Evaluation Gap Monitor for AI Research OS.

//!
//! Ported from `llm/eval_gap_monitor.py` (150 LOC, pure stdlib).
//!
//! Flags deployment timelines that outpace benchmark research.

use chrono::{Datelike, Local};
use rairos_core::constants::PAPERS_DB_PATH;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

// ─── Constants ────────────────────────────────────────────────────────────────

const GAP_THRESHOLD: f64 = 0.1;

const DEPLOYMENT_KEYWORDS: &[&str] = &[
    "deployment",
    "deployed",
    "in production",
    "in deployment",
    "real-world",
    "field trial",
    "pilot program",
    "operational",
];

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn load_papers() -> Vec<serde_json::Value> {
    let path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(PAPERS_DB_PATH);
    if !path.exists() {
        return vec![];
    }
    let Ok(contents) = fs::read_to_string(&path) else {
        return vec![];
    };
    let Ok(data) = serde_json::from_str::<serde_json::Value>(&contents) else {
        return vec![];
    };
    data.get("papers")
        .and_then(|v| v.as_array()).cloned()
        .unwrap_or_default()
}

fn detect_deployment_claims(title: &str, abstract_text: &str) -> Option<String> {
    let text = format!("{} {}", title.to_lowercase(), abstract_text.to_lowercase());
    if !DEPLOYMENT_KEYWORDS.iter().any(|kw| text.contains(*kw)) {
        return None;
    }
    let year_re = Regex::new(r"\b(202[4-9]|203[0-5])\b").ok()?;
    year_re
        .captures(&format!("{} {}", title, abstract_text))
        .map(|caps| caps.get(1).unwrap().as_str().to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentClaim {
    pub title: String,
    pub year: String,
    pub paper_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalGapAlert {
    pub category: String,
    pub paper_count: usize,
    pub nearest_deployment_year: i32,
    pub headroom_years: i32,
    pub ratio: f64,
    pub deploying_papers: Vec<DeploymentClaim>,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalGapResult {
    pub alerts: Vec<EvalGapAlert>,
    pub total_domains_checked: usize,
    pub alert_count: usize,
}

pub fn check_eval_gaps() -> EvalGapResult {
    let papers = load_papers();

    // Group by category
    let mut by_category: HashMap<String, Vec<&serde_json::Value>> = HashMap::new();
    for p in &papers {
        let cats = p.get("categories");
        let cats_array = cats.and_then(|v| v.as_array());
        let Some(cats_array) = cats_array else {
            continue;
        };
        for c in cats_array {
            let cat_name = c.as_str().unwrap_or("").to_string();
            if cat_name.is_empty() {
                continue;
            }
            by_category.entry(cat_name).or_default().push(p);
        }
    }

    let mut alerts: Vec<EvalGapAlert> = Vec::new();
    let current_year = Local::now().date_naive().year();

    for (cat, cat_papers) in &by_category {
        if cat_papers.len() < 3 {
            continue;
        }

        let mut deploying: Vec<DeploymentClaim> = Vec::new();
        for p in cat_papers {
            let title = p.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let abstract_text = p.get("abstract").and_then(|v| v.as_str()).unwrap_or("");
            if let Some(yr) = detect_deployment_claims(title, abstract_text) {
                deploying.push(DeploymentClaim {
                    title: title.to_string(),
                    year: yr.clone(),
                    paper_id: p.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                });
            }
        }

        if deploying.is_empty() {
            continue;
        }

        let deploying_years: Vec<i32> = deploying
            .iter()
            .filter_map(|d| d.year.parse().ok())
            .collect();
        let Some(nearest_deploy) = deploying_years.iter().min().copied() else {
            continue;
        };
        let headroom = std::cmp::max(0, nearest_deploy - current_year);
        let paper_count = cat_papers.len();

        let ratio = paper_count as f64 / headroom.max(1) as f64;
        if ratio < GAP_THRESHOLD && headroom >= 1 {
            alerts.push(EvalGapAlert {
                category: cat.clone(),
                paper_count,
                nearest_deployment_year: nearest_deploy,
                headroom_years: headroom,
                ratio: (ratio * 1000.0).round() / 1000.0,
                deploying_papers: deploying.into_iter().take(3).collect(),
                severity: if headroom >= 3 {
                    "high".to_string()
                } else {
                    "medium".to_string()
                },
            });
        }
    }

    alerts.sort_by_key(|x| std::cmp::Reverse(x.headroom_years));
    let alert_count = alerts.len();
    EvalGapResult {
        alerts,
        total_domains_checked: by_category.len(),
        alert_count,
    }
}

pub fn render_eval_gap_html(data: Option<EvalGapResult>) -> String {
    let data = data.unwrap_or_else(check_eval_gaps);
    let alerts = &data.alerts;

    let mut lines = vec!["<div class=\"eval-gap\">".to_string()];
    lines.push("<h3>⚠️ Evaluation Gap Monitor</h3>".to_string());
    lines.push(format!(
        "<p style='font-size:13px;color:#A89E8C;margin-bottom:16px'>{} deployment-timeframe gaps detected across {} domains. <span style='color:#C4706A'>Red</span> = ≥3yr headroom · <span style='color:#D4A055'>Orange</span> = 1-2yr</p>",
        data.alert_count, data.total_domains_checked
    ));

    if alerts.is_empty() {
        lines.push("No evaluation gaps detected. Deployment timelines appear adequately covered by benchmark research.".to_string());
    } else {
        for alert in alerts {
            let color = if alert.severity == "high" {
                "#C4706A"
            } else {
                "#D4A055"
            };
            lines.push(format!(
                "<div style='border-left: 4px solid {}; padding: 10px 14px; margin-bottom: 14px; background: rgba(0,0,0,0.02);'>",
                color
            ));
            lines.push(format!(
                "<div style='font-weight:700;font-size:14px;color:#2a2a2a'>{}</div>",
                alert.category
            ));
            lines.push(format!(
                "<div style='font-size:12px;color:#7a7570;margin:4px 0'><b>{}</b> papers · deployment in <b>{}</b> ({}yr headroom) · ratio={:.2}</div>",
                alert.paper_count, alert.nearest_deployment_year, alert.headroom_years, alert.ratio
            ));
            for dep in alert.deploying_papers.iter().take(2) {
                let title_short = if dep.title.len() > 70 {
                    &dep.title[..70]
                } else {
                    &dep.title
                };
                lines.push(format!(
                    "<div style='font-size:11px;color:#A89E8C;margin-left:8px'>• {}</div>",
                    title_short
                ));
            }
            lines.push("</div>".to_string());
        }
    }

    lines.push("<style>".to_string());
    lines.push(".eval-gap { font-family: Georgia, serif; }".to_string());
    lines.push("</style>".to_string());
    lines.push("</div>".to_string());

    lines.join("\n")
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_deployment_claims_none() {
        assert_eq!(detect_deployment_claims("A boring paper", "Nothing here"), None);
    }

    #[test]
    fn test_detect_deployment_claims_found() {
        let result = detect_deployment_claims("Deployment in 2025", "This system is in production");
        assert_eq!(result, Some("2025".to_string()));
    }

    #[test]
    fn test_detect_deployment_claims_no_year() {
        let result = detect_deployment_claims("Deployment study", "Nothing about years");
        assert_eq!(result, None);
    }

    #[test]
    fn test_check_eval_gaps_empty() {
        let result = check_eval_gaps();
        assert_eq!(result.alert_count, 0);
    }

    #[test]
    fn test_render_eval_gap_html_empty() {
        let html = render_eval_gap_html(None);
        assert!(html.contains("No evaluation gaps"));
    }

    #[test]
    fn test_deployment_claim_struct() {
        let dc = DeploymentClaim {
            title: "Test".to_string(),
            year: "2025".to_string(),
            paper_id: "p1".to_string(),
        };
        assert_eq!(dc.year, "2025");
    }

    #[test]
    fn test_eval_gap_result_struct() {
        let result = EvalGapResult {
            alerts: vec![],
            total_domains_checked: 0,
            alert_count: 0,
        };
        assert_eq!(result.alert_count, 0);
    }

    #[test]
    fn test_severity_assignment() {
        // high if headroom >= 3
        let high_alert = EvalGapAlert {
            category: "AI Safety".to_string(),
            paper_count: 5,
            nearest_deployment_year: 2030,
            headroom_years: 5,
            ratio: 0.05,
            deploying_papers: vec![],
            severity: "high".to_string(),
        };
        assert_eq!(high_alert.severity, "high");

        let medium_alert = EvalGapAlert {
            category: "Robotics".to_string(),
            paper_count: 3,
            nearest_deployment_year: 2027,
            headroom_years: 2,
            ratio: 0.08,
            deploying_papers: vec![],
            severity: "medium".to_string(),
        };
        assert_eq!(medium_alert.severity, "medium");
    }
}
