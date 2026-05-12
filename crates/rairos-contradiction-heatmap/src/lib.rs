//! rairos-contradiction-heatmap — Contradiction Heatmap HTML Renderer.
//!
//! Ported from `llm/contradiction_heatmap.py` (independent functions only).
//! Independent functions: `render_heatmap_html`, `_badge_color`.

/// Badge color for contradiction count (0-3+ scale).
pub fn badge_color(count: usize) -> &'static str {
    match count {
        0 => "#e8e4de",
        1 => "#f5d76e",
        2 => "#e67e22",
        _ => "#e74c3c",
    }
}

/// Render a grid of paper cards with contradiction heat colors.
pub fn render_heatmap_html(
    papers: &[serde_json::Value],
    contrad_map: &serde_json::Value,
) -> String {
    if papers.is_empty() {
        return "<p>No papers yet.</p>".to_string();
    }

    let mut lines = vec!["<div class=\"heatmap-grid\">".to_string()];

    for p in papers {
        let pid = p.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let info = contrad_map.get(pid);
        let count = info
            .and_then(|v| v.get("count"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        let bg = badge_color(count);
        let color = if count >= 2 { "#fff" } else { "#555" };
        let border_color = if count >= 3 { "#c0392b" } else { "#bdc3c7" };

        // Build tooltip
        let contrad_list = info
            .and_then(|v| v.get("contradictions"))
            .and_then(|v| v.as_array())
            .map(|a| a.iter().take(5).filter_map(|c| {
                let polarity = c.get("polarity").and_then(|v| v.as_str()).unwrap_or("");
                let gap_type = c.get("gap_type").and_then(|v| v.as_str()).unwrap_or("");
                let partner_id = c.get("partner_id").and_then(|v| v.as_str()).unwrap_or("");
                let keywords = c.get("shared_keywords")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter()
                        .filter_map(|k| k.as_str())
                        .collect::<Vec<_>>()
                        .join(","))
                    .unwrap_or_default();
                Some(format!(
                    "{} {} (→ {}) kw={}",
                    &polarity[..polarity.len().min(3)].to_uppercase(),
                    gap_type,
                    if partner_id.len() > 12 { &partner_id[..12] } else { partner_id },
                    keywords
                ))
            }).collect::<Vec<_>>())
            .unwrap_or_default();
        let tooltip_text = if contrad_list.is_empty() {
            "No contradictions".to_string()
        } else {
            contrad_list.join(" | ")
        };

        let title_short = p.get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .chars()
            .take(60)
            .collect::<String>();
        let category = p.get("primary_category")
            .and_then(|v| v.as_str())
            .unwrap_or("?");

        lines.push(format!(
            "<div class=\"heatmap-card\" style=\"background:{bg};color:{color};border-color:{border_color}\" title=\"{tooltip}\">\
             <div class=\"heatmap-card-cat\">{category}</div>\
             <div class=\"heatmap-card-title\">{title_short}</div>\
             <div class=\"heatmap-card-count\">{count} 🔥</div>\
             </div>",
            bg = bg,
            color = color,
            border_color = border_color,
            tooltip = tooltip_text,
            category = category,
            title_short = title_short,
            count = count
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

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_badge_color() {
        assert_eq!(badge_color(0), "#e8e4de");
        assert_eq!(badge_color(1), "#f5d76e");
        assert_eq!(badge_color(2), "#e67e22");
        assert_eq!(badge_color(99), "#e74c3c");
    }

    #[test]
    fn test_render_empty() {
        let result = render_heatmap_html(&[], &serde_json::json!({}));
        assert!(result.contains("No papers"));
    }

    #[test]
    fn test_render_single_paper() {
        let papers = vec![serde_json::json!({
            "id": "p1",
            "title": "Test Paper About AI Safety",
            "primary_category": "cs.AI",
            "published": "2024"
        })];
        let contrad_map = serde_json::json!({
            "p1": { "count": 3, "contradictions": [] }
        });
        let result = render_heatmap_html(&papers, &contrad_map);
        assert!(result.contains("cs.AI"));
        assert!(result.contains("Test Paper"));
        assert!(result.contains("3 🔥"));
    }

    #[test]
    fn test_render_no_contradictions() {
        let papers = vec![serde_json::json!({
            "id": "p1", "title": "Quiet Paper", "primary_category": "cs.CL"
        })];
        let result = render_heatmap_html(&papers, &serde_json::json!({}));
        assert!(result.contains("0 🔥"));
        assert!(result.contains("#e8e4de")); // neutral color
    }
}
