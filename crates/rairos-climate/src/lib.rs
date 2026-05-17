//! Rairos Climate — Climate AI Monitor
//!
//! Tracks papers at the intersection of climate science and AI.

use rairos_core::constants::{CLIMATE_CATS, papers_db_path};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

const CLIMATE_KEYWORDS: &[&str] = &[
    "climate change",
    "global warming",
    "carbon",
    "emissions",
    "greenhouse gas",
    "renewable energy",
    "solar",
    "wind power",
    "energy efficiency",
    "sustainable",
    "sustainability",
    "fossil fuel",
    "net-zero",
    "carbon neutral",
    "climate model",
    "weather prediction",
    "earth system",
    "carbon capture",
    "data center",
    "water consumption",
    "e-waste",
    "environmental impact",
    "green AI",
    "energy-aware",
    "low-carbon",
    "carbon footprint",
    "flops per watt",
    "compute efficiency",
    "model efficiency",
];

fn climate_watch_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".ai_research_os")
        .join("climate_watch.json")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PapersFile {
    #[serde(rename = "papers")]
    papers: Vec<PaperEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperEntry {
    #[serde(rename = "id")]
    id: Option<String>,
    #[serde(rename = "title")]
    title: Option<String>,
    #[serde(rename = "abstract")]
    abstract_text: Option<String>,
    #[serde(rename = "published")]
    published: Option<String>,
    #[serde(rename = "categories")]
    categories: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ClimateWatch {
    #[serde(rename = "watched_ids")]
    watched_ids: Vec<String>,
    #[serde(rename = "last_scan")]
    last_scan: Option<String>,
}

fn load_papers() -> Vec<PaperEntry> {
    let path = papers_db_path();
    if !path.exists() {
        return Vec::new();
    }
    match std::fs::read_to_string(&path) {
        Ok(contents) => match serde_json::from_str::<PapersFile>(&contents) {
            Ok(file) => file.papers,
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    }
}

fn load_watch_list() -> ClimateWatch {
    let path = climate_watch_path();
    if !path.exists() {
        return ClimateWatch::default();
    }
    match std::fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => ClimateWatch::default(),
    }
}

fn save_watch_list(watch: &ClimateWatch) -> std::io::Result<()> {
    let path = climate_watch_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(watch)?;
    std::fs::write(path, data)
}

pub fn is_climate_related(title: &str, abstract_text: &str, categories: &[String]) -> bool {
    let text = format!("{} {}", title, abstract_text).to_lowercase();
    if categories
        .iter()
        .any(|c| CLIMATE_CATS.contains(&c.as_str()))
    {
        return true;
    }
    CLIMATE_KEYWORDS
        .iter()
        .any(|kw| text.contains(&kw.to_lowercase()))
}

pub fn get_climate_papers() -> Vec<PaperEntry> {
    let papers = load_papers();
    papers
        .into_iter()
        .filter(|p| {
            let title = p.title.as_deref().unwrap_or("");
            let abstract_text = p.abstract_text.as_deref().unwrap_or("");
            let empty_cats: Vec<String> = Vec::new();
            let cats = p.categories.as_deref().unwrap_or(&empty_cats);
            is_climate_related(title, abstract_text, cats)
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClimateStats {
    pub total_climate_papers: usize,
    pub watched_count: usize,
    pub recent_count: usize,
    pub last_scan: String,
}

pub fn get_watch_stats() -> ClimateStats {
    let climate_papers = get_climate_papers();
    let watch_list = load_watch_list();
    let watched_ids: HashSet<&str> = watch_list.watched_ids.iter().map(|s| s.as_str()).collect();

    let recent_count = climate_papers
        .iter()
        .filter(|p| {
            p.published
                .as_ref()
                .map(|s| s.as_str() >= "2025-01-01")
                .unwrap_or(false)
        })
        .count();

    ClimateStats {
        total_climate_papers: climate_papers.len(),
        watched_count: climate_papers
            .iter()
            .filter(|p| {
                p.id.as_ref()
                    .map(|id| watched_ids.contains(id.as_str()))
                    .unwrap_or(false)
            })
            .count(),
        recent_count,
        last_scan: watch_list.last_scan.unwrap_or_else(|| "never".to_string()),
    }
}

pub fn render_climate_monitor_html(stats: Option<ClimateStats>) -> String {
    let stats = stats.unwrap_or_else(get_watch_stats);
    let climate_papers = get_climate_papers();
    let watch_list = load_watch_list();
    let watched_ids: HashSet<&str> = watch_list.watched_ids.iter().map(|s| s.as_str()).collect();

    let mut lines = vec!["<div class=\"climate-monitor\">".to_string()];
    lines.push("<h3>🌍 Climate AI Monitor</h3>".to_string());
    lines.push(
        "<p style='font-size:13px;color:#A89E8C;margin-bottom:14px'>\
         Papers at the intersection of climate science and AI. \
         High priority in gap watch matching.</p>"
            .to_string(),
    );

    lines.push(
        "<div style='display:grid;grid-template-columns:1fr 1fr 1fr;gap:12px;margin-bottom:20px'>"
            .to_string(),
    );

    let stats_cells = [
        (
            "Total Climate Papers",
            stats.total_climate_papers,
            "#6B8FB5",
        ),
        ("In Your Watch List", stats.watched_count, "#6BBF8A"),
        ("Published 2025+", stats.recent_count, "#D4A055"),
    ];

    for (label, val, color) in stats_cells {
        lines.push(format!(
            "<div style='background:#f8f4ef;border-radius:6px;padding:12px;text-align:center'>\
             <div style='font-size:22px;font-weight:700;color:{}'>{}</div>\
             <div style='font-size:11px;color:#A89E8C;margin-top:2px'>{}</div></div>",
            color, val, label
        ));
    }
    lines.push("</div>".to_string());

    if climate_papers.is_empty() {
        lines.push(
            "<p style='color:#A89E8C;font-size:13px'>\
             No climate-related papers in your library yet.</p>"
                .to_string(),
        );
    } else {
        for p in climate_papers.iter().take(15) {
            let pid = p.id.as_deref().unwrap_or("");
            let is_watched = watched_ids.contains(pid);
            let cats = p
                .categories
                .as_ref()
                .map(|c| {
                    c.iter()
                        .take(2)
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            let title = p
                .title
                .as_deref()
                .unwrap_or("")
                .chars()
                .take(70)
                .collect::<String>();
            let published = p
                .published
                .as_deref()
                .unwrap_or("")
                .chars()
                .take(4)
                .collect::<String>();

            let text_lower = format!(
                "{} {}",
                p.title.as_deref().unwrap_or(""),
                p.abstract_text.as_deref().unwrap_or("")
            )
            .to_lowercase();

            let kw_matches: Vec<&str> = CLIMATE_KEYWORDS
                .iter()
                .filter(|kw| text_lower.contains(&(**kw).to_lowercase()))
                .take(3)
                .copied()
                .collect();

            let kw_display = kw_matches
                .iter()
                .map(|k| format!("<code>{}</code>", k))
                .collect::<Vec<_>>()
                .join(", ");

            let btn_color = if is_watched { "#6BBF8A" } else { "#A89E8C" };
            let btn_text = if is_watched { "✓ Watched" } else { "+ Watch" };

            lines.push(format!(
                "<div style='border: 1px solid #e0dbd4; border-radius: 6px; padding: 12px; margin-bottom: 10px'>\
                 <div style='display: flex; justify-content: space-between; align-items: flex-start'>\
                   <div style='flex:1'>\
                     <div style='font-size: 12px; color: #6B8FB5; font-weight: 600'>{}</div>\
                     <div style='font-size: 11px; color: #A89E8C; margin-top: 2px'>{} · {}</div>\
                     <div style='font-size: 11px; color: #7a7570; margin-top: 4px'>{}</div>\
                   </div>\
                   <button onclick=\"toggleWatch('{}', this)\"\
                     style='font-size: 10px; padding: 3px 8px; cursor: pointer; border-radius: 3px;\
                            border: 1px solid #ccc; background: transparent; color: {}'>{}</button>\
                 </div>\
                 </div>",
                title, cats, published, kw_display, pid, btn_color, btn_text
            ));
        }
    }

    lines.push(
        r#"<script>
function toggleWatch(paperId, btn) {
    fetch('/climate-monitor/toggle-watch', {
        method: 'POST',
        headers: {'Content-Type': 'application/json'},
        body: JSON.stringify({paper_id: paperId})
    }).then(function(r) { return r.json(); })
      .then(function(d) {
          if (d.success) {
              var isWatched = btn.textContent.trim() === 'Watched';
              btn.textContent = isWatched ? '+ Watch' : '✓ Watched';
              btn.style.color = isWatched ? '#A89E8C' : '#6BBF8A';
          }
      });
}
</script>"#
            .to_string(),
    );

    lines.push("<style>.climate-monitor { font-family: Georgia, serif; }</style>".to_string());
    lines.push("</div>".to_string());
    lines.join("\n")
}

pub fn toggle_watch(paper_id: &str) -> bool {
    let mut watch = load_watch_list();
    let pos = watch.watched_ids.iter().position(|id| id == paper_id);
    match pos {
        Some(idx) => {
            watch.watched_ids.remove(idx);
            let _ = save_watch_list(&watch);
            false
        }
        None => {
            watch.watched_ids.push(paper_id.to_string());
            let _ = save_watch_list(&watch);
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_climate_related_keyword() {
        let cats: Vec<String> = vec![];
        assert!(is_climate_related(
            "Carbon Footprint of AI Models",
            "We measure carbon emissions from transformers",
            &cats
        ));
    }

    #[test]
    fn test_is_climate_related_category() {
        let cats = vec!["cs.AI".to_string()];
        assert!(is_climate_related(
            "Some AI Paper",
            "Deep learning for vision",
            &cats
        ));
    }

    #[test]
    fn test_is_not_climate_related() {
        let cats: Vec<String> = vec![];
        assert!(!is_climate_related(
            "Attention Is All You Need",
            "We propose the transformer architecture",
            &cats
        ));
    }

    #[test]
    fn test_get_climate_papers_empty() {
        let result = get_climate_papers();
        assert!(result.is_empty());
    }

    #[test]
    fn test_climate_stats() {
        let stats = get_watch_stats();
        // `total_climate_papers` is usize — always non-negative by type
        assert!(stats.watched_count <= stats.total_climate_papers);
        assert!(stats.recent_count <= stats.total_climate_papers);
    }
}
