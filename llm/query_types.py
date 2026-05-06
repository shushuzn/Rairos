"""Query type classification and RAG data structures."""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import List


class QueryType(Enum):
    """Query type classification for adaptive routing."""

    FACTUAL = "factual"  # Who, when, what (exact facts)
    CONCEPTUAL = "conceptual"  # Explain, how, why (understanding)
    COMPARATIVE = "comparative"  # vs, compared, difference (analysis)
    TEMPORAL = "temporal"  # recent, latest, 2024, new (time-sensitive)
    GENERAL = "general"  # Default fallback


# Query type → BM25 weight (semantic weight = 1 - BM25 weight)
_QUERY_WEIGHTS = {
    QueryType.FACTUAL: 0.65,
    QueryType.CONCEPTUAL: 0.20,
    QueryType.COMPARATIVE: 0.50,
    QueryType.TEMPORAL: 0.55,
    QueryType.GENERAL: 0.40,
}

# Query type → MMR lambda (0.7=relevance-biased, 0.5=balanced, 0.3=diversity-biased)
_MMR_LAMBDA = {
    QueryType.FACTUAL: 0.8,
    QueryType.CONCEPTUAL: 0.6,
    QueryType.COMPARATIVE: 0.5,
    QueryType.TEMPORAL: 0.7,
    QueryType.GENERAL: 0.6,
}


# ─── Cross-paper analysis prompts ──────────────────────────────────────────────

_CROSS_PAPER_SYSTEM_PROMPT = """你是一个研究综述助手，擅长发现论文之间的关联。

分析多篇论文，找出：
1. 共同点 (connection): 讨论相似主题或互补方法
2. 对比 (comparison): 同一问题的不同解决方法
3. 矛盾 (contradiction): 结论或方法冲突
4. 演进 (evolution): 后人如何在前人基础上改进

输出格式（最多3个洞察）：
- 类型: 一句话总结 [论文1] [论文2]
例如：
- comparison: BERT vs GPT的预训练目标不同 [BERT] [GPT-2]
- evolution: LoRA基于Adapter思想提出低秩更新 [Adapter] [LoRA]"""

_CROSS_PAPER_USER_PROMPT_TEMPLATE = """请分析以下论文之间的关联：

{context_text}

找出最重要的关联（最多3个）："""


# ─── Data Structures ───────────────────────────────────────────────────────────


@dataclass
class Citation:
    """A citation extracted from a paper with source tracing."""

    paper_id: str
    paper_title: str
    authors: List[str]
    published: str
    snippet: str
    relevance_score: float
    section: str = ""  # 论文章节 (abstract, intro, method, etc.)
    char_start: int = 0  # 在原文中的起始位置
    char_end: int = 0  # 在原文中的结束位置
    quote: str = ""  # 精确引用语句


@dataclass
class ChatContext:
    """A retrieved context from a paper."""

    paper_id: str
    paper_title: str
    authors: List[str]
    published: str
    snippet: str
    relevance_score: float


@dataclass
class ConfidenceScore:
    """Confidence score for RAG answer quality."""

    score: float  # 0-100 置信度
    papers_count: int  # 引用的论文数
    coverage: str  # 覆盖描述 (e.g., "3篇论文，覆盖Method章节")
    warnings: List[str] = field(default_factory=list)  # 低置信度警告
    sources: List[str] = field(default_factory=list)  # 主要来源章节

    @property
    def level(self) -> str:
        """Return confidence level label."""
        if self.score >= 80:
            return "高"
        elif self.score >= 50:
            return "中"
        return "低"


@dataclass
class CrossPaperInsight:
    """Cross-paper synthesis insight."""

    insight_type: str  # "comparison", "connection", "contradiction", "evolution"
    summary: str  # 一句话总结
    papers: List[str]  # 涉及的论文
    detail: str = ""  # 详细说明


# ─── RAG System Prompt ──────────────────────────────────────────────────────────

_RAG_SYSTEM_PROMPT = """你是一个严谨的 AI 研究助手，精通论文阅读和学术分析。

核心原则：
1. 基于原文回答，不要捏造或推测未提及的内容
2. 不确定的信息必须加 [推测] 标注
3. 使用 > 块引用格式引用原文片段
4. 区分"原文明确说"和"可推断"
5. 回答使用中文，但引用原文时保留英文原句

输出格式：
- 开头总结回答要点（1-2句话）
- 详细解释部分引用原文片段
- 结尾标注信息来源
"""


# ─── Chat Result ───────────────────────────────────────────────────────────────


@dataclass
class ChatResult:
    """Result of a RAG chat interaction."""

    answer: str
    citations: List[Citation] = field(default_factory=list)
    papers_used: List[str] = field(default_factory=list)
    session_id: Optional[str] = None  # 会话ID for continuity
    resolved_context: Optional[dict] = None  # 解析的上下文信息
    probing_questions: List[str] = field(default_factory=list)  # 智能追问建议
    confidence: Optional[ConfidenceScore] = None  # 答案可信度评分
    cross_paper_insights: List[CrossPaperInsight] = field(default_factory=list)  # 跨论文洞察
