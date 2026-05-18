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
    pub verification_warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct LitReviewVerificationResult {
    pub is_valid: bool,
    pub warnings: Vec<String>,
}

impl LitReviewVerificationResult {
    pub fn valid() -> Self {
        Self { is_valid: true, warnings: Vec::new() }
    }

    pub fn with_warnings(warnings: Vec<String>) -> Self {
        Self { is_valid: warnings.is_empty(), warnings }
    }
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
            verification_warnings: Vec::new(),
        },
    };

    let (sections, summary, references) = parse_review(&ns.content, papers);
    let verification = verify_lit_review(llm, model, topic, papers, &sections, &summary).await;
    LitReviewResult {
        title: topic.to_string(),
        sections,
        summary,
        references,
        verification_warnings: verification.warnings,
    }
}

const VERIFY_LIT_REVIEW_PROMPT: &str = r#"你是一个严谨的文献综述验证助手。检查以下文献综述是否准确基于提供的论文。

主题: {topic}
论文数量: {paper_count}

综述摘要: {summary}

请验证：
1. 综述是否涵盖所有论文的主要贡献？
2. 综述是否准确反映论文之间的关系？
3. 总结是否与论文内容一致？

请以JSON格式返回：
{{"is_valid": true/false, "warnings": ["问题1", "问题2"]}}

如果综述准确，返回 {{"is_valid": true, "warnings": []}}。
如果有问题，返回 {{"is_valid": false, "warnings": ["具体问题"]}}。"#;

async fn verify_lit_review(
    llm: &dyn LlmClient,
    model: &str,
    topic: &str,
    papers: &[LitReviewPaper],
    sections: &[LitReviewSection],
    summary: &str,
) -> LitReviewVerificationResult {
    if sections.is_empty() || summary.is_empty() {
        return LitReviewVerificationResult::valid();
    }

    let paper_count = papers.len();
    let summary_preview = summary.chars().take(300).collect::<String>();

    let prompt = VERIFY_LIT_REVIEW_PROMPT
        .replace("{topic}", topic)
        .replace("{paper_count}", &paper_count.to_string())
        .replace("{summary}", &summary_preview);

    let msg = crate::Message { role: "user".to_string(), content: prompt };

    match llm.complete(vec![msg], model, 0.1, 300).await {
        Ok(crate::LlmResponse::NonStream(ns)) => {
            parse_verification_result(&ns.content)
        }
        _ => LitReviewVerificationResult::valid(),
    }
}

fn parse_verification_result(content: &str) -> LitReviewVerificationResult {
    let content = content.trim();

    let _is_valid = if content.contains("\"is_valid\": true") || content.contains("\"is_valid\":true") {
        true
    } else if content.contains("\"is_valid\": false") || content.contains("\"is_valid\":false") {
        false
    } else {
        return LitReviewVerificationResult::valid();
    };

    let mut warnings = Vec::new();
    if let Some(start) = content.find("\"warnings\":") {
        let warnings_str = &content[start..];
        if let Some(arr_start) = warnings_str.find('[') {
            if let Some(arr_end) = warnings_str.find(']') {
                let items = &warnings_str[arr_start + 1..arr_end];
                for item in items.split(',') {
                    let item = item.trim().trim_matches('"').trim_matches(|c| c == '"' || c == ' ');
                    if !item.is_empty() && item != "[]" && item != "warnings" {
                        warnings.push(item.to_string());
                    }
                }
            }
        }
    }

    LitReviewVerificationResult::with_warnings(warnings)
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

    #[test]
    fn test_parse_verification_result_valid() {
        let json = r#"{"is_valid": true, "warnings": []}"#;
        let result = parse_verification_result(json);
        assert!(result.is_valid);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_parse_verification_result_invalid() {
        let json = r#"{"is_valid": false, "warnings": ["综述未涵盖所有论文"]}"#;
        let result = parse_verification_result(json);
        assert!(!result.is_valid);
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn test_parse_verification_result_malformed() {
        let json = "not json at all";
        let result = parse_verification_result(json);
        assert!(result.is_valid);
    }

    #[test]
    fn test_lit_review_verification_result_valid() {
        let result = LitReviewVerificationResult::valid();
        assert!(result.is_valid);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_lit_review_verification_result_with_warnings() {
        let warnings = vec!["总结不准确".to_string()];
        let result = LitReviewVerificationResult::with_warnings(warnings);
        assert!(!result.is_valid);
        assert_eq!(result.warnings.len(), 1);
    }
}
