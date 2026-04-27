"""
Research Pipeline Orchestrator: Chain gap→trend→story→hypothesis analysis.

研究流水线编排器：将多个分析模块串联成完整的研究闭环。

流水线阶段：
1. Trend Analysis - 研究趋势和热点关键词
2. Story Weaving - 研究故事线和矛盾点
3. Question Validation - 研究问题验证和创新评分
4. Hypothesis Generation - 可验证假说生成
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import Optional, List, Any


class PipelineStage(Enum):
    """Pipeline execution stages."""
    TREND = "trend"
    STORY = "story"
    VALIDATE = "validate"
    HYPOTHESIZE = "hypothesize"


@dataclass
class PipelineResult:
    """Complete research pipeline result."""
    topic: str
    stage: PipelineStage  # Current/completed stage
    trend_result: Optional[Any] = None
    story_result: Optional[Any] = None
    validate_result: Optional[Any] = None
    hypothesis_result: Optional[Any] = None
    errors: List[str] = field(default_factory=list)


class ResearchPipeline:
    """Orchestrate research analysis pipeline by chaining multiple modules."""

    def __init__(self, db=None):
        self.db = db

    def run(
        self,
        topic: str,
        stages: List[PipelineStage] = None,
        quick: bool = False,
        use_llm: bool = True,
        model: Optional[str] = None,
    ) -> PipelineResult:
        """
        Run the research pipeline.

        Args:
            topic: Research topic to analyze
            stages: Specific stages to run (default: all)
            quick: Quick mode with fewer papers
            use_llm: Enable LLM enhancement
            model: LLM model name

        Returns:
            PipelineResult with results from all stages
        """
        if stages is None:
            stages = [
                PipelineStage.TREND,
                PipelineStage.STORY,
                PipelineStage.VALIDATE,
                PipelineStage.HYPOTHESIZE,
            ]

        result = PipelineResult(topic=topic, stage=stages[-1] if stages else PipelineStage.TREND)

        # Stage 1: Trend Analysis
        if PipelineStage.TREND in stages:
            try:
                result.trend_result = self._run_trend(topic, quick)
            except Exception as e:
                result.errors.append(f"Trend: {str(e)}")

        # Stage 2: Story Weaving
        if PipelineStage.STORY in stages:
            try:
                result.story_result = self._run_story(topic, quick)
            except Exception as e:
                result.errors.append(f"Story: {str(e)}")

        # Stage 3: Question Validation
        if PipelineStage.VALIDATE in stages:
            try:
                question = self._extract_question(result)
                result.validate_result = self._run_validate(question, use_llm, model)
            except Exception as e:
                result.errors.append(f"Validate: {str(e)}")

        # Stage 4: Hypothesis Generation
        if PipelineStage.HYPOTHESIZE in stages:
            try:
                result.hypothesis_result = self._run_hypothesize(
                    topic, result, use_llm, model
                )
            except Exception as e:
                result.errors.append(f"Hypothesize: {str(e)}")

        return result

    def _run_trend(self, topic: str, quick: bool) -> Any:
        """Run trend analysis stage."""
        from llm.trend_analyzer import TrendAnalyzer
        analyzer = TrendAnalyzer(db=self.db)
        return analyzer.analyze(
            topic,
            min_papers=5 if quick else 10,
        )

    def _run_story(self, topic: str, quick: bool) -> Any:
        """Run story weaving stage."""
        from llm.story_weaver import StoryWeaver
        weaver = StoryWeaver(db=self.db)
        return weaver.weave(
            topic,
            max_papers=10 if quick else 20,
            use_llm=False,  # Disable LLM for faster execution
        )

    def _run_validate(self, question: str, use_llm: bool, model: Optional[str]) -> Any:
        """Run question validation stage."""
        from llm.question_validator import QuestionValidator
        validator = QuestionValidator()
        return validator.validate(
            question,
            use_llm=use_llm,
            model=model,
        )

    def _run_hypothesize(
        self,
        topic: str,
        pipeline_result: PipelineResult,
        use_llm: bool,
        model: Optional[str],
    ) -> Any:
        """Run hypothesis generation with context from previous stages."""
        from llm.hypothesis_generator import HypothesisGenerator

        # Build context strings from previous stages
        trend_context = self._build_trend_context(pipeline_result.trend_result)
        story_context = self._build_story_context(pipeline_result.story_result)
        gap_context = self._build_gap_context(pipeline_result.validate_result)

        generator = HypothesisGenerator(db=self.db)
        return generator.generate(
            topic=topic,
            gap_context=gap_context,
            trend_context=trend_context,
            story_context=story_context,
            use_llm=use_llm,
            model=model,
        )

    def _build_trend_context(self, trend_result) -> str:
        """Extract trend context for hypothesis generator."""
        if not trend_result:
            return ""
        keywords = getattr(trend_result, 'hot_keywords', [])[:5]
        growth = getattr(trend_result, 'growth_rate', 0)
        return f"热点关键词: {', '.join(keywords)}, 增长率: {growth:.0%}"

    def _build_story_context(self, story_result) -> str:
        """Extract story context for hypothesis generator."""
        if not story_result:
            return ""
        themes = getattr(story_result, 'themes', [])[:3]
        contradictions = getattr(story_result, 'contradictions', [])
        summary = getattr(story_result, 'summary', '')
        return f"主题: {', '.join(themes)}, 矛盾点: {len(contradictions)}个, {summary}"

    def _build_gap_context(self, validate_result) -> str:
        """Extract gap context for hypothesis generator."""
        if not validate_result:
            return ""
        return getattr(validate_result, 'gap_summary', '')

    def _extract_question(self, pipeline_result: PipelineResult) -> str:
        """Extract a research question from pipeline results."""
        topic = pipeline_result.topic
        # Try to form a question from themes
        if pipeline_result.story_result:
            themes = getattr(pipeline_result.story_result, 'themes', [])
            if themes:
                return f"How to improve {themes[0]}?"
        return f"What are the research gaps in {topic}?"

    def render_result(self, result: PipelineResult) -> str:
        """Render complete pipeline result as formatted text."""
        lines = [
            f"🔬 Research Pipeline: {result.topic}",
            "",
            "=" * 60,
        ]

        # Trend Summary
        if result.trend_result:
            lines.extend(self._render_trend(result.trend_result))

        # Story Summary
        if result.story_result:
            lines.extend(self._render_story(result.story_result))

        # Validation Summary
        if result.validate_result:
            lines.extend(self._render_validation(result.validate_result))

        # Hypothesis Summary
        if result.hypothesis_result:
            lines.extend(self._render_hypothesis(result.hypothesis_result))

        # Errors
        if result.errors:
            lines.append("")
            lines.append("⚠️  Errors:")
            for err in result.errors:
                lines.append(f"  • {err}")

        return '\n'.join(lines)

    def _render_trend(self, trend_result) -> List[str]:
        """Render trend analysis summary."""
        keywords = getattr(trend_result, 'hot_keywords', [])[:5]
        growth = getattr(trend_result, 'growth_rate', 0)
        return [
            "",
            "📈 趋势分析",
            f"  热点: {', '.join(keywords) or 'N/A'}",
            f"  增长: {growth:.0%}",
        ]

    def _render_story(self, story_result) -> List[str]:
        """Render story weaving summary."""
        themes = getattr(story_result, 'themes', [])[:3]
        contradictions = getattr(story_result, 'contradictions', [])
        return [
            "",
            "📖 研究故事",
            f"  主题: {', '.join(themes) or 'N/A'}",
            f"  矛盾: {len(contradictions)} 个待解决",
        ]

    def _render_validation(self, validate_result) -> List[str]:
        """Render validation summary."""
        score = getattr(validate_result, 'innovation_score', None)
        score_val = score.overall if score else 0
        is_novel = getattr(validate_result, 'is_novel', False)
        return [
            "",
            "✅ 问题验证",
            f"  创新分: {score_val:.1f}/10",
            f"  原创性: {'✓' if is_novel else '✗'}",
        ]

    def _render_hypothesis(self, hypothesis_result) -> List[str]:
        """Render hypothesis generation summary."""
        hyps = hypothesis_result.hypotheses[:3]
        if not hyps:
            return ["", "🎯 假说: 无"]

        lines = ["", "🎯 研究假说"]
        for i, h in enumerate(hyps, 1):
            stmt = h.core_statement[:60] + "..." if len(h.core_statement) > 60 else h.core_statement
            lines.append(f"  {i}. {stmt}")
        return lines
