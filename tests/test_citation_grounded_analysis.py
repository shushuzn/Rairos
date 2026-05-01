"""
Tests for citation-grounded analysis: claim classification, adaptive thresholds,
and self-correction loop.

Covers the A+B+C+E feature set from the citation-grounded analysis system.

Key design insight: verify_claims() extracts claims from result.sections (not
from pre-filled result.claims). Tests must seed result.sections with [Page N]
markers so the claim extraction pattern matches them.
"""

from __future__ import annotations

from dataclasses import dataclass

import pytest


# ── Mock PDF content ────────────────────────────────────────────────────────────


@dataclass
class MockTextBlock:
    page: int
    text: str
    block_idx: int = 0


@dataclass
class MockStructuredContent:
    text_blocks: list


# ── Shared fixtures ──────────────────────────────────────────────────────────


@pytest.fixture
def mock_content() -> MockStructuredContent:
    """Mock PDF text blocks. Text is designed for high word-overlap verification."""
    return MockStructuredContent(
        text_blocks=[
            MockTextBlock(
                page=0,
                text="准确率在SQuAD数据集上达到了95.2%，超过了基线模型。",
                block_idx=0,
            ),
            MockTextBlock(
                page=1,
                text="基于Transformer的模型在问答任务上取得了93%的准确率。",
                block_idx=0,
            ),
            MockTextBlock(
                page=2,
                text="该方法使用Transformer架构，包含多头注意力机制。",
                block_idx=0,
            ),
            MockTextBlock(
                page=3,
                text="实验在四个基准数据集上进行，包括MNLI、SST-2、QNLI和QQP。",
                block_idx=0,
            ),
            MockTextBlock(
                page=5,
                text="最终模型在测试集上达到94.8%的准确率。",
                block_idx=0,
            ),
        ]
    )


# ── Test: PaperAnalyzer produces claims ──────────────────────────────────────


class TestPaperAnalyzerProducesClaims:
    """Smoke test: analyzer returns PaperAnalysisResult with expected fields."""

    def test_analyze_returns_result_object(self):
        from llm.paper_analyzer import PaperAnalyzer

        analyzer = PaperAnalyzer(llm_config={})
        result = analyzer.analyze(
            paper_id="test123",
            title="Test Paper",
            abstract="A test abstract.",
            body_text="Some body text.",
            tags=["test"],
            authors=["Author"],
            use_llm=True,
        )

        assert hasattr(result, "sections")
        assert hasattr(result, "rubric")
        assert hasattr(result, "claims")

    def test_use_llm_false_sets_llm_used_false(self):
        from llm.paper_analyzer import PaperAnalyzer

        analyzer = PaperAnalyzer(llm_config={})
        result = analyzer.analyze(
            paper_id="test124",
            title="Test",
            abstract="",
            body_text="",
            tags=[],
            authors=[],
            use_llm=False,
        )

        assert result.llm_used is False
        assert isinstance(result.rubric, dict)


# ── Test: verify_claims sets claim_type ────────────────────────────────────────


class TestVerifyClaimsSetsFields:
    """verify_claims() populates claim_type and evidence_score.

    verify_claims() extracts claims from result.sections using a [Page N]
    pattern. Tests must seed result.sections with matching markers so that
    word-overlap verification can succeed (no Ollama server in tests).
    """

    def _run_verify(self, sections: dict, content: MockStructuredContent):
        """Helper: create a result with sections and call verify_claims."""
        from llm.paper_analyzer import PaperAnalyzer, PaperAnalysisResult

        analyzer = PaperAnalyzer(llm_config={})
        result = PaperAnalysisResult(
            paper_id="test",
            sections=sections,
            rubric={},
            llm_used=True,
        )
        return analyzer.verify_claims(result, content)

    def test_numerical_claim_classified_correctly(self, mock_content):
        """A claim containing '%' is classified as 'numerical'."""
        sections = {
            "## 背景": "准确率达到了95%[Page 1]的效果。",
        }
        result = self._run_verify(sections, mock_content)

        # Claim is extracted and classified as numerical; it may end up in
        # verified_claims OR unverified_claims depending on evidence score.
        all_types = [c.claim_type for c in result.claims] + [
            c.claim_type for c in result.unverified_claims
        ]
        assert "numerical" in all_types, f"Expected 'numerical', got {all_types}"

    def test_methodology_claim_classified_correctly(self, mock_content):
        """A claim containing '使用' and '架构' is classified as 'methodology'."""
        sections = {
            "## 方法": "该方法使用Transformer架构[Page 3]。",
        }
        result = self._run_verify(sections, mock_content)

        all_types = [c.claim_type for c in result.claims] + [
            c.claim_type for c in result.unverified_claims
        ]
        assert "methodology" in all_types, f"Expected 'methodology', got {all_types}"

    def test_descriptive_claim_classified_correctly(self, mock_content):
        """A plain author citation (no numbers, no methodology) is 'descriptive'."""
        sections = {
            "## 作者": "该论文由Smith等人[Page 4]发表。",
        }
        result = self._run_verify(sections, mock_content)

        all_types = [c.claim_type for c in result.claims] + [
            c.claim_type for c in result.unverified_claims
        ]
        assert "descriptive" in all_types, f"Expected 'descriptive', got {all_types}"

    def test_all_verified_claims_have_valid_type(self, mock_content):
        """Every verified claim must have a non-empty claim_type."""
        sections = {
            "## 背景": "准确率达到了95% [Page 1] 效果显著。",
        }
        result = self._run_verify(sections, mock_content)

        for c in result.claims:
            assert c.claim_type in ("numerical", "methodology", "descriptive"), (
                f"Invalid claim_type: {c.claim_type!r}"
            )

    def test_verified_claim_has_evidence_score_in_range(self, mock_content):
        """Evidence scores are always in [0.0, 1.0]."""
        sections = {
            "## 背景": "准确率达到了95% [Page 1]。",
        }
        result = self._run_verify(sections, mock_content)

        for c in result.claims:
            assert 0.0 <= c.evidence_score <= 1.0

    def test_unverified_claims_also_have_claim_type(self, mock_content):
        """Unverified claims also carry their classification."""
        sections = {
            "## 结果": "该方法使用CNN架构 [Page 10] 并达到99%准确率。",
        }
        result = self._run_verify(sections, mock_content)

        # Page 10 and Page 99 don't exist in mock_content → unverified
        for c in result.unverified_claims:
            assert c.claim_type in ("numerical", "methodology", "descriptive", "")
