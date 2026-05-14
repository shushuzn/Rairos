//! Literature Review Generator — builds structured lit reviews from paper collections.
//!
//! Mirrors llm/lit_review_generator.py

use crate::LlmClient;
use serde::{Deserialize, Serialize};

const _LIT_REVIEW_SYSTEM: &str = r#"You are a literature review expert. Synthesize multiple papers into a coherent review section.

Focus on:
1. The main research question or theme
2. How each paper contributes
3. Agreements and disagreements between papers
4. Gaps that remain
5. How this body of work connects to the user's research

Use a clear narrative flow: general → specific, established → emerging."#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LitReviewSection {
    pub heading: String,
    pub content: String,
    pub cited_papers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LitReviewResult {
    pub title: String,
    pub sections: Vec<LitReviewSection>,
    pub summary: String,
    pub references: Vec<String>,
}

pub async fn generate_lit_review(
    llm: &dyn LlmClient,
    model: &str,
    topic: &str,
    papers: &[LitReviewPaper],
) -> LitReviewResult {
    let paper_list: String = papers.iter()
        .enumerate()
        .map(|(i, p)| format!("[{i}] {t}. {a}", t = p.title, a = p.authors.join(", ")))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!(
        "Topic: {}\n\nPapers:\n{}\n\nGenerate a structured literature review with 3-4 sections covering the theme, contributions, debates, and gaps.",
        topic, paper_list
    );

    let msg = crate::Message { role: "user".to_string(), content: prompt };

    let ns = match llm.complete(vec![msg], model, 0.2, 3000).await {
        Ok(crate::LlmResponse::NonStream(ns)) => ns,
        _ => return LitReviewResult {
            title: "Literature Review".to_string(),
            sections: vec![],
            summary: "Failed to generate.".to_string(),
            references: vec![],
        },
    };

    let (sections, summary, references) = parse_review(&ns.content, papers);
    LitReviewResult { title: topic.to_string(), sections, summary, references }
}

fn parse_review(text: &str, papers: &[LitReviewPaper]) -> (Vec<LitReviewSection>, String, Vec<String>) {
    let lines: Vec<&str> = text.lines().collect();
    let mut sections = Vec::new();
    let mut current_heading = String::new();
    let mut current_content: Vec<String> = Vec::new();
    let mut summary = String::new();
    let mut in_summary = false;

    for &line in &lines {
        let trimmed = line.trim().trim_matches('#').trim();
        if trimmed.starts_with("Summary") || trimmed.starts_with("Conclusion") {
            in_summary = true;
            if !current_heading.is_empty() && !current_content.is_empty() {
                sections.push(LitReviewSection {
                    heading: std::mem::take(&mut current_heading),
                    content: current_content.join("\n"),
                    cited_papers: vec![],
                });
                current_content.clear();
            }
        } else if line.starts_with("## ") || line.starts_with("### ") {
            if !current_heading.is_empty() && !current_content.is_empty() {
                sections.push(LitReviewSection {
                    heading: std::mem::take(&mut current_heading),
                    content: current_content.join("\n"),
                    cited_papers: vec![],
                });
                current_content.clear();
            }
            current_heading = trimmed.to_string();
        } else if in_summary {
            summary.push_str(line);
            summary.push('\n');
        } else if !current_heading.is_empty() {
            current_content.push(line.to_string());
        }
    }
    if !current_heading.is_empty() && !current_content.is_empty() {
        sections.push(LitReviewSection {
            heading: current_heading,
            content: current_content.join("\n"),
            cited_papers: vec![],
        });
    }

    let references: Vec<String> = papers.iter().map(|p| {
        let authors = p.authors.join(", ");
        format!("{} ({})", p.title, authors)
    }).collect();
    (sections, summary.trim().to_string(), references)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LitReviewPaper {
    pub title: String,
    pub authors: Vec<String>,
    pub abstract_text: String,
    pub year: i32,
    pub arxiv_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_review_sections() {
        let text = "## Theme\nContent about theme.\n## Contributions\nContent about contributions.\n## Summary\nOverall conclusion.";
        let papers = vec![LitReviewPaper {
            title: "Test Paper".to_string(),
            authors: vec!["Author A".to_string()],
            abstract_text: String::new(),
            year: 2024,
            arxiv_id: None,
        }];
        let (sections, summary, _) = parse_review(text, &papers);
        assert_eq!(sections.len(), 2);
        assert!(!summary.is_empty());
    }
}
