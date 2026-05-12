//! rairos-impact-ranking — Impact Ranking HTML Renderer for AI Research OS.
//!
//! Ported from `llm/impact_ranking.py` (79 LOC, part of it is db-dependent).
//! Independent function: `render_impact_html`.

/// Render impact ranking data as an HTML table.
pub fn render_impact_html(data: &[serde_json::Value]) -> String {
    if data.is_empty() {
        return "<p>No impact data available.</p>".to_string();
    }

    let mut rows = Vec::new();
    for (i, item) in data.iter().take(20).enumerate() {
        let i = i + 1;
        let score = item.get("impact_score").and_then(|v| v.as_i64()).unwrap_or(0);
        let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("Unknown");
        let title_short = if title.len() > 70 { &title[..70] } else { title };
        let abs_url = item.get("abs_url").and_then(|v| v.as_str()).unwrap_or("#");
        let published = item.get("published").and_then(|v| v.as_str()).unwrap_or("").chars().take(4).collect::<String>();

        rows.push(format!(
            "<tr>\
             <td style=\"text-align:center\">{i}</td>\
             <td><a href=\"{abs_url}\">{title_short}</a></td>\
             <td style=\"text-align:center\">{published}</td>\
             <td style=\"text-align:right;font-weight:600\">{score}</td>\
             </tr>"
        ));
    }

    format!(
        "<table style=\"width:100%;border-collapse:collapse;font-size:14px\">\
         <thead><tr style=\"background:#f5f5f5\">\
         <th style=\"padding:8px 12px;text-align:center\">#</th>\
         <th style=\"padding:8px 12px;text-align:left\">Title</th>\
         <th style=\"padding:8px 12px;text-align:center\">Year</th>\
         <th style=\"padding:8px 12px;text-align:right\">Impact</th>\
         </tr></thead>\
         <tbody>{}</tbody>\
         </table>",
        rows.join("")
    )
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_impact_html_empty() {
        let result = render_impact_html(&[]);
        assert!(result.contains("No impact data"));
    }

    #[test]
    fn test_render_impact_html_single() {
        let data = vec![serde_json::json!({
            "title": "Test Paper",
            "impact_score": 42,
            "published": "2024",
            "abs_url": "https://example.com"
        })];
        let result = render_impact_html(&data);
        assert!(result.contains("Test Paper"));
        assert!(result.contains("42"));
    }

    #[test]
    fn test_render_impact_html_truncates_title() {
        let long_title = "A".repeat(100);
        let data = vec![serde_json::json!({
            "title": long_title,
            "impact_score": 10,
            "published": "2023",
            "abs_url": "#"
        })];
        let result = render_impact_html(&data);
        assert!(result.contains("AAAAAA")); // first 70 chars
        assert!(!result.contains(&"A".repeat(71))); // 71st char not present
    }
}
