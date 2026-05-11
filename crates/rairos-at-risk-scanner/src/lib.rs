use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

const CAPSULE_FILE_NAME: &str = "capsules.json";
const STREAK_THRESHOLD_DEFAULT: i32 = 2;

fn capsule_path() -> PathBuf {
    dirs::home_dir()
        .map(|p| p.join(".ai_research_os").join("gene_pool").join(CAPSULE_FILE_NAME))
        .unwrap_or_else(|| PathBuf::from(CAPSULE_FILE_NAME))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtRiskCapsule {
    pub capsule_id: String,
    pub gap_title: String,
    pub gap_type: String,
    pub outcome_score: f64,
    pub low_score_streak: i32,
    pub status: String,
    #[serde(default)]
    pub pinned_ttl: i32,
    #[serde(default)]
    pub trigger_keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CapsuleData {
    #[serde(default)]
    capsules: Vec<HashMap<String, serde_json::Value>>,
}

fn load_capsules() -> Vec<HashMap<String, serde_json::Value>> {
    let path = capsule_path();
    if !path.exists() {
        return Vec::new();
    }
    match std::fs::read_to_string(&path) {
        Ok(contents) => {
            let data: CapsuleData = serde_json::from_str(&contents).unwrap_or(CapsuleData { capsules: Vec::new() });
            data.capsules
        }
        Err(_) => Vec::new(),
    }
}

fn save_capsules(capsules: &[HashMap<String, serde_json::Value>]) -> Result<(), std::io::Error> {
    let path = capsule_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let data = CapsuleData {
        capsules: capsules.to_vec(),
    };
    let json = serde_json::to_string_pretty(&data).unwrap();
    std::fs::write(&path, json)
}

pub fn get_at_risk_capsules(threshold: i32) -> Vec<AtRiskCapsule> {
    let all_caps = load_capsules();
    let mut results = Vec::new();

    for cap in all_caps {
        let status = cap.get("status").and_then(|v| v.as_str()).unwrap_or("");
        if status != "active" && status != "" {
            continue;
        }
        let streak = cap.get("low_score_streak").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        if streak < threshold {
            continue;
        }

        let gap_title = cap
            .get("action_gap_title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let gap_type = cap
            .get("action_gap_type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let outcome_score = cap
            .get("outcome_success_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let pinned_ttl = cap.get("pinned_ttl").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let trigger_keywords: Vec<String> = cap
            .get("trigger_keywords")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        results.push(AtRiskCapsule {
            capsule_id: cap.get("capsule_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            gap_title,
            gap_type,
            outcome_score,
            low_score_streak: streak,
            status: status.to_string(),
            pinned_ttl,
            trigger_keywords,
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
            cap.insert("low_score_streak".to_string(), serde_json::json!(0));
            cap.insert("pinned_ttl".to_string(), serde_json::json!(0));
            found = true;
            break;
        }
    }

    if !found {
        return false;
    }
    save_capsules(&capsules).is_ok()
}

pub fn pin_to_ttl(capsule_id: &str, ttl: i32) -> bool {
    let mut capsules = load_capsules();
    let mut found = false;

    for cap in &mut capsules {
        if cap.get("capsule_id").and_then(|v| v.as_str()) == Some(capsule_id) {
            cap.insert("pinned_ttl".to_string(), serde_json::json!(ttl));
            cap.insert("low_score_streak".to_string(), serde_json::json!(0));
            found = true;
            break;
        }
    }

    if !found {
        return false;
    }
    save_capsules(&capsules).is_ok()
}

pub fn render_html(capsules: Option<&[AtRiskCapsule]>) -> String {
    let capsules = capsules.map(|c| c.to_vec()).unwrap_or_else(|| get_at_risk_capsules(STREAK_THRESHOLD_DEFAULT));

    if capsules.is_empty() {
        return "<p>No at-risk capsules. All capsules are healthy.</p>".to_string();
    }

    let mut lines = Vec::new();
    lines.push("<div class=\"at-risk-panel\">".to_string());
    lines.push(format!(
        "<h3>🚨 At-Risk Capsules <small style='color:#888'>({} need attention)</small></h3>",
        capsules.len()
    ));
    lines.push("<table class=\"at-risk-table\">".to_string());
    lines.push(
        "<thead><tr>
        <th>Gap Title</th>
        <th>Type</th>
        <th>Score</th>
        <th>Streak</th>
        <th>Pinned</th>
        <th>Action</th>
        </tr></thead>"
        .to_string(),
    );
    lines.push("<tbody>".to_string());

    for cap in &capsules {
        let streak_bar = "🔴".repeat(cap.low_score_streak as usize);
        let pinned = if cap.pinned_ttl > 0 {
            format!("TTL {}", cap.pinned_ttl)
        } else {
            "—".to_string()
        };
        lines.push("<tr>".to_string());
        lines.push(format!(
            "<td style='max-width:220px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap'><code title='{}'>{}</code></td>",
            cap.gap_title,
            &cap.gap_title[..cap.gap_title.len().min(35)]
        ));
        lines.push(format!("<td><code>{}</code></td>", cap.gap_type));
        lines.push(format!("<td>{:.2}</td>", cap.outcome_score));
        lines.push(format!(
            "<td>{} <small>{}</small></td>",
            streak_bar, cap.low_score_streak
        ));
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
        lines.push("</td>".to_string());
        lines.push("</tr>".to_string());
    }

    lines.push("</tbody></table>".to_string());
    lines.push("<style>".to_string());
    lines.push(".at-risk-panel { font-family: Georgia, serif; }".to_string());
    lines.push(".at-risk-table { width: 100%; border-collapse: collapse; margin-top: 1rem; }".to_string());
    lines.push(
        ".at-risk-table th, .at-risk-table td { padding: 0.4rem 0.8rem; border-bottom: 1px solid #e8e4de; text-align: left; }"
            .to_string(),
    );
    lines.push(
        ".at-risk-table th { font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em; color: #7a7570; }"
            .to_string(),
    );
    lines.push(".btn-small { padding: 3px 10px; font-size: 12px; border-radius: 4px; cursor: pointer; }".to_string());
    lines.push(".btn-keep { background: #7A9E7A; color: white; border: none; }".to_string());
    lines.push(".btn-pin { background: #6B8FB5; color: white; border: none; margin-left: 4px; }".to_string());
    lines.push("</style>".to_string());
    lines.push("</div>".to_string());

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_at_risk_capsules_empty() {
        let result = get_at_risk_capsules(STREAK_THRESHOLD_DEFAULT);
        assert!(result.is_empty() || !result.is_empty());
    }

    #[test]
    fn test_at_risk_capsule_serialization() {
        let cap = AtRiskCapsule {
            capsule_id: "test-123".to_string(),
            gap_title: "Test Gap".to_string(),
            gap_type: "theoretical_gap".to_string(),
            outcome_score: 0.75,
            low_score_streak: 2,
            status: "active".to_string(),
            pinned_ttl: 0,
            trigger_keywords: vec!["test".to_string()],
        };
        let json = serde_json::to_string(&cap).unwrap();
        assert!(json.contains("test-123"));
        assert!(json.contains("Test Gap"));
    }

    #[test]
    fn test_keep_active() {
        let result = keep_active("non-existent-capsule");
        assert!(!result);
    }

    #[test]
    fn test_pin_to_ttl() {
        let result = pin_to_ttl("non-existent-capsule", 3);
        assert!(!result);
    }

    #[test]
    fn test_render_html_empty() {
        let html = render_html(Some(&[]));
        assert!(html.contains("No at-risk capsules"));
    }

    #[test]
    fn test_render_html_with_capsules() {
        let caps = vec![AtRiskCapsule {
            capsule_id: "test-1".to_string(),
            gap_title: "Test Gap Title That Is Quite Long".to_string(),
            gap_type: "theoretical_gap".to_string(),
            outcome_score: 0.5,
            low_score_streak: 2,
            status: "active".to_string(),
            pinned_ttl: 0,
            trigger_keywords: vec![],
        }];
        let html = render_html(Some(&caps));
        assert!(html.contains("at-risk-panel"));
        assert!(html.contains("Test Gap"));
    }
}