"""
Research Argument Builder: Build structured arguments from evidence.

研究论证构建器：从证据构建结构化论证。

核心功能：
1. 论点管理：核心论点 + 支持/反驳证据
2. 证据收集：论文证据 + 用户洞察 + 引用关系
3. 论证生成：结构化论证 + 论文章节建议
4. 对话集成：与 Session 联动
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import Optional, List, Dict, Any

# Optional imports
try:
    from llm.chat import call_llm_chat_completions
    LLM_AVAILABLE = True
except ImportError:
    LLM_AVAILABLe = False


class EvidenceType(Enum):
    """Type of evidence."""
    SUPPORT = "support"           # 支持证据
    CONTRADICT = "contradict"     # 反驳证据
    QUALIFY = "qualify"          # 限定条件
    METHODOLOGICAL = "methodological"  # 方法论问题


class ArgumentSection(Enum):
    """Standard argument sections."""
    INTRODUCTION = "introduction"
    RELATED_WORK = "related_work"
    METHODOLOGY = "methodology"
    EXPERIMENTS = "experiments"
    DISCUSSION = "discussion"
    LIMITATION = "limitation"


@dataclass
class Evidence:
    """A piece of evidence for or against a claim."""
    evidence_type: EvidenceType
    source: str  # paper title, insight ID, etc.
    content: str  # the actual evidence
    citation: str = ""  # citation info
    weight: float = 1.0  # evidence strength (0-1)


@dataclass
class Claim:
    """A single claim in an argument."""
    text: str
    evidence: List[Evidence] = field(default_factory=list)
    confidence: float = 0.5  # confidence level (0-1)


@dataclass
class Argument:
    """A structured research argument."""
    thesis: str  # 核心论点
    claims: List[Claim] = field(default_factory=list)
    supporting_evidence: List[Evidence] = field(default_factory=list)
    contradicting_evidence: List[Evidence] = field(default_factory=list)
    related_gaps: List[str] = field(default_factory=list)
    paper_suggestions: List[ArgumentSection] = field(default_factory=list)


@dataclass
class ArgumentResult:
    """Complete argument building result."""
    topic: str
    argument: Argument
    summary: str = ""
    section_guidance: Dict[ArgumentSection, str] = field(default_factory=dict)


class ArgumentBuilder:
    """Build structured arguments from research evidence."""

    def __init__(self, db=None, insight_manager=None, gap_analyzer=None):
        self.db = db
        self.insight_manager = insight_manager
        self.gap_analyzer = gap_analyzer

    def build(
        self,
        thesis: str,
        use_llm: bool = True,
        model: Optional[str] = None,
    ) -> ArgumentResult:
        """Build an argument from a thesis."""
        # 1. Search for evidence in papers
        paper_evidence = self._search_paper_evidence(thesis)

        # 2. Collect user insights
        insight_evidence = self._collect_insight_evidence(thesis)

        # 3. Find related gaps
        related_gaps = self._find_related_gaps(thesis)

        # 4. Categorize evidence
        supporting, contradicting = self._categorize_evidence(
            paper_evidence + insight_evidence
        )

        # 5. Generate section guidance
        section_guidance = self._generate_section_guidance(
            thesis, supporting, contradicting, use_llm, model
        )

        # 6. Build argument
        argument = Argument(
            thesis=thesis,
            supporting_evidence=supporting,
            contradicting_evidence=contradicting,
            related_gaps=related_gaps,
            paper_suggestions=self._suggest_sections(contradicting),
        )

        return ArgumentResult(
            topic=thesis,
            argument=argument,
            summary=self._summarize(argument),
            section_guidance=section_guidance,
        )

    def _search_paper_evidence(self, thesis: str) -> List[Evidence]:
        """Search papers for evidence."""
        if not self.db:
            return []

        evidence_list = []
        papers, _ = self.db.search_papers(thesis, limit=10)

        for paper in papers:
            # Extract key claims from paper
            if paper.abstract:
                evidence_list.append(Evidence(
                    evidence_type=EvidenceType.SUPPORT,
                    source=paper.title,
                    content=paper.abstract[:300],
                    citation=f"{paper.title} ({paper.year})" if paper.year else paper.title,
                    weight=0.8,
                ))

        return evidence_list

    def _collect_insight_evidence(self, thesis: str) -> List[Evidence]:
        """Collect user insights as evidence."""
        if not self.insight_manager:
            return []

        evidence_list = []
        cards = self.insight_manager.search_cards(query=thesis, limit=20)

        for card in cards:
            # Determine if insight supports or contradicts
            etype = self._classify_insight(card.content, thesis)

            evidence_list.append(Evidence(
                evidence_type=etype,
                source=f"insight:{card.id}" if hasattr(card, 'id') else "user_insight",
                content=card.content,
                weight=0.6,  # User insights have moderate weight
            ))

        return evidence_list

    def _classify_insight(self, content: str, thesis: str) -> EvidenceType:
        """Classify if insight supports or contradicts thesis."""
        # Simple keyword-based classification
        support_keywords = ["有效", "提升", "改进", "成功", "positive", "improve", "enhance"]
        contradict_keywords = ["局限", "问题", "失败", "缺陷", "limitation", "problem", "fail"]

        content_lower = content.lower()
        thesis_lower = thesis.lower()

        # Check for contradictions
        for kw in contradict_keywords:
            if kw in content_lower:
                return EvidenceType.CONTRADICT

        # Default to support
        return EvidenceType.SUPPORT

    def _find_related_gaps(self, thesis: str) -> List[str]:
        """Find gaps related to the thesis."""
        if not self.gap_analyzer:
            return []

        gaps = []
        try:
            result = self.gap_analyzer.analyze(topic=thesis, use_llm=False)
            gaps = [gap.title for gap in result.gaps[:3]]
        except Exception:
            pass

        return gaps

    def _categorize_evidence(
        self,
        evidence_list: List[Evidence],
    ) -> tuple[List[Evidence], List[Evidence]]:
        """Separate evidence into supporting and contradicting."""
        supporting = []
        contradicting = []

        for e in evidence_list:
            if e.evidence_type in (EvidenceType.SUPPORT, EvidenceType.QUALIFY):
                supporting.append(e)
            else:
                contradicting.append(e)

        # Sort by weight
        supporting.sort(key=lambda x: x.weight, reverse=True)
        contradicting.sort(key=lambda x: x.weight, reverse=True)

        return supporting, contradicting

    def _generate_section_guidance(
        self,
        thesis: str,
        supporting: List[Evidence],
        contradicting: List[Evidence],
        use_llm: bool,
        model: Optional[str],
    ) -> Dict[ArgumentSection, str]:
        """Generate guidance for each paper section."""
        guidance = {}

        if use_llm and LLM_AVAILABLE:
            guidance = self._llm_generate_guidance(
                thesis, supporting, contradicting, model
            )
        else:
            guidance = self._template_guidance(supporting, contradicting)

        return guidance

    def _template_guidance(
        self,
        supporting: List[Evidence],
        contradicting: List[Evidence],
    ) -> Dict[ArgumentSection, str]:
        """Generate guidance from templates."""
        guidance = {}

        guidance[ArgumentSection.INTRODUCTION] = (
            "开篇应明确研究动机：为什么这个问题重要？"
            "引用主要支持证据说明该方向的潜力。"
        )

        guidance[ArgumentSection.RELATED_WORK] = (
            "综述现有工作，区分本文与前人贡献。"
            f"识别 {len(contradicting)} 个需要回应的质疑。"
        )

        guidance[ArgumentSection.METHODOLOGY] = (
            "方法论需针对反驳证据设计消融实验。"
            "说明如何衡量论点成立的条件边界。"
        )

        guidance[ArgumentSection.DISCUSSION] = (
            "承认局限性（尤其是反驳证据指出的）。"
            "解释为什么在特定条件下论点仍然成立。"
        )

        guidance[ArgumentSection.LIMITATION] = (
            "诚实讨论适用边界："
            f"基于 {len(contradicting)} 条反驳证据，"
            "明确指出哪些场景下论点可能不成立。"
        )

        return guidance

    def _llm_generate_guidance(
        self,
        thesis: str,
        supporting: List[Evidence],
        contradicting: List[Evidence],
        model: Optional[str],
    ) -> Dict[ArgumentSection, str]:
        """Use LLM to generate section guidance."""
        prompt = f"""为以下论点生成论文章节建议：

论点：{thesis}

支持证据：{len(supporting)} 条
反驳证据：{len(contradicting)} 条

请为以下章节生成具体建议：
1. Introduction - 如何建立研究动机
2. Related Work - 如何定位本文贡献
3. Methodology - 如何设计实验
4. Discussion - 如何回应质疑
5. Limitation - 如何诚实讨论局限

请用中文回答，每节 2-3 句话。"""

        try:
            response = call_llm_chat_completions(
                messages=[{"role": "user", "content": prompt}],
                model=model or "qwen3.5-plus",
            )
            # Parse response into sections
            return self._parse_guidance(response)
        except Exception:
            return self._template_guidance(supporting, contradicting)

    def _parse_guidance(self, response: str) -> Dict[ArgumentSection, str]:
        """Parse LLM response into section guidance."""
        guidance = {}

        sections = [
            (ArgumentSection.INTRODUCTION, ["introduction", "引言", "动机"]),
            (ArgumentSection.RELATED_WORK, ["related", "相关工作", "贡献"]),
            (ArgumentSection.METHODOLOGY, ["method", "方法", "实验"]),
            (ArgumentSection.DISCUSSION, ["discussion", "讨论", "回应"]),
            (ArgumentSection.LIMITATION, ["limitation", "局限", "边界"]),
        ]

        lines = response.split("\n")
        current_section = None
        current_content = []

        for line in lines:
            line = line.strip()
            if not line:
                continue

            matched = False
            for section, keywords in sections:
                if any(kw in line.lower() for kw in keywords):
                    if current_section and current_content:
                        guidance[current_section] = "\n".join(current_content)
                    current_section = section
                    current_content = [line.split(".", 1)[-1].strip() if "." in line else line]
                    matched = True
                    break

            if not matched and current_section and line:
                current_content.append(line)

        if current_section and current_content:
            guidance[current_section] = "\n".join(current_content)

        # Fill in missing sections with templates
        for section in ArgumentSection:
            if section not in guidance:
                guidance[section] = f"建议在 {section.value} 部分讨论相关内容。"

        return guidance

    def _suggest_sections(
        self,
        contradicting: List[Evidence],
    ) -> List[ArgumentSection]:
        """Suggest which sections need most attention."""
        sections = [ArgumentSection.INTRODUCTION, ArgumentSection.DISCUSSION]

        if contradicting:
            sections.append(ArgumentSection.LIMITATION)

        return sections

    def _summarize(self, argument: Argument) -> str:
        """Generate a summary of the argument."""
        support_count = len(argument.supporting_evidence)
        contradict_count = len(argument.contradicting_evidence)

        return (
            f"论点「{argument.thesis[:50]}...」"
            f"有 {support_count} 条支持证据，"
            f"{contradict_count} 条反驳证据。"
            f"涉及 {len(argument.related_gaps)} 个相关研究空白。"
        )


def render_argument(result: ArgumentResult) -> str:
    """Render argument result as formatted text."""
    lines = []
    arg = result.argument

    lines.append("=" * 70)
    lines.append(f"📝 论点论证")
    lines.append("=" * 70)
    lines.append("")
    lines.append(f"论点：{arg.thesis}")
    lines.append("")

    # Supporting evidence
    lines.append("✅ 支持证据:")
    if arg.supporting_evidence:
        for i, e in enumerate(arg.supporting_evidence[:5], 1):
            lines.append(f"   {i}. [{e.source}]")
            lines.append(f"      {e.content[:80]}...")
    else:
        lines.append("   暂无支持证据")
    lines.append("")

    # Contradicting evidence
    lines.append("❌ 反驳/质疑证据:")
    if arg.contradicting_evidence:
        for i, e in enumerate(arg.contradicting_evidence[:5], 1):
            lines.append(f"   {i}. [{e.source}]")
            lines.append(f"      {e.content[:80]}...")
    else:
        lines.append("   暂无明显反驳证据")
    lines.append("")

    # Related gaps
    if arg.related_gaps:
        lines.append("🔗 相关研究空白:")
        for gap in arg.related_gaps:
            lines.append(f"   • {gap}")
        lines.append("")

    # Section guidance
    if result.section_guidance:
        lines.append("📚 论文章节建议:")
        for section, guidance in result.section_guidance.items():
            section_name = {
                ArgumentSection.INTRODUCTION: "引言",
                ArgumentSection.RELATED_WORK: "相关工作",
                ArgumentSection.METHODOLOGY: "方法论",
                ArgumentSection.EXPERIMENTS: "实验",
                ArgumentSection.DISCUSSION: "讨论",
                ArgumentSection.LIMITATION: "局限",
            }.get(section, section.value)
            lines.append(f"   {section_name}:")
            lines.append(f"      {guidance[:100]}...")
        lines.append("")

    lines.append("=" * 70)
    return "\n".join(lines)
