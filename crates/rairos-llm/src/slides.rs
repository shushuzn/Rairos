//! Paper Slides Generator — converts briefing into presentation outline.
//!
//! Mirrors llm/slides_generator.py

use crate::LlmClient;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slide {
    pub title: String,
    pub content: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlidesResult {
    pub title: String,
    pub slides: Vec<Slide>,
    pub speaker_notes: String,
}

pub async fn generate_slides(
    llm: &dyn LlmClient,
    model: &str,
    paper_title: &str,
    briefing_markdown: &str,
) -> SlidesResult {
    let prompt = format!(
        "Convert this paper briefing into a presentation outline (6-10 slides).\
        \nFormat: each slide has a title and 3-5 bullet points.\n\
        \nTitle: {}\n\nBriefing:\n{}",
        paper_title, briefing_markdown
    );

    let msg = crate::Message { role: "user".to_string(), content: prompt };

    let body = match llm.complete(vec![msg], model, 0.3, 2000).await {
        Ok(crate::LlmResponse::NonStream(ns)) => ns.content,
        _ => return SlidesResult {
            title: paper_title.to_string(),
            slides: vec![],
            speaker_notes: String::new(),
        },
    };

    parse_slides(paper_title, &body)
}

fn parse_slides(paper_title: &str, text: &str) -> SlidesResult {
    let mut slides = Vec::new();
    let mut current_title = String::new();
    let mut current_bullets: Vec<String> = Vec::new();
    let mut speaker_notes = String::new();
    let mut in_notes = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## ") || trimmed.starts_with("Slide") {
            if !current_title.is_empty() {
                slides.push(Slide {
                    title: std::mem::take(&mut current_title),
                    content: std::mem::take(&mut current_bullets),
                });
            }
            current_title = trimmed.trim_start_matches('#').trim().to_string();
            in_notes = false;
        } else if trimmed.starts_with("Notes:") || trimmed.starts_with("Speaker notes:") {
            in_notes = true;
        } else if in_notes {
            speaker_notes.push_str(trimmed);
            speaker_notes.push('\n');
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            current_bullets.push(trimmed.trim_start_matches("- ").trim_start_matches("* ").to_string());
        }
    }
    if !current_title.is_empty() {
        slides.push(Slide { title: current_title, content: current_bullets });
    }

    SlidesResult {
        title: paper_title.to_string(),
        slides,
        speaker_notes: speaker_notes.trim().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_slides_basic() {
        let text = "## Introduction\n- Point 1\n- Point 2\n## Method\n- Step A";
        let result = parse_slides("Test", text);
        assert_eq!(result.slides.len(), 2);
        assert_eq!(result.slides[0].title, "Introduction");
        assert_eq!(result.slides[0].content.len(), 2);
    }
}
