//! rairos-labor-displacement-tracker — Filter papers for AI vs. human labor gaps.
//!
//! Ported from `llm/labor_displacement_tracker.py`.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

const PAPERS_DB_FILE: &str = ".ai_research_os/papers.json";

static LABOR_KEYWORDS: &[&str] = &[
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

static LABOR_CATS: &[&str] = &["cs.cyber-ph", "cs.soc", "cs.HC", "econ.GN"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paper {
    pub id: Option<String>,
    #[serde(rename = "arxiv_id")]
    pub arxiv_id: Option<String>,
    pub title: Option<String>,
    #[serde(default)]
    pub abstract_text: Option<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub published: Option<String>,
}

impl Paper {
    pub fn title(&self) -> &str {
        self.title.as_deref().unwrap_or("")
    }

    pub fn abstract_text(&self) -> &str {
        self.abstract_text.as_deref().unwrap_or("")
    }

    pub fn categories(&self) -> &[String] {
        &self.categories
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PapersData {
    #[serde(default)]
    papers: Vec<Paper>,
}

fn get_papers_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(PAPERS_DB_FILE)
}

fn load_papers() -> Vec<Paper> {
    let path = get_papers_path();
    if !path.exists() {
        return Vec::new();
    }
    match fs::read_to_string(&path) {
        Ok(text) => {
            if text.trim().is_empty() {
                return Vec::new();
            }
            match serde_json::from_str::<PapersData>(&text) {
                Ok(data) => data.papers,
                Err(_) => Vec::new(),
            }
        }
        Err(_) => Vec::new(),
    }
}

pub fn is_labor_related(paper: &Paper) -> bool {
    let text = format!(
        "{} {}",
        paper.title().to_lowercase(),
        paper.abstract_text().to_lowercase()
    );
    let cats: HashSet<_> = paper.categories.iter().collect();

    if LABOR_CATS.iter().any(|c| cats.contains(&c.to_string())) {
        return true;
    }

    LABOR_KEYWORDS
        .iter()
        .any(|kw| text.contains(&kw.to_lowercase()))
}

pub fn get_labor_papers() -> Vec<Paper> {
    load_papers().into_iter().filter(is_labor_related).collect()
}

pub fn render_labor_tracker_html() -> String {
    let papers = get_labor_papers();
    let mut lines = Vec::new();

    lines.push("<div class=\"labor-tracker\">".to_string());
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
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            let title = &p.title()[..p.title().len().min(70)];
            let published = p.published.as_deref().unwrap_or("").chars().take(4).collect::<String>();

            let text_lower = format!(
                "{} {}",
                p.title().to_lowercase(),
                p.abstract_text().to_lowercase()
            );
            let kw_matches: Vec<_> = LABOR_KEYWORDS
                .iter()
                .filter(|kw| {
                    kw.len() > 4 && text_lower.contains(&kw.to_lowercase())
                })
                .take(3)
                .collect();

            let kw_display = if !kw_matches.is_empty() {
                kw_matches
                    .iter()
                    .map(|k| format!("<code>{}</code>", k))
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                String::new()
            };

            lines.push(format!(
                "<div style='border:1px solid #e0dbd4;border-radius:6px;padding:12px;margin-bottom:10px'>\
                 <div style='font-size:13px;font-weight:600;color:#2a2a2a;margin-bottom:3px'>{}</div>\
                 <div style='font-size:11px;color:#A89E8C;margin-bottom:4px'>{} · {}</div>\
                 {}</div>",
                title,
                cats,
                published,
                if !kw_display.is_empty() {
                    format!("<div style='font-size:11px;color:#7a7570'>{}</div>", kw_display)
                } else {
                    String::new()
                }
            ));
        }
    }

    lines.push("<style>.labor-tracker { font-family: Georgia, serif; }</style>".to_string());
    lines.push("</div>".to_string());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_labor_related_with_category() {
        let paper = Paper {
            id: Some("1".to_string()),
            arxiv_id: Some("2301.12345".to_string()),
            title: Some("AI and Employment".to_string()),
            abstract_text: Some("A study on AI impact".to_string()),
            authors: vec![],
            categories: vec!["cs.cyber-ph".to_string()],
            published: Some("2023-01-01".to_string()),
        };
        assert!(is_labor_related(&paper));
    }

    #[test]
    fn test_is_labor_related_with_keyword() {
        let paper = Paper {
            id: Some("2".to_string()),
            arxiv_id: Some("2301.12346".to_string()),
            title: Some("Automation in Manufacturing".to_string()),
            abstract_text: Some("Impact of automation on jobs".to_string()),
            authors: vec![],
            categories: vec!["cs.AI".to_string()],
            published: Some("2023-01-01".to_string()),
        };
        assert!(is_labor_related(&paper));
    }

    #[test]
    fn test_is_labor_related_negative() {
        let paper = Paper {
            id: Some("3".to_string()),
            arxiv_id: Some("2301.12347".to_string()),
            title: Some("Computer Vision Advances".to_string()),
            abstract_text: Some("New techniques in CV".to_string()),
            authors: vec![],
            categories: vec!["cs.CV".to_string()],
            published: Some("2023-01-01".to_string()),
        };
        assert!(!is_labor_related(&paper));
    }

    #[test]
    fn test_get_labor_papers_empty_db() {
        let papers = get_labor_papers();
        let _ = papers.len();
    }

    #[test]
    fn test_render_html_structure() {
        let html = render_labor_tracker_html();
        assert!(html.contains("labor-tracker"));
        assert!(html.contains("Labor Displacement Tracker"));
        assert!(html.contains("<style>"));
    }
}
