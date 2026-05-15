//! rairos-litreview-generator — LLM-powered literature review generator.
#![allow(dead_code)]
#![allow(clippy::too_many_arguments)]
//!
//! Ported from `llm/litreview_generator.py`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paper {
    pub arxiv_id: Option<String>,
    pub title: Option<String>,
    #[serde(default)]
    pub abstract_text: Option<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub published: Option<String>,
    #[serde(default)]
    pub score: f64,
}

impl Paper {
    pub fn title(&self) -> &str {
        self.title.as_deref().unwrap_or("Untitled")
    }

    pub fn abstract_text(&self) -> &str {
        self.abstract_text.as_deref().unwrap_or("")
    }

    pub fn published(&self) -> &str {
        self.published.as_deref().unwrap_or("")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LitReviewSection {
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub paper_refs: Vec<String>,
    #[serde(default)]
    pub subsection_titles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LitReview {
    pub topic: String,
    #[serde(default)]
    pub sections: Vec<LitReviewSection>,
    #[serde(default)]
    pub papers_used: Vec<String>,
    pub total_papers: usize,
    #[serde(default)]
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LitReviewResult {
    pub success: bool,
    pub topic: String,
    #[serde(default)]
    pub review: Option<LitReview>,
    #[serde(default)]
    pub markdown: String,
    #[serde(default)]
    pub error: String,
}

pub struct LitReviewGenerator {
    pub db: Option<()>,
}

impl LitReviewGenerator {
    pub fn new() -> Self {
        Self { db: None }
    }

    pub fn generate(
        &self,
        topic: &str,
        limit: usize,
        _use_llm: bool,
        _api_key: Option<&str>,
        _base_url: Option<&str>,
        _model: Option<&str>,
        output_dir: Option<PathBuf>,
    ) -> LitReviewResult {
        let papers = self.collect_papers(topic, limit);
        if papers.is_empty() {
            return LitReviewResult {
                success: false,
                topic: topic.to_string(),
                review: None,
                markdown: String::new(),
                error: format!("No papers found for topic: {}", topic),
            };
        }

        let review = self.generate_template_review(topic, &papers);

        let markdown = self.render_markdown(&review);

        let result = LitReviewResult {
            success: true,
            topic: topic.to_string(),
            review: Some(review),
            markdown: markdown.clone(),
            error: String::new(),
        };

        if let Some(dir) = output_dir {
            if result.success {
                let _ = self.save_review(result.review.as_ref().unwrap(), &markdown, &dir);
            }
        }

        result
    }

    fn collect_papers(&self, _topic: &str, _limit: usize) -> Vec<Paper> {
        Vec::new()
    }

    fn build_papers_text(&self, papers: &[Paper]) -> String {
        let mut lines = Vec::new();
        for (i, p) in papers.iter().enumerate().take(30) {
            let authors = if !p.authors.is_empty() {
                p.authors
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                "Unknown".to_string()
            };
            let year = p.published().chars().take(4).collect::<String>();
            let title = p.title();
            let abstract_snippet = &p.abstract_text()[..p.abstract_text().len().min(300)];
            lines.push(format!(
                "[{}] ({}) {}\n    Authors: {}\n    Abstract: {}...",
                i + 1,
                year,
                title,
                authors,
                abstract_snippet
            ));
        }
        lines.join("\n\n")
    }

    fn generate_template_review(&self, topic: &str, papers: &[Paper]) -> LitReview {
        let now = chrono::Utc::now().to_rfc3339();

        let mut by_year: HashMap<String, Vec<&Paper>> = HashMap::new();
        for p in papers {
            let year = p.published().chars().take(4).collect::<String>();
            by_year.entry(year).or_default().push(p);
        }

        let years: Vec<_> = by_year.keys().cloned().collect();
        let min_year = years
            .iter()
            .min()
            .cloned()
            .unwrap_or_else(|| "N/A".to_string());
        let max_year = years
            .iter()
            .max()
            .cloned()
            .unwrap_or_else(|| "N/A".to_string());

        let mut scored_papers = papers.to_vec();
        scored_papers.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let top: Vec<_> = scored_papers.iter().take(10).collect();

        let sections = vec![
            LitReviewSection {
                title: "Overview".to_string(),
                content: format!(
                    "This review covers {} papers on **{}**. \
                     Research spans {} to {}.",
                    papers.len(),
                    topic,
                    min_year,
                    max_year
                ),
                paper_refs: Vec::new(),
                subsection_titles: Vec::new(),
            },
            LitReviewSection {
                title: "Top Papers by Relevance".to_string(),
                content: top
                    .iter()
                    .map(|p| {
                        format!(
                            "- **{}** ({})",
                            &p.title()[..p.title().len().min(60)],
                            p.published().chars().take(4).collect::<String>()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
                paper_refs: Vec::new(),
                subsection_titles: Vec::new(),
            },
            LitReviewSection {
                title: "Timeline".to_string(),
                content: {
                    let mut year_entries: Vec<_> = by_year.iter().collect();
                    year_entries.sort_by(|a, b| b.0.cmp(a.0));
                    year_entries
                        .iter()
                        .map(|(year, plist)| format!("- **{}**: {} paper(s)", year, plist.len()))
                        .collect::<Vec<_>>()
                        .join("\n")
                },
                paper_refs: Vec::new(),
                subsection_titles: Vec::new(),
            },
        ];

        LitReview {
            topic: topic.to_string(),
            sections,
            papers_used: papers.iter().filter_map(|p| p.arxiv_id.clone()).collect(),
            total_papers: papers.len(),
            generated_at: now,
        }
    }

    fn render_markdown(&self, review: &LitReview) -> String {
        let now = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let mut lines = vec![
            format!("# Literature Review: {}", review.topic),
            String::new(),
            format!(
                "**Generated:** {} | **Papers reviewed:** {}",
                now, review.total_papers
            ),
            String::new(),
            "---".to_string(),
            String::new(),
        ];

        for section in &review.sections {
            lines.push(format!("## {}", section.title));
            lines.push(String::new());
            if !section.content.is_empty() {
                lines.push(section.content.clone());
            }
            lines.push(String::new());
        }

        lines.push("---".to_string());
        lines.push(format!(
            "_Generated by Rairos LitReviewGenerator on {}_",
            now
        ));

        lines.join("\n")
    }

    fn save_review(
        &self,
        review: &LitReview,
        markdown: &str,
        output_dir: &Path,
    ) -> Option<PathBuf> {
        let dir = output_dir.to_path_buf();
        if fs::create_dir_all(&dir).is_err() {
            return None;
        }

        let slug: String = review
            .topic
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
            .chars()
            .take(40)
            .collect();

        let filename = format!(
            "litreview_{}_{}.md",
            slug,
            &uuid::Uuid::new_v4().to_string()[..6]
        );
        let filepath = dir.join(&filename);

        if fs::write(&filepath, markdown).is_ok() {
            Some(filepath)
        } else {
            None
        }
    }
}

impl Default for LitReviewGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_paper(arxiv_id: &str, title: &str, published: &str, score: f64) -> Paper {
        Paper {
            arxiv_id: Some(arxiv_id.to_string()),
            title: Some(title.to_string()),
            abstract_text: Some("Abstract text".to_string()),
            authors: vec!["Author 1".to_string(), "Author 2".to_string()],
            published: Some(published.to_string()),
            score,
        }
    }

    #[test]
    fn test_generate_with_no_papers() {
        let generator = LitReviewGenerator::new();
        let result = generator.generate("nonexistent topic", 10, false, None, None, None, None);
        assert!(!result.success);
        assert!(result.error.contains("No papers found"));
    }

    #[test]
    fn test_generate_with_stubbed_collect_returns_no_papers() {
        let generator = LitReviewGenerator::new();
        let result = generator.generate("AI Research", 10, false, None, None, None, None);
        assert!(!result.success);
        assert!(result.error.contains("No papers found"));
    }

    #[test]
    fn test_generate_with_stubbed_collect_papers() {
        let generator = LitReviewGenerator::new();
        let result = generator.generate("AI Research", 10, false, None, None, None, None);
        assert!(!result.success);
        assert!(result.error.contains("No papers found"));
    }

    #[test]
    fn test_build_papers_text() {
        let generator = LitReviewGenerator::new();
        let papers = vec![make_paper("2301.00001", "Test Paper", "2023-06-15", 0.9)];
        let text = generator.build_papers_text(&papers);
        assert!(text.contains("Test Paper"));
        assert!(text.contains("2023"));
    }

    #[test]
    fn test_render_markdown() {
        let generator = LitReviewGenerator::new();
        let review = LitReview {
            topic: "Test Topic".to_string(),
            sections: vec![LitReviewSection {
                title: "Overview".to_string(),
                content: "Test content".to_string(),
                paper_refs: Vec::new(),
                subsection_titles: Vec::new(),
            }],
            papers_used: vec!["2301.00001".to_string()],
            total_papers: 1,
            generated_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let markdown = generator.render_markdown(&review);
        assert!(markdown.contains("Literature Review: Test Topic"));
        assert!(markdown.contains("Overview"));
        assert!(markdown.contains("Test content"));
    }

    #[test]
    fn test_litreview_result_fields() {
        let result = LitReviewResult {
            success: true,
            topic: "AI".to_string(),
            review: None,
            markdown: "# Test".to_string(),
            error: String::new(),
        };
        assert!(result.success);
        assert_eq!(result.topic, "AI");
    }

    #[test]
    fn test_litreview_section_serialization() {
        let section = LitReviewSection {
            title: "Test".to_string(),
            content: "Content".to_string(),
            paper_refs: vec!["ref1".to_string()],
            subsection_titles: vec!["sub1".to_string()],
        };
        let json = serde_json::to_string(&section).unwrap();
        assert!(json.contains("Test"));
        assert!(json.contains("Content"));
    }
}
