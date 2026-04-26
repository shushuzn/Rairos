"""Tests for enhanced gap analyzer."""
import pytest
from unittest.mock import MagicMock, patch

from llm.gap_analyzer import (
    GapAnalyzerV2,
    ResearchGapV2,
    GapAnalysisResultV2,
    render_gap_report,
    render_combined_report,
)
from llm.gap_detector import GapType, GapSeverity
from llm.insight_evolution import ExplorationAction


class TestGapAnalyzerV2Init:
    """Test GapAnalyzerV2 initialization."""

    def test_init_with_no_dependencies(self):
        """Test initialization without db or insight_manager."""
        analyzer = GapAnalyzerV2()
        assert analyzer.db is None
        assert analyzer.insight_manager is None

    def test_init_with_db(self):
        """Test initialization with db."""
        mock_db = MagicMock()
        analyzer = GapAnalyzerV2(db=mock_db)
        assert analyzer.db is mock_db
        assert analyzer.insight_manager is None

    def test_init_with_insight_manager(self):
        """Test initialization with insight_manager."""
        mock_db = MagicMock()
        mock_im = MagicMock()
        analyzer = GapAnalyzerV2(db=mock_db, insight_manager=mock_im)
        assert analyzer.db is mock_db
        assert analyzer.insight_manager is mock_im


class TestInsightCollection:
    """Test insight collection."""

    @pytest.fixture
    def analyzer(self):
        return GapAnalyzerV2()

    def test_collect_insights_no_manager(self, analyzer):
        """Test collecting insights when no manager."""
        insights = analyzer._collect_insights("RAG")
        assert insights == []

    def test_collect_insights_with_manager(self, analyzer):
        """Test collecting insights with manager."""
        mock_im = MagicMock()
        mock_card1 = MagicMock()
        mock_card1.content = "RAG improves retrieval"
        mock_card2 = MagicMock()
        mock_card2.content = "Context is key for RAG"
        mock_im.search_cards.return_value = [mock_card1, mock_card2]
        analyzer.insight_manager = mock_im

        insights = analyzer._collect_insights("RAG")
        assert len(insights) == 2
        assert "RAG improves retrieval" in insights
        assert "Context is key for RAG" in insights

    def test_collect_insights_calls_search_with_limit(self, analyzer):
        """Test that insight collection uses query parameter."""
        mock_im = MagicMock()
        mock_im.search_cards.return_value = []
        analyzer.insight_manager = mock_im

        analyzer._collect_insights("test topic")
        mock_im.search_cards.assert_called_once()
        call_kwargs = mock_im.search_cards.call_args[1]
        assert call_kwargs["query"] == "test topic"


class TestSubQuestionGeneration:
    """Test sub-question generation."""

    @pytest.fixture
    def analyzer(self):
        return GapAnalyzerV2()

    def test_method_limitation_questions(self, analyzer):
        """Test questions for method limitation gap."""
        gap = ResearchGapV2(
            gap_type=GapType.METHOD_LIMITATION,
            title="Transformer attention",
            description="High complexity",
            severity=GapSeverity.HIGH,
        )
        questions = analyzer._generate_sub_questions(gap)
        assert len(questions) == 3
        assert any("root causes" in q.lower() for q in questions)
        assert any("alternative" in q.lower() for q in questions)

    def test_unexplored_application_questions(self, analyzer):
        """Test questions for unexplored application gap."""
        gap = ResearchGapV2(
            gap_type=GapType.UNEXPLORED_APPLICATION,
            title="RAG for code",
            description="Not explored",
            severity=GapSeverity.MEDIUM,
        )
        questions = analyzer._generate_sub_questions(gap)
        assert len(questions) == 3

    def test_contradiction_questions(self, analyzer):
        """Test questions for contradiction gap."""
        gap = ResearchGapV2(
            gap_type=GapType.CONTRADICTION,
            title="Conflicting results",
            description="Results differ",
            severity=GapSeverity.HIGH,
        )
        questions = analyzer._generate_sub_questions(gap)
        assert len(questions) == 3

    def test_evaluation_gap_questions(self, analyzer):
        """Test questions for evaluation gap."""
        gap = ResearchGapV2(
            gap_type=GapType.EVALUATION_GAP,
            title="Missing benchmarks",
            description="No standard evaluation",
            severity=GapSeverity.MEDIUM,
        )
        questions = analyzer._generate_sub_questions(gap)
        assert len(questions) == 3
        assert any("metric" in q.lower() for q in questions)

    def test_scalability_issue_questions(self, analyzer):
        """Test questions for scalability gap."""
        gap = ResearchGapV2(
            gap_type=GapType.SCALABILITY_ISSUE,
            title="O(n²) complexity",
            description="Not scalable",
            severity=GapSeverity.HIGH,
        )
        questions = analyzer._generate_sub_questions(gap)
        assert len(questions) == 3
        assert any("scale" in q.lower() for q in questions)

    def test_theoretical_gap_questions(self, analyzer):
        """Test questions for theoretical gap."""
        gap = ResearchGapV2(
            gap_type=GapType.THEORETICAL_GAP,
            title="Missing theory",
            description="No theoretical foundation",
            severity=GapSeverity.LOW,
        )
        questions = analyzer._generate_sub_questions(gap)
        assert len(questions) == 3
        assert any("theoretical" in q.lower() for q in questions)

    def test_dataset_gap_questions(self, analyzer):
        """Test questions for dataset gap."""
        gap = ResearchGapV2(
            gap_type=GapType.DATASET_GAP,
            title="No domain data",
            description="Missing dataset",
            severity=GapSeverity.MEDIUM,
        )
        questions = analyzer._generate_sub_questions(gap)
        assert len(questions) == 3
        assert any("data" in q.lower() for q in questions)

    def test_generalization_gap_questions(self, analyzer):
        """Test questions for generalization gap."""
        gap = ResearchGapV2(
            gap_type=GapType.GENERALIZATION_GAP,
            title="Limited generalization",
            description="Only works on one domain",
            severity=GapSeverity.MEDIUM,
        )
        questions = analyzer._generate_sub_questions(gap)
        assert len(questions) == 3

    def test_unknown_type_questions(self, analyzer):
        """Test fallback for unknown gap type."""
        gap = ResearchGapV2(
            gap_type="unknown",
            title="Unknown gap",
            description="Unknown",
            severity=GapSeverity.LOW,
        )
        questions = analyzer._generate_sub_questions(gap)
        assert len(questions) == 1


class TestPriorityCalculation:
    """Test priority calculation."""

    @pytest.fixture
    def analyzer(self):
        return GapAnalyzerV2()

    def test_high_severity_high_priority(self, analyzer):
        """Test high severity gives high priority."""
        priority = analyzer._calculate_priority(
            paper_count=5,
            insight_count=2,
            severity=GapSeverity.HIGH,
        )
        assert priority >= 300  # base 3*100

    def test_medium_severity_medium_priority(self, analyzer):
        """Test medium severity gives medium priority."""
        priority = analyzer._calculate_priority(
            paper_count=5,
            insight_count=2,
            severity=GapSeverity.MEDIUM,
        )
        assert 200 <= priority < 300

    def test_low_severity_low_priority(self, analyzer):
        """Test low severity gives low priority."""
        priority = analyzer._calculate_priority(
            paper_count=5,
            insight_count=2,
            severity=GapSeverity.LOW,
        )
        assert priority < 200

    def test_more_papers_higher_priority(self, analyzer):
        """Test more evidence papers increases priority."""
        low_papers = analyzer._calculate_priority(
            paper_count=2,
            insight_count=5,
            severity=GapSeverity.MEDIUM,
        )
        high_papers = analyzer._calculate_priority(
            paper_count=10,
            insight_count=5,
            severity=GapSeverity.MEDIUM,
        )
        assert high_papers > low_papers

    def test_fewer_insights_higher_priority(self, analyzer):
        """Test fewer insights = higher novelty bonus."""
        many_insights = analyzer._calculate_priority(
            paper_count=5,
            insight_count=10,
            severity=GapSeverity.MEDIUM,
        )
        few_insights = analyzer._calculate_priority(
            paper_count=5,
            insight_count=2,
            severity=GapSeverity.MEDIUM,
        )
        # Fewer insights = more room to explore = higher priority
        assert few_insights > many_insights


class TestRelatedInsightFinding:
    """Test finding related insights."""

    @pytest.fixture
    def analyzer(self):
        return GapAnalyzerV2()

    def test_find_no_insights(self, analyzer):
        """Test with empty insights list."""
        mock_gap = MagicMock()
        mock_gap.description = "RAG methods are limited"
        result = analyzer._find_related_insights(mock_gap, [])
        assert result == []

    def test_find_with_keyword_match(self, analyzer):
        """Test finding insights with keyword match."""
        mock_gap = MagicMock()
        mock_gap.description = "Transformer attention complexity"

        insights = [
            "Transformers are powerful but have quadratic complexity",
            "RAG improves retrieval quality",
            "Attention mechanisms are key",
        ]
        result = analyzer._find_related_insights(mock_gap, insights)
        # Should match "attention" or "transformer" or "complexity"
        assert len(result) <= 3

    def test_find_limits_results(self, analyzer):
        """Test that results are limited to 3."""
        mock_gap = MagicMock()
        mock_gap.description = "RAG and retrieval"

        insights = [
            "RAG is great",
            "Retrieval matters",
            "Context window",
            "Transformer",
            "Attention",
        ]
        result = analyzer._find_related_insights(mock_gap, insights)
        assert len(result) <= 3


class TestGapReportRendering:
    """Test gap report rendering."""

    def test_render_empty_result(self):
        """Test rendering with no gaps."""
        result = GapAnalysisResultV2(topic="RAG")
        output = render_gap_report(result)
        assert "RAG" in output
        assert "No gaps found" in output

    def test_render_with_gaps(self):
        """Test rendering with gaps."""
        result = GapAnalysisResultV2(
            topic="RAG",
            gaps=[
                ResearchGapV2(
                    gap_type=GapType.METHOD_LIMITATION,
                    title="Attention complexity",
                    description="Transformers have O(n²) complexity",
                    severity=GapSeverity.HIGH,
                    supporting_papers=["paper1", "paper2"],
                    user_insights=["Key insight 1"],
                    sub_questions=["Q1", "Q2"],
                )
            ],
            total_papers_analyzed=10,
            total_insights_used=5,
            gaps_by_type={GapType.METHOD_LIMITATION: 1},
        )
        output = render_gap_report(result)
        assert "RAG" in output
        assert "Attention complexity" in output
        assert "HIGH" in output
        assert "10" in output  # papers analyzed

    def test_render_severity_icons(self):
        """Test severity icons are rendered."""
        result = GapAnalysisResultV2(
            topic="Test",
            gaps=[
                ResearchGapV2(
                    gap_type=GapType.METHOD_LIMITATION,
                    title="High Gap",
                    description="High severity",
                    severity=GapSeverity.HIGH,
                ),
                ResearchGapV2(
                    gap_type=GapType.METHOD_LIMITATION,
                    title="Medium Gap",
                    description="Medium severity",
                    severity=GapSeverity.MEDIUM,
                ),
                ResearchGapV2(
                    gap_type=GapType.METHOD_LIMITATION,
                    title="Low Gap",
                    description="Low severity",
                    severity=GapSeverity.LOW,
                ),
            ],
        )
        output = render_gap_report(result)
        assert "HIGH" in output
        assert "MEDIUM" in output
        assert "LOW" in output


class TestGapConversion:
    """Test gap conversion to V2 format."""

    @pytest.fixture
    def mock_db(self):
        """Create mock database."""
        db = MagicMock()
        return db

    @pytest.fixture
    def analyzer(self, mock_db):
        return GapAnalyzerV2(db=mock_db)

    def test_convert_base_gap(self, analyzer):
        """Test converting base gap to V2."""
        from llm.gap_detector import ResearchGap

        base_gap = ResearchGap(
            gap_type=GapType.METHOD_LIMITATION,
            description="Test gap description",
            evidence_papers=["p1", "p2"],
            severity=GapSeverity.HIGH,
        )

        insights = ["insight 1", "insight 2"]
        papers = ["paper1", "paper2"]

        result, pref_applied = analyzer._convert_to_v2([base_gap], insights, papers)
        assert len(result) == 1
        assert result[0].gap_type == GapType.METHOD_LIMITATION
        assert result[0].severity == GapSeverity.HIGH
        assert result[0].supporting_papers == ["p1", "p2"]
        assert len(result[0].user_insights) >= 0

    def test_convert_multiple_gaps_sorted(self, analyzer):
        """Test that gaps are sorted by severity."""
        from llm.gap_detector import ResearchGap

        gaps = [
            ResearchGap(
                gap_type=GapType.METHOD_LIMITATION,
                description="Low severity gap",
                evidence_papers=["p1"],
                severity=GapSeverity.LOW,
            ),
            ResearchGap(
                gap_type=GapType.METHOD_LIMITATION,
                description="High severity gap",
                evidence_papers=["p1"],
                severity=GapSeverity.HIGH,
            ),
            ResearchGap(
                gap_type=GapType.METHOD_LIMITATION,
                description="Medium severity gap",
                evidence_papers=["p1"],
                severity=GapSeverity.MEDIUM,
            ),
        ]

        result, pref_applied = analyzer._convert_to_v2(gaps, [], [])
        assert len(result) == 3
        # High should be first
        assert result[0].severity == GapSeverity.HIGH
        assert result[1].severity == GapSeverity.MEDIUM
        assert result[2].severity == GapSeverity.LOW

    def test_preference_sorting_boosts_preferred_types(self, analyzer):
        """Test that gaps matching user preferences are boosted."""
        from llm.gap_detector import ResearchGap
        from llm.insight_evolution import EvolutionTracker
        import tempfile
        from pathlib import Path

        # Create tracker with known preferences
        with tempfile.TemporaryDirectory() as tmpdir:
            tracker = EvolutionTracker(data_dir=Path(tmpdir))
            # Record that user prefers method_limitation
            for _ in range(3):
                tracker.record_event(
                    topic="RAG",
                    action=ExplorationAction.ACCEPTED,
                    gap_type="method_limitation",
                )

            analyzer.evolution_tracker = tracker

            gaps = [
                ResearchGap(
                    gap_type=GapType.UNEXPLORED_APPLICATION,
                    description="App gap",
                    evidence_papers=["p1"],
                    severity=GapSeverity.MEDIUM,
                ),
                ResearchGap(
                    gap_type=GapType.METHOD_LIMITATION,
                    description="Method gap",
                    evidence_papers=["p1"],
                    severity=GapSeverity.MEDIUM,
                ),
            ]

            result, pref_applied = analyzer._convert_to_v2(gaps, [], [])
            # Method limitation should be first due to preference
            assert result[0].gap_type == GapType.METHOD_LIMITATION
            assert result[0].preference_boost == True
            assert pref_applied == True


class TestHypothesisGeneration:
    """Test hypothesis generation from gaps."""

    @pytest.fixture
    def analyzer(self):
        return GapAnalyzerV2()

    @pytest.fixture
    def sample_gap_result(self):
        """Create sample gap analysis result."""
        return GapAnalysisResultV2(
            topic="RAG",
            gaps=[
                ResearchGapV2(
                    gap_type=GapType.METHOD_LIMITATION,
                    title="Context window limitation",
                    description="Transformers have limited context window",
                    severity=GapSeverity.HIGH,
                    sub_questions=["How to extend context?", "What are alternatives?"],
                ),
                ResearchGapV2(
                    gap_type=GapType.EVALUATION_GAP,
                    title="Missing benchmarks",
                    description="No standard evaluation for long documents",
                    severity=GapSeverity.MEDIUM,
                    sub_questions=["What metrics to use?", "How to compare?"],
                ),
            ],
            total_papers_analyzed=10,
            total_insights_used=3,
        )

    def test_generate_hypotheses_empty(self, analyzer):
        """Test hypothesis generation with no gaps."""
        empty_result = GapAnalysisResultV2(topic="Test")
        result = analyzer.generate_hypotheses(empty_result, use_llm=False)
        assert result.topic == "Test"
        assert len(result.hypotheses) == 0

    def test_generate_hypotheses_from_gaps(self, analyzer, sample_gap_result):
        """Test hypothesis generation from gap results."""
        with patch("llm.hypothesis_generator.HypothesisGenerator.generate") as mock_gen:
            from llm.hypothesis_generator import HypothesisResult, HypothesisType, ExperimentDesign
            mock_result = HypothesisResult(topic="RAG")
            mock_result.hypotheses = []
            mock_gen.return_value = mock_result

            result = analyzer.generate_hypotheses(sample_gap_result, use_llm=False)
            mock_gen.assert_called_once()

    def test_build_gap_context(self, analyzer, sample_gap_result):
        """Test building context from gap results."""
        context = analyzer._build_gap_context(sample_gap_result)
        assert "RAG" in context
        assert "Context window limitation" in context
        assert "Method Limitation" in context or "method_limitation" in context

    def test_analyze_with_hypotheses(self, analyzer):
        """Test combined analysis method."""
        with patch.object(analyzer, "analyze") as mock_analyze:
            with patch.object(analyzer, "generate_hypotheses") as mock_hypothesis:
                from llm.gap_analyzer import GapAnalysisResultV2
                from llm.hypothesis_generator import HypothesisResult

                mock_analyze.return_value = GapAnalysisResultV2(topic="Test")
                mock_hypothesis.return_value = HypothesisResult(topic="Test")

                gap_result, hyp_result = analyzer.analyze_with_hypotheses("Test", use_llm=False)
                assert gap_result.topic == "Test"
                assert hyp_result.topic == "Test"


class TestCombinedReportRendering:
    """Test combined report rendering."""

    def test_render_combined_empty(self):
        """Test rendering combined report with no gaps."""
        from llm.gap_analyzer import GapAnalysisResultV2
        from llm.hypothesis_generator import HypothesisResult

        gap_result = GapAnalysisResultV2(topic="Test")
        hyp_result = HypothesisResult(topic="Test")

        output = render_combined_report(gap_result, hyp_result)
        assert "Test" in output
        assert "Research Pipeline" in output

    def test_render_combined_with_data(self):
        """Test rendering combined report with data."""
        from llm.hypothesis_generator import HypothesisResult, HypothesisType, ExperimentDesign, ResearchHypothesis

        gap_result = GapAnalysisResultV2(
            topic="RAG",
            gaps=[
                ResearchGapV2(
                    gap_type=GapType.METHOD_LIMITATION,
                    title="Test Gap",
                    description="Test description",
                    severity=GapSeverity.HIGH,
                )
            ],
            total_papers_analyzed=10,
        )

        hyp_result = HypothesisResult(topic="RAG")
        hyp_result.hypotheses = [
            ResearchHypothesis(
                title="Test Hypothesis",
                hypothesis_type=HypothesisType.CAUSAL,
                core_statement="This is a test hypothesis",
                based_on="Test gap",
                experiment_design=ExperimentDesign(
                    baseline="Test",
                    variables=["var1"],
                    controls=["ctrl1"],
                    evaluation_metrics=["metric1"],
                    expected_results="result",
                ),
                novelty_score=0.7,
                feasibility_score=0.8,
            )
        ]

        output = render_combined_report(gap_result, hyp_result)
        assert "RAG" in output
        assert "Test Gap" in output
        assert "Test Hypothesis" in output or "test hypothesis" in output.lower()
        assert "70%" in output  # novelty score
        assert "80%" in output  # feasibility score
