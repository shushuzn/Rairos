"""
Research Question Validator: Validate novelty and feasibility of research questions.

研究问题验证器：验证研究问题的新颖性和可行性。

核心算法：
1. 问题扩展：将研究问题扩展为关键词组合
2. 文献匹配：BM25 + Embedding 双层检索
3. 新颖性分析：与现有工作对比
4. 方向建议：生成具体改进建议
"""

from __future__ import annotations

import re
from collections import defaultdict
from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum
from typing import Optional, List, Dict, Any, Tuple

from llm.constants import AI_RESEARCH_KEYWORDS, LLM_BASE_URL, LLM_MODEL

# Optional LLM import
try:
    from llm.chat import call_llm_chat_completions
    LLM_AVAILABLE = True
except ImportError:
    LLM_AVAILABLE = False


# ------------------------------------------------------------------
# Prompt constants for LLM-based analysis
# ------------------------------------------------------------------
_INNOVATION_ANALYSIS_SYSTEM_PROMPT = """分析研究问题的新颖性，从以下维度打分（0-10）：
1. method: 方法创新性 - 是否提出了新方法/架构/技术
2. task: 任务创新性 - 是否应用于新任务/场景/领域
3. evaluation: 评估创新性 - 是否提出了新评估标准/基准

输出格式：
method: X
task: X
evaluation: X
reasoning: 简短的评分理由"""

_INNOVATION_ANALYSIS_USER_PROMPT_TEMPLATE = """研究问题: {question}

相关工作:
{related_text}

请分析这个问题的创新潜力："""

_SUGGESTION_SYSTEM_PROMPT = """基于研究问题和相关工作，提出3-5条具体改进建议。
每条建议格式：[方向] 具体建议

方向可选：方法、任务、评估、数据、理论"""

_SUGGESTION_USER_PROMPT_TEMPLATE = """问题: {question}

创新评分: 方法{ method}/10, 任务{ task}/10, 评估{ evaluation}/10

相关工作:
{related_text}

请提出改进建议："""


class NoveltyLevel(Enum):
    """Novelty level for research questions."""
    HIGH = "high"       # 高创新性
    MEDIUM = "medium"   # 中等创新性
    LOW = "low"         # 低创新性 (已被充分研究)
    UNKNOWN = "unknown"  # 无法判断


class InnovationDimension(Enum):
    """Dimensions of research innovation."""
    METHOD = "method"           # 方法创新
    TASK = "task"               # 任务创新
    EVALUATION = "evaluation"   # 评估创新
    THEORY = "theory"           # 理论创新
    APPLICATION = "application"  # 应用创新


@dataclass
class RelatedWork:
    """A related paper that addresses similar questions."""
    paper_id: str
    title: str
    year: int
    relevance_score: float
    overlap_aspects: List[str]  # 重叠的方面
    difference_aspects: List[str]  # 差异方面
    conclusion: str  # 结论（是否解决了问题）


@dataclass
class InnovationScore:
    """Innovation score for a research question."""
    overall: float  # 0-10
    method: float   # 方法创新
    task: float     # 任务创新
    evaluation: float  # 评估创新
    dimensions: List[InnovationDimension]
    reasoning: str  # 打分理由


@dataclass
class ValidationResult:
    """Result of question validation."""
    question: str
    is_novel: bool
    novelty_level: NoveltyLevel
    innovation_score: InnovationScore
    related_works: List[RelatedWork] = field(default_factory=list)
    gap_summary: str = ""
    suggestions: List[str] = field(default_factory=list)
    confidence: float = 0.5  # 置信度 0-1


class QuestionValidator:
    """Validate novelty and feasibility of research questions."""

    # Question type patterns
    QUESTION_PATTERNS = [
        r"can\s+\w+\s+(learn|do|achieve|understand)\s+.+",
        r"how\s+to\s+.+",
        r"what\s+is\s+.+",
        r"why\s+.+",
        r"is\s+.+\s+better\s+than\s+.+",
        r"does\s+.+\s+work\s+for\s+.+",
        r"can\s+we\s+.+",
        r"is\s+it\s+possible\s+to\s+.+",
    ]

    def __init__(self, db=None):
        self.db = db

    def validate(
        self,
        question: str,
        use_llm: bool = True,
        api_key: Optional[str] = None,
        base_url: Optional[str] = None,
        model: Optional[str] = None,
        depth: str = "quick",
    ) -> ValidationResult:
        """
        Validate a research question.

        Args:
            question: Research question to validate
            use_llm: Whether to use LLM for analysis
            api_key: LLM API key
            base_url: LLM API base URL
            model: Model name
            depth: Analysis depth ("quick" or "full")

        Returns:
            ValidationResult with novelty analysis
        """
        result = ValidationResult(
            question=question,
            is_novel=False,
            novelty_level=NoveltyLevel.UNKNOWN,
            innovation_score=InnovationScore(
                overall=0.0,
                method=0.0,
                task=0.0,
                evaluation=0.0,
                dimensions=[],
                reasoning="",
            ),
        )

        # 1. Expand question to keywords
        keywords = self._expand_question(question)
        result.gap_summary = f"关键词: {', '.join(keywords[:5])}"

        # 2. Find related works
        related = self._find_related_works(keywords, limit=5 if depth == "quick" else 10)
        result.related_works = related

        # 3. Analyze innovation
        if use_llm and LLM_AVAILABLE:
            innovation = self._analyze_innovation_llm(
                question, related, api_key, base_url, model
            )
            result.innovation_score = innovation
            result.suggestions = self._generate_suggestions_llm(
                question, related, innovation, api_key, base_url, model
            )
        else:
            innovation = self._analyze_innovation_rules(related)
            result.innovation_score = innovation
            result.suggestions = self._generate_suggestions_rules(related)

        # 4. Determine novelty level
        result.novelty_level = self._determine_novelty(innovation, related)
        result.is_novel = result.novelty_level != NoveltyLevel.LOW

        # 5. Calculate confidence
        result.confidence = self._calculate_confidence(related, innovation)

        return result

    def _expand_question(self, question: str) -> List[str]:
        """Expand question into searchable keywords."""
        # Remove common question words
        cleaned = re.sub(r'\b(can|how|what|why|is|does|to|the|a|an)\b', '', question.lower())
        cleaned = re.sub(r'[^\w\s]', ' ', cleaned)
        words = [w.strip() for w in cleaned.split() if len(w.strip()) > 2]

        # Add key technical terms
        for term in AI_RESEARCH_KEYWORDS:
            if term in question.lower():
                words.append(term)

        return list(set(words))[:10]

    def _find_related_works(
        self,
        keywords: List[str],
        limit: int = 5,
    ) -> List[RelatedWork]:
        """Find related works from database."""
        if not self.db:
            return []

        related = []
        try:
            # Search by each keyword
            for kw in keywords[:3]:
                rows, _ = self.db.search_papers(kw, limit=limit)
                for row in rows:
                    paper_id = getattr(row, 'id', '')
                    title = getattr(row, 'title', '') or ''
                    year = getattr(row, 'year', 0) or 0
                    abstract = getattr(row, 'abstract', '') or ''

                    # Check if already in results
                    if any(r.paper_id == paper_id for r in related):
                        continue

                    # Calculate relevance
                    text = f"{title} {abstract}".lower()
                    matches = sum(1 for kw in keywords if kw.lower() in text)
                    relevance = matches / len(keywords)

                    if relevance > 0.1:
                        related.append(RelatedWork(
                            paper_id=paper_id,
                            title=title[:80],
                            year=year,
                            relevance_score=relevance,
                            overlap_aspects=[],
                            difference_aspects=[],
                            conclusion="",
                        ))

        except Exception:
            # Semantic related-work search failed — return empty list without crashing.
            pass

        # Sort by relevance
        related.sort(key=lambda x: x.relevance_score, reverse=True)
        return related[:limit]

    def _analyze_innovation_llm(
        self,
        question: str,
        related: List[RelatedWork],
        api_key: Optional[str],
        base_url: Optional[str],
        model: Optional[str],
    ) -> InnovationScore:
        """Use LLM to analyze innovation potential."""
        import os

        api_key = api_key or os.getenv("OPENAI_API_KEY", "")
        if not api_key:
            return self._analyze_innovation_rules(related)

        related_text = "\n".join([
            f"- {r.title} ({r.year})"
            for r in related
        ]) or "无相关论文"

        user_prompt = _INNOVATION_ANALYSIS_USER_PROMPT_TEMPLATE.format(
            question=question,
            related_text=related_text,
        )

        try:
            response = call_llm_chat_completions(
                base_url=base_url or LLM_BASE_URL,
                api_key=api_key,
                model=model or LLM_MODEL,
                system_prompt=_INNOVATION_ANALYSIS_SYSTEM_PROMPT,
                user_prompt=user_prompt,
            )

            return self._parse_innovation_response(response, related)

        except Exception:
            # LLM innovation analysis failed — fall back to rule-based analysis without crashing.
            return self._analyze_innovation_rules(related)

    def _parse_innovation_response(
        self,
        response: str,
        related: List[RelatedWork],
    ) -> InnovationScore:
        """Parse LLM innovation analysis response."""
        method_score = 5.0
        task_score = 5.0
        eval_score = 5.0
        reasoning = ""

        for line in response.strip().split('\n'):
            line = line.strip().lower()
            if line.startswith('method:'):
                try:
                    method_score = float(line.split(':')[1].strip())
                except (ValueError, IndexError):
                    pass
            elif line.startswith('task:'):
                try:
                    task_score = float(line.split(':')[1].strip())
                except (ValueError, IndexError):
                    pass
            elif line.startswith('evaluation:'):
                try:
                    eval_score = float(line.split(':')[1].strip())
                except (ValueError, IndexError):
                    pass
            elif line.startswith('reasoning:'):
                reasoning = line.split(':', 1)[1].strip()

        # Overall as weighted average
        overall = method_score * 0.4 + task_score * 0.3 + eval_score * 0.3

        # Determine dimensions with high scores
        dimensions = []
        if method_score >= 7:
            dimensions.append(InnovationDimension.METHOD)
        if task_score >= 7:
            dimensions.append(InnovationDimension.TASK)
        if eval_score >= 7:
            dimensions.append(InnovationDimension.EVALUATION)

        return InnovationScore(
            overall=overall,
            method=method_score,
            task=task_score,
            evaluation=eval_score,
            dimensions=dimensions,
            reasoning=reasoning,
        )

    def _analyze_innovation_rules(self, related: List[RelatedWork]) -> InnovationScore:
        """Rule-based innovation analysis."""
        if not related:
            # No related works = high novelty
            return InnovationScore(
                overall=8.0,
                method=7.0,
                task=8.0,
                evaluation=7.0,
                dimensions=[
                    InnovationDimension.METHOD,
                    InnovationDimension.TASK,
                    InnovationDimension.EVALUATION,
                ],
                reasoning="未发现相关工作，可能是全新领域",
            )

        # High overlap = low novelty
        avg_relevance = sum(r.relevance_score for r in related) / len(related)
        max_relevance = max(r.relevance_score for r in related)

        if max_relevance > 0.8:
            # Very similar work exists
            return InnovationScore(
                overall=3.0,
                method=3.0,
                task=4.0,
                evaluation=3.0,
                dimensions=[],
                reasoning=f"发现高度相关工作 (相似度 {max_relevance:.0%})",
            )
        elif max_relevance > 0.5:
            # Some related but different angle
            return InnovationScore(
                overall=6.0,
                method=6.0,
                task=5.0,
                evaluation=6.0,
                dimensions=[InnovationDimension.METHOD],
                reasoning=f"有相关工作，但有新角度 (相似度 {max_relevance:.0%})",
            )
        else:
            # Different scope or application
            return InnovationScore(
                overall=7.5,
                method=7.0,
                task=8.0,
                evaluation=7.0,
                dimensions=[
                    InnovationDimension.TASK,
                    InnovationDimension.APPLICATION,
                ],
                reasoning="发现部分相关，但领域/应用不同",
            )

    def _generate_suggestions_llm(
        self,
        question: str,
        related: List[RelatedWork],
        innovation: InnovationScore,
        api_key: Optional[str],
        base_url: Optional[str],
        model: Optional[str],
    ) -> List[str]:
        """Use LLM to generate improvement suggestions."""
        import os

        api_key = api_key or os.getenv("OPENAI_API_KEY", "")
        if not api_key:
            return self._generate_suggestions_rules(related)

        related_text = "\n".join([f"- {r.title}" for r in related[:3]]) or "无"

        user_prompt = _SUGGESTION_USER_PROMPT_TEMPLATE.format(
            question=question,
            method=innovation.method,
            task=innovation.task,
            evaluation=innovation.evaluation,
            related_text=related_text,
        )

        try:
            response = call_llm_chat_completions(
                base_url=base_url or LLM_BASE_URL,
                api_key=api_key,
                model=model or LLM_MODEL,
                system_prompt=_SUGGESTION_SYSTEM_PROMPT,
                user_prompt=user_prompt,
            )

            suggestions = []
            for line in response.strip().split('\n'):
                line = line.strip()
                if line and (line.startswith('[') or line.startswith('-')):
                    suggestions.append(line.lstrip('[-] '))

            return suggestions[:5]

        except Exception:
            # LLM suggestion generation failed — fall back to rule-based suggestions without crashing.
            return self._generate_suggestions_rules(related)

    def _generate_suggestions_rules(self, related: List[RelatedWork]) -> List[str]:
        """Rule-based suggestion generation."""
        suggestions = []

        if not related:
            suggestions.append("[方法] 设计全新的方法框架")
            suggestions.append("[任务] 探索具体的落地场景")
            suggestions.append("[评估] 建立评估基准和指标")
        else:
            # Analyze gaps in related works
            recent_papers = [r for r in related if r.year >= 2023]
            if recent_papers:
                suggestions.append(f"[方法] 参考 {len(recent_papers)} 篇最新工作，选择差异化路线")

            suggestions.append("[任务] 考虑跨领域应用场景")
            suggestions.append("[评估] 设计针对新问题的评估指标")
            suggestions.append("[数据] 构建专用数据集")

        return suggestions

    def _determine_novelty(
        self,
        innovation: InnovationScore,
        related: List[RelatedWork],
    ) -> NoveltyLevel:
        """Determine overall novelty level."""
        if not related and innovation.overall >= 7:
            return NoveltyLevel.HIGH

        if innovation.overall >= 7:
            return NoveltyLevel.HIGH
        elif innovation.overall >= 5:
            return NoveltyLevel.MEDIUM
        else:
            return NoveltyLevel.LOW

    def _calculate_confidence(
        self,
        related: List[RelatedWork],
        innovation: InnovationScore,
    ) -> float:
        """Calculate confidence of the validation."""
        # More related works = higher confidence
        related_score = min(len(related) / 5, 1.0) * 0.4

        # Clear reasoning = higher confidence
        reasoning_score = 0.3 if innovation.reasoning else 0.15

        # Diverse dimensions = higher confidence
        dimension_score = len(innovation.dimensions) / 3 * 0.3

        return min(related_score + reasoning_score + dimension_score, 0.95)

    def render_result(self, result: ValidationResult) -> str:
        """Render validation result as formatted string."""
        novelty_icon = {
            NoveltyLevel.HIGH: "🟢",
            NoveltyLevel.MEDIUM: "🟡",
            NoveltyLevel.LOW: "🔴",
            NoveltyLevel.UNKNOWN: "⚪",
        }.get(result.novelty_level, "⚪")

        lines = [
            f"🔬 研究问题验证: \"{result.question[:60]}{'...' if len(result.question) > 60 else ''}\"",
            "",
            f"{novelty_icon} 创新指数: {result.innovation_score.overall:.1f}/10",
            f"   方法创新: {result.innovation_score.method:.0f}/10",
            f"   任务创新: {result.innovation_score.task:.0f}/10",
            f"   评估创新: {result.innovation_score.evaluation:.0f}/10",
            "",
        ]

        if result.innovation_score.dimensions:
            dims = [d.value for d in result.innovation_score.dimensions]
            lines.append(f"   亮点维度: {', '.join(dims)}")

        if result.innovation_score.reasoning:
            lines.append(f"   分析: {result.innovation_score.reasoning}")

        lines.append("")

        if result.related_works:
            lines.append("📚 相关工作:")
            for i, work in enumerate(result.related_works[:3], 1):
                lines.append(f"   {i}. {work.title} ({work.year})")
                lines.append(f"      相关度: {work.relevance_score:.0%}")
            lines.append("")

        if result.suggestions:
            lines.append("💡 改进建议:")
            for suggestion in result.suggestions[:4]:
                lines.append(f"   • {suggestion}")
            lines.append("")

        lines.append(f"📊 置信度: {result.confidence:.0%}")
        lines.append(f"🎯 结论: {'✅ 值得探索' if result.is_novel else '⚠️ 需要更细致的角度'}")

        return "\n".join(lines)

    def render_json(self, result: ValidationResult) -> str:
        """Render result as JSON."""
        import json

        data = {
            "question": result.question,
            "is_novel": result.is_novel,
            "novelty_level": result.novelty_level.value,
            "innovation_score": {
                "overall": result.innovation_score.overall,
                "method": result.innovation_score.method,
                "task": result.innovation_score.task,
                "evaluation": result.innovation_score.evaluation,
                "dimensions": [d.value for d in result.innovation_score.dimensions],
                "reasoning": result.innovation_score.reasoning,
            },
            "related_works": [
                {
                    "paper_id": w.paper_id,
                    "title": w.title,
                    "year": w.year,
                    "relevance_score": w.relevance_score,
                }
                for w in result.related_works
            ],
            "suggestions": result.suggestions,
            "confidence": result.confidence,
        }

        return json.dumps(data, ensure_ascii=False, indent=2)
