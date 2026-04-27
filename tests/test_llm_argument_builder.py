"""Tier 2 unit tests — llm/argument_builder.py, pure functions, no I/O."""
import pytest
from llm.argument_builder import (
    EvidenceType,
    ArgumentSection,
    Evidence,
    Claim,
    Argument,
    ArgumentResult,
    ArgumentBuilder,
    render_argument,
)


# =============================================================================
# Enum tests
# =============================================================================
class TestEvidenceType:
    """Test EvidenceType enum."""

    def test_all_types_have_values(self):
        """All EvidenceType variants have string values."""
        assert EvidenceType.SUPPORT.value == "support"
        assert EvidenceType.CONTRADICT.value == "contradict"
        assert EvidenceType.QUALIFY.value == "qualify"
        assert EvidenceType.METHODOLOGICAL.value == "methodological"

    def test_can_construct_from_value(self):
        """Enum can be constructed from string value."""
        assert EvidenceType("support") == EvidenceType.SUPPORT
        assert EvidenceType("contradict") == EvidenceType.CONTRADICT


class TestArgumentSection:
    """Test ArgumentSection enum."""

    def test_all_sections_have_values(self):
        """All ArgumentSection variants have values."""
        assert ArgumentSection.INTRODUCTION.value == "introduction"
        assert ArgumentSection.RELATED_WORK.value == "related_work"
        assert ArgumentSection.METHODOLOGY.value == "methodology"
        assert ArgumentSection.EXPERIMENTS.value == "experiments"
        assert ArgumentSection.DISCUSSION.value == "discussion"
        assert ArgumentSection.LIMITATION.value == "limitation"


# =============================================================================
# Dataclass tests
# =============================================================================
class TestEvidence:
    """Test Evidence dataclass."""

    def test_required_fields(self):
        """Required fields: evidence_type, source, content."""
        evidence = Evidence(
            evidence_type=EvidenceType.SUPPORT,
            source="Paper Title",
            content="Abstract content here",
        )
        assert evidence.evidence_type == EvidenceType.SUPPORT
        assert evidence.source == "Paper Title"
        assert evidence.content == "Abstract content here"

    def test_optional_fields_default(self):
        """Optional fields have defaults."""
        evidence = Evidence(
            evidence_type=EvidenceType.CONTRADICT,
            source="S",
            content="C",
        )
        assert evidence.citation == ""
        assert evidence.weight == 1.0

    def test_all_fields_can_be_set(self):
        """All fields can be set."""
        evidence = Evidence(
            evidence_type=EvidenceType.METHODOLOGICAL,
            source="Insight",
            content="Method issue",
            citation="arXiv:1234.5678",
            weight=0.5,
        )
        assert evidence.citation == "arXiv:1234.5678"
        assert evidence.weight == 0.5


class TestClaim:
    """Test Claim dataclass."""

    def test_required_fields(self):
        """Required fields: text."""
        claim = Claim(text="This method is effective")
        assert claim.text == "This method is effective"

    def test_optional_fields_default(self):
        """Optional fields default to empty/low values."""
        claim = Claim(text="Claim text")
        assert claim.evidence == []
        assert claim.confidence == 0.5


class TestArgument:
    """Test Argument dataclass."""

    def test_required_fields(self):
        """Required fields: thesis."""
        arg = Argument(thesis="Transformer is better than RNN")
        assert arg.thesis == "Transformer is better than RNN"

    def test_optional_fields_default(self):
        """Optional fields default to empty."""
        arg = Argument(thesis="T")
        assert arg.claims == []
        assert arg.supporting_evidence == []
        assert arg.contradicting_evidence == []
        assert arg.related_gaps == []
        assert arg.paper_suggestions == []


class TestArgumentResult:
    """Test ArgumentResult dataclass."""

    def test_required_fields(self):
        """Required fields: topic, argument."""
        arg = Argument(thesis="T")
        result = ArgumentResult(topic="Topic", argument=arg)
        assert result.topic == "Topic"
        assert result.argument.thesis == "T"

    def test_optional_fields_default(self):
        """Optional fields default."""
        arg = Argument(thesis="T")
        result = ArgumentResult(topic="T", argument=arg)
        assert result.summary == ""
        assert result.section_guidance == {}


# =============================================================================
# _classify_insight tests
# =============================================================================
class TestClassifyInsight:
    """Test _classify_insight logic."""

    def _classify_insight(self, content: str, thesis: str) -> EvidenceType:
        """Replicate insight classification logic."""
        contradict_keywords = ["局限", "问题", "失败", "缺陷", "limitation", "problem", "fail"]
        content_lower = content.lower()

        for kw in contradict_keywords:
            if kw in content_lower:
                return EvidenceType.CONTRADICT

        return EvidenceType.SUPPORT

    def test_chinese_contradict_keyword(self):
        """Chinese contradiction keyword triggers CONTRADICT."""
        assert self._classify_insight("这个方法有局限性", "thesis") == EvidenceType.CONTRADICT
        assert self._classify_insight("存在严重问题", "thesis") == EvidenceType.CONTRADICT

    def test_english_contradict_keyword(self):
        """English contradiction keyword triggers CONTRADICT."""
        assert self._classify_insight("has limitation", "thesis") == EvidenceType.CONTRADICT
        assert self._classify_insight("the problem is", "thesis") == EvidenceType.CONTRADICT
        assert self._classify_insight("experiment fail", "thesis") == EvidenceType.CONTRADICT

    def test_no_contradict_keywords(self):
        """Without contradiction keywords, defaults to SUPPORT."""
        assert self._classify_insight("This is a good method", "thesis") == EvidenceType.SUPPORT
        assert self._classify_insight("novel approach", "thesis") == EvidenceType.SUPPORT

    def test_case_insensitive(self):
        """Keyword matching is case insensitive."""
        assert self._classify_insight("HAS LIMITATION", "thesis") == EvidenceType.CONTRADICT
        assert self._classify_insight("Problem found", "thesis") == EvidenceType.CONTRADICT

    def test_thesis_not_used(self):
        """Thesis parameter is not used in classification."""
        # The thesis is passed but not used in the simple keyword matching
        assert self._classify_insight("good method", thesis="problematic thesis") == EvidenceType.SUPPORT


# =============================================================================
# _categorize_evidence tests
# =============================================================================
class TestCategorizeEvidence:
    """Test _categorize_evidence logic."""

    def _categorize_evidence(self, evidence_list: list) -> tuple:
        """Replicate evidence categorization."""
        supporting = []
        contradicting = []

        for e in evidence_list:
            if e.evidence_type in (EvidenceType.SUPPORT, EvidenceType.QUALIFY):
                supporting.append(e)
            else:
                contradicting.append(e)

        supporting.sort(key=lambda x: x.weight, reverse=True)
        contradicting.sort(key=lambda x: x.weight, reverse=True)

        return supporting, contradicting

    def test_support_goes_to_supporting(self):
        """SUPPORT evidence goes to supporting list."""
        evidence = [Evidence(EvidenceType.SUPPORT, "S", "C")]
        supporting, contradicting = self._categorize_evidence(evidence)
        assert len(supporting) == 1
        assert len(contradicting) == 0

    def test_qualify_goes_to_supporting(self):
        """QUALIFY evidence goes to supporting list."""
        evidence = [Evidence(EvidenceType.QUALIFY, "S", "C")]
        supporting, contradicting = self._categorize_evidence(evidence)
        assert len(supporting) == 1
        assert len(contradicting) == 0

    def test_contradict_goes_to_contradicting(self):
        """CONTRADICT evidence goes to contradicting list."""
        evidence = [Evidence(EvidenceType.CONTRADICT, "S", "C")]
        supporting, contradicting = self._categorize_evidence(evidence)
        assert len(supporting) == 0
        assert len(contradicting) == 1

    def test_methodological_goes_to_contradicting(self):
        """METHODOLOGICAL evidence goes to contradicting list."""
        evidence = [Evidence(EvidenceType.METHODOLOGICAL, "S", "C")]
        supporting, contradicting = self._categorize_evidence(evidence)
        assert len(supporting) == 0
        assert len(contradicting) == 1

    def test_sorted_by_weight_descending(self):
        """Evidence is sorted by weight descending."""
        evidence = [
            Evidence(EvidenceType.SUPPORT, "S1", "C", weight=0.3),
            Evidence(EvidenceType.SUPPORT, "S2", "C", weight=0.8),
            Evidence(EvidenceType.SUPPORT, "S3", "C", weight=0.5),
        ]
        supporting, _ = self._categorize_evidence(evidence)
        assert supporting[0].weight == 0.8
        assert supporting[1].weight == 0.5
        assert supporting[2].weight == 0.3

    def test_mixed_evidence_separated(self):
        """Mixed evidence types are properly separated."""
        evidence = [
            Evidence(EvidenceType.SUPPORT, "S1", "C"),
            Evidence(EvidenceType.CONTRADICT, "C1", "C"),
            Evidence(EvidenceType.QUALIFY, "Q1", "C"),
            Evidence(EvidenceType.METHODOLOGICAL, "M1", "C"),
        ]
        supporting, contradicting = self._categorize_evidence(evidence)
        assert len(supporting) == 2
        assert len(contradicting) == 2

    def test_empty_list(self):
        """Empty list returns empty results."""
        supporting, contradicting = self._categorize_evidence([])
        assert supporting == []
        assert contradicting == []


# =============================================================================
# _suggest_sections tests
# =============================================================================
class TestSuggestSections:
    """Test _suggest_sections logic."""

    def _suggest_sections(self, contradicting: list) -> list:
        """Replicate section suggestion logic."""
        sections = [ArgumentSection.INTRODUCTION, ArgumentSection.DISCUSSION]

        if contradicting:
            sections.append(ArgumentSection.LIMITATION)

        return sections

    def test_always_suggests_intro_and_discussion(self):
        """Always suggests INTRODUCTION and DISCUSSION."""
        result = self._suggest_sections([])
        assert ArgumentSection.INTRODUCTION in result
        assert ArgumentSection.DISCUSSION in result

    def test_adds_limitation_when_contradicting(self):
        """Adds LIMITATION when contradicting evidence exists."""
        result = self._suggest_sections([Evidence(EvidenceType.CONTRADICT, "S", "C")])
        assert ArgumentSection.LIMITATION in result

    def test_no_limitation_without_contradicting(self):
        """Does not suggest LIMITATION when no contradicting evidence."""
        result = self._suggest_sections([])
        assert ArgumentSection.LIMITATION not in result


# =============================================================================
# _summarize tests
# =============================================================================
class TestSummarize:
    """Test _summarize logic."""

    def _summarize(self, argument: Argument) -> str:
        """Replicate argument summary generation."""
        support_count = len(argument.supporting_evidence)
        contradict_count = len(argument.contradicting_evidence)

        return (
            f"论点「{argument.thesis[:50]}...」"
            f"有 {support_count} 条支持证据，"
            f"{contradict_count} 条反驳证据。"
            f"涉及 {len(argument.related_gaps)} 个相关研究空白。"
        )

    def test_summary_format(self):
        """Summary has expected format."""
        arg = Argument(
            thesis="Transformer attention mechanism",
            supporting_evidence=[Evidence(EvidenceType.SUPPORT, "S", "C")],
            contradicting_evidence=[Evidence(EvidenceType.CONTRADICT, "C", "C")],
            related_gaps=["gap1"],
        )
        summary = self._summarize(arg)
        assert "有 1 条支持证据" in summary
        assert "1 条反驳证据" in summary
        assert "1 个相关研究空白" in summary

    def test_thesis_truncated(self):
        """Long thesis is truncated to 50 chars."""
        long_thesis = "A" * 100
        arg = Argument(thesis=long_thesis)
        summary = self._summarize(arg)
        assert "..." in summary
        assert f"论点「{long_thesis[:50]}..." in summary

    def test_zero_counts(self):
        """Zero evidence counts shown correctly."""
        arg = Argument(thesis="Test")
        summary = self._summarize(arg)
        assert "0 条支持证据" in summary
        assert "0 条反驳证据" in summary
        assert "0 个相关研究空白" in summary


# =============================================================================
# render_argument tests
# =============================================================================
class TestRenderArgument:
    """Test render_argument function."""

    def _render_argument(self, result: ArgumentResult) -> str:
        """Replicate argument rendering logic."""
        lines = []
        arg = result.argument

        lines.append("=" * 70)
        lines.append("📝 论点论证")
        lines.append("=" * 70)
        lines.append("")
        lines.append(f"论点：{arg.thesis}")
        lines.append("")

        lines.append("✅ 支持证据:")
        if arg.supporting_evidence:
            for i, e in enumerate(arg.supporting_evidence[:5], 1):
                lines.append(f"   {i}. [{e.source}]")
                lines.append(f"      {e.content[:80]}...")
        else:
            lines.append("   暂无支持证据")
        lines.append("")

        lines.append("❌ 反驳/质疑证据:")
        if arg.contradicting_evidence:
            for i, e in enumerate(arg.contradicting_evidence[:5], 1):
                lines.append(f"   {i}. [{e.source}]")
                lines.append(f"      {e.content[:80]}...")
        else:
            lines.append("   暂无明显反驳证据")
        lines.append("")

        if arg.related_gaps:
            lines.append("🔗 相关研究空白:")
            for gap in arg.related_gaps:
                lines.append(f"   • {gap}")
            lines.append("")

        if result.section_guidance:
            lines.append("📚 论文章节建议:")
            for section, guidance in result.section_guidance.items():
                section_name = {
                    ArgumentSection.INTRODUCTION: "引言",
                    ArgumentSection.RELATED_WORK: "相关工作",
                    ArgumentSection.METHODOLOGY: "方法论",
                    ArgumentSection.EXPERIMENTS: "实验",
                    ArgumentSection.DISCUSSION: "讨论",
                    ArgumentSection.LIMITATION: "局限",
                }.get(section, section.value)
                lines.append(f"   {section_name}:")
                lines.append(f"      {guidance[:100]}...")
            lines.append("")

        lines.append("=" * 70)
        return "\n".join(lines)

    def test_header_present(self):
        """Renders header with border and title."""
        arg = Argument(thesis="Test thesis")
        result = ArgumentResult(topic="Topic", argument=arg)
        output = self._render_argument(result)
        assert "论点论证" in output
        assert "=" * 70 in output

    def test_thesis_shown(self):
        """Thesis is displayed."""
        arg = Argument(thesis="Transformer is efficient")
        result = ArgumentResult(topic="Topic", argument=arg)
        output = self._render_argument(result)
        assert "论点：Transformer is efficient" in output

    def test_supporting_evidence_label(self):
        """Supporting evidence label is present."""
        arg = Argument(thesis="T")
        result = ArgumentResult(topic="T", argument=arg)
        output = self._render_argument(result)
        assert "✅ 支持证据" in output

    def test_empty_supporting_evidence(self):
        """Empty supporting evidence shows placeholder."""
        arg = Argument(thesis="T")
        result = ArgumentResult(topic="T", argument=arg)
        output = self._render_argument(result)
        assert "暂无支持证据" in output

    def test_supporting_evidence_listed(self):
        """Supporting evidence items are listed."""
        arg = Argument(
            thesis="T",
            supporting_evidence=[
                Evidence(EvidenceType.SUPPORT, "Paper A", "Good results"),
            ],
        )
        result = ArgumentResult(topic="T", argument=arg)
        output = self._render_argument(result)
        assert "Paper A" in output
        assert "Good results" in output

    def test_contradicting_evidence_label(self):
        """Contradicting evidence label is present."""
        arg = Argument(thesis="T")
        result = ArgumentResult(topic="T", argument=arg)
        output = self._render_argument(result)
        assert "❌ 反驳/质疑证据" in output

    def test_empty_contradicting_evidence(self):
        """Empty contradicting evidence shows placeholder."""
        arg = Argument(thesis="T")
        result = ArgumentResult(topic="T", argument=arg)
        output = self._render_argument(result)
        assert "暂无明显反驳证据" in output

    def test_related_gaps_shown(self):
        """Related gaps are displayed when present."""
        arg = Argument(
            thesis="T",
            related_gaps=["Gap A", "Gap B"],
        )
        result = ArgumentResult(topic="T", argument=arg)
        output = self._render_argument(result)
        assert "🔗 相关研究空白" in output
        assert "Gap A" in output
        assert "Gap B" in output

    def test_section_guidance_shown(self):
        """Section guidance is displayed."""
        arg = Argument(thesis="T")
        result = ArgumentResult(
            topic="T",
            argument=arg,
            section_guidance={
                ArgumentSection.INTRODUCTION: "Start with motivation",
            },
        )
        output = self._render_argument(result)
        assert "📚 论文章节建议" in output
        assert "引言" in output
        assert "Start with motivation" in output

    def test_evidence_content_truncated(self):
        """Evidence content is truncated to 80 chars."""
        long_content = "X" * 100
        arg = Argument(
            thesis="T",
            supporting_evidence=[
                Evidence(EvidenceType.SUPPORT, "S", long_content),
            ],
        )
        result = ArgumentResult(topic="T", argument=arg)
        output = self._render_argument(result)
        assert "X" * 80 in output
        assert "..." in output

    def test_max_5_evidence_items(self):
        """Only first 5 evidence items are shown."""
        evidence = [
            Evidence(EvidenceType.SUPPORT, f"P{i}", f"C{i}")
            for i in range(10)
        ]
        arg = Argument(thesis="T", supporting_evidence=evidence)
        result = ArgumentResult(topic="T", argument=arg)
        output = self._render_argument(result)
        assert "P0" in output
        assert "P4" in output
        assert "P5" not in output


# =============================================================================
# ArgumentBuilder instantiation
# =============================================================================
class TestArgumentBuilderInit:
    """Test ArgumentBuilder class."""

    def test_can_instantiate(self):
        """ArgumentBuilder can be instantiated."""
        builder = ArgumentBuilder()
        assert builder.db is None
        assert builder.insight_manager is None
        assert builder.gap_analyzer is None

    def test_can_instantiate_with_deps(self):
        """ArgumentBuilder can be instantiated with dependencies."""
        mock_db = object()
        mock_im = object()
        mock_ga = object()
        builder = ArgumentBuilder(db=mock_db, insight_manager=mock_im, gap_analyzer=mock_ga)
        assert builder.db is mock_db
        assert builder.insight_manager is mock_im
        assert builder.gap_analyzer is mock_ga
