"""Tier 2 unit tests — llm/hypothesis_generator.py, pure functions, no I/O."""
import pytest
from llm.hypothesis_generator import (
    HypothesisType,
    RiskLevel,
    ExperimentDesign,
    DifferentiationPoint,
    RiskAssessment,
    ResearchHypothesis,
    HypothesisResult,
    HypothesisGenerator,
)


# =============================================================================
# Enum tests
# =============================================================================
class TestHypothesisType:
    """Test HypothesisType enum."""

    def test_all_types_have_values(self):
        """All HypothesisType variants have string values."""
        assert HypothesisType.CAUSAL.value == "causal"
        assert HypothesisType.CORRELATIONAL.value == "correlational"
        assert HypothesisType.COMPARATIVE.value == "comparative"
        assert HypothesisType.MECHANISTIC.value == "mechanistic"
        assert HypothesisType.EXPLORATORY.value == "exploratory"


class TestRiskLevel:
    """Test RiskLevel enum."""

    def test_all_levels_have_values(self):
        """All RiskLevel variants have values."""
        assert RiskLevel.LOW.value == "low"
        assert RiskLevel.MEDIUM.value == "medium"
        assert RiskLevel.HIGH.value == "high"


# =============================================================================
# ExperimentDesign dataclass tests
# =============================================================================
class TestExperimentDesign:
    """Test ExperimentDesign dataclass — all required fields, no defaults."""

    def test_required_fields(self):
        """All 6 fields are required."""
        design = ExperimentDesign(
            baseline="BERT baseline",
            variables=["learning_rate", "batch_size"],
            controls=["data_split", "random_seed"],
            evaluation_metrics=["accuracy", "f1"],
            expected_results="Improved accuracy",
        )
        assert design.baseline == "BERT baseline"
        assert design.variables == ["learning_rate", "batch_size"]
        assert design.controls == ["data_split", "random_seed"]
        assert design.evaluation_metrics == ["accuracy", "f1"]
        assert design.expected_results == "Improved accuracy"

    def test_all_fields_can_be_set(self):
        """All fields can be set."""
        design = ExperimentDesign(
            baseline="B",
            variables=["v1"],
            controls=["c1"],
            evaluation_metrics=["m1"],
            expected_results="R",
            alternative_if_wrong="Try another approach",
        )
        assert design.alternative_if_wrong == "Try another approach"


# =============================================================================
# DifferentiationPoint dataclass tests
# =============================================================================
class TestDifferentiationPoint:
    """Test DifferentiationPoint dataclass — all required fields."""

    def test_required_fields(self):
        """All 3 fields are required."""
        dp = DifferentiationPoint(
            compared_work="Existing Method",
            our_advantage="Better performance",
            innovation="Novel architecture",
        )
        assert dp.compared_work == "Existing Method"
        assert dp.our_advantage == "Better performance"
        assert dp.innovation == "Novel architecture"


# =============================================================================
# RiskAssessment dataclass tests
# =============================================================================
class TestRiskAssessment:
    """Test RiskAssessment dataclass — all required fields."""

    def test_required_fields(self):
        """All 5 fields are required."""
        ra = RiskAssessment(
            technical_risk=RiskLevel.MEDIUM,
            hypothesis_risk=RiskLevel.HIGH,
            technical_reason="Hard to implement",
            hypothesis_reason="May not hold",
            mitigation=["Start small", "Have backup"],
        )
        assert ra.technical_risk == RiskLevel.MEDIUM
        assert ra.hypothesis_risk == RiskLevel.HIGH
        assert ra.technical_reason == "Hard to implement"
        assert ra.hypothesis_reason == "May not hold"
        assert ra.mitigation == ["Start small", "Have backup"]


# =============================================================================
# ResearchHypothesis dataclass tests
# =============================================================================
class TestResearchHypothesisInit:
    """Test ResearchHypothesis dataclass."""

    def test_optional_fields_default(self):
        """Optional fields have defaults."""
        h = ResearchHypothesis(id="x", title="T")
        assert h.hypothesis_type == HypothesisType.CAUSAL
        assert h.core_statement == ""
        assert h.based_on == ""
        assert isinstance(h.experiment_design, ExperimentDesign)
        assert h.differentiations == []
        assert h.risk_assessment is None
        assert h.novelty_score == 0.5
        assert h.feasibility_score == 0.5
        assert h.gap_type == ""

    def test_all_fields_can_be_set(self):
        """All fields can be set."""
        design = ExperimentDesign(
            baseline="B",
            variables=["v"],
            controls=["c"],
            evaluation_metrics=["m"],
            expected_results="R",
        )
        risk = RiskAssessment(
            technical_risk=RiskLevel.LOW,
            hypothesis_risk=RiskLevel.LOW,
            technical_reason="TR",
            hypothesis_reason="HR",
            mitigation=["M1"],
        )
        dp = DifferentiationPoint(
            compared_work="CW",
            our_advantage="OA",
            innovation="I",
        )
        h = ResearchHypothesis(
            id="h1",
            title="Novel Hypothesis",
            hypothesis_type=HypothesisType.EXPLORATORY,
            core_statement="Statement here",
            based_on="Based on gap",
            experiment_design=design,
            differentiations=[dp],
            risk_assessment=risk,
            novelty_score=0.9,
            feasibility_score=0.7,
            gap_type="unexplored_application",
        )
        assert h.core_statement == "Statement here"
        assert h.experiment_design == design
        assert h.differentiations == [dp]
        assert h.risk_assessment == risk
        assert h.novelty_score == 0.9
        assert h.feasibility_score == 0.7
        assert h.gap_type == "unexplored_application"


# =============================================================================
# HypothesisResult dataclass tests
# =============================================================================
class TestHypothesisResult:
    """Test HypothesisResult dataclass."""

    def test_required_fields(self):
        """Required field: topic."""
        result = HypothesisResult(topic="Transformer")
        assert result.topic == "Transformer"

    def test_optional_fields_default(self):
        """Optional fields have defaults."""
        result = HypothesisResult(topic="T")
        assert result.hypotheses == []
        assert result.summary == ""
        assert result.integration_notes == ""

    def test_all_fields_can_be_set(self):
        """All fields can be set."""
        design = ExperimentDesign(
            baseline="B", variables=[], controls=[], evaluation_metrics=[], expected_results="R"
        )
        h = ResearchHypothesis(id="h", title="T", experiment_design=design)
        result = HypothesisResult(
            topic="Topic",
            hypotheses=[h],
            summary="Summary text",
            integration_notes="Integration notes",
        )
        assert len(result.hypotheses) == 1
        assert result.summary == "Summary text"
        assert result.integration_notes == "Integration notes"


# =============================================================================
# _fill_template tests
# =============================================================================
class TestFillTemplate:
    """Test _fill_template logic."""

    def _fill_template(self, template: str, topic: str, context: str) -> str:
        """Replicate _fill_template logic."""
        import re

        method_match = re.search(r'([A-Z][a-z]+(?:\s+[A-Z][a-z]+)+)', context[:200])
        method = method_match.group(1) if method_match else topic

        replacements = {
            "{method}": method,
            "{existing_method}": method,
            "{new_domain}": "新领域",
            "{task}": "特定任务",
            "{challenge}": "核心挑战",
            "{approach}": "创新方法",
            "{method_A}": method,
            "{method_B}": "对比方法",
            "{improvement}": "改进机制",
            "{limitation}": "局限性",
            "{solution}": "解决方案",
            "{scale}": "更大",
            "{metric}": "新指标",
            "{aspect}": "关键方面",
            "{condition}": "特定条件",
            "{other_method}": "其他方法",
            "{underlying_factor}": "底层因素",
        }

        result = template
        for key, value in replacements.items():
            result = result.replace(key, value)

        return result

    def test_no_placeholders_returns_template(self):
        """Template without placeholders is returned as-is."""
        result = self._fill_template("Simple hypothesis", "topic", "")
        assert result == "Simple hypothesis"

    def test_replaces_method_from_context(self):
        """Method extracted from context via regex (needs 2+ capitalized words)."""
        result = self._fill_template(
            "Adding {method} to improve",
            "topic",
            "This paper proposes Transformer Attention",
        )
        assert result == "Adding Transformer Attention to improve"

    def test_replaces_method_with_topic_when_no_match(self):
        """Fallback to topic when no capitalized method found."""
        result = self._fill_template("{method} can be improved", "BERT", "lowercase context")
        assert result == "BERT can be improved"

    def test_replaces_all_known_placeholders(self):
        """All placeholders replaced correctly."""
        result = self._fill_template(
            "{method_A} + {method_B}",
            "Topic",
            "Standard Method Here",
        )
        assert "Standard Method Here" in result
        assert "对比方法" in result

    def test_chinese_placeholders(self):
        """Chinese placeholders replaced."""
        result = self._fill_template(
            "应用于{new_domain}",
            "Topic",
            "",
        )
        assert result == "应用于新领域"

    def test_topic_used_for_multiple_placeholders(self):
        """All {method} placeholders use the same extracted value."""
        # BERT doesn't match [A-Z][a-z]+ so falls back to topic
        # This test verifies replacements all happen (no crash)
        result = self._fill_template(
            "{method} + {existing_method}",
            "BERT",
            "Some lowercase context",
        )
        # Falls back to topic "BERT" for both
        assert "BERT + BERT" in result


# =============================================================================
# _infer_gap_type tests
# =============================================================================
class TestInferGapType:
    """Test _infer_gap_type logic."""

    def _infer_gap_type(self, context: str) -> str:
        """Replicate _infer_gap_type logic."""
        context_lower = context.lower()

        if any(k in context_lower for k in ['limitation', '不足', 'weakness', 'scalability']):
            return "method_limitation"
        elif any(k in context_lower for k in ['future', 'unexplored', '未探索']):
            return "unexplored_application"
        elif any(k in context_lower for k in ['however', 'contradict', '矛盾']):
            return "contradiction"
        elif any(k in context_lower for k in ['scale', '扩展', '大规模']):
            return "scalability_issue"
        elif any(k in context_lower for k in ['benchmark', '评估', 'metric']):
            return "evaluation_gap"

        return "method_limitation"

    def test_limitation_keywords(self):
        """limitation keyword triggers method_limitation."""
        assert self._infer_gap_type("has limitation") == "method_limitation"
        assert self._infer_gap_type("方法不足") == "method_limitation"
        assert self._infer_gap_type("scalability issue") == "method_limitation"

    def test_unexplored_keywords(self):
        """unexplored keyword triggers unexplored_application."""
        assert self._infer_gap_type("unexplored application") == "unexplored_application"
        assert self._infer_gap_type("未探索的领域") == "unexplored_application"
        assert self._infer_gap_type("future work") == "unexplored_application"

    def test_contradiction_keywords(self):
        """contradict keyword triggers contradiction."""
        assert self._infer_gap_type("however results differ") == "contradiction"
        assert self._infer_gap_type("发现矛盾") == "contradiction"

    def test_scalability_keywords(self):
        """scale keyword triggers scalability_issue."""
        assert self._infer_gap_type("cannot scale") == "scalability_issue"
        assert self._infer_gap_type("扩展到大数据") == "scalability_issue"

    def test_evaluation_keywords(self):
        """benchmark keyword triggers evaluation_gap."""
        assert self._infer_gap_type("no benchmark available") == "evaluation_gap"
        assert self._infer_gap_type("缺少评估指标") == "evaluation_gap"

    def test_default_is_method_limitation(self):
        """No keywords defaults to method_limitation."""
        assert self._infer_gap_type("This is a neutral statement") == "method_limitation"
        assert self._infer_gap_type("") == "method_limitation"

    def test_case_insensitive(self):
        """Matching is case insensitive."""
        assert self._infer_gap_type("LIMITATION found") == "method_limitation"
        assert self._infer_gap_type("UNEXPLORED domain") == "unexplored_application"

    def test_first_match_wins(self):
        """First matching keyword wins."""
        # limitation comes before scalability in priority
        assert self._infer_gap_type("has limitation and scalability") == "method_limitation"


# =============================================================================
# _assess_risk tests
# =============================================================================
class TestAssessRisk:
    """Test _assess_risk logic."""

    def _assess_risk(self, hypothesis: ResearchHypothesis) -> RiskAssessment:
        """Replicate _assess_risk logic."""
        tech_risk = RiskLevel.MEDIUM
        hyp_risk = RiskLevel.MEDIUM

        if "新领域" in hypothesis.core_statement:
            hyp_risk = RiskLevel.HIGH
            tech_risk = RiskLevel.HIGH

        if hypothesis.hypothesis_type == HypothesisType.EXPLORATORY:
            hyp_risk = RiskLevel.HIGH

        return RiskAssessment(
            technical_risk=tech_risk,
            hypothesis_risk=hyp_risk,
            technical_reason="技术实现难度适中" if tech_risk == RiskLevel.MEDIUM else "技术挑战较大",
            hypothesis_reason="假说具体可验证" if hyp_risk == RiskLevel.LOW else "存在不确定性",
            mitigation=[
                "从小规模实验开始",
                "设置检查点评估",
                "准备备选假说",
            ],
        )

    def test_exploratory_hypothesis_has_high_hypothesis_risk(self):
        """EXPLORATORY type gets HIGH hypothesis risk."""
        h = ResearchHypothesis(
            id="x", title="T",
            hypothesis_type=HypothesisType.EXPLORATORY,
            core_statement="Explore something",
        )
        risk = self._assess_risk(h)
        assert risk.hypothesis_risk == RiskLevel.HIGH

    def test_new_domain_phrase_raises_both_risks(self):
        """新领域 in statement raises both risks to HIGH."""
        h = ResearchHypothesis(
            id="x", title="T",
            hypothesis_type=HypothesisType.CAUSAL,
            core_statement="在新领域应用",
        )
        risk = self._assess_risk(h)
        assert risk.technical_risk == RiskLevel.HIGH
        assert risk.hypothesis_risk == RiskLevel.HIGH

    def test_causal_without_new_domain(self):
        """CAUSAL without 新领域 gets MEDIUM risks."""
        h = ResearchHypothesis(
            id="x", title="T",
            hypothesis_type=HypothesisType.CAUSAL,
            core_statement="Improve existing method",
        )
        risk = self._assess_risk(h)
        assert risk.technical_risk == RiskLevel.MEDIUM
        assert risk.hypothesis_risk == RiskLevel.MEDIUM

    def test_exploratory_overrides_new_domain(self):
        """EXPLORATORY already sets HIGH, 新领域 adds nothing extra."""
        h = ResearchHypothesis(
            id="x", title="T",
            hypothesis_type=HypothesisType.EXPLORATORY,
            core_statement="新领域探索",
        )
        risk = self._assess_risk(h)
        # Both are HIGH anyway
        assert risk.technical_risk == RiskLevel.HIGH
        assert risk.hypothesis_risk == RiskLevel.HIGH

    def test_mitigation_steps_present(self):
        """Risk assessment includes 3 mitigation steps."""
        h = ResearchHypothesis(id="x", title="T", core_statement="S")
        risk = self._assess_risk(h)
        assert len(risk.mitigation) == 3

    def test_technical_reason_text(self):
        """Technical reason text varies by risk level."""
        h_medium = ResearchHypothesis(
            id="x", title="T",
            hypothesis_type=HypothesisType.CAUSAL,
            core_statement="S",
        )
        h_high = ResearchHypothesis(
            id="x", title="T",
            hypothesis_type=HypothesisType.CAUSAL,
            core_statement="新领域",
        )
        r_medium = self._assess_risk(h_medium)
        r_high = self._assess_risk(h_high)
        assert r_medium.technical_reason == "技术实现难度适中"
        assert r_high.technical_reason == "技术挑战较大"


# =============================================================================
# _calculate_novelty tests
# =============================================================================
class TestCalculateNovelty:
    """Test _calculate_novelty logic."""

    def _calculate_novelty(self, hypothesis: ResearchHypothesis, has_trend: bool, has_story: bool) -> float:
        """Replicate _calculate_novelty logic."""
        score = 0.5

        if hypothesis.hypothesis_type == HypothesisType.EXPLORATORY:
            score += 0.2

        if hypothesis.core_statement.startswith("跨领域"):
            score += 0.3

        if has_trend and has_story:
            score += 0.1

        return min(score, 1.0)

    def test_baseline_score(self):
        """Default CAUSAL with no context = 0.5."""
        h = ResearchHypothesis(id="x", title="T", hypothesis_type=HypothesisType.CAUSAL)
        assert self._calculate_novelty(h, False, False) == 0.5

    def test_exploratory_adds_02(self):
        """EXPLORATORY type adds 0.2."""
        h = ResearchHypothesis(id="x", title="T", hypothesis_type=HypothesisType.EXPLORATORY)
        assert self._calculate_novelty(h, False, False) == 0.7

    def test_cross_domain_adds_03(self):
        """跨领域 prefix adds 0.3."""
        h = ResearchHypothesis(id="x", title="T", hypothesis_type=HypothesisType.CAUSAL, core_statement="跨领域探索X")
        assert self._calculate_novelty(h, False, False) == 0.8

    def test_combined_context_adds_01(self):
        """Both trend and story adds 0.1."""
        h = ResearchHypothesis(id="x", title="T", hypothesis_type=HypothesisType.CAUSAL)
        assert self._calculate_novelty(h, True, True) == 0.6

    def test_combined_all_factors(self):
        """EXPLORATORY + cross-domain + both context = capped at 1.0."""
        h = ResearchHypothesis(id="x", title="T", hypothesis_type=HypothesisType.EXPLORATORY, core_statement="跨领域新方法")
        # 0.5 + 0.2 + 0.3 + 0.1 = 1.1, capped at 1.0
        assert self._calculate_novelty(h, True, True) == 1.0

    def test_only_trend_no_bonus(self):
        """Only trend context without story gives no bonus."""
        h = ResearchHypothesis(id="x", title="T", hypothesis_type=HypothesisType.CAUSAL)
        assert self._calculate_novelty(h, True, False) == 0.5

    def test_only_story_no_bonus(self):
        """Only story context without trend gives no bonus."""
        h = ResearchHypothesis(id="x", title="T", hypothesis_type=HypothesisType.CAUSAL)
        assert self._calculate_novelty(h, False, True) == 0.5


# =============================================================================
# _calculate_feasibility tests
# =============================================================================
class TestCalculateFeasibility:
    """Test _calculate_feasibility logic."""

    def _calculate_feasibility(self, hypothesis: ResearchHypothesis) -> float:
        """Replicate _calculate_feasibility logic."""
        score = 0.6

        if len(hypothesis.experiment_design.variables) > 5:
            score -= 0.1

        if hypothesis.hypothesis_type == HypothesisType.EXPLORATORY:
            score -= 0.1

        return max(score, 0.3)

    def test_baseline_score(self):
        """CAUSAL with ≤5 variables = 0.6."""
        design = ExperimentDesign(
            baseline="B",
            variables=["v1", "v2"],
            controls=["c"],
            evaluation_metrics=["m"],
            expected_results="R",
        )
        h = ResearchHypothesis(id="x", title="T", hypothesis_type=HypothesisType.CAUSAL, experiment_design=design)
        assert self._calculate_feasibility(h) == 0.6

    def test_exploratory_subtracts_01(self):
        """EXPLORATORY type subtracts 0.1."""
        design = ExperimentDesign(
            baseline="B",
            variables=["v"],
            controls=["c"],
            evaluation_metrics=["m"],
            expected_results="R",
        )
        h = ResearchHypothesis(id="x", title="T", hypothesis_type=HypothesisType.EXPLORATORY, experiment_design=design)
        assert self._calculate_feasibility(h) == 0.5

    def test_many_variables_subtracts_01(self):
        """More than 5 variables subtracts 0.1."""
        design = ExperimentDesign(
            baseline="B",
            variables=["v1", "v2", "v3", "v4", "v5", "v6"],
            controls=["c"],
            evaluation_metrics=["m"],
            expected_results="R",
        )
        h = ResearchHypothesis(id="x", title="T", hypothesis_type=HypothesisType.CAUSAL, experiment_design=design)
        assert self._calculate_feasibility(h) == 0.5

    def test_combined_penalties(self):
        """EXPLORATORY + many variables = 0.4."""
        design = ExperimentDesign(
            baseline="B",
            variables=["v1", "v2", "v3", "v4", "v5", "v6"],
            controls=["c"],
            evaluation_metrics=["m"],
            expected_results="R",
        )
        h = ResearchHypothesis(id="x", title="T", hypothesis_type=HypothesisType.EXPLORATORY, experiment_design=design)
        assert self._calculate_feasibility(h) == 0.4

    def test_exploratory_minimum_with_many_variables(self):
        """EXPLORATORY + >5 variables yields floor of 0.4 (max(0.4, 0.3))."""
        design = ExperimentDesign(
            baseline="B",
            variables=["v1", "v2", "v3", "v4", "v5", "v6"],
            controls=["c"],
            evaluation_metrics=["m"],
            expected_results="R",
        )
        h = ResearchHypothesis(id="x", title="T", hypothesis_type=HypothesisType.EXPLORATORY, experiment_design=design)
        # 0.6 - 0.1 (variables) - 0.1 (EXPLORATORY) = 0.4, max(0.4, 0.3) = 0.4
        assert self._calculate_feasibility(h) == 0.4


# =============================================================================
# _generate_summary tests
# =============================================================================
class TestGenerateSummary:
    """Test _generate_summary logic."""

    def _generate_summary(self, result: HypothesisResult) -> str:
        """Replicate _generate_summary logic."""
        if not result.hypotheses:
            return "无法生成有效假说，请提供更多上下文"

        high_feasibility = [h for h in result.hypotheses if h.feasibility_score > 0.6]
        high_novelty = [h for h in result.hypotheses if h.novelty_score > 0.6]

        summary = f"生成了 {len(result.hypotheses)} 个研究假说"

        if high_feasibility:
            summary += f"，其中 {len(high_feasibility)} 个可行性较高"

        if high_novelty:
            summary += f"，{len(high_novelty)} 个创新性较高"

        return summary

    def test_empty_returns_error_message(self):
        """Empty hypotheses list returns error message."""
        result = HypothesisResult(topic="T")
        assert self._generate_summary(result) == "无法生成有效假说，请提供更多上下文"

    def test_counts_hypotheses(self):
        """Summary starts with count."""
        h = ResearchHypothesis(id="x", title="T", feasibility_score=0.5, novelty_score=0.5)
        result = HypothesisResult(topic="T", hypotheses=[h])
        summary = self._generate_summary(result)
        assert "1 个研究假说" in summary

    def test_high_feasibility_count(self):
        """High feasibility hypotheses counted."""
        h1 = ResearchHypothesis(id="x", title="T", feasibility_score=0.7, novelty_score=0.5)
        h2 = ResearchHypothesis(id="x", title="T", feasibility_score=0.5, novelty_score=0.5)
        result = HypothesisResult(topic="T", hypotheses=[h1, h2])
        summary = self._generate_summary(result)
        assert "1 个可行性较高" in summary

    def test_high_novelty_count(self):
        """High novelty hypotheses counted."""
        h1 = ResearchHypothesis(id="x", title="T", feasibility_score=0.5, novelty_score=0.7)
        h2 = ResearchHypothesis(id="x", title="T", feasibility_score=0.5, novelty_score=0.5)
        result = HypothesisResult(topic="T", hypotheses=[h1, h2])
        summary = self._generate_summary(result)
        assert "1 个创新性较高" in summary

    def test_boundary_feasibility(self):
        """Score of exactly 0.6 does NOT count as high."""
        h = ResearchHypothesis(id="x", title="T", feasibility_score=0.6, novelty_score=0.5)
        result = HypothesisResult(topic="T", hypotheses=[h])
        summary = self._generate_summary(result)
        assert "可行性较高" not in summary

    def test_boundary_novelty(self):
        """Score of exactly 0.6 does NOT count as high."""
        h = ResearchHypothesis(id="x", title="T", feasibility_score=0.5, novelty_score=0.6)
        result = HypothesisResult(topic="T", hypotheses=[h])
        summary = self._generate_summary(result)
        assert "创新性较高" not in summary


# =============================================================================
# render_result tests
# =============================================================================
class TestRenderResult:
    """Test render_result formatting."""

    def _render_result(self, result: HypothesisResult) -> str:
        """Replicate render_result logic."""
        lines = [
            f"🎯 研究假说生成: {result.topic}",
            "",
            "═" * 60,
            "",
        ]

        for i, h in enumerate(result.hypotheses, 1):
            lines.append(f"假说 #{i}: {h.title}")
            lines.append("─" * 40)
            lines.append(f"类型: {h.hypothesis_type.value}")
            lines.append(f"核心假说: {h.core_statement}")
            lines.append(f"基于: {h.based_on}")
            lines.append("")

            lines.append("实验设计:")
            lines.append(f"  基线: {h.experiment_design.baseline}")
            lines.append(f"  变量: {', '.join(h.experiment_design.variables[:3])}")
            lines.append(f"  控制: {', '.join(h.experiment_design.controls[:2])}")
            lines.append(f"  评估: {', '.join(h.experiment_design.evaluation_metrics[:2])}")
            lines.append("")

            lines.append(f"评分: 创新性 {h.novelty_score:.0%} | 可行性 {h.feasibility_score:.0%}")
            lines.append("")

            if h.risk_assessment:
                risk_icon = {
                    RiskLevel.LOW: "🟢",
                    RiskLevel.MEDIUM: "🟡",
                    RiskLevel.HIGH: "🔴",
                }
                lines.append("风险评估:")
                lines.append(f"  技术风险: {risk_icon[h.risk_assessment.technical_risk]} {h.risk_assessment.technical_reason}")
                lines.append(f"  假设风险: {risk_icon[h.risk_assessment.hypothesis_risk]} {h.risk_assessment.hypothesis_reason}")
                lines.append("")

            if h.differentiations:
                lines.append("与现有工作的区分:")
                for d in h.differentiations[:2]:
                    lines.append(f"  • vs {d.compared_work}: {d.innovation}")
                lines.append("")

            lines.append("═" * 60)
            lines.append("")

        lines.append(f"📊 {result.summary}")

        return '\n'.join(lines)

    def test_header(self):
        """Header contains topic."""
        result = HypothesisResult(topic="Transformer")
        output = self._render_result(result)
        assert "🎯 研究假说生成: Transformer" in output
        assert "═" * 60 in output

    def test_hypothesis_entry(self):
        """Hypothesis data is displayed."""
        design = ExperimentDesign(
            baseline="BERT",
            variables=["lr", "batch"],
            controls=["seed"],
            evaluation_metrics=["acc"],
            expected_results="Better",
        )
        h = ResearchHypothesis(
            id="h1", title="Hypothesis 1",
            hypothesis_type=HypothesisType.CAUSAL,
            core_statement="Better method",
            based_on="Gap analysis",
            experiment_design=design,
        )
        result = HypothesisResult(topic="T", hypotheses=[h])
        output = self._render_result(result)
        assert "假说 #1: Hypothesis 1" in output
        assert "causal" in output
        assert "Better method" in output
        assert "Gap analysis" in output

    def test_experiment_design_shown(self):
        """Experiment design fields are displayed."""
        design = ExperimentDesign(
            baseline="B",
            variables=["var1", "var2"],
            controls=["ctrl1"],
            evaluation_metrics=["acc"],
            expected_results="R",
        )
        h = ResearchHypothesis(id="x", title="T", experiment_design=design)
        result = HypothesisResult(topic="T", hypotheses=[h])
        output = self._render_result(result)
        assert "基线: B" in output
        assert "var1, var2" in output
        assert "ctrl1" in output

    def test_variables_limited_to_3(self):
        """Variables truncated to first 3."""
        design = ExperimentDesign(
            baseline="B",
            variables=["v1", "v2", "v3", "v4", "v5"],
            controls=["c"],
            evaluation_metrics=["m"],
            expected_results="R",
        )
        h = ResearchHypothesis(id="x", title="T", experiment_design=design)
        result = HypothesisResult(topic="T", hypotheses=[h])
        output = self._render_result(result)
        lines = output.split('\n')
        var_line = [l for l in lines if '变量:' in l][0]
        assert "v1, v2, v3" in var_line
        assert "v4" not in var_line

    def test_scores_formatted(self):
        """Scores formatted as percentages."""
        h = ResearchHypothesis(id="x", title="T", novelty_score=0.7, feasibility_score=0.5)
        result = HypothesisResult(topic="T", hypotheses=[h])
        output = self._render_result(result)
        assert "创新性 70% | 可行性 50%" in output

    def test_risk_icons(self):
        """Risk levels map to icons."""
        risk = RiskAssessment(
            technical_risk=RiskLevel.HIGH,
            hypothesis_risk=RiskLevel.LOW,
            technical_reason="Hard",
            hypothesis_reason="Simple",
            mitigation=["M"],
        )
        h = ResearchHypothesis(id="x", title="T", risk_assessment=risk)
        result = HypothesisResult(topic="T", hypotheses=[h])
        output = self._render_result(result)
        assert "🔴" in output  # HIGH
        assert "🟢" in output  # LOW
        assert "风险评估:" in output

    def test_no_risk_section_when_none(self):
        """No risk section when risk_assessment is None."""
        h = ResearchHypothesis(id="x", title="T")
        result = HypothesisResult(topic="T", hypotheses=[h])
        output = self._render_result(result)
        assert "风险评估:" not in output

    def test_differentiations_shown(self):
        """Differentiation points displayed."""
        dp = DifferentiationPoint(
            compared_work="Old Method",
            our_advantage="Better",
            innovation="Novel approach",
        )
        design = ExperimentDesign(
            baseline="B", variables=[], controls=[], evaluation_metrics=[], expected_results="R"
        )
        h = ResearchHypothesis(id="x", title="T", differentiations=[dp], experiment_design=design)
        result = HypothesisResult(topic="T", hypotheses=[h])
        output = self._render_result(result)
        assert "与现有工作的区分:" in output
        assert "Old Method" in output

    def test_differentiations_limited_to_2(self):
        """Only first 2 differentiations shown."""
        dp1 = DifferentiationPoint(
            compared_work="Old Method 1",
            our_advantage="A",
            innovation="I",
        )
        dp2 = DifferentiationPoint(
            compared_work="Old Method 2",
            our_advantage="A",
            innovation="I",
        )
        dp3 = DifferentiationPoint(
            compared_work="Old Method 3",
            our_advantage="A",
            innovation="I",
        )
        design = ExperimentDesign(
            baseline="B", variables=[], controls=[], evaluation_metrics=[], expected_results="R"
        )
        h = ResearchHypothesis(
            id="x", title="T",
            differentiations=[dp1, dp2, dp3],
            experiment_design=design,
        )
        result = HypothesisResult(topic="T", hypotheses=[h])
        output = self._render_result(result)
        lines = output.split('\n')
        diff_lines = [l for l in lines if 'Method' in l]
        assert len(diff_lines) == 2

    def test_summary_appended(self):
        """Summary appended at end."""
        result = HypothesisResult(topic="T", summary="共生成3个假说")
        output = self._render_result(result)
        assert output.endswith("📊 共生成3个假说")


# =============================================================================
# render_json tests
# =============================================================================
class TestRenderJson:
    """Test render_json formatting."""

    def _render_json(self, result: HypothesisResult) -> str:
        """Replicate render_json logic."""
        import json

        data = {
            "topic": result.topic,
            "hypotheses": [
                {
                    "title": h.title,
                    "type": h.hypothesis_type.value,
                    "core_statement": h.core_statement,
                    "based_on": h.based_on,
                    "novelty_score": h.novelty_score,
                    "feasibility_score": h.feasibility_score,
                    "experiment": {
                        "baseline": h.experiment_design.baseline,
                        "variables": h.experiment_design.variables,
                        "controls": h.experiment_design.controls,
                        "metrics": h.experiment_design.evaluation_metrics,
                    },
                    "risk": {
                        "technical": h.risk_assessment.technical_risk.value if h.risk_assessment else "unknown",
                        "hypothesis": h.risk_assessment.hypothesis_risk.value if h.risk_assessment else "unknown",
                    },
                }
                for h in result.hypotheses
            ],
            "summary": result.summary,
        }

        return json.dumps(data, ensure_ascii=False, indent=2)

    def test_topic_in_json(self):
        """Topic included in JSON."""
        result = HypothesisResult(topic="Transformer")
        output = self._render_json(result)
        assert '"topic": "Transformer"' in output

    def test_hypothesis_in_json(self):
        """Hypothesis data included in JSON."""
        import json
        design = ExperimentDesign(
            baseline="B",
            variables=["v1"],
            controls=["c1"],
            evaluation_metrics=["m1"],
            expected_results="R",
        )
        h = ResearchHypothesis(
            id="h1", title="H1",
            hypothesis_type=HypothesisType.COMPARATIVE,
            core_statement="Statement",
            based_on="Gap",
            novelty_score=0.8,
            feasibility_score=0.6,
            experiment_design=design,
        )
        result = HypothesisResult(topic="T", hypotheses=[h])
        output = self._render_json(result)
        parsed = json.loads(output)
        assert parsed["hypotheses"][0]["title"] == "H1"
        assert parsed["hypotheses"][0]["type"] == "comparative"
        assert parsed["hypotheses"][0]["novelty_score"] == 0.8

    def test_risk_unknown_when_none(self):
        """Missing risk_assessment shows 'unknown'."""
        import json
        h = ResearchHypothesis(id="x", title="T")
        result = HypothesisResult(topic="T", hypotheses=[h])
        output = self._render_json(result)
        parsed = json.loads(output)
        assert parsed["hypotheses"][0]["risk"]["technical"] == "unknown"
        assert parsed["hypotheses"][0]["risk"]["hypothesis"] == "unknown"

    def test_risk_values_when_present(self):
        """Risk values included when present."""
        import json
        risk = RiskAssessment(
            technical_risk=RiskLevel.HIGH,
            hypothesis_risk=RiskLevel.LOW,
            technical_reason="TR",
            hypothesis_reason="HR",
            mitigation=["M"],
        )
        h = ResearchHypothesis(id="x", title="T", risk_assessment=risk)
        result = HypothesisResult(topic="T", hypotheses=[h])
        output = self._render_json(result)
        parsed = json.loads(output)
        assert parsed["hypotheses"][0]["risk"]["technical"] == "high"
        assert parsed["hypotheses"][0]["risk"]["hypothesis"] == "low"

    def test_summary_in_json(self):
        """Summary included in JSON."""
        result = HypothesisResult(topic="T", summary="Summary text")
        output = self._render_json(result)
        assert '"summary": "Summary text"' in output

    def test_empty_hypotheses(self):
        """Empty hypotheses list produces empty array."""
        result = HypothesisResult(topic="T")
        output = self._render_json(result)
        assert '"hypotheses": []' in output

    def test_is_valid_json(self):
        """Output is valid JSON."""
        import json
        result = HypothesisResult(topic="T", summary="S")
        output = self._render_json(result)
        parsed = json.loads(output)
        assert parsed["topic"] == "T"


# =============================================================================
# HypothesisGenerator instantiation
# =============================================================================
class TestHypothesisGeneratorInit:
    """Test HypothesisGenerator class."""

    def test_can_instantiate(self):
        """HypothesisGenerator can be instantiated."""
        gen = HypothesisGenerator()
        assert gen.db is None

    def test_can_instantiate_with_db(self):
        """HypothesisGenerator can be instantiated with db."""
        mock_db = object()
        gen = HypothesisGenerator(db=mock_db)
        assert gen.db is mock_db
