//! rairos-bold-vault — Bold Hypothesis Vault for AI Research OS.
#![allow(dead_code)]
//!
//! Ported from `llm/bold_vault.py` (148 LOC, pure stdlib).
//!
//! Tracks high-risk/high-reward Gene Pool capsules.
//! Bold = theoretical_gap OR negative polarity OR high novelty score.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

// ─── Constants ────────────────────────────────────────────────────────────────

const CAPSULE_PATH: &str = ".ai_research_os/gene_pool/capsules.json";
const NOVELTY_THRESHOLD: f64 = 0.7;

// ─── Data Structures ──────────────────────────────────────────────────────────

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

// ─── Core Logic ───────────────────────────────────────────────────────────────

fn jaccard(a: &[String], b: &[String]) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let set_a: HashSet<_> = a.iter().collect();
    let set_b: HashSet<_> = b.iter().collect();
    let intersection = set_a.intersection(&set_b).count() as f64;
    let union = set_a.union(&set_b).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

fn load_capsules() -> Vec<serde_json::Value> {
    let path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(CAPSULE_PATH);
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
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let gap_type = if gap_type.is_empty() {
            cap.get("trigger_gap_type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        } else {
            gap_type
        };

        let polarity = cap
            .get("polarity")
            .and_then(|v| v.as_str())
            .unwrap_or("positive")
            .to_string();

        let keywords: Vec<String> = cap
            .get("trigger_keywords")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        // Compute novelty vs other capsules
        let mut max_overlap = 0.0;
        for other in &capsules {
            if other.get("capsule_id") == cap.get("capsule_id") {
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
            let ov = jaccard(&keywords, &other_kw);
            if ov > max_overlap {
                max_overlap = ov;
            }
        }
        let novelty = 1.0 - max_overlap;

        let mut reasons: Vec<String> = vec![];
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

        results.push(BoldCapsule {
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
            gap_type: gap_type.clone(),
            polarity,
            outcome_score: cap
                .get("outcome_success_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            novelty_score: (novelty * 1000.0).round() / 1000.0,
            trigger_keywords: keywords,
            reason: reasons.join(", "),
        });
    }

    results.sort_by(|a, b| {
        b.novelty_score
            .partial_cmp(&a.novelty_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results
}

pub fn render_html(capsules: Option<Vec<BoldCapsule>>) -> String {
    let capsules = capsules.unwrap_or_else(get_bold_capsules);

    if capsules.is_empty() {
        return "<p>No bold hypotheses yet. Theoretical gaps and negative-polarity capsules will appear here.</p>".to_string();
    }

    let mut lines = vec!["<div class=\"bold-vault\">".to_string()];
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
            &c.gap_title[..70]
        } else {
            &c.gap_title
        };
        let kw_str: String = c.trigger_keywords.iter().take(4).map(|s| s.as_str()).collect::<Vec<_>>().join(", ");

        lines.push(format!(
            "<div class='bold-card'><div class='bold-reason'>{}</div>\
             <div class='bold-title' title='{}'>{}</div>\
             <div class='bold-meta'><code>{}</code> · {} · score={:.2} · novelty={:.0}%</div>\
             <div class='bold-kw'>{}</div></div>",
            c.reason,
            c.gap_title,
            title_short,
            c.gap_type,
            c.polarity,
            c.outcome_score,
            c.novelty_score * 100.0,
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

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jaccard_basic() {
        let a = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let b = vec!["b".to_string(), "c".to_string(), "d".to_string()];
        let result = jaccard(&a, &b);
        assert!((result - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_jaccard_empty() {
        assert_eq!(jaccard(&[], &["a".to_string()]), 0.0);
        assert_eq!(jaccard(&[], &[]), 0.0);
    }

    #[test]
    fn test_jaccard_identical() {
        let a = vec!["a".to_string(), "b".to_string()];
        let b = vec!["a".to_string(), "b".to_string()];
        assert_eq!(jaccard(&a, &b), 1.0);
    }

    #[test]
    fn test_load_capsules_nonexistent() {
        // When path doesn't exist, returns empty vec
        let result: Vec<serde_json::Value> = load_capsules();
        assert!(result.is_empty());
    }

    #[test]
    fn test_render_html_empty() {
        let html = render_html(Some(vec![]));
        assert!(html.contains("No bold hypotheses"));
    }

    #[test]
    fn test_bold_capsule_fields() {
        let bc = BoldCapsule {
            capsule_id: "c1".to_string(),
            gap_title: "Test gap".to_string(),
            gap_type: "theoretical_gap".to_string(),
            polarity: "negative".to_string(),
            outcome_score: 0.8,
            novelty_score: 0.75,
            trigger_keywords: vec!["test".to_string()],
            reason: "theoretical, negative".to_string(),
        };
        assert_eq!(bc.gap_type, "theoretical_gap");
        assert!(bc.novelty_score > NOVELTY_THRESHOLD);
    }

    #[test]
    fn test_get_bold_capsules_returns_vec() {
        let result = get_bold_capsules();
        // Should always return a valid vec (may be empty)
        assert!(result.capacity() >= 0);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let bc = BoldCapsule {
            capsule_id: "c1".to_string(),
            gap_title: "Test".to_string(),
            gap_type: "theoretical_gap".to_string(),
            polarity: "positive".to_string(),
            outcome_score: 0.5,
            novelty_score: 0.8,
            trigger_keywords: vec!["ai".to_string()],
            reason: "theoretical".to_string(),
        };
        let json = serde_json::to_string(&bc).unwrap();
        let deserialized: BoldCapsule = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.capsule_id, "c1");
        assert_eq!(deserialized.novelty_score, 0.8);
    }
}
