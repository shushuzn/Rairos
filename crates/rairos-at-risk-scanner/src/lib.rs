//! rairos-at-risk-scanner — At-Risk Capsule Scanner for AI Research OS.

#![allow(

)]
//!
//! Ported from `llm/at_risk_scanner.py` (166 LOC, pure stdlib).
//!
//! Shows capsules approaching auto-archive threshold (low_score_streak >= 2).
//! Supports keep-active (reset streak) and pin-to-TTL operations.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// ─── Constants ────────────────────────────────────────────────────────────────

const CAPSULE_PATH: &str = ".ai_research_os/gene_pool/capsules.json";
const STREAK_THRESHOLD: u32 = 2;

// ─── Data Structures ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtRiskCapsule {
    pub capsule_id: String,
    pub gap_title: String,
    pub gap_type: String,
    pub outcome_score: f64,
    pub low_score_streak: u32,
    pub status: String,
    #[serde(default)]
    pub pinned_ttl: u32,
    #[serde(default)]
    pub trigger_keywords: Vec<String>,
}

// ─── Helpers ───────────────────────────────────────────────────────────────────

fn capsules_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(CAPSULE_PATH)
}

fn load_capsules() -> Vec<serde_json::Value> {
    let path = capsules_path();
    if !path.exists() {
        return vec![];
    }
    let Ok(contents) = fs::read_to_string(&path) else {
        return vec![];
    };
    let Ok(data) = serde_json::from_str::<serde_json::Value>(&contents) else {
        return vec![];
    };
    data.get("capsules")
        .and_then(|v| v.as_array()).cloned()
        .unwrap_or_default()
}

fn save_capsules(capsules: &[serde_json::Value]) -> std::io::Result<()> {
    let path = capsules_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&serde_json::json!({"capsules": capsules}))?;
    fs::write(&path, json)
}

// ─── Core Logic ───────────────────────────────────────────────────────────────

pub fn get_at_risk_capsules(threshold: u32) -> Vec<AtRiskCapsule> {
    let all_caps = load_capsules();
    let threshold = if threshold == 0 { STREAK_THRESHOLD } else { threshold };
    let mut results = Vec::new();

    for cap in &all_caps {
        let status = cap.get("status").and_then(|v| v.as_str()).unwrap_or("");
        if status != "active" && !status.is_empty() {
            continue;
        }
        let streak = cap
            .get("low_score_streak")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        if streak < threshold {
            continue;
        }
        results.push(AtRiskCapsule {
            capsule_id: cap
                .get("capsule_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            gap_title: cap
                .get("action_gap_title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            gap_type: cap
                .get("action_gap_type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            outcome_score: cap
                .get("outcome_success_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            low_score_streak: streak,
            status: status.to_string(),
            pinned_ttl: cap
                .get("pinned_ttl")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32,
            trigger_keywords: cap
                .get("trigger_keywords")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
        });
    }

    results.sort_by(|a, b| b.low_score_streak.cmp(&a.low_score_streak));
    results
}

pub fn keep_active(capsule_id: &str) -> bool {
    let mut capsules = load_capsules();
    let mut found = false;
    for cap in &mut capsules {
        if cap.get("capsule_id").and_then(|v| v.as_str()) == Some(capsule_id) {
            cap["low_score_streak"] = serde_json::json!(0);
            cap["pinned_ttl"] = serde_json::json!(0);
            found = true;
            break;
        }
    }
    if !found {
        return false;
    }
    save_capsules(&capsules).is_ok()
}

pub fn pin_to_ttl(capsule_id: &str, ttl: u32) -> bool {
    let mut capsules = load_capsules();
    let mut found = false;
    for cap in &mut capsules {
        if cap.get("capsule_id").and_then(|v| v.as_str()) == Some(capsule_id) {
            cap["pinned_ttl"] = serde_json::json!(ttl);
            cap["low_score_streak"] = serde_json::json!(0);
            found = true;
            break;
        }
    }
    if !found {
        return false;
    }
    save_capsules(&capsules).is_ok()
}

pub fn render_html(capsules: Option<Vec<AtRiskCapsule>>) -> String {
    let capsules = capsules.unwrap_or_else(|| get_at_risk_capsules(0));

    if capsules.is_empty() {
        return "<p>No at-risk capsules. All capsules are healthy.</p>".to_string();
    }

    let mut lines = vec!["<div class=\"at-risk-panel\">".to_string()];
    lines.push(format!(
        "<h3>🚨 At-Risk Capsules <small style='color:#888'>({} need attention)</small></h3>",
        capsules.len()
    ));
    lines.push("<table class=\"at-risk-table\">".to_string());
    lines.push(
        "<thead><tr><th>Gap Title</th><th>Type</th><th>Score</th>\
         <th>Streak</th><th>Pinned</th><th>Action</th></tr></thead>".to_string(),
    );
    lines.push("<tbody>".to_string());

    for cap in &capsules {
        let streak_bar = "🔴".repeat(cap.low_score_streak as usize);
        let pinned = if cap.pinned_ttl > 0 {
            format!("TTL {}", cap.pinned_ttl)
        } else {
            "—".to_string()
        };
        let title_short = if cap.gap_title.len() > 35 {
            &cap.gap_title[..35]
        } else {
            &cap.gap_title
        };
        lines.push("<tr>".to_string());
        lines.push(format!(
            "<td style='max-width:220px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap'><code title='{}'>{}</code></td>",
            cap.gap_title, title_short
        ));
        lines.push(format!("<td><code>{}</code></td>", cap.gap_type));
        lines.push(format!("<td>{:.2}</td>", cap.outcome_score));
        lines.push(format!("<td>{} <small>{}</small></td>", streak_bar, cap.low_score_streak));
        lines.push(format!("<td>{}</td>", pinned));
        lines.push("<td>".to_string());
        lines.push(format!(
            "<button class=\"btn btn-small btn-keep\" onclick=\"keepActive('{}')\">✓ Keep Active</button>",
            cap.capsule_id
        ));
        lines.push(format!(
            "<button class=\"btn btn-small btn-pin\" onclick=\"pinToTTL('{}')\">📌 Pin TTL</button>",
            cap.capsule_id
        ));
        lines.push("</td></tr>".to_string());
    }

    lines.push("</tbody></table>".to_string());
    lines.push("<style>".to_string());
    lines.push(".at-risk-panel { font-family: Georgia, serif; }".to_string());
    lines.push(".at-risk-table { width: 100%; border-collapse: collapse; margin-top: 1rem; }".to_string());
    lines.push(".at-risk-table th, .at-risk-table td { padding: 0.4rem 0.8rem; border-bottom: 1px solid #e8e4de; text-align: left; }".to_string());
    lines.push(".at-risk-table th { font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em; color: #7a7570; }".to_string());
    lines.push(".btn-small { padding: 3px 10px; font-size: 12px; border-radius: 4px; cursor: pointer; }".to_string());
    lines.push(".btn-keep { background: #7A9E7A; color: white; border: none; }".to_string());
    lines.push(".btn-pin { background: #6B8FB5; color: white; border: none; margin-left: 4px; }".to_string());
    lines.push("</style>".to_string());
    lines.push("</div>".to_string());

    lines.join("\n")
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_at_risk_capsule_default_pinned_ttl() {
        let arc = AtRiskCapsule {
            capsule_id: "c1".to_string(),
            gap_title: "Test".to_string(),
            gap_type: "eval".to_string(),
            outcome_score: 0.3,
            low_score_streak: 3,
            status: "active".to_string(),
            pinned_ttl: 0,
            trigger_keywords: vec![],
        };
        assert_eq!(arc.pinned_ttl, 0);
        assert!(arc.low_score_streak >= 2);
    }

    #[test]
    fn test_render_html_empty() {
        let html = render_html(Some(vec![]));
        assert!(html.contains("No at-risk capsules"));
    }

    #[test]
    fn test_get_at_risk_capsules_empty() {
        let result = get_at_risk_capsules(0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_keep_active_not_found() {
        // Should return false when capsule doesn't exist
        let result = keep_active("nonexistent-capsule-id");
        // Function returns false for nonexistent capsule
        assert!(!result);
    }

    #[test]
    fn test_pin_to_ttl_not_found() {
        let result = pin_to_ttl("nonexistent-capsule-id", 3);
        assert!(!result);
    }

    #[test]
    fn test_streak_threshold_default() {
        assert_eq!(STREAK_THRESHOLD, 2);
    }
}
