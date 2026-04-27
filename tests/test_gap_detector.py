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


# =============================================================================
# _parse_gaps — pure string/regex parsing
# =============================================================================
class TestParseGaps:
    """Test _parse_gaps — pure parsing, no I/O."""

    def test_parses_well_formed_response(self):
        detector = GapDetector()
        response = """
[method_limitation] Transformers have quadratic complexity | Attention Is All You Need | 0.9 | high
[unexplored_application] RAG for code generation not explored | RAG Paper, CodeLlama Paper | 0.7 | medium
"""
        gaps = detector._parse_gaps(response, "Transformers")

        assert len(gaps) == 2
        assert gaps[0].gap_type == GapType.METHOD_LIMITATION
        assert "quadratic" in gaps[0].description.lower()
        assert gaps[0].severity == GapSeverity.HIGH
        assert gaps[1].gap_type == GapType.UNEXPLORED_APPLICATION

    def test_maps_all_gap_types(self):
        detector = GapDetector()
        response = """
[method_limitation] Limitation | Paper1 | 0.8 | high
[unexplored_application] Unexplored | Paper2 | 0.7 | medium
[contradiction] Contradiction | Paper3 | 0.6 | high
[evaluation_gap] No benchmark | Paper4 | 0.5 | medium
"""
        gaps = detector._parse_gaps(response, "Test")
        types = {g.gap_type for g in gaps}
        assert GapType.METHOD_LIMITATION in types
        assert GapType.UNEXPLORED_APPLICATION in types
        assert GapType.CONTRADICTION in types
        assert GapType.EVALUATION_GAP in types

    def test_unknown_type_defaults_to_method_limitation(self):
        detector = GapDetector()
        response = "[unknown_type] Some gap | Paper1 | 0.5 | medium"
        gaps = detector._parse_gaps(response, "Test")
        assert len(gaps) == 1
        assert gaps[0].gap_type == GapType.METHOD_LIMITATION  # default fallback

    def test_parses_evidence_papers(self):
        detector = GapDetector()
        response = "[method_limitation] Gap | Paper A, Paper B, Paper C | 0.8 | high"
        gaps = detector._parse_gaps(response, "Test")
        assert len(gaps) == 1
        assert "Paper A" in gaps[0].evidence_papers
        assert "Paper B" in gaps[0].evidence_papers
        assert "Paper C" in gaps[0].evidence_papers

    def test_empty_response_returns_empty_list(self):
        detector = GapDetector()
        gaps = detector._parse_gaps("", "Test")
        assert gaps == []

    def test_skips_comments_and_empty_lines(self):
        detector = GapDetector()
        response = """
# This is a comment
[method_limitation] Valid gap | Paper1 | 0.8 | high

   # another comment
[unexplored_application] Another | Paper2 | 0.7 | medium
"""
        gaps = detector._parse_gaps(response, "Test")
        assert len(gaps) == 2


# =============================================================================
# _parse_questions — pure string parsing
# =============================================================================
class TestParseQuestions:
    """Test _parse_questions — pure parsing, no I/O."""

    def test_parses_well_formed_response(self):
        detector = GapDetector()
        response = """
How to improve scalability? | Hypothesis text | Use alternative methods | High impact | 0.7 | 0.8
What metrics to use? | Metric hypothesis | Design new metrics | Medium impact | 0.6 | 0.5
"""
        gaps = [ResearchGap(gap_type=GapType.METHOD_LIMITATION, description="Test", evidence_papers=["p1"])]
        questions = detector._parse_questions(response, gaps)

        assert len(questions) == 2
        assert "scalability" in questions[0].question.lower()
        assert "metrics" in questions[1].question.lower()
        assert questions[0].hypothesis == "Hypothesis text"
        assert questions[0].methodology_suggestion == "Use alternative methods"

    def test_parses_minimal_pipe_delimited(self):
        detector = GapDetector()
        response = "Just a question?"
        gaps = [ResearchGap(gap_type=GapType.METHOD_LIMITATION, description="Test", evidence_papers=[])]
        questions = detector._parse_questions(response, gaps)
        assert len(questions) == 1
        assert questions[0].question == "Just a question?"
        assert questions[0].feasibility == 0.5  # default
        assert questions[0].novelty_score == 0.5  # default

    def test_empty_response_returns_empty_list(self):
        detector = GapDetector()
        questions = detector._parse_questions("", [])
        assert questions == []

    def test_skips_comments_and_empty_lines(self):
        detector = GapDetector()
        response = """
# This is a comment

Valid question? | Hypothesis | Method | Impact | 0.7 | 0.8

   # whitespace-only
"""
        gaps = [ResearchGap(gap_type=GapType.METHOD_LIMITATION, description="Test", evidence_papers=[])]
        questions = detector._parse_questions(response, gaps)
        assert len(questions) == 1
        assert questions[0].question == "Valid question?"

    def test_uses_first_gap_when_no_gaps_provided(self):
        """When gaps list is empty, default_gap is None — should not crash."""
        detector = GapDetector()
        response = "Question text"
        questions = detector._parse_questions(response, [])
        assert len(questions) == 1
        assert questions[0].gap is None

    def test_partial_pipe_fields_default(self):
        detector = GapDetector()
        # Two pipes: parts[0]=question, parts[1]=hypothesis, parts[2]=methodology
        response = "Question text | Hypothesis field | Methodology field"
        gaps = [ResearchGap(gap_type=GapType.METHOD_LIMITATION, description="Test", evidence_papers=[])]
        questions = detector._parse_questions(response, gaps)
        assert len(questions) == 1
        assert questions[0].question == "Question text"
        assert questions[0].hypothesis == "Hypothesis field"
        assert questions[0].methodology_suggestion == "Methodology field"
        # With 3 parts, impact/feasibility/novelty get defaults
        assert questions[0].expected_impact == ""
        assert questions[0].feasibility == 0.5
        assert questions[0].novelty_score == 0.5
