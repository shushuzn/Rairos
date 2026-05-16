//! rairos-labor-displacement-tracker — Labor Displacement Tracker for AI Research OS.
//!
//! Ported from `llm/labor_displacement_tracker.py`.
//!
//! Tracks papers about AI's impact on employment across cs.cyber-ph, cs.soc, and related categories.

use rairos_core::constants::PAPERS_DB_PATH;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const LABOR_KEYWORDS: &[&str] = &[
    "labor displacement",
    "job displacement",
    "automation",
    "unemployment",
    "employment",
    "workforce",
    "labor market",
    "skill gap",
    "income inequality",
    "AI and jobs",
    "AI impact on employment",
    "future of work",
    "automation risk",
    "robots",
    "replacement",
    "outsourcing",
    "gig economy",
    "platform work",
    "social protection",
    "universal basic income",
    "reskilling",
    "cs.cyber-ph",
    "cs.soc",
    "econ.GN",
    "econ.GR",
];

const LABOR_CATS: &[&str] = &["cs.cyber-ph", "cs.soc", "cs.HC", "econ.GN"];

/// Returns the path to the papers database.
fn papers_db_path() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "~".to_string()))
        .join(PAPERS_DB_PATH)
}

/// A paper entry as stored in papers.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paper {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub arxiv_id: Option<String>,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub abstract_: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub published: String,
}

/// Internal storage wrapper.
#[derive(Debug, Deserialize)]
#[derive(Default)]
struct PapersDb {
    #[serde(default)]
    papers: Vec<Paper>,
}


fn load_papers() -> Vec<Paper> {
    let path = papers_db_path();
    if !path.exists() {
        return vec![];
    }
    match fs::read_to_string(&path) {
        Ok(data) => match serde_json::from_str::<PapersDb>(&data) {
            Ok(db) => db.papers,
            Err(_) => vec![],
        },
        Err(_) => vec![],
    }
}

/// Check if a paper is labor-related based on categories or keyword matching.
pub fn is_labor_related(paper: &Paper) -> bool {
    // Check categories
    if paper.categories.iter().any(|c| LABOR_CATS.contains(&c.as_str())) {
        return true;
    }
    // Check keywords
    let text = format!(
        "{} {}",
        paper.title,
        paper.abstract_.as_deref().unwrap_or("")
    )
    .to_lowercase();
    LABOR_KEYWORDS
        .iter()
        .any(|kw| text.contains(&kw.to_lowercase()))
}

/// Return all labor-related papers from the database.
pub fn get_labor_papers() -> Vec<Paper> {
    load_papers().into_iter().filter(is_labor_related).collect()
}

/// Render the labor tracker as an HTML fragment.
pub fn render_labor_tracker_html() -> String {
    let papers = get_labor_papers();
    let mut lines = vec!["<div class=\"labor-tracker\">".to_string()];
    lines.push("<h3>👷 Labor Displacement Tracker</h3>".to_string());
    lines.push(
        "<p style='font-size:13px;color:#A89E8C;margin-bottom:16px'>\
         Papers about AI's impact on employment, workforce, and labor markets. \
         ArXiv: cs.cyber-ph, cs.soc, cs.HC</p>"
            .to_string(),
    );
    lines.push(format!(
        "<p style='font-size:13px;color:#6B8FB5;margin-bottom:14px'>\
         <b>{}</b> labor-related papers in your library.</p>",
        papers.len()
    ));

    if papers.is_empty() {
        lines.push(
            "<p style='color:#A89E8C;font-size:13px'>No labor-related papers yet. \
             Papers from cs.cyber-ph and cs.soc categories will appear here.</p>"
                .to_string(),
        );
    } else {
        for p in papers.iter().take(20) {
            let cats = p
                .categories
                .iter()
                .take(2)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            let title = if p.title.len() > 70 {
                format!("{}...", &p.title[..70])
            } else {
                p.title.clone()
            };
            let published = if p.published.len() >= 4 {
                p.published[..4].to_string()
            } else {
                p.published.clone()
            };
            let text = format!(
                "{} {}",
                p.title,
                p.abstract_.as_deref().unwrap_or("")
            )
            .to_lowercase();
            let kw_matches: Vec<_> = LABOR_KEYWORDS
                .iter()
                .filter(|kw| kw.len() > 4 && text.contains(&kw.to_lowercase()))
                .take(3)
                .copied()
                .collect();
            let kw_display = if kw_matches.is_empty() {
                String::new()
            } else {
                format!(
                    "<div style='font-size:11px;color:#7a7570'>{}</div>",
                    kw_matches
                        .iter()
                        .map(|k| format!("<code>{}</code>", k))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };

            lines.push(format!(
                "<div style='border:1px solid #e0dbd4;border-radius:6px;padding:12px;margin-bottom:10px'>\
                 <div style='font-size:13px;font-weight:600;color:#2a2a2a;margin-bottom:3px'>{}</div>\
                 <div style='font-size:11px;color:#A89E8C;margin-bottom:4px'>{} · {}</div>\
                 {}\
                 </div>",
                title, cats, published, kw_display
            ));
        }
    }

    lines.push("<style>.labor-tracker {{ font-family: Georgia, serif; }}</style>".to_string());
    lines.push("</div>".to_string());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_paper(title: &str, abstract_: &str, categories: Vec<&str>) -> Paper {
        Paper {
            id: None,
            arxiv_id: None,
            title: title.to_string(),
            abstract_: Some(abstract_.to_string()),
            categories: categories.into_iter().map(String::from).collect(),
            authors: vec![],
            published: "2024-01-15".to_string(),
        }
    }

    #[test]
    fn test_is_labor_related_by_category() {
        let p = make_paper(
            "A Paper About Something",
            "Something unrelated",
            vec!["cs.cyber-ph"],
        );
        assert!(is_labor_related(&p));
    }

    #[test]
    fn test_is_labor_related_by_keyword() {
        let p = make_paper(
            "The Future of Work and Automation",
            "This paper discusses how AI is changing employment.",
            vec!["cs.AI"],
        );
        assert!(is_labor_related(&p));
    }

    #[test]
    fn test_not_labor_related() {
        let p = make_paper(
            "Deep Learning for Image Classification",
            "We present a new CNN architecture.",
            vec!["cs.CV"],
        );
        assert!(!is_labor_related(&p));
    }

    #[test]
    fn test_get_labor_papers_filters() {
        let papers = [make_paper("Automation and Jobs", "Discussion on job displacement.", vec!["cs.AI"]),
            make_paper(
                "ImageNet Classifier",
                "A newResNet architecture.",
                vec!["cs.CV"],
            )];
        // We can't easily test get_labor_papers without mocking the file,
        // but we test the filter logic via is_labor_related
        assert!(is_labor_related(&papers[0]));
        assert!(!is_labor_related(&papers[1]));
    }

    #[test]
    fn test_render_html_empty() {
        // When no papers DB exists, render_html should show empty state
        let html = render_labor_tracker_html();
        assert!(html.contains("labor-tracker"));
        assert!(html.contains("Labor Displacement Tracker"));
    }
}
