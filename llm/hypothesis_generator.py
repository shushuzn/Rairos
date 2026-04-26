"""
Research Hypothesis Generator: Generate testable research hypotheses from gaps.

研究假说生成器：从研究空白生成可验证的假说。

核心算法：
1. 假说模板匹配：从空白类型到假说结构
2. 实验设计生成：基线、变量、控制策略
3. 差异化分析：与现有工作的区分点
4. 风险评估：技术风险、假设风险
"""

from __future__ import annotations

import re
import uuid
from dataclasses import dataclass, field
from enum import Enum
from typing import Optional, List, Dict, Any, Tuple

# Optional imports
try:
    from llm.chat import call_llm_chat_completions
    LLM_AVAILABLE = True
except ImportError:
    LLM_AVAILABLE = False


class HypothesisType(Enum):
    """Type of research hypothesis."""
    CAUSAL = "causal"           # 因果假说
    CORRELATIONAL = "correlational"  # 相关性假说
    COMPARATIVE = "comparative"   # 比较假说
    MECHANISTIC = "mechanistic"   # 机制假说
    EXPLORATORY = "exploratory"   # 探索性假说


class RiskLevel(Enum):
    """Risk level for hypothesis."""
    LOW = "low"
    MEDIUM = "medium"
    HIGH = "high"


@dataclass
class ExperimentDesign:
    """Experiment design for testing a hypothesis."""
    baseline: str  # 基线选择
    variables: List[str]  # 实验变量
    controls: List[str]  # 控制变量
    evaluation_metrics: List[str]  # 评估指标
    expected_results: str  # 预期结果
    alternative_if_wrong: str = ""  # 如果假说错误的方向


@dataclass
class DifferentiationPoint:
    """How hypothesis differs from existing work."""
    compared_work: str
    our_advantage: str
    innovation: str


@dataclass
class RiskAssessment:
    """Risk assessment for hypothesis."""
    technical_risk: RiskLevel
    hypothesis_risk: RiskLevel
    technical_reason: str
    hypothesis_reason: str
    mitigation: List[str]


@dataclass
class ResearchHypothesis:
    """A generated research hypothesis."""
    id: str  # Unique ID for tracking experiment → hypothesis linkage
    title: str
    hypothesis_type: HypothesisType
    core_statement: str  # 核心假说陈述
    based_on: str  # 基于什么（空白、趋势、矛盾）
    experiment_design: ExperimentDesign
    differentiations: List[DifferentiationPoint] = field(default_factory=list)
    risk_assessment: Optional[RiskAssessment] = None
    novelty_score: float = 0.5
    feasibility_score: float = 0.5
    gap_type: str = ""  # GapType value for verdict-driven gap reordering


@dataclass
class HypothesisResult:
    """Complete hypothesis generation result."""
    topic: str
    hypotheses: List[ResearchHypothesis] = field(default_factory=list)
    summary: str = ""
    integration_notes: str = ""  # 与其他模块的协同说明


class HypothesisGenerator:
    """Generate testable research hypotheses from analysis."""

    # Hypothesis templates by gap type
    HYPOTHESIS_TEMPLATES = {
        "method_limitation": [
            {
                "template": "通过在{method}中引入{improvement}，可以解决现有方法的{limitation}问题",
                "type": HypothesisType.CAUSAL,
                "variables": ["{method}实现", "{improvement}参数", "{limitation}指标"],
            },
            {
                "template": "{method_A} + {method_B}的组合可以克服各自的{limitation}",
                "type": HypothesisType.COMPARATIVE,
                "variables": ["组合比例", "融合层位置", "训练策略"],
            },
        ],
        "unexplored_application": [
            {
                "template": "{existing_method}可以应用于{new_domain}领域，并取得良好效果",
                "type": HypothesisType.EXPLORATORY,
                "variables": ["领域适配", "数据准备", "评估指标"],
            },
            {
                "template": "{task}任务中的{challenge}挑战可以通过{approach}方法解决",
                "type": HypothesisType.MECHANISTIC,
                "variables": ["任务难度", "方法适用性", "资源需求"],
            },
        ],
        "contradiction": [
            {
                "template": "{method_A}和{method_B}的差异源于{underlying_factor}，可通过实验验证",
                "type": HypothesisType.CAUSAL,
                "variables": ["控制因素", "测量方法", "统计分析"],
            },
            {
                "template": "存在{condition}使得{method}效果优于{other_method}",
                "type": HypothesisType.COMPARATIVE,
                "variables": ["条件变量", "效果指标", "临界点"],
            },
        ],
        "scalability_issue": [
            {
                "template": "通过{solution}可以使{method}扩展到{scale}规模",
                "type": HypothesisType.CAUSAL,
                "variables": ["扩展策略", "效率指标", "质量保证"],
            },
        ],
        "evaluation_gap": [
            {
                "template": "新的评估指标{metric}可以更准确地衡量{method}的{aspect}",
                "type": HypothesisType.CORRELATIONAL,
                "variables": ["指标定义", "标注成本", "与其他指标的相关性"],
            },
        ],
    }

    def __init__(self, db=None):
        self.db = db

    def generate(
        self,
        topic: str,
        gap_context: str = "",
        trend_context: str = "",
        story_context: str = "",
        use_llm: bool = True,
        api_key: Optional[str] = None,
        base_url: Optional[str] = None,
        model: Optional[str] = None,
        creative: bool = False,
    ) -> HypothesisResult:
        """
        Generate research hypotheses.

        Args:
            topic: Research topic
            gap_context: Context from gap detection
            trend_context: Context from trend analysis
            story_context: Context from story weaving
            use_llm: Whether to use LLM for generation
            api_key: LLM API key
            base_url: LLM API base URL
            model: Model name
            creative: Generate creative cross-domain hypotheses

        Returns:
            HypothesisResult with generated hypotheses
        """
        result = HypothesisResult(topic=topic)

        # Determine which context is available
        has_gap = bool(gap_context)
        has_trend = bool(trend_context)
        has_story = bool(story_context)

        # Generate hypotheses from templates
        hypotheses = self._generate_from_templates(
            topic, gap_context, trend_context, creative
        )

        # Enhance with LLM
        if use_llm and LLM_AVAILABLE:
            hypotheses = self._enhance_with_llm(
                topic, hypotheses, gap_context, trend_context,
                api_key, base_url, model
            )

        result.hypotheses = hypotheses

        # Generate experiment designs
        for h in result.hypotheses:
            h.experiment_design = self._generate_experiment_design(h, topic)

        # Generate differentiations
        if self.db:
            for h in result.hypotheses:
                h.differentiations = self._find_differentiations(h, topic)

        # Risk assessments
        for h in result.hypotheses:
            h.risk_assessment = self._assess_risk(h)

        # Calculate scores
        for h in result.hypotheses:
            h.novelty_score = self._calculate_novelty(h, has_trend, has_story)
            h.feasibility_score = self._calculate_feasibility(h)

        result.summary = self._generate_summary(result)

        return result

    def _generate_from_templates(
        self,
        topic: str,
        gap_context: str,
        trend_context: str,
        creative: bool,
    ) -> List[ResearchHypothesis]:
        """Generate hypotheses from templates."""
        hypotheses = []

        # Determine gap type from context
        gap_type = self._infer_gap_type(gap_context)

        templates = self.HYPOTHESIS_TEMPLATES.get(gap_type, [])
        for i, template_info in enumerate(templates[:2]):
            template = template_info["template"]

            # Fill in template with topic
            hypothesis = ResearchHypothesis(
                id=str(uuid.uuid4())[:8],
                title=f"假说 {i+1}: {topic} 研究",
                hypothesis_type=template_info["type"],
                core_statement=self._fill_template(template, topic, gap_context),
                based_on=f"基于{gap_type.replace('_', ' ')}类型",
                gap_type=gap_type,
                experiment_design=ExperimentDesign(
                    baseline="待确定",
                    variables=template_info["variables"],
                    controls=["计算资源", "训练数据", "随机种子"],
                    evaluation_metrics=["性能指标", "效率指标"],
                    expected_results="预期显著改进",
                ),
            )
            hypotheses.append(hypothesis)

        # Add creative hypothesis if requested
        if creative:
            creative_h = self._generate_creative_hypothesis(topic, gap_context)
            if creative_h:
                hypotheses.append(creative_h)

        return hypotheses

    def _fill_template(self, template: str, topic: str, context: str) -> str:
        """Fill in hypothesis template."""
        # Extract method names from context
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

    def _infer_gap_type(self, context: str) -> str:
        """Infer gap type from context."""
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

    def _generate_creative_hypothesis(
        self,
        topic: str,
        context: str,
    ) -> Optional[ResearchHypothesis]:
        """Generate creative cross-domain hypothesis."""
        # Look for cross-domain opportunities
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

        # Generate cross-domain hypothesis
        return ResearchHypothesis(
            id=str(uuid.uuid4())[:8],
            title=f"跨领域假说: {topic}",
            hypothesis_type=HypothesisType.EXPLORATORY,
            core_statement=f"将{topic}的方法/机制应用于跨领域任务可能产生意外的效果提升",
            based_on="跨领域创新思维",
            gap_type="cross_domain",
            experiment_design=ExperimentDesign(
                baseline="标准方法",
                variables=["领域迁移策略", "适配层设计", "预训练权重"],
                controls=["数据集规模", "模型大小", "训练轮数"],
                evaluation_metrics=["目标任务准确率", "迁移效率"],
                expected_results="跨领域迁移有效性",
            ),
        )

    def _enhance_with_llm(
        self,
        topic: str,
        hypotheses: List[ResearchHypothesis],
        gap_context: str,
        trend_context: str,
        api_key: Optional[str],
        base_url: Optional[str],
        model: Optional[str],
    ) -> List[ResearchHypothesis]:
        """Use LLM to enhance hypothesis generation."""
        import os

        api_key = api_key or os.getenv("OPENAI_API_KEY", "")
        if not api_key:
            return hypotheses

        context = f"Topic: {topic}\nGap: {gap_context[:200]}\nTrend: {trend_context[:200]}"

        system_prompt = """基于研究空白和趋势，生成3-5个具体可验证的研究假说。
每个假说需要包含：
1. 核心陈述：具体可测量的假说
2. 基于什么：空白类型、趋势方向
3. 预期结果：如果假说成立会观察到什么

输出格式：
[假说N] 核心陈述 | 基于 | 预期结果"""

        user_prompt = f"""研究领域: {topic}

上下文:
{context}

请生成具体的研究假说："""

        try:
            response = call_llm_chat_completions(
                base_url=base_url or os.getenv("OPENAI_BASE_URL", "https://api.openai.com/v1"),
                api_key=api_key,
                model=model or os.getenv("DEFAULT_LLM_MODEL", "gpt-4o-mini"),
                system_prompt=system_prompt,
                user_prompt=user_prompt,
            )

            # Parse LLM response and enhance hypotheses
            for line in response.strip().split('\n'):
                if line.startswith('[假说') or line.startswith('Hypothesis'):
                    # Add new hypothesis from LLM
                    parts = line.split('|')
                    if len(parts) >= 1:
                        hypothesis = ResearchHypothesis(
                            id=str(uuid.uuid4())[:8],
                            title=f"LLM生成假说",
                            hypothesis_type=HypothesisType.EXPLORATORY,
                            core_statement=parts[0].split(']')[1].strip() if ']' in parts[0] else parts[0],
                            based_on="LLM增强生成",
                            gap_type=self._infer_gap_type(gap_context),
                            experiment_design=ExperimentDesign(
                                baseline="待设计",
                                variables=["待确定"],
                                controls=["待确定"],
                                evaluation_metrics=["待确定"],
                                expected_results="待确定",
                            ),
                        )
                        hypotheses.append(hypothesis)

        except Exception:
            pass

        return hypotheses[:5]  # Limit to 5 hypotheses

    def _generate_experiment_design(
        self,
        hypothesis: ResearchHypothesis,
        topic: str,
    ) -> ExperimentDesign:
        """Generate experiment design for hypothesis."""
        design = hypothesis.experiment_design

        # Enhance based on hypothesis type
        if hypothesis.hypothesis_type == HypothesisType.COMPARATIVE:
            design.baseline = f"{topic}的标准实现"
            design.evaluation_metrics.extend(["相对提升", "统计显著性"])

        elif hypothesis.hypothesis_type == HypothesisType.CAUSAL:
            design.controls.extend(["消融变量", "干预点"])

        elif hypothesis.hypothesis_type == HypothesisType.EXPLORATORY:
            design.evaluation_metrics.extend(["可行性", "资源消耗"])

        return design

    def _find_differentiations(
        self,
        hypothesis: ResearchHypothesis,
        topic: str,
    ) -> List[DifferentiationPoint]:
        """Find differentiation points from existing work."""
        if not self.db:
            return []

        differentiations = []
        try:
            rows, _ = self.db.search_papers(topic, limit=5)
            for row in rows[:3]:
                title = getattr(row, 'title', '')[:50] or '现有方法'
                differentiations.append(DifferentiationPoint(
                    compared_work=title,
                    our_advantage="创新点待确定",
                    innovation="方法/应用/评估创新",
                ))
        except Exception:
            pass

        return differentiations

    def _assess_risk(self, hypothesis: ResearchHypothesis) -> RiskAssessment:
        """Assess risk for hypothesis."""
        # Simple rule-based assessment
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

    def _calculate_novelty(
        self,
        hypothesis: ResearchHypothesis,
        has_trend: bool,
        has_story: bool,
    ) -> float:
        """Calculate novelty score."""
        score = 0.5

        if hypothesis.hypothesis_type == HypothesisType.EXPLORATORY:
            score += 0.2

        if hypothesis.core_statement.startswith("跨领域"):
            score += 0.3

        if has_trend and has_story:
            score += 0.1  # More context = more informed novelty

        return min(score, 1.0)

    def _calculate_feasibility(self, hypothesis: ResearchHypothesis) -> float:
        """Calculate feasibility score."""
        score = 0.6

        if len(hypothesis.experiment_design.variables) > 5:
            score -= 0.1

        if hypothesis.hypothesis_type == HypothesisType.EXPLORATORY:
            score -= 0.1

        return max(score, 0.3)

    def _generate_summary(self, result: HypothesisResult) -> str:
        """Generate summary of hypothesis generation."""
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

    def render_result(self, result: HypothesisResult) -> str:
        """Render hypothesis result as formatted text."""
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

            # Experiment design
            lines.append("实验设计:")
            lines.append(f"  基线: {h.experiment_design.baseline}")
            lines.append(f"  变量: {', '.join(h.experiment_design.variables[:3])}")
            lines.append(f"  控制: {', '.join(h.experiment_design.controls[:2])}")
            lines.append(f"  评估: {', '.join(h.experiment_design.evaluation_metrics[:2])}")
            lines.append("")

            # Scores
            lines.append(f"评分: 创新性 {h.novelty_score:.0%} | 可行性 {h.feasibility_score:.0%}")
            lines.append("")

            # Risk assessment
            if h.risk_assessment:
                risk_icon = {
                    RiskLevel.LOW: "🟢",
                    RiskLevel.MEDIUM: "🟡",
                    RiskLevel.HIGH: "🔴",
                }
                lines.append(f"风险评估:")
                lines.append(f"  技术风险: {risk_icon[h.risk_assessment.technical_risk]} {h.risk_assessment.technical_reason}")
                lines.append(f"  假设风险: {risk_icon[h.risk_assessment.hypothesis_risk]} {h.risk_assessment.hypothesis_reason}")
                lines.append("")

            # Differentiations
            if h.differentiations:
                lines.append("与现有工作的区分:")
                for d in h.differentiations[:2]:
                    lines.append(f"  • vs {d.compared_work}: {d.innovation}")
                lines.append("")

            lines.append("═" * 60)
            lines.append("")

        # Summary
        lines.append(f"📊 {result.summary}")

        return '\n'.join(lines)

    def render_json(self, result: HypothesisResult) -> str:
        """Render result as JSON."""
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
