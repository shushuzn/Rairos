use serde::{Deserialize, Serialize};

#![allow(
    clippy::vec_init_then_push,
    dead_code,
)]
use std::collections::HashMap;
use std::path::PathBuf;

const CHANNELS_FILE_NAME: &str = "arxiv_channels.json";

fn channels_file() -> PathBuf {
    dirs::home_dir()
        .map(|p| p.join(".ai_research_os").join(CHANNELS_FILE_NAME))
        .unwrap_or_else(|| PathBuf::from(CHANNELS_FILE_NAME))
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
struct ChannelsData(HashMap<String, ChannelDef>);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChannelDef {
    name: String,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    keywords: Vec<String>,
    #[serde(default = "default_priority")]
    priority: i32,
    #[serde(default = "default_enabled")]
    enabled: bool,
}

fn default_priority() -> i32 {
    1
}

fn default_enabled() -> bool {
    true
}

fn default_channels() -> HashMap<String, ChannelDef> {
    let mut channels = HashMap::new();

    channels.insert(
        "general".to_string(),
        ChannelDef {
            name: "General AI/ML".to_string(),
            categories: vec![
                "cs.AI".to_string(),
                "cs.LG".to_string(),
                "cs.CL".to_string(),
                "cs.CV".to_string(),
                "cs.NE".to_string(),
            ],
            keywords: vec![],
            priority: 1,
            enabled: true,
        },
    );

    channels.insert(
        "climate".to_string(),
        ChannelDef {
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

    channels.insert(
        "ai_safety".to_string(),
        ChannelDef {
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

    channels.insert(
        "regulation".to_string(),
        ChannelDef {
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

    channels
}

fn load_channels() -> HashMap<String, ChannelDef> {
    let path = channels_file();
    if !path.exists() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let default = default_channels();
        let json = serde_json::to_string_pretty(&default).unwrap();
        let _ = std::fs::write(&path, json);
        return default;
    }
    match std::fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_else(|_| default_channels()),
        Err(_) => default_channels(),
    }
}

fn save_channels(channels: &HashMap<String, ChannelDef>) -> Result<(), std::io::Error> {
    let path = channels_file();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(channels).unwrap();
    std::fs::write(&path, json)
}

pub fn match_paper_to_channels(paper: &HashMap<String, serde_json::Value>) -> Vec<String> {
    let channels = load_channels();
    let cats: std::collections::HashSet<&str> = paper
        .get("categories")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    let abstract_text = format!(
        "{} {}",
        paper.get("abstract").and_then(|v| v.as_str()).unwrap_or(""),
        paper.get("title").and_then(|v| v.as_str()).unwrap_or("")
    )
    .to_lowercase();

    let _title = paper
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();

    let mut matched = Vec::new();

    for (cid, cfg) in &channels {
        if !cfg.enabled {
            continue;
        }
        if cfg.categories.iter().any(|c| cats.contains(c.as_str())) {
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
        .map(|(id, cfg)| ChannelConfig {
            id,
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
    if let Some(cfg) = channels.get_mut(cid) {
        if let Some(name) = updates.get("name").and_then(|v| v.as_str()) {
            cfg.name = name.to_string();
        }
        if let Some(cats) = updates.get("categories").and_then(|v| v.as_array()) {
            cfg.categories = cats
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }
        if let Some(kws) = updates.get("keywords").and_then(|v| v.as_array()) {
            cfg.keywords = kws
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }
        if let Some(priority) = updates.get("priority").and_then(|v| v.as_i64()) {
            cfg.priority = priority as i32;
        }
        if let Some(enabled) = updates.get("enabled").and_then(|v| v.as_bool()) {
            cfg.enabled = enabled;
        }
    }
    save_channels(&channels).is_ok()
}

pub fn render_channels_html(
    check_results: Option<&HashMap<String, Vec<HashMap<String, serde_json::Value>>>>,
) -> String {
    let channels = get_channels();
    let empty_map: &HashMap<String, Vec<HashMap<String, serde_json::Value>>> = &HashMap::new();
    let check_results = check_results.unwrap_or(empty_map);
    let mut lines = Vec::new();

    lines.push("<div class=\"channels-panel\">".to_string());
    lines.push("<h3>📡 arXiv Watch Alert Channels</h3>".to_string());
    lines.push(
        "<p style='font-size:13px;color:#A89E8C;margin-bottom:16px'>
        Configure multiple feed channels with different matching criteria.
        Higher priority = shown first in alerts.</p>"
            .to_string(),
    );

    lines.push(
        r#"    <div style="margin-bottom: 20px;">
      <button id="run-check-btn" onclick="runCheck()" style="
        background: #1a73e8; color: #fff; border: none; border-radius: 6px;
        padding: 10px 20px; font-size: 14px; cursor: pointer; font-family: Georgia, serif;">
        🔍 Run Check Now
      </button>
      <span id="check-status" style="font-size:13px;color:#888;margin-left:12px;display:none;"></span>
    </div>
    <div id="check-results"></div>
    "#
        .to_string(),
    );

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
        let kw_str: String = ch
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
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        let channel_results = check_results.get(&ch.id);
        let result_rows = if let Some(results) = channel_results {
            let mut rows = String::new();
            for rp in results.iter().take(5) {
                let title = rp.get("title").and_then(|v| v.as_str()).unwrap_or("");
                let published = rp.get("published").and_then(|v| v.as_str()).unwrap_or("");
                let score = rp.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                rows.push_str("<div style=\"display:flex;gap:8px;align-items:flex-start;padding:6px 0;border-bottom:1px solid #f0ebe5;\">");
                rows.push_str("<span style=\"color:#4CAF50;font-size:12px;\">●</span>");
                rows.push_str("<div style=\"flex:1;\">");
                rows.push_str(&format!(
                    "<div style=\"font-size:12px;color:#2a2a2a;font-weight:600;\">{}</div>",
                    &title[..title.len().min(80)]
                ));
                rows.push_str(&format!(
                    "<div style=\"font-size:11px;color:#888;\">{} · score={:.2}</div>",
                    published, score
                ));
                rows.push_str("</div></div>");
            }
            if rows.is_empty() {
                "<div style='font-size:12px;color:#bbb;padding:4px 0;'>No new papers in last check</div>"
                    .to_string()
            } else {
                rows
            }
        } else {
            "<div style='font-size:12px;color:#bbb;padding:4px 0;'>No new papers in last check</div>".to_string()
        };

        lines.push(format!(
            r#"<div style='border: 1px solid #e0dbd4; border-radius: 6px; padding: 14px; margin-bottom: 12px; border-left: 4px solid {};'>
  <div style='display: flex; justify-content: space-between; align-items: center; margin-bottom: 6px;'>
    <div style='font-weight: 700; font-size: 14px; color: #2a2a2a'>{}</div>
    <div style='font-size: 11px; color: #A89E8C'>priority {} · {}</div>
  </div>
  <div style='font-size: 12px; color: #7a7570; margin-bottom: 4px'>Categories: {}</div>
  <div style='font-size: 12px; color: #A89E8C; margin-bottom: 8px'>Keywords: {}</div>
  <div style='margin-bottom: 10px; padding: 8px; background: #faf9f7; border-radius: 4px;'>
    <div style='font-size:11px;color:#888;margin-bottom:6px;'>Recent papers from this channel:</div>
    {}
  </div>
  <div style='display: flex; gap: 8px;'>
    <button onclick="toggleChannel('{}')" style="font-size: 11px; padding: 3px 10px; cursor: pointer; border-radius: 3px; border: 1px solid #ccc; background: transparent">
      Toggle
    </button>
  </div>
</div>"#,
            color,
            ch.name,
            ch.priority,
            status,
            cat_str,
            if kw_str.is_empty() { "(none)".to_string() } else { kw_str },
            result_rows,
            ch.id
        ));
    }

    lines.push(
        r#"<script>
function toggleChannel(cid) {
    fetch('/arxiv-channels/toggle/' + cid, {method: 'POST'})
      .then(function(r) { return r.json(); })
      .then(function(d) { if (d.success) location.reload(); });
}
function runCheck() {
    var btn = document.getElementById('run-check-btn');
    var status = document.getElementById('check-status');
    btn.disabled = true;
    btn.textContent = '⏳ Checking...';
    status.style.display = 'inline';
    status.textContent = 'Querying arXiv...';
    fetch('/arxiv-channels/check', {method: 'POST'})
      .then(function(r) { return r.json(); })
      .then(function(d) {
          btn.disabled = false;
          btn.textContent = '🔍 Run Check Now';
          status.textContent = '';
          location.reload();
      })
      .catch(function(e) {
          btn.disabled = false;
          btn.textContent = '🔍 Run Check Now';
          status.textContent = 'Error: ' + e.message;
      });
}
</script>"#
            .to_string(),
    );

    lines.push("<style>.channels-panel { font-family: Georgia, serif; }</style>".to_string());
    lines.push("</div>".to_string());

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_channels() {
        let channels = get_channels();
        assert!(!channels.is_empty());
        let ids: Vec<&str> = channels.iter().map(|c| c.id.as_str()).collect();
        assert!(ids.contains(&"general"));
        assert!(ids.contains(&"climate"));
        assert!(ids.contains(&"ai_safety"));
        assert!(ids.contains(&"regulation"));
    }

    #[test]
    fn test_match_paper_to_channels() {
        let mut paper = HashMap::new();
        paper.insert(
            "categories".to_string(),
            serde_json::json!(["cs.AI", "cs.LG"]),
        );
        paper.insert(
            "title".to_string(),
            serde_json::json!("Safety in AI Systems"),
        );
        paper.insert(
            "abstract".to_string(),
            serde_json::json!("This paper discusses AI safety and alignment."),
        );

        let matched = match_paper_to_channels(&paper);
        assert!(matched.contains(&"general".to_string()));
        assert!(matched.contains(&"ai_safety".to_string()));
    }

    #[test]
    fn test_match_paper_by_keywords() {
        let mut paper = HashMap::new();
        paper.insert("categories".to_string(), serde_json::json!(["cs.AI"]));
        paper.insert(
            "title".to_string(),
            serde_json::json!("Climate Impact of AI"),
        );
        paper.insert(
            "abstract".to_string(),
            serde_json::json!("This paper discusses carbon emissions from AI."),
        );

        let matched = match_paper_to_channels(&paper);
        assert!(matched.contains(&"climate".to_string()));
    }

    #[test]
    fn test_channel_config_serialization() {
        let ch = ChannelConfig {
            id: "test".to_string(),
            name: "Test Channel".to_string(),
            categories: vec!["cs.AI".to_string()],
            keywords: vec!["test".to_string()],
            priority: 1,
            enabled: true,
        };
        let json = serde_json::to_string(&ch).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("Test Channel"));
    }

    #[test]
    fn test_update_channel() {
        let mut updates = HashMap::new();
        updates.insert("priority".to_string(), serde_json::json!(5));
        let result = update_channel("general", updates);
        assert!(result);
    }
}
