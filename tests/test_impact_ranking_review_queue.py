"""Tests for impact_ranking, research_log, review_queue."""

import pytest
from llm.impact_ranking import compute_impact, render_impact_html
from llm.research_log import add_note, get_notes, render_log
from llm.review_queue import QueuedCapsule, get_review_queue, render_review_queue_html, _days_ago


class TestImpactRanking:
    def test_compute_impact_db_none(self):
        result = compute_impact(None)
        assert isinstance(result, (list, dict))

    def test_render_impact_html(self):
        result = render_impact_html([])
        assert isinstance(result, str)


class TestResearchLog:
    def test_add_note(self):
        ok = add_note("paper1", "This is a test note", tags=["test"])
        assert isinstance(ok, bool)

    def test_get_notes(self):
        notes = get_notes(paper_id="fake-paper-id", limit=5)
        assert isinstance(notes, list)

    def test_render_log(self):
        result = render_log(paper_id="fake")
        assert isinstance(result, str)


class TestQueuedCapsule:
    def test_fields(self):
        qc = QueuedCapsule(
            capsule_id="c1",
            gap_title="Test Gap",
            gap_type="methodology",
            polarity="positive",
            trigger_keywords=["AI"],
            outcome_score=0.8,
            source_paper_id="p1",
            created_days_ago=3,
        )
        assert qc.capsule_id == "c1"
        assert qc.outcome_score == 0.8


class TestReviewQueue:
    def test_get_review_queue(self):
        queue = get_review_queue()
        assert isinstance(queue, list)

    def test_render_review_queue_html(self):
        result = render_review_queue_html(queue=[])
        assert isinstance(result, str)

    def test_days_ago(self):
        result = _days_ago("2024-01-01T00:00:00")
        assert isinstance(result, int)
        assert result > 0
