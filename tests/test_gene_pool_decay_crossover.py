"""Tests for gene_pool_decay and crossover core functions."""

import math
import pytest
from llm.gene_pool_decay import (
    CapsuleImpact,
    DecayState,
    MomentumState,
    compute_impact_score,
    compute_citation_boost,
    _get_adaptive_lambda,
    _now_iso,
    DEFAULT_LAMBDA,
)
from llm.crossover import (
    compute_fitness,
    compute_trust,
    _sanitize_archetype,
    DebateEntry,
    _score_argument,
)


# ── gene_pool_decay dataclasses ──────────────────────────────────────────────


class TestCapsuleImpact:
    def test_defaults(self):
        ci = CapsuleImpact("c1", 0.5, 10.0, 3, 0.8, 1.1, 5)
        assert ci.inbound_citations == 5
        assert ci.indirect_citations == 0
        assert ci.archived is False


class TestDecayState:
    def test_defaults(self):
        ds = DecayState()
        assert ds.last_decay_at == ""

    def test_with_data(self):
        ds = DecayState(
            last_decay_at="2026-01-01", consecutive_low_impact={"c1": 2}, total_archived=5
        )
        assert ds.consecutive_low_impact["c1"] == 2


class TestMomentumState:
    def test_defaults(self):
        ms = MomentumState()
        assert ms.new_by_gap_type == {}


# ── compute_impact_score ──────────────────────────────────────────────────────


class TestComputeImpactScore:
    def test_recent_with_feedback(self):
        """Recent capsule with feedback should have positive impact."""
        impact, age = compute_impact_score(1.0, _now_iso(), feedback_count=10, lambda_=0.01)
        assert impact > 0.5
        assert age < 1.0

    def test_old_decays(self):
        """Old capsule decays more."""
        impact_old, _ = compute_impact_score(1.0, "2020-01-01T00:00:00", 10, lambda_=0.01)
        impact_new, _ = compute_impact_score(1.0, _now_iso(), 10, lambda_=0.01)
        assert impact_old < impact_new

    def test_feedback_zero_makes_zero(self):
        """log(1)=0 so impact=0 regardless of other factors."""
        impact, _ = compute_impact_score(1.0, _now_iso(), 0, lambda_=0.01)
        assert impact == 0.0

    def test_citations_boost(self):
        """With feedback>0, more citations → higher impact."""
        impact_low, _ = compute_impact_score(
            1.0, "2024-01-01T00:00:00", 5, inbound_citations=0, lambda_=0.01
        )
        impact_high, _ = compute_impact_score(
            1.0, "2024-01-01T00:00:00", 5, inbound_citations=10, lambda_=0.01
        )
        assert impact_high > impact_low

    def test_invalid_date(self):
        impact, age = compute_impact_score(0.5, "bad-date", 0, lambda_=0.01)
        assert age == 0.0

    def test_citation_override(self):
        impact1, _ = compute_impact_score(
            1.0, _now_iso(), 1, inbound_citations=100, citation_boost_override=1.0, lambda_=0.01
        )
        impact2, _ = compute_impact_score(1.0, _now_iso(), 1, inbound_citations=0, lambda_=0.01)
        assert impact1 == pytest.approx(impact2)

    def test_lambda_effect(self):
        """Higher lambda reduces impact."""
        # Use very recent date so decay is visible but not zero
        from datetime import datetime, timedelta

        recent = (datetime.now() - timedelta(days=10)).isoformat()
        impact_slow, _ = compute_impact_score(1.0, recent, 5, lambda_=0.01)
        impact_fast, _ = compute_impact_score(1.0, recent, 5, lambda_=0.10)
        assert impact_fast < impact_slow


# ── compute_citation_boost ────────────────────────────────────────────────────


class TestComputeCitationBoost:
    def test_no_citations(self):
        assert compute_citation_boost(0) == 1.0

    def test_direct(self):
        assert compute_citation_boost(5) == pytest.approx(1.5)

    def test_with_indirect(self):
        assert compute_citation_boost(5, indirect=3) > 1.5


# ── _get_adaptive_lambda ──────────────────────────────────────────────────────


class TestGetAdaptiveLambda:
    def test_known(self):
        assert _get_adaptive_lambda("cs.AI") == 0.02

    def test_unknown(self):
        assert _get_adaptive_lambda("unknown.field") == DEFAULT_LAMBDA


# ── crossover: compute_fitness ────────────────────────────────────────────────


class FakeCapsule:
    def __init__(self, score, feedback, created_at="2026-01-01T00:00:00"):
        self.outcome_success_score = score
        self.feedback_count = feedback
        self.created_at = created_at


class TestComputeFitness:
    def test_positive(self):
        cap = FakeCapsule(0.9, 100)
        assert compute_fitness(cap) > 1.0

    def test_zero_feedback(self):
        cap = FakeCapsule(0.5, 0)
        assert compute_fitness(cap) == 0.0


# ── crossover: compute_trust ──────────────────────────────────────────────────


class TestComputeTrust:
    def test_with_citations(self):
        cap = FakeCapsule(0.8, 5)
        trust = compute_trust(cap, inbound_citations=10)
        assert trust > 0.5

    def test_no_citations(self):
        cap = FakeCapsule(0.8, 5)
        trust = compute_trust(cap, inbound_citations=0)
        assert trust > 0


# ── crossover: _sanitize_archetype ────────────────────────────────────────────


class TestSanitizeArchetype:
    def test_passthrough(self):
        """_sanitize_archetype doesn't filter — it passes values through."""
        data = {"method_focused": 0.5, "app_focused": 0.3}
        result = _sanitize_archetype(data)
        assert result == data

    def test_extra_keys_preserved(self):
        data = {"method_focused": 0.5, "extra_key": "value"}
        result = _sanitize_archetype(data)
        assert "extra_key" in result


# ── crossover: DebateEntry ────────────────────────────────────────────────────


class TestDebateEntry:
    def test_fields(self):
        entry = DebateEntry(
            debate_id="d1",
            capsule_a_id="ca",
            capsule_b_id="cb",
            gap_type="method",
            score_a=0.8,
            score_b=0.6,
            winner_id="ca",
            loser_id="cb",
            judged_at="2026-01-01",
        )
        assert entry.winner_id == "ca"
        assert entry.score_a > entry.score_b


# ── crossover: _score_argument ────────────────────────────────────────────────


class TestScoreArgument:
    def test_positive(self):
        cap = FakeCapsule(0.7, 10)
        score = _score_argument(cap, inbound_citations=5)
        assert score > 0

    def test_no_citations(self):
        cap = FakeCapsule(0.5, 1)
        score = _score_argument(cap, inbound_citations=0)
        assert score > 0
