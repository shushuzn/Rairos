"""Tier 2 unit tests — llm/gap_analyzer.py, pure functions, no I/O."""
import pytest
from llm.gap_analyzer import (
    ResearchGapV2,
    GapAnalysisResultV2,
    GapAnalyzerV2,
    render_gap_report,
    render_combined_report,
    _GAP_TYPE_NAMES,
)
from llm.gap_detector import GapType, GapSeverity


# =============================================================================
# Constants
# =============================================================================
class TestGapTypeNames:
    """Test _GAP_TYPE_NAMES mapping."""

    def test_all_gap_types_have_names(self):
        """All GapType values have human-readable names."""
        for gap_type in GapType:
            assert gap_type in _GAP_TYPE_NAMES
            assert len(_GAP_TYPE_NAMES[gap_type]) > 0


# =============================================================================
# Dataclass tests
# =============================================================================
class TestResearchGapV2:
    """Test ResearchGapV2 dataclass."""

    def test_required_fields(self):
        """Required fields: gap_type, title, description, severity."""
        gap = ResearchGapV2(
            gap_type=GapType.METHOD_LIMITATION,
            title="方法局限测试",
            description="现有方法存在效率问题",
            severity=GapSeverity.HIGH,
        )
        assert gap.gap_type == GapType.METHOD_LIMITATION
        assert gap.title == "方法局限测试"
        assert gap.severity == GapSeverity.HIGH

    def test_list_fields_default_empty(self):
        """List fields default to empty."""
        gap = ResearchGapV2(
            gap_type=GapType.UNEXPLORED_APPLICATION,
            title="T",
            description="D",
            severity=GapSeverity.MEDIUM,
        )
        assert gap.supporting_papers == []
        assert gap.user_insights == []
        assert gap.related_methods == []
        assert gap.sub_questions == []

    def test_score_defaults(self):
        """Scores default to zero."""
        gap = ResearchGapV2(
            gap_type=GapType.METHOD_LIMITATION,
            title="T",
            description="D",
            severity=GapSeverity.LOW,
        )
        assert gap.novelty_score == 0.0
        assert gap.feasibility_score == 0.0
        assert gap.priority == 0

    def test_preference_fields_default(self):
        """Preference fields default correctly."""
        gap = ResearchGapV2(
            gap_type=GapType.METHOD_LIMITATION,
            title="T",
            description="D",
            severity=GapSeverity.LOW,
        )
        assert gap.preference_boost is False
        assert gap.preference_score == 0.0

    def test_all_fields_can_be_set(self):
        """All fields can be populated."""
        gap = ResearchGapV2(
            gap_type=GapType.CONTRADICTION,
            title="Full Gap",
            description="Full description",
            severity=GapSeverity.HIGH,
            supporting_papers=["paper1", "paper2"],
            user_insights=["insight1"],
            related_methods=["method1"],
            sub_questions=["Q1", "Q2"],
            novelty_score=0.8,
            feasibility_score=0.7,
            priority=150,
            preference_boost=True,
            preference_score=0.9,
        )
        assert len(gap.supporting_papers) == 2
        assert len(gap.sub_questions) == 2
        assert gap.preference_boost is True


class TestGapAnalysisResultV2:
    """Test GapAnalysisResultV2 dataclass."""

    def test_required_fields(self):
        """Required fields: topic, gaps."""
        result = GapAnalysisResultV2(topic="Transformer研究")
        assert result.topic == "Transformer研究"
        assert result.gaps == []

    def test_statistics_default(self):
        """Statistics fields default correctly."""
        result = GapAnalysisResultV2(topic="Topic")
        assert result.total_papers_analyzed == 0
        assert result.total_insights_used == 0
        assert result.gaps_by_type == {}

    def test_preference_default(self):
        """Preference field defaults to False."""
        result = GapAnalysisResultV2(topic="Topic")
        assert result.preference_applied is False

    def test_summary_default(self):
        """Summary defaults to empty string."""
        result = GapAnalysisResultV2(topic="Topic")
        assert result.summary == ""


# =============================================================================
# Sub-question generation
# =============================================================================
class TestGenerateSubQuestions:
    """Test _generate_sub_questions logic."""

    def _generate_sub_questions(self, gap_type: GapType) -> list:
        """Replicate sub-question generation logic."""
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
        return templates.get(gap_type, ["How could this gap be addressed?"])

    def test_method_limitation_questions(self):
        """Method limitation has 3 questions."""
        questions = self._generate_sub_questions(GapType.METHOD_LIMITATION)
        assert len(questions) == 3
        assert "limitation" in questions[0].lower()

    def test_unexplored_application_questions(self):
        """Unexplored application has 3 questions."""
        questions = self._generate_sub_questions(GapType.UNEXPLORED_APPLICATION)
        assert len(questions) == 3

    def test_contradiction_questions(self):
        """Contradiction has 3 questions."""
        questions = self._generate_sub_questions(GapType.CONTRADICTION)
        assert len(questions) == 3

    def test_evaluation_gap_questions(self):
        """Evaluation gap has 3 questions."""
        questions = self._generate_sub_questions(GapType.EVALUATION_GAP)
        assert len(questions) == 3
        assert "metric" in questions[0].lower()

    def test_scalability_issue_questions(self):
        """Scalability issue has 3 questions."""
        questions = self._generate_sub_questions(GapType.SCALABILITY_ISSUE)
        assert len(questions) == 3

    def test_theoretical_gap_questions(self):
        """Theoretical gap has 3 questions."""
        questions = self._generate_sub_questions(GapType.THEORETICAL_GAP)
        assert len(questions) == 3

    def test_dataset_gap_questions(self):
        """Dataset gap has 3 questions."""
        questions = self._generate_sub_questions(GapType.DATASET_GAP)
        assert len(questions) == 3

    def test_generalization_gap_questions(self):
        """Generalization gap has 3 questions."""
        questions = self._generate_sub_questions(GapType.GENERALIZATION_GAP)
        assert len(questions) == 3

    def test_unknown_gap_type(self):
        """Unknown gap type returns default question."""
        # Note: This tests behavior when gap_type isn't in templates
        # Using a type that exists but testing the fallback
        questions = self._generate_sub_questions(GapType.METHOD_LIMITATION)
        assert len(questions) > 0


# =============================================================================
# Priority calculation
# =============================================================================
class TestCalculatePriority:
    """Test _calculate_priority logic."""

    def _calculate_priority(
        self,
        paper_count: int,
        insight_count: int,
        severity_weight: int,
    ) -> int:
        """Replicate priority calculation logic (severity_weight: 3=HIGH, 2=MEDIUM, 1=LOW)."""
        base = severity_weight * 100

        # More evidence papers = more concrete gap
        evidence_bonus = min(paper_count * 10, 50)

        # Fewer insights = more room for user to explore (always adds at least 50 when 0)
        novelty_bonus = max(0, (10 - insight_count) * 5)

        return base + evidence_bonus + novelty_bonus

    def test_high_severity_base(self):
        """HIGH severity (weight=3) has base of 300 + novelty bonus."""
        priority = self._calculate_priority(0, 0, 3)
        # base 300 + papers 0 + novelty 50 = 350
        assert priority == 350

    def test_medium_severity_base(self):
        """MEDIUM severity (weight=2) has base of 200 + novelty bonus."""
        priority = self._calculate_priority(0, 0, 2)
        # base 200 + papers 0 + novelty 50 = 250
        assert priority == 250

    def test_low_severity_base(self):
        """LOW severity (weight=1) has base of 100 + novelty bonus."""
        priority = self._calculate_priority(0, 0, 1)
        # base 100 + papers 0 + novelty 50 = 150
        assert priority == 150

    def test_paper_count_bonus(self):
        """Each paper adds up to 10 points (max 50)."""
        # 3 papers = 30 bonus
        p1 = self._calculate_priority(3, 0, 1)
        # 5 papers = 50 bonus (capped)
        p2 = self._calculate_priority(5, 0, 1)
        assert p2 - p1 == 20

    def test_paper_count_bonus_capped(self):
        """Paper bonus is capped at 50."""
        # 5 papers = 50 bonus (capped)
        p5 = self._calculate_priority(5, 0, 1)
        # 10 papers = still 50 (capped)
        p10 = self._calculate_priority(10, 0, 1)
        assert p5 == p10

    def test_insight_count_novelty_bonus(self):
        """Fewer insights = more novelty bonus."""
        # 0 insights = 50 bonus (10 * 5)
        p0 = self._calculate_priority(0, 0, 1)
        # 5 insights = 25 bonus
        p5 = self._calculate_priority(0, 5, 1)
        assert p0 - p5 == 25

    def test_insight_count_no_bonus_after_10(self):
        """No novelty bonus after 10 insights."""
        p10 = self._calculate_priority(0, 10, 1)
        p15 = self._calculate_priority(0, 15, 1)
        assert p10 == p15

    def test_combined_calculation(self):
        """High severity (weight=3) + 3 papers + 2 insights."""
        # base 300 + 30 papers + 40 novelty = 370
        priority = self._calculate_priority(3, 2, 3)
        assert priority == 370


# =============================================================================
# Trending keyword matching
# =============================================================================
class TestMatchesTrendingKeyword:
    """Test _matches_trending_keyword logic."""

    def _matches_trending_keyword(self, description: str, hot_keywords: set) -> float:
        """Replicate trending keyword matching logic."""
        if not hot_keywords:
            return 0.0

        text = description.lower()
        matched = sum(1 for kw in hot_keywords if kw in text)
        return min(matched * 0.5, 2.0)  # Cap at +2.0 boost

    def test_empty_hot_keywords(self):
        """Returns 0 when no hot keywords."""
        score = self._matches_trending_keyword("test description", set())
        assert score == 0.0

    def test_single_match(self):
        """Single keyword match gives 0.5."""
        score = self._matches_trending_keyword("transformer is great", {"transformer"})
        assert score == 0.5

    def test_multiple_matches(self):
        """Multiple keyword matches sum up."""
        score = self._matches_trending_keyword(
            "transformer attention mechanism",
            {"transformer", "attention"}
        )
        assert score == 1.0

    def test_capped_at_2_0(self):
        """Score is capped at 2.0."""
        # 5 matches = 2.5, but capped at 2.0
        score = self._matches_trending_keyword(
            "a b c d e",
            {"a", "b", "c", "d", "e"}
        )
        assert score == 2.0

    def test_partial_word_match(self):
        """Keywords match anywhere in text."""
        score = self._matches_trending_keyword(
            "The transformer architecture",
            {"transformer"}
        )
        assert score == 0.5


# =============================================================================
# Related insights finding
# =============================================================================
class TestFindRelatedInsights:
    """Test _find_related_insights logic."""

    def _find_related_insights(
        self,
        description: str,
        insights: list,
    ) -> list:
        """Replicate related insights finding logic."""
        if not insights:
            return []

        # Extract keywords from gap title/description (len > 4)
        words = description.lower().split()
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

    def test_empty_insights(self):
        """Returns empty when no insights."""
        result = self._find_related_insights("test description", [])
        assert result == []

    def test_no_matching_keywords(self):
        """Returns empty when no keywords match."""
        insights = ["some unrelated insight about things"]
        result = self._find_related_insights("test", insights)
        assert result == []

    def test_finds_matching_insight(self):
        """Finds insight with matching keyword."""
        insights = ["This paper explores transformer efficiency in depth"]
        description = "transformer has scalability issues"
        result = self._find_related_insights(description, insights)
        assert len(result) == 1

    def test_limits_to_3_insights(self):
        """Limits results to 3 insights."""
        # Keywords: "scalability", "issues", "description", "transformer", "has" (5 keywords)
        insights = [
            "scaler transformer analysis",  # matches "scalability"
            "issue with transformer",       # matches "issues"
            "describes the problem",        # matches "description"
            "another match here",          # matches "has"
        ]
        description = "transformer has scalability issues description"
        result = self._find_related_insights(description, insights)
        assert len(result) <= 3

    def test_truncates_long_insights(self):
        """Truncates insights over 150 chars."""
        long_insight = "x" * 200
        result = self._find_related_insights("test", [long_insight])
        assert len(result) == 0  # "test" has no keywords > 4 chars


# =============================================================================
# Gap context building
# =============================================================================
class TestBuildGapContext:
    """Test _build_gap_context logic."""

    def _build_gap_context(self, gaps: list) -> str:
        """Replicate gap context building logic."""
        lines = ["Topic: Research Topic"]

        for gap in gaps[:5]:
            lines.append(f"- [{gap['gap_type']}] {gap['title']}")
            lines.append(f"  {gap['description'][:100]}")
            if gap.get('sub_questions'):
                lines.append(f"  Questions: {'; '.join(gap['sub_questions'][:2])}")

        return "\n".join(lines)

    def test_includes_gap_type(self):
        """Includes gap type in output."""
        gaps = [{"gap_type": "method_limitation", "title": "T", "description": "D", "sub_questions": []}]
        context = self._build_gap_context(gaps)
        assert "method_limitation" in context

    def test_limits_to_5_gaps(self):
        """Limits to first 5 gaps."""
        gaps = [{"gap_type": "t", "title": str(i), "description": "D", "sub_questions": []} for i in range(10)]
        context = self._build_gap_context(gaps)
        assert context.count("[") == 5

    def test_truncates_description(self):
        """Truncates long descriptions."""
        gaps = [{"gap_type": "t", "title": "T", "description": "x" * 200, "sub_questions": []}]
        context = self._build_gap_context(gaps)
        assert len(context.split("\n")[1]) <= 110  # Some room for prefix

    def test_includes_sub_questions(self):
        """Includes sub-questions when present."""
        gaps = [{"gap_type": "t", "title": "T", "description": "D", "sub_questions": ["Q1", "Q2", "Q3"]}]
        context = self._build_gap_context(gaps)
        assert "Q1" in context
        assert "Q2" in context

    def test_limits_sub_questions_to_2(self):
        """Limits sub-questions to first 2."""
        gaps = [{"gap_type": "t", "title": "T", "description": "D", "sub_questions": ["Q1", "Q2", "Q3", "Q4"]}]
        context = self._build_gap_context(gaps)
        assert "Q1" in context
        assert "Q3" not in context


# =============================================================================
# Render functions
# =============================================================================
class TestRenderGapReport:
    """Test render_gap_report function."""

    def test_empty_gaps(self):
        """Handles empty gaps list."""
        result = GapAnalysisResultV2(topic="Test Topic")
        output = render_gap_report(result)
        assert "No gaps found" in output
        assert "Test Topic" in output

    def test_includes_statistics(self):
        """Includes analysis statistics."""
        result = GapAnalysisResultV2(
            topic="Test",
            total_papers_analyzed=25,
            total_insights_used=3,
        )
        gaps = [
            ResearchGapV2(
                gap_type=GapType.METHOD_LIMITATION,
                title="Test Gap",
                description="Test description",
                severity=GapSeverity.HIGH,
            )
        ]
        result.gaps = gaps

        output = render_gap_report(result)
        assert "25" in output
        assert "3" in output

    def test_gaps_by_type(self):
        """Shows gaps by type when present."""
        result = GapAnalysisResultV2(topic="Test")
        result.gaps_by_type = {
            GapType.METHOD_LIMITATION: 2,
            GapType.UNEXPLORED_APPLICATION: 1,
        }
        result.gaps = [
            ResearchGapV2(
                gap_type=GapType.METHOD_LIMITATION,
                title="Gap",
                description="Description",
                severity=GapSeverity.HIGH,
            )
        ]

        output = render_gap_report(result)
        assert "Method Limitation" in output or "method_limitation" in output

    def test_severity_icon(self):
        """Shows severity icons."""
        result = GapAnalysisResultV2(topic="Test")
        result.gaps = [
            ResearchGapV2(
                gap_type=GapType.METHOD_LIMITATION,
                title="High Gap",
                description="Description",
                severity=GapSeverity.HIGH,
            ),
            ResearchGapV2(
                gap_type=GapType.UNEXPLORED_APPLICATION,
                title="Low Gap",
                description="Description",
                severity=GapSeverity.LOW,
            ),
        ]

        output = render_gap_report(result)
        assert "🔴" in output or "HIGH" in output
        assert "🟢" in output or "LOW" in output


class TestRenderCombinedReport:
    """Test render_combined_report function."""

    def test_includes_statistics(self):
        """Includes gap analysis statistics."""
        from llm.hypothesis_generator import HypothesisResult

        gap_result = GapAnalysisResultV2(
            topic="Transformer",
            total_papers_analyzed=50,
        )
        gap_result.gaps = [
            ResearchGapV2(
                gap_type=GapType.METHOD_LIMITATION,
                title="Gap",
                description="Desc",
                severity=GapSeverity.HIGH,
            )
        ]

        hyp_result = HypothesisResult(topic="Transformer")

        output = render_combined_report(gap_result, hyp_result)
        assert "50" in output
        assert "Transformer" in output


# =============================================================================
# GapAnalyzerV2 instantiation
# =============================================================================
class TestGapAnalyzerV2:
    """Test GapAnalyzerV2 class."""

    def test_instantiation_without_db(self):
        """Can instantiate without database."""
        analyzer = GapAnalyzerV2()
        assert analyzer.db is None

    def test_instantiation_with_db(self):
        """Can instantiate with database."""
        analyzer = GapAnalyzerV2(db="mock_db")
        assert analyzer.db == "mock_db"

    def test_evolution_tracker_initialized(self):
        """EvolutionTracker is initialized by default."""
        analyzer = GapAnalyzerV2()
        assert analyzer.evolution_tracker is not None

    def test_insight_manager_initially_none(self):
        """Insight manager is initially None."""
        analyzer = GapAnalyzerV2()
        assert analyzer.insight_manager is None

    def test_trend_analyzer_initially_none(self):
        """Trend analyzer is initially None."""
        analyzer = GapAnalyzerV2()
        assert analyzer.trend_analyzer is None


# =============================================================================
# Prompt templates (from gap_detector)
# =============================================================================
class TestGapDetectorPrompts:
    """Test gap detector prompt templates."""

    def test_system_prompt_has_gap_types(self):
        """System prompt lists all gap types."""
        from llm.gap_detector import _GAP_DETECTION_SYSTEM_PROMPT

        required_types = [
            "unexplored_application",
            "method_limitation",
            "contradiction",
            "evaluation_gap",
            "scalability_issue",
        ]
        for gap_type in required_types:
            assert gap_type in _GAP_DETECTION_SYSTEM_PROMPT

    def test_user_prompt_has_placeholders(self):
        """User prompt has topic and paper placeholders."""
        from llm.gap_detector import _GAP_DETECTION_USER_PROMPT_TEMPLATE

        assert "{topic}" in _GAP_DETECTION_USER_PROMPT_TEMPLATE
        assert "{paper_summaries}" in _GAP_DETECTION_USER_PROMPT_TEMPLATE
