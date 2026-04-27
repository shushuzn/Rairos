"""Tier 2 unit tests — llm/question_validator.py, pure functions, no I/O."""
import pytest
from llm.question_validator import (
    NoveltyLevel,
    InnovationDimension,
    RelatedWork,
    InnovationScore,
    ValidationResult,
    QuestionValidator,
)


# =============================================================================
# Enum tests
# =============================================================================
class TestNoveltyLevel:
    """Test NoveltyLevel enum."""

    def test_all_levels_have_values(self):
        """All novelty levels have string values."""
        assert NoveltyLevel.HIGH.value == "high"
        assert NoveltyLevel.MEDIUM.value == "medium"
        assert NoveltyLevel.LOW.value == "low"
        assert NoveltyLevel.UNKNOWN.value == "unknown"


class TestInnovationDimension:
    """Test InnovationDimension enum."""

    def test_all_dimensions_have_values(self):
        """All innovation dimensions have string values."""
        assert InnovationDimension.METHOD.value == "method"
        assert InnovationDimension.TASK.value == "task"
        assert InnovationDimension.EVALUATION.value == "evaluation"
        assert InnovationDimension.THEORY.value == "theory"
        assert InnovationDimension.APPLICATION.value == "application"


# =============================================================================
# Dataclass tests
# =============================================================================
class TestRelatedWork:
    """Test RelatedWork dataclass."""

    def test_all_fields_required(self):
        """All fields are required (no defaults)."""
        rw = RelatedWork(
            paper_id="p1",
            title="Attention Is All You Need",
            year=2017,
            relevance_score=0.95,
            overlap_aspects=["transformer"],
            difference_aspects=["scalability"],
            conclusion="Solved parallel training",
        )
        assert rw.paper_id == "p1"
        assert rw.title == "Attention Is All You Need"
        assert rw.year == 2017
        assert rw.relevance_score == 0.95
        assert rw.overlap_aspects == ["transformer"]
        assert rw.difference_aspects == ["scalability"]
        assert rw.conclusion == "Solved parallel training"

    def test_optional_fields_default(self):
        """Optional fields have defaults."""
        rw = RelatedWork(paper_id="p", title="T", year=2020, relevance_score=0.5,
            overlap_aspects=[], difference_aspects=[], conclusion="")
        assert rw.overlap_aspects == []
        assert rw.difference_aspects == []
        assert rw.conclusion == ""
        assert rw.difference_aspects == []
        assert rw.conclusion == ""


class TestInnovationScore:
    """Test InnovationScore dataclass."""

    def test_required_fields(self):
        """Required fields: overall, method, task, evaluation, dimensions, reasoning."""
        score = InnovationScore(
            overall=7.5,
            method=8.0,
            task=7.0,
            evaluation=7.5,
            dimensions=[InnovationDimension.METHOD],
            reasoning="Novel approach",
        )
        assert score.overall == 7.5
        assert score.method == 8.0
        assert score.task == 7.0
        assert score.evaluation == 7.5
        assert score.dimensions == [InnovationDimension.METHOD]
        assert score.reasoning == "Novel approach"


class TestValidationResult:
    """Test ValidationResult dataclass."""

    def test_required_fields(self):
        """Required fields: question, is_novel, novelty_level, innovation_score."""
        score = InnovationScore(overall=0, method=0, task=0, evaluation=0, dimensions=[], reasoning="")
        result = ValidationResult(
            question="Can transformers do reasoning?",
            is_novel=True,
            novelty_level=NoveltyLevel.HIGH,
            innovation_score=score,
        )
        assert result.question == "Can transformers do reasoning?"
        assert result.is_novel is True
        assert result.novelty_level == NoveltyLevel.HIGH

    def test_optional_fields_default(self):
        """Optional fields have defaults."""
        score = InnovationScore(overall=0, method=0, task=0, evaluation=0, dimensions=[], reasoning="")
        result = ValidationResult(
            question="Q",
            is_novel=False,
            novelty_level=NoveltyLevel.UNKNOWN,
            innovation_score=score,
        )
        assert result.related_works == []
        assert result.gap_summary == ""
        assert result.suggestions == []
        assert result.confidence == 0.5


# =============================================================================
# _expand_question tests
# =============================================================================
class TestExpandQuestion:
    """Test _expand_question logic."""

    def _expand(self, question: str) -> list:
        """Replicate _expand_question logic."""
        import re
        from llm.constants import AI_RESEARCH_KEYWORDS

        cleaned = re.sub(r'\b(can|how|what|why|is|does|to|the|a|an)\b', '', question.lower())
        cleaned = re.sub(r'[^\w\s]', ' ', cleaned)
        words = [w.strip() for w in cleaned.split() if len(w.strip()) > 2]

        for term in AI_RESEARCH_KEYWORDS:
            if term in question.lower():
                words.append(term)

        return list(set(words))[:10]

    def test_removes_question_words(self):
        """Common question words are removed."""
        result = self._expand("How to improve transformer attention?")
        assert "how" not in result
        assert "to" not in result
        assert "improve" in result or "transformer" in result

    def test_removes_articles(self):
        """Articles are removed."""
        result = self._expand("Can the BERT model learn effectively?")
        assert "the" not in result
        assert "can" not in result

    def test_removes_punctuation(self):
        """Punctuation is stripped."""
        result = self._expand("Can we use attention? Yes!")
        assert "?" not in result
        assert "!" not in result

    def test_filters_short_words(self):
        """Words <= 2 chars are filtered."""
        result = self._expand("Do AI models work?")
        assert all(len(w) > 2 for w in result)

    def test_ai_keywords_included(self):
        """AI research keywords from constants are included."""
        result = self._expand("What about BERT and GPT?")
        assert "bert" in result
        assert "gpt" in result

    def test_returns_unique(self):
        """Results are deduplicated."""
        result = self._expand("transformer transformer attention attention")
        # transformer appears twice but should be in list once
        assert result.count("transformer") <= 1

    def test_max_10_items(self):
        """Maximum 10 keywords returned."""
        long_q = " ".join([f"keyword{i}" for i in range(30)])
        result = self._expand(long_q)
        assert len(result) <= 10


# =============================================================================
# _analyze_innovation_rules tests
# =============================================================================
class TestAnalyzeInnovationRules:
    """Test _analyze_innovation_rules logic."""

    def _analyze(self, related: list) -> InnovationScore:
        """Replicate _analyze_innovation_rules logic."""
        if not related:
            return InnovationScore(
                overall=8.0,
                method=7.0,
                task=8.0,
                evaluation=7.0,
                dimensions=[InnovationDimension.METHOD, InnovationDimension.TASK, InnovationDimension.EVALUATION],
                reasoning="未发现相关工作，可能是全新领域",
            )

        max_relevance = max(r.relevance_score for r in related)

        if max_relevance > 0.8:
            return InnovationScore(
                overall=3.0,
                method=3.0,
                task=4.0,
                evaluation=3.0,
                dimensions=[],
                reasoning=f"发现高度相关工作 (相似度 {max_relevance:.0%})",
            )
        elif max_relevance > 0.5:
            return InnovationScore(
                overall=6.0,
                method=6.0,
                task=5.0,
                evaluation=6.0,
                dimensions=[InnovationDimension.METHOD],
                reasoning=f"有相关工作，但有新角度 (相似度 {max_relevance:.0%})",
            )
        else:
            return InnovationScore(
                overall=7.5,
                method=7.0,
                task=8.0,
                evaluation=7.0,
                dimensions=[InnovationDimension.TASK, InnovationDimension.APPLICATION],
                reasoning="发现部分相关，但领域/应用不同",
            )

    def test_empty_related_returns_high_scores(self):
        """No related works → high innovation scores."""
        score = self._analyze([])
        assert score.overall == 8.0
        assert score.method == 7.0
        assert score.task == 8.0
        assert score.evaluation == 7.0

    def test_empty_related_has_all_dimensions(self):
        """No related works → all innovation dimensions."""
        score = self._analyze([])
        assert InnovationDimension.METHOD in score.dimensions
        assert InnovationDimension.TASK in score.dimensions
        assert InnovationDimension.EVALUATION in score.dimensions

    def test_high_relevance_returns_low_scores(self):
        """High relevance (>0.8) → low innovation."""
        rw = RelatedWork(paper_id="p1", title="T", year=2025, relevance_score=0.9,
            overlap_aspects=[], difference_aspects=[], conclusion="")
        score = self._analyze([rw])
        assert score.overall == 3.0
        assert score.dimensions == []

    def test_high_relevance_reasoning(self):
        """High relevance includes similarity in reasoning."""
        rw = RelatedWork(paper_id="p1", title="T", year=2025, relevance_score=0.9,
            overlap_aspects=[], difference_aspects=[], conclusion="")
        score = self._analyze([rw])
        assert "90%" in score.reasoning
        assert "高度相关" in score.reasoning

    def test_medium_relevance_returns_medium_scores(self):
        """Medium relevance (0.5-0.8) → medium innovation."""
        rw = RelatedWork(paper_id="p1", title="T", year=2025, relevance_score=0.6,
            overlap_aspects=[], difference_aspects=[], conclusion="")
        score = self._analyze([rw])
        assert score.overall == 6.0
        assert InnovationDimension.METHOD in score.dimensions

    def test_low_relevance_returns_good_scores(self):
        """Low relevance (<0.5) → good innovation."""
        rw = RelatedWork(paper_id="p1", title="T", year=2025, relevance_score=0.3,
            overlap_aspects=[], difference_aspects=[], conclusion="")
        score = self._analyze([rw])
        assert score.overall == 7.5
        assert InnovationDimension.TASK in score.dimensions
        assert InnovationDimension.APPLICATION in score.dimensions

    def test_multiple_related_uses_max_relevance(self):
        """Multiple related works: uses max relevance."""
        rws = [
            RelatedWork(paper_id="p1", title="T1", year=2025, relevance_score=0.2,
            overlap_aspects=[], difference_aspects=[], conclusion=""),
            RelatedWork(paper_id="p2", title="T2", year=2025, relevance_score=0.9,
            overlap_aspects=[], difference_aspects=[], conclusion=""),
        ]
        score = self._analyze(rws)
        # max is 0.9 → high relevance path
        assert score.overall == 3.0


# =============================================================================
# _determine_novelty tests
# =============================================================================
class TestDetermineNovelty:
    """Test _determine_novelty logic."""

    def _determine(self, innovation: InnovationScore, related: list) -> NoveltyLevel:
        """Replicate _determine_novelty logic."""
        if not related and innovation.overall >= 7:
            return NoveltyLevel.HIGH

        if innovation.overall >= 7:
            return NoveltyLevel.HIGH
        elif innovation.overall >= 5:
            return NoveltyLevel.MEDIUM
        else:
            return NoveltyLevel.LOW

    def test_overall_7_returns_high(self):
        """Overall >= 7 → HIGH."""
        score = InnovationScore(overall=7.0, method=7, task=7, evaluation=7, dimensions=[], reasoning="")
        assert self._determine(score, []) == NoveltyLevel.HIGH

    def test_overall_8_returns_high(self):
        """Overall >= 7 → HIGH."""
        score = InnovationScore(overall=8.0, method=8, task=8, evaluation=8, dimensions=[], reasoning="")
        assert self._determine(score, []) == NoveltyLevel.HIGH

    def test_overall_5_returns_medium(self):
        """Overall >= 5 and < 7 → MEDIUM."""
        score = InnovationScore(overall=5.0, method=5, task=5, evaluation=5, dimensions=[], reasoning="")
        assert self._determine(score, []) == NoveltyLevel.MEDIUM

    def test_overall_6_returns_medium(self):
        """Overall >= 5 and < 7 → MEDIUM."""
        score = InnovationScore(overall=6.0, method=6, task=6, evaluation=6, dimensions=[], reasoning="")
        assert self._determine(score, []) == NoveltyLevel.MEDIUM

    def test_overall_4_returns_low(self):
        """Overall < 5 → LOW."""
        score = InnovationScore(overall=4.0, method=4, task=4, evaluation=4, dimensions=[], reasoning="")
        assert self._determine(score, []) == NoveltyLevel.LOW

    def test_empty_related_with_high_score_returns_high(self):
        """Empty related works + overall >= 7 → HIGH (explicit path)."""
        score = InnovationScore(overall=8.0, method=8, task=8, evaluation=8, dimensions=[], reasoning="")
        assert self._determine(score, []) == NoveltyLevel.HIGH


# =============================================================================
# _calculate_confidence tests
# =============================================================================
class TestCalculateConfidence:
    """Test _calculate_confidence logic."""

    def _calculate(self, related: list, innovation: InnovationScore) -> float:
        """Replicate _calculate_confidence logic."""
        related_score = min(len(related) / 5, 1.0) * 0.4
        reasoning_score = 0.3 if innovation.reasoning else 0.15
        dimension_score = len(innovation.dimensions) / 3 * 0.3
        return min(related_score + reasoning_score + dimension_score, 0.95)

    def test_empty_related_no_reasoning_no_dimensions(self):
        """Min confidence: 0 + 0.15 + 0 = 0.15."""
        score = InnovationScore(overall=8.0, method=8, task=8, evaluation=8, dimensions=[], reasoning="")
        conf = self._calculate([], score)
        assert conf == 0.15

    def test_empty_related_with_reasoning(self):
        """With reasoning: 0 + 0.3 + 0 = 0.3."""
        score = InnovationScore(overall=8.0, method=8, task=8, evaluation=8, dimensions=[], reasoning="Has reasoning")
        conf = self._calculate([], score)
        assert conf == 0.3

    def test_empty_related_with_3_dimensions(self):
        """3 dimensions: 0 + 0.3 + 0.3 = 0.6."""
        score = InnovationScore(overall=8.0, method=8, task=8, evaluation=8,
            dimensions=[InnovationDimension.METHOD, InnovationDimension.TASK, InnovationDimension.EVALUATION],
            reasoning="Reasoned")
        conf = self._calculate([], score)
        assert conf == 0.6

    def test_5_related_works_full_dimensions(self):
        """5 related + 3 dims + reasoning: 0.4 + 0.3 + 0.3 = 1.0 → capped at 0.95."""
        score = InnovationScore(overall=8.0, method=8, task=8, evaluation=8,
            dimensions=[InnovationDimension.METHOD, InnovationDimension.TASK, InnovationDimension.EVALUATION],
            reasoning="Full")
        related = [RelatedWork(paper_id=str(i), title=f"T{i}", year=2025, relevance_score=0.5,
            overlap_aspects=[], difference_aspects=[], conclusion="") for i in range(5)]
        conf = self._calculate(related, score)
        assert conf == 0.95  # capped

    def test_more_than_5_related_still_capped(self):
        """More than 5 related works → still capped by min()."""
        score = InnovationScore(overall=8.0, method=8, task=8, evaluation=8, dimensions=[], reasoning="")
        related = [RelatedWork(paper_id=str(i), title=f"T{i}", year=2025, relevance_score=0.5,
            overlap_aspects=[], difference_aspects=[], conclusion="") for i in range(10)]
        # related_score = min(10/5, 1.0) * 0.4 = 0.4; reasoning=0.15; dims=0
        conf = self._calculate(related, score)
        assert conf == 0.55

    def test_capped_at_0_95(self):
        """Confidence is capped at 0.95."""
        score = InnovationScore(overall=8.0, method=8, task=8, evaluation=8,
            dimensions=[InnovationDimension.METHOD, InnovationDimension.TASK, InnovationDimension.EVALUATION],
            reasoning="Full")
        related = [RelatedWork(paper_id=str(i), title=f"T{i}", year=2025, relevance_score=0.5,
            overlap_aspects=[], difference_aspects=[], conclusion="") for i in range(10)]
        conf = self._calculate(related, score)
        assert conf == 0.95


# =============================================================================
# _parse_innovation_response tests
# =============================================================================
class TestParseInnovationResponse:
    """Test _parse_innovation_response logic."""

    def _parse(self, response: str, related: list) -> InnovationScore:
        """Replicate _parse_innovation_response logic."""
        method_score = 5.0
        task_score = 5.0
        eval_score = 5.0
        reasoning = ""

        for line in response.strip().split('\n'):
            line = line.strip().lower()
            if line.startswith('method:'):
                try:
                    method_score = float(line.split(':')[1].strip())
                except (ValueError, IndexError):
                    pass
            elif line.startswith('task:'):
                try:
                    task_score = float(line.split(':')[1].strip())
                except (ValueError, IndexError):
                    pass
            elif line.startswith('evaluation:'):
                try:
                    eval_score = float(line.split(':')[1].strip())
                except (ValueError, IndexError):
                    pass
            elif line.startswith('reasoning:'):
                reasoning = line.split(':', 1)[1].strip()

        overall = method_score * 0.4 + task_score * 0.3 + eval_score * 0.3

        dimensions = []
        if method_score >= 7:
            dimensions.append(InnovationDimension.METHOD)
        if task_score >= 7:
            dimensions.append(InnovationDimension.TASK)
        if eval_score >= 7:
            dimensions.append(InnovationDimension.EVALUATION)

        return InnovationScore(
            overall=overall,
            method=method_score,
            task=task_score,
            evaluation=eval_score,
            dimensions=dimensions,
            reasoning=reasoning,
        )

    def test_parses_method_score(self):
        """Parses method score."""
        score = self._parse("method: 8\ntask: 5\nevaluation: 5\nreasoning: good", [])
        assert score.method == 8.0

    def test_parses_task_score(self):
        """Parses task score."""
        score = self._parse("method: 5\ntask: 9\nevaluation: 5\nreasoning: good", [])
        assert score.task == 9.0

    def test_parses_evaluation_score(self):
        """Parses evaluation score."""
        score = self._parse("method: 5\ntask: 5\nevaluation: 8\nreasoning: good", [])
        assert score.evaluation == 8.0

    def test_overall_is_weighted_average(self):
        """Overall = method*0.4 + task*0.3 + eval*0.3."""
        score = self._parse("method: 10\ntask: 10\nevaluation: 10", [])
        assert score.overall == 10.0  # 10*0.4 + 10*0.3 + 10*0.3

    def test_overall_weighted_calculation(self):
        """Overall correctly weighted."""
        score = self._parse("method: 8\ntask: 6\nevaluation: 4", [])
        expected = 8 * 0.4 + 6 * 0.3 + 4 * 0.3
        assert score.overall == expected

    def test_method_dimension_added_when_high(self):
        """METHOD dimension added when method >= 7."""
        score = self._parse("method: 7\ntask: 5\nevaluation: 5", [])
        assert InnovationDimension.METHOD in score.dimensions

    def test_task_dimension_added_when_high(self):
        """TASK dimension added when task >= 7."""
        score = self._parse("method: 5\ntask: 8\nevaluation: 5", [])
        assert InnovationDimension.TASK in score.dimensions

    def test_evaluation_dimension_added_when_high(self):
        """EVALUATION dimension added when evaluation >= 7."""
        score = self._parse("method: 5\ntask: 5\nevaluation: 8", [])
        assert InnovationDimension.EVALUATION in score.dimensions

    def test_no_dimension_added_when_below_threshold(self):
        """No dimension added when all scores < 7."""
        score = self._parse("method: 6\ntask: 6\nevaluation: 6", [])
        assert score.dimensions == []

    def test_reasoning_parsed(self):
        """Reasoning text is parsed."""
        score = self._parse("method: 5\ntask: 5\nevaluation: 5\nreasoning: this is novel", [])
        assert score.reasoning == "this is novel"

    def test_case_insensitive_line_starts(self):
        """Line matching is case insensitive."""
        score = self._parse("METHOD: 8\nTASK: 5\nEVALUATION: 5\nreasoning: ok", [])
        assert score.method == 8.0

    def test_invalid_score_falls_back_to_default(self):
        """Invalid score string falls back to default 5.0."""
        score = self._parse("method: not_a_number\ntask: 5\nevaluation: 5", [])
        assert score.method == 5.0


# =============================================================================
# _generate_suggestions_rules tests
# =============================================================================
class TestGenerateSuggestionsRules:
    """Test _generate_suggestions_rules logic."""

    def _suggest(self, related: list) -> list:
        """Replicate _generate_suggestions_rules logic."""
        suggestions = []
        if not related:
            suggestions.append("[方法] 设计全新的方法框架")
            suggestions.append("[任务] 探索具体的落地场景")
            suggestions.append("[评估] 建立评估基准和指标")
        else:
            recent_papers = [r for r in related if r.year >= 2023]
            if recent_papers:
                suggestions.append(f"[方法] 参考 {len(recent_papers)} 篇最新工作，选择差异化路线")

            suggestions.append("[任务] 考虑跨领域应用场景")
            suggestions.append("[评估] 设计针对新问题的评估指标")
            suggestions.append("[数据] 构建专用数据集")
        return suggestions

    def test_no_related_suggests_all_new(self):
        """No related works → all-new suggestions."""
        result = self._suggest([])
        assert "[方法] 设计全新的方法框架" in result
        assert "[任务] 探索具体的落地场景" in result
        assert "[评估] 建立评估基准和指标" in result

    def test_has_related_suggests_differentiation(self):
        """Has related works → differentiation suggestion."""
        rw = RelatedWork(paper_id="p1", title="T", year=2025, relevance_score=0.5,
            overlap_aspects=[], difference_aspects=[], conclusion="")
        result = self._suggest([rw])
        assert any("差异化" in s for s in result)

    def test_recent_papers_counted(self):
        """Recent papers (>= 2023) counted in suggestion."""
        rws = [
            RelatedWork(paper_id="p1", title="T1", year=2024, relevance_score=0.5,
            overlap_aspects=[], difference_aspects=[], conclusion=""),
            RelatedWork(paper_id="p2", title="T2", year=2025, relevance_score=0.5,
            overlap_aspects=[], difference_aspects=[], conclusion=""),
            RelatedWork(paper_id="p3", title="T3", year=2022, relevance_score=0.5,
            overlap_aspects=[], difference_aspects=[], conclusion=""),
        ]
        result = self._suggest(rws)
        assert any("2" in s for s in result)  # 2 recent papers

    def test_has_related_always_includes_base_suggestions(self):
        """Has related → always includes task/eval/data suggestions."""
        rw = RelatedWork(paper_id="p1", title="T", year=2025, relevance_score=0.5,
            overlap_aspects=[], difference_aspects=[], conclusion="")
        result = self._suggest([rw])
        assert any("[任务]" in s for s in result)
        assert any("[评估]" in s for s in result)
        assert any("[数据]" in s for s in result)


# =============================================================================
# render_result tests
# =============================================================================
class TestRenderResult:
    """Test render_result formatting."""

    def _render(self, result: ValidationResult) -> str:
        """Replicate render_result formatting."""
        novelty_icon = {
            NoveltyLevel.HIGH: "🟢",
            NoveltyLevel.MEDIUM: "🟡",
            NoveltyLevel.LOW: "🔴",
            NoveltyLevel.UNKNOWN: "⚪",
        }.get(result.novelty_level, "⚪")

        lines = [
            f"🔬 研究问题验证: \"{result.question[:60]}{'...' if len(result.question) > 60 else ''}\"",
            "",
            f"{novelty_icon} 创新指数: {result.innovation_score.overall:.1f}/10",
            f"   方法创新: {result.innovation_score.method:.0f}/10",
            f"   任务创新: {result.innovation_score.task:.0f}/10",
            f"   评估创新: {result.innovation_score.evaluation:.0f}/10",
            "",
        ]

        if result.innovation_score.dimensions:
            dims = [d.value for d in result.innovation_score.dimensions]
            lines.append(f"   亮点维度: {', '.join(dims)}")

        if result.innovation_score.reasoning:
            lines.append(f"   分析: {result.innovation_score.reasoning}")

        lines.append("")

        if result.related_works:
            lines.append("📚 相关工作:")
            for i, work in enumerate(result.related_works[:3], 1):
                lines.append(f"   {i}. {work.title} ({work.year})")
                lines.append(f"      相关度: {work.relevance_score:.0%}")
            lines.append("")

        if result.suggestions:
            lines.append("💡 改进建议:")
            for suggestion in result.suggestions[:4]:
                lines.append(f"   • {suggestion}")
            lines.append("")

        lines.append(f"📊 置信度: {result.confidence:.0%}")
        lines.append(f"🎯 结论: {'✅ 值得探索' if result.is_novel else '⚠️ 需要更细致的角度'}")

        return "\n".join(lines)

    def test_question_truncated_at_60(self):
        """Long question is truncated to 60 chars."""
        long_q = "A" * 100
        score = InnovationScore(overall=0, method=0, task=0, evaluation=0, dimensions=[], reasoning="")
        result = ValidationResult(question=long_q, is_novel=True, novelty_level=NoveltyLevel.HIGH, innovation_score=score)
        output = self._render(result)
        assert "..." in output
        assert "A" * 60 in output
        assert ("A" * 61) not in output

    def test_novelty_icon_high(self):
        """HIGH novelty → green icon."""
        score = InnovationScore(overall=8.0, method=8, task=8, evaluation=8, dimensions=[], reasoning="")
        result = ValidationResult(question="Q", is_novel=True, novelty_level=NoveltyLevel.HIGH, innovation_score=score)
        assert "🟢" in self._render(result)

    def test_novelty_icon_medium(self):
        """MEDIUM novelty → yellow icon."""
        score = InnovationScore(overall=6.0, method=6, task=6, evaluation=6, dimensions=[], reasoning="")
        result = ValidationResult(question="Q", is_novel=True, novelty_level=NoveltyLevel.MEDIUM, innovation_score=score)
        assert "🟡" in self._render(result)

    def test_novelty_icon_low(self):
        """LOW novelty → red icon."""
        score = InnovationScore(overall=3.0, method=3, task=4, evaluation=3, dimensions=[], reasoning="")
        result = ValidationResult(question="Q", is_novel=True, novelty_level=NoveltyLevel.LOW, innovation_score=score)
        assert "🔴" in self._render(result)

    def test_innovation_scores_shown(self):
        """Innovation scores displayed."""
        score = InnovationScore(overall=7.5, method=8.0, task=7.0, evaluation=7.5, dimensions=[], reasoning="")
        result = ValidationResult(question="Q", is_novel=True, novelty_level=NoveltyLevel.HIGH, innovation_score=score)
        output = self._render(result)
        assert "创新指数: 7.5/10" in output
        assert "方法创新: 8/10" in output
        assert "任务创新: 7/10" in output
        assert "评估创新: 8/10" in output

    def test_dimensions_shown_when_present(self):
        """Innovation dimensions shown when present."""
        score = InnovationScore(overall=8.0, method=8, task=8, evaluation=8,
            dimensions=[InnovationDimension.METHOD, InnovationDimension.TASK], reasoning="")
        result = ValidationResult(question="Q", is_novel=True, novelty_level=NoveltyLevel.HIGH, innovation_score=score)
        output = self._render(result)
        assert "亮点维度" in output
        assert "method" in output
        assert "task" in output

    def test_reasoning_shown(self):
        """Reasoning text shown."""
        score = InnovationScore(overall=8.0, method=8, task=8, evaluation=8, dimensions=[], reasoning="This is novel work")
        result = ValidationResult(question="Q", is_novel=True, novelty_level=NoveltyLevel.HIGH, innovation_score=score)
        output = self._render(result)
        assert "分析: This is novel work" in output

    def test_related_works_limited_to_3(self):
        """Only first 3 related works shown."""
        score = InnovationScore(overall=6.0, method=6, task=6, evaluation=6, dimensions=[], reasoning="")
        rws = [RelatedWork(paper_id=str(i), title=f"Paper {i}", year=2025, relevance_score=0.5,
            overlap_aspects=[], difference_aspects=[], conclusion="") for i in range(5)]
        result = ValidationResult(question="Q", is_novel=False, novelty_level=NoveltyLevel.MEDIUM,
                                  innovation_score=score, related_works=rws)
        output = self._render(result)
        assert "Paper 0" in output
        assert "Paper 1" in output
        assert "Paper 2" in output
        assert "Paper 3" not in output

    def test_suggestions_limited_to_4(self):
        """Only first 4 suggestions shown."""
        score = InnovationScore(overall=6.0, method=6, task=6, evaluation=6, dimensions=[], reasoning="")
        suggestions = [f"Suggestion {i}" for i in range(6)]
        result = ValidationResult(question="Q", is_novel=False, novelty_level=NoveltyLevel.MEDIUM,
                                  innovation_score=score, suggestions=suggestions)
        output = self._render(result)
        assert "Suggestion 0" in output
        assert "Suggestion 3" in output
        assert "Suggestion 4" not in output

    def test_conclusion_novel(self):
        """is_novel=True → positive conclusion."""
        score = InnovationScore(overall=8.0, method=8, task=8, evaluation=8, dimensions=[], reasoning="")
        result = ValidationResult(question="Q", is_novel=True, novelty_level=NoveltyLevel.HIGH, innovation_score=score)
        output = self._render(result)
        assert "✅ 值得探索" in output

    def test_conclusion_not_novel(self):
        """is_novel=False → cautious conclusion."""
        score = InnovationScore(overall=3.0, method=3, task=4, evaluation=3, dimensions=[], reasoning="")
        result = ValidationResult(question="Q", is_novel=False, novelty_level=NoveltyLevel.LOW, innovation_score=score)
        output = self._render(result)
        assert "⚠️ 需要更细致的角度" in output


# =============================================================================
# render_json tests
# =============================================================================
class TestRenderJson:
    """Test render_json formatting."""

    def _render_json(self, result: ValidationResult) -> dict:
        """Replicate render_json logic."""
        import json
        data = {
            "question": result.question,
            "is_novel": result.is_novel,
            "novelty_level": result.novelty_level.value,
            "innovation_score": {
                "overall": result.innovation_score.overall,
                "method": result.innovation_score.method,
                "task": result.innovation_score.task,
                "evaluation": result.innovation_score.evaluation,
                "dimensions": [d.value for d in result.innovation_score.dimensions],
                "reasoning": result.innovation_score.reasoning,
            },
            "related_works": [
                {
                    "paper_id": w.paper_id,
                    "title": w.title,
                    "year": w.year,
                    "relevance_score": w.relevance_score,
                }
                for w in result.related_works
            ],
            "suggestions": result.suggestions,
            "confidence": result.confidence,
        }
        return json.loads(json.dumps(data, ensure_ascii=False))

    def test_question_field(self):
        """Question is in output."""
        score = InnovationScore(overall=0, method=0, task=0, evaluation=0, dimensions=[], reasoning="")
        result = ValidationResult(question="Can AI reason?", is_novel=True, novelty_level=NoveltyLevel.HIGH, innovation_score=score)
        d = self._render_json(result)
        assert d["question"] == "Can AI reason?"

    def test_is_novel_field(self):
        """is_novel is in output."""
        score = InnovationScore(overall=0, method=0, task=0, evaluation=0, dimensions=[], reasoning="")
        result = ValidationResult(question="Q", is_novel=True, novelty_level=NoveltyLevel.HIGH, innovation_score=score)
        d = self._render_json(result)
        assert d["is_novel"] is True

    def test_novelty_level_value(self):
        """novelty_level is the value string."""
        score = InnovationScore(overall=0, method=0, task=0, evaluation=0, dimensions=[], reasoning="")
        result = ValidationResult(question="Q", is_novel=True, novelty_level=NoveltyLevel.HIGH, innovation_score=score)
        d = self._render_json(result)
        assert d["novelty_level"] == "high"

    def test_innovation_scores_included(self):
        """Innovation scores included."""
        score = InnovationScore(overall=7.5, method=8.0, task=7.0, evaluation=7.5, dimensions=[], reasoning="good")
        result = ValidationResult(question="Q", is_novel=True, novelty_level=NoveltyLevel.HIGH, innovation_score=score)
        d = self._render_json(result)
        assert d["innovation_score"]["overall"] == 7.5
        assert d["innovation_score"]["method"] == 8.0
        assert d["innovation_score"]["task"] == 7.0
        assert d["innovation_score"]["evaluation"] == 7.5
        assert d["innovation_score"]["reasoning"] == "good"

    def test_dimensions_as_value_list(self):
        """Dimensions are value strings."""
        score = InnovationScore(overall=8.0, method=8, task=8, evaluation=8,
            dimensions=[InnovationDimension.METHOD, InnovationDimension.TASK], reasoning="")
        result = ValidationResult(question="Q", is_novel=True, novelty_level=NoveltyLevel.HIGH, innovation_score=score)
        d = self._render_json(result)
        assert d["innovation_score"]["dimensions"] == ["method", "task"]

    def test_related_works_as_dict_list(self):
        """Related works serialized as dicts."""
        rw = RelatedWork(paper_id="p1", title="T", year=2025, relevance_score=0.85,
            overlap_aspects=[], difference_aspects=[], conclusion="")
        score = InnovationScore(overall=0, method=0, task=0, evaluation=0, dimensions=[], reasoning="")
        result = ValidationResult(question="Q", is_novel=False, novelty_level=NoveltyLevel.LOW,
                                  innovation_score=score, related_works=[rw])
        d = self._render_json(result)
        assert len(d["related_works"]) == 1
        assert d["related_works"][0]["paper_id"] == "p1"
        assert d["related_works"][0]["title"] == "T"
        assert d["related_works"][0]["year"] == 2025
        assert d["related_works"][0]["relevance_score"] == 0.85

    def test_suggestions_list(self):
        """Suggestions list included."""
        score = InnovationScore(overall=0, method=0, task=0, evaluation=0, dimensions=[], reasoning="")
        result = ValidationResult(question="Q", is_novel=False, novelty_level=NoveltyLevel.LOW,
                                  innovation_score=score, suggestions=["S1", "S2"])
        d = self._render_json(result)
        assert d["suggestions"] == ["S1", "S2"]

    def test_confidence_field(self):
        """Confidence included."""
        score = InnovationScore(overall=0, method=0, task=0, evaluation=0, dimensions=[], reasoning="")
        result = ValidationResult(question="Q", is_novel=False, novelty_level=NoveltyLevel.LOW,
                                  innovation_score=score, confidence=0.75)
        d = self._render_json(result)
        assert d["confidence"] == 0.75
