//! rairos-bridge — Cross-Domain Bridge Finder
//!
//! Finds capsule pairs from different gap types that share keywords.
//! Bridges appear when capsules from different gap types share 2+ keywords.
//!
//! Ported from `llm/cross_domain_bridge.py`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

fn jsonl_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ai_research_os")
        .join("evolution")
        .join("gene_pool.jsonl")
}

fn load_capsules() -> Vec<serde_json::Value> {
    let path = jsonl_path();
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
pub struct Bridge {
    #[serde(rename = "type_a")]
    pub type_a: String,
    #[serde(rename = "type_b")]
    pub type_b: String,
    #[serde(rename = "capsule_a")]
    pub capsule_a: String,
    #[serde(rename = "capsule_b")]
    pub capsule_b: String,
    #[serde(rename = "shared_keywords")]
    pub shared_keywords: Vec<String>,
    pub strength: f64,
}

pub fn get_bridges() -> Vec<Bridge> {
    let capsules = load_capsules();

    let mut by_type: HashMap<String, Vec<&serde_json::Value>> = HashMap::new();
    for cap in &capsules {
        let gap_type = cap
            .get("action_gap_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        by_type.entry(gap_type.to_string()).or_default().push(cap);
    }

    let mut bridges = Vec::new();
    let types: Vec<String> = by_type.keys().cloned().collect();

    for i in 0..types.len() {
        for j in (i + 1)..types.len() {
            let type_a = &types[i];
            let type_b = &types[j];

            for cap_a in by_type.get(type_a).unwrap().iter().take(5) {
                let kw_a: std::collections::HashSet<String> = cap_a
                    .get("trigger_keywords")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_lowercase()))
                            .collect()
                    })
                    .unwrap_or_default();

                let title_a = cap_a
                    .get("action_gap_title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                for cap_b in by_type.get(type_b).unwrap().iter().take(5) {
                    let kw_b: std::collections::HashSet<String> = cap_b
                        .get("trigger_keywords")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_lowercase()))
                                .collect()
                        })
                        .unwrap_or_default();

                    let shared: Vec<String> = kw_a
                        .intersection(&kw_b)
                        .cloned()
                        .collect();

                    if shared.len() >= 2 {
                        let title_b = cap_b
                            .get("action_gap_title")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        let combined: std::collections::HashSet<_> = kw_a
                            .union(&kw_b)
                            .cloned()
                            .collect();
                        let strength = (shared.len() as f64)
                            / (combined.len().max(1) as f64);

                        bridges.push(Bridge {
                            type_a: type_a.clone(),
                            type_b: type_b.clone(),
                            capsule_a: title_a.chars().take(60).collect(),
                            capsule_b: title_b.chars().take(60).collect(),
                            shared_keywords: shared.into_iter().take(5).collect(),
                            strength: (strength * 100.0).round() / 100.0,
                        });
                    }
                }
            }
        }
    }

    bridges.sort_by(|a, b| b.strength.partial_cmp(&a.strength).unwrap_or(std::cmp::Ordering::Equal));
    bridges
}

pub fn render_html(bridges: Option<&[Bridge]>) -> String {
    let bridges: Vec<Bridge> = match bridges {
        Some(b) => b.to_vec(),
        None => get_bridges(),
    };

    if bridges.is_empty() {
        let capsules = load_capsules();
        let total = capsules.len();
        let types: std::collections::HashSet<String> = capsules
            .iter()
            .filter_map(|c| {
                c.get("action_gap_type")
                    .and_then(|v| v.as_str())
                    .map(String::from)
            })
            .collect();

        return format!(
            "<div class='cross-domain'>\
             <h3>No cross-domain bridges found</h3>\
             <p>Gene Pool has {} capsules across {} gap types.</p>\
             <p>Bridges appear when capsules from different gap types share 2+ keywords.</p>\
             </div>",
            total,
            types.len()
        );
    }

    let mut html = vec!["<div class=\"cross-domain\"><h3>Cross-Domain Bridges</h3>".to_string()];

    for b in bridges.iter().take(20) {
        html.push(
            "<div style=\"border:1px solid #eee;padding:10px;margin:8px 0;border-radius:6px;\">"
                .to_string(),
        );
        html.push(format!(
            "<div style=\"font-size:11px;color:#888;\">{} ↔ {} (strength={})</div>",
            b.type_a, b.type_b, b.strength
        ));
        html.push(format!(
            "<div style=\"font-size:13px;margin:4px 0;\">{}</div>",
            &b.capsule_a[..b.capsule_a.len().min(40)]
        ));
        html.push(format!(
            "<div style=\"font-size:13px;\">{}</div>",
            &b.capsule_b[..b.capsule_b.len().min(40)]
        ));
        html.push(format!(
            "<div style=\"font-size:11px;color:#888;\">shared: {}</div>",
            b.shared_keywords.join(", ")
        ));
        html.push("</div>".to_string());
    }

    html.push("</div>".to_string());
    html.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_bridges_empty() {
        let bridges = get_bridges();
        assert!(bridges.is_empty());
    }

    #[test]
    fn test_render_html_empty() {
        let html = render_html(None);
        assert!(html.contains("No cross-domain bridges found"));
    }
}