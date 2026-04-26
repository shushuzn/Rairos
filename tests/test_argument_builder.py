"""Tests for argument builder."""
import pytest
from unittest.mock import MagicMock, patch

from llm.argument_builder import (
    ArgumentBuilder,
    ArgumentResult,
    Argument,
    Evidence,
    EvidenceType,
    ArgumentSection,
    render_argument,
)


class TestEvidenceTypes:
    """Test evidence type classification."""

    def test_support_keywords(self):
        """Test support keyword detection."""
        builder = ArgumentBuilder()
        content = "This method effectively improves accuracy"
        thesis = "method improves accuracy"
        result = builder._classify_insight(content, thesis)
        assert result == EvidenceType.SUPPORT

    def test_contradict_keywords(self):
        """Test contradict keyword detection."""
        builder = ArgumentBuilder()
        content = "The method has significant limitations"
        thesis = "method is effective"
        result = builder._classify_insight(content, thesis)
        assert result == EvidenceType.CONTRADICT


class TestEvidenceCategorization:
    """Test evidence categorization."""

    def test_categorize_separates_support_contradict(self):
        """Test evidence is separated by type."""
        builder = ArgumentBuilder()

        evidence = [
            Evidence(evidence_type=EvidenceType.SUPPORT, source="p1", content="a", weight=0.8),
            Evidence(evidence_type=EvidenceType.CONTRADICT, source="p2", content="b", weight=0.9),
            Evidence(evidence_type=EvidenceType.SUPPORT, source="p3", content="c", weight=0.5),
        ]

        supporting, contradicting = builder._categorize_evidence(evidence)

        assert len(supporting) == 2
        assert len(contradicting) == 1

    def test_categorize_sorts_by_weight(self):
        """Test evidence is sorted by weight."""
        builder = ArgumentBuilder()

        evidence = [
            Evidence(evidence_type=EvidenceType.SUPPORT, source="p1", content="low", weight=0.3),
            Evidence(evidence_type=EvidenceType.SUPPORT, source="p2", content="high", weight=0.9),
        ]

        supporting, _ = builder._categorize_evidence(evidence)
        assert supporting[0].weight == 0.9
        assert supporting[1].weight == 0.3


class TestArgumentBuilding:
    """Test argument building."""

    def test_build_returns_result(self):
        """Test build returns ArgumentResult."""
        builder = ArgumentBuilder()
        result = builder.build("Test thesis", use_llm=False)
        assert isinstance(result, ArgumentResult)
        assert result.topic == "Test thesis"
        assert result.argument is not None

    def test_build_with_no_db(self):
        """Test build without database."""
        builder = ArgumentBuilder(db=None)
        result = builder.build("Test thesis", use_llm=False)
        assert len(result.argument.supporting_evidence) == 0

    def test_build_with_mock_insights(self):
        """Test build with mock insights."""
        mock_im = MagicMock()
        mock_card = MagicMock()
        mock_card.id = "insight-1"
        mock_card.content = "This is effective"
        mock_im.search_cards.return_value = [mock_card]

        builder = ArgumentBuilder(insight_manager=mock_im)
        result = builder.build("method effectiveness", use_llm=False)

        # Should have collected insight evidence
        assert len(result.argument.supporting_evidence) >= 0


class TestSectionSuggestion:
    """Test section suggestion logic."""

    def test_suggest_sections_basic(self):
        """Test basic section suggestion."""
        builder = ArgumentBuilder()
        sections = builder._suggest_sections([])
        assert ArgumentSection.INTRODUCTION in sections
        assert ArgumentSection.DISCUSSION in sections

    def test_suggest_sections_with_contradictions(self):
        """Test sections include limitation when contradictions exist."""
        builder = ArgumentBuilder()
        contradict = [Evidence(EvidenceType.CONTRADICT, "s", "c", weight=1.0)]
        sections = builder._suggest_sections(contradict)
        assert ArgumentSection.LIMITATION in sections


class TestTemplateGuidance:
    """Test template-based guidance generation."""

    def test_generate_template_guidance(self):
        """Test template guidance generation."""
        builder = ArgumentBuilder()
        guidance = builder._template_guidance([], [])

        assert ArgumentSection.INTRODUCTION in guidance
        assert ArgumentSection.DISCUSSION in guidance
        assert ArgumentSection.LIMITATION in guidance

    def test_guidance_has_content(self):
        """Test guidance has meaningful content."""
        builder = ArgumentBuilder()
        guidance = builder._template_guidance([], [])

        for section, content in guidance.items():
            assert len(content) > 10


class TestRenderArgument:
    """Test argument rendering."""

    def test_render_basic(self):
        """Test basic argument rendering."""
        arg = Argument(thesis="Test thesis")
        result = ArgumentResult(topic="Test", argument=arg)
        output = render_argument(result)

        assert "Test thesis" in output
        assert "支持证据" in output
        assert "反驳证据" in output

    def test_render_with_evidence(self):
        """Test rendering with evidence."""
        supporting = [
            Evidence(EvidenceType.SUPPORT, "Paper A", "This is good", weight=0.9),
        ]
        contradicting = [
            Evidence(EvidenceType.CONTRADICT, "Paper B", "This has issues", weight=0.7),
        ]

        arg = Argument(
            thesis="Method works",
            supporting_evidence=supporting,
            contradicting_evidence=contradicting,
        )
        result = ArgumentResult(topic="Test", argument=arg)
        output = render_argument(result)

        assert "✅ 支持证据" in output
        assert "❌ 反驳" in output
        assert "Paper A" in output

    def test_render_with_gaps(self):
        """Test rendering with related gaps."""
        arg = Argument(
            thesis="Test",
            related_gaps=["Gap 1", "Gap 2"],
        )
        result = ArgumentResult(topic="Test", argument=arg)
        output = render_argument(result)

        assert "🔗 相关研究空白" in output
        assert "Gap 1" in output


class TestSummarize:
    """Test argument summarization."""

    def test_summarize_basic(self):
        """Test basic summarization."""
        builder = ArgumentBuilder()
        arg = Argument(
            thesis="A" * 100,  # Long thesis
            supporting_evidence=[Evidence(EvidenceType.SUPPORT, "s", "c")],
            contradicting_evidence=[Evidence(EvidenceType.CONTRADICT, "s", "c")],
            related_gaps=["g1", "g2"],
        )
        summary = builder._summarize(arg)

        assert "A..." in summary  # Truncated
        assert "1 条支持证据" in summary
        assert "1 条反驳证据" in summary
        assert "2 个相关研究空白" in summary
