"""Enhanced Gap Analyzer: Multi-source gap detection with insights fusion."""
from dataclasses import dataclass, field
from typing import List, Optional, Dict, Any

from llm.gap_detector import (
    GapDetector,
    GapAnalysisResult,
    ResearchGap,
    GapType,
    GapSeverity,
)
from llm.hypothesis_generator import (
    HypothesisGenerator,
    HypothesisResult,
    ResearchHypothesis,
    HypothesisType,
    ExperimentDesign,
)
from llm.insight_evolution import EvolutionTracker


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

    # Preference learning
    preference_boost: bool = False  # True if matches user preferences


@dataclass
class GapAnalysisResultV2:
    """Enhanced analysis result with multi-source context."""
    topic: str
    gaps: List[ResearchGapV2] = field(default_factory=list)

    # Statistics
    total_papers_analyzed: int = 0
    total_insights_used: int = 0
    gaps_by_type: Dict[GapType, int] = field(default_factory=dict)

    # Preference learning applied
    preference_applied: bool = False

    # Summary
    summary: str = ""


class GapAnalyzerV2(GapDetector):
    """Enhanced gap analyzer with insight fusion and preference learning."""

    def __init__(self, db=None, insight_manager=None, evolution_tracker=None,
                 trend_analyzer=None):
        super().__init__(db)
        self.insight_manager = insight_manager
        self.evolution_tracker = evolution_tracker or EvolutionTracker()
        self.trend_analyzer = trend_analyzer

    def _collect_papers(self, topic: str, limit: int = 30) -> List[Dict[str, Any]]:
        """Collect papers with full abstracts for gap analysis.

        Uses search to find relevant papers, then fetches full PaperRecord
        with abstract for deeper analysis.
        """
        if not self.db:
            return []

        # Try multi-word search first
        rows, _ = self.db.search_papers(topic, limit=limit)
        search_results = list(rows)

        # If insufficient results, try searching each word separately
        if len(search_results) < limit and topic.strip():
            seen_ids = {getattr(r, 'paper_id', '') or getattr(r, 'id', '') for r in search_results}
            for word in topic.split():
                if word.strip() and len(search_results) >= limit:
                    break
                word_rows, _ = self.db.search_papers(word.strip(), limit=limit)
                for row in word_rows:
                    pid = getattr(row, 'paper_id', '') or getattr(row, 'id', '')
                    if pid not in seen_ids:
                        seen_ids.add(pid)
                        search_results.append(row)
                        if len(search_results) >= limit:
                            break

        if not search_results:
            return []

        # Fetch full PaperRecord for each result (has abstract)
        paper_ids = [
            getattr(r, 'paper_id', '') or getattr(r, 'id', '')
            for r in search_results
        ]
        paper_records = self.db.get_papers_bulk(paper_ids)

        papers = []
        for row in search_results:
            pid = getattr(row, 'paper_id', '') or getattr(row, 'id', '')
            record = paper_records.get(pid)
            if record:
                papers.append({
                    "id": pid,
                    "title": getattr(record, 'title', '') or getattr(row, 'title', topic),
                    "abstract": getattr(record, 'abstract', '') or '',
                    "year": getattr(record, 'published', '')[:4] if getattr(record, 'published', '') else '',
                    "authors": getattr(record, 'authors', '') or '',
                })
        return papers

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

        # 1. Collect papers using V2 method (with limit support)
        papers = self._collect_papers(topic, limit=30)
        if len(papers) < min_papers:
            return GapAnalysisResultV2(
                topic=topic,
                summary=f"Not enough papers found (need {min_papers}, found {len(papers)})",
            )

        # 2. Collect user insights (NEW)
        insights = []
        if use_insights and self.insight_manager:
            insights = self._collect_insights(topic)

        # 3. Analyze trends for trending keyword boost
        hot_keywords = self._analyze_trends(topic)

        # 4. Run gap detection with collected papers
        # Override parent's paper collection for this call
        original_collect = self._collect_papers
        self._collect_papers = lambda t: papers  # Use same papers

        base_result = super().analyze(
            topic=topic,
            use_llm=use_llm,
            api_key=api_key,
            base_url=base_url,
            model=model,
            min_papers=min_papers,
        )

        self._collect_papers = original_collect  # Restore

        # 5. Convert to enhanced format with insights + trend boost
        enhanced_gaps, preference_applied = self._convert_to_v2(
            base_result.gaps, insights, papers, hot_keywords
        )

        # 6. Generate sub-questions
        for gap in enhanced_gaps:
            gap.sub_questions = self._generate_sub_questions(gap)

        # 7. Calculate statistics
        gaps_by_type = {}
        for gap in enhanced_gaps:
            gaps_by_type[gap.gap_type] = gaps_by_type.get(gap.gap_type, 0) + 1

        return GapAnalysisResultV2(
            topic=topic,
            gaps=enhanced_gaps,
            total_papers_analyzed=len(papers),
            total_insights_used=len(insights),
            gaps_by_type=gaps_by_type,
            preference_applied=preference_applied,
            summary=base_result.summary,
        )

    def _collect_insights(self, topic: str) -> List[str]:
        """Collect relevant user insights."""
        if not self.insight_manager:
            return []

        cards = self.insight_manager.search_cards(query=topic)
        return [card.content for card in cards]

    def _analyze_trends(self, topic: str) -> set:
        """Run TrendAnalyzer to get rising/emerging keywords for a topic.

        Returns a set of keyword strings that are currently rising or emerging.
        """
        if not self.trend_analyzer:
            return set()

        try:
            result = self.trend_analyzer.analyze(topic, min_papers=5)
            # Collect hot keywords from rising and emerging trends
            hot = set()
            for t in result.rising_trends[:10]:
                hot.add(t.keyword.lower())
            for t in result.emerging_trends[:10]:
                hot.add(t.keyword.lower())
            return hot
        except Exception:
            return set()

    def _convert_to_v2(
        self,
        gaps: List[ResearchGap],
        insights: List[str],
        papers: List,
        hot_keywords: set = None,
    ) -> List[ResearchGapV2]:
        """Convert base gaps to enhanced V2 format with preference learning."""
        hot_keywords = hot_keywords or set()
        enhanced = []

        for gap in gaps:
            # Find related insights
            related_insights = self._find_related_insights(gap, insights)

            # Calculate priority with preference boost
            priority = self._calculate_priority(
                len(gap.evidence_papers),
                len(related_insights),
                gap.severity,
                gap.gap_type,
            )

            # Check if gap title/description matches a hot keyword
            trend_boost = self._matches_trending_keyword(gap, hot_keywords)

            enhanced.append(ResearchGapV2(
                gap_type=gap.gap_type,
                title=gap.description[:100] if gap.description else "Untitled Gap",
                description=gap.description,
                severity=gap.severity,
                supporting_papers=gap.evidence_papers,
                user_insights=related_insights,
                priority=priority,
                novelty_score=trend_boost,  # reuse field to carry trend signal
            ))

        # Sort by preference score + trend boost + severity + priority
        enhanced, preference_applied = self._apply_preference_sorting(enhanced, hot_keywords)

        return enhanced, preference_applied

    def _matches_trending_keyword(self, gap: ResearchGap, hot_keywords: set) -> float:
        """Check if a gap matches a trending keyword, return boost score."""
        if not hot_keywords:
            return 0.0

        text = (gap.description or "").lower()
        matched = sum(1 for kw in hot_keywords if kw in text)
        return min(matched * 0.5, 2.0)  # Cap at +2.0 boost

    def _apply_preference_sorting(
        self,
        gaps: List[ResearchGapV2],
        hot_keywords: set = None,
    ) -> List[ResearchGapV2]:
        """Apply user preference-based sorting + trend boost to gaps.

        Gaps matching user preferences or trending keywords are boosted to appear first.
        Returns both sorted gaps and whether preferences were applied.
        """
        hot_keywords = hot_keywords or set()

        # Get user preferences
        preferred_types = set(self.evolution_tracker.get_preferred_gap_types(limit=5))
        disliked_types = set(self.evolution_tracker.get_disliked_gap_types(limit=3))

        has_preferences = bool(preferred_types or disliked_types)

        def gap_preference_score(gap: ResearchGapV2) -> tuple:
            """Calculate sorting score: (trend_score, preference_score, severity_score, priority_score).

            Higher is better. disliked/deprioritized gap types get negative pref_score
            so they sort to the end of their trend/severity tier.
            """
            gap_type_str = gap.gap_type.value

            # Trend score: from novelty_score (set to trend boost in _convert_to_v2)
            trend_score = gap.novelty_score  # 0-2 scale

            # Preference score: +2 for liked, -1 for disliked, 0 for neutral
            pref_score = 0
            if gap_type_str in preferred_types:
                pref_score = 2
                gap.preference_boost = True
            elif gap_type_str in disliked_types or self.evolution_tracker.should_deprioritize_gap_type(gap_type_str):
                pref_score = -2  # stronger penalty than before
                gap.preference_boost = False
            else:
                gap.preference_boost = False

            # Severity score
            severity_order = {GapSeverity.HIGH: 3, GapSeverity.MEDIUM: 2, GapSeverity.LOW: 1}
            severity_score = severity_order.get(gap.severity, 0)

            # Priority score (original calculation)
            priority_score = gap.priority

            # Return tuple: trend first (highest), then preference, then severity, then priority
            return (trend_score, pref_score, severity_score, priority_score)

        gaps.sort(key=gap_preference_score, reverse=True)
        return gaps, has_preferences

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
        gap_type: GapType = None,
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

    # ── Hypothesis Generation ────────────────────────────────

    def generate_hypotheses(
        self,
        gap_result: GapAnalysisResultV2,
        use_llm: bool = True,
        model: Optional[str] = None,
    ) -> HypothesisResult:
        """Generate hypotheses from gap analysis results."""
        from llm.hypothesis_generator import HypothesisGenerator

        if not gap_result.gaps:
            return HypothesisResult(topic=gap_result.topic)

        # Build gap context from analysis
        gap_context = self._build_gap_context(gap_result)

        # Create generator and generate
        generator = HypothesisGenerator(db=self.db)
        result = generator.generate(
            topic=gap_result.topic,
            gap_context=gap_context,
            use_llm=use_llm,
            model=model,
        )

        # Enrich with gap-specific data
        for hypothesis in result.hypotheses[:3]:
            hypothesis.based_on = f"Gap: {gap_result.gaps[0].title[:50]}"

        return result

    def _build_gap_context(self, gap_result: GapAnalysisResultV2) -> str:
        """Build context string from gap results."""
        lines = [f"Topic: {gap_result.topic}"]

        for gap in gap_result.gaps[:5]:
            lines.append(f"- [{gap.gap_type.value}] {gap.title}")
            lines.append(f"  {gap.description[:100]}")
            if gap.sub_questions:
                lines.append(f"  Questions: {'; '.join(gap.sub_questions[:2])}")

        return "\n".join(lines)

    def analyze_with_hypotheses(
        self,
        topic: str,
        use_insights: bool = True,
        min_papers: int = 5,
        use_llm: bool = True,
        model: Optional[str] = None,
    ) -> tuple:
        """Combined analysis: gaps + hypotheses."""
        gap_result = self.analyze(
            topic=topic,
            use_insights=use_insights,
            min_papers=min_papers,
            use_llm=use_llm,
            model=model,
        )

        hypothesis_result = self.generate_hypotheses(
            gap_result, use_llm=use_llm, model=model
        )

        return gap_result, hypothesis_result


def render_gap_report(result: GapAnalysisResultV2, show_preferences: bool = True) -> str:
    """Render gap analysis report.

    Args:
        result: The gap analysis result
        show_preferences: Whether to show preference influence info
    """
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
    lines.append("🎯 Research Gaps (sorted by preference + priority):")
    if show_preferences and hasattr(result, 'preference_applied') and result.preference_applied:
        lines.append("   (✨ = matches your research preferences)")
    lines.append("")

    for i, gap in enumerate(result.gaps, 1):
        severity_icon = {
            GapSeverity.HIGH: "🔴 HIGH",
            GapSeverity.MEDIUM: "🟡 MEDIUM",
            GapSeverity.LOW: "🟢 LOW",
        }.get(gap.severity, "⚪")

        # Preference indicator
        pref_indicator = ""
        if show_preferences and hasattr(gap, 'preference_boost') and gap.preference_boost:
            pref_indicator = " ✨"

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


def render_combined_report(
    gap_result: GapAnalysisResultV2,
    hypothesis_result: HypothesisResult,
) -> str:
    """Render combined gap analysis and hypothesis report."""
    lines = []
    lines.append("=" * 70)
    lines.append(f"🎯 {gap_result.topic} — Research Pipeline")
    lines.append("=" * 70)
    lines.append("")

    # Gap Analysis Summary
    lines.append("📊 Gap Analysis")
    lines.append(f"   Papers analyzed: {gap_result.total_papers_analyzed}")
    lines.append(f"   Gaps identified: {len(gap_result.gaps)}")
    if gap_result.preference_applied:
        boosted = sum(1 for g in gap_result.gaps if getattr(g, 'preference_boost', False))
        boosted_type = gap_result.gaps[0].gap_type.value if gap_result.gaps else "preferred"
        lines.append(f"   🧠 Sorted by your preferences ({boosted} {boosted_type} gaps boosted ✨)")
    lines.append("")

    # Top 3 Gaps
    lines.append("🔍 Top Research Gaps:")
    for i, gap in enumerate(gap_result.gaps[:3], 1):
        severity_icon = {
            GapSeverity.HIGH: "🔴",
            GapSeverity.MEDIUM: "🟡",
            GapSeverity.LOW: "🟢",
        }.get(gap.severity, "⚪")
        boost_marker = " ✨" if getattr(gap, 'preference_boost', False) else ""
        lines.append(f"   {i}. {severity_icon} {gap.title[:60]}{boost_marker}")
    lines.append("")

    # Hypotheses
    lines.append("💡 Research Hypotheses:")
    for i, h in enumerate(hypothesis_result.hypotheses[:3], 1):
        lines.append(f"   {i}. {h.core_statement[:60]}...")
        lines.append(f"      Type: {h.hypothesis_type.value} | "
                    f"Novelty: {h.novelty_score:.0%} | "
                    f"Feasibility: {h.feasibility_score:.0%}")
    lines.append("")

    lines.append("=" * 70)
    return "\n".join(lines)
