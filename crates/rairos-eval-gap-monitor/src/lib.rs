//! rairos-eval-gap-monitor — Evaluation Gap Monitor
//!
//! Flags deployment timelines outpacing benchmark research.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const PAPERS_DB: &str = ".ai_research_os/papers.json";
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

lazy_static::lazy_static! {
    static ref YEAR_PATTERN: Regex = Regex::new(r"\b(202[4-9]|203[0-5])\b").unwrap();
}

fn papers_db_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(PAPERS_DB)
}

fn load_papers() -> Vec<serde_json::Value> {
    let path = papers_db_path();
    if !path.exists() {
        return Vec::new();
    }
    let data: serde_json::Value = match fs::read_to_string(&path) {
        Ok(t) => match serde_json::from_str(&t) {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        },
        Err(_) => return Vec::new(),
    };
    data.get("papers")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

pub fn detect_deployment_claims(title: &str, abstract_text: &str) -> Option<String> {
    let text = format!("{} {}", title.to_lowercase(), abstract_text.to_lowercase());
    if !DEPLOYMENT_KEYWORDS.iter().any(|kw| text.contains(*kw)) {
        return None;
    }
    YEAR_PATTERN
        .captures(&format!("{} {}", title, abstract_text))
        .and_then(|m| m.get(1))
        .map(|m| m.as_str().to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployingPaper {
    pub title: String,
    pub year: String,
    pub paper_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalGapAlert {
    pub category: String,
    pub paper_count: usize,
    pub nearest_deployment_year: usize,
    pub headroom_years: usize,
    pub ratio: f64,
    pub deploying_papers: Vec<DeployingPaper>,
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

    let mut by_category: HashMap<String, Vec<&serde_json::Value>> = HashMap::new();
    for p in &papers {
        if let Some(cats) = p.get("categories").and_then(|v| v.as_array()) {
            for cat in cats {
                if let Some(c) = cat.as_str() {
                    by_category.entry(c.to_string()).or_default().push(p);
                }
            }
        }
    }

    let mut alerts: Vec<EvalGapAlert> = Vec::new();
    let current_year = 2026;

    for (cat, cat_papers) in &by_category {
        if cat_papers.len() < 3 {
            continue;
        }

        let mut deploying: Vec<DeployingPaper> = Vec::new();
        for p in cat_papers {
            let title = p.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let abstract_text = p.get("abstract").and_then(|v| v.as_str()).unwrap_or("");
            if let Some(year) = detect_deployment_claims(title, abstract_text) {
                deploying.push(DeployingPaper {
                    title: title.to_string(),
                    year: year.clone(),
                    paper_id: p.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                });
            }
        }

        if deploying.is_empty() {
            continue;
        }

        let deploying_years: Vec<usize> = deploying
            .iter()
            .filter_map(|d| d.year.parse().ok())
            .collect();

        let Some(&nearest_deploy) = deploying_years.iter().min() else {
            continue;
        };

        let headroom = nearest_deploy.saturating_sub(current_year);
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
                severity: if headroom >= 3 { "high".to_string() } else { "medium".to_string() },
            });
        }
    }

    alerts.sort_by_key(|b| std::cmp::Reverse(b.headroom_years));
    let alert_count = alerts.len();

    EvalGapResult {
        alerts,
        total_domains_checked: by_category.len(),
        alert_count,
    }
}

pub fn render_eval_gap_html(data: Option<&EvalGapResult>) -> String {
    let data = data.cloned().unwrap_or_else(check_eval_gaps);

    let mut lines = vec!["<div class=\"eval-gap\">".to_string()];
    lines.push("<h3>&#9888; Evaluation Gap Monitor</h3>".to_string());
    lines.push(format!(
        "<p style='font-size:13px;color:#A89E8C;margin-bottom:16px'>{} deployment-timeframe gaps detected across {} domains. <span style='color:#C4706A'>Red</span> = &#8805;3yr headroom &middot; <span style='color:#D4A055'>Orange</span> = 1-2yr</p>",
        data.alert_count, data.total_domains_checked
    ));

    if data.alerts.is_empty() {
        lines.push("<p>No evaluation gaps detected. Deployment timelines appear adequately covered by benchmark research.</p>".to_string());
    } else {
        for alert in &data.alerts {
            let color = if alert.severity == "high" { "#C4706A" } else { "#D4A055" };
            lines.push(format!(
                "<div style='border-left: 4px solid {}; padding: 10px 14px; margin-bottom: 14px; background: rgba(0,0,0,0.02);'>",
                color
            ));
            lines.push(format!(
                "<div style='font-weight:700;font-size:14px;color:#2a2a2a'>{}</div>",
                alert.category
            ));
            lines.push(format!(
                "<div style='font-size:12px;color:#7a7570;margin:4px 0'><b>{}</b> papers &middot; deployment in <b>{}</b> ({}yr headroom) &middot; ratio={:.2}</div>",
                alert.paper_count, alert.nearest_deployment_year, alert.headroom_years, alert.ratio
            ));
            for dep in alert.deploying_papers.iter().take(2) {
                let title_short = if dep.title.len() > 70 { &dep.title[..70] } else { &dep.title };
                lines.push(format!("<div style='font-size:11px;color:#A89E8C;margin-left:8px'>&bull; {}</div>", title_short));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_deployment_claims_no_match() {
        assert!(detect_deployment_claims("Some Title", "Some abstract").is_none());
    }

    #[test]
    fn test_detect_deployment_claims_with_deployment() {
        let result = detect_deployment_claims("Deployment in 2025", "This paper discusses deployment");
        assert_eq!(result, Some("2025".to_string()));
    }

    #[test]
    fn test_detect_deployment_claims_no_year() {
        let result = detect_deployment_claims("Deployment paper", "In deployment phase");
        assert!(result.is_none());
    }

    #[test]
    fn test_check_eval_gaps_empty() {
        let result = check_eval_gaps();
        assert_eq!(result.alert_count, 0);
    }

    #[test]
    fn test_render_eval_gap_html_empty() {
        let html = render_eval_gap_html(None);
        assert!(html.contains("Evaluation Gap Monitor"));
    }
}
