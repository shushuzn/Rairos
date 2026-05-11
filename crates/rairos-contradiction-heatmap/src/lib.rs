//! rairos-contradiction-heatmap — Contradiction Heatmap
//!
//! Computes per-paper contradiction counts from Gene Pool capsules.
//! Papers are linked via archetype.source_paper_id.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

fn gene_pool_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ai_research_os")
        .join("evolution")
        .join("gene_pool.jsonl")
}

pub fn load_capsules() -> Vec<serde_json::Value> {
    let path = gene_pool_path();
    if !path.exists() {
        return Vec::new();
    }
    let text = match fs::read_to_string(&path) {
        Ok(t) => t.trim().to_string(),
        Err(_) => return Vec::new(),
    };
    if text.is_empty() {
        return Vec::new();
    }
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContradictionEntry {
    pub gap_type: String,
    pub partner_id: String,
    pub polarity: String,
    pub shared_keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperContradictionInfo {
    pub count: usize,
    pub contradictions: Vec<ContradictionEntry>,
}

pub type ContradMap = HashMap<String, PaperContradictionInfo>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapsulesWithPolarity {
    #[serde(rename = "positive_capsule")]
    pub positive_capsule: serde_json::Value,
    #[serde(rename = "negative_capsule")]
    pub negative_capsule: serde_json::Value,
    #[serde(rename = "gap_type")]
    pub gap_type: String,
    #[serde(rename = "shared_keywords")]
    pub shared_keywords: Vec<String>,
}

pub fn detect_contradictions_full(capsules: &[serde_json::Value]) -> Vec<CapsulesWithPolarity> {
    use std::collections::HashSet;

    let mut by_type: HashMap<String, Vec<&serde_json::Value>> = HashMap::new();
    for c in capsules {
        if let Some(gt) = c
            .get("trigger_gap_type")
            .or_else(|| c.get("action_gap_type"))
            .and_then(|v| v.as_str())
        {
            by_type.entry(gt.to_string()).or_default().push(c);
        }
    }

    let mut results: Vec<CapsulesWithPolarity> = Vec::new();

    for (gt, caps) in &by_type {
        for i in 0..caps.len() {
            let p_i = caps[i]
                .get("polarity")
                .and_then(|v| v.as_str())
                .unwrap_or("open");

            for j in (i + 1)..caps.len() {
                let p_j = caps[j]
                    .get("polarity")
                    .and_then(|v| v.as_str())
                    .unwrap_or("open");

                if p_i != p_j && p_i != "open" && p_j != "open" {
                    let (pos, neg) = if p_i == "positive" {
                        (caps[i].clone(), caps[j].clone())
                    } else {
                        (caps[j].clone(), caps[i].clone())
                    };

                    let kw_pos: HashSet<String> = pos
                        .get("trigger_keywords")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_lowercase()))
                                .collect()
                        })
                        .unwrap_or_default();

                    let kw_neg: HashSet<String> = neg
                        .get("trigger_keywords")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_lowercase()))
                                .collect()
                        })
                        .unwrap_or_default();

                    let shared: Vec<String> = kw_pos.intersection(&kw_neg).cloned().collect();

                    results.push(CapsulesWithPolarity {
                        positive_capsule: pos,
                        negative_capsule: neg,
                        gap_type: gt.to_string(),
                        shared_keywords: shared,
                    });
                }
            }
        }
    }

    results
}

pub fn compute_paper_contradictions() -> ContradMap {
    let capsules = load_capsules();
    let contrad = detect_contradictions_full(&capsules);

    let mut by_paper: HashMap<String, PaperContradictionInfo> = HashMap::new();

    for c in &contrad {
        let pos_id = c
            .positive_capsule
            .get("archetype")
            .and_then(|a| a.get("source_paper_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("?")
            .to_string();

        let neg_id = c
            .negative_capsule
            .get("archetype")
            .and_then(|a| a.get("source_paper_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("?")
            .to_string();

        let gap_type = c.gap_type.clone();
        let shared = c.shared_keywords.clone();

        for (pid, partner_id, polarity) in [
            (pos_id.clone(), neg_id.clone(), "positive"),
            (neg_id, pos_id, "negative"),
        ] {
            if pid != "?" {
                let entry = by_paper.entry(pid).or_insert_with(|| PaperContradictionInfo {
                    count: 0,
                    contradictions: Vec::new(),
                });
                entry.count += 1;
                entry.contradictions.push(ContradictionEntry {
                    gap_type: gap_type.clone(),
                    partner_id,
                    polarity: polarity.to_string(),
                    shared_keywords: shared.clone(),
                });
            }
        }
    }

    by_paper
}

fn badge_color(count: usize) -> &'static str {
    match count {
        0 => "#e8e4de",
        1 => "#f5d76e",
        2 => "#e67e22",
        _ => "#e74c3c",
    }
}

pub fn render_heatmap_html(
    papers: &[serde_json::Value],
    contrad_map: &ContradMap,
) -> String {
    if papers.is_empty() {
        return "<p>No papers yet.</p>".to_string();
    }

    let mut lines: Vec<String> = vec!["<div class=\"heatmap-grid\">".to_string()];

    for p in papers {
        let pid = p.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let info = contrad_map.get(pid);
        let count = info.map(|i| i.count).unwrap_or(0);
        let bg = badge_color(count);
        let color = if count >= 2 { "#fff" } else { "#555" };

        let tooltip_lines: Vec<String> = info
            .map(|i| {
                i.contradictions
                    .iter()
                    .take(5)
                    .map(|c| {
                        let polarity_short = c.polarity.chars().take(3).collect::<String>().to_uppercase();
                        let partner_short = if c.partner_id.len() > 12 {
                            format!("{}...", &c.partner_id[..12])
                        } else {
                            c.partner_id.clone()
                        };
                        format!(
                            "• {} {} (→ {}) kw={}",
                            polarity_short,
                            c.gap_type,
                            partner_short,
                            c.shared_keywords.join(",")
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();

        let tooltip_text = if tooltip_lines.is_empty() {
            "No contradictions".to_string()
        } else {
            tooltip_lines.join(" | ")
        };

        let title_short = p
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .chars()
            .take(60)
            .collect::<String>();

        let primary_cat = p
            .get("primary_category")
            .and_then(|v| v.as_str())
            .unwrap_or("?");

        let border_color = if count >= 3 { "#c0392b" } else { "#bdc3c7" };

        lines.push(format!(
            "<div class=\"heatmap-card\" \
             style=\"background:{};color:{};border-color:{}\" \
             title=\"{}\">\
             <div class=\"heatmap-card-cat\">{}</div>\
             <div class=\"heatmap-card-title\">{}</div>\
             <div class=\"heatmap-card-count\">{} &#128293;</div>\
             </div>",
            bg,
            color,
            border_color,
            tooltip_text,
            primary_cat,
            title_short,
            count
        ));
    }

    lines.push("</div>".to_string());
    lines.push("<style>".to_string());
    lines.push(".heatmap-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 10px; }".to_string());
    lines.push(".heatmap-card { border-radius: 6px; padding: 12px; border: 1.5px solid #bdc3c7; cursor: help; transition: transform 0.1s; }".to_string());
    lines.push(".heatmap-card:hover { transform: scale(1.02); z-index: 1; position: relative; }".to_string());
    lines.push(".heatmap-card-cat { font-size: 10px; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 4px; opacity: 0.8; }".to_string());
    lines.push(".heatmap-card-title { font-size: 12px; font-weight: 600; line-height: 1.4; margin-bottom: 6px; }".to_string());
    lines.push(".heatmap-card-count { font-size: 11px; font-weight: 700; text-align: right; }".to_string());
    lines.push("</style>".to_string());

    lines.join("\n")
}

pub fn detect_contradictions(papers: &[serde_json::Value]) -> Vec<serde_json::Value> {
    detect_contradictions_full(papers)
        .into_iter()
        .filter_map(|c| serde_json::to_value(c).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_badge_color() {
        assert_eq!(badge_color(0), "#e8e4de");
        assert_eq!(badge_color(1), "#f5d76e");
        assert_eq!(badge_color(2), "#e67e22");
        assert_eq!(badge_color(5), "#e74c3c");
    }

    #[test]
    fn test_compute_paper_contradictions_empty() {
        let result = compute_paper_contradictions();
        assert!(result.is_empty());
    }

    #[test]
    fn test_render_heatmap_html_empty() {
        let html = render_heatmap_html(&[], &HashMap::new());
        assert!(html.contains("No papers"));
    }

    #[test]
    fn test_detect_contradictions_empty() {
        let result = detect_contradictions(&[]);
        assert!(result.is_empty());
    }
}
