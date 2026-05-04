"""Enhanced Gap Analyzer: Multi-source gap detection with insights fusion."""

from dataclasses import dataclass, field
from typing import List, Optional, Dict, Any, Tuple

from llm.gap_detector import (
    GapDetector,
    ResearchGap,
    GapType,
    GapSeverity,
)
from llm.hypothesis_generator import (
    HypothesisGenerator,
    HypothesisResult,
)
from llm.insight_evolution import EvolutionTracker
from llm.text_utils import extract_keywords

# Shared human-readable names for GapType enum values.
_GAP_TYPE_NAMES: Dict[GapType, str] = {
    GapType.METHOD_LIMITATION: "Method Limitation",
    GapType.UNEXPLORED_APPLICATION: "Unexplored Application",
    GapType.CONTRADICTION: "Contradiction",
    GapType.EVALUATION_GAP: "Evaluation Gap",
    GapType.SCALABILITY_ISSUE: "Scalability Issue",
    GapType.THEORETICAL_GAP: "Theoretical Gap",
    GapType.DATASET_GAP: "Dataset Gap",
    GapType.GENERALIZATION_GAP: "Generalization Gap",
}


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
    preference_score: float = 0.0  # Numeric score for display

    # Gene Pool signal — success pattern match from accepted gaps
    gene_pool_score: float = 0.0  # 0.0–1.0, from best matching CapsuleGene

    # Credibility — from best matching CapsuleGene credibility_score
    credibility_score: float = 0.5  # 0.0–1.0


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

    def __init__(self, db=None, insight_manager=None, evolution_tracker=None, trend_analyzer=None):
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
            seen_ids = {getattr(r, "paper_id", "") or getattr(r, "id", "") for r in search_results}
            for word in topic.split():
                if word.strip() and len(search_results) >= limit:
                    break
                word_rows, _ = self.db.search_papers(word.strip(), limit=limit)
                for row in word_rows:
                    pid = getattr(row, "paper_id", "") or getattr(row, "id", "")
                    if pid not in seen_ids:
                        seen_ids.add(pid)
                        search_results.append(row)
                        if len(search_results) >= limit:
                            break

        if not search_results:
            return []

        # Fetch full PaperRecord for each result (has abstract)
        paper_ids = [getattr(r, "paper_id", "") or getattr(r, "id", "") for r in search_results]
        paper_records = self.db.get_papers_bulk(paper_ids)

        papers = []
        for row in search_results:
            pid = getattr(row, "paper_id", "") or getattr(row, "id", "")
            record = paper_records.get(pid)
            if record:
                papers.append(
                    {
                        "id": pid,
                        "title": getattr(record, "title", "") or getattr(row, "title", topic),
                        "abstract": getattr(record, "abstract", "") or "",
                        "year": getattr(record, "published", "")[:4]
                        if getattr(record, "published", "")
                        else "",
                        "authors": getattr(record, "authors", "") or "",
                    }
                )
        return papers

    def analyze(  # type: ignore[override]
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
        self._collect_papers = lambda t, limit=30: papers  # type: ignore[method-assign,misc,assignment]

        base_result = super().analyze(
            topic=topic,
            use_llm=use_llm,
            api_key=api_key,
            base_url=base_url,
            model=model,
            min_papers=min_papers,
        )

        self._collect_papers = original_collect  # type: ignore[method-assign]

        # 5. Convert to enhanced format with insights + trend boost + gene pool signal
        enhanced_gaps, preference_applied = self._convert_to_v2(
            base_result.gaps, insights, papers, hot_keywords, topic
        )

        # 6. Generate sub-questions
        for gap in enhanced_gaps:
            gap.sub_questions = self._generate_sub_questions(gap)

        # 7. Calculate statistics
        gaps_by_type: Dict[GapType, int] = {}
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

    def find_similar_papers_for_gaps(
        self,
        seed_paper_ids: List[str],
        limit: int = 10,
    ) -> List[Dict[str, Any]]:
        """Find papers similar to existing papers for gap expansion.

        Uses vector similarity to discover related papers that might
        have been missed by keyword search. Returns papers with
        full metadata (title, abstract, year, authors).
        """
        if not self.db or not seed_paper_ids:
            return []

        all_similar = []
        seen_ids = set(seed_paper_ids)

        # Find similar papers for each seed paper
        for paper_id in seed_paper_ids[:5]:  # Limit to avoid too many queries
            try:
                similar = self.db.find_similar(paper_id, threshold=0.80, limit=limit)
                for record, score in similar:
                    pid = getattr(record, "id", "") or paper_id
                    if pid not in seen_ids:
                        seen_ids.add(pid)
                        all_similar.append(
                            {
                                "id": pid,
                                "title": getattr(record, "title", "") or "",
                                "abstract": getattr(record, "abstract", "") or "",
                                "year": getattr(record, "published", "")[:4]
                                if getattr(record, "published", "")
                                else "",
                                "authors": getattr(record, "authors", "") or "",
                                "similarity": score,
                                "source": f"similar_to_{paper_id[:8]}",
                            }
                        )
            except Exception:
                continue

        # Sort by similarity and return top results
        all_similar.sort(key=lambda x: x.get("similarity", 0), reverse=True)
        return all_similar[:limit]

    def enrich_papers_with_similar(
        self,
        papers: List[Dict[str, Any]],
        additional_limit: int = 5,
    ) -> List[Dict[str, Any]]:
        """Enrich paper list with vector-similar papers.

        For papers that have embeddings, find semantically similar
        papers to expand the analysis scope beyond keyword matches.
        """
        if not papers or not self.db:
            return papers

        # Get seed paper IDs that have embeddings
        paper_ids = [p["id"] for p in papers if p.get("id")]
        if not paper_ids:
            return papers

        # Find similar papers
        similar = self.find_similar_papers_for_gaps(paper_ids, limit=additional_limit)

        # Merge similar papers into result
        existing_ids = {p["id"] for p in papers}
        for sim_paper in similar:
            if sim_paper["id"] not in existing_ids:
                papers.append(sim_paper)
                existing_ids.add(sim_paper["id"])

        return papers

    def _convert_to_v2(
        self,
        gaps: List[ResearchGap],
        insights: List[str],
        papers: List,
        hot_keywords: set | None = None,
        topic: str = "",
    ) -> Tuple[List[ResearchGapV2], bool]:
        """Convert base gaps to enhanced V2 format with preference learning + Gene Pool signal."""
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

            # Gene Pool lookup: find matching successful capsules
            # This is the "success pattern match" — if user accepted similar gaps before,
            # we boost this gap because it matches their proven interest pattern.
            gene_pool_score = self._get_gene_pool_score(topic, gap)
            credibility_score = self._get_gene_pool_credibility(topic, gap)

            enhanced.append(
                ResearchGapV2(
                    gap_type=gap.gap_type,
                    title=gap.description[:100] if gap.description else "Untitled Gap",
                    description=gap.description,
                    severity=gap.severity,
                    supporting_papers=gap.evidence_papers,
                    user_insights=related_insights,
                    priority=priority,
                    novelty_score=trend_boost,  # reuse field to carry trend signal
                    gene_pool_score=gene_pool_score,
                    credibility_score=credibility_score,
                )
            )

        # Sort by preference score + trend boost + Gene Pool signal + severity + priority
        enhanced, preference_applied = self._apply_preference_sorting(enhanced, hot_keywords)

        return enhanced, preference_applied

    def _get_gene_pool_score(self, topic: str, gap: ResearchGap) -> float:
        """Look up Gene Pool for capsules matching this gap context.

        Returns the success-weighted match score from the best matching CapsuleGene.
        If no match found, returns 0.0 (no boost).
        """
        try:
            gap_keywords = extract_keywords(gap.description or "")
            gap_type_str = (
                gap.gap_type.value if hasattr(gap.gap_type, "value") else str(gap.gap_type)
            )

            capsules = self.evolution_tracker.find_capsule(
                topic=topic,
                gap_type=gap_type_str,
                keywords=gap_keywords,
                min_score=0.1,
            )

            if not capsules:
                return 0.0

            # Best capsule's success score, weighted by match quality
            best = capsules[0]
            match_score = best.trigger_match(topic, gap_type_str, gap_keywords)
            return best.outcome_success_score * match_score
        except Exception:
            return 0.0

    def _get_gene_pool_credibility(self, topic: str, gap: ResearchGap) -> float:
        """Look up Gene Pool and return the credibility score of the best capsule match.

        Returns 0.5 (neutral) if no match or error.
        """
        try:
            gap_keywords = extract_keywords(gap.description or "")
            gap_type_str = (
                gap.gap_type.value if hasattr(gap.gap_type, "value") else str(gap.gap_type)
            )

            capsules = self.evolution_tracker.find_capsule(
                topic=topic,
                gap_type=gap_type_str,
                keywords=gap_keywords,
                min_score=0.1,
            )

            if not capsules:
                return 0.5

            best = capsules[0]
            return getattr(best, "credibility_score", 0.5)
        except Exception:
            return 0.5

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
        hot_keywords: set | None = None,
    ) -> Tuple[List[ResearchGapV2], bool]:
        """Apply user preference-based sorting + Gene Pool signal + trend boost to gaps.

        Gaps matching user preferences (gap_type or keywords) or trending keywords
        are boosted to appear first. Gaps with Gene Pool matches (user accepted
        similar gaps before) are boosted even higher. Returns both sorted gaps and
        whether preferences were applied.
        """
        hot_keywords = hot_keywords or set()

        # Get user preferences
        preferred_types = set(self.evolution_tracker.get_preferred_gap_types(limit=5))
        disliked_types = set(self.evolution_tracker.get_disliked_gap_types(limit=3))
        top_keywords = set(self.evolution_tracker.get_top_keywords(limit=10))

        has_preferences = bool(preferred_types or disliked_types or top_keywords)

        def _extract_gap_keywords(title: str) -> list:
            """Extract research keywords from gap title."""
            return extract_keywords(title)

        def gap_composite_score(gap: ResearchGapV2) -> tuple:
            """Normalized composite score: Gene Pool is primary sort key (0-1 scale).

            Returns (gene_pool_score, trend_normalized, composite).
            - gene_pool_score (0.0-1.0): Gene Pool pattern match — PRIMARY, dominant weight.
            - trend_normalized (0.0-1.0): novelty_score / 2, normalized for comparability.
            - composite: weighted sum; disliked gaps get -0.2 penalty.

            Gene Pool is the dominant signal because it encodes historically successful
            research patterns discovered through user feedback. This makes it the
            primary sort dimension, replacing the previous trend-first tuple that had
            misaligned scales (trend 0-2 dominated gene_pool 0-1).
            """
            gap_type_str = gap.gap_type.value

            # Gene Pool signal (0.0-1.0) — PRIMARY sort dimension
            gene_pool = gap.gene_pool_score

            # Trend normalized from 0-2 scale to 0-1 scale
            trend_normalized = gap.novelty_score / 2.0

            # Preference score: +0.2 liked, -0.2 disliked, 0.0 neutral
            numeric_score = self.evolution_tracker.get_gap_type_score(gap_type_str)
            pref_normalized = 0.0
            if gap_type_str in preferred_types:
                pref_normalized = 0.2
                gap.preference_boost = True
                gap.preference_score = numeric_score
            elif (
                gap_type_str in disliked_types
                or self.evolution_tracker.should_deprioritize_gap_type(gap_type_str)
            ):
                pref_normalized = -0.2
                gap.preference_boost = False
                gap.preference_score = numeric_score
            else:
                gap.preference_boost = False
                gap.preference_score = 0.0

            # Severity normalized (0-1)
            severity_normalized = {
                GapSeverity.HIGH: 1.0,
                GapSeverity.MEDIUM: 0.5,
                GapSeverity.LOW: 0.0,
            }.get(gap.severity, 0.0)

            # Keyword score normalized (0-1): max +3 keyword matches → cap at 1.0
            gap_kws = _extract_gap_keywords(gap.title)
            keyword_normalized = min(
                sum(
                    self.evolution_tracker.get_keyword_score(kw)
                    for kw in gap_kws
                    if kw in top_keywords
                )
                / 3.0,
                1.0,
            )

            # Credibility: from best matching capsule's credibility_score
            # Penalizes gaps backed by trendslop capsules
            credibility = getattr(gap, "credibility_score", 0.5)

            # Composite: Gene Pool is dominant (35%), rest weighted
            composite = (
                0.35 * gene_pool
                + 0.20 * pref_normalized
                + 0.15 * trend_normalized
                + 0.10 * severity_normalized
                + 0.05 * keyword_normalized
                + 0.15 * credibility
            )

            # Primary: gene_pool; Tiebreaker: composite
            return (round(gene_pool, 3), round(composite, 3))

        gaps.sort(key=gap_composite_score, reverse=True)
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
        gap_type: GapType | None = None,
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

        hypothesis_result = self.generate_hypotheses(gap_result, use_llm=use_llm, model=model)

        return gap_result, hypothesis_result


def render_gap_report(result: GapAnalysisResultV2, show_preferences: bool = True) -> str:
    """Render gap analysis report (WarpBlocks Rich output)."""
    from rich.console import Console
    from cli.warp import WarpBlocks

    c = Console()

    if not result.gaps:
        return WarpBlocks.panel("No Results", f"[#8E8E8E]No gaps found for: {result.topic}[/]")

    # Summary stats
    [
        ["Papers Analyzed", f"[#A5D5FE]{result.total_papers_analyzed}[/]"],
        ["Insights Used", f"[#A5D5FE]{result.total_insights_used}[/]"],
        ["Gaps Found", f"[#B4FA72]{len(result.gaps)}[/]"],
    ]

    # Gap by type
    type_rows = []
    if result.gaps_by_type:
        for gtype, count in result.gaps_by_type.items():
            type_rows.append([_GAP_TYPE_NAMES.get(gtype, gtype.value), f"[#A5D5FE]{count}[/]"])

    # Gap details
    gap_rows = []
    for i, gap in enumerate(result.gaps, 1):
        severity_color = {
            GapSeverity.HIGH: "[#FF5555]",
            GapSeverity.MEDIUM: "[#FEFDC2]",
            GapSeverity.LOW: "[#B4FA72]",
        }.get(gap.severity, "[#8E8E8E]")
        sev_label = {
            GapSeverity.HIGH: "🔴",
            GapSeverity.MEDIUM: "🟡",
            GapSeverity.LOW: "🟢",
        }.get(gap.severity, "⚪")
        type_name = _GAP_TYPE_NAMES.get(gap.gap_type, gap.gap_type.value)
        title_short = gap.title[:55]
        boost = " ✨" if getattr(gap, "preference_boost", False) else ""
        gap_rows.append(
            [
                f"[#FEFDC2]{i}.[/]",
                f"{sev_label}{severity_color}{type_name}[/]",
                f"{title_short}{boost}",
            ]
        )
        # Sub-questions as sub-row
        for q in (gap.sub_questions or [])[:2]:
            gap_rows.append(["", "", f"   📋 {q[:65]}..."])

    pref_note = (
        "  ([#B4FA72]✨[/] = matches your preferences)"
        if (show_preferences and getattr(result, "preference_applied", False))
        else ""
    )

    lines = [
        WarpBlocks.panel(
            f"[#FF8272]{result.topic}[/] — Research Gap Analysis",
            "\n".join(
                [
                    f"[#A5D5FE]{len(result.gaps)} gaps[/] · {result.total_papers_analyzed} papers · {result.total_insights_used} insights{pref_note}",
                ]
            ),
            width=75,
        ),
        "",
    ]

    # Capture Rich output using Console.capture()
    with c.capture() as capture:
        if type_rows:
            c.print(WarpBlocks.table(["Gap Type", "Count"], type_rows, title="Gaps by Type"))
            c.print()

        if gap_rows:
            c.print(
                WarpBlocks.table(
                    ["#", "Type", "Gap / Question"],
                    gap_rows,
                    title=f"Research Gaps ({len(result.gaps)})",
                )
            )

    if capture.get():
        lines.append(capture.get().rstrip("\n"))

    return "\n".join(lines)


def render_combined_report(
    gap_result: GapAnalysisResultV2,
    hypothesis_result: HypothesisResult,
) -> str:
    """Render combined gap + hypothesis report (WarpBlocks Rich output)."""
    from rich.console import Console
    from cli.warp import WarpBlocks

    c = Console()

    # Top gaps table
    gap_rows = []
    for i, gap in enumerate(gap_result.gaps[:5], 1):
        sev_icon = {
            GapSeverity.HIGH: "🔴",
            GapSeverity.MEDIUM: "🟡",
            GapSeverity.LOW: "🟢",
        }.get(gap.severity, "⚪")
        boost = " ✨" if getattr(gap, "preference_boost", False) else ""
        gap_rows.append(
            [
                f"[#FEFDC2]{i}.[/]",
                sev_icon,
                f"{gap.title[:58]}{boost}",
            ]
        )

    # Hypotheses table
    hypo_rows = []
    for i, h in enumerate(hypothesis_result.hypotheses[:5], 1):
        hypo_rows.append(
            [
                f"[#FEFDC2]{i}.[/]",
                h.hypothesis_type.value[:15],
                f"[#B4FA72]{h.novelty_score:.0%}[/]",
                f"[#A5D5FE]{h.feasibility_score:.0%}[/]",
                h.core_statement[:38],
            ]
        )

    pref_line = ""
    if gap_result.preference_applied:
        boosted = sum(1 for g in gap_result.gaps if getattr(g, "preference_boost", False))
        pref_line = f"  [#A5D5FE]🧠 {boosted} gaps boosted by preferences ✨[/]"

    parts = [
        WarpBlocks.panel(
            f"[#FF8272]🎯 {gap_result.topic}[/] — Research Pipeline",
            f"[#A5D5FE]{len(gap_result.gaps)} gaps[/] · {gap_result.total_papers_analyzed} papers{pref_line}",
            width=75,
        ),
        "",
    ]

    # Capture Rich output using Console.capture()
    with c.capture() as capture:
        if gap_rows:
            c.print(
                WarpBlocks.table(["#", "", "Top Research Gaps"], gap_rows, title="Gap Analysis")
            )
            c.print()

        if hypo_rows:
            c.print(
                WarpBlocks.table(
                    ["#", "Type", "Novelty", "Feas.", "Hypothesis"],
                    hypo_rows,
                    title=f"Research Hypotheses ({len(hypothesis_result.hypotheses)})",
                )
            )

    if capture.get():
        parts.append(capture.get().rstrip("\n"))
    return "\n".join(parts)
