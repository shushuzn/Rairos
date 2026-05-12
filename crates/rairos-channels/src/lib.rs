//! rairos-channels — arXiv Alert Channels
//!
//! Multiple feed configurations with different matching criteria for arXiv paper alerts.
//! Channels: general, climate, ai_safety, regulation.
//!
//! Ported from `llm/arxiv_alert_channels.py`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

fn channels_file() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ai_research_os")
        .join("arxiv_channels.json")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    pub id: String,
    pub name: String,
    pub categories: Vec<String>,
    pub keywords: Vec<String>,
    pub priority: i32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChannelData {
    #[serde(rename = "name")]
    name: String,
    #[serde(rename = "categories")]
    categories: Vec<String>,
    #[serde(rename = "keywords")]
    keywords: Vec<String>,
    #[serde(rename = "priority")]
    priority: i32,
    #[serde(rename = "enabled")]
    enabled: bool,
}

fn default_channels() -> HashMap<String, ChannelData> {
    let mut m = HashMap::new();
    m.insert(
        "general".to_string(),
        ChannelData {
            name: "General AI/ML".to_string(),
            categories: vec![
                "cs.AI".to_string(),
                "cs.LG".to_string(),
                "cs.CL".to_string(),
                "cs.CV".to_string(),
                "cs.NE".to_string(),
            ],
            keywords: Vec::new(),
            priority: 1,
            enabled: true,
        },
    );
    m.insert(
        "climate".to_string(),
        ChannelData {
            name: "Climate AI".to_string(),
            categories: vec![
                "cs.AI".to_string(),
                "cs.LG".to_string(),
                "cs.ET".to_string(),
                "envir.ArXiv".to_string(),
            ],
            keywords: vec![
                "climate".to_string(),
                "carbon".to_string(),
                "emissions".to_string(),
                "renewable".to_string(),
                "energy".to_string(),
                "sustainability".to_string(),
                "green AI".to_string(),
            ],
            priority: 3,
            enabled: true,
        },
    );
    m.insert(
        "ai_safety".to_string(),
        ChannelData {
            name: "AI Safety".to_string(),
            categories: vec!["cs.AI".to_string(), "cs.LG".to_string()],
            keywords: vec![
                "safety".to_string(),
                "alignment".to_string(),
                "robustness".to_string(),
                "interpretability".to_string(),
                "fairness".to_string(),
                "trustworthy".to_string(),
                "hazard".to_string(),
                "risk".to_string(),
            ],
            priority: 3,
            enabled: true,
        },
    );
    m.insert(
        "regulation".to_string(),
        ChannelData {
            name: "AI Regulation".to_string(),
            categories: vec![
                "cs.AI".to_string(),
                "cs.CY".to_string(),
                "cs.SI".to_string(),
            ],
            keywords: vec![
                "regulation".to_string(),
                "policy".to_string(),
                "governance".to_string(),
                "law".to_string(),
                "GDPR".to_string(),
                "compliance".to_string(),
                "legal".to_string(),
                "legislation".to_string(),
            ],
            priority: 2,
            enabled: true,
        },
    );
    m
}

fn load_channels() -> HashMap<String, ChannelData> {
    let path = channels_file();
    if !path.exists() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&default_channels()) {
            let _ = fs::write(&path, json);
        }
        return default_channels();
    }

    match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|_| default_channels()),
        Err(_) => default_channels(),
    }
}

fn save_channels(channels: &HashMap<String, ChannelData>) {
    let path = channels_file();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(channels) {
        let _ = fs::write(&path, json);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paper {
    pub title: Option<String>,
    #[serde(rename = "abstract")]
    pub abstract_text: Option<String>,
    pub categories: Option<Vec<String>>,
    pub published: Option<String>,
    pub score: Option<f64>,
}

impl Paper {
    fn abstract_lowercase(&self) -> String {
        let abs = self.abstract_text.as_deref().unwrap_or("");
        let title = self.title.as_deref().unwrap_or("");
        format!("{} {}", title.to_lowercase(), abs.to_lowercase())
    }
}

pub fn match_paper_to_channels(paper: &Paper) -> Vec<String> {
    let channels = load_channels();
    let cats: std::collections::HashSet<_> = paper
        .categories
        .as_ref()
        .map(|v| v.iter().collect())
        .unwrap_or_default();
    let abstract_text = paper.abstract_lowercase();
    let mut matched = Vec::new();

    for (cid, cfg) in &channels {
        if !cfg.enabled {
            continue;
        }
        if cfg.categories.iter().any(|c| cats.contains(c)) {
            matched.push(cid.clone());
            continue;
        }
        if !cfg.keywords.is_empty()
            && cfg
                .keywords
                .iter()
                .any(|kw| abstract_text.contains(&kw.to_lowercase()))
        {
            matched.push(cid.clone());
        }
    }

    matched
}

pub fn get_channels() -> Vec<ChannelConfig> {
    let channels = load_channels();
    channels
        .into_iter()
        .map(|(cid, cfg)| ChannelConfig {
            id: cid,
            name: cfg.name,
            categories: cfg.categories,
            keywords: cfg.keywords,
            priority: cfg.priority,
            enabled: cfg.enabled,
        })
        .collect()
}

pub fn update_channel(cid: &str, updates: HashMap<String, serde_json::Value>) -> bool {
    let mut channels = load_channels();
    if !channels.contains_key(cid) {
        return false;
    }
    if let Some(name) = updates.get("name").and_then(|v| v.as_str()) {
        channels.get_mut(cid).unwrap().name = name.to_string();
    }
    if let Some(cats) = updates.get("categories").and_then(|v| v.as_array()) {
        channels.get_mut(cid).unwrap().categories = cats
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
    }
    if let Some(kws) = updates.get("keywords").and_then(|v| v.as_array()) {
        channels.get_mut(cid).unwrap().keywords = kws
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
    }
    if let Some(pri) = updates.get("priority").and_then(|v| v.as_i64()) {
        channels.get_mut(cid).unwrap().priority = pri as i32;
    }
    if let Some(en) = updates.get("enabled").and_then(|v| v.as_bool()) {
        channels.get_mut(cid).unwrap().enabled = en;
    }
    save_channels(&channels);
    true
}

pub fn toggle_channel(cid: &str) -> bool {
    let mut channels = load_channels();
    if let Some(cfg) = channels.get_mut(cid) {
        cfg.enabled = !cfg.enabled;
        save_channels(&channels);
        true
    } else {
        false
    }
}

#[allow(clippy::vec_init_then_push)]
pub fn render_channels_html(check_results: Option<&HashMap<String, Vec<Paper>>>) -> String {
    let channels = get_channels();
    let results: HashMap<String, Vec<Paper>> = check_results.cloned().unwrap_or_default();

    let mut lines: Vec<String> = {
        let mut v = Vec::new();
        v.push("<div class=\"channels-panel\">".to_string());
        v.push("<h3>📡 arXiv Watch Alert Channels</h3>".to_string());
        v.push(
            "<p style='font-size:13px;color:#A89E8C;margin-bottom:16px'>\
             Configure multiple feed channels with different matching criteria. \
             Higher priority = shown first in alerts.</p>"
                .to_string(),
        );
        v.push(
            "<div style=\"margin-bottom: 20px;\">\
               <button id=\"run-check-btn\" onclick=\"runCheck()\" style=\"\
                 background: #1a73e8; color: #fff; border: none; border-radius: 6px;\
                 padding: 10px 20px; font-size: 14px; cursor: pointer; font-family: Georgia, serif;\">\
                 🔍 Run Check Now\
               </button>\
               <span id=\"check-status\" style=\"font-size:13px;color:#888;margin-left:12px;display:none;\"></span>\
             </div>\
             <div id=\"check-results\"></div>"
                .to_string(),
        );
        v
    };

    for ch in &channels {
        let color = match ch.priority {
            3 => "#C4706A",
            2 => "#D4A055",
            _ => "#6B8FB5",
        };
        let status = if ch.enabled {
            "✅ Enabled"
        } else {
            "❌ Disabled"
        };
        let kw_str = ch
            .keywords
            .iter()
            .take(6)
            .map(|k| format!("<code>{}</code>", k))
            .collect::<Vec<_>>()
            .join(", ");
        let cat_str = ch
            .categories
            .iter()
            .take(4)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");

        let channel_results = results.get(&ch.id);
        let result_rows = if let Some(rps) = channel_results {
            let mut rows = String::new();
            for rp in rps.iter().take(5) {
                let title = rp
                    .title
                    .as_deref()
                    .unwrap_or("")
                    .chars()
                    .take(80)
                    .collect::<String>();
                let published = rp.published.as_deref().unwrap_or("");
                let score = rp.score.unwrap_or(0.0);
                rows.push_str(&format!(
                    "<div style=\"display:flex;gap:8px;align-items:flex-start;padding:6px 0;border-bottom:1px solid #f0ebe5;\">\
                       <span style=\"color:#4CAF50;font-size:12px;\">●</span>\
                       <div style=\"flex:1;\">\
                         <div style=\"font-size:12px;color:#2a2a2a;font-weight:600;\">{}</div>\
                         <div style=\"font-size:11px;color:#888;\">{} · score={:.2}</div>\
                       </div>\
                     </div>",
                    title, published, score
                ));
            }
            if rows.is_empty() {
                "<div style='font-size:12px;color:#bbb;padding:4px 0;'>No new papers in last check</div>".to_string()
            } else {
                rows
            }
        } else {
            "<div style='font-size:12px;color:#bbb;padding:4px 0;'>No new papers in last check</div>".to_string()
        };

        lines.push(format!(
            "<div style='border: 1px solid #e0dbd4; border-radius: 6px; padding: 14px; margin-bottom: 12px; border-left: 4px solid {};'>\
               <div style='display: flex; justify-content: space-between; align-items: center; margin-bottom: 6px;'>\
                 <div style='font-weight: 700; font-size: 14px; color: #2a2a2a'>{}</div>\
                 <div style='font-size: 11px; color: #A89E8C'>priority {} · {}</div>\
               </div>\
               <div style='font-size: 12px; color: #7a7570; margin-bottom: 4px'>Categories: {}</div>\
               <div style='font-size: 12px; color: #A89E8C; margin-bottom: 8px'>Keywords: {}</div>\
               <div style='margin-bottom: 10px; padding: 8px; background: #faf9f7; border-radius: 4px;'>\
                 <div style='font-size:11px;color:#888;margin-bottom:6px;'>Recent papers from this channel:</div>\
                 {}\
               </div>\
               <div style='display: flex; gap: 8px;'>\
                 <button onclick=\"toggleChannel('{}')\" style=\"font-size: 11px; padding: 3px 10px; cursor: pointer; border-radius: 3px; border: 1px solid #ccc; background: transparent\">\
                   Toggle\
                 </button>\
               </div>\
             </div>",
            color, ch.name, ch.priority, status,
            cat_str,
            if kw_str.is_empty() { "(none)".to_string() } else { kw_str },
            result_rows,
            ch.id
        ));
    }

    lines.push(
        "<script>\
         function toggleChannel(cid) {\
             fetch('/arxiv-channels/toggle/' + cid, {method: 'POST'})\
               .then(function(r) { return r.json(); })\
               .then(function(d) { if (d.success) location.reload(); });\
         }\
         function runCheck() {\
             var btn = document.getElementById('run-check-btn');\
             var status = document.getElementById('check-status');\
             btn.disabled = true;\
             btn.textContent = '⏳ Checking...';\
             status.style.display = 'inline';\
             status.textContent = 'Querying arXiv...';\
             fetch('/arxiv-channels/check', {method: 'POST'})\
               .then(function(r) { return r.json(); })\
               .then(function(d) {\
                   btn.disabled = false;\
                   btn.textContent = '🔍 Run Check Now';\
                   status.textContent = '';\
                   location.reload();\
               })\
               .catch(function(e) {\
                   btn.disabled = false;\
                   btn.textContent = '🔍 Run Check Now';\
                   status.textContent = 'Error: ' + e.message;\
               });\
         }\
         </script>"
            .to_string(),
    );
    lines.push("<style>.channels-panel { font-family: Georgia, serif; }</style>".to_string());
    lines.push("</div>".to_string());

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_paper(title: &str, abs: &str, cats: Vec<&str>) -> Paper {
        Paper {
            title: Some(title.to_string()),
            abstract_text: Some(abs.to_string()),
            categories: Some(cats.into_iter().map(String::from).collect()),
            published: Some("2024-01-01".to_string()),
            score: Some(0.5),
        }
    }

    #[test]
    fn test_get_channels_returns_default() {
        let channels = get_channels();
        assert!(!channels.is_empty());
        assert!(channels.iter().any(|c| c.id == "general"));
        assert!(channels.iter().any(|c| c.id == "climate"));
    }

    #[test]
    fn test_match_paper_general_channel() {
        let paper = make_paper(
            "Deep Learning for NLP",
            "We present a new transformer model for language understanding",
            vec!["cs.CL", "cs.AI"],
        );
        let matched = match_paper_to_channels(&paper);
        assert!(matched.contains(&"general".to_string()));
    }

    #[test]
    fn test_match_paper_climate_channel() {
        let paper = make_paper(
            "Climate Model",
            "Using AI to model climate change and carbon emissions",
            vec!["cs.AI"],
        );
        let matched = match_paper_to_channels(&paper);
        assert!(matched.contains(&"climate".to_string()));
    }

    #[test]
    fn test_match_paper_safety_channel() {
        let paper = make_paper(
            "AI Safety",
            "Alignment and robustness in large language models",
            vec!["cs.AI"],
        );
        let matched = match_paper_to_channels(&paper);
        assert!(matched.contains(&"ai_safety".to_string()));
    }

    #[test]
    fn test_match_paper_regulation_channel() {
        let paper = make_paper(
            "AI Policy",
            "Regulation and governance of AI systems and GDPR compliance",
            vec!["cs.CY"],
        );
        let matched = match_paper_to_channels(&paper);
        assert!(matched.contains(&"regulation".to_string()));
    }

    #[test]
    fn test_render_channels_html() {
        let html = render_channels_html(None);
        assert!(html.contains("arXiv Watch Alert Channels"));
        assert!(html.contains("general"));
        assert!(html.contains("Run Check Now"));
    }

    #[test]
    fn test_update_channel_not_found() {
        let mut updates = HashMap::new();
        updates.insert("name".to_string(), serde_json::json!("New Name"));
        assert!(!update_channel("nonexistent", updates));
    }
}
