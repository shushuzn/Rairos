"""Tests for research pipeline orchestrator."""
import pytest

from llm.pipeline import (
    ResearchPipeline,
    PipelineStage,
    PipelineResult,
)


class TestResearchPipeline:
    """Test ResearchPipeline."""

    def test_pipeline_stage_enum(self):
        """Test PipelineStage enum values."""
        assert PipelineStage.TREND.value == "trend"
        assert PipelineStage.STORY.value == "story"
        assert PipelineStage.VALIDATE.value == "validate"
        assert PipelineStage.HYPOTHESIZE.value == "hypothesize"

    def test_pipeline_result_creation(self):
        """Test PipelineResult dataclass."""
        result = PipelineResult(
            topic="transformer",
            stage=PipelineStage.HYPOTHESIZE,
        )

        assert result.topic == "transformer"
        assert result.stage == PipelineStage.HYPOTHESIZE
        assert result.trend_result is None
        assert result.story_result is None
        assert result.errors == []

    def test_pipeline_result_with_errors(self):
        """Test PipelineResult with errors."""
        result = PipelineResult(
            topic="test",
            stage=PipelineStage.TREND,
            errors=["Trend: No data"],
        )

        assert len(result.errors) == 1
        assert "Trend" in result.errors[0]

    def test_pipeline_initialization(self):
        """Test pipeline initialization."""
        pipeline = ResearchPipeline()
        assert pipeline.db is None

        pipeline = ResearchPipeline(db="mock_db")
        assert pipeline.db == "mock_db"

    def test_run_single_stage(self):
        """Test running pipeline with single stage."""
        pipeline = ResearchPipeline()
        result = pipeline.run(
            topic="test topic",
            stages=[PipelineStage.HYPOTHESIZE],
            use_llm=False,
        )

        assert result.topic == "test topic"
        assert result.stage == PipelineStage.HYPOTHESIZE
        assert result.hypothesis_result is not None
        assert result.trend_result is None

    def test_run_quick_mode(self):
        """Test quick mode reduces paper count."""
        pipeline = ResearchPipeline()
        result = pipeline.run(
            topic="attention",
            stages=[PipelineStage.STORY],
            quick=True,
        )

        # Should complete without errors even in quick mode
        assert result.topic == "attention"

    def test_build_trend_context(self):
        """Test trend context building."""
        pipeline = ResearchPipeline()

        # Mock trend result
        class MockTrend:
            hot_keywords = ["transformer", "attention", "LLM"]
            growth_rate = 0.75

        context = pipeline._build_trend_context(MockTrend())
        assert "transformer" in context
        assert "75%" in context

    def test_build_trend_context_empty(self):
        """Test trend context with no result."""
        pipeline = ResearchPipeline()
        context = pipeline._build_trend_context(None)
        assert context == ""

    def test_build_story_context(self):
        """Test story context building."""
        pipeline = ResearchPipeline()

        # Mock story result
        class MockStory:
            themes = ["scaling", "efficiency", "reasoning"]
            contradictions = [("A", "B"), ("C", "D")]
            summary = "Test summary"

        context = pipeline._build_story_context(MockStory())
        assert "scaling" in context
        assert "2" in context  # 2 contradictions
        assert "Test summary" in context

    def test_build_story_context_empty(self):
        """Test story context with no result."""
        pipeline = ResearchPipeline()
        context = pipeline._build_story_context(None)
        assert context == ""

    def test_build_gap_context(self):
        """Test gap context building."""
        pipeline = ResearchPipeline()

        # Mock validate result
        class MockValidate:
            gap_summary = "Keywords: X, Y, Z"

        context = pipeline._build_gap_context(MockValidate())
        assert "Keywords" in context

    def test_build_gap_context_empty(self):
        """Test gap context with no result."""
        pipeline = ResearchPipeline()
        context = pipeline._build_gap_context(None)
        assert context == ""

    def test_extract_question_from_themes(self):
        """Test question extraction from themes."""
        pipeline = ResearchPipeline()

        class MockStory:
            themes = ["attention mechanism"]

        class MockResult:
            topic = "transformer"
            story_result = MockStory()

        question = pipeline._extract_question(MockResult())
        assert "attention" in question.lower()

    def test_extract_question_fallback(self):
        """Test question extraction fallback."""
        pipeline = ResearchPipeline()

        class MockResult:
            topic = "custom topic"
            story_result = None

        question = pipeline._extract_question(MockResult())
        assert "custom topic" in question

    def test_render_result(self):
        """Test result rendering."""
        pipeline = ResearchPipeline()
        result = PipelineResult(
            topic="test",
            stage=PipelineStage.HYPOTHESIZE,
        )

        output = pipeline.render_result(result)
        assert "test" in output
        assert "Pipeline" in output

    def test_render_trend_summary(self):
        """Test trend summary rendering."""
        pipeline = ResearchPipeline()

        class MockTrend:
            hot_keywords = ["AI", "ML", "DL"]
            growth_rate = 0.5

        lines = pipeline._render_trend(MockTrend())
        assert "趋势分析" in ' '.join(lines)
        assert "AI" in ' '.join(lines)

    def test_render_story_summary(self):
        """Test story summary rendering."""
        pipeline = ResearchPipeline()

        class MockStory:
            themes = ["theme1", "theme2"]
            contradictions = [("a", "b")]

        lines = pipeline._render_story(MockStory())
        assert "研究故事" in ' '.join(lines)
        assert "theme1" in ' '.join(lines)

    def test_render_validation_summary(self):
        """Test validation summary rendering."""
        pipeline = ResearchPipeline()

        class MockScore:
            overall = 7.5

        class MockValidate:
            innovation_score = MockScore()
            is_novel = True

        lines = pipeline._render_validation(MockValidate())
        assert "问题验证" in ' '.join(lines)
        assert "7.5" in ' '.join(lines)

    def test_render_hypothesis_summary(self):
        """Test hypothesis summary rendering."""
        pipeline = ResearchPipeline()

        class MockHypothesis:
            core_statement = "Test hypothesis statement"

        class MockHypResult:
            hypotheses = [MockHypothesis()]

        lines = pipeline._render_hypothesis(MockHypResult())
        assert "研究假说" in ' '.join(lines)
        assert "Test hypothesis" in ' '.join(lines)

    def test_render_hypothesis_empty(self):
        """Test hypothesis rendering with no results."""
        pipeline = ResearchPipeline()

        class MockResult:
            hypotheses = []

        lines = pipeline._render_hypothesis(MockResult())
        assert "无" in lines[1]
