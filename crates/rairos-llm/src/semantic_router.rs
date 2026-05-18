//! Semantic Router — natural-language query routing with keyword fallback.
//!
//! Mirrors llm/routing/semantic_router.py (keyword-based routing subset)

use serde::{Deserialize, Serialize};

// ─── Query types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QueryType {
    GapAnalysis,
    HypothesisGeneration,
    Experiment,
    Insight,
    Narrative,
    PaperSearch,
    QuestionAnswer,
    General,
}

impl QueryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::GapAnalysis => "gap_analysis",
            Self::HypothesisGeneration => "hypothesis_generation",
            Self::Experiment => "experiment",
            Self::Insight => "insight",
            Self::Narrative => "narrative",
            Self::PaperSearch => "paper_search",
            Self::QuestionAnswer => "question_answer",
            Self::General => "general",
        }
    }

    pub fn command(&self) -> &'static str {
        match self {
            Self::GapAnalysis => "gap",
            Self::HypothesisGeneration => "hypothesize",
            Self::Experiment => "experiment",
            Self::Insight => "insight",
            Self::Narrative => "narrative",
            Self::PaperSearch => "search",
            Self::QuestionAnswer => "ask",
            Self::General => "chat",
        }
    }
}

// ─── Route decision ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDecision {
    pub query_type: String,
    pub confidence: f64,
    pub primary_command: String,
    pub reasoning: String,
    pub verification_warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RouteVerificationResult {
    pub is_valid: bool,
    pub warnings: Vec<String>,
}

impl RouteVerificationResult {
    pub fn valid() -> Self {
        Self { is_valid: true, warnings: Vec::new() }
    }

    pub fn with_warnings(warnings: Vec<String>) -> Self {
        Self { is_valid: warnings.is_empty(), warnings }
    }
}

// ─── Keyword patterns per query type ────────────────────────────────────────

const KEYWORDS: &[(QueryType, &[&str])] = &[
    (QueryType::GapAnalysis, &[
        "gap", "gaps", "空白", "未解决", "missing", "unresolved", "opportunity",
        "差距", "limitation", "limitations", "不足", "untouched", "overlooked",
        "open problem", "open question",
    ]),
    (QueryType::HypothesisGeneration, &[
        "hypothesis", "假设", "hypothesize", "conjecture", "predict", "预测",
        "if-then",
    ]),
    (QueryType::Experiment, &[
        "experiment", "实验", "ab test", "evaluate", "评估", "validate",
        "验证", "trial", "benchmark", "benchmarking",
    ]),
    (QueryType::Insight, &[
        "insight", "insights", "发现", "洞察", "pattern", "patterns",
        "key finding", "takeaway", "synthesis",
    ]),
    (QueryType::Narrative, &[
        "narrative", "story", "线程", "progress", "phase", "跟踪",
        "进展", "状态", "story arc",
    ]),
    (QueryType::PaperSearch, &[
        "paper", "papers", "search", "find", "论文", "搜索", "arxiv",
        "找论文", "文献", "publication",
    ]),
    (QueryType::QuestionAnswer, &[
        "what", "who", "how", "why", "explain", "什么", "如何", "为什么",
        "请问", "回答", "answer", "can you",
    ]),
    (QueryType::General, &[
        "chat", "talk", "discuss", "对话", "聊聊", "tell me", "about",
        "introduction", "介绍",
    ]),
];

/// Route a query by keyword scoring (fast, no LLM needed).
/// Returns the best-matching QueryType with confidence 0.0-1.0.
pub fn route_by_keyword(query: &str) -> RouteDecision {
    let lower = query.to_lowercase();
    let mut best_score = 0.0f64;
    let mut best_qt = &QueryType::General;

    for (qt, keywords) in KEYWORDS {
        let score = keywords.iter().filter(|kw| lower.contains(*kw)).count() as f64;
        if score > best_score {
            best_score = score;
            best_qt = qt;
        }
    }

    let confidence = (best_score / 3.0).min(1.0);
    RouteDecision {
        query_type: best_qt.as_str().to_string(),
        confidence,
        primary_command: best_qt.command().to_string(),
        reasoning: format!("[keyword fallback: score={}]", best_score),
        verification_warnings: Vec::new(),
    }
}

/// Route a query using LLM (requires LlmClient).
pub async fn route_by_llm(
    llm: &dyn crate::LlmClient,
    model: &str,
    query: &str,
) -> RouteDecision {
    let capability_lines: Vec<String> = [
        "gap_analysis: Identify research gaps, unanswered questions, or underexplored areas",
        "hypothesis_generation: Generate testable research hypotheses or conjectures",
        "experiment: Design, run, or track experiments to validate hypotheses",
        "insight: Extract key insights, patterns, or synthesis from research papers",
        "narrative: Track research narrative threads or story arcs",
        "paper_search: Search for papers by keywords or topic",
        "question_answer: Answer a research question from the paper library",
        "general: General research conversation or open-ended discussion",
    ].iter().map(|s| format!("  - {}", s)).collect();

    let capabilities = capability_lines.join("\n");

    let _system_prompt = format!(
        "You are a CLI research-command classifier. Given a user's query, classify it into one type.\n\n\
        Available types:\n{}\n\n\
        Return ONLY valid JSON: {{\"query_type\": \"...\", \"confidence\": 0.0-1.0, \"reasoning\": \"...\"}}",
        capabilities,
    );

    let msg = crate::Message { role: "user".to_string(), content: query.to_string() };

    // Try LLM; fall back to keyword on any error
    let body = match llm.complete(vec![msg], model, 0.2, 500).await {
        Ok(crate::LlmResponse::NonStream(ns)) => ns.content,
        _ => return route_by_keyword(query),
    };

    // Parse JSON response
    let json_start = body.find('{');
    let json_end = body.rfind('}');
    let json_str = match (json_start, json_end) {
        (Some(s), Some(e)) if s < e => &body[s..=e],
        _ => return route_by_keyword(query),
    };

    match serde_json::from_str::<serde_json::Value>(json_str) {
        Ok(val) => {
            let qt_str = val.get("query_type").and_then(|v| v.as_str()).unwrap_or("general");
            let confidence = val.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let reasoning = val.get("reasoning").and_then(|v| v.as_str()).unwrap_or("");
            let qt = match qt_str {
                "gap_analysis" => Some(QueryType::GapAnalysis),
                "hypothesis_generation" => Some(QueryType::HypothesisGeneration),
                "experiment" => Some(QueryType::Experiment),
                "insight" => Some(QueryType::Insight),
                "narrative" => Some(QueryType::Narrative),
                "paper_search" => Some(QueryType::PaperSearch),
                "question_answer" => Some(QueryType::QuestionAnswer),
                "general" => Some(QueryType::General),
                _ => None,
            };
            match qt {
                Some(qt) => {
                    let verification = verify_route_decision(query, qt.as_str(), confidence);
                    RouteDecision {
                        query_type: qt.as_str().to_string(),
                        confidence,
                        primary_command: qt.command().to_string(),
                        reasoning: reasoning.to_string(),
                        verification_warnings: verification.warnings,
                    }
                }
                None => route_by_keyword(query),
            }
        }
        Err(_) => route_by_keyword(query),
    }
}

const VERIFY_ROUTE_PROMPT: &str = r#"你是一个严谨的路由决策验证助手。检查以下路由决策是否合理。

用户查询: {query}
路由类型: {query_type}
置信度: {confidence}

请验证：
1. 路由类型是否与查询匹配？
2. 置信度是否合理？

请以JSON格式返回：
{{"is_valid": true/false, "warnings": ["问题1"]}}

如果决策合理，返回 {{"is_valid": true, "warnings": []}}。
如果有问题，返回 {{"is_valid": false, "warnings": ["具体问题"]}}。"#;

async fn verify_route_decision_async(
    llm: &dyn crate::LlmClient,
    model: &str,
    query: &str,
    query_type: &str,
    confidence: f64,
) -> RouteVerificationResult {
    if confidence > 0.8 {
        return RouteVerificationResult::valid();
    }

    let prompt = VERIFY_ROUTE_PROMPT
        .replace("{query}", &query.chars().take(100).collect::<String>())
        .replace("{query_type}", query_type)
        .replace("{confidence}", &format!("{:.2}", confidence));

    let msg = crate::Message { role: "user".to_string(), content: prompt };

    match llm.complete(vec![msg], model, 0.1, 200).await {
        Ok(crate::LlmResponse::NonStream(ns)) => {
            parse_route_verification_result(&ns.content)
        }
        _ => RouteVerificationResult::valid(),
    }
}

fn verify_route_decision(query: &str, query_type: &str, confidence: f64) -> RouteVerificationResult {
    let mut warnings = Vec::new();

    if confidence < 0.3 {
        warnings.push("置信度较低，可能需要人工确认".to_string());
    }

    let query_lower = query.to_lowercase();
    let type_keywords: &[(&str, &[&str])] = &[
        ("gap_analysis", &["gap", "gaps", "空白", "未解决", "missing", "opportunity", "limitation"]),
        ("hypothesis_generation", &["hypothesis", "假设", "hypothesize", "predict", "预测"]),
        ("experiment", &["experiment", "实验", "ab test", "evaluate", "评估", "validate", "验证"]),
        ("insight", &["insight", "insights", "发现", "洞察", "pattern", "key finding"]),
        ("narrative", &["narrative", "story", "线程", "progress", "跟踪", "进展"]),
        ("paper_search", &["paper", "papers", "search", "论文", "搜索", "arxiv", "文献"]),
        ("question_answer", &["what", "who", "how", "why", "explain", "什么", "如何", "为什么", "请问"]),
        ("general", &["chat", "talk", "discuss", "对话", "聊聊", "tell me", "介绍"]),
    ];

    if let Some((_, keywords)) = type_keywords.iter().find(|(t, _)| *t == query_type) {
        let match_count = keywords.iter().filter(|kw| query_lower.contains(*kw)).count();
        if match_count == 0 && confidence < 0.5 {
            warnings.push(format!("查询类型 '{}' 与关键词不匹配", query_type));
        }
    }

    RouteVerificationResult::with_warnings(warnings)
}

fn parse_route_verification_result(content: &str) -> RouteVerificationResult {
    let content = content.trim();

    let _is_valid = if content.contains("\"is_valid\": true") || content.contains("\"is_valid\":true") {
        true
    } else if content.contains("\"is_valid\": false") || content.contains("\"is_valid\":false") {
        false
    } else {
        return RouteVerificationResult::valid();
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

    RouteVerificationResult::with_warnings(warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_gap() {
        let r = route_by_keyword("What are the research gaps in transformer efficiency?");
        assert_eq!(r.query_type, "gap_analysis");
        assert!(r.confidence > 0.0);
    }

    #[test]
    fn test_keyword_paper_search() {
        let r = route_by_keyword("Find papers about diffusion models");
        assert_eq!(r.query_type, "paper_search");
    }

    #[test]
    fn test_keyword_general() {
        let r = route_by_keyword("Good morning, nice to meet you");
        assert_eq!(r.query_type, "general");
    }

    #[test]
    fn test_keyword_question() {
        let r = route_by_keyword("What is the meaning of life?");
        assert_eq!(r.query_type, "question_answer");
    }

    #[test]
    fn test_confidence_scaling() {
        let r = route_by_keyword("gap gaps missing unresolved opportunity");
        assert!(r.confidence > 0.5, "high keyword count should boost confidence");
    }

    #[test]
    fn test_multilingual_chinese() {
        let r = route_by_keyword("论文搜索 transformer 注意力机制");
        assert_eq!(r.query_type, "paper_search");
    }

    #[test]
    fn test_query_type_as_str() {
        assert_eq!(QueryType::GapAnalysis.as_str(), "gap_analysis");
        assert_eq!(QueryType::General.as_str(), "general");
    }

    #[test]
    fn test_query_type_command() {
        assert_eq!(QueryType::GapAnalysis.command(), "gap");
        assert_eq!(QueryType::General.command(), "chat");
    }

    #[test]
    fn test_route_decision_has_verification_warnings() {
        let r = route_by_keyword("What are the research gaps in transformer efficiency?");
        assert!(r.verification_warnings.is_empty());
    }

    #[test]
    fn test_verify_route_decision_low_confidence() {
        let result = verify_route_decision("some random text", "gap_analysis", 0.2);
        assert!(!result.is_valid);
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn test_verify_route_decision_high_confidence() {
        let result = verify_route_decision("gap gaps missing opportunity", "gap_analysis", 0.9);
        assert!(result.is_valid);
    }

    #[test]
    fn test_route_verification_result_valid() {
        let result = RouteVerificationResult::valid();
        assert!(result.is_valid);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_route_verification_result_with_warnings() {
        let warnings = vec!["置信度过低".to_string()];
        let result = RouteVerificationResult::with_warnings(warnings);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_parse_route_verification_result_valid() {
        let json = r#"{"is_valid": true, "warnings": []}"#;
        let result = parse_route_verification_result(json);
        assert!(result.is_valid);
    }

    #[test]
    fn test_parse_route_verification_result_invalid() {
        let json = r#"{"is_valid": false, "warnings": ["查询类型不匹配"]}"#;
        let result = parse_route_verification_result(json);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_parse_route_verification_result_malformed() {
        let json = "not json";
        let result = parse_route_verification_result(json);
        assert!(result.is_valid);
    }
}
