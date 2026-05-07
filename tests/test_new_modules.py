"""Tests for scout, events, intelligence, signal, report, mcp_jin10."""

from __future__ import annotations

import json
import tempfile
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest

from llm.insight.gene import CapsuleGene


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


def _make_capsule(
    capsule_id: str = "c1",
    trigger_topic: str = "geopolitical risk",
    trigger_gap_type: str = "evaluation_gap",
    trigger_keywords: list[str] | None = None,
    outcome_success_score: float = 0.8,
    source_category: str = "cs.GL",
    credibility_score: float = 0.7,
) -> CapsuleGene:
    if trigger_keywords is None:
        trigger_keywords = ["oil", "military", "risk"]
    return CapsuleGene(
        capsule_id=capsule_id,
        created_at="2026-05-01T00:00:00",
        trigger_topic=trigger_topic,
        trigger_gap_type=trigger_gap_type,
        trigger_keywords=trigger_keywords,
        action_gap_type=trigger_gap_type,
        action_gap_title="Test geopolitical capsule",
        outcome_success_score=outcome_success_score,
        feedback_count=5,
        archetype={"source_arxiv_category": source_category} if source_category else {},
        status="active",
        credibility_score=credibility_score,
    )


# ---------------------------------------------------------------------------
# llm.scout
# ---------------------------------------------------------------------------


class TestScout:
    def test_get_topics_from_pool(self):
        from llm.scout import _get_topics_from_pool

        caps = [
            _make_capsule(trigger_keywords=["oil", "iran", "military"]),
            _make_capsule(trigger_keywords=["gold", "safe", "haven"]),
        ]
        topics = _get_topics_from_pool(caps)
        assert isinstance(topics, list)
        assert len(topics) > 0

    def test_scout_empty_pool(self):
        from llm.scout import scout

        results = scout(topic="test", max_results=5, min_match_score=0.1)
        # Should not crash — may return 0 results or some if pool has data
        assert isinstance(results, list)


# ---------------------------------------------------------------------------
# llm.events
# ---------------------------------------------------------------------------


class TestEvents:
    def test_process_event_no_keyword(self):
        from llm.events import _fetch_event_news, _build_summary, _infer_gap_type

        # Test internal functions without live MCP calls
        news = _fetch_event_news.__doc__  # just verify the function exists
        assert news is not None

        # Test render_event_report with error result
        from llm.events import render_event_report

        rendered = render_event_report({"error": "No news found"})
        assert "Error" in rendered

    def test_infer_gap_type_military(self):
        from llm.events import _infer_gap_type

        summary = {"brief": "导弹袭击石油设施"}
        assert _infer_gap_type(summary) == "scalability_issue"

    def test_infer_gap_type_oil(self):
        from llm.events import _infer_gap_type

        summary = {"brief": "原油价格波动影响市场"}
        assert _infer_gap_type(summary) == "evaluation_gap"

    def test_infer_gap_type_finance(self):
        from llm.events import _infer_gap_type

        summary = {"brief": "美联储加息利率通胀"}
        assert _infer_gap_type(summary) == "method_limitation"

    def test_infer_gap_type_default(self):
        from llm.events import _infer_gap_type

        summary = {"brief": "普通新闻事件"}
        assert _infer_gap_type(summary) == "unexplored_application"


# ---------------------------------------------------------------------------
# llm.signal
# ---------------------------------------------------------------------------


class TestSignal:
    def test_render_signal_empty(self):
        from llm.signal import render_signal

        result = {
            "event": "test",
            "signal": "LOW",
            "timestamp": "2026-01-01T00:00",
            "capsule_matches": [],
            "markets": {},
            "impact_sectors": [],
            "recommendation": "No significant match.",
        }
        rendered = render_signal(result)
        assert "Signal" in rendered
        assert "LOW" in rendered


# ---------------------------------------------------------------------------
# llm.intelligence
# ---------------------------------------------------------------------------


class TestIntelligence:
    def test_intelligence_basic(self):
        from llm.intelligence import render_report

        report = {
            "generated_at": "2026-01-01T00:00",
            "topic": "test",
            "flash_news": [],
            "markets": [],
            "gene_pool": {"total": 10, "avg_score": 0.5, "by_type": {}, "high_credibility": 2},
            "top_capsules": [],
            "watch": {},
            "papers": [],
        }
        rendered = render_report(report)
        assert "Intelligence" in rendered


# ---------------------------------------------------------------------------
# llm.report
# ---------------------------------------------------------------------------


class TestReport:
    def test_generate_report(self):
        from llm.report import generate

        report = generate()
        assert isinstance(report, str)
        assert "GENE POOL" in report or "REPORT" in report

    def test_save_report(self):
        from llm.report import save

        path = save()
        assert Path(path).exists()
        content = Path(path).read_text(encoding="utf-8")
        assert "GENE POOL" in content or "REPORT" in content


# ---------------------------------------------------------------------------
# llm.discover
# ---------------------------------------------------------------------------


class TestDiscover:
    def test_render_discovery(self):
        from llm.discover import render_discovery

        result = {
            "patterns_discovered": 0,
            "total_patterns": 2,
            "event_capsules": 5,
            "research_capsules": 10,
            "new_patterns": [],
            "markets": {"USOIL": {"price": "100"}},
        }
        rendered = render_discovery(result)
        assert "Pattern" in rendered or "Discovery" in rendered


# ---------------------------------------------------------------------------
# llm.mcp_jin10 (mock-based, no live API)
# ---------------------------------------------------------------------------


class TestMCPJin10:
    def test_mcp_error(self):
        from llm.mcp_jin10 import MCPError

        err = MCPError("test error")
        assert str(err) == "test error"

    def test_client_init(self):
        from llm.mcp_jin10 import Jin10Client

        client = Jin10Client(url="http://localhost:1", token="test")
        assert client.url == "http://localhost:1"
        assert client.token == "test"
        assert not client._initialized

    def test_client_init_fails_noconnect(self):
        from llm.mcp_jin10 import Jin10Client

        client = Jin10Client(url="http://localhost:1", token="test")
        with pytest.raises(Exception):
            client.initialize()
