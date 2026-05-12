//! rairos-cross-domain-bridge — Cross-domain bridge finder
//!
//! Finds bridges between Gene Pool capsules from different gap types
//! that share keywords.

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

fn load_capsules() -> Vec<serde_json::Value> {
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
pub struct Bridge {
    pub type_a: String,
    pub type_b: String,
    pub capsule_a: String,
    pub capsule_b: String,
    pub shared_keywords: Vec<String>,
    pub strength: f64,
}

pub fn get_bridges() -> Vec<Bridge> {
    let capsules = load_capsules();

    let mut by_type: HashMap<String, Vec<&serde_json::Value>> = HashMap::new();
    for c in &capsules {
        let t = c
            .get("action_gap_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if !t.is_empty() {
            by_type.entry(t.to_string()).or_default().push(c);
        }
    }

    let mut bridges = Vec::new();
    let types: Vec<String> = by_type.keys().cloned().collect();

    for i in 0..types.len() {
        for j in (i + 1)..types.len() {
            let type_a = &types[i];
            let type_b = &types[j];

            for ca in by_type[type_a].iter().take(5) {
                for cb in by_type[type_b].iter().take(5) {
                    let kw_a: std::collections::HashSet<String> = ca
                        .get("trigger_keywords")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str())
                                .map(|s| s.to_lowercase())
                                .collect()
                        })
                        .unwrap_or_default();

                    let kw_b: std::collections::HashSet<String> = cb
                        .get("trigger_keywords")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str())
                                .map(|s| s.to_lowercase())
                                .collect()
                        })
                        .unwrap_or_default();

                    let shared: std::collections::HashSet<String> =
                        kw_a.intersection(&kw_b).cloned().collect();
                    if shared.len() >= 2 {
                        let title_a = ca
                            .get("action_gap_title")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .chars()
                            .take(60)
                            .collect::<String>();
                        let title_b = cb
                            .get("action_gap_title")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .chars()
                            .take(60)
                            .collect::<String>();

                        let all_kw: Vec<String> = kw_a.union(&kw_b).cloned().collect();
                        let strength =
                            (shared.len() as f64) / (all_kw.len().max(1) as f64) * 100.0 / 100.0;

                        bridges.push(Bridge {
                            type_a: type_a.clone(),
                            type_b: type_b.clone(),
                            capsule_a: title_a,
                            capsule_b: title_b,
                            shared_keywords: shared.iter().take(5).cloned().collect(),
                            strength: (strength * 100.0).round() / 100.0,
                        });
                    }
                }
            }
        }
    }

    bridges
}

pub fn render_html(bridges: &[Bridge]) -> String {
    if bridges.is_empty() {
        let capsules = load_capsules();
        let total = capsules.len();
        let mut types = std::collections::HashSet::new();
        for c in &capsules {
            if let Some(t) = c.get("action_gap_type").and_then(|v| v.as_str()) {
                types.insert(t);
            }
        }
        return format!(
            "<div class='cross-domain'><h3>No cross-domain bridges found</h3><p>Gene Pool has {} capsules across {} gap types.</p><p>Bridges appear when capsules from different gap types share 2+ keywords.</p></div>",
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
            "<div style=\"font-size:11px;color:#888;\">{} &#8596; {} (strength={})</div>",
            b.type_a, b.type_b, b.strength
        ));
        html.push(format!(
            "<div style=\"font-size:13px;margin:4px 0;\">{}</div>",
            if b.capsule_a.len() > 40 {
                &b.capsule_a[..40]
            } else {
                &b.capsule_a
            }
        ));
        html.push(format!(
            "<div style=\"font-size:13px;\">{}</div>",
            if b.capsule_b.len() > 40 {
                &b.capsule_b[..40]
            } else {
                &b.capsule_b
            }
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
        let html = render_html(&[]);
        assert!(html.contains("No cross-domain bridges"));
    }

    #[test]
    fn test_render_html_with_bridges() {
        let bridges = vec![Bridge {
            type_a: "method_gap".to_string(),
            type_b: "theoretical_gap".to_string(),
            capsule_a: "Bridge A".to_string(),
            capsule_b: "Bridge B".to_string(),
            shared_keywords: vec!["test".to_string(), "keyword".to_string()],
            strength: 0.5,
        }];
        let html = render_html(&bridges);
        assert!(html.contains("Cross-Domain Bridges"));
        assert!(html.contains("method_gap"));
    }
}
