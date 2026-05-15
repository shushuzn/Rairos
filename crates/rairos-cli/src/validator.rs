//! rairos-validator — Research question novelty validator.
//!
//! Ported from `llm/question_validator.py` + `cli/cmd/validate.py`.
//! Pure in-memory rule engine — no persistence, no file I/O.

use std::collections::HashSet;

// ═══════════════════════════════════════════════════════════════════════════
// Types
// ═══════════════════════════════════════════════════════════════════════════

/// Novelty level for a research question.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NoveltyLevel {
    High,
    Medium,
    Low,
    Unknown,
}

impl NoveltyLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            NoveltyLevel::High => "high",
            NoveltyLevel::Medium => "medium",
            NoveltyLevel::Low => "low",
            NoveltyLevel::Unknown => "unknown",
        }
    }
}

/// Dimensions of research innovation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InnovationDimension {
    Method,
    Task,
    Evaluation,
    Theory,
    Application,
}

/// Innovation score for a research question.
#[derive(Debug, Clone)]
pub struct InnovationScore {
    pub overall: f64,
    pub method: f64,
    pub task: f64,
    pub evaluation: f64,
    pub dimensions: Vec<InnovationDimension>,
    pub reasoning: String,
}

/// A related paper found via keyword search.
#[derive(Debug, Clone)]
pub struct RelatedWork {
    pub paper_id: String,
    pub title: String,
    pub year: i32,
    pub relevance_score: f64,
}

/// Full validation result for a research question.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub question: String,
    pub is_novel: bool,
    pub novelty_level: NoveltyLevel,
    pub innovation_score: InnovationScore,
    pub related_works: Vec<RelatedWork>,
    pub gap_summary: String,
    pub suggestions: Vec<String>,
    pub confidence: f64,
}

// ═══════════════════════════════════════════════════════════════════════════
// Question keyword expansion
// ═══════════════════════════════════════════════════════════════════════════

/// AI research keywords matched to Python's `AI_RESEARCH_KEYWORDS`.
pub fn default_ai_keywords() -> HashSet<&'static str> {
    [
        // Core NLP/LLM
        "transformer", "attention", "bert", "gpt", "llm", "language model",
        "neural", "network", "embedding", "fine-tuning", "rlhf", "rag",
        "retrieval", "generative", "diffusion", "gan", "clip", "vit",
        // RL
        "reinforcement", "policy", "reward", "rl", "dpo", "ppo", "reward model",
        // Training
        "training", "optimization", "pre-training", "instruction", "alignment",
        // Multimodal
        "multimodal", "vision", "language", "speech", "audio",
        // Reasoning
        "constitutional", "reasoning", "chain-of-thought", "cot", "synthetic data",
        // Generic
        "model", "learning",
    ]
    .into()
}

/// Stop words removed during question expansion.
pub fn stop_words() -> HashSet<&'static str> {
    [
        "can", "how", "what", "why", "is", "does", "to", "the", "a", "an",
        "do", "are", "was", "were", "be", "been", "being", "have", "has", "had",
        "will", "would", "could", "should", "may", "might", "must", "shall",
        "of", "in", "for", "on", "with", "at", "by", "from", "as", "into",
        "through", "during", "before", "after", "above", "below", "between",
        "under", "again", "further", "then", "once", "here", "there", "when",
        "where", "which", "who", "whom", "this", "that", "these", "those",
        "all", "each", "every", "both", "few", "more", "most", "other", "some",
        "such", "no", "nor", "not", "only", "own", "same", "so", "than", "too",
        "very", "just", "but", "and", "or", "if", "because", "until", "while",
    ]
    .into()
}

/// Expand a research question into searchable keywords.
///
/// 1. Lowercase, remove punctuation
/// 2. Remove stop words, keep words > 2 chars
/// 3. Add matching AI research keywords
/// 4. Deduplicate, max 10
pub fn expand_question(question: &str, ai_keywords: &HashSet<&str>) -> Vec<String> {
    let q_lower = question.to_lowercase();
    let stops = stop_words();

    // Split by whitespace, clean punctuation
    let words: Vec<&str> = q_lower
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|w| w.len() > 2 && !stops.contains(w))
        .collect();

    let mut result: HashSet<String> = words.into_iter().map(|w| w.to_string()).collect();

    // Add matching AI research keywords (multi-word checked separately)
    let lowered = q_lower.clone();
    for keyword in ai_keywords {
        if lowered.contains(keyword) {
            result.insert(keyword.to_string());
        }
    }

    let mut final_list: Vec<String> = result.into_iter().collect();
    final_list.sort();
    final_list.truncate(10);
    final_list
}

// ═══════════════════════════════════════════════════════════════════════════
// Rule-based innovation analysis
// ═══════════════════════════════════════════════════════════════════════════

/// Rule-based innovation analysis. Matches Python's `_analyze_innovation_rules`.
///
/// Three tiers:
/// - No related works → high novelty (overall 8.0)
/// - max relevance > 0.8 → low novelty (overall 3.0)
/// - max relevance > 0.5 → medium (overall 6.0)
/// - else → medium-high (overall 7.5)
pub fn analyze_innovation_rules(related: &[RelatedWork]) -> InnovationScore {
    if related.is_empty() {
        return InnovationScore {
            overall: 8.0,
            method: 7.0,
            task: 8.0,
            evaluation: 7.0,
            dimensions: vec![
                InnovationDimension::Method,
                InnovationDimension::Task,
                InnovationDimension::Evaluation,
            ],
            reasoning: "未发现相关工作，可能是全新领域".into(),
        };
    }

    let max_relevance = related
        .iter()
        .map(|r| r.relevance_score)
        .fold(0.0_f64, f64::max);

    if max_relevance > 0.8 {
        InnovationScore {
            overall: 3.0,
            method: 3.0,
            task: 4.0,
            evaluation: 3.0,
            dimensions: vec![],
            reasoning: format!("发现高度相关工作 (相似度 {:.0}%)", max_relevance * 100.0),
        }
    } else if max_relevance > 0.5 {
        InnovationScore {
            overall: 6.0,
            method: 6.0,
            task: 5.0,
            evaluation: 6.0,
            dimensions: vec![InnovationDimension::Method],
            reasoning: format!("有相关工作，但有新角度 (相似度 {:.0}%)", max_relevance * 100.0),
        }
    } else {
        InnovationScore {
            overall: 7.5,
            method: 7.0,
            task: 8.0,
            evaluation: 7.0,
            dimensions: vec![
                InnovationDimension::Task,
                InnovationDimension::Application,
            ],
            reasoning: "发现部分相关，但领域/应用不同".into(),
        }
    }
}

/// Determine overall novelty level. Matches Python's `_determine_novelty`.
pub fn determine_novelty(innovation: &InnovationScore, related: &[RelatedWork]) -> NoveltyLevel {
    if related.is_empty() && innovation.overall >= 7.0 {
        return NoveltyLevel::High;
    }
    if innovation.overall >= 7.0 {
        NoveltyLevel::High
    } else if innovation.overall >= 5.0 {
        NoveltyLevel::Medium
    } else {
        NoveltyLevel::Low
    }
}

/// Calculate confidence of the validation. Matches Python's `_calculate_confidence`.
pub fn calculate_confidence(related: &[RelatedWork], innovation: &InnovationScore) -> f64 {
    let related_score = (related.len() as f64 / 5.0).min(1.0) * 0.4;
    let reasoning_score = if innovation.reasoning.is_empty() {
        0.15
    } else {
        0.3
    };
    let dimension_score = (innovation.dimensions.len() as f64 / 3.0).min(1.0) * 0.3;
    (related_score + reasoning_score + dimension_score).min(0.95)
}

/// Generate rule-based improvement suggestions. Matches Python's `_generate_suggestions_rules`.
pub fn generate_suggestions_rules(related: &[RelatedWork]) -> Vec<String> {
    if related.is_empty() {
        vec![
            "[方法] 设计全新的方法框架".into(),
            "[任务] 探索具体的落地场景".into(),
            "[评估] 建立评估基准和指标".into(),
        ]
    } else {
        let recent_count = related.iter().filter(|r| r.year >= 2023).count();
        let mut suggestions = Vec::new();
        if recent_count > 0 {
            suggestions.push(format!(
                "[方法] 参考 {} 篇最新工作，选择差异化路线",
                recent_count
            ));
        }
        suggestions.push("[任务] 考虑跨领域应用场景".into());
        suggestions.push("[评估] 设计针对新问题的评估指标".into());
        suggestions.push("[数据] 构建专用数据集".into());
        suggestions
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Rendering
// ═══════════════════════════════════════════════════════════════════════════

/// Render validation result as formatted text. Matches Python's `render_result`.
pub fn render_result(result: &ValidationResult) -> String {
    let novelty_icon = match result.novelty_level {
        NoveltyLevel::High => "🟢",
        NoveltyLevel::Medium => "🟡",
        NoveltyLevel::Low => "🔴",
        NoveltyLevel::Unknown => "⚪",
    };

    let q_display = if result.question.len() > 60 {
        format!("{}...", &result.question[..60])
    } else {
        result.question.clone()
    };

    let mut lines = vec![
        format!("🔬 研究问题验证: \"{}\"", q_display),
        String::new(),
        format!(
            "{} 创新指数: {:.1}/10",
            novelty_icon, result.innovation_score.overall
        ),
        format!("   方法创新: {:.0}/10", result.innovation_score.method),
        format!("   任务创新: {:.0}/10", result.innovation_score.task),
        format!("   评估创新: {:.0}/10", result.innovation_score.evaluation),
        String::new(),
    ];

    if !result.innovation_score.dimensions.is_empty() {
        let dim_strs: Vec<&str> = result
            .innovation_score
            .dimensions
            .iter()
            .map(|d| match d {
                InnovationDimension::Method => "method",
                InnovationDimension::Task => "task",
                InnovationDimension::Evaluation => "evaluation",
                InnovationDimension::Theory => "theory",
                InnovationDimension::Application => "application",
            })
            .collect();
        lines.push(format!("   亮点维度: {}", dim_strs.join(", ")));
    }

    if !result.innovation_score.reasoning.is_empty() {
        lines.push(format!("   分析: {}", result.innovation_score.reasoning));
    }

    lines.push(String::new());

    if !result.related_works.is_empty() {
        lines.push("📚 相关工作:".into());
        for (i, work) in result.related_works.iter().enumerate().take(3) {
            lines.push(format!(
                "   {}. {} ({})",
                i + 1,
                work.title,
                work.year
            ));
            lines.push(format!("      相关度: {:.0}%", work.relevance_score * 100.0));
        }
        lines.push(String::new());
    }

    if !result.suggestions.is_empty() {
        lines.push("💡 改进建议:".into());
        for s in result.suggestions.iter().take(4) {
            lines.push(format!("   • {}", s));
        }
        lines.push(String::new());
    }

    lines.push(format!("📊 置信度: {:.0}%", result.confidence * 100.0));
    if result.is_novel {
        lines.push("🎯 结论: ✅ 值得探索".into());
    } else {
        lines.push("🎯 结论: ⚠️ 需要更细致的角度".into());
    }

    lines.join("\n")
}

// ═══════════════════════════════════════════════════════════════════════════
// Higher-level orchestration
// ═══════════════════════════════════════════════════════════════════════════

/// Run the full rule-based validation pipeline.
///
/// This is the main entry point for rule-only mode (`--no-llm`).
/// Takes a question, the found related works, and returns a complete result.
pub fn validate_rules(question: &str, related_works: Vec<RelatedWork>) -> ValidationResult {
    let keywords = expand_question(question, &default_ai_keywords());
    let gap_summary = format!(
        "关键词: {}",
        keywords
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<&str>>()
            .join(", ")
    );

    let innovation = analyze_innovation_rules(&related_works);
    let novelty = determine_novelty(&innovation, &related_works);
    let confidence = calculate_confidence(&related_works, &innovation);
    let suggestions = generate_suggestions_rules(&related_works);

    ValidationResult {
        question: question.to_string(),
        is_novel: novelty != NoveltyLevel::Low,
        novelty_level: novelty,
        innovation_score: innovation,
        related_works,
        gap_summary,
        suggestions,
        confidence,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_question_basic() {
        let keywords = default_ai_keywords();
        let result = expand_question("Can transformers learn to reason?", &keywords);
        assert!(result.contains(&"transformer".to_string()));
        assert!(result.contains(&"reason".to_string()));
        assert!(!result.contains(&"can".to_string())); // stop word
        assert!(result.len() <= 10);
    }

    #[test]
    fn test_expand_question_ai_keyword_matching() {
        let keywords = default_ai_keywords();
        let result = expand_question("How does RAG improve LLM reasoning?", &keywords);
        assert!(result.contains(&"rag".to_string()));
        assert!(result.contains(&"llm".to_string()));
        assert!(result.contains(&"reasoning".to_string()));
    }

    // ── Innovation rules ────────────────────────────────────────────────

    #[test]
    fn test_analyze_empty_related_high_novelty() {
        let score = analyze_innovation_rules(&[]);
        assert!((score.overall - 8.0).abs() < 0.01);
        assert_eq!(score.dimensions.len(), 3);
    }

    #[test]
    fn test_analyze_high_relevance_low_novelty() {
        let related = vec![RelatedWork {
            paper_id: "p1".into(),
            title: "Very similar work".into(),
            year: 2024,
            relevance_score: 0.9,
        }];
        let score = analyze_innovation_rules(&related);
        assert!((score.overall - 3.0).abs() < 0.01);
        assert!(score.dimensions.is_empty());
    }

    #[test]
    fn test_analyze_medium_relevance() {
        let related = vec![RelatedWork {
            paper_id: "p2".into(),
            title: "Somewhat related".into(),
            year: 2023,
            relevance_score: 0.6,
        }];
        let score = analyze_innovation_rules(&related);
        assert!((score.overall - 6.0).abs() < 0.01);
        assert_eq!(score.dimensions.len(), 1);
        assert_eq!(score.dimensions[0], InnovationDimension::Method);
    }

    #[test]
    fn test_analyze_low_relevance() {
        let related = vec![RelatedWork {
            paper_id: "p3".into(),
            title: "Distant work".into(),
            year: 2022,
            relevance_score: 0.3,
        }];
        let score = analyze_innovation_rules(&related);
        assert!((score.overall - 7.5).abs() < 0.01);
        assert_eq!(score.dimensions.len(), 2);
    }

    // ── Novelty determination ───────────────────────────────────────────

    #[test]
    fn test_determine_novelty_high() {
        let innovation = InnovationScore {
            overall: 8.0,
            method: 7.0,
            task: 8.0,
            evaluation: 7.0,
            dimensions: vec![],
            reasoning: "test".into(),
        };
        assert_eq!(determine_novelty(&innovation, &[]), NoveltyLevel::High);
    }

    #[test]
    fn test_determine_novelty_medium() {
        let innovation = InnovationScore {
            overall: 6.0,
            method: 6.0,
            task: 5.0,
            evaluation: 6.0,
            dimensions: vec![],
            reasoning: "test".into(),
        };
        assert_eq!(determine_novelty(&innovation, &[]), NoveltyLevel::Medium);
    }

    #[test]
    fn test_determine_novelty_low() {
        let innovation = InnovationScore {
            overall: 3.0,
            method: 3.0,
            task: 4.0,
            evaluation: 3.0,
            dimensions: vec![],
            reasoning: "test".into(),
        };
        assert_eq!(determine_novelty(&innovation, &[]), NoveltyLevel::Low);
    }

    // ── Confidence ──────────────────────────────────────────────────────

    #[test]
    fn test_confidence_empty_related() {
        let innovation = InnovationScore {
            overall: 8.0,
            method: 7.0,
            task: 8.0,
            evaluation: 7.0,
            dimensions: vec![
                InnovationDimension::Method,
                InnovationDimension::Task,
                InnovationDimension::Evaluation,
            ],
            reasoning: "测试".into(),
        };
        let confidence = calculate_confidence(&[], &innovation);
        // related=0→0 + reasoning=0.3 + dims=3/3*0.3=0.3 = 0.6
        assert!((confidence - 0.6).abs() < 0.01);
    }

    #[test]
    fn test_confidence_with_related_works() {
        let related = vec![
            RelatedWork { paper_id: "p1".into(), title: "A".into(), year: 2024, relevance_score: 0.5 },
            RelatedWork { paper_id: "p2".into(), title: "B".into(), year: 2024, relevance_score: 0.3 },
        ];
        let innovation = InnovationScore {
            overall: 6.0,
            method: 6.0,
            task: 5.0,
            evaluation: 6.0,
            dimensions: vec![InnovationDimension::Method],
            reasoning: "test".into(),
        };
        let confidence = calculate_confidence(&related, &innovation);
        // related=2/5*0.4=0.16 + reasoning=0.3 + dims=1/3*0.3=0.1 = 0.56
        assert!((confidence - 0.56).abs() < 0.01);
    }

    // ── Suggestions ─────────────────────────────────────────────────────

    #[test]
    fn test_suggestions_no_related() {
        let suggestions = generate_suggestions_rules(&[]);
        assert_eq!(suggestions.len(), 3);
        assert!(suggestions[0].contains("方法"));
    }

    #[test]
    fn test_suggestions_with_related() {
        let related = vec![RelatedWork {
            paper_id: "p1".into(),
            title: "Recent work".into(),
            year: 2024,
            relevance_score: 0.5,
        }];
        let suggestions = generate_suggestions_rules(&related);
        assert_eq!(suggestions.len(), 4);
        assert!(suggestions[0].contains("参考"));
    }

    // ── Full pipeline ───────────────────────────────────────────────────

    #[test]
    fn test_validate_rules_empty_related() {
        let result = validate_rules("Can we improve LLM reasoning?", vec![]);
        assert!(result.is_novel);
        assert_eq!(result.novelty_level, NoveltyLevel::High);
        assert!((result.innovation_score.overall - 8.0).abs() < 0.01);
        assert!(!result.gap_summary.is_empty());
    }

    #[test]
    fn test_validate_rules_with_related() {
        let related = vec![RelatedWork {
            paper_id: "arxiv:2301.00001".into(),
            title: "Chain-of-Thought Prompting Elicits Reasoning in Large Language Models".into(),
            year: 2023,
            relevance_score: 0.95,
        }];
        let result = validate_rules("How can CoT improve LLM math reasoning?", related);
        assert!(!result.is_novel);
        assert_eq!(result.novelty_level, NoveltyLevel::Low);
    }

    // ── Render ──────────────────────────────────────────────────────────

    #[test]
    fn test_render_result_contains_key_sections() {
        let result = validate_rules("Test question", vec![]);
        let output = render_result(&result);
        assert!(output.contains("创新指数"));
        assert!(output.contains("置信度"));
        assert!(output.contains("值得探索"));
    }
}
