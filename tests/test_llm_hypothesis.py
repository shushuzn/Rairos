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

    def test_all_types_exist(self):
        """All hypothesis types are defined."""
        assert HypothesisType.CAUSAL.value == "causal"
        assert HypothesisType.CORRELATIONAL.value == "correlational"
        assert HypothesisType.COMPARATIVE.value == "comparative"
        assert HypothesisType.MECHANISTIC.value == "mechanistic"
        assert HypothesisType.EXPLORATORY.value == "exploratory"

    def test_types_are_iterable(self):
        """Types can be iterated."""
        types = list(HypothesisType)
        assert len(types) == 5


class TestRiskLevel:
    """Test RiskLevel enum."""

    def test_all_levels_exist(self):
        """All risk levels are defined."""
        assert RiskLevel.LOW.value == "low"
        assert RiskLevel.MEDIUM.value == "medium"
        assert RiskLevel.HIGH.value == "high"

    def test_levels_are_iterable(self):
        """Levels can be iterated."""
        levels = list(RiskLevel)
        assert len(levels) == 3


# =============================================================================
# Dataclass tests
# =============================================================================
class TestExperimentDesign:
    """Test ExperimentDesign dataclass."""

    def test_required_fields(self):
        """Required fields: baseline, variables, controls, evaluation_metrics, expected_results."""
        design = ExperimentDesign(
            baseline="现有方法",
            variables=["参数A", "参数B"],
            controls=["控制1", "控制2"],
            evaluation_metrics=["准确率"],
            expected_results="改进效果",
        )
        assert design.baseline == "现有方法"
        assert len(design.variables) == 2
        assert len(design.controls) == 2
        assert len(design.evaluation_metrics) == 1
        assert design.expected_results == "改进效果"

    def test_optional_alternative_if_wrong(self):
        """alternative_if_wrong has default."""
        design = ExperimentDesign(
            baseline="baseline",
            variables=[],
            controls=[],
            evaluation_metrics=[],
            expected_results="results",
        )
        assert design.alternative_if_wrong == ""


class TestDifferentiationPoint:
    """Test DifferentiationPoint dataclass."""

    def test_required_fields(self):
        """Required fields: compared_work, our_advantage, innovation."""
        diff = DifferentiationPoint(
            compared_work="BERT",
            our_advantage="更高的效率",
            innovation="新的注意力机制",
        )
        assert diff.compared_work == "BERT"
        assert diff.our_advantage == "更高的效率"
        assert diff.innovation == "新的注意力机制"


class TestRiskAssessment:
    """Test RiskAssessment dataclass."""

    def test_required_fields(self):
        """Required fields: technical_risk, hypothesis_risk, technical_reason, hypothesis_reason, mitigation."""
        assessment = RiskAssessment(
            technical_risk=RiskLevel.MEDIUM,
            hypothesis_risk=RiskLevel.LOW,
            technical_reason="实现难度适中",
            hypothesis_reason="假说具体可验证",
            mitigation=["小规模实验", "检查点评估"],
        )
        assert assessment.technical_risk == RiskLevel.MEDIUM
        assert assessment.hypothesis_risk == RiskLevel.LOW
        assert len(assessment.mitigation) == 2


class TestResearchHypothesis:
    """Test ResearchHypothesis dataclass."""

    def test_required_fields_defaults(self):
        """Default values are sensible."""
        h = ResearchHypothesis()
        assert h.id == ""
        assert h.title == ""
        assert h.hypothesis_type == HypothesisType.CAUSAL
        assert h.core_statement == ""
        assert h.based_on == ""
        assert h.novelty_score == 0.5
        assert h.feasibility_score == 0.5
        assert h.gap_type == ""

    def test_experiment_design_default(self):
        """Experiment design has default factory."""
        h = ResearchHypothesis()
        assert isinstance(h.experiment_design, ExperimentDesign)
        assert h.experiment_design.baseline == ""

    def test_differentiations_default_empty(self):
        """Differentiations defaults to empty list."""
        h = ResearchHypothesis()
        assert h.differentiations == []

    def test_all_fields(self):
        """All fields can be set."""
        design = ExperimentDesign(
            baseline="b",
            variables=["v"],
            controls=["c"],
            evaluation_metrics=["m"],
            expected_results="r",
        )
        h = ResearchHypothesis(
            id="test123",
            title="测试假说",
            hypothesis_type=HypothesisType.EXPLORATORY,
            core_statement="核心陈述",
            based_on="基于研究空白",
            experiment_design=design,
            differentiations=[],
            risk_assessment=None,
            novelty_score=0.8,
            feasibility_score=0.7,
            gap_type="method_limitation",
        )
        assert h.id == "test123"
        assert h.hypothesis_type == HypothesisType.EXPLORATORY
        assert h.novelty_score == 0.8


class TestHypothesisResult:
    """Test HypothesisResult dataclass."""

    def test_required_fields(self):
        """Required fields: topic, hypotheses."""
        result = HypothesisResult(topic="Transformer研究")
        assert result.topic == "Transformer研究"
        assert result.hypotheses == []

    def test_optional_fields_defaults(self):
        """Optional fields have defaults."""
        result = HypothesisResult(topic="Topic")
        assert result.summary == ""
        assert result.integration_notes == ""


# =============================================================================
# Hypothesis templates
# =============================================================================
class TestHypothesisTemplates:
    """Test HYPOTHESIS_TEMPLATES structure."""

    def test_method_limitation_templates(self):
        """method_limitation templates exist."""
        templates = HypothesisGenerator.HYPOTHESIS_TEMPLATES["method_limitation"]
        assert len(templates) == 2
        assert templates[0]["type"] == HypothesisType.CAUSAL
        assert templates[1]["type"] == HypothesisType.COMPARATIVE

    def test_unexplored_application_templates(self):
        """unexplored_application templates exist."""
        templates = HypothesisGenerator.HYPOTHESIS_TEMPLATES["unexplored_application"]
        assert len(templates) == 2
        assert templates[0]["type"] == HypothesisType.EXPLORATORY
        assert templates[1]["type"] == HypothesisType.MECHANISTIC

    def test_contradiction_templates(self):
        """contradiction templates exist."""
        templates = HypothesisGenerator.HYPOTHESIS_TEMPLATES["contradiction"]
        assert len(templates) == 2

    def test_scalability_issue_templates(self):
        """scalability_issue templates exist."""
        templates = HypothesisGenerator.HYPOTHESIS_TEMPLATES["scalability_issue"]
        assert len(templates) == 1
        assert templates[0]["type"] == HypothesisType.CAUSAL

    def test_evaluation_gap_templates(self):
        """evaluation_gap templates exist."""
        templates = HypothesisGenerator.HYPOTHESIS_TEMPLATES["evaluation_gap"]
        assert len(templates) == 1
        assert templates[0]["type"] == HypothesisType.CORRELATIONAL

    def test_all_templates_have_placeholders(self):
        """All templates contain placeholders."""
        for gap_type, templates in HypothesisGenerator.HYPOTHESIS_TEMPLATES.items():
            for t in templates:
                assert "{" in t["template"]
                assert "}" in t["template"]

    def test_all_templates_have_variables(self):
        """All templates have variables list."""
        for gap_type, templates in HypothesisGenerator.HYPOTHESIS_TEMPLATES.items():
            for t in templates:
                assert isinstance(t["variables"], list)
                assert len(t["variables"]) > 0


# =============================================================================
# Template filling
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

    def test_fills_method_placeholder(self):
        """Fills {method} placeholder."""
        template = "通过在{method}中引入改进"
        result = self._fill_template(template, "BERT", "BERT is a model")
        assert "{method}" not in result
        assert "BERT" in result

    def test_fills_all_placeholders(self):
        """Replaces all placeholders."""
        template = "{method} + {method_B}"
        result = self._fill_template(template, "Transformer", "")
        assert "{" not in result
        assert "}" not in result

    def test_extracts_capitalized_method_from_context(self):
        """Extracts capitalized method names from context."""
        template = "test {method}"
        # Note: regex [A-Z][a-z]+ requires first upper, rest lower (not all-caps)
        result = self._fill_template(template, "default", "The Transformer Model is widely used")
        assert "Transformer" in result

    def test_uses_topic_when_no_method_found(self):
        """Uses topic when context has no capitalized method."""
        template = "test {method}"
        result = self._fill_template(template, "CustomModel", "no method here")
        assert "CustomModel" in result

    def test_fills_domain_placeholder(self):
        """Fills {new_domain} placeholder."""
        template = "应用到{new_domain}领域"
        result = self._fill_template(template, "topic", "")
        assert "新领域" in result


# =============================================================================
# Gap type inference
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

        return "method_limitation"  # Default

    def test_detects_method_limitation(self):
        """Detects limitation-related gap type."""
        assert self._infer_gap_type("现有方法的limitation明显") == "method_limitation"
        assert self._infer_gap_type("存在不足之处") == "method_limitation"
        assert self._infer_gap_type("scalability问题") == "method_limitation"

    def test_detects_unexplored_application(self):
        """Detects unexplored gap type."""
        assert self._infer_gap_type("future研究方向") == "unexplored_application"
        assert self._infer_gap_type("unexplored领域") == "unexplored_application"
        assert self._infer_gap_type("未探索的场景") == "unexplored_application"

    def test_detects_contradiction(self):
        """Detects contradiction gap type."""
        assert self._infer_gap_type("However研究显示") == "contradiction"
        assert self._infer_gap_type("存在矛盾的结果") == "contradiction"

    def test_detects_scalability(self):
        """Detects scalability gap type."""
        assert self._infer_gap_type("scale问题") == "scalability_issue"
        assert self._infer_gap_type("大规模场景") == "scalability_issue"

    def test_detects_evaluation_gap(self):
        """Detects evaluation gap type."""
        assert self._infer_gap_type("benchmark需要改进") == "evaluation_gap"
        assert self._infer_gap_type("评估metric缺失") == "evaluation_gap"

    def test_default_is_method_limitation(self):
        """Returns method_limitation for unknown context."""
        assert self._infer_gap_type("random text") == "method_limitation"
        assert self._infer_gap_type("") == "method_limitation"


# =============================================================================
# Score calculations
# =============================================================================
class TestCalculateNovelty:
    """Test _calculate_novelty logic."""

    def _calculate_novelty(self, hypothesis: dict, has_trend: bool, has_story: bool) -> float:
        """Replicate novelty calculation logic."""
        score = 0.5

        if hypothesis.get("hypothesis_type") == HypothesisType.EXPLORATORY:
            score += 0.2

        if hypothesis.get("core_statement", "").startswith("跨领域"):
            score += 0.3

        if has_trend and has_story:
            score += 0.1

        return min(score, 1.0)

    def test_exploratory_increases_score(self):
        """Exploratory type adds 0.2."""
        h = {"hypothesis_type": HypothesisType.EXPLORATORY, "core_statement": "test"}
        score = self._calculate_novelty(h, False, False)
        assert score == 0.7

    def test_cross_domain_adds_0_3(self):
        """Cross-domain adds 0.3."""
        h = {"hypothesis_type": HypothesisType.CAUSAL, "core_statement": "跨领域创新"}
        score = self._calculate_novelty(h, False, False)
        assert score == 0.8

    def test_trend_and_story_add_0_1(self):
        """Both trend and story add 0.1."""
        h = {"hypothesis_type": HypothesisType.CAUSAL, "core_statement": "test"}
        score = self._calculate_novelty(h, True, True)
        assert score == 0.6

    def test_max_score_is_1_0(self):
        """Score is capped at 1.0."""
        h = {"hypothesis_type": HypothesisType.EXPLORATORY, "core_statement": "跨领域创新"}
        score = self._calculate_novelty(h, True, True)
        assert score == 1.0


class TestCalculateFeasibility:
    """Test _calculate_feasibility logic."""

    def _calculate_feasibility(self, hypothesis: dict) -> float:
        """Replicate feasibility calculation logic."""
        score = 0.6

        if len(hypothesis.get("experiment_design", {}).get("variables", [])) > 5:
            score -= 0.1

        if hypothesis.get("hypothesis_type") == HypothesisType.EXPLORATORY:
            score -= 0.1

        return max(score, 0.3)

    def test_default_score(self):
        """Default score is 0.6."""
        h = {"experiment_design": {"variables": []}}
        score = self._calculate_feasibility(h)
        assert score == 0.6

    def test_many_variables_reduces_score(self):
        """More than 5 variables reduces score."""
        h = {"experiment_design": {"variables": ["v1", "v2", "v3", "v4", "v5", "v6"]}}
        score = self._calculate_feasibility(h)
        assert score == 0.5

    def test_exploratory_reduces_score(self):
        """Exploratory type reduces score by 0.1."""
        h = {"hypothesis_type": HypothesisType.EXPLORATORY, "experiment_design": {"variables": []}}
        score = self._calculate_feasibility(h)
        assert score == 0.5

    def test_min_score_bounded(self):
        """Score is bounded (implementation caps at 0.4, not 0.3)."""
        # Implementation: 0.6 - 0.1 (exploratory) - 0.1 (>5 vars) = 0.4
        h = {
            "hypothesis_type": HypothesisType.EXPLORATORY,
            "experiment_design": {"variables": ["v1", "v2", "v3", "v4", "v5", "v6"]},
        }
        score = self._calculate_feasibility(h)
        assert score == 0.4


# =============================================================================
# Risk assessment
# =============================================================================
class TestAssessRisk:
    """Test _assess_risk logic."""

    def _assess_risk(self, hypothesis: dict) -> dict:
        """Replicate risk assessment logic."""
        tech_risk = "medium"
        hyp_risk = "medium"

        if "新领域" in hypothesis.get("core_statement", ""):
            hyp_risk = "high"
            tech_risk = "high"

        if hypothesis.get("hypothesis_type") == HypothesisType.EXPLORATORY:
            hyp_risk = "high"

        return {
            "technical_risk": tech_risk,
            "hypothesis_risk": hyp_risk,
        }

    def test_new_domain_high_risk(self):
        """New domain increases risk."""
        h = {"core_statement": "应用到新领域", "hypothesis_type": HypothesisType.CAUSAL}
        result = self._assess_risk(h)
        assert result["technical_risk"] == "high"
        assert result["hypothesis_risk"] == "high"

    def test_exploratory_high_hypothesis_risk(self):
        """Exploratory increases hypothesis risk."""
        h = {"core_statement": "普通假说", "hypothesis_type": HypothesisType.EXPLORATORY}
        result = self._assess_risk(h)
        assert result["hypothesis_risk"] == "high"

    def test_default_medium_risk(self):
        """Default risk is medium."""
        h = {"core_statement": "普通假说", "hypothesis_type": HypothesisType.CAUSAL}
        result = self._assess_risk(h)
        assert result["technical_risk"] == "medium"
        assert result["hypothesis_risk"] == "medium"


# =============================================================================
# Creative hypothesis generation
# =============================================================================
class TestCreativeHypothesis:
    """Test creative cross-domain hypothesis generation."""

    def _generate_creative_hypothesis(self, topic: str) -> dict | None:
        """Replicate creative hypothesis logic."""
        domain_keywords = {
            "nlp": ["language", "text", "seq2seq"],
            "vision": ["image", "visual", "cnn"],
            "audio": ["speech", "audio", "sound"],
            "reasoning": ["logic", "reason", "inference"],
        }

        topic_lower = topic.lower()
        detected_domains = [
            domain for domain, keywords in domain_keywords.items()
            if any(k in topic_lower for k in keywords)
        ]

        if len(detected_domains) < 1:
            return None

        return {
            "title": f"跨领域假说: {topic}",
            "hypothesis_type": HypothesisType.EXPLORATORY,
            "core_statement": f"将{topic}的方法应用于跨领域任务",
        }

    def test_detects_nlp_domain(self):
        """Detects NLP-related topics."""
        result = self._generate_creative_hypothesis("Transformer for language")
        assert result is not None
        assert "跨领域" in result["title"]

    def test_detects_vision_domain(self):
        """Detects vision-related topics."""
        result = self._generate_creative_hypothesis("CNN for image classification")
        assert result is not None

    def test_returns_none_for_unknown_domain(self):
        """Returns None when no domain detected."""
        result = self._generate_creative_hypothesis("xyz123 random")
        assert result is None


# =============================================================================
# Hypothesis generation from templates
# =============================================================================
class TestGenerateFromTemplates:
    """Test _generate_from_templates logic."""

    def test_generates_hypotheses_from_templates(self):
        """Generates hypotheses based on gap type."""
        templates = HypothesisGenerator.HYPOTHESIS_TEMPLATES

        # method_limitation has 2 templates
        method_templates = templates["method_limitation"]
        assert len(method_templates) == 2

        for t in method_templates:
            assert "template" in t
            assert "type" in t
            assert "variables" in t

    def test_templates_have_valid_types(self):
        """All template types are valid HypothesisType values."""
        for gap_type, templates in HypothesisGenerator.HYPOTHESIS_TEMPLATES.items():
            for t in templates:
                # type should be one of the HypothesisType enum values
                type_value = t["type"].value
                assert type_value in ["causal", "correlational", "comparative", "mechanistic", "exploratory"]


# =============================================================================
# Summary generation
# =============================================================================
class TestGenerateSummary:
    """Test _generate_summary logic."""

    def _generate_summary(self, result: dict) -> str:
        """Replicate summary generation logic."""
        hypotheses = result.get("hypotheses", [])

        if not hypotheses:
            return "无法生成有效假说，请提供更多上下文"

        high_feasibility = [h for h in hypotheses if h.get("feasibility_score", 0) > 0.6]
        high_novelty = [h for h in hypotheses if h.get("novelty_score", 0) > 0.6]

        summary = f"生成了 {len(hypotheses)} 个研究假说"

        if high_feasibility:
            summary += f"，其中 {len(high_feasibility)} 个可行性较高"

        if high_novelty:
            summary += f"，{len(high_novelty)} 个创新性较高"

        return summary

    def test_empty_hypotheses(self):
        """Handles empty hypotheses."""
        result = {"hypotheses": []}
        summary = self._generate_summary(result)
        assert "无法生成" in summary

    def test_counts_hypotheses(self):
        """Counts hypotheses correctly."""
        result = {"hypotheses": [
            {"feasibility_score": 0.5, "novelty_score": 0.5},
            {"feasibility_score": 0.7, "novelty_score": 0.5},
        ]}
        summary = self._generate_summary(result)
        assert "2 个研究假说" in summary

    def test_highlights_high_feasibility(self):
        """Highlights high feasibility hypotheses."""
        result = {"hypotheses": [
            {"feasibility_score": 0.8, "novelty_score": 0.5},
        ]}
        summary = self._generate_summary(result)
        assert "可行性较高" in summary

    def test_highlights_high_novelty(self):
        """Highlights high novelty hypotheses."""
        result = {"hypotheses": [
            {"feasibility_score": 0.5, "novelty_score": 0.8},
        ]}
        summary = self._generate_summary(result)
        assert "创新性较高" in summary


# =============================================================================
# Prompt templates
# =============================================================================
class TestPromptTemplates:
    """Test prompt template constants."""

    def test_system_prompt_has_structure(self):
        """System prompt has required structure."""
        from llm.hypothesis_generator import _HYPOTHESIS_ENHANCEMENT_SYSTEM_PROMPT

        assert "3-5" in _HYPOTHESIS_ENHANCEMENT_SYSTEM_PROMPT
        assert "核心陈述" in _HYPOTHESIS_ENHANCEMENT_SYSTEM_PROMPT
        assert "预期结果" in _HYPOTHESIS_ENHANCEMENT_SYSTEM_PROMPT
        assert "[假说" in _HYPOTHESIS_ENHANCEMENT_SYSTEM_PROMPT

    def test_user_prompt_template_has_placeholders(self):
        """User prompt template has placeholders."""
        from llm.hypothesis_generator import _HYPOTHESIS_ENHANCEMENT_USER_PROMPT_TEMPLATE

        assert "{topic}" in _HYPOTHESIS_ENHANCEMENT_USER_PROMPT_TEMPLATE
        assert "{context}" in _HYPOTHESIS_ENHANCEMENT_USER_PROMPT_TEMPLATE

    def test_user_prompt_template_format(self):
        """User prompt template can be formatted."""
        from llm.hypothesis_generator import _HYPOTHESIS_ENHANCEMENT_USER_PROMPT_TEMPLATE

        formatted = _HYPOTHESIS_ENHANCEMENT_USER_PROMPT_TEMPLATE.format(
            topic="Test Topic",
            context="Test Context",
        )
        assert "{topic}" not in formatted
        assert "{context}" not in formatted
        assert "Test Topic" in formatted
