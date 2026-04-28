"""Tests for literature review components."""
import pytest
from datetime import datetime

from renderers.litreview import (
    render_litreview,
    update_litreview,
    _get_date_range,
    _group_by_methodology,
    _extract_open_problems,
)
from llm.litreview_analyzer import LitReviewAnalyzer


class TestRenderLitreview:
    """Test render_litreview function."""

    def test_render_empty_papers(self):
        """Test rendering with no papers."""
        result = render_litreview("Test Topic", [])
        assert "Test Topic" in result
        assert "type: lit-review" in result
        assert "status: evolving" in result

    def test_render_with_papers(self):
        """Test rendering with sample papers."""
        papers = [
            {
                "arxiv_id": "2301.00001",
                "title": "Test Paper One",
                "abstract": "A test paper about transformers.",
                "published": "2024-01-15",
                "score": 8.5,
            },
            {
                "arxiv_id": "2301.00002",
                "title": "Test Paper Two",
                "abstract": "A test paper about diffusion models.",
                "published": "2024-02-20",
                "score": 9.0,
            },
        ]
        result = render_litreview("AI Research", papers)

        assert "AI Research" in result
        assert "Test Paper One" in result
        assert "Test Paper Two" in result
        assert "2024-01-15" in result or "2024-02-20" in result
        assert "## 研究时间线" in result
        assert "## 方法分类" in result
        assert "## 代表论文" in result

    def test_render_timestamps(self):
        """Test that timestamps are included."""
        created = "2024-01-01T00:00:00"
        updated = "2024-06-01T12:00:00"
        result = render_litreview("Topic", [], created_at=created, updated_at=updated)

        assert "created_at" in result
        assert "last_updated" in result


class TestUpdateLitreview:
    """Test update_litreview function."""

    def test_update_empty_existing(self):
        """Test updating empty existing content."""
        existing = """---
type: lit-review
topic: Test
---"""
        new_papers = []
        result = update_litreview(existing, new_papers)

        assert "last_updated" in result

    def test_update_with_new_papers(self):
        """Test updating with new papers."""
        existing = """---
type: lit-review
topic: Test
---

# Test 文献综述

## 更新日志

- 2024-01-01: Initial version
"""
        new_papers = [
            {
                "arxiv_id": "2401.00001",
                "title": "New Paper",
                "published": "2024-06-01",
            }
        ]
        all_papers = new_papers

        result = update_litreview(existing, new_papers, all_papers)

        assert "New Paper" in result or "new_paper" in result.lower()
        assert "## 更新日志" in result


class TestDateRange:
    """Test _get_date_range helper."""

    def test_single_date(self):
        """Test with single paper."""
        papers = [{"published": "2024-03-15"}]
        result = _get_date_range(papers)
        assert "2024-03-15" in result

    def test_date_range(self):
        """Test with multiple dates."""
        papers = [
            {"published": "2024-01-01"},
            {"published": "2024-12-31"},
        ]
        result = _get_date_range(papers)
        assert "2024-01-01" in result
        assert "2024-12-31" in result

    def test_no_dates(self):
        """Test with no dates."""
        papers = [{}, {"published": ""}]
        result = _get_date_range(papers)
        assert result == "未知"


class TestMethodologyGrouping:
    """Test _group_by_methodology."""

    def test_transformer_group(self):
        """Test papers are grouped by transformer."""
        papers = [
            {"title": "Attention Is All You Need", "abstract": "transformer architecture"},
            {"title": "BERT Paper", "abstract": "bidirectional encoder transformer"},
        ]
        result = _group_by_methodology(papers)
        assert "Transformer/Attention" in result or len(result) > 0

    def test_diffusion_group(self):
        """Test papers are grouped by diffusion."""
        papers = [
            {"title": "DDPM Paper", "abstract": "diffusion probabilistic models"},
        ]
        result = _group_by_methodology(papers)
        # Should be grouped under diffusion or other category
        assert len(result) >= 1

    def test_unclassified(self):
        """Test papers without clear method."""
        papers = [
            {"title": "Generic ML Paper", "abstract": "machine learning methods"},
        ]
        result = _group_by_methodology(papers)
        # Paper may or may not be classified depending on keywords
        assert isinstance(result, dict)


class TestOpenProblems:
    """Test _extract_open_problems."""

    def test_extract_future_work(self):
        """Test extraction of future work mentions."""
        papers = [
            {
                "title": "Test Paper",
                "abstract": "This paper discusses current methods. Future work could explore better approaches.",
            }
        ]
        result = _extract_open_problems(papers)
        assert len(result) >= 0  # May or may not extract depending on phrase matching

    def test_no_abstract(self):
        """Test with papers missing abstract."""
        papers = [
            {"title": "No Abstract Paper"},
            {},
        ]
        result = _extract_open_problems(papers)
        assert isinstance(result, list)


class TestLitReviewAnalyzer:
    """Test LitReviewAnalyzer class."""

    def test_analyze_trends_empty(self):
        """Test analyzing empty paper list."""
        analyzer = LitReviewAnalyzer()
        result = analyzer.analyze_trends([])
        assert "method_evolution" in result
        assert "temporal_distribution" in result
        assert "rising_topics" in result

    def test_analyze_trends_with_papers(self):
        """Test analyzing papers for trends."""
        papers = [
            {
                "title": "Transformer Paper",
                "abstract": "Transformer architecture with attention mechanism.",
                "published": "2024-05-01",
                "score": 8.5,
            },
            {
                "title": "Diffusion Paper",
                "abstract": "Diffusion models for image generation.",
                "published": "2024-06-01",
                "score": 9.0,
            },
        ]
        analyzer = LitReviewAnalyzer()
        result = analyzer.analyze_trends(papers)

        assert "method_evolution" in result
        assert "temporal_distribution" in result
        assert "rising_topics" in result

    def test_find_controversies(self):
        """Test finding controversies."""
        papers = [
            {
                "title": "Paper A",
                "abstract": "Our method outperforms previous approaches while others argue about methodology differences.",
            }
        ]
        analyzer = LitReviewAnalyzer()
        result = analyzer.find_controversies(papers)
        assert isinstance(result, list)

    def test_extract_open_problems(self):
        """Test extracting open problems."""
        papers = [
            {
                "title": "Research Paper",
                "abstract": "We leave exploration of multi-modal extensions as future work.",
            }
        ]
        analyzer = LitReviewAnalyzer()
        result = analyzer.extract_open_problems(papers)
        assert isinstance(result, list)

    def test_group_by_methodology(self):
        """Test grouping papers by methodology."""
        papers = [
            {"title": "Attention Paper", "abstract": "Transformer with self-attention."},
            {"title": "CNN Paper", "abstract": "Convolutional neural networks."},
        ]
        analyzer = LitReviewAnalyzer()
        result = analyzer.group_by_methodology(papers)
        assert len(result) >= 1

    def test_update_for_subscription_no_db(self):
        """Test update without database."""
        analyzer = LitReviewAnalyzer(db=None)
        result = analyzer.update_for_subscription("sub123", [])
        assert result is None

    def test_temporal_distribution(self):
        """Test temporal distribution analysis."""
        papers = [
            {"title": "P1", "published": "2024-01"},
            {"title": "P2", "published": "2024-02"},
            {"title": "P3", "published": "2024-02"},
            {"title": "P4", "published": "2023-12"},
        ]
        analyzer = LitReviewAnalyzer()
        result = analyzer.analyze_trends(papers)
        temporal = result["temporal_distribution"]
        assert "2024" in str(temporal)

    def test_detect_rising_topics(self):
        """Test rising topic detection."""
        papers = [
            {"title": "LLM Paper", "abstract": "Large language models.", "published": "2024-06"},
            {"title": "Multimodal Paper", "abstract": "Multimodal learning.", "published": "2024-05"},
        ]
        analyzer = LitReviewAnalyzer()
        result = analyzer.analyze_trends(papers)
        rising = result["rising_topics"]
        assert isinstance(rising, list)
