//! rairos-impact-ranking — Compute and render paper impact scores from the database.
//!
//! Impact score = citation_count (from CrossRef/OpenAlex) + reference_count.
//!
//! Reference: Python llm/impact_ranking.py

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

/// Impact entry for a single paper.
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

/// Compute impact ranking for all papers in the database.
///
/// Impact score = citation_count (from CrossRef/OpenAlex) + reference_count.
///
/// Returns a vector of ImpactEntry sorted by impact_score descending.
pub fn compute_impact(db: &Connection) -> Vec<ImpactEntry> {
    let Ok(mut stmt) = db.prepare(
        r#"
        SELECT id, title, citation_count, reference_count, published, abs_url
        FROM papers
        WHERE title IS NOT NULL AND title != ''
        ORDER BY (COALESCE(citation_count, 0) + COALESCE(reference_count, 0)) DESC
        LIMIT 100
        "#,
    ) else {
        return vec![];
    };

    let rows = stmt.query_map([], |row| {
        let citations: Option<i32> = row.get(2)?;
        let references: Option<i32> = row.get(3)?;
        let citations = citations.unwrap_or(0);
        let references = references.unwrap_or(0);
        Ok(ImpactEntry {
            paper_id: row.get(0)?,
            title: row.get(1)?,
            citation_count: citations,
            reference_count: references,
            impact_score: citations + references,
            published: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
            abs_url: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
        })
    });

    match rows {
        Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
        Err(_) => vec![],
    }
}

/// Render impact ranking as an HTML table.
pub fn render_impact_html(data: &[ImpactEntry]) -> String {
    if data.is_empty() {
        return "<p>No impact data available.</p>".to_string();
    }

    let mut rows = Vec::new();
    for (i, item) in data.iter().take(20).enumerate() {
        let title = if item.title.len() > 70 {
            &item.title[..70]
        } else {
            &item.title
        };
        let pub_year = item.published.chars().take(4).collect::<String>();

        rows.push(format!(
            "<tr>\
             <td style=\"text-align:center\">{}</td>\
             <td><a href=\"{}\">{}</a></td>\
             <td style=\"text-align:center\">{}</td>\
             <td style=\"text-align:right;font-weight:600\">{}</td>\
             </tr>",
            i + 1,
            html_escape(&item.abs_url),
            html_escape(title),
            html_escape(&pub_year),
            item.impact_score
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

// ============================================================================
// Tests
// ============================================================================

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
        let data = vec![ImpactEntry {
            paper_id: "1".to_string(),
            title: "Test Paper".to_string(),
            citation_count: 100,
            reference_count: 50,
            impact_score: 150,
            published: "2020-01-01".to_string(),
            abs_url: "https://example.com".to_string(),
        }];
        let result = render_impact_html(&data);
        assert!(result.contains("<table"));
        assert!(result.contains("Test Paper"));
        assert!(result.contains("150"));
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape("a < b"), "a &lt; b");
        assert_eq!(html_escape("a > b"), "a &gt; b");
        assert_eq!(html_escape("a \"b\""), "a &quot;b&quot;");
    }

    #[test]
    fn test_impact_entry_fields() {
        let entry = ImpactEntry {
            paper_id: "p1".to_string(),
            title: "A Test Paper".to_string(),
            citation_count: 10,
            reference_count: 5,
            impact_score: 15,
            published: "2023-06-15".to_string(),
            abs_url: "https://arxiv.org/abs/2301.00001".to_string(),
        };
        assert_eq!(entry.impact_score, 15);
        assert_eq!(entry.citation_count, 10);
        assert_eq!(entry.reference_count, 5);
    }

    #[test]
    fn test_render_truncates_to_20() {
        let data: Vec<ImpactEntry> = (0..25)
            .map(|i| ImpactEntry {
                paper_id: i.to_string(),
                title: format!("Paper {}", i),
                citation_count: i,
                reference_count: 0,
                impact_score: i,
                published: "2020".to_string(),
                abs_url: "#".to_string(),
            })
            .collect();
        let result = render_impact_html(&data);
        // Should not contain row 21 (index 20)
        assert!(!result.contains(">21</td>"));
    }

    #[test]
    fn test_render_truncates_title() {
        let long_title = "A".repeat(100);
        let data = vec![ImpactEntry {
            paper_id: "1".to_string(),
            title: long_title,
            citation_count: 0,
            reference_count: 0,
            impact_score: 0,
            published: "2020".to_string(),
            abs_url: "#".to_string(),
        }];
        let result = render_impact_html(&data);
        // Title should be truncated to 70 chars
        assert!(!result.contains(&"A".repeat(71)));
    }
}
