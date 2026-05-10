"""Tests for review_simulator, reasoning, code_trace."""

from llm.review_simulator import (
    ReviewDimension,
    Severity,
    ReviewAnnotation,
    ReviewPersona,
    SimulatedReview,
)
from llm.reasoning import ReasoningBlock, _infer_phase
from research_loop.code_trace import ParsedSourceComment, parse_source_comments


class TestReviewEnums:
    def test_dimensions(self):
        assert ReviewDimension.METHODOLOGY is not None
        assert ReviewDimension.NOVELTY_CONTRIBUTION is not None
        assert ReviewDimension.CLARITY_PRESENTATION is not None

    def test_severity(self):
        assert Severity.CRITICAL is not None
        assert Severity.MAJOR is not None
        assert Severity.MINOR is not None


class TestReviewAnnotation:
    def test_fields(self):
        ann = ReviewAnnotation(
            annotation_id="a1",
            dimension=ReviewDimension.CLARITY_PRESENTATION,
            severity=Severity.MINOR,
            location="Section 2",
            headline="Unclear wording",
            comment="The sentence is ambiguous",
        )
        assert ann.dimension == ReviewDimension.CLARITY_PRESENTATION
        assert ann.severity == Severity.MINOR


class TestReviewPersona:
    def test_fields(self):
        persona = ReviewPersona(
            name="Skeptical Reviewer",
            focus="methodology",
            tone="critical",
            priority_instructions="Check for flaws",
        )
        assert persona.name == "Skeptical Reviewer"
        assert persona.tone == "critical"


class TestSimulatedReview:
    def test_fields(self):
        ann = ReviewAnnotation(
            "a1",
            ReviewDimension.NOVELTY_CONTRIBUTION,
            Severity.MAJOR,
            "S1",
            "Headline",
            "Comment",
        )
        review = SimulatedReview(
            review_id="r1",
            persona="Skeptical",
            overall_score=7.5,
            summary="Good paper",
            strengths=["Novel"],
            weaknesses=["Unclear"],
            annotations=[ann],
            recommendation="accept",
        )
        assert review.overall_score == 7.5
        assert len(review.annotations) == 1


class TestReasoningBlock:
    def test_defaults(self):
        block = ReasoningBlock()
        assert block.phase == ""
        assert block.done is False

    def test_fields(self):
        block = ReasoningBlock(phase="analyze", content="Analyzing data...", done=True)
        assert block.phase == "analyze"
        assert block.done is True


class TestInferPhase:
    def test_returns_string(self):
        result = _infer_phase("analyze the experiment results")
        assert isinstance(result, str)


class TestParsedSourceComment:
    def test_fields(self):
        psc = ParsedSourceComment(line_number=42, tags=["TODO"], description="Fix this")
        assert psc.line_number == 42
        assert "TODO" in psc.tags


class TestParseSourceComments:
    def test_empty(self):
        result = parse_source_comments("")
        assert result == []

    def test_no_comments(self):
        code = "def foo():\n    return 1"
        result = parse_source_comments(code)
        assert isinstance(result, list)
