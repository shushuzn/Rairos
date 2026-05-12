//! rairos-paradigm-monitor — Paradigm Concentration Monitor for AI Research OS.
//!
//! Ported from `llm/paradigm_monitor.py` (independent `render_html` only).

/// Render paradigm concentration results as HTML.
pub fn render_html(data: &serde_json::Value) -> String {
    if data.is_null() || data.get("error").is_some() {
        return "<p>Paradigm concentration monitor temporarily unavailable</p>".to_string();
    }

    let alerts = data.get("alerts").and_then(|v| v.as_array()).map(|a| a.clone()).unwrap_or_default();
    let categories = data.get("categories").and_then(|v| v.as_array()).map(|a| a.clone()).unwrap_or_default();

    if categories.is_empty() && alerts.is_empty() {
        return "<p>No paradigm concentration detected.</p>".to_string();
    }

    let mut parts: Vec<String> = Vec::new();

    // Render alerts
    for alert in &alerts {
        let message = alert.get("message").and_then(|v| v.as_str()).unwrap_or("");
        parts.push(format!(
            "<div style=\"background:#fff3cd;border:1px solid #ffc107;border-radius:6px;padding:12px;margin-bottom:12px;font-size:13px\">\
             <strong>⚠️ Paradigm Concentration Alert</strong><br>\
             {}</div>",
            message
        ));
    }

    // Render table
    if !categories.is_empty() {
        let mut rows = Vec::new();
        for (i, cat) in categories.iter().enumerate() {
            let i = i + 1;
            let title = cat.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let share_pct = cat.get("share_pct").and_then(|v| v.as_f64()).unwrap_or(0.0);
            rows.push(format!(
                "<tr><td style=\"padding:6px 12px\">{}</td>\
                 <td style=\"padding:6px 12px\">{}</td>\
                 <td style=\"padding:6px 12px;text-align:center\">{:.1}%</td></tr>",
                i, title, share_pct
            ));
        }
        parts.push(format!(
            "<table style=\"width:100%;border-collapse:collapse;font-size:13px\">\
             <tr style=\"background:#f5f5f5\">\
             <th style=\"padding:8px 12px\">#</th>\
             <th style=\"padding:8px 12px;text-align:left\">Paper</th>\
             <th style=\"padding:8px 12px;text-align:center\">Citation Share</th>\
             </tr>{}</table>",
            rows.join("")
        ));
    }

    parts.join("")
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_unavailable() {
        let data = serde_json::json!({ "error": "db unavailable" });
        let result = render_html(&data);
        assert!(result.contains("unavailable"));
    }

    #[test]
    fn test_render_empty() {
        let data = serde_json::json!({ "categories": [], "alerts": [] });
        let result = render_html(&data);
        assert!(result.contains("No paradigm concentration"));
    }

    #[test]
    fn test_render_alert() {
        let data = serde_json::json!({
            "alerts": [{
                "type": "paradigm_concentration",
                "severity": "high",
                "message": "75% of citations in 'cs.AI' cluster around 3 papers."
            }],
            "categories": []
        });
        let result = render_html(&data);
        assert!(result.contains("Paradigm Concentration Alert"));
        assert!(result.contains("75%"));
    }

    #[test]
    fn test_render_table() {
        let data = serde_json::json!({
            "categories": [
                { "title": "Attention Is All You Need", "share_pct": 45.0 },
                { "title": "BERT", "share_pct": 20.0 }
            ],
            "alerts": []
        });
        let result = render_html(&data);
        assert!(result.contains("Attention Is All You Need"));
        assert!(result.contains("45.0%"));
        assert!(result.contains("BERT"));
    }
}
