//! rairos-vault — Bold Hypothesis Vault & At-Risk Capsule Scanner
//!
//! Bold Vault: Tracks high-risk/high-reward Gene Pool capsules
//!   (theoretical_gap OR negative polarity OR novelty > 0.7).
//! At-Risk Scanner: Shows capsules approaching auto-archive threshold
//!   (low_score_streak >= 2).
//!
//! Ported from `llm/bold_vault.py` and `llm/at_risk_scanner.py`.

use rairos_core::jaccard_similarity;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const CAPSULE_PATH: &str = ".ai_research_os/gene_pool/capsules.json";
const NOVELTY_THRESHOLD: f64 = 0.7;
const STREAK_THRESHOLD: i32 = 2;

fn capsule_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(CAPSULE_PATH)
}

fn load_capsules() -> Vec<serde_json::Value> {
    let path = capsule_path();
    if !path.exists() {
        return Vec::new();
    }
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    let data: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    data.get("capsules")
        .and_then(|v| v.as_array().map(|arr| arr.to_vec()))
        .unwrap_or_default()
}

fn save_capsules(capsules: &[serde_json::Value]) -> bool {
    let path = capsule_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let data = serde_json::json!({ "capsules": capsules });
    match serde_json::to_string_pretty(&data) {
        Ok(json) => fs::write(&path, json).is_ok(),
        Err(_) => false,
    }
}

// ============================================================================
// Bold Vault
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoldCapsule {
    pub capsule_id: String,
    pub gap_title: String,
    pub gap_type: String,
    pub polarity: String,
    pub outcome_score: f64,
    pub novelty_score: f64,
    pub trigger_keywords: Vec<String>,
    pub reason: String,
}

pub fn get_bold_capsules() -> Vec<BoldCapsule> {
    let capsules = load_capsules();
    let mut results = Vec::new();

    for cap in &capsules {
        let status = cap.get("status").and_then(|v| v.as_str()).unwrap_or("");
        if status != "active" && !status.is_empty() {
            continue;
        }

        let gap_type = cap
            .get("action_gap_type")
            .and_then(|v| v.as_str())
            .or_else(|| cap.get("trigger_gap_type").and_then(|v| v.as_str()))
            .unwrap_or("");
        let polarity = cap
            .get("polarity")
            .and_then(|v| v.as_str())
            .unwrap_or("positive");
        let keywords: Vec<String> = cap
            .get("trigger_keywords")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let mut max_overlap = 0.0_f64;
        let cap_id = cap.get("capsule_id").and_then(|v| v.as_str()).unwrap_or("");
        for other in &capsules {
            let other_id = other
                .get("capsule_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if other_id == cap_id {
                continue;
            }
            let other_kw: Vec<String> = other
                .get("trigger_keywords")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let ov = jaccard_similarity(&keywords, &other_kw);
            if ov > max_overlap {
                max_overlap = ov;
            }
        }
        let novelty = 1.0 - max_overlap;

        let mut reasons = Vec::new();
        if gap_type == "theoretical_gap" {
            reasons.push("theoretical".to_string());
        }
        if polarity == "negative" {
            reasons.push("negative".to_string());
        }
        if novelty > NOVELTY_THRESHOLD {
            reasons.push(format!("high-novelty({:.0}%)", novelty * 100.0));
        }

        if reasons.is_empty() {
            continue;
        }

        let outcome_score = cap
            .get("outcome_success_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        results.push(BoldCapsule {
            capsule_id: cap_id.to_string(),
            gap_title: cap
                .get("action_gap_title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            gap_type: gap_type.to_string(),
            polarity: polarity.to_string(),
            outcome_score,
            novelty_score: (novelty * 1000.0).round() / 1000.0,
            trigger_keywords: keywords,
            reason: reasons.join(", "),
        });
    }

    results.sort_by(|a, b| b.novelty_score.partial_cmp(&a.novelty_score).unwrap());
    results
}

pub fn render_bold_vault_html() -> String {
    let capsules = get_bold_capsules();
    if capsules.is_empty() {
        return "<p>No bold hypotheses yet. Theoretical gaps and negative-polarity capsules will appear here.</p>".to_string();
    }

    let mut lines = Vec::new();
    lines.push("<div class=\"bold-vault\">".to_string());
    lines.push(
        "<h3>Bold Hypothesis Vault <small style='color:#888'>(high-risk / high-reward gaps)</small></h3>".to_string(),
    );
    lines.push(format!(
        "<p style='font-size:13px;color:#A89E8C;margin-bottom:16px'>{} bold capsules tracked.</p>",
        capsules.len()
    ));
    lines.push("<div class='bold-grid'>".to_string());

    for c in &capsules {
        let title_short = if c.gap_title.len() > 70 {
            &c.gap_title[..70]
        } else {
            &c.gap_title
        };
        let kw_str = c
            .trigger_keywords
            .iter()
            .take(4)
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!(
            "<div class='bold-card'><div class='bold-reason'>{}</div><div class='bold-title' title='{}'>{}</div><div class='bold-meta'><code>{}</code> &middot; {} &middot; score={:.2} &middot; novelty={:.0}%</div><div class='bold-kw'>{}</div></div>",
            c.reason,
            c.gap_title,
            title_short,
            c.gap_type,
            c.polarity,
            c.outcome_score,
            c.novelty_score,
            kw_str
        ));
    }

    lines.push("</div>".to_string());
    lines.push("<style>".to_string());
    lines.push(".bold-vault { font-family: Georgia, serif; }".to_string());
    lines.push(".bold-card { border: 2px solid #C4706A; border-radius: 6px; padding: 12px 14px; margin-bottom: 10px; background: rgba(196,112,106,0.06); }".to_string());
    lines.push(".bold-reason { font-size: 10px; text-transform: uppercase; letter-spacing: 0.5px; color: #C4706A; font-weight: 700; margin-bottom: 4px; }".to_string());
    lines.push(".bold-title { font-size: 14px; font-weight: 600; color: #2a2a2a; margin-bottom: 6px; line-height: 1.4; }".to_string());
    lines.push(".bold-meta { font-size: 11px; color: #7a7570; margin-bottom: 4px; }".to_string());
    lines.push(".bold-kw { font-size: 11px; color: #A89E8C; font-style: italic; }".to_string());
    lines.push("</style>".to_string());
    lines.push("</div>".to_string());

    lines.join("\n")
}

// ============================================================================
// At-Risk Scanner
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtRiskCapsule {
    pub capsule_id: String,
    pub gap_title: String,
    pub gap_type: String,
    pub outcome_score: f64,
    pub low_score_streak: i32,
    pub status: String,
    pub pinned_ttl: i32,
    pub trigger_keywords: Vec<String>,
}

pub fn get_at_risk_capsules(threshold: i32) -> Vec<AtRiskCapsule> {
    let capsules = load_capsules();
    let mut results = Vec::new();

    for cap in &capsules {
        let status = cap.get("status").and_then(|v| v.as_str()).unwrap_or("");
        if status != "active" && !status.is_empty() {
            continue;
        }

        let streak = cap
            .get("low_score_streak")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        if streak < threshold {
            continue;
        }

        let outcome_score = cap
            .get("outcome_success_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let pinned_ttl = cap.get("pinned_ttl").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let trigger_keywords: Vec<String> = cap
            .get("trigger_keywords")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

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
            outcome_score,
            low_score_streak: streak,
            status: status.to_string(),
            pinned_ttl,
            trigger_keywords,
        });
    }

    results.sort_by_key(|b| std::cmp::Reverse(b.low_score_streak));
    results
}

pub fn keep_active(capsule_id: &str) -> bool {
    let mut capsules = load_capsules();
    let mut found = false;
    for cap in &mut capsules {
        if cap
            .get("capsule_id")
            .and_then(|v| v.as_str())
            .map(|s| s == capsule_id)
            .unwrap_or(false)
        {
            cap["low_score_streak"] = serde_json::json!(0);
            cap["pinned_ttl"] = serde_json::json!(0);
            found = true;
            break;
        }
    }
    if !found {
        return false;
    }
    save_capsules(&capsules)
}

pub fn pin_to_ttl(capsule_id: &str, ttl: i32) -> bool {
    let mut capsules = load_capsules();
    let mut found = false;
    for cap in &mut capsules {
        if cap
            .get("capsule_id")
            .and_then(|v| v.as_str())
            .map(|s| s == capsule_id)
            .unwrap_or(false)
        {
            cap["pinned_ttl"] = serde_json::json!(ttl);
            cap["low_score_streak"] = serde_json::json!(0);
            found = true;
            break;
        }
    }
    if !found {
        return false;
    }
    save_capsules(&capsules)
}

pub fn render_at_risk_html() -> String {
    let capsules = get_at_risk_capsules(STREAK_THRESHOLD);
    if capsules.is_empty() {
        return "<p>No at-risk capsules. All capsules are healthy.</p>".to_string();
    }

    let mut lines = Vec::new();
    lines.push("<div class=\"at-risk-panel\">".to_string());
    lines.push(format!(
        "<h3>At-Risk Capsules <small style='color:#888'>({} need attention)</small></h3>",
        capsules.len()
    ));
    lines.push("<table class=\"at-risk-table\">".to_string());
    lines.push("<thead><tr><th>Gap Title</th><th>Type</th><th>Score</th><th>Streak</th><th>Pinned</th><th>Action</th></tr></thead>".to_string());
    lines.push("<tbody>".to_string());

    for cap in &capsules {
        let streak_bar = "🔴".repeat(cap.low_score_streak as usize);
        let pinned = if cap.pinned_ttl > 0 {
            format!("TTL {}", cap.pinned_ttl)
        } else {
            "—".to_string()
        };
        let title_short = if cap.gap_title.len() > 35 {
            format!("{}...", &cap.gap_title[..35])
        } else {
            cap.gap_title.clone()
        };
        lines.push(format!(
            "<tr>\
             <td style='max-width:220px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap'><code title='{}'>{}</code></td>\
             <td><code>{}</code></td>\
             <td>{:.2}</td>\
             <td>{} <small>{}</small></td>\
             <td>{}</td>\
             <td>\
             <button class=\"btn btn-small btn-keep\" onclick=\"keepActive('{}')\">Keep Active</button>\
             <button class=\"btn btn-small btn-pin\" onclick=\"pinToTTL('{}')\">Pin TTL</button>\
             </td>\
             </tr>",
            cap.gap_title,
            title_short,
            cap.gap_type,
            cap.outcome_score,
            streak_bar,
            cap.low_score_streak,
            pinned,
            cap.capsule_id,
            cap.capsule_id
        ));
    }

    lines.push("</tbody></table>".to_string());
    lines.push("<style>".to_string());
    lines.push(".at-risk-panel { font-family: Georgia, serif; }".to_string());
    lines.push(
        ".at-risk-table { width: 100%; border-collapse: collapse; margin-top: 1rem; }".to_string(),
    );
    lines.push(".at-risk-table th, .at-risk-table td { padding: 0.4rem 0.8rem; border-bottom: 1px solid #e8e4de; text-align: left; }".to_string());
    lines.push(".at-risk-table th { font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em; color: #7a7570; }".to_string());
    lines.push(
        ".btn-small { padding: 3px 10px; font-size: 12px; border-radius: 4px; cursor: pointer; }"
            .to_string(),
    );
    lines.push(".btn-keep { background: #7A9E7A; color: white; border: none; }".to_string());
    lines.push(
        ".btn-pin { background: #6B8FB5; color: white; border: none; margin-left: 4px; }"
            .to_string(),
    );
    lines.push("</style>".to_string());
    lines.push("</div>".to_string());

    lines.join("\n")
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jaccard_identical() {
        let a: Vec<String> = vec![
            "transformer".to_string(),
            "attention".to_string(),
            "llm".to_string(),
        ];
        let b: Vec<String> = vec![
            "transformer".to_string(),
            "attention".to_string(),
            "llm".to_string(),
        ];
        assert!((jaccard_similarity(&a, &b) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_jaccard_disjoint() {
        let a: Vec<String> = vec!["transformer".to_string()];
        let b: Vec<String> = vec!["llm".to_string()];
        assert!((jaccard_similarity(&a, &b) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_jaccard_partial() {
        let a: Vec<String> = vec![
            "transformer".to_string(),
            "attention".to_string(),
            "llm".to_string(),
        ];
        let b: Vec<String> = vec![
            "transformer".to_string(),
            "rl".to_string(),
            "policy".to_string(),
        ];
        let result = jaccard_similarity(&a, &b);
        assert!((result - 1.0 / 5.0).abs() < 1e-9);
    }

    #[test]
    fn test_jaccard_empty() {
        let empty: Vec<String> = vec![];
        let x: Vec<String> = vec!["x".to_string()];
        assert!((jaccard_similarity(&empty, &x).abs()) < 1e-9);
        assert!((jaccard_similarity(&x, &empty).abs()) < 1e-9);
    }

    #[test]
    fn test_load_capsules_missing_file() {
        let capsules = load_capsules();
        assert!(capsules.is_empty());
    }
}
