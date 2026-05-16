//! LLM-powered Research Gap Detector.
//!
//! Identifies research gaps from paper summaries using LLM prompts.
//! Mirrors llm/research/gap_detector.py
//! Falls back to rule-based detection (gap_analysis.rs) when LLM unavailable.

use crate::{LlmClient, Message};
use regex::Regex;

// ─── Prompt Templates ─────────────────────────────────────────────────────────
// ─── Gap Types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapType {
    UnexploredApplication,
    MethodLimitation,
    Contradiction,
    EvaluationGap,
    ScalabilityIssue,
    TheoreticalGap,
    DatasetGap,
    GeneralizationGap,
}

impl GapType {
    pub fn as_str(&self) -> &'static str {
        match self {
            GapType::UnexploredApplication => "unexplored_application",
            GapType::MethodLimitation => "method_limitation",
            GapType::Contradiction => "contradiction",
            GapType::EvaluationGap => "evaluation_gap",
            GapType::ScalabilityIssue => "scalability_issue",
            GapType::TheoreticalGap => "theoretical_gap",
            GapType::DatasetGap => "dataset_gap",
            GapType::GeneralizationGap => "generalization_gap",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "unexplored_application" => GapType::UnexploredApplication,
            "method_limitation" => GapType::MethodLimitation,
            "contradiction" => GapType::Contradiction,
            "evaluation_gap" => GapType::EvaluationGap,
            "scalability_issue" => GapType::ScalabilityIssue,
            "theoretical_gap" => GapType::TheoreticalGap,
            "dataset_gap" => GapType::DatasetGap,
            "generalization_gap" => GapType::GeneralizationGap,
            _ => GapType::MethodLimitation,
        }
    }
}

// ─── Gap data ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ResearchGap {
    pub gap_type: GapType,
    pub description: String,
    pub evidence_papers: Vec<String>,
    pub confidence: f64,
    pub severity: String,
}

// ─── Detect gaps using keyword patterns ────────────────────────────────────────

/// Detect research gaps using keyword pattern matching (no LLM needed).
pub fn detect_gaps_keyword(topic: &str) -> Vec<ResearchGap> {
    let lower = topic.to_lowercase();
    let mut gaps = Vec::new();

    let patterns: Vec<(&str, GapType, f64)> = vec![
        ("limited", GapType::DatasetGap, 0.4),
        ("lack of", GapType::DatasetGap, 0.5),
        ("insufficient", GapType::DatasetGap, 0.4),
        ("not well understood", GapType::TheoreticalGap, 0.5),
        ("poorly understood", GapType::TheoreticalGap, 0.5),
        ("unexplored", GapType::UnexploredApplication, 0.4),
        ("inconsistent", GapType::Contradiction, 0.4),
        ("conflicting", GapType::Contradiction, 0.5),
        ("no consensus", GapType::Contradiction, 0.6),
        ("open problem", GapType::MethodLimitation, 0.3),
        ("challenge", GapType::MethodLimitation, 0.3),
        ("future work", GapType::GeneralizationGap, 0.2),
    ];

    for (keyword, gap_type, confidence) in &patterns {
        if lower.contains(keyword) {
            gaps.push(ResearchGap {
                gap_type: *gap_type,
                description: format!("Keyword '{}' detected in topic '{}'", keyword, topic),
                evidence_papers: Vec::new(),
                confidence: *confidence,
                severity: "medium".to_string(),
            });
        }
    }

    if gaps.is_empty() {
        gaps.push(ResearchGap {
            gap_type: GapType::MethodLimitation,
            description: format!("No specific gap keywords found for '{}'", topic),
            evidence_papers: Vec::new(),
            confidence: 0.1,
            severity: "low".to_string(),
        });
    }

    gaps
}

// ─── Detect gaps using LLM ────────────────────────────────────────────────────

/// Use LLM to detect research gaps from paper summaries.
/// Falls back to empty vec on error.
pub async fn detect_gaps_llm(
    llm: &dyn LlmClient,
    model: &str,
    topic: &str,
    paper_summaries: &str,
) -> Vec<ResearchGap> {
    let user_prompt = format!("分析领域：{}\n\n论文列表：\n{}\n\n请识别该领域的研究空白：", topic, paper_summaries);

    let messages = vec![
        Message {
            role: "user".to_string(),
            content: user_prompt,
        },
    ];

    match llm.complete(messages, model, 0.3, 2000).await {
        Ok(response) => {
            match response {
                crate::LlmResponse::NonStream(non_stream) => {
                    parse_gaps(&non_stream.content, topic)
                }
                _ => Vec::new(),
            }
        }
        Err(_) => Vec::new(),
    }
}

/// Generate research questions from gaps using LLM.
pub async fn generate_questions_llm(
    llm: &dyn LlmClient,
    model: &str,
    topic: &str,
    gaps_text: &str,
) -> Vec<String> {
    let _system = "基于研究空白，生成3-5个具体、可验证的研究问题。\
    每个问题应该：1. 明确说明要研究什么 2. 有清晰的研究假设 \
    3. 包含方法建议 4. 说明预期影响\n\n\
    输出格式（每行一个问题）：\n\
    问题 | 研究假设 | 方法建议 | 预期影响 | 可行性(0-1) | 创新性(0-1)";

    let user = format!("领域：{}\n\n发现的研究空白：\n{}\n\n请生成研究问题：", topic, gaps_text);

    let messages = vec![
        Message { role: "user".to_string(), content: user },
    ];

    match llm.complete(messages, model, 0.3, 2000).await {
        Ok(crate::LlmResponse::NonStream(ns)) => {
            ns.content.lines().filter(|l| l.contains('|')).map(|l| l.to_string()).collect()
        }
        _ => Vec::new(),
    }
}

// ─── Response Parser ──────────────────────────────────────────────────────────

/// Parse LLM response into ResearchGap objects.
/// Format: [GAP_TYPE] description | papers | confidence | severity
fn parse_gaps(response: &str, _topic: &str) -> Vec<ResearchGap> {
    let re = Regex::new(r"\[(\w+)\]\s*(.+?)\s*\|\s*(.+?)\s*\|\s*([\d.]+)\s*\|\s*(\w+)").unwrap();

    // Strip thinking tags
    let thinking_re = Regex::new(r"<think>.*?</think>").unwrap();
    let cleaned = thinking_re.replace_all(response, "");

    let mut gaps = Vec::new();
    for line in cleaned.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(caps) = re.captures(line) {
            let gap_type_str = caps.get(1).unwrap().as_str();
            let description = caps.get(2).unwrap().as_str().trim();
            let papers_str = caps.get(3).unwrap().as_str();
            let conf_str = caps.get(4).unwrap().as_str();
            let severity_str = caps.get(5).unwrap().as_str();

            let confidence: f64 = conf_str.parse().unwrap_or(0.5);
            let papers: Vec<String> = papers_str.split(',')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect();

            gaps.push(ResearchGap {
                gap_type: GapType::from_str(gap_type_str),
                description: description.to_string(),
                evidence_papers: papers,
                confidence,
                severity: severity_str.to_string(),
            });
        }
    }
    gaps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gaps() {
        let response = "[method_limitation] Current methods don't scale | paper1, paper2 | 0.85 | high\n\
                        [dataset_gap] No benchmark available | paper3 | 0.6 | medium";
        let gaps = parse_gaps(response, "test");
        assert_eq!(gaps.len(), 2);
        assert!(matches!(gaps[0].gap_type, GapType::MethodLimitation));
        assert_eq!(gaps[0].evidence_papers.len(), 2);
        assert!((gaps[0].confidence - 0.85).abs() < 0.01);
        assert_eq!(gaps[0].severity, "high");
    }

    #[test]
    fn test_parse_empty_response() {
        let gaps = parse_gaps("", "test");
        assert!(gaps.is_empty());
    }

    #[test]
    fn test_parse_ignores_comments() {
        let response = "# This is a comment\n[method_limitation] test | p1 | 0.5 | medium";
        let gaps = parse_gaps(response, "test");
        assert_eq!(gaps.len(), 1);
    }

    #[test]
    fn test_parse_strips_think_tags() {
        let response = "<think>Some reasoning</think>\n[method_limitation] gap | p1 | 0.9 | high";
        let gaps = parse_gaps(response, "test");
        assert_eq!(gaps.len(), 1);
    }

    #[test]
    fn test_gap_type_from_str() {
        assert!(matches!(GapType::from_str("method_limitation"), GapType::MethodLimitation));
        assert!(matches!(GapType::from_str("unknown"), GapType::MethodLimitation));
        assert!(matches!(GapType::from_str("dataset_gap"), GapType::DatasetGap));
    }

    #[test]
    fn test_gap_type_as_str() {
        assert_eq!(GapType::MethodLimitation.as_str(), "method_limitation");
        assert_eq!(GapType::Contradiction.as_str(), "contradiction");
    }
}
