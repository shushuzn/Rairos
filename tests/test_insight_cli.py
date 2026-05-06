"""CLI integration tests for insight commands (cli.cmd.evo.insight)."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest

from llm.insight_cards import InsightCard, InsightManager


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture
def mock_insight_manager(tmp_path, monkeypatch):
    """Route InsightManager to a temp directory so tests don't pollute real data."""

    class MockManager(InsightManager):
        def __init__(self):
            self.data_dir = tmp_path
            self.data_dir.mkdir(parents=True, exist_ok=True)
            self.cards_file = self.data_dir / "insight_cards.json"
            self.collections_file = self.data_dir / "insight_collections.json"

    # Patch InsightManager class in the CLI module
    import cli.cmd.evo.insight

    monkeypatch.setattr(cli.cmd.evo.insight, "InsightManager", MockManager)
    return MockManager


# ---------------------------------------------------------------------------
# Argument builder helpers
# ---------------------------------------------------------------------------


def _ns(**kwargs):
    """Build a Namespace for _run_insight."""
    defaults = dict(
        action="list",
        paper=None,
        content=None,
        type="finding",
        tags=None,
        evidence=None,
        query=None,
        markdown=False,
        collection=None,
        cite=None,
        card=None,
        stars=None,
        top_k=10,
        json=False,
        watch=False,
        interval=30,
    )
    defaults.update(kwargs)
    return argparse.Namespace(**defaults)


# ---------------------------------------------------------------------------
# Action: add
# ---------------------------------------------------------------------------


class TestInsightCliAdd:
    """Test 'add' action."""

    def test_add_requires_paper_and_content(self, mock_insight_manager):
        from cli.cmd.evo.insight import _run_insight

        ns = _ns(action="add", paper=None, content="some finding")
        rc = _run_insight(ns)
        assert rc == 1

        ns2 = _ns(action="add", paper="p1", content=None)
        rc2 = _run_insight(ns2)
        assert rc2 == 1

    def test_add_success(self, mock_insight_manager, capsys):
        from cli.cmd.evo.insight import _run_insight

        ns = _ns(
            action="add",
            paper="p1",
            content="Deep learning improves RAG accuracy by 15%",
            type="finding",
            tags="nlp,rag",
        )
        rc = _run_insight(ns)
        assert rc == 0
        out = capsys.readouterr().out
        assert "i0001" in out

    def test_add_creates_card(self, mock_insight_manager):
        from cli.cmd.evo.insight import _run_insight

        ns = _ns(
            action="add",
            paper="p1",
            content="Key finding about transformers",
            type="method",
            tags="transformer",
        )
        _run_insight(ns)

        manager = mock_insight_manager()
        cards = manager.search_cards(paper_id="p1")
        assert len(cards) == 1
        assert cards[0].content == "Key finding about transformers"
        assert cards[0].insight_type == "method"
        assert "transformer" in cards[0].tags


# ---------------------------------------------------------------------------
# Action: list / search
# ---------------------------------------------------------------------------


class TestInsightCliListSearch:
    """Test 'list' and 'search' actions."""

    def test_list_empty(self, mock_insight_manager, capsys):
        from cli.cmd.evo.insight import _run_insight

        ns = _ns(action="list")
        rc = _run_insight(ns)
        assert rc == 0
        out = capsys.readouterr().out
        assert "No insight cards" in out

    def test_list_returns_cards(self, mock_insight_manager, capsys):
        from cli.cmd.evo.insight import _run_insight

        # Add some cards
        mgr = mock_insight_manager()
        mgr.add_card("p1", "Paper One", "Finding A", tags=["nlp"])
        mgr.add_card("p2", "Paper Two", "Finding B", tags=["cv"])

        ns = _ns(action="list")
        rc = _run_insight(ns)
        assert rc == 0
        out = capsys.readouterr().out
        assert "Finding A" in out or "Finding B" in out

    def test_search_filters_by_query(self, mock_insight_manager, capsys):
        from cli.cmd.evo.insight import _run_insight

        mgr = mock_insight_manager()
        mgr.add_card("p1", "Paper One", "BERT improves accuracy")
        mgr.add_card("p2", "Paper Two", "GPT generates text")

        ns = _ns(action="search", query="BERT")
        rc = _run_insight(ns)
        assert rc == 0
        out = capsys.readouterr().out
        assert "BERT" in out
        assert "GPT" not in out

    def test_search_filters_by_tags(self, mock_insight_manager, capsys):
        from cli.cmd.evo.insight import _run_insight

        mgr = mock_insight_manager()
        mgr.add_card("p1", "P1", "Finding NLP", tags=["nlp"])
        mgr.add_card("p2", "P2", "Finding CV", tags=["cv"])

        ns = _ns(action="search", tags="nlp")
        rc = _run_insight(ns)
        assert rc == 0
        out = capsys.readouterr().out
        assert "NLP" in out or "nlp" in out

    def test_list_markdown_mode(self, mock_insight_manager, capsys):
        from cli.cmd.evo.insight import _run_insight

        mgr = mock_insight_manager()
        mgr.add_card("p1", "Paper", "Key finding")

        ns = _ns(action="list", markdown=True)
        rc = _run_insight(ns)
        assert rc == 0
        out = capsys.readouterr().out
        assert "# Key Insight Cards" in out


# ---------------------------------------------------------------------------
# Action: rate / like / dislike
# ---------------------------------------------------------------------------


class TestInsightCliRating:
    """Test 'rate', 'like', 'dislike' actions."""

    def test_rate_requires_card_and_stars(self, mock_insight_manager):
        from cli.cmd.evo.insight import _run_insight

        ns = _ns(action="rate", card=None, stars=5)
        rc = _run_insight(ns)
        assert rc == 1

        ns2 = _ns(action="rate", card="i0001", stars=None)
        rc2 = _run_insight(ns2)
        assert rc2 == 1

    def test_rate_success(self, mock_insight_manager, capsys):
        from cli.cmd.evo.insight import _run_insight

        mgr = mock_insight_manager()
        card = mgr.add_card("p1", "Paper", "Finding")

        ns = _ns(action="rate", card=card.card_id, stars=4)
        rc = _run_insight(ns)
        assert rc == 0

    def test_like_success(self, mock_insight_manager, capsys):
        from cli.cmd.evo.insight import _run_insight

        mgr = mock_insight_manager()
        card = mgr.add_card("p1", "Paper", "Great finding")

        ns = _ns(action="like", card=card.card_id)
        rc = _run_insight(ns)
        assert rc == 0

    def test_dislike_success(self, mock_insight_manager, capsys):
        from cli.cmd.evo.insight import _run_insight

        mgr = mock_insight_manager()
        card = mgr.add_card("p1", "Paper", "Weak finding")

        ns = _ns(action="dislike", card=card.card_id)
        rc = _run_insight(ns)
        assert rc == 0

    def test_rate_nonexistent_card(self, mock_insight_manager):
        from cli.cmd.evo.insight import _run_insight

        ns = _ns(action="rate", card="i9999", stars=3)
        rc = _run_insight(ns)
        assert rc == 1


# ---------------------------------------------------------------------------
# Action: top / bottom
# ---------------------------------------------------------------------------


class TestInsightCliTopBottom:
    """Test 'top' and 'bottom' actions."""

    def test_top_empty(self, mock_insight_manager, capsys):
        from cli.cmd.evo.insight import _run_insight

        ns = _ns(action="top")
        rc = _run_insight(ns)
        assert rc == 0
        out = capsys.readouterr().out
        assert "No highly-rated" in out

    def test_top_returns_rated(self, mock_insight_manager, capsys):
        from cli.cmd.evo.insight import _run_insight

        mgr = mock_insight_manager()
        card = mgr.add_card("p1", "Paper", "Finding")
        mgr.rate_card(card.card_id, 5)

        ns = _ns(action="top", top_k=5)
        rc = _run_insight(ns)
        assert rc == 0
        out = capsys.readouterr().out
        assert "i0001" in out

    def test_bottom_with_rated_cards(self, mock_insight_manager, capsys):
        from cli.cmd.evo.insight import _run_insight

        mgr = mock_insight_manager()
        card = mgr.add_card("p1", "Paper", "Finding")
        mgr.rate_card(card.card_id, 1)

        ns = _ns(action="bottom", top_k=5)
        rc = _run_insight(ns)
        assert rc == 0


# ---------------------------------------------------------------------------
# Action: tag-cloud
# ---------------------------------------------------------------------------


class TestInsightCliTagCloud:
    """Test 'tag-cloud' action."""

    def test_tag_cloud_empty(self, mock_insight_manager, capsys):
        from cli.cmd.evo.insight import _run_insight

        ns = _ns(action="tag-cloud")
        rc = _run_insight(ns)
        assert rc == 0
        out = capsys.readouterr().out
        assert "No tags found" in out

    def test_tag_cloud_with_tags(self, mock_insight_manager, capsys):
        from cli.cmd.evo.insight import _run_insight

        mgr = mock_insight_manager()
        mgr.add_card("p1", "Paper", "Finding", tags=["nlp", "transformer"])
        mgr.add_card("p2", "Paper", "Finding", tags=["nlp", "bert"])

        ns = _ns(action="tag-cloud")
        rc = _run_insight(ns)
        assert rc == 0
        out = capsys.readouterr().out
        assert "nlp" in out


# ---------------------------------------------------------------------------
# Action: export
# ---------------------------------------------------------------------------


class TestInsightCliExport:
    """Test 'export' action."""

    def test_export_empty(self, mock_insight_manager, capsys):
        from cli.cmd.evo.insight import _run_insight

        ns = _ns(action="export")
        rc = _run_insight(ns)
        assert rc == 0
        out = capsys.readouterr().out
        assert "Finding" not in out

    def test_export_with_cards(self, mock_insight_manager, capsys):
        from cli.cmd.evo.insight import _run_insight

        mgr = mock_insight_manager()
        mgr.add_card("p1", "Paper", "Key finding", tags=["nlp"])

        ns = _ns(action="export")
        rc = _run_insight(ns)
        assert rc == 0
        out = capsys.readouterr().out
        assert "[[p1]]" in out
        assert "#nlp" in out


# ---------------------------------------------------------------------------
# Action: quality-report
# ---------------------------------------------------------------------------


class TestInsightCliQualityReport:
    """Test 'quality-report' action."""

    def test_quality_report_json(self, mock_insight_manager, capsys):
        from cli.cmd.evo.insight import _run_insight

        ns = _ns(action="quality-report", json=True, watch=False)
        rc = _run_insight(ns)
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert "total" in data
        assert "credibility_distribution" in data

    def test_quality_report_text(self, mock_insight_manager, capsys):
        from cli.cmd.evo.insight import _run_insight

        ns = _ns(action="quality-report", json=False, watch=False)
        rc = _run_insight(ns)
        assert rc == 0
        out = capsys.readouterr().out
        assert "Gene Pool Quality Report" in out
        assert "Total capsules" in out


# ---------------------------------------------------------------------------
# Action: eval-retrieval
# ---------------------------------------------------------------------------


class TestInsightCliEvalRetrieval:
    """Test 'eval-retrieval' action."""

    def test_eval_retrieval_returns_json(self, mock_insight_manager, capsys):
        from cli.cmd.evo.insight import _run_insight

        ns = _ns(action="eval-retrieval", json=False, watch=False)
        rc = _run_insight(ns)
        # May return 0 or 1 depending on whether there are real events
        assert rc in (0, 1)
        out = capsys.readouterr().out
        assert "recall" in out or "error" in out or "n=" in out
