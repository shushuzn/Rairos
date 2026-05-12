use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

const CAPSULE_FILE_NAME: &str = "capsules.json";
const NOVELTY_THRESHOLD: f64 = 0.7;

fn capsule_path() -> PathBuf {
    dirs::home_dir()
        .map(|p| p.join(".ai_research_os").join("gene_pool").join(CAPSULE_FILE_NAME))
        .unwrap_or_else(|| PathBuf::from(CAPSULE_FILE_NAME))
}

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

#[derive(Debug, Clone, Deserialize)]
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
            let data: CapsuleData =
                serde_json::from_str(&contents).unwrap_or(CapsuleData { capsules: Vec::new() });
            data.capsules
        }
        Err(_) => Vec::new(),
    }
}

pub fn jaccard(a: &[String], b: &[String]) -> f64 {
    let s_a: HashSet<_> = a.iter().collect();
    let s_b: HashSet<_> = b.iter().collect();
    if s_a.is_empty() || s_b.is_empty() {
        return 0.0;
    }
    let intersection = s_a.intersection(&s_b).count();
    let union = s_a.union(&s_b).count();
    intersection as f64 / union as f64
}

pub fn get_bold_capsules() -> Vec<BoldCapsule> {
    let capsules = load_capsules();
    let mut results: Vec<BoldCapsule> = Vec::new();

    for cap in &capsules {
        let status = cap.get("status").and_then(|v| v.as_str()).unwrap_or("");
        if status != "active" && !status.is_empty() {
            continue;
        }

        let gap_type = cap
            .get("action_gap_type")
            .or_else(|| cap.get("trigger_gap_type"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let polarity = cap.get("polarity").and_then(|v| v.as_str()).unwrap_or("positive");
        let trigger_keywords: Vec<String> = cap
            .get("trigger_keywords")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let mut max_overlap = 0.0f64;
        for other in &capsules {
            if other.get("capsule_id") == cap.get("capsule_id") {
                continue;
            }
            let other_keywords: Vec<String> = other
                .get("trigger_keywords")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let ov = jaccard(&trigger_keywords, &other_keywords);
            if ov > max_overlap {
                max_overlap = ov;
            }
        }
        let novelty = 1.0 - max_overlap;

        let mut reasons: Vec<String> = Vec::new();
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

        let gap_title = cap
            .get("action_gap_title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let outcome_score = cap
            .get("outcome_success_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        results.push(BoldCapsule {
            capsule_id: cap.get("capsule_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            gap_title,
            gap_type: gap_type.to_string(),
            polarity: polarity.to_string(),
            outcome_score,
            novelty_score: (novelty * 1000.0).round() / 1000.0,
            trigger_keywords,
            reason: reasons.join(", "),
        });
    }

    results.sort_by(|a, b| b.novelty_score.partial_cmp(&a.novelty_score).unwrap_or(std::cmp::Ordering::Equal));
    results
}

pub fn render_html(capsules: Option<&[BoldCapsule]>) -> String {
    let capsules = capsules.map(|c| c.to_vec()).unwrap_or_else(get_bold_capsules);

    if capsules.is_empty() {
        return "<p>No bold hypotheses yet. Theoretical gaps and negative-polarity capsules will appear here.</p>".to_string();
    }

    let mut lines = Vec::new();
    lines.push("<div class=\"bold-vault\">".to_string());
    lines.push(
        "<h3>🔴 Bold Hypothesis Vault <small style='color:#888'>(high-risk / high-reward gaps)</small></h3>".to_string(),
    );
    lines.push(format!(
        "<p style='font-size:13px;color:#A89E8C;margin-bottom:16px'>{} bold capsules tracked.</p>",
        capsules.len()
    ));
    lines.push("<div class='bold-grid'>".to_string());

    for c in &capsules {
        let title_short = if c.gap_title.len() > 70 {
            c.gap_title[..70].to_string()
        } else {
            c.gap_title.clone()
        };
        let kw_str = c.trigger_keywords.iter().take(4).map(|s| s.as_str()).collect::<Vec<_>>().join(", ");

        let html = format!(
            "<div class='bold-card'>\
<div class='bold-reason'>{}</div>\
<div class='bold-title' title='{}'>{}</div>\
<div class='bold-meta'>\
<code>{}</code> · {} · score={:.2} · novelty={:.0}%\
</div>\
<div class='bold-kw'>{}</div>\
</div>",
            c.reason,
            c.gap_title,
            title_short,
            c.gap_type,
            c.polarity,
            c.outcome_score,
            c.novelty_score * 100.0,
            kw_str
        );
        lines.push(html);
    }

    lines.push("</div>".to_string());
    lines.push("<style>".to_string());
    lines.push(".bold-vault { font-family: Georgia, serif; }".to_string());
    lines.push(
        ".bold-card { border: 2px solid #C4706A; border-radius: 6px; padding: 12px 14px; margin-bottom: 10px; background: rgba(196,112,106,0.06); }".to_string(),
    );
    lines.push(
        ".bold-reason { font-size: 10px; text-transform: uppercase; letter-spacing: 0.5px; color: #C4706A; font-weight: 700; margin-bottom: 4px; }".to_string(),
    );
    lines.push(
        ".bold-title { font-size: 14px; font-weight: 600; color: #2a2a2a; margin-bottom: 6px; line-height: 1.4; }".to_string(),
    );
    lines.push(".bold-meta { font-size: 11px; color: #7a7570; margin-bottom: 4px; }".to_string());
    lines.push(".bold-kw { font-size: 11px; color: #A89E8C; font-style: italic; }".to_string());
    lines.push("</style>".to_string());
    lines.push("</div>".to_string());

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jaccard() {
        let a = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let b = vec!["b".to_string(), "c".to_string(), "d".to_string()];
        let score = jaccard(&a, &b);
        assert!((score - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_jaccard_empty() {
        let a: Vec<String> = vec![];
        let b = vec!["a".to_string()];
        assert_eq!(jaccard(&a, &b), 0.0);
    }

    #[test]
    fn test_jaccard_full_overlap() {
        let a = vec!["a".to_string(), "b".to_string()];
        let b = vec!["a".to_string(), "b".to_string()];
        assert_eq!(jaccard(&a, &b), 1.0);
    }

    #[test]
    fn test_bold_capsule_serialization() {
        let cap = BoldCapsule {
            capsule_id: "test-123".to_string(),
            gap_title: "Test Gap".to_string(),
            gap_type: "theoretical_gap".to_string(),
            polarity: "negative".to_string(),
            outcome_score: 0.8,
            novelty_score: 0.75,
            trigger_keywords: vec!["test".to_string()],
            reason: "theoretical, negative, high-novelty(75%)".to_string(),
        };
        let json = serde_json::to_string(&cap).unwrap();
        assert!(json.contains("test-123"));
        assert!(json.contains("theoretical_gap"));
    }

    #[test]
    fn test_render_html_empty() {
        let html = render_html(Some(&[]));
        assert!(html.contains("No bold hypotheses"));
    }

    #[test]
    fn test_render_html_with_capsules() {
        let caps = vec![BoldCapsule {
            capsule_id: "test-1".to_string(),
            gap_title: "Test Bold Gap Title".to_string(),
            gap_type: "theoretical_gap".to_string(),
            polarity: "negative".to_string(),
            outcome_score: 0.8,
            novelty_score: 0.75,
            trigger_keywords: vec!["ai".to_string(), "safety".to_string()],
            reason: "theoretical".to_string(),
        }];
        let html = render_html(Some(&caps));
        assert!(html.contains("bold-vault"));
        assert!(html.contains("Test Bold Gap Title"));
    }
}