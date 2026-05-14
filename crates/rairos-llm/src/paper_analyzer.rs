//! LLM-powered Paper Analyzer.
//!
//! Analyzes paper PDF text using LLM prompts to produce structured analysis
//! with sections, rubric scores, and keyword extraction.
//! Mirrors llm/research/paper_analyzer.py

use crate::{LlmClient, Message};
use regex::Regex;
use std::collections::HashMap;

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

// ─── System Prompt ────────────────────────────────────────────────────────────

const SYSTEM_PROMPT: &str = r#"你是一个严谨的 AI 研究助理，擅长对抗式审稿和深度论文分析。

任务：分析论文，按指定格式输出各章节内容。

硬规则：
1. 每个章节标题必须严格使用 `## N. 标题` 格式
2. 内容必须基于论文原文；不确定的加 [推测] 标注
3. 禁止捏造实验/数据/结果
4. 输出中文 Markdown
5. 每条关键陈述必须标注来源页码，格式为 [Page N]
6. 末尾输出 JSON 评分块（见评分量表说明）

评分量表：
- Novelty (1-5): 1=增量改进 2=组合已有 3=新任务/视角 4=新范式 5=开创性
- Leverage (1-5): 1=难落地 2=需适配 3=可直接用 4=显著降本 5=范式级
- Evidence (1-5): 1=无实验 2=部分 3=充分覆盖 4=强基线 5=消融完整
- Cost (1-5): 1=极高 2=较高 3=中等 4=较低 5=极低
- Moat (1-5): 1=无壁垒 2=代码 3=数据 4=算法/专利 5=生态
- Adoption (1-5): 1=无 2=<100stars 3>1k/引用>10 4=工业落地 5=生态标配"#;

// ─── Result Types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PaperAnalysisResult {
    pub sections: HashMap<String, String>,
    pub rubric: HashMap<String, u32>,
    pub keywords: Vec<String>,
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
            parse_analysis(&ns.content)
        }
        _ => PaperAnalysisResult {
            sections: HashMap::new(),
            rubric: HashMap::new(),
            keywords: extract_keywords(&format!("{} {}", title, abstract_text)),
        },
    }
}

/// Parse LLM response into sections + rubric
fn parse_analysis(response: &str) -> PaperAnalysisResult {
    let mut sections = HashMap::new();
    let mut rubric = HashMap::new();
    let mut current_section = String::new();
    let mut current_content = Vec::new();

    let section_re = Regex::new(r"^##\s*\d+\.?\d*\s*.*$").unwrap();
    let rubric_re = Regex::new(r#""([^"]+)":\s*(\d+)"#).unwrap();

    for line in response.lines() {
        let trimmed = line.trim();
        if section_re.is_match(trimmed) {
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
    for cap in rubric_re.captures_iter(response) {
        let key = cap[1].to_lowercase();
        let val: u32 = cap[2].parse().unwrap_or(3);
        if key.len() <= 20 && val >= 1 && val <= 5 {
            rubric.insert(key, val);
        }
    }

    // Build keyword set from all text content
    let all_text: String = sections.values().cloned().collect::<Vec<_>>().join(" ");
    let keywords = extract_keywords(&all_text);

    PaperAnalysisResult { sections, rubric, keywords }
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
}
