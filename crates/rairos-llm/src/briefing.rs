//! Briefing Generator — produces structured research briefings via LLM.
//!
//! Mirrors llm/briefing_generator.py

use crate::{LlmClient, Message};
use serde::{Deserialize, Serialize};
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
    pub verification_warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BriefingVerificationResult {
    pub is_valid: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BriefingVerificationInput<'a> {
    pub title: &'a str,
    pub abstract_text: &'a str,
    pub summary: &'a str,
    pub contributions: &'a [String],
    pub methodology: &'a str,
    pub results: &'a str,
}

impl BriefingVerificationResult {
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            warnings: Vec::new(),
        }
    }

    pub fn with_warnings(warnings: Vec<String>) -> Self {
        Self {
            is_valid: warnings.is_empty(),
            warnings,
        }
    }
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

            let verification = verify_briefing(
                llm,
                model,
                BriefingVerificationInput {
                    title,
                    abstract_text,
                    summary: &summary,
                    contributions: &contributions,
                    methodology: &methodology,
                    results: &results,
                },
            )
            .await;

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
                verification_warnings: verification.warnings,
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
            verification_warnings: Vec::new(),
        },
    }
}

const VERIFY_BRIEFING_PROMPT: &str = r#"你是一个严谨的研究简报验证助手。检查以下简报内容是否准确基于论文。

论文标题: {title}
论文摘要: {abstract}

简报摘要: {summary}
关键贡献: {contributions}
方法论: {methodology}
结果: {results}

请验证：
1. 摘要是否准确反映论文主题？
2. 关键贡献是否有论文支持？
3. 方法论描述是否与论文一致？
4. 结果描述是否有原文支持？

请以JSON格式返回：
{{"is_valid": true/false, "warnings": ["问题1", "问题2"]}}

如果简报准确，返回 {{"is_valid": true, "warnings": []}}。
如果有问题，返回 {{"is_valid": false, "warnings": ["具体问题"]}}。"#;

async fn verify_briefing(
    llm: &dyn LlmClient,
    model: &str,
    input: BriefingVerificationInput<'_>,
) -> BriefingVerificationResult {
    if input.summary.is_empty() || input.summary == "No summary generated." {
        return BriefingVerificationResult::valid();
    }

    let contributions_str = if input.contributions.is_empty() {
        "无".to_string()
    } else {
        input.contributions.iter().map(|s| format!("- {}", s)).collect::<Vec<_>>().join("\n")
    };

    let prompt = VERIFY_BRIEFING_PROMPT
        .replace("{title}", input.title)
        .replace("{abstract}", &input.abstract_text.chars().take(500).collect::<String>())
        .replace("{summary}", input.summary)
        .replace("{contributions}", &contributions_str)
        .replace("{methodology}", input.methodology)
        .replace("{results}", input.results);

    let msg = Message { role: "user".to_string(), content: prompt };

    match llm.complete(vec![msg], model, 0.1, 300).await {
        Ok(crate::LlmResponse::NonStream(ns)) => {
            parse_verification_result(&ns.content)
        }
        _ => BriefingVerificationResult::valid(),
    }
}

fn parse_verification_result(content: &str) -> BriefingVerificationResult {
    let content = content.trim();

    if !content.contains("\"is_valid\": true") && !content.contains("\"is_valid\":true")
        && !content.contains("\"is_valid\": false") && !content.contains("\"is_valid\":false") {
        return BriefingVerificationResult::valid();
    }

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

    BriefingVerificationResult::with_warnings(warnings)
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

    #[test]
    fn test_parse_verification_result_valid() {
        let json = r#"{"is_valid": true, "warnings": []}"#;
        let result = parse_verification_result(json);
        assert!(result.is_valid);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_parse_verification_result_invalid() {
        let json = r#"{"is_valid": false, "warnings": ["摘要与论文不符", "方法论描述错误"]}"#;
        let result = parse_verification_result(json);
        assert!(!result.is_valid);
        assert_eq!(result.warnings.len(), 2);
    }

    #[test]
    fn test_parse_verification_result_malformed() {
        let json = "not json at all";
        let result = parse_verification_result(json);
        assert!(result.is_valid);
    }

    #[test]
    fn test_briefing_verification_result_valid() {
        let result = BriefingVerificationResult::valid();
        assert!(result.is_valid);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_briefing_verification_result_with_warnings() {
        let warnings = vec!["摘要过短".to_string(), "缺少方法论描述".to_string()];
        let result = BriefingVerificationResult::with_warnings(warnings.clone());
        assert!(!result.is_valid);
        assert_eq!(result.warnings, warnings);
    }
}
