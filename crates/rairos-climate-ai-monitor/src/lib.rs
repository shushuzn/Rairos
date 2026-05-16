//! rairos-climate-ai-monitor — Climate AI Monitor for AI Research OS.
//!
//! Ported from `llm/climate_ai_monitor.py` (independent functions only).
//! `render_climate_monitor_html` renders climate AI paper monitoring stats.

const CLIMATE_KEYWORDS: &[&str] = &[
    "climate change", "global warming", "carbon", "emissions", "greenhouse gas",
    "renewable energy", "solar", "wind power", "energy efficiency", "sustainable",
    "sustainability", "fossil fuel", "net-zero", "carbon neutral", "climate model",
    "weather prediction", "earth system", "carbon capture", "data center",
    "water consumption", "e-waste", "environmental impact", "green AI",
    "energy-aware", "low-carbon", "carbon footprint", "FLOPs per watt",
    "compute efficiency", "model efficiency",
];

const CLIMATE_CATS: &[&str] = &["cs.AI", "cs.LG", "cs.ET", "physics.ao-ph", "atm.ph"];

/// Returns true if the paper is climate-related.
pub fn is_climate_related(paper: &serde_json::Value) -> bool {
    let text = format!(
        "{} {}",
        paper.get("title").and_then(|v| v.as_str()).unwrap_or(""),
        paper.get("abstract").and_then(|v| v.as_str()).unwrap_or("")
    ).to_lowercase();

    if let Some(cats) = paper.get("categories").and_then(|v| v.as_array()) {
        let cat_set: std::collections::HashSet<_> = cats.iter()
            .filter_map(|v| v.as_str())
            .collect();
        if CLIMATE_CATS.iter().any(|c| cat_set.contains(*c)) {
            return true;
        }
    }

    CLIMATE_KEYWORDS.iter().any(|kw| text.contains(&kw.to_lowercase()))
}

/// Render climate AI monitoring HTML.
pub fn render_climate_monitor_html(
    papers: &[serde_json::Value],
    stats: Option<&serde_json::Value>,
    watched_ids: &[String],
) -> String {
    let stats: &serde_json::Value = match stats {
        Some(s) => s,
        None => &serde_json::json!({}),
    };
    let total_climate = stats.get("total_climate_papers").and_then(|v| v.as_u64()).unwrap_or(0);
    let watched_count = stats.get("watched_count").and_then(|v| v.as_u64()).unwrap_or(0);
    let recent_count = stats.get("recent_count").and_then(|v| v.as_u64()).unwrap_or(0);

    let mut lines = vec!["<div class=\"climate-monitor\">".to_string()];
    lines.push("<h3>🌍 Climate AI Monitor</h3>".to_string());
    lines.push(
        "<p style='font-size:13px;color:#A89E8C;margin-bottom:14px'>\
         Papers at the intersection of climate science and AI. \
         High priority in gap watch matching.</p>".to_string()
    );

    // Stats grid
    lines.push(
        "<div style='display:grid;grid-template-columns:1fr 1fr 1fr;gap:12px;margin-bottom:20px'>".to_string()
    );
    for (label, val, color) in [
        ("Total Climate Papers", total_climate as i64, "#6B8FB5"),
        ("In Your Watch List", watched_count as i64, "#6BBF8A"),
        ("Published 2025+", recent_count as i64, "#D4A055"),
    ] {
        lines.push(format!(
            "<div style='background:#f8f4ef;border-radius:6px;padding:12px;text-align:center'>\
             <div style='font-size:22px;font-weight:700;color:{}'>{}</div>\
             <div style='font-size:11px;color:#A89E8C;margin-top:2px'>{}</div></div>",
            color, val, label
        ));
    }
    lines.push("</div>".to_string());

    // Paper list
    if papers.is_empty() {
        lines.push(
            "<p style='color:#A89E8C;font-size:13px'>\
             No climate-related papers in your library yet.</p>".to_string()
        );
    } else {
        for p in papers.iter().take(15) {
            let pid = p.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let is_watched = watched_ids.iter().any(|id| id == pid);
            let cats = p.get("categories")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().take(2).filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", "))
                .unwrap_or_default();
            let title = p.get("title").and_then(|v| v.as_str()).unwrap_or("")[..70.min(p.get("title").and_then(|v| v.as_str()).unwrap_or("").len())].to_string();
            let published = p.get("published").and_then(|v| v.as_str()).unwrap_or("").chars().take(4).collect::<String>();
            let text = format!(
                "{} {}",
                p.get("title").and_then(|v| v.as_str()).unwrap_or(""),
                p.get("abstract").and_then(|v| v.as_str()).unwrap_or("")
            ).to_lowercase();
            let kw_matches: Vec<_> = CLIMATE_KEYWORDS.iter()
                .filter(|kw| text.contains(&kw.to_lowercase()))
                .take(3)
                .collect();
            let kw_display = kw_matches.iter()
                .map(|k| format!("<code>{}</code>", k))
                .collect::<Vec<_>>()
                .join(", ");

            let btn_color = if is_watched { "#6BBF8A" } else { "#A89E8C" };
            let btn_text = if is_watched { "✓ Watched" } else { "+ Watch" };

            lines.push(format!(r#"
<div style='border: 1px solid #e0dbd4; border-radius: 6px; padding: 12px; margin-bottom: 10px'>
  <div style='display: flex; justify-content: space-between; align-items: flex-start'>
    <div style='flex:1'>
      <div style='font-size: 12px; color: #6B8FB5; font-weight: 600'>{}</div>
      <div style='font-size: 11px; color: #A89E8C; margin-top: 2px'>{} · {}</div>
      <div style='font-size: 11px; color: #7a7570; margin-top: 4px'>{}</div>
    </div>
    <button onclick="toggleWatch('{}', this)"
      style='font-size: 10px; padding: 3px 8px; cursor: pointer; border-radius: 3px;
             border: 1px solid #ccc; background: transparent; color: {}'>
      {}
    </button>
  </div>
</div>"#, title, cats, published, kw_display, pid, btn_color, btn_text));
        }
    }

    lines.push(r#"
<script>
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
</script>"#.to_string());

    lines.push("<style>.climate-monitor { font-family: Georgia, serif; }</style>".to_string());
    lines.push("</div>".to_string());
    lines.join("\n")
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_climate_related_keyword() {
        let paper = serde_json::json!({
            "title": "Carbon Footprint of Large Language Models",
            "abstract": "We measure energy consumption..."
        });
        assert!(is_climate_related(&paper));
    }

    #[test]
    fn test_is_climate_related_category() {
        let paper = serde_json::json!({
            "title": "Deep Learning for Weather",
            "abstract": "A new model for prediction...",
            "categories": ["cs.LG", "physics.ao-ph"]
        });
        assert!(is_climate_related(&paper));
    }

    #[test]
    fn test_is_climate_related_negative() {
        let paper = serde_json::json!({
            "title": "Attention Is All You Need",
            "abstract": "We propose transformers...",
            "categories": ["cs.CL"]
        });
        assert!(!is_climate_related(&paper));
    }

    #[test]
    fn test_render_empty() {
        let result = render_climate_monitor_html(&[], None, &[]);
        assert!(result.contains("Climate AI Monitor"));
        assert!(result.contains("No climate-related papers"));
    }
}
