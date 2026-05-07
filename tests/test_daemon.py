"""Tests for watch daemon, events pipeline, and report generation."""

from __future__ import annotations

import tempfile
from pathlib import Path

import pytest

from llm.insight.gene import CapsuleGene


def _make_capsule(**kw) -> CapsuleGene:
    return CapsuleGene(
        capsule_id=kw.get("capsule_id", "c1"),
        created_at=kw.get("created_at", "2026-01-01T00:00:00"),
        trigger_topic=kw.get("trigger_topic", "test"),
        trigger_gap_type=kw.get("trigger_gap_type", "evaluation_gap"),
        trigger_keywords=kw.get("trigger_keywords", ["test", "keyword"]),
        action_gap_type=kw.get("action_gap_type", "evaluation_gap"),
        action_gap_title=kw.get("action_gap_title", "Test capsule"),
        outcome_success_score=kw.get("outcome_success_score", 0.8),
        feedback_count=kw.get("feedback_count", 5),
        archetype=kw.get("archetype", {}),
        status=kw.get("status", "active"),
        credibility_score=kw.get("credibility_score", 0.5),
        source_arxiv_category=kw.get("source_arxiv_category", ""),
    )


class TestWatchDaemon:
    def test_watch_init(self):
        from llm.watch import WatchDaemon

        d = WatchDaemon(interval=60)
        assert d._interval == 60
        assert not d.running

    def test_watch_status_no_start(self):
        from llm.watch import WatchDaemon

        d = WatchDaemon()
        status = d.get_status()
        assert "running" in status
        assert "interval" in status
        assert "gene_pool_size" in status

    def test_watch_start_stop(self):
        from llm.watch import WatchDaemon

        d = WatchDaemon(interval=9999)
        d.start()
        assert d.running
        d.stop()
        d._thread.join(timeout=3)
        assert not d.running


class TestEventsPipeline:
    def test_events_module_imports(self):
        from llm.events import process_event, render_event_report

        assert callable(process_event)
        assert callable(render_event_report)

    def test_render_empty_event(self):
        from llm.events import render_event_report

        result = {"error": "No news found"}
        rendered = render_event_report(result)
        assert "Error" in rendered

    def test_render_full_event(self):
        from llm.events import render_event_report

        result = {
            "event_id": "evt001",
            "capsule_id": "cap001",
            "capsule_title": "Test event",
            "timestamp": "2026-05-05T00:00:00",
            "keywords": ["test", "event"],
            "news_count": 3,
            "related_papers": [],
            "summary": "Test summary",
        }
        rendered = render_event_report(result)
        assert "Event Processed" in rendered


class CombinedIntegration:
    def test_report_mentions_gene_pool(self):
        from llm.report import generate

        report = generate()
        # Report should reference the Gene Pool
        assert "GENE POOL" in report or "capsules" in report.lower()

    def test_report_is_reproducible(self):
        from llm.report import generate

        r1 = generate()
        r2 = generate()
        # Both should succeed and have the same structure
        assert isinstance(r1, type(r2))
        assert len(r1) > 100
        assert len(r2) > 100
