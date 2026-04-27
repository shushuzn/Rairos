"""Tests for research hypothesis generator."""
import pytest
from unittest.mock import MagicMock

from llm.hypothesis_generator import (
    HypothesisGenerator,
    HypothesisType,
    RiskLevel,
    ExperimentDesign,
    DifferentiationPoint,
    RiskAssessment,
    ResearchHypothesis,
    HypothesisResult,
)


class TestHypothesisGenerator:
    """Test HypothesisGenerator."""

    def test_empty_result_when_no_context(self):
        """Test empty result generation."""
        generator = HypothesisGenerator()
        result = generator.generate(
            topic="transformer",
            gap_context="",
            use_llm=False,
        )

        assert isinstance(result, HypothesisResult)
        assert result.topic == "transformer"
        assert len(result.hypotheses) > 0

    def test_hypothesis_type_enum(self):
        """Test HypothesisType enum values."""
        assert HypothesisType.CAUSAL.value == "causal"
        assert HypothesisType.CORRELATIONAL.value == "correlational"
        assert HypothesisType.COMPARATIVE.value == "comparative"
        assert HypothesisType.MECHANISTIC.value == "mechanistic"
        assert HypothesisType.EXPLORATORY.value == "exploratory"

    def test_risk_level_enum(self):
        """Test RiskLevel enum values."""
        assert RiskLevel.LOW.value == "low"
        assert RiskLevel.MEDIUM.value == "medium"
        assert RiskLevel.HIGH.value == "high"

    def test_experiment_design_creation(self):
        """Test ExperimentDesign dataclass."""
        design = ExperimentDesign(
            baseline="baseline method",
            variables=["var1", "var2"],
            controls=["control1"],
            evaluation_metrics=["accuracy"],
            expected_results="improvement",
        )

        assert design.baseline == "baseline method"
        assert len(design.variables) == 2

    def test_differentiation_point_creation(self):
        """Test DifferentiationPoint dataclass."""
        diff = DifferentiationPoint(
            compared_work="existing method",
            our_advantage="better performance",
            innovation="new approach",
        )

        assert diff.compared_work == "existing method"
        assert diff.innovation == "new approach"

    def test_risk_assessment_creation(self):
        """Test RiskAssessment dataclass."""
        risk = RiskAssessment(
            technical_risk=RiskLevel.MEDIUM,
            hypothesis_risk=RiskLevel.LOW,
            technical_reason="moderate difficulty",
            hypothesis_reason="well defined",
            mitigation=["start small", "checkpoints"],
        )

        assert risk.technical_risk == RiskLevel.MEDIUM
        assert risk.hypothesis_risk == RiskLevel.LOW
        assert len(risk.mitigation) == 2

    def test_research_hypothesis_creation(self):
        """Test ResearchHypothesis dataclass."""
        hypothesis = ResearchHypothesis(
            title="Test Hypothesis",
            hypothesis_type=HypothesisType.CAUSAL,
            core_statement="X causes Y",
            based_on="prior research",
            experiment_design=ExperimentDesign(
                baseline="control",
                variables=["X"],
                controls=["environment"],
                evaluation_metrics=["Y"],
                expected_results="Y changes",
            ),
        )

        assert hypothesis.title == "Test Hypothesis"
        assert hypothesis.hypothesis_type == HypothesisType.CAUSAL
        assert hypothesis.novelty_score == 0.5
        assert hypothesis.feasibility_score == 0.5

    def test_infer_gap_type_limitation(self):
        """Test gap type inference for limitations."""
        generator = HypothesisGenerator()
        context = "This method has scalability limitations"

        gap_type = generator._infer_gap_type(context)
        assert gap_type == "method_limitation"

    def test_infer_gap_type_unexplored(self):
        """Test gap type inference for unexplored areas."""
        generator = HypothesisGenerator()
        context = "future work unexplored"

        gap_type = generator._infer_gap_type(context)
        assert gap_type == "unexplored_application"

    def test_infer_gap_type_contradiction(self):
        """Test gap type inference for contradictions."""
        generator = HypothesisGenerator()
        context = "However, previous results contradict"

        gap_type = generator._infer_gap_type(context)
        assert gap_type == "contradiction"

    def test_infer_gap_type_scalability(self):
        """Test gap type inference for scalability."""
        generator = HypothesisGenerator()
        context = "cannot scale to large datasets"

        gap_type = generator._infer_gap_type(context)
        assert gap_type == "scalability_issue"

    def test_infer_gap_type_evaluation(self):
        """Test gap type inference for evaluation gaps."""
        generator = HypothesisGenerator()
        context = "current benchmarks are insufficient"

        gap_type = generator._infer_gap_type(context)
        assert gap_type == "evaluation_gap"

    def test_infer_gap_type_default(self):
        """Test gap type default fallback."""
        generator = HypothesisGenerator()
        context = "some random text"

        gap_type = generator._infer_gap_type(context)
        assert gap_type == "method_limitation"

    def test_fill_template(self):
        """Test template filling."""
        generator = HypothesisGenerator()
        template = "通过在{method}中引入{improvement}，可以解决现有方法的{limitation}问题"
        topic = "attention"
        context = "transformer has scalability issues"

        filled = generator._fill_template(template, topic, context)
        assert "attention" in filled or "transformer" in filled
        assert "{method}" not in filled
        assert "{improvement}" not in filled

    def test_generate_experiment_design_comparative(self):
        """Test experiment design for comparative hypothesis."""
        generator = HypothesisGenerator()
        hypothesis = ResearchHypothesis(
            title="Compare",
            hypothesis_type=HypothesisType.COMPARATIVE,
            core_statement="A vs B",
            based_on="comparison",
            experiment_design=ExperimentDesign(
                baseline="baseline",
                variables=["method"],
                controls=["data"],
                evaluation_metrics=["accuracy"],
                expected_results="results",
            ),
        )

        design = generator._generate_experiment_design(hypothesis, "test topic")

        assert "相对提升" in design.evaluation_metrics
        assert "统计显著性" in design.evaluation_metrics

    def test_generate_experiment_design_causal(self):
        """Test experiment design for causal hypothesis."""
        generator = HypothesisGenerator()
        hypothesis = ResearchHypothesis(
            title="Causal",
            hypothesis_type=HypothesisType.CAUSAL,
            core_statement="X causes Y",
            based_on="causality",
            experiment_design=ExperimentDesign(
                baseline="baseline",
                variables=["X"],
                controls=["data"],
                evaluation_metrics=["Y"],
                expected_results="Y changes",
            ),
        )

        design = generator._generate_experiment_design(hypothesis, "test")

        assert "消融变量" in design.controls
        assert "干预点" in design.controls

    def test_generate_experiment_design_exploratory(self):
        """Test experiment design for exploratory hypothesis."""
        generator = HypothesisGenerator()
        hypothesis = ResearchHypothesis(
            title="Explore",
            hypothesis_type=HypothesisType.EXPLORATORY,
            core_statement="exploring new domain",
            based_on="exploration",
            experiment_design=ExperimentDesign(
                baseline="baseline",
                variables=["domain"],
                controls=["data"],
                evaluation_metrics=["accuracy"],
                expected_results="results",
            ),
        )

        design = generator._generate_experiment_design(hypothesis, "test")

        assert "可行性" in design.evaluation_metrics
        assert "资源消耗" in design.evaluation_metrics

    def test_assess_risk_high_for_new_domain(self):
        """Test risk assessment for new domain hypothesis."""
        generator = HypothesisGenerator()
        hypothesis = ResearchHypothesis(
            title="New Domain",
            hypothesis_type=HypothesisType.EXPLORATORY,
            core_statement="应用到新领域",
            based_on="exploration",
            experiment_design=ExperimentDesign(
                baseline="",
                variables=[],
                controls=[],
                evaluation_metrics=[],
                expected_results="",
            ),
        )

        risk = generator._assess_risk(hypothesis)

        assert risk.hypothesis_risk == RiskLevel.HIGH
        assert risk.technical_risk == RiskLevel.HIGH

    def test_assess_risk_medium_default(self):
        """Test default risk assessment."""
        generator = HypothesisGenerator()
        hypothesis = ResearchHypothesis(
            title="Normal",
            hypothesis_type=HypothesisType.COMPARATIVE,
            core_statement="compare A and B",
            based_on="comparison",
            experiment_design=ExperimentDesign(
                baseline="",
                variables=[],
                controls=[],
                evaluation_metrics=[],
                expected_results="",
            ),
        )

        risk = generator._assess_risk(hypothesis)

        assert risk.technical_risk == RiskLevel.MEDIUM
        assert risk.hypothesis_risk == RiskLevel.MEDIUM

    def test_calculate_novelty_exploratory(self):
        """Test novelty calculation for exploratory hypothesis."""
        generator = HypothesisGenerator()
        hypothesis = ResearchHypothesis(
            title="Explore",
            hypothesis_type=HypothesisType.EXPLORATORY,
            core_statement="exploring",
            based_on="",
            experiment_design=ExperimentDesign(
                baseline="",
                variables=[],
                controls=[],
                evaluation_metrics=[],
                expected_results="",
            ),
        )

        score = generator._calculate_novelty(hypothesis, has_trend=True, has_story=True)

        assert score > 0.5

    def test_calculate_novelty_cross_domain(self):
        """Test novelty calculation for cross-domain hypothesis."""
        generator = HypothesisGenerator()
        hypothesis = ResearchHypothesis(
            title="Cross",
            hypothesis_type=HypothesisType.EXPLORATORY,
            core_statement="跨领域创新",
            based_on="",
            experiment_design=ExperimentDesign(
                baseline="",
                variables=[],
                controls=[],
                evaluation_metrics=[],
                expected_results="",
            ),
        )

        score = generator._calculate_novelty(hypothesis, has_trend=False, has_story=False)

        assert score >= 0.8

    def test_calculate_feasibility(self):
        """Test feasibility calculation."""
        generator = HypothesisGenerator()
        hypothesis = ResearchHypothesis(
            title="Test",
            hypothesis_type=HypothesisType.COMPARATIVE,
            core_statement="test",
            based_on="",
            experiment_design=ExperimentDesign(
                baseline="",
                variables=["v1", "v2", "v3", "v4", "v5", "v6"],
                controls=[],
                evaluation_metrics=[],
                expected_results="",
            ),
        )

        score = generator._calculate_feasibility(hypothesis)

        assert score < 0.6

    def test_generate_summary(self):
        """Test summary generation."""
        generator = HypothesisGenerator()

        hypotheses = [
            ResearchHypothesis(
                title="High Feas",
                hypothesis_type=HypothesisType.COMPARATIVE,
                core_statement="compare",
                based_on="",
                experiment_design=ExperimentDesign(
                    baseline="",
                    variables=[],
                    controls=[],
                    evaluation_metrics=[],
                    expected_results="",
                ),
                novelty_score=0.7,
                feasibility_score=0.7,
            ),
        ]

        result = HypothesisResult(
            topic="test",
            hypotheses=hypotheses,
        )

        summary = generator._generate_summary(result)

        assert "test" in summary.lower() or "假说" in summary
        assert "可行" in summary or "feasibility" in summary.lower()

    def test_generate_summary_empty(self):
        """Test empty summary."""
        generator = HypothesisGenerator()
        result = HypothesisResult(topic="empty")

        summary = generator._generate_summary(result)

        assert "无法" in summary or "empty" in summary.lower()

    def test_render_result(self):
        """Test result rendering."""
        generator = HypothesisGenerator()
        result = HypothesisResult(
            topic="Transformer",
            hypotheses=[
                ResearchHypothesis(
                    title="Attention Hypothesis",
                    hypothesis_type=HypothesisType.CAUSAL,
                    core_statement="Attention causes improvement",
                    based_on="prior research",
                    experiment_design=ExperimentDesign(
                        baseline="baseline",
                        variables=["attention"],
                        controls=["data"],
                        evaluation_metrics=["accuracy"],
                        expected_results="improvement",
                    ),
                    novelty_score=0.7,
                    feasibility_score=0.6,
                    risk_assessment=RiskAssessment(
                        technical_risk=RiskLevel.MEDIUM,
                        hypothesis_risk=RiskLevel.LOW,
                        technical_reason="moderate",
                        hypothesis_reason="well defined",
                        mitigation=["checkpoints"],
                    ),
                ),
            ],
        )

        output = generator.render_result(result)

        assert "Transformer" in output
        assert "Attention Hypothesis" in output
        assert "CAUSAL" in output or "causal" in output
        assert "70%" in output or "0.7" in output

    def test_render_json(self):
        """Test JSON rendering."""
        generator = HypothesisGenerator()
        result = HypothesisResult(
            topic="BERT",
            hypotheses=[
                ResearchHypothesis(
                    title="BERT Hypothesis",
                    hypothesis_type=HypothesisType.COMPARATIVE,
                    core_statement="BERT vs GPT",
                    based_on="comparison",
                    experiment_design=ExperimentDesign(
                        baseline="GPT",
                        variables=["pre-training"],
                        controls=["data"],
                        evaluation_metrics=["accuracy"],
                        expected_results="better",
                    ),
                ),
            ],
        )

        output = generator.render_json(result)

        assert '"topic"' in output
        assert '"hypotheses"' in output
        assert "BERT Hypothesis" in output


# =============================================================================
# _generate_from_templates — pure, no I/O
# =============================================================================
class TestGenerateFromTemplates:
    """Test _generate_from_templates — pure template-filling, no LLM or DB."""

    def _make_hypothesis(self, **kwargs):
        defaults = dict(
            id="test-id",
            title="Test",
            hypothesis_type=HypothesisType.CAUSAL,
            core_statement="X causes Y",
            based_on="unit test",
            experiment_design=ExperimentDesign(
                baseline="", variables=[], controls=[],
                evaluation_metrics=[], expected_results="",
            ),
        )
        defaults.update(kwargs)
        return ResearchHypothesis(**defaults)

    def test_returns_list_of_hypotheses(self):
        generator = HypothesisGenerator()
        result = generator._generate_from_templates(
            topic="Transformer",
            gap_context="scalability issues",
            trend_context="",
            creative=False,
        )
        assert isinstance(result, list)
        assert len(result) > 0

    def test_hypothesis_has_core_statement(self):
        generator = HypothesisGenerator()
        results = generator._generate_from_templates(
            topic="Attention",
            gap_context="quadratic complexity",
            trend_context="",
            creative=False,
        )
        for h in results:
            assert h.core_statement
            assert len(h.core_statement) > 5

    def test_hypothesis_type_from_template(self):
        generator = HypothesisGenerator()
        results = generator._generate_from_templates(
            topic="RAG",
            gap_context="method_limitation",
            trend_context="",
            creative=False,
        )
        for h in results:
            assert h.hypothesis_type in list(HypothesisType)

    def test_creative_flag_adds_one(self):
        generator = HypothesisGenerator()
        no_creative = generator._generate_from_templates(
            topic="Test",
            gap_context="scalability issues",
            trend_context="",
            creative=False,
        )
        with_creative = generator._generate_from_templates(
            topic="Test",
            gap_context="scalability issues",
            trend_context="",
            creative=True,
        )
        # Creative=True should produce at least as many results
        assert len(with_creative) >= len(no_creative)

    def test_gap_type_inferred_and_stored(self):
        generator = HypothesisGenerator()
        results = generator._generate_from_templates(
            topic="Test",
            gap_context="scalability issues with the method",
            trend_context="",
            creative=False,
        )
        for h in results:
            assert h.gap_type is not None


# =============================================================================
# _find_differentiations — requires db (graceful fallback)
# =============================================================================
class TestFindDifferentiations:
    """Test _find_differentiations — graceful empty list when no DB."""

    def test_returns_empty_when_no_db(self):
        generator = HypothesisGenerator()
        hypothesis = ResearchHypothesis(
            title="Test",
            hypothesis_type=HypothesisType.CAUSAL,
            core_statement="X causes Y",
            based_on="test",
            experiment_design=ExperimentDesign(
                baseline="", variables=[], controls=[],
                evaluation_metrics=[], expected_results="",
            ),
        )
        diffs = generator._find_differentiations(hypothesis, "transformer")
        assert diffs == []

    def test_returns_empty_when_db_raises(self):
        generator = HypothesisGenerator()
        mock_db = MagicMock()
        mock_db.search_papers.side_effect = RuntimeError("search failed")
        generator.db = mock_db
        hypothesis = ResearchHypothesis(
            title="Test",
            hypothesis_type=HypothesisType.CAUSAL,
            core_statement="X causes Y",
            based_on="test",
            experiment_design=ExperimentDesign(
                baseline="", variables=[], controls=[],
                evaluation_metrics=[], expected_results="",
            ),
        )
        diffs = generator._find_differentiations(hypothesis, "transformer")
        # Should not raise — graceful fallback
        assert diffs == []
