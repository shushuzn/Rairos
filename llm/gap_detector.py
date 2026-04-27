"""
Research Gap Detector: Identify research gaps and generate research questions.

研究空白检测器：分析论文库，自动发现研究空白并生成研究问题。

核心算法：
1. 收集领域内的核心论文
2. LLM 分析论文间的逻辑关系（矛盾、未探索领域、方法局限）
3. 识别研究空白类型
4. 生成可验证的研究问题
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from enum import Enum
from typing import Optional, List, Dict, Any, Tuple

# Optional LLM import
try:
    from llm.chat import call_llm_chat_completions
    LLM_AVAILABLE = True
except ImportError:
    LLM_AVAILABLE = False

from llm.constants import LLM_BASE_URL, LLM_MODEL

# =============================================================================
# Prompt templates
# =============================================================================
_GAP_DETECTION_SYSTEM_PROMPT = """你是一个研究空白检测专家。分析给定领域的论文列表，识别：
1. 未充分探索的应用场景
2. 方法的局限性
3. 论文间的矛盾点
4. 评估标准的缺失
5. 可扩展性问题
6. 理论基础薄弱之处
7. 数据集缺失
8. 泛化能力未验证的问题

输出格式（每行一个gap）：
[GAP_TYPE] 描述 | 证据论文 | 置信度(0-1) | 严重程度(high/medium/low)

GAP_TYPE 可选：
- unexplored_application: 未探索的应用
- method_limitation: 方法局限
- contradiction: 矛盾
- evaluation_gap: 评估缺失
- scalability_issue: 可扩展性
- theoretical_gap: 理论空白
- dataset_gap: 数据集缺失
- generalization_gap: 泛化问题"""

_GAP_DETECTION_USER_PROMPT_TEMPLATE = """分析领域：{topic}

论文列表：
{paper_summaries}

请识别该领域的研究空白："""

_QUESTION_GENERATION_SYSTEM_PROMPT = """基于研究空白，生成3-5个具体、可验证的研究问题。
每个问题应该：
1. 明确说明要研究什么
2. 有清晰的研究假设
3. 包含方法建议
4. 说明预期影响

输出格式（每行一个问题）：
问题 | 研究假设 | 方法建议 | 预期影响 | 可行性(0-1) | 创新性(0-1)"""

_QUESTION_GENERATION_USER_PROMPT_TEMPLATE = """领域：{topic}

发现的研究空白：
{gaps_text}

请生成研究问题："""


class GapType(Enum):
    """Types of research gaps."""
    UNEXPLORED_APPLICATION = "unexplored_application"  # 未探索的应用场景
    METHOD_LIMITATION = "method_limitation"  # 方法局限
    CONTRADICTION = "contradiction"  # 论文间的矛盾
    EVALUATION_GAP = "evaluation_gap"  # 评估标准缺失
    SCALABILITY_ISSUE = "scalability_issue"  # 可扩展性问题
    THEORETICAL_GAP = "theoretical_gap"  # 理论基础薄弱
    DATASET_GAP = "dataset_gap"  # 数据集缺失
    GENERALIZATION_GAP = "generalization_gap"  # 泛化能力未验证


class GapSeverity(Enum):
    """Severity of the research gap."""
    HIGH = "high"  # 高 - 核心问题，影响领域发展
    MEDIUM = "medium"  # 中 - 重要但非核心
    LOW = "low"  # 低 - 边缘问题


# Module-level constants shared across gap detection and question generation
# All 8 GapType values are defined here; detection only uses a subset.
_GAP_TYPE_PATTERNS: Dict[GapType, List[str]] = {
    GapType.UNEXPLORED_APPLICATION: [
        r'未探索|未研究|future work|future directions|open problem',
        r'potential application|limitation.*future|future research',
    ],
    GapType.METHOD_LIMITATION: [
        r'limitation|不足|weakness|shortcoming|constraint',
        r'does not scale|only works for|restricted to',
    ],
    GapType.CONTRADICTION: [
        r'however|but|in contrast|on the contrary',
        r'conflicting|disagree|differ|contradict',
    ],
    GapType.EVALUATION_GAP: [
        r'no benchmark|lack.*evaluation|evaluation.*limited',
        r'no standard|without baseline',
    ],
}

_GAP_QUESTION_TEMPLATES: Dict[GapType, str] = {
    GapType.UNEXPLORED_APPLICATION: "如何将 {topic} 应用于新场景？",
    GapType.METHOD_LIMITATION: "如何改进 {topic} 的方法以解决局限？",
    GapType.CONTRADICTION: "如何协调/解决 {topic} 中的矛盾发现？",
    GapType.EVALUATION_GAP: "如何为 {topic} 建立标准化评估？",
    GapType.SCALABILITY_ISSUE: "{topic} 如何扩展到更大规模？",
    GapType.THEORETICAL_GAP: "如何加强 {topic} 的理论基础？",
    GapType.DATASET_GAP: "如何构建 {topic} 的基准数据集？",
    GapType.GENERALIZATION_GAP: "{topic} 的泛化能力如何验证？",
}


@dataclass
class ResearchGap:
    """Represents a research gap."""
    gap_type: GapType
    description: str  # 空白描述
    evidence_papers: List[str]  # 支持证据的论文
    severity: GapSeverity = GapSeverity.MEDIUM
    confidence: float = 0.5  # 置信度 0-1
    suggested_approach: str = ""  # 建议的研究方法
    related_gaps: List[str] = field(default_factory=list)  # 相关的其他空白


@dataclass
class ResearchQuestion:
    """A generated research question."""
    question: str
    gap: ResearchGap  # 来源的空白
    hypothesis: str = ""  # 研究假设
    methodology_suggestion: str = ""  # 方法建议
    expected_impact: str = ""  # 预期影响
    feasibility: float = 0.5  # 可行性 0-1
    novelty_score: float = 0.5  # 创新性 0-1


@dataclass
class GapAnalysisResult:
    """Result of gap analysis."""
    topic: str
    gaps: List[ResearchGap] = field(default_factory=list)
    questions: List[ResearchQuestion] = field(default_factory=list)
    coverage_score: float = 0.0  # 已覆盖的研究程度
    opportunities_score: float = 0.0  # 研究机会程度
    analyzed_papers_count: int = 0
    summary: str = ""


class GapDetector:
    """Detect research gaps from paper corpus."""

    def __init__(self, db=None):
        self.db = db

    def analyze(
        self,
        topic: str,
        use_llm: bool = True,
        api_key: Optional[str] = None,
        base_url: Optional[str] = None,
        model: Optional[str] = None,
        min_papers: int = 3,
    ) -> GapAnalysisResult:
        """
        Analyze a research topic for gaps.

        Args:
            topic: Research topic/keyword
            use_llm: Whether to use LLM for deep analysis
            api_key: LLM API key
            base_url: LLM API base URL
            model: Model name
            min_papers: Minimum papers needed for analysis

        Returns:
            GapAnalysisResult with identified gaps and questions
        """
        # 1. Collect papers
        papers = self._collect_papers(topic)
        if len(papers) < min_papers:
            return self._empty_result(topic)

        result = GapAnalysisResult(
            topic=topic,
            analyzed_papers_count=len(papers),
        )

        # 2. Extract key information
        paper_summaries = self._summarize_papers(papers)

        if use_llm and LLM_AVAILABLE:
            # LLM-powered gap analysis
            gaps = self._detect_gaps_llm(
                topic, paper_summaries, api_key, base_url, model
            )
            questions = self._generate_questions_llm(
                gaps, topic, api_key, base_url, model
            )
        else:
            # Rule-based fallback
            gaps = self._detect_gaps_rules(paper_summaries)
            questions = self._generate_questions_rules(gaps)

        result.gaps = gaps
        result.questions = questions

        # 3. Calculate scores
        result.coverage_score = self._calculate_coverage(papers)
        result.opportunities_score = len(gaps) / max(len(papers), 1) * 10

        # 4. Generate summary
        result.summary = self._generate_summary(result)

        return result

    def _collect_papers(self, topic: str) -> List[Dict[str, Any]]:
        """Collect papers related to topic from DB."""
        if not self.db:
            return []

        papers = []
        try:
            rows, _ = self.db.search_papers(topic, limit=20)
            if not rows:
                return []

            # Get full paper data via bulk fetch
            paper_ids = [r.paper_id for r in rows]
            full_papers = self.db.get_papers_bulk(paper_ids)

            for row in rows:
                paper_id = row.paper_id
                full = full_papers.get(paper_id)
                if full:
                    paper = {
                        "id": paper_id,
                        "title": full.title or topic,
                        "abstract": full.abstract or '',
                        "year": full.published[:4] if full.published else '',
                        "authors": full.authors or '',
                    }
                    papers.append(paper)
        except Exception:
            # Paper enrichment is optional — return partial results without crashing.
            pass

        return papers

    def _summarize_papers(self, papers: List[Dict]) -> str:
        """Create a summary of papers for LLM analysis."""
        summaries = []
        for p in papers[:10]:  # Limit to 10 papers
            title = p.get('title', '')[:80]
            abstract = p.get('abstract', '')[:200]
            year = p.get('year', '')
            summaries.append(f"- [{year}] {title}")
            if abstract:
                summaries.append(f"  摘要: {abstract}...")

        return "\n".join(summaries)

    def _detect_gaps_llm(
        self,
        topic: str,
        paper_summaries: str,
        api_key: Optional[str],
        base_url: Optional[str],
        model: Optional[str],
    ) -> List[ResearchGap]:
        """Use LLM to detect research gaps."""
        import os

        api_key = api_key or os.getenv("OPENAI_API_KEY", "")
        if not api_key:
            return self._detect_gaps_rules(paper_summaries)

        user_prompt = _GAP_DETECTION_USER_PROMPT_TEMPLATE.format(
            topic=topic,
            paper_summaries=paper_summaries,
        )

        try:
            response = call_llm_chat_completions(
                messages=[{"role": "user", "content": user_prompt}],
                base_url=base_url or LLM_BASE_URL,
                api_key=api_key,
                model=model or LLM_MODEL,
                system_prompt=_GAP_DETECTION_SYSTEM_PROMPT,
            )

            return self._parse_gaps(response, topic)

        except Exception:
            # LLM gap detection failed — fall back to rule-based detection without crashing.
            return self._detect_gaps_rules(paper_summaries)

    def _parse_gaps(self, response: str, topic: str) -> List[ResearchGap]:
        """Parse LLM response into ResearchGap objects."""
        gaps = []
        type_map = {
            "unexplored_application": GapType.UNEXPLORED_APPLICATION,
            "method_limitation": GapType.METHOD_LIMITATION,
            "contradiction": GapType.CONTRADICTION,
            "evaluation_gap": GapType.EVALUATION_GAP,
            "scalability_issue": GapType.SCALABILITY_ISSUE,
            "theoretical_gap": GapType.THEORETICAL_GAP,
            "dataset_gap": GapType.DATASET_GAP,
            "generalization_gap": GapType.GENERALIZATION_GAP,
        }
        severity_map = {
            "high": GapSeverity.HIGH,
            "medium": GapSeverity.MEDIUM,
            "low": GapSeverity.LOW,
        }

        # Strip thinking tags for MiniMax/M2.7 models
        clean_response = re.sub(r'<think>.*?</think>', '', response, flags=re.DOTALL).strip()

        for line in clean_response.split('\n'):
            line = line.strip()
            if not line or line.startswith('#'):
                continue

            # Parse [TYPE] description | papers | confidence | severity
            match = re.match(r'\[(\w+)\]\s*(.+?)\s*\|\s*(.+?)\s*\|\s*([\d.]+)\s*\|\s*(\w+)', line)
            if match:
                gap_type_str, description, papers_str, conf_str, severity_str = match.groups()

                gap_type = type_map.get(gap_type_str, GapType.METHOD_LIMITATION)
                severity = severity_map.get(severity_str, GapSeverity.MEDIUM)
                confidence = float(conf_str) if conf_str else 0.5
                papers = [p.strip() for p in papers_str.split(',') if p.strip()]

                gap = ResearchGap(
                    gap_type=gap_type,
                    description=description.strip(),
                    evidence_papers=papers,
                    severity=severity,
                    confidence=confidence,
                )
                gaps.append(gap)

        return gaps

    def _detect_gaps_rules(self, paper_summaries: str) -> List[ResearchGap]:
        """Rule-based gap detection (fallback when LLM unavailable)."""
        gaps = []

        for gap_type, type_patterns in _GAP_TYPE_PATTERNS.items():
            for pattern in type_patterns:
                if re.search(pattern, paper_summaries, re.IGNORECASE):
                    gaps.append(ResearchGap(
                        gap_type=gap_type,
                        description=f"基于关键词 '{pattern}' 发现的潜在研究空白",
                        evidence_papers=["从摘要中推断"],
                        confidence=0.3,
                    ))
                    break

        return gaps

    def _generate_questions_llm(
        self,
        gaps: List[ResearchGap],
        topic: str,
        api_key: Optional[str],
        base_url: Optional[str],
        model: Optional[str],
    ) -> List[ResearchQuestion]:
        """Use LLM to generate research questions from gaps."""
        import os

        api_key = api_key or os.getenv("OPENAI_API_KEY", "")
        if not api_key or not gaps:
            return self._generate_questions_rules(gaps)

        gaps_text = "\n".join([
            f"- {g.gap_type.value}: {g.description}"
            for g in gaps
        ])

        user_prompt = _QUESTION_GENERATION_USER_PROMPT_TEMPLATE.format(
            topic=topic,
            gaps_text=gaps_text,
        )

        try:
            response = call_llm_chat_completions(
                base_url=base_url or LLM_BASE_URL,
                api_key=api_key,
                model=model or LLM_MODEL,
                system_prompt=_QUESTION_GENERATION_SYSTEM_PROMPT,
                user_prompt=user_prompt,
            )

            return self._parse_questions(response, gaps)

        except Exception:
            # LLM question generation failed — fall back to rule-based generation without crashing.
            return self._generate_questions_rules(gaps)

    def _parse_questions(self, response: str, gaps: List[ResearchGap]) -> List[ResearchQuestion]:
        """Parse LLM response into ResearchQuestion objects."""
        questions = []
        default_gap = gaps[0] if gaps else None

        for line in response.strip().split('\n'):
            line = line.strip()
            if not line or line.startswith('#'):
                continue

            parts = [p.strip() for p in line.split('|')]
            if len(parts) >= 1:
                question_text = parts[0]
                hypothesis = parts[1] if len(parts) > 1 else ""
                methodology = parts[2] if len(parts) > 2 else ""
                impact = parts[3] if len(parts) > 3 else ""
                feasibility = float(parts[4]) if len(parts) > 4 and parts[4] else 0.5
                novelty = float(parts[5]) if len(parts) > 5 and parts[5] else 0.5

                q = ResearchQuestion(
                    question=question_text,
                    gap=default_gap,
                    hypothesis=hypothesis,
                    methodology_suggestion=methodology,
                    expected_impact=impact,
                    feasibility=feasibility,
                    novelty_score=novelty,
                )
                questions.append(q)

        return questions

    def _generate_questions_rules(self, gaps: List[ResearchGap]) -> List[ResearchQuestion]:
        """Rule-based question generation."""
        questions = []
        topic = "该方法"  # Generic fallback
        for gap in gaps[:5]:  # Max 5 questions
            template = _GAP_QUESTION_TEMPLATES.get(gap.gap_type, "如何改进 {topic}？")
            question_text = template.format(topic=topic)

            q = ResearchQuestion(
                question=question_text,
                gap=gap,
                feasibility=0.6,
                novelty_score=0.5,
            )
            questions.append(q)

        return questions

    def _calculate_coverage(self, papers: List[Dict]) -> float:
        """Calculate how well the topic is covered."""
        if not papers:
            return 0.0

        # Factors: number of papers, recency, abstract quality
        count_score = min(len(papers) / 20, 1.0)  # Max at 20 papers

        # Recency score
        recency_scores = []
        for p in papers:
            try:
                year = int(p.get('year', 0))
            except (ValueError, TypeError):
                year = 0
            if year >= 2024:
                recency_scores.append(1.0)
            elif year >= 2022:
                recency_scores.append(0.7)
            elif year >= 2020:
                recency_scores.append(0.4)
            else:
                recency_scores.append(0.1)

        recency = sum(recency_scores) / len(recency_scores) if recency_scores else 0

        # Abstract quality
        has_abstract = sum(1 for p in papers if p.get('abstract')) / len(papers)

        return (count_score * 0.4 + recency * 0.4 + has_abstract * 0.2)

    def _generate_summary(self, result: GapAnalysisResult) -> str:
        """Generate a human-readable summary."""
        high_gaps = [g for g in result.gaps if g.severity == GapSeverity.HIGH]
        medium_gaps = [g for g in result.gaps if g.severity == GapSeverity.MEDIUM]

        summary_parts = []

        if high_gaps:
            summary_parts.append(f"发现 {len(high_gaps)} 个高优先级研究空白")

        if medium_gaps:
            summary_parts.append(f"{len(medium_gaps)} 个中优先级空白")

        if result.coverage_score > 0.7:
            summary_parts.append("该领域研究较为成熟")
        elif result.coverage_score > 0.4:
            summary_parts.append("该领域有一定基础，仍有探索空间")
        else:
            summary_parts.append("该领域研究较少，创新机会较多")

        if result.questions:
            summary_parts.append(f"生成 {len(result.questions)} 个研究问题建议")

        return "；".join(summary_parts)

    def _empty_result(self, topic: str) -> GapAnalysisResult:
        """Return empty result when not enough papers."""
        return GapAnalysisResult(
            topic=topic,
            summary="论文数量不足，无法进行有效分析",
        )

    def render_result(self, result: GapAnalysisResult) -> str:
        """Render analysis result as formatted string."""
        lines = [
            f"🔬 《{result.topic}》研究空白分析",
            f"   分析论文数: {result.analyzed_papers_count}",
            f"   覆盖程度: {result.coverage_score:.0%}",
            f"   机会评分: {result.opportunities_score:.1f}/10",
            "",
        ]

        if result.gaps:
            lines.append("💡 研究空白：")
            for i, gap in enumerate(result.gaps, 1):
                severity_icon = {
                    GapSeverity.HIGH: "🔴",
                    GapSeverity.MEDIUM: "🟡",
                    GapSeverity.LOW: "🟢",
                }.get(gap.severity, "⚪")

                gap_type_name = {
                    GapType.UNEXPLORED_APPLICATION: "未探索应用",
                    GapType.METHOD_LIMITATION: "方法局限",
                    GapType.CONTRADICTION: "矛盾",
                    GapType.EVALUATION_GAP: "评估缺失",
                    GapType.SCALABILITY_ISSUE: "可扩展性",
                    GapType.THEORETICAL_GAP: "理论空白",
                    GapType.DATASET_GAP: "数据集缺失",
                    GapType.GENERALIZATION_GAP: "泛化问题",
                }.get(gap.gap_type, gap.gap_type.value)

                lines.append(f"  {i}. {severity_icon} [{gap_type_name}] {gap.description}")
                if gap.evidence_papers:
                    lines.append(f"     证据: {', '.join(gap.evidence_papers[:2])}")
            lines.append("")

        if result.questions:
            lines.append("📝 研究问题建议：")
            for i, q in enumerate(result.questions, 1):
                lines.append(f"  {i}. {q.question}")
                if q.hypothesis:
                    lines.append(f"     假设: {q.hypothesis[:60]}...")
                if q.methodology_suggestion:
                    lines.append(f"     方法: {q.methodology_suggestion[:50]}...")
            lines.append("")

        lines.append(f"📊 {result.summary}")

        return "\n".join(lines)

    def render_json(self, result: GapAnalysisResult) -> str:
        """Render result as JSON."""
        import json

        data = {
            "topic": result.topic,
            "gaps": [
                {
                    "type": g.gap_type.value,
                    "description": g.description,
                    "severity": g.severity.value,
                    "confidence": g.confidence,
                    "evidence_papers": g.evidence_papers,
                }
                for g in result.gaps
            ],
            "questions": [
                {
                    "question": q.question,
                    "hypothesis": q.hypothesis,
                    "methodology": q.methodology_suggestion,
                    "feasibility": q.feasibility,
                    "novelty": q.novelty_score,
                }
                for q in result.questions
            ],
            "coverage_score": result.coverage_score,
            "opportunities_score": result.opportunities_score,
            "analyzed_papers_count": result.analyzed_papers_count,
        }

        return json.dumps(data, ensure_ascii=False, indent=2)
