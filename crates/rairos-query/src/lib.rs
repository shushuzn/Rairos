//! rairos-query — Query type classification and RAG data structures.
//!
//! Ported from `llm/query_types.py`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum QueryType {
    Factual,
    Conceptual,
    Comparative,
    Temporal,
    #[default]
    General,
}

impl QueryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            QueryType::Factual => "factual",
            QueryType::Conceptual => "conceptual",
            QueryType::Comparative => "comparative",
            QueryType::Temporal => "temporal",
            QueryType::General => "general",
        }
    }

    pub fn bm25_weight(&self) -> f64 {
        match self {
            QueryType::Factual => 0.65,
            QueryType::Conceptual => 0.20,
            QueryType::Comparative => 0.50,
            QueryType::Temporal => 0.55,
            QueryType::General => 0.40,
        }
    }

    pub fn semantic_weight(&self) -> f64 {
        1.0 - self.bm25_weight()
    }

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

impl std::fmt::Display for QueryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub paper_id: String,
    pub paper_title: String,
    pub authors: Vec<String>,
    pub published: String,
    pub snippet: String,
    pub relevance_score: f64,
    #[serde(default)]
    pub section: String,
    #[serde(default)]
    pub char_start: i32,
    #[serde(default)]
    pub char_end: i32,
    #[serde(default)]
    pub quote: String,
}

impl Citation {
    pub fn new(
        paper_id: &str,
        paper_title: &str,
        authors: Vec<String>,
        published: &str,
        snippet: &str,
        relevance_score: f64,
    ) -> Self {
        Self {
            paper_id: paper_id.to_string(),
            paper_title: paper_title.to_string(),
            authors,
            published: published.to_string(),
            snippet: snippet.to_string(),
            relevance_score,
            section: String::new(),
            char_start: 0,
            char_end: 0,
            quote: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatContext {
    pub paper_id: String,
    pub paper_title: String,
    pub authors: Vec<String>,
    pub published: String,
    pub snippet: String,
    pub relevance_score: f64,
}

impl ChatContext {
    pub fn new(
        paper_id: &str,
        paper_title: &str,
        authors: Vec<String>,
        published: &str,
        snippet: &str,
        relevance_score: f64,
    ) -> Self {
        Self {
            paper_id: paper_id.to_string(),
            paper_title: paper_title.to_string(),
            authors,
            published: published.to_string(),
            snippet: snippet.to_string(),
            relevance_score,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceScore {
    pub score: f64,
    pub papers_count: i32,
    pub coverage: String,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub sources: Vec<String>,
}

impl ConfidenceScore {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossPaperInsight {
    pub insight_type: String,
    pub summary: String,
    pub papers: Vec<String>,
    #[serde(default)]
    pub detail: String,
}

impl CrossPaperInsight {
    pub fn new(insight_type: &str, summary: &str, papers: Vec<String>) -> Self {
        Self {
            insight_type: insight_type.to_string(),
            summary: summary.to_string(),
            papers,
            detail: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResult {
    pub answer: String,
    #[serde(default)]
    pub citations: Vec<Citation>,
    #[serde(default)]
    pub papers_used: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_context: Option<serde_json::Value>,
    #[serde(default)]
    pub probing_questions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<ConfidenceScore>,
    #[serde(default)]
    pub cross_paper_insights: Vec<CrossPaperInsight>,
}

impl ChatResult {
    pub fn new(answer: &str) -> Self {
        Self {
            answer: answer.to_string(),
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

pub const CROSS_PAPER_SYSTEM_PROMPT: &str = "你是一个研究综述助手，擅长发现论文之间的关联。\n\n分析多篇论文，找出：\n1. 共同点 (connection): 讨论相似主题或互补方法\n2. 对比 (comparison): 同一问题的不同解决方法\n3. 矛盾 (contradiction): 结论或方法冲突\n4. 演进 (evolution): 后人如何在前人基础上改进\n\n输出格式（最多3个洞察）：\n- 类型: 一句话总结 [论文1] [论文2]\n例如：\n- comparison: BERT vs GPT的预训练目标不同 [BERT] [GPT-2]\n- evolution: LoRA基于Adapter思想提出低秩更新 [Adapter] [LoRA]";

pub const CROSS_PAPER_USER_PROMPT_TEMPLATE: &str =
    "请分析以下论文之间的关联：\n\n{context_text}\n\n找出最重要的关联（最多3个）：";

pub const RAG_SYSTEM_PROMPT: &str = "你是一个严谨的 AI 研究助手，精通论文阅读和学术分析。\n\n核心原则：\n1. 基于原文回答，不要捏造或推测未提及的内容\n2. 不确定的信息必须加 [推测] 标注\n3. 使用 > 块引用格式引用原文片段\n4. 区分\"原文明确说\"和\"可推断\"\n5. 回答使用中文，但引用原文时保留英文原句\n\n输出格式：\n- 开头总结回答要点（1-2句话）\n- 详细解释部分引用原文片段\n- 结尾标注信息来源";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_type_as_str() {
        assert_eq!(QueryType::Factual.as_str(), "factual");
        assert_eq!(QueryType::Conceptual.as_str(), "conceptual");
        assert_eq!(QueryType::Comparative.as_str(), "comparative");
        assert_eq!(QueryType::Temporal.as_str(), "temporal");
        assert_eq!(QueryType::General.as_str(), "general");
    }

    #[test]
    fn test_query_type_weights() {
        assert!((QueryType::Factual.bm25_weight() - 0.65).abs() < 1e-9);
        assert!((QueryType::Conceptual.bm25_weight() - 0.20).abs() < 1e-9);
        assert!((QueryType::General.bm25_weight() - 0.40).abs() < 1e-9);
    }

    #[test]
    fn test_query_type_semantic_weights() {
        assert!((QueryType::Factual.semantic_weight() - 0.35).abs() < 1e-9);
        assert!((QueryType::Conceptual.semantic_weight() - 0.80).abs() < 1e-9);
    }

    #[test]
    fn test_query_type_mmr_lambda() {
        assert!((QueryType::Factual.mmr_lambda() - 0.8).abs() < 1e-9);
        assert!((QueryType::General.mmr_lambda() - 0.6).abs() < 1e-9);
    }

    #[test]
    fn test_citation_new() {
        let c = Citation::new(
            "p1",
            "Title",
            vec!["Author".to_string()],
            "2024",
            "abstract text",
            0.9,
        );
        assert_eq!(c.paper_id, "p1");
        assert_eq!(c.relevance_score, 0.9);
        assert_eq!(c.section, "");
    }

    #[test]
    fn test_confidence_score_level() {
        let high = ConfidenceScore {
            score: 85.0,
            papers_count: 5,
            coverage: "5 papers".to_string(),
            warnings: vec![],
            sources: vec![],
        };
        assert_eq!(high.level(), "高");

        let mid = ConfidenceScore {
            score: 60.0,
            papers_count: 3,
            coverage: "3 papers".to_string(),
            warnings: vec![],
            sources: vec![],
        };
        assert_eq!(mid.level(), "中");

        let low = ConfidenceScore {
            score: 30.0,
            papers_count: 1,
            coverage: "1 paper".to_string(),
            warnings: vec![],
            sources: vec![],
        };
        assert_eq!(low.level(), "低");
    }

    #[test]
    fn test_chat_result_new() {
        let r = ChatResult::new("The answer is 42.");
        assert_eq!(r.answer, "The answer is 42.");
        assert!(r.citations.is_empty());
        assert!(r.papers_used.is_empty());
    }

    #[test]
    fn test_cross_paper_insight_new() {
        let insight = CrossPaperInsight::new(
            "comparison",
            "BERT vs GPT",
            vec!["BERT".to_string(), "GPT".to_string()],
        );
        assert_eq!(insight.insight_type, "comparison");
        assert_eq!(insight.summary, "BERT vs GPT");
        assert_eq!(insight.papers.len(), 2);
        assert!(insight.detail.is_empty());
    }

    #[test]
    fn test_chat_context_new() {
        let ctx = ChatContext::new("p1", "Title", vec!["A".to_string()], "2024", "snippet", 0.5);
        assert_eq!(ctx.paper_id, "p1");
        assert_eq!(ctx.relevance_score, 0.5);
    }

    #[test]
    fn test_query_type_display() {
        assert_eq!(format!("{}", QueryType::Temporal), "temporal");
    }

    #[test]
    fn test_query_type_default() {
        assert_eq!(QueryType::default(), QueryType::General);
    }
}
