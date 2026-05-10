"""Tests for briefing_generator and daemon."""

from datetime import datetime
from llm.briefing_generator import (
    BriefingSection,
    Briefing,
    BriefingResult,
    BriefingGenerator,
    _load_gene_pool,
    _load_research_memory,
)
from research_loop.daemon import EventBus, DaemonEvent, ResearchDaemon, SSEServer


# ── briefing_generator ────────────────────────────────────────────────────────


class TestBriefingSection:
    def test_fields(self):
        s = BriefingSection(title="Intro", content="This is the intro", level=1)
        assert s.title == "Intro"
        assert s.level == 1

    def test_defaults(self):
        s = BriefingSection(title="A", content="B")
        assert s.level == 2  # default level is 2


class TestBriefing:
    def test_fields(self):
        b = Briefing(
            paper_arxiv_id="2401.00001",
            paper_title="Test Paper",
            sections=[],
            gene_pool_matches=[],
            memory_stances=[],
            verdict="positive",
            verdict_reason="Good",
            generated_at="2024-01-01T00:00:00",
        )
        assert b.paper_arxiv_id == "2401.00001"
        assert b.verdict == "positive"


class TestBriefingResult:
    def test_success_fields(self):
        b = Briefing(
            paper_arxiv_id="x",
            paper_title="T",
            sections=[],
            gene_pool_matches=[],
            memory_stances=[],
            verdict="",
            verdict_reason="",
            generated_at="",
        )
        r = BriefingResult(success=True, briefing=b, markdown="# Test", error=None)
        assert r.success is True
        assert r.markdown == "# Test"

    def test_error_fields(self):
        r = BriefingResult(success=False, briefing=None, markdown="", error="Failed")
        assert r.success is False
        assert r.error == "Failed"


class TestBriefingGenerator:
    def test_init(self):
        bg = BriefingGenerator(db=None)
        assert bg is not None


class TestLoadGenePool:
    def test_returns_list(self):
        result = _load_gene_pool()
        assert isinstance(result, list)


class TestLoadResearchMemory:
    def test_returns_list(self):
        result = _load_research_memory()
        assert isinstance(result, list)


# ── daemon ───────────────────────────────────────────────────────────────────


class TestEventBus:
    def test_init(self):
        eb = EventBus()
        assert eb is not None


class TestDaemonEvent:
    def test_fields(self):
        e = DaemonEvent(event_type="start", data={})
        assert e.event_type == "start"

    def test_timestamp(self):
        e = DaemonEvent(event_type="start", data={})
        assert isinstance(e.timestamp, (datetime, float))


class TestResearchDaemon:
    def test_init(self):
        d = ResearchDaemon(interval_minutes=5, webhook_enabled=False)
        assert d is not None


class TestSSEServer:
    def test_init(self):
        s = SSEServer(port=8080)
        assert s is not None
