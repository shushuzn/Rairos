"""Enhanced Gap Analyzer: Multi-source gap detection with insights fusion."""
from dataclasses import dataclass, field
from typing import List, Optional, Dict

from llm.gap_detector import (
    GapDetector,
    GapAnalysisResult,
    ResearchGap,
    GapType,
    GapSeverity,
)


@dataclass
class ResearchGapV2:
    """Enhanced research gap with multi-source evidence."""
    gap_type: GapType
    title: str
    description: str
    severity: GapSeverity

    # Multi-source evidence
    supporting_papers: List[str] = field(default_factory=list)
    user_insights: List[str] = field(default_factory=list)
    related_methods: List[str] = field(default_factory=list)
    sub_questions: List[str] = field(default_factory=list)

    # Scoring
    novelty_score: float = 0.0
    feasibility_score: float = 0.0
    priority: int = 0


@dataclass
class GapAnalysisResultV2:
    """Enhanced analysis result with multi-source context."""
    topic: str
    gaps: List[ResearchGapV2] = field(default_factory=list)

    # Statistics
    total_papers_analyzed: int = 0
    total_insights_used: int = 0
    gaps_by_type: Dict[GapType, int] = field(default_factory=dict)

    # Summary
    summary: str = ""


class GapAnalyzerV2(GapDetector):
    """Enhanced gap analyzer with insight fusion."""

    def __init__(self, db=None, insight_manager=None):
        super().__init__(db)
        self.insight_manager = insight_manager

    def analyze(
        self,
        topic: str,
        use_insights: bool = True,
        min_papers: int = 5,
        use_llm: bool = True,
        api_key: Optional[str] = None,
        base_url: Optional[str] = None,
        model: Optional[str] = None,
    ) -> GapAnalysisResultV2:
        """Enhanced analysis with multi-source evidence."""

        # 1. Collect papers (use parent method)
        papers = self._collect_papers(topic, limit=30)
        if len(papers) < min_papers:
            return GapAnalysisResultV2(
                topic=topic,
                summary=f"Not enough papers found (need {min_papers})",
            )

        # 2. Collect user insights (NEW)
        insights = []
        if use_insights and self.insight_manager:
            insights = self._collect_insights(topic)

        # 3. Run base gap detection
        base_result = super().analyze(
            topic=topic,
            use_llm=use_llm,
            api_key=api_key,
            base_url=base_url,
            model=model,
            min_papers=min_papers,
        )

        # 4. Convert to enhanced format with insights
        enhanced_gaps = self._convert_to_v2(base_result.gaps, insights, papers)

        # 5. Generate sub-questions
        for gap in enhanced_gaps:
            gap.sub_questions = self._generate_sub_questions(gap)

        # 6. Calculate statistics
        gaps_by_type = {}
        for gap in enhanced_gaps:
            gaps_by_type[gap.gap_type] = gaps_by_type.get(gap.gap_type, 0) + 1

        return GapAnalysisResultV2(
            topic=topic,
            gaps=enhanced_gaps,
            total_papers_analyzed=len(papers),
            total_insights_used=len(insights),
            gaps_by_type=gaps_by_type,
            summary=base_result.summary,
        )

    def _collect_insights(self, topic: str) -> List[str]:
        """Collect relevant user insights."""
        if not self.insight_manager:
            return []

        cards = self.insight_manager.search_cards(
            query=topic,
            limit=20,
        )
        return [card.content for card in cards]

    def _convert_to_v2(
        self,
        gaps: List[ResearchGap],
        insights: List[str],
        papers: List,
    ) -> List[ResearchGapV2]:
        """Convert base gaps to enhanced V2 format."""
        enhanced = []

        for gap in gaps:
            # Find related insights
            related_insights = self._find_related_insights(gap, insights)

            # Calculate priority
            priority = self._calculate_priority(
                len(gap.evidence_papers),
                len(related_insights),
                gap.severity,
            )

            enhanced.append(ResearchGapV2(
                gap_type=gap.gap_type,
                title=gap.description[:100] if gap.description else "Untitled Gap",
                description=gap.description,
                severity=gap.severity,
                supporting_papers=gap.evidence_papers,
                user_insights=related_insights,
                priority=priority,
            ))

        # Sort by severity and priority
        severity_order = {GapSeverity.HIGH: 3, GapSeverity.MEDIUM: 2, GapSeverity.LOW: 1}
        enhanced.sort(key=lambda x: severity_order.get(x.severity, 0) * 1000 - x.priority, reverse=True)

        return enhanced

    def _find_related_insights(
        self,
        gap: ResearchGap,
        insights: List[str],
    ) -> List[str]:
        """Find insights related to a gap."""
        if not insights:
            return []

        # Extract keywords from gap title/description
        words = (gap.description or "").lower().split()
        keywords = [w for w in words if len(w) > 4][:5]

        if not keywords:
            return []

        # Simple keyword matching
        related = []
        for insight in insights:
            insight_lower = insight.lower()
            if any(kw in insight_lower for kw in keywords):
                related.append(insight[:150] + "..." if len(insight) > 150 else insight)

        return related[:3]

    def _calculate_priority(
        self,
        paper_count: int,
        insight_count: int,
        severity: GapSeverity,
    ) -> int:
        """Calculate gap priority score."""
        severity_weight = {GapSeverity.HIGH: 3, GapSeverity.MEDIUM: 2, GapSeverity.LOW: 1}
        base = severity_weight.get(severity, 1) * 100

        # More evidence papers = more concrete gap
        evidence_bonus = min(paper_count * 10, 50)

        # Fewer insights = more room for user to explore
        novelty_bonus = max(0, (10 - insight_count) * 5)

        return base + evidence_bonus + novelty_bonus

    def _generate_sub_questions(self, gap: ResearchGapV2) -> List[str]:
        """Generate sub-questions for a gap."""
        templates = {
            GapType.METHOD_LIMITATION: [
                "What are the root causes of this limitation?",
                "What alternative approaches could overcome this?",
                "What trade-offs would those alternatives introduce?",
            ],
            GapType.UNEXPLORED_APPLICATION: [
                "What are the key challenges in applying this to {context}?",
                "What adaptations would be needed?",
                "What would success look like?",
            ],
            GapType.CONTRADICTION: [
                "What explains the contradiction between these findings?",
                "Are there moderating variables at play?",
                "How could this be resolved experimentally?",
            ],
            GapType.EVALUATION_GAP: [
                "What metrics would best capture progress here?",
                "What would a comprehensive benchmark look like?",
                "How could we establish ground truth?",
            ],
            GapType.SCALABILITY_ISSUE: [
                "At what scale does this become problematic?",
                "What is the computational bottleneck?",
                "Are there approximation strategies that could help?",
            ],
            GapType.THEORETICAL_GAP: [
                "What theoretical framework could explain this?",
                "What predictions does theory make?",
                "How could theory be empirically tested?",
            ],
            GapType.DATASET_GAP: [
                "What data would be needed to address this?",
                "Are there proxy datasets that could be used?",
                "What are the data collection challenges?",
            ],
            GapType.GENERALIZATION_GAP: [
                "To what populations/tasks does this currently generalize?",
                "What are the boundaries of applicability?",
                "How could generalization be improved?",
            ],
        }

        return templates.get(gap.gap_type, ["How could this gap be addressed?"])


def render_gap_report(result: GapAnalysisResultV2) -> str:
    """Render gap analysis report."""
    if not result.gaps:
        return f"No gaps found for: {result.topic}"

    lines = []
    lines.append("=" * 70)
    lines.append(f"🔍 {result.topic} — Research Gap Analysis")
    lines.append("=" * 70)
    lines.append("")

    # Statistics
    lines.append("📊 Analysis Summary")
    lines.append(f"   Papers analyzed: {result.total_papers_analyzed}")
    lines.append(f"   User insights used: {result.total_insights_used}")
    lines.append(f"   Gaps identified: {len(result.gaps)}")
    lines.append("")

    # Gap by type
    if result.gaps_by_type:
        type_names = {
            GapType.METHOD_LIMITATION: "Method Limitation",
            GapType.UNEXPLORED_APPLICATION: "Unexplored Application",
            GapType.CONTRADICTION: "Contradiction",
            GapType.EVALUATION_GAP: "Evaluation Gap",
            GapType.SCALABILITY_ISSUE: "Scalability Issue",
            GapType.THEORETICAL_GAP: "Theoretical Gap",
            GapType.DATASET_GAP: "Dataset Gap",
            GapType.GENERALIZATION_GAP: "Generalization Gap",
        }
        lines.append("📈 Gaps by Type:")
        for gtype, count in result.gaps_by_type.items():
            lines.append(f"   {type_names.get(gtype, gtype.value)}: {count}")
        lines.append("")

    # Gap details
    lines.append("🎯 Research Gaps (sorted by priority):")
    lines.append("")

    for i, gap in enumerate(result.gaps, 1):
        severity_icon = {
            GapSeverity.HIGH: "🔴 HIGH",
            GapSeverity.MEDIUM: "🟡 MEDIUM",
            GapSeverity.LOW: "🟢 LOW",
        }.get(gap.severity, "⚪")

        type_name = {
            GapType.METHOD_LIMITATION: "Method Limitation",
            GapType.UNEXPLORED_APPLICATION: "Unexplored Application",
            GapType.CONTRADICTION: "Contradiction",
            GapType.EVALUATION_GAP: "Evaluation Gap",
            GapType.SCALABILITY_ISSUE: "Scalability Issue",
            GapType.THEORETICAL_GAP: "Theoretical Gap",
            GapType.DATASET_GAP: "Dataset Gap",
            GapType.GENERALIZATION_GAP: "Generalization Gap",
        }.get(gap.gap_type, gap.gap_type.value)

        lines.append(f"{i}. {severity_icon} [{type_name}]")
        lines.append(f"   {gap.title}")
        lines.append(f"   {gap.description[:120]}...")

        if gap.user_insights:
            lines.append(f"   💡 Related Insights: {len(gap.user_insights)} found")
            for insight in gap.user_insights[:1]:
                lines.append(f"      → {insight[:80]}...")

        if gap.sub_questions:
            lines.append("   📋 Suggested Questions:")
            for q in gap.sub_questions[:2]:
                lines.append(f"      • {q}")

        lines.append("")

    lines.append("=" * 70)
    return "\n".join(lines)
