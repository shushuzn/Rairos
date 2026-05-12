//! rairos-query-types — Query type classification and RAG data structures.
//!
//! Ported from `llm/query_types.py`.
//!
//! Provides query type classification for adaptive routing and RAG data structures.

use std::collections::HashMap;

// ============================================================================
// Query Type Classification
// ============================================================================

/// Query type classification for adaptive routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryType {
    /// Who, when, what (exact facts)
    Factual,
    /// Explain, how, why (understanding)
    Conceptual,
    /// vs, compared, difference (analysis)
    Comparative,
    /// recent, latest, 2024, new (time-sensitive)
    Temporal,
    /// Default fallback
    General,
}

impl QueryType {
    /// Return the BM25 weight for this query type.
    ///
    /// Semantic weight = 1 - BM25 weight.
    pub fn bm25_weight(&self) -> f64 {
        match self {
            QueryType::Factual => 0.65,
            QueryType::Conceptual => 0.20,
            QueryType::Comparative => 0.50,
            QueryType::Temporal => 0.55,
            QueryType::General => 0.40,
        }
    }

    /// Return the semantic weight for this query type.
    ///
    /// Semantic weight = 1 - BM25 weight.
    pub fn semantic_weight(&self) -> f64 {
        1.0 - self.bm25_weight()
    }

    /// Return the MMR lambda for this query type.
    ///
    /// - 0.8: relevance-biased
    /// - 0.6: balanced (conceptual, general)
    /// - 0.5: diversity-biased
    /// - 0.7: temporal
    pub fn mmr_lambda(&self) -> f64 {
        match self {
            QueryType::Factual => 0.8,
            QueryType::Conceptual => 0.6,
            QueryType::Comparative => 0.5,
            QueryType::Temporal => 0.7,
            QueryType::General => 0.6,
        }
    }
}

/// Mapping from query type to BM25 weight.
pub fn query_weights() -> HashMap<QueryType, f64> {
    HashMap::from([
        (QueryType::Factual, 0.65),
        (QueryType::Conceptual, 0.20),
        (QueryType::Comparative, 0.50),
        (QueryType::Temporal, 0.55),
        (QueryType::General, 0.40),
    ])
}

/// Mapping from query type to MMR lambda.
pub fn mmr_lambdas() -> HashMap<QueryType, f64> {
    HashMap::from([
        (QueryType::Factual, 0.8),
        (QueryType::Conceptual, 0.6),
        (QueryType::Comparative, 0.5),
        (QueryType::Temporal, 0.7),
        (QueryType::General, 0.6),
    ])
}

// ============================================================================
// Prompts
// ============================================================================

/// Cross-paper analysis system prompt (Chinese).
pub const CROSS_PAPER_SYSTEM_PROMPT: &str = "你是一个研究综述助手，擅长发现论文之间的关联。

分析多篇论文，找出：
1. 共同点 (connection): 讨论相似主题或互补方法
2. 对比 (comparison): 同一问题的不同解决方法
3. 矛盾 (contradiction): 结论或方法冲突
4. 演进 (evolution): 后人如何在前人基础上改进

输出格式（最多3个洞察）：
- 类型: 一句话总结 [论文1] [论文2]
例如：
- comparison: BERT vs GPT的预训练目标不同 [BERT] [GPT-2]
- evolution: LoRA基于Adapter思想提出低秩更新 [Adapter] [LoRA]";

/// Cross-paper analysis user prompt template.
pub const CROSS_PAPER_USER_PROMPT_TEMPLATE: &str = "请分析以下论文之间的关联：

{context_text}

找出最重要的关联（最多3个）：";

/// RAG system prompt (Chinese).
pub const RAG_SYSTEM_PROMPT: &str = "你是一个严谨的 AI 研究助手，精通论文阅读和学术分析。

核心原则：
1. 基于原文回答，不要捏造或推测未提及的内容
2. 不确定的信息必须加 [推测] 标注
3. 使用 > 块引用格式引用原文片段
4. 区分「原文明确说」和「可推断」
5. 回答使用中文，但引用原文时保留英文原句

输出格式：
- 开头总结回答要点（1-2句话）
- 详细解释部分引用原文片段
- 结尾标注信息来源";

// ============================================================================
// Data Structures
// ============================================================================

/// A citation extracted from a paper with source tracing.
#[derive(Debug, Clone)]
pub struct Citation {
    pub paper_id: String,
    pub paper_title: String,
    pub authors: Vec<String>,
    pub published: String,
    pub snippet: String,
    pub relevance_score: f64,
    /// 论文章节 (abstract, intro, method, etc.)
    pub section: String,
    /// 在原文中的起始位置
    pub char_start: usize,
    /// 在原文中的结束位置
    pub char_end: usize,
    /// 精确引用语句
    pub quote: String,
}

impl Default for Citation {
    fn default() -> Self {
        Self {
            paper_id: String::new(),
            paper_title: String::new(),
            authors: Vec::new(),
            published: String::new(),
            snippet: String::new(),
            relevance_score: 0.0,
            section: String::new(),
            char_start: 0,
            char_end: 0,
            quote: String::new(),
        }
    }
}

/// A retrieved context from a paper.
#[derive(Debug, Clone)]
pub struct ChatContext {
    pub paper_id: String,
    pub paper_title: String,
    pub authors: Vec<String>,
    pub published: String,
    pub snippet: String,
    pub relevance_score: f64,
}

/// Confidence score for RAG answer quality.
#[derive(Debug, Clone)]
pub struct ConfidenceScore {
    /// 0-100 置信度
    pub score: f64,
    /// 引用的论文数
    pub papers_count: usize,
    /// 覆盖描述 (e.g., "3篇论文，覆盖Method章节")
    pub coverage: String,
    /// 低置信度警告
    pub warnings: Vec<String>,
    /// 主要来源章节
    pub sources: Vec<String>,
}

impl ConfidenceScore {
    /// Return confidence level label.
    pub fn level(&self) -> &'static str {
        if self.score >= 80.0 {
            "高"
        } else if self.score >= 50.0 {
            "中"
        } else {
            "低"
        }
    }
}

impl Default for ConfidenceScore {
    fn default() -> Self {
        Self {
            score: 0.0,
            papers_count: 0,
            coverage: String::new(),
            warnings: Vec::new(),
            sources: Vec::new(),
        }
    }
}

/// Cross-paper synthesis insight.
#[derive(Debug, Clone)]
pub struct CrossPaperInsight {
    /// "comparison", "connection", "contradiction", "evolution"
    pub insight_type: String,
    /// 一句话总结
    pub summary: String,
    /// 涉及的论文
    pub papers: Vec<String>,
    /// 详细说明
    pub detail: String,
}

impl Default for CrossPaperInsight {
    fn default() -> Self {
        Self {
            insight_type: String::new(),
            summary: String::new(),
            papers: Vec::new(),
            detail: String::new(),
        }
    }
}

/// Result of a RAG chat interaction.
#[derive(Debug, Clone)]
pub struct ChatResult {
    pub answer: String,
    pub citations: Vec<Citation>,
    pub papers_used: Vec<String>,
    /// 会话ID for continuity
    pub session_id: Option<String>,
    /// 解析的上下文信息
    pub resolved_context: Option<HashMap<String, String>>,
    /// 智能追问建议
    pub probing_questions: Vec<String>,
    /// 答案可信度评分
    pub confidence: Option<ConfidenceScore>,
    /// 跨论文洞察
    pub cross_paper_insights: Vec<CrossPaperInsight>,
}

impl Default for ChatResult {
    fn default() -> Self {
        Self {
            answer: String::new(),
            citations: Vec::new(),
            papers_used: Vec::new(),
            session_id: None,
            resolved_context: None,
            probing_questions: Vec::new(),
            confidence: None,
            cross_paper_insights: Vec::new(),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_type_bm25_weights() {
        assert!((QueryType::Factual.bm25_weight() - 0.65).abs() < f64::EPSILON);
        assert!((QueryType::Conceptual.bm25_weight() - 0.20).abs() < f64::EPSILON);
        assert!((QueryType::Comparative.bm25_weight() - 0.50).abs() < f64::EPSILON);
        assert!((QueryType::Temporal.bm25_weight() - 0.55).abs() < f64::EPSILON);
        assert!((QueryType::General.bm25_weight() - 0.40).abs() < f64::EPSILON);
    }

    #[test]
    fn test_query_type_semantic_weights() {
        assert!((QueryType::Factual.semantic_weight() - 0.35).abs() < f64::EPSILON);
        assert!((QueryType::Conceptual.semantic_weight() - 0.80).abs() < f64::EPSILON);
    }

    #[test]
    fn test_query_type_mmr_lambdas() {
        assert!((QueryType::Factual.mmr_lambda() - 0.8).abs() < f64::EPSILON);
        assert!((QueryType::Conceptual.mmr_lambda() - 0.6).abs() < f64::EPSILON);
        assert!((QueryType::Comparative.mmr_lambda() - 0.5).abs() < f64::EPSILON);
        assert!((QueryType::Temporal.mmr_lambda() - 0.7).abs() < f64::EPSILON);
        assert!((QueryType::General.mmr_lambda() - 0.6).abs() < f64::EPSILON);
    }

    #[test]
    fn test_confidence_level() {
        let high = ConfidenceScore {
            score: 85.0,
            papers_count: 3,
            coverage: "3篇论文".to_string(),
            warnings: vec![],
            sources: vec![],
        };
        assert_eq!(high.level(), "高");

        let medium = ConfidenceScore {
            score: 60.0,
            papers_count: 2,
            coverage: "2篇论文".to_string(),
            warnings: vec![],
            sources: vec![],
        };
        assert_eq!(medium.level(), "中");

        let low = ConfidenceScore {
            score: 30.0,
            papers_count: 1,
            coverage: "1篇论文".to_string(),
            warnings: vec!["证据不足".to_string()],
            sources: vec![],
        };
        assert_eq!(low.level(), "低");
    }

    #[test]
    fn test_citation_defaults() {
        let citation = Citation::default();
        assert_eq!(citation.section, "");
        assert_eq!(citation.quote, "");
    }

    #[test]
    fn test_chat_result_defaults() {
        let result = ChatResult::default();
        assert!(result.session_id.is_none());
        assert!(result.confidence.is_none());
        assert!(result.answer.is_empty());
    }

    #[test]
    fn test_cross_paper_insight_defaults() {
        let insight = CrossPaperInsight::default();
        assert_eq!(insight.detail, "");
        assert!(insight.papers.is_empty());
    }

    #[test]
    fn test_prompts_not_empty() {
        assert!(!CROSS_PAPER_SYSTEM_PROMPT.is_empty());
        assert!(!CROSS_PAPER_USER_PROMPT_TEMPLATE.is_empty());
        assert!(!RAG_SYSTEM_PROMPT.is_empty());
    }

    #[test]
    fn test_query_weights_map() {
        let weights = query_weights();
        assert_eq!(weights.len(), 5);
        assert!((weights[&QueryType::Factual] - 0.65).abs() < f64::EPSILON);
    }

    #[test]
    fn test_mmr_lambdas_map() {
        let lambdas = mmr_lambdas();
        assert_eq!(lambdas.len(), 5);
        assert!((lambdas[&QueryType::Factual] - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn test_query_type_equality() {
        assert_eq!(QueryType::Factual, QueryType::Factual);
        assert_ne!(QueryType::Factual, QueryType::Conceptual);
    }

    #[test]
    fn test_query_type_copy() {
        let qt = QueryType::Factual;
        let qt2 = qt;
        assert_eq!(qt, qt2);
    }
}
