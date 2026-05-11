use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactEntry {
    pub paper_id: String,
    pub title: String,
    pub citation_count: i32,
    pub reference_count: i32,
    pub impact_score: i32,
    pub published: String,
    pub abs_url: String,
}

pub fn compute_impact(db: Option<()>) -> Vec<HashMap<String, serde_json::Value>> {
    if db.is_none() {
        return vec![];
    }
    vec![]
}

pub fn render_impact_html(data: &[HashMap<String, serde_json::Value>]) -> String {
    if data.is_empty() {
        return "<p>No impact data available.</p>".to_string();
    }

    let mut rows = Vec::new();
    for (i, item) in data.iter().take(20).enumerate() {
        let score = item.get("impact_score").and_then(|v| v.as_i64()).unwrap_or(0);
        let title = item.get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");
        let title = &title[..title.len().min(70)];
        let pub_year = item.get("published")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .chars()
            .take(4)
            .collect::<String>();
        let url = item.get("abs_url").and_then(|v| v.as_str()).unwrap_or("#");

        rows.push(format!(
            "<tr>\
             <td style=\"text-align:center\">{}</td>\
             <td><a href=\"{}\">{}</a></td>\
             <td style=\"text-align:center\">{}</td>\
             <td style=\"text-align:right;font-weight:600\">{}</td>\
             </tr>",
            i + 1,
            url,
            html_escape(title),
            pub_year,
            score
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

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_impact_html_empty() {
        let result = render_impact_html(&[]);
        assert_eq!(result, "<p>No impact data available.</p>");
    }

    #[test]
    fn test_render_impact_html_with_data() {
        let mut item = HashMap::new();
        item.insert("paper_id".to_string(), serde_json::json!("1"));
        item.insert("title".to_string(), serde_json::json!("Test Paper"));
        item.insert("citation_count".to_string(), serde_json::json!(100));
        item.insert("reference_count".to_string(), serde_json::json!(50));
        item.insert("impact_score".to_string(), serde_json::json!(150));
        item.insert("published".to_string(), serde_json::json!("2020-01-01"));
        item.insert("abs_url".to_string(), serde_json::json!("https://example.com"));

        let result = render_impact_html(&[item]);
        assert!(result.contains("<table"));
        assert!(result.contains("Test Paper"));
        assert!(result.contains("150"));
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape("a < b"), "a &lt; b");
        assert_eq!(html_escape("a > b"), "a &gt; b");
    }
}
