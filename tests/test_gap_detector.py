"""Tests for research gap detector."""
import pytest

from llm.gap_detector import (
    GapDetector,
    GapType,
    GapSeverity,
    ResearchGap,
    ResearchQuestion,
    GapAnalysisResult,
)


class TestGapDetector:
    """Test GapDetector."""

    def test_empty_result_when_no_db(self):
        """Test empty result when no DB available."""
        detector = GapDetector(db=None)
        result = detector.analyze("nonexistent_topic_xyz_123", use_llm=False)

        assert isinstance(result, GapAnalysisResult)
        assert result.topic == "nonexistent_topic_xyz_123"

    def test_gap_severity_enum(self):
        """Test GapSeverity enum values."""
        assert GapSeverity.HIGH.value == "high"
        assert GapSeverity.MEDIUM.value == "medium"
        assert GapSeverity.LOW.value == "low"

    def test_gap_type_enum(self):
        """Test GapType enum values."""
        assert GapType.UNEXPLORED_APPLICATION.value == "unexplored_application"
        assert GapType.METHOD_LIMITATION.value == "method_limitation"
        assert GapType.CONTRADICTION.value == "contradiction"
        assert GapType.EVALUATION_GAP.value == "evaluation_gap"

    def test_research_gap_creation(self):
        """Test ResearchGap dataclass."""
        gap = ResearchGap(
            gap_type=GapType.METHOD_LIMITATION,
            description="Test gap description",
            evidence_papers=["Paper A", "Paper B"],
            severity=GapSeverity.HIGH,
            confidence=0.8,
        )

        assert gap.gap_type == GapType.METHOD_LIMITATION
        assert gap.description == "Test gap description"
        assert len(gap.evidence_papers) == 2
        assert gap.severity == GapSeverity.HIGH
        assert gap.confidence == 0.8

    def test_research_question_creation(self):
        """Test ResearchQuestion dataclass."""
        gap = ResearchGap(
            gap_type=GapType.UNEXPLORED_APPLICATION,
            description="Test",
            evidence_papers=[],
        )
        q = ResearchQuestion(
            question="How to apply X to Y?",
            gap=gap,
            hypothesis="X will work in Y scenario",
            methodology_suggestion="Experiment with different Y configurations",
            expected_impact="Enable new applications",
            feasibility=0.7,
            novelty_score=0.8,
        )

        assert q.question == "How to apply X to Y?"
        assert q.gap == gap
        assert q.hypothesis == "X will work in Y scenario"
        assert q.feasibility == 0.7
        assert q.novelty_score == 0.8

    def test_detect_gaps_rules(self):
        """Test rule-based gap detection."""
        detector = GapDetector()
        paper_text = """
        This method has limitations in scalability.
        However, other approaches show conflicting results.
        Future work should explore potential applications.
        """

        gaps = detector._detect_gaps_rules(paper_text)

        assert len(gaps) > 0
        gap_types = {g.gap_type for g in gaps}
        assert GapType.METHOD_LIMITATION in gap_types

    def test_generate_questions_rules(self):
        """Test rule-based question generation."""
        detector = GapDetector()
        gaps = [
            ResearchGap(
                gap_type=GapType.METHOD_LIMITATION,
                description="Scalability issue",
                evidence_papers=["Paper A"],
            ),
            ResearchGap(
                gap_type=GapType.UNEXPLORED_APPLICATION,
                description="New domain unexplored",
                evidence_papers=["Paper B"],
            ),
        ]

        questions = detector._generate_questions_rules(gaps)

        assert len(questions) > 0
        assert all(isinstance(q, ResearchQuestion) for q in questions)
        assert all(q.gap in gaps for q in questions)

    def test_calculate_coverage(self):
        """Test coverage score calculation."""
        detector = GapDetector()
        papers = [
            {"year": 2024, "abstract": "Good abstract"},
            {"year": 2023, "abstract": "Another abstract"},
            {"year": 2022, "abstract": ""},
        ]

        score = detector._calculate_coverage(papers)
        assert 0 <= score <= 1

    def test_calculate_coverage_empty(self):
        """Test coverage with empty papers."""
        detector = GapDetector()
        score = detector._calculate_coverage([])
        assert score == 0.0

    def test_generate_summary(self):
        """Test summary generation."""
        detector = GapDetector()
        result = GapAnalysisResult(
            topic="Test Topic",
            gaps=[
                ResearchGap(gap_type=GapType.METHOD_LIMITATION, description="Gap 1",
                          evidence_papers=[], severity=GapSeverity.HIGH),
                ResearchGap(gap_type=GapType.UNEXPLORED_APPLICATION, description="Gap 2",
                          evidence_papers=[], severity=GapSeverity.MEDIUM),
            ],
            questions=[
                ResearchQuestion(question="Q1", gap=None),
                ResearchQuestion(question="Q2", gap=None),
            ],
            coverage_score=0.5,
            opportunities_score=2.0,
            analyzed_papers_count=10,
        )

        summary = detector._generate_summary(result)

        assert "高优先级" in summary or "high" in summary.lower()
        assert "2 个" in summary or "2" in summary

    def test_render_result(self):
        """Test result rendering."""
        detector = GapDetector()
        result = GapAnalysisResult(
            topic="Transformer",
            gaps=[
                ResearchGap(
                    gap_type=GapType.METHOD_LIMITATION,
                    description="Scalability limitation",
                    evidence_papers=["Attention Is All You Need"],
                    severity=GapSeverity.HIGH,
                    confidence=0.9,
                ),
            ],
            questions=[
                ResearchQuestion(
                    question="How to scale transformers?",
                    gap=None,
                    hypothesis="Larger models will show better performance",
                ),
            ],
            coverage_score=0.7,
            opportunities_score=3.0,
            analyzed_papers_count=5,
        )

        output = detector.render_result(result)

        assert "Transformer" in output
        assert "METHOD_LIMITATION" not in output  # Should use Chinese
        assert "Scalability limitation" in output
        assert "How to scale transformers?" in output

    def test_render_json(self):
        """Test JSON rendering."""
        import json

        detector = GapDetector()
        result = GapAnalysisResult(
            topic="RLHF",
            gaps=[
                ResearchGap(
                    gap_type=GapType.EVALUATION_GAP,
                    description="No standard benchmark",
                    evidence_papers=["InstructGPT"],
                    severity=GapSeverity.HIGH,
                ),
            ],
            questions=[],
            coverage_score=0.6,
            opportunities_score=2.5,
            analyzed_papers_count=8,
        )

        json_output = detector.render_json(result)
        data = json.loads(json_output)

        assert data["topic"] == "RLHF"
        assert len(data["gaps"]) == 1
        assert data["gaps"][0]["type"] == "evaluation_gap"
        assert data["coverage_score"] == 0.6

    def test_empty_result(self):
        """Test empty result handling."""
        detector = GapDetector()
        result = detector._empty_result("Test Topic")

        assert result.topic == "Test Topic"
        assert len(result.gaps) == 0
        assert len(result.questions) == 0
