"""Tests for research question validator."""
import pytest

from llm.question_validator import (
    QuestionValidator,
    NoveltyLevel,
    InnovationDimension,
    RelatedWork,
    InnovationScore,
    ValidationResult,
)


class TestQuestionValidator:
    """Test QuestionValidator."""

    def test_empty_result_when_no_db(self):
        """Test empty result when no DB available."""
        validator = QuestionValidator(db=None)
        result = validator.validate("Can transformers learn causal reasoning?", use_llm=False)

        assert isinstance(result, ValidationResult)
        assert result.question == "Can transformers learn causal reasoning?"
        assert result.confidence < 1.0

    def test_novelty_level_enum(self):
        """Test NoveltyLevel enum values."""
        assert NoveltyLevel.HIGH.value == "high"
        assert NoveltyLevel.MEDIUM.value == "medium"
        assert NoveltyLevel.LOW.value == "low"
        assert NoveltyLevel.UNKNOWN.value == "unknown"

    def test_innovation_dimension_enum(self):
        """Test InnovationDimension enum values."""
        assert InnovationDimension.METHOD.value == "method"
        assert InnovationDimension.TASK.value == "task"
        assert InnovationDimension.EVALUATION.value == "evaluation"
        assert InnovationDimension.THEORY.value == "theory"
        assert InnovationDimension.APPLICATION.value == "application"

    def test_related_work_creation(self):
        """Test RelatedWork dataclass."""
        work = RelatedWork(
            paper_id="1234.5678",
            title="Attention Is All You Need",
            year=2017,
            relevance_score=0.9,
            overlap_aspects=["attention mechanism"],
            difference_aspects=["architecture"],
            conclusion="Foundational work",
        )

        assert work.paper_id == "1234.5678"
        assert work.title == "Attention Is All You Need"
        assert work.year == 2017
        assert work.relevance_score == 0.9

    def test_innovation_score_creation(self):
        """Test InnovationScore dataclass."""
        score = InnovationScore(
            overall=7.5,
            method=7.0,
            task=8.0,
            evaluation=7.0,
            dimensions=[InnovationDimension.TASK, InnovationDimension.APPLICATION],
            reasoning="Strong task novelty",
        )

        assert score.overall == 7.5
        assert score.method == 7.0
        assert len(score.dimensions) == 2
        assert "Strong task novelty" in score.reasoning

    def test_expand_question(self):
        """Test question expansion to keywords."""
        validator = QuestionValidator()

        keywords = validator._expand_question("Can transformer models learn causal reasoning?")

        assert len(keywords) > 0
        assert "transformer" in keywords
        assert "reasoning" in keywords
        assert "causal" in keywords

    def test_expand_question_removes_common_words(self):
        """Test that common words are removed."""
        validator = QuestionValidator()

        keywords = validator._expand_question("How to improve the model?")

        assert "how" not in keywords
        assert "to" not in keywords
        assert "the" not in keywords

    def test_analyze_innovation_rules_no_related(self):
        """Test innovation analysis with no related works."""
        validator = QuestionValidator()
        innovation = validator._analyze_innovation_rules([])

        assert innovation.overall >= 7.0
        assert innovation.dimensions  # Should have some dimensions
        assert "未发现相关工作" in innovation.reasoning

    def test_analyze_innovation_rules_high_overlap(self):
        """Test innovation analysis with highly relevant works."""
        validator = QuestionValidator()
        related = [
            RelatedWork(
                paper_id="1", title="Test", year=2024,
                relevance_score=0.9,
                overlap_aspects=[], difference_aspects=[], conclusion=""
            )
        ]
        innovation = validator._analyze_innovation_rules(related)

        assert innovation.overall < 5.0
        assert "高度相关工作" in innovation.reasoning

    def test_analyze_innovation_rules_partial_overlap(self):
        """Test innovation analysis with partial overlap."""
        validator = QuestionValidator()
        related = [
            RelatedWork(
                paper_id="1", title="Test", year=2024,
                relevance_score=0.4,
                overlap_aspects=[], difference_aspects=[], conclusion=""
            )
        ]
        innovation = validator._analyze_innovation_rules(related)

        assert 5.0 <= innovation.overall <= 8.0
        assert "部分相关" in innovation.reasoning

    def test_determine_novelty_high(self):
        """Test novelty determination for high innovation."""
        validator = QuestionValidator()
        innovation = InnovationScore(
            overall=8.0, method=7.0, task=8.0, evaluation=7.0,
            dimensions=[InnovationDimension.TASK], reasoning=""
        )

        novelty = validator._determine_novelty(innovation, [])
        assert novelty == NoveltyLevel.HIGH

    def test_determine_novelty_low(self):
        """Test novelty determination for low innovation."""
        validator = QuestionValidator()
        innovation = InnovationScore(
            overall=3.0, method=3.0, task=3.0, evaluation=3.0,
            dimensions=[], reasoning=""
        )

        novelty = validator._determine_novelty(innovation, [
            RelatedWork(
                paper_id="1", title="Test", year=2024,
                relevance_score=0.9,
                overlap_aspects=[], difference_aspects=[], conclusion=""
            )
        ])
        assert novelty == NoveltyLevel.LOW

    def test_calculate_confidence(self):
        """Test confidence calculation."""
        validator = QuestionValidator()

        related = [
            RelatedWork(paper_id=str(i), title=f"Paper {i}", year=2024,
                       relevance_score=0.8, overlap_aspects=[], difference_aspects=[], conclusion="")
            for i in range(5)
        ]
        innovation = InnovationScore(
            overall=7.0, method=7.0, task=7.0, evaluation=7.0,
            dimensions=[InnovationDimension.METHOD, InnovationDimension.TASK],
            reasoning="Clear analysis"
        )

        confidence = validator._calculate_confidence(related, innovation)
        assert 0.5 < confidence < 0.95

    def test_generate_suggestions_rules_no_related(self):
        """Test suggestion generation with no related works."""
        validator = QuestionValidator()
        suggestions = validator._generate_suggestions_rules([])

        assert len(suggestions) > 0
        assert any("方法" in s for s in suggestions)

    def test_generate_suggestions_rules_with_related(self):
        """Test suggestion generation with related works."""
        validator = QuestionValidator()
        related = [
            RelatedWork(paper_id="1", title="Test", year=2024,
                       relevance_score=0.8, overlap_aspects=[], difference_aspects=[], conclusion="")
        ]
        suggestions = validator._generate_suggestions_rules(related)

        assert len(suggestions) > 0
        assert any("评估" in s or "数据" in s for s in suggestions)

    def test_render_result(self):
        """Test result rendering."""
        validator = QuestionValidator()
        result = ValidationResult(
            question="Can LLMs learn causal reasoning?",
            is_novel=True,
            novelty_level=NoveltyLevel.HIGH,
            innovation_score=InnovationScore(
                overall=7.5,
                method=7.0,
                task=8.0,
                evaluation=7.5,
                dimensions=[InnovationDimension.TASK],
                reasoning="Strong task innovation",
            ),
            related_works=[
                RelatedWork(
                    paper_id="1", title="Causal Reasoning in LLMs", year=2023,
                    relevance_score=0.7,
                    overlap_aspects=["causal"], difference_aspects=["method"], conclusion=""
                )
            ],
            suggestions=[
                "[方法] Design intervention-based evaluation",
                "[数据] Build causal benchmark dataset",
            ],
            confidence=0.8,
        )

        output = validator.render_result(result)

        assert "LLMs learn causal reasoning" in output
        assert "7.5" in output
        assert "Causal Reasoning in LLMs" in output
        assert "✅" in output

    def test_render_json(self):
        """Test JSON rendering."""
        import json

        validator = QuestionValidator()
        result = ValidationResult(
            question="Test question",
            is_novel=True,
            novelty_level=NoveltyLevel.HIGH,
            innovation_score=InnovationScore(
                overall=8.0, method=7.0, task=8.0, evaluation=7.0,
                dimensions=[InnovationDimension.TASK], reasoning="Good"
            ),
            confidence=0.9,
        )

        output = validator.render_json(result)
        data = json.loads(output)

        assert data["question"] == "Test question"
        assert data["is_novel"] is True
        assert data["novelty_level"] == "high"
        assert data["innovation_score"]["overall"] == 8.0
