//! LLM-powered Paper Analyzer.
//!
//! Analyzes paper PDF text using LLM prompts to produce structured analysis
//! with sections, rubric scores, and keyword extraction.
//! Mirrors llm/research/paper_analyzer.py

use crate::{LlmClient, Message};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;

static SECTION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^##\s*\d+\.?\d*\s*.*$").expect("valid regex"));
static RUBRIC_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#""([^"]+)":\s*(\d+)"#).expect("valid regex"));

// ─── Section Keys ─────────────────────────────────────────────────────────────

pub const SECTION_KEYS: &[&str] = &[
    "## 1. 背景",
    "## 2. 核心问题",
    "## 3.1 架构拆解",
    "## 3.2 算法逻辑",
    "## 3.3 关键组件",
    "## 4. 关键创新",
    "## 5.1 数据集",
    "## 5.2 基线对比",
    "## 5.3 消融实验",
    "## 5.4 成本分析",
    "## 6. 对抗式审稿",
    "## 7. 优势",
    "## 8. 局限",
    "## 9. 本质抽象",
    "## 10. 与其他方法对比",
    "## 11.  Decision",
    "## 12. 知识蒸馏",
    "## 13. 认知升级",
];

pub const RUBRIC_KEYS: &[&str] = &["novelty", "leverage", "evidence", "cost", "moat", "adoption"];

pub const METHOD_KEYWORDS: &[&str] = &[
    "transformer", "attention", "cnn", "rnn", "lstm", "gru", "gnn", "diffusion",
    "reinforcement", "bert", "gpt", "llm", "foundation", "multi-modal", "contrastive",
    "self-supervised", "semi-supervised", "few-shot", "zero-shot", "transfer",
];

// ─── Result Types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperAnalysisResult {
    pub sections: HashMap<String, String>,
    pub rubric: HashMap<String, u32>,
    pub keywords: Vec<String>,
    pub verification_warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PaperAnalysisVerificationResult {
    pub is_valid: bool,
    pub warnings: Vec<String>,
}

impl PaperAnalysisVerificationResult {
    pub fn valid() -> Self {
        Self { is_valid: true, warnings: Vec::new() }
    }

    pub fn with_warnings(warnings: Vec<String>) -> Self {
        Self { is_valid: warnings.is_empty(), warnings }
    }
}

// ─── Analyze Paper ────────────────────────────────────────────────────────────

/// Analyze a paper using LLM, with fallback to basic keyword analysis.
pub async fn analyze_paper(
    llm: &dyn LlmClient,
    model: &str,
    title: &str,
    abstract_text: &str,
    authors: &str,
    body: &str,
) -> PaperAnalysisResult {
    let user_prompt = format!(
        "论文标题：{}\n作者：{}\n\n【Abstract】\n{}\n\n【抽取正文片段】\n{}\n\n请按章节要求输出分析报告：",
        title, authors, abstract_text, body
    );

    let messages = vec![
        Message { role: "user".to_string(), content: user_prompt },
    ];

    match llm.complete(messages, model, 0.3, 3000).await {
        Ok(crate::LlmResponse::NonStream(ns)) => {
            let mut result = parse_analysis(&ns.content);
            let verification = verify_paper_analysis(llm, model, title, abstract_text, body, &result.sections).await;
            result.verification_warnings = verification.warnings;
            result
        }
        _ => PaperAnalysisResult {
            sections: HashMap::new(),
            rubric: HashMap::new(),
            keywords: extract_keywords(&format!("{} {}", title, abstract_text)),
            verification_warnings: Vec::new(),
        },
    }
}

/// Parse LLM response into sections + rubric
fn parse_analysis(response: &str) -> PaperAnalysisResult {
    let mut sections = HashMap::new();
    let mut rubric = HashMap::new();
    let mut current_section = String::new();
    let mut current_content = Vec::new();

    for line in response.lines() {
        let trimmed = line.trim();
        if SECTION_RE.is_match(trimmed) {
            // Save previous section
            if !current_section.is_empty() {
                sections.insert(current_section.clone(), current_content.join("\n").trim().to_string());
                current_content.clear();
            }
            current_section = trimmed.to_string();

            // Check if this looks like a rubric section
            for sec_key in SECTION_KEYS {
                if trimmed.contains(sec_key.trim_start_matches("## ")) {
                    current_section = sec_key.to_string();
                    break;
                }
            }
        } else if current_section.is_empty() {
            continue;
        } else {
            current_content.push(line.to_string());
        }
    }

    // Last section
    if !current_section.is_empty() {
        sections.insert(current_section, current_content.join("\n").trim().to_string());
    }

    // Extract rubric from entire response
    for cap in RUBRIC_RE.captures_iter(response) {
        let key = cap[1].to_lowercase();
        let val: u32 = cap[2].parse().unwrap_or(3);
        if key.len() <= 20 && (1..=5).contains(&val) {
            rubric.insert(key, val);
        }
    }

    // Build keyword set from all text content
    let all_text: String = sections.values().cloned().collect::<Vec<_>>().join(" ");
    let keywords = extract_keywords(&all_text);

    PaperAnalysisResult { sections, rubric, keywords, verification_warnings: Vec::new() }
}

const VERIFY_PAPER_ANALYSIS_PROMPT: &str = r#"你是一个严谨的论文分析验证助手。检查以下分析内容是否准确基于论文。

论文标题: {title}
论文摘要: {abstract}

分析章节数: {section_count}
方法论关键词: {keywords}

请验证：
1. 分析是否与论文摘要一致？
2. 章节内容是否有原文支持？
3. 评分是否在合理范围内(1-5)？

请以JSON格式返回：
{{"is_valid": true/false, "warnings": ["问题1", "问题2"]}}

如果分析准确，返回 {{"is_valid": true, "warnings": []}}。
如果有问题，返回 {{"is_valid": false, "warnings": ["具体问题"]}}。"#;

async fn verify_paper_analysis(
    llm: &dyn LlmClient,
    model: &str,
    title: &str,
    abstract_text: &str,
    body: &str,
    sections: &HashMap<String, String>,
) -> PaperAnalysisVerificationResult {
    if sections.is_empty() {
        return PaperAnalysisVerificationResult::valid();
    }

    let section_count = sections.len();
    let body_preview = body.chars().take(200).collect::<String>();
    let abstract_preview = abstract_text.chars().take(300).collect::<String>();

    let prompt = VERIFY_PAPER_ANALYSIS_PROMPT
        .replace("{title}", title)
        .replace("{abstract}", &abstract_preview)
        .replace("{section_count}", &section_count.to_string())
        .replace("{keywords}", &body_preview);

    let msg = Message { role: "user".to_string(), content: prompt };

    match llm.complete(vec![msg], model, 0.1, 300).await {
        Ok(crate::LlmResponse::NonStream(ns)) => {
            parse_paper_verification_result(&ns.content)
        }
        _ => PaperAnalysisVerificationResult::valid(),
    }
}

fn parse_paper_verification_result(content: &str) -> PaperAnalysisVerificationResult {
    let content = content.trim();

    let _is_valid = if content.contains("\"is_valid\": true") || content.contains("\"is_valid\":true") {
        true
    } else if content.contains("\"is_valid\": false") || content.contains("\"is_valid\":false") {
        false
    } else {
        return PaperAnalysisVerificationResult::valid();
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

    PaperAnalysisVerificationResult::with_warnings(warnings)
}

/// Extract known method keywords from text
fn extract_keywords(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    METHOD_KEYWORDS.iter()
        .filter(|kw| lower.contains(*kw))
        .map(|s| s.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_analysis_with_sections() {
        let response = "## 1. 背景\nThis is background content.\n\n## 2. 核心问题\nCore problem description.\n\n{\"novelty\": 4, \"evidence\": 3}";
        let result = parse_analysis(response);

        assert!(result.sections.contains_key("## 1. 背景"));
        assert!(result.sections.contains_key("## 2. 核心问题"));
        assert!(result.rubric.contains_key("novelty"));
        assert_eq!(result.rubric.get("novelty"), Some(&4));
    }

    #[test]
    fn test_empty_response() {
        let result = parse_analysis("");
        assert!(result.sections.is_empty());
        assert!(result.rubric.is_empty());
    }

    #[test]
    fn test_extract_keywords() {
        let kws = extract_keywords("transformer attention model");
        assert!(kws.contains(&"transformer".to_string()));
        assert!(kws.contains(&"attention".to_string()));
        assert!(!kws.contains(&"model".to_string()));
    }

    #[test]
    fn test_rubric_extraction() {
        let text = "Some text {\"novelty\": 5, \"evidence\": 2} more text";
        let result = parse_analysis(text);
        assert_eq!(result.rubric.get("novelty"), Some(&5));
        assert_eq!(result.rubric.get("evidence"), Some(&2));
    }

    #[test]
    fn test_parse_paper_verification_result_valid() {
        let json = r#"{"is_valid": true, "warnings": []}"#;
        let result = parse_paper_verification_result(json);
        assert!(result.is_valid);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_parse_paper_verification_result_invalid() {
        let json = r#"{"is_valid": false, "warnings": ["分析内容与原文不符"]}"#;
        let result = parse_paper_verification_result(json);
        assert!(!result.is_valid);
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn test_paper_analysis_verification_result_valid() {
        let result = PaperAnalysisVerificationResult::valid();
        assert!(result.is_valid);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_paper_analysis_verification_result_with_warnings() {
        let warnings = vec!["评分过高".to_string()];
        let result = PaperAnalysisVerificationResult::with_warnings(warnings);
        assert!(!result.is_valid);
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn test_parse_paper_verification_result_malformed() {
        let json = "not json";
        let result = parse_paper_verification_result(json);
        assert!(result.is_valid);
    }
}
