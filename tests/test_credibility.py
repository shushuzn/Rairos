"""Tests for llm.insight.credibility and llm.insight.trust_tracker."""

from __future__ import annotations

import json
import tempfile
from datetime import datetime, timedelta
from pathlib import Path

import pytest

from llm.insight.gene import CapsuleGene
from llm.insight.credibility import (
    CredibilityScorer,
    CredibilityScore,
    TRENDSLOP_KEYWORD_OVERLAP_THRESHOLD,
)
from llm.insight.trust_tracker import SourceTrustTracker, SourceTrustEntry


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_capsule(
    capsule_id: str = "c1",
    trigger_topic: str = "test",
    trigger_gap_type: str = "method_limitation",
    trigger_keywords: list[str] | None = None,
    outcome_success_score: float = 0.8,
    feedback_count: int = 5,
    source_category: str = "",
) -> CapsuleGene:
    if trigger_keywords is None:
        trigger_keywords = ["test", "keyword"]
    return CapsuleGene(
        capsule_id=capsule_id,
        created_at=datetime.now().isoformat(),
        trigger_topic=trigger_topic,
        trigger_gap_type=trigger_gap_type,
        trigger_keywords=trigger_keywords,
        action_gap_type=trigger_gap_type,
        action_gap_title="Test gap",
        outcome_success_score=outcome_success_score,
        feedback_count=feedback_count,
        archetype={"source_arxiv_category": source_category} if source_category else {},
        status="active",
    )


# ---------------------------------------------------------------------------
# CredibilityScorer
# ---------------------------------------------------------------------------


class TestCredibilityScorer:
    def test_empty_capsules(self):
        scorer = CredibilityScorer()
        scores = scorer.compute_novelty_scores([])
        assert scores == {}

    def test_single_capsule_no_overlap(self):
        c = _make_capsule(capsule_id="c1", trigger_keywords=["unique", "pattern"])
        scorer = CredibilityScorer()
        scores = scorer.compute_novelty_scores([c])
        s = scores.get("c1")
        assert s is not None
        assert s.novelty_v2 == 1.0  # no overlap possible
        assert not s.trendslop
        assert s.badge == "high"

    def test_trendslop_detection(self):
        c1 = _make_capsule(
            capsule_id="c1",
            trigger_keywords=["attention", "transformer", "scaling"],
            outcome_success_score=0.8,
            feedback_count=5,
        )
        c2 = _make_capsule(
            capsule_id="c2",
            trigger_keywords=["attention", "transformer", "efficiency"],
            outcome_success_score=0.7,
            feedback_count=3,
        )
        c3 = _make_capsule(
            capsule_id="c3",
            trigger_keywords=["graph", "gnn", "message_passing"],
            outcome_success_score=0.9,
            feedback_count=8,
        )
        scorer = CredibilityScorer()
        scores = scorer.compute_novelty_scores([c1, c2, c3])

        s3 = scores["c3"]
        assert not s3.trendslop  # unique keywords
        assert s3.badge == "high"

    def test_is_trendslop_utility(self):
        pool = [
            _make_capsule(
                capsule_id="p1",
                trigger_keywords=["attention", "transformer", "scaling"],
            ),
            _make_capsule(
                capsule_id="p2",
                trigger_keywords=["attention", "transformer", "efficiency"],
            ),
        ]
        new_c = _make_capsule(
            capsule_id="new",
            trigger_keywords=["attention", "transformer", "scaling", "efficiency"],
        )
        scorer = CredibilityScorer()
        is_ts, overlap, reason = scorer.is_trendslop(new_c, pool)
        # 3/4 keywords overlap vs p1, 3/4 vs p2 => Jaccard >= 0.5 but depends on exact sets
        assert isinstance(is_ts, bool)
        assert isinstance(overlap, float)
        assert 0.0 <= overlap <= 1.0
        assert isinstance(reason, str)

    def test_credibility_high_beats_low(self):
        c_high = _make_capsule(
            capsule_id="high",
            trigger_keywords=["unique", "breakthrough", "novel", "direction"],
            outcome_success_score=0.95,
            feedback_count=20,
        )
        c_low = _make_capsule(
            capsule_id="low",
            trigger_keywords=["generic", "common", "well_trodden", "path"],
            outcome_success_score=0.1,
            feedback_count=0,
        )
        scorer = CredibilityScorer()
        scores = scorer.compute_novelty_scores([c_high, c_low])
        assert scores["high"].overall > scores["low"].overall

    def test_render_report_empty(self):
        scorer = CredibilityScorer()
        report = scorer.render_credibility_report({}, [])
        assert "No capsules to assess" in report

    def test_render_report_with_data(self):
        c = _make_capsule(capsule_id="c1", trigger_keywords=["unique", "pattern"])
        scorer = CredibilityScorer()
        scores = scorer.compute_novelty_scores([c])
        report = scorer.render_credibility_report(scores, [c])
        assert "Credibility Report" in report
        assert "c1" in report or "HIGH" in report


# ---------------------------------------------------------------------------
# CredibilityScore dataclass
# ---------------------------------------------------------------------------


class TestCredibilityScore:
    def test_to_dict(self):
        s = CredibilityScore(
            capsule_id="c1",
            overall=0.85,
            novelty_v2=0.9,
            evidence_strength=0.8,
            source_trust=0.7,
            consistency=0.9,
            trendslop=False,
            trendslop_reason="",
            badge="high",
        )
        d = s.to_dict()
        assert d["capsule_id"] == "c1"
        assert d["overall"] == 0.85
        assert d["badge"] == "high"
        assert not d["trendslop"]

    def test_to_dict_rounds_floats(self):
        s = CredibilityScore(
            capsule_id="c2",
            overall=0.333333,
            novelty_v2=0.666666,
            evidence_strength=0.5,
            source_trust=0.5,
            consistency=0.5,
            trendslop=True,
            trendslop_reason="too similar",
            badge="medium",
        )
        d = s.to_dict()
        assert d["overall"] == 0.333
        assert d["novelty_v2"] == 0.667


# ---------------------------------------------------------------------------
# SourceTrustTracker
# ---------------------------------------------------------------------------


class TestSourceTrustTracker:
    def test_default_trust(self):
        with tempfile.TemporaryDirectory() as td:
            tracker = SourceTrustTracker(Path(td))
            assert tracker.get_trust("cs.Unknown") == 0.5
            assert tracker.get_all_trusts() == {}

    def test_update_from_capsule(self):
        with tempfile.TemporaryDirectory() as td:
            tracker = SourceTrustTracker(Path(td))
            c = _make_capsule(
                capsule_id="c1",
                trigger_keywords=["test"],
                outcome_success_score=0.9,
                feedback_count=5,
                source_category="cs.LG",
            )
            tracker.update_from_capsule(c)
            trust = tracker.get_trust("cs.LG")
            assert 0.0 <= trust <= 1.0
            assert trust > 0.4  # should be reasonable

    def test_batch_update(self):
        with tempfile.TemporaryDirectory() as td:
            tracker = SourceTrustTracker(Path(td))
            capsules = [
                _make_capsule(
                    "c1", outcome_success_score=0.9, feedback_count=5, source_category="cs.LG"
                ),
                _make_capsule(
                    "c2", outcome_success_score=0.8, feedback_count=3, source_category="cs.LG"
                ),
                _make_capsule(
                    "c3", outcome_success_score=0.7, feedback_count=2, source_category="cs.CL"
                ),
            ]
            tracker.batch_update(capsules)
            lg_trust = tracker.get_trust("cs.LG")
            cl_trust = tracker.get_trust("cs.CL")
            assert lg_trust > 0.0
            assert cl_trust > 0.0

    def test_render_table_empty(self):
        with tempfile.TemporaryDirectory() as td:
            tracker = SourceTrustTracker(Path(td))
            table = tracker.render_trust_table()
            assert "No source trust data yet" in table

    def test_render_table_with_data(self):
        with tempfile.TemporaryDirectory() as td:
            tracker = SourceTrustTracker(Path(td))
            c = _make_capsule(
                source_category="cs.LG",
                outcome_success_score=0.85,
                feedback_count=5,
            )
            tracker.update_from_capsule(c)
            table = tracker.render_trust_table()
            assert "cs.LG" in table
            assert "Trust" in table

    def test_no_category_does_not_crash(self):
        with tempfile.TemporaryDirectory() as td:
            tracker = SourceTrustTracker(Path(td))
            c = _make_capsule(source_category="")
            tracker.update_from_capsule(c)  # should be no-op
            assert tracker.get_all_trusts() == {}

    def test_multiple_categories(self):
        with tempfile.TemporaryDirectory() as td:
            tracker = SourceTrustTracker(Path(td))
            cats = ["cs.LG", "cs.CL", "cs.CV"]
            for i, cat in enumerate(cats):
                c = _make_capsule(f"c{i}", outcome_success_score=0.5 + i * 0.2, source_category=cat)
                tracker.update_from_capsule(c)
            all_trusts = tracker.get_all_trusts()
            assert len(all_trusts) == 3
            for cat in cats:
                assert cat in all_trusts

    def test_get_all_entries(self):
        with tempfile.TemporaryDirectory() as td:
            tracker = SourceTrustTracker(Path(td))
            c = _make_capsule(source_category="cs.LG")
            tracker.update_from_capsule(c)
            entries = tracker.get_all_entries()
            assert "cs.LG" in entries
            entry = entries["cs.LG"]
            assert isinstance(entry, SourceTrustEntry)
            assert entry.capsule_count == 1

    def test_persistence(self):
        with tempfile.TemporaryDirectory() as td:
            # Write
            tracker = SourceTrustTracker(Path(td))
            c = _make_capsule(
                capsule_id="c1",
                outcome_success_score=0.9,
                source_category="cs.LG",
            )
            tracker.update_from_capsule(c)
            first_trust = tracker.get_trust("cs.LG")

            # Read from new instance (round-trip through JSON)
            tracker2 = SourceTrustTracker(Path(td))
            second_trust = tracker2.get_trust("cs.LG")
            # Allow small diff from JSON round-trip rounding
            assert abs(second_trust - first_trust) < 0.01
