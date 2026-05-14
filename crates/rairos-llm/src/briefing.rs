//! Briefing Generator — produces structured research briefings via LLM.
//!
//! Mirrors llm/briefing_generator.py

use crate::{LlmClient, Message};
use serde::{Deserialize, Serialize};

const BRIEFING_SYSTEM: &str = r#"You are a research briefing assistant. Generate a concise, structured briefing for a research paper.

Output format (Markdown):
## Summary
2-3 sentence summary of the paper.

## Key Contributions
- Bullet list of 3-5 main contributions

## Methodology
Brief description of the approach.

## Results
Key findings and results.

## Relevance
Why this matters to the research community.

## Verdict
Strong Accept / Accept / Weak Accept / Borderline / Reject"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefingResult {
    pub success: bool,
    pub arxiv_id: String,
    pub summary: String,
    pub key_contributions: Vec<String>,
    pub methodology: String,
    pub results: String,
    pub relevance: String,
    pub verdict: String,
    pub markdown: String,
}

pub async fn generate_briefing(
    llm: &dyn LlmClient,
    model: &str,
    arxiv_id: &str,
    title: &str,
    abstract_text: &str,
    authors: &[String],
) -> BriefingResult {
    let prompt = format!(
        "Paper: {}\nAuthors: {}\n\nAbstract:\n{}\n\nGenerate a structured briefing.",
        title,
        authors.join(", "),
        abstract_text
    );

    let msg = Message { role: "user".to_string(), content: prompt };

    match llm.complete(vec![msg], model, 0.3, 1500).await {
        Ok(crate::LlmResponse::NonStream(ns)) => {
            let markdown = ns.content;
            let summary = extract_section(&markdown, "## Summary")
                .unwrap_or_else(|| "No summary generated.".to_string());
            let contributions: Vec<String> = extract_bullets(&markdown, "## Key Contributions");
            let methodology = extract_section(&markdown, "## Methodology")
                .unwrap_or_else(|| "Not described.".to_string());
            let results = extract_section(&markdown, "## Results")
                .unwrap_or_else(|| "Not described.".to_string());
            let relevance = extract_section(&markdown, "## Relevance")
                .unwrap_or_else(|| "Not assessed.".to_string());
            let verdict = extract_section(&markdown, "## Verdict")
                .unwrap_or_else(|| "No verdict.".to_string());

            BriefingResult {
                success: true,
                arxiv_id: arxiv_id.to_string(),
                summary,
                key_contributions: contributions,
                methodology,
                results,
                relevance,
                verdict,
                markdown,
            }
        }
        _ => BriefingResult {
            success: false,
            arxiv_id: arxiv_id.to_string(),
            summary: String::new(),
            key_contributions: Vec::new(),
            methodology: String::new(),
            results: String::new(),
            relevance: String::new(),
            verdict: String::new(),
            markdown: String::new(),
        },
    }
}

fn extract_section(text: &str, heading: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut found = false;
    let mut content = Vec::new();
    for line in lines {
        if line.trim().starts_with(heading) {
            found = true;
            continue;
        }
        if found {
            if line.starts_with("## ") { break; }
            content.push(line);
        }
    }
    if content.is_empty() { None } else { Some(content.join("\n").trim().to_string()) }
}

fn extract_bullets(text: &str, heading: &str) -> Vec<String> {
    let section = extract_section(text, heading).unwrap_or_default();
    section.lines()
        .filter(|l| l.trim().starts_with("- ") || l.trim().starts_with("* "))
        .map(|l| l.trim().trim_start_matches("- ").trim_start_matches("* ").to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_section() {
        let text = "## Summary\nThis is a summary.\n\n## Results\nKey finding.";
        assert_eq!(extract_section(text, "## Summary"), Some("This is a summary.".to_string()));
    }

    #[test]
    fn test_extract_bullets() {
        let text = "## Key Contributions\n- Contribution 1\n- Contribution 2";
        let bullets = extract_bullets(text, "## Key Contributions");
        assert_eq!(bullets.len(), 2);
        assert_eq!(bullets[0], "Contribution 1");
    }
}
