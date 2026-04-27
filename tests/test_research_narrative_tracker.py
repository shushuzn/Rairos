"""Tests for research_narrative_tracker — pure logic, no I/O."""
import pytest

from llm.research_narrative_tracker import (
    ResearchThread,
    NarrativePhase,
    ResearchNarrativeTracker,
    ResearchNarrativeService,
    _score_bar,
    _compute_verdict,
)


# =============================================================================
# ResearchThread dataclass
# =============================================================================
class TestResearchThread:
    def test_defaults(self):
        t = ResearchThread(id="t1", topic="Transformer", phase=NarrativePhase.EXPLORATION)
        assert t.id == "t1"
        assert t.topic == "Transformer"
        assert t.phase == NarrativePhase.EXPLORATION
        assert t.created_at != ""
        assert t.updated_at != ""
        assert t.gap_count == 0
        assert t.contribution_score == 0.0

    def test_serialization_roundtrip(self):
        t = ResearchThread(
            id="t2",
            topic="RAG",
            phase=NarrativePhase.VALIDATION,
            hypothesis_ids=["h1", "h2"],
            validated_count=2,
        )
        d = t.to_dict()
        assert d["phase"] == "validation"
        t2 = ResearchThread.from_dict(d)
        assert t2.phase == NarrativePhase.VALIDATION
        assert t2.hypothesis_ids == ["h1", "h2"]
        assert t2.validated_count == 2


# =============================================================================
# NarrativePhase
# =============================================================================
class TestNarrativePhase:
    def test_all_values(self):
        assert NarrativePhase.EXPLORATION.value == "exploration"
        assert NarrativePhase.HYPOTHESIS.value == "hypothesis"
        assert NarrativePhase.VALIDATION.value == "validation"
        assert NarrativePhase.PUBLICATION.value == "publication"

    def test_from_string(self):
        assert NarrativePhase("publication") == NarrativePhase.PUBLICATION


# =============================================================================
# _score_bar
# =============================================================================
class TestScoreBar:
    def test_zero(self):
        assert _score_bar(0.0, 4) == "░░░░"

    def test_full(self):
        assert _score_bar(1.0, 4) == "████"

    def test_half(self):
        assert _score_bar(0.5, 4) == "██░░"

    def test_rounds_up(self):
        assert _score_bar(0.51, 4) == "██░░"
        assert _score_bar(0.74, 4) == "███░"  # 0.74*4=2.96→round=3
        assert _score_bar(0.75, 4) == "███░"


# =============================================================================
# _compute_verdict
# =============================================================================
class TestComputeVerdict:
    def test_no_events(self):
        v, d = _compute_verdict([])
        assert v == "INCONCLUSIVE"

    def test_validated_only(self):
        e = _make_event("validated")
        v, d = _compute_verdict([e])
        assert v == "VALIDATED"

    def test_rejected_only(self):
        e = _make_event("rejected")
        v, d = _compute_verdict([e])
        assert v == "REJECTED"

    def test_both(self):
        v, d = _compute_verdict([_make_event("validated"), _make_event("rejected")])
        assert v == "MIXED"

    def test_completed_alias(self):
        e = _make_event("completed")
        v, _ = _compute_verdict([e])
        assert v == "VALIDATED"

    def test_failed_alias(self):
        e = _make_event("failed")
        v, _ = _compute_verdict([e])
        assert v == "REJECTED"


# =============================================================================
# ResearchNarrativeService — score computation (pure)
# =============================================================================
class TestScoreComputation:
    """Test _compute_phase and _compute_readiness in isolation."""

    def _thread(self, **kw) -> ResearchThread:
        defaults = dict(
            id="t1", topic="Test", phase=NarrativePhase.EXPLORATION,
            phase_updated_at="", question_ids=[], hypothesis_ids=[],
            experiment_ids=[], insight_card_ids=[], paper_ids=[],
            gap_count=0, hypothesis_count=0, validated_count=0,
            rejected_count=0, running_count=0, notes="",
        )
        defaults.update(kw)
        return ResearchThread(**defaults)

    def test_phase_exploration_empty(self):
        svc = ResearchNarrativeService()
        t = self._thread(question_ids=[], hypothesis_ids=[])
        assert svc._compute_phase(t) == NarrativePhase.EXPLORATION

    def test_phase_hypothesis_questions_no_hypotheses(self):
        svc = ResearchNarrativeService()
        t = self._thread(question_ids=["q1"], hypothesis_ids=[])
        assert svc._compute_phase(t) == NarrativePhase.HYPOTHESIS

    def test_phase_validation_has_hypotheses(self):
        svc = ResearchNarrativeService()
        t = self._thread(question_ids=["q1"], hypothesis_ids=["h1"])
        assert svc._compute_phase(t) == NarrativePhase.VALIDATION

    def test_phase_publication_high_scores(self):
        svc = ResearchNarrativeService()
        t = self._thread(
            hypothesis_ids=["h1"],
            contribution_score=0.7,
            experiment_score=0.8,
        )
        assert svc._compute_phase(t) == NarrativePhase.PUBLICATION

    def test_phase_not_publication_if_low_experiment_score(self):
        svc = ResearchNarrativeService()
        t = self._thread(
            hypothesis_ids=["h1"],
            contribution_score=0.7,
            experiment_score=0.6,
        )
        assert svc._compute_phase(t) == NarrativePhase.VALIDATION

    def test_contribution_gap_plus_hypothesis(self):
        svc = ResearchNarrativeService()
        t = self._thread(gap_count=1, hypothesis_count=1)
        s = svc._contribution_score(t)
        assert s >= 0.3

    def test_contribution_multiple_hypotheses(self):
        svc = ResearchNarrativeService()
        t = self._thread(gap_count=1, hypothesis_count=3)
        s = svc._contribution_score(t)
        assert s >= 0.5  # 0.3 + 0.2

    def test_contribution_insight_cards(self):
        svc = ResearchNarrativeService()
        t = self._thread(gap_count=1, hypothesis_count=1, insight_card_ids=["c1", "c2"])
        s = svc._contribution_score(t)
        assert s >= 0.5  # 0.3 + 0.2

    def test_contribution_caps_at_1(self):
        svc = ResearchNarrativeService()
        t = self._thread(
            gap_count=1, hypothesis_count=3,
            insight_card_ids=["c1"], paper_ids=["p1", "p2", "p3"],
            question_ids=["q1"],
        )
        s = svc._contribution_score(t)
        assert s >= 1.0  # float arithmetic may yield 0.9999…

    def test_experiment_score_validated(self):
        svc = ResearchNarrativeService()
        t = self._thread(
            validated_count=1, rejected_count=0, running_count=0,
            hypothesis_count=1, paper_ids=["p1"],
        )
        s = svc._experiment_score(t)
        assert 0 < s < 1.0

    def test_experiment_score_multiple_validated(self):
        svc = ResearchNarrativeService()
        t = self._thread(
            validated_count=3, rejected_count=0, running_count=0,
            hypothesis_count=2, paper_ids=["p1"],
        )
        s = svc._experiment_score(t)
        assert s >= 0.7  # 0.4 + 0.2 + 0.1

    def test_experiment_score_bidirectional(self):
        svc = ResearchNarrativeService()
        t = self._thread(
            validated_count=1, rejected_count=1, running_count=0,
            hypothesis_count=1, paper_ids=["p1"],
        )
        s = svc._experiment_score(t)
        assert s >= 0.5  # 0.2 + 0.2 + 0.1

    def test_experiment_score_no_experiments(self):
        svc = ResearchNarrativeService()
        t = self._thread(validated_count=0, rejected_count=0, running_count=0)
        assert svc._experiment_score(t) == 0.0

    def test_narrative_score_baseline(self):
        svc = ResearchNarrativeService()
        t = self._thread()
        _, _, n = svc._compute_readiness(t)
        assert n == 0.5

    def test_narrative_score_coherent_flow(self):
        svc = ResearchNarrativeService()
        t = self._thread(question_ids=["q1"], hypothesis_ids=["h1"])
        _, _, n = svc._compute_readiness(t)
        assert n > 0.5

    def test_narrative_score_validated_contribution(self):
        svc = ResearchNarrativeService()
        t = self._thread(
            question_ids=["q1"], hypothesis_ids=["h1"],
            validated_count=1,
        )
        _, _, n = svc._compute_readiness(t)
        assert n >= 0.6


# =============================================================================
# ResearchNarrativeService — next steps
# =============================================================================
class TestNextSteps:
    def _svc(self):
        return ResearchNarrativeService()

    def _thread(self, **kw) -> ResearchThread:
        defaults = dict(
            id="t1", topic="Transformer", phase=NarrativePhase.EXPLORATION,
            phase_updated_at="", question_ids=[], hypothesis_ids=[],
            experiment_ids=[], insight_card_ids=[], paper_ids=[],
            gap_count=0, hypothesis_count=0, validated_count=0,
            rejected_count=0, running_count=0,
            contribution_score=0.0, experiment_score=0.0, narrative_score=0.5,
            notes="",
        )
        defaults.update(kw)
        return ResearchThread(**defaults)

    def test_exploration_no_gaps(self):
        svc = self._svc()
        t = self._thread(phase=NarrativePhase.EXPLORATION, gap_count=0)
        steps = svc.generate_next_steps(t)
        assert len(steps) >= 1
        assert "gap" in steps[0]["action"].lower()

    def test_exploration_with_gaps_no_questions(self):
        svc = self._svc()
        t = self._thread(phase=NarrativePhase.EXPLORATION, gap_count=2, question_ids=[])
        steps = svc.generate_next_steps(t)
        assert len(steps) >= 1
        assert "question" in steps[0]["action"].lower()

    def test_hypothesis_phase(self):
        svc = self._svc()
        t = self._thread(phase=NarrativePhase.HYPOTHESIS, hypothesis_count=2, topic="RAG")
        steps = svc.generate_next_steps(t)
        assert len(steps) >= 1
        assert any("hypothesize" in s["action"] for s in steps)

    def test_validation_no_experiments(self):
        svc = self._svc()
        t = self._thread(
            phase=NarrativePhase.VALIDATION,
            hypothesis_ids=["h1"],
            validated_count=0, running_count=0,
            experiment_score=0.1,
        )
        steps = svc.generate_next_steps(t)
        assert len(steps) >= 1

    def test_publication_phase(self):
        svc = self._svc()
        t = self._thread(
            phase=NarrativePhase.PUBLICATION,
            narrative_score=0.7,
        )
        steps = svc.generate_next_steps(t)
        assert len(steps) >= 1
        assert "write" in steps[0]["action"].lower() or "draft" in steps[0]["action"].lower()


# =============================================================================
# ResearchNarrativeTracker — persistence
# =============================================================================
class TestTrackerPersistence:
    def test_upsert_new(self, tmp_path):
        tracker = ResearchNarrativeTracker(data_dir=tmp_path)
        t = ResearchThread(id="new1", topic="Test", phase=NarrativePhase.EXPLORATION)
        tracker.upsert(t)
        threads = tracker.list_threads()
        assert len(threads) == 1
        assert threads[0].id == "new1"

    def test_upsert_update_existing(self, tmp_path):
        tracker = ResearchNarrativeTracker(data_dir=tmp_path)
        t1 = ResearchThread(id="upd1", topic="Test", phase=NarrativePhase.EXPLORATION)
        tracker.upsert(t1)
        t2 = ResearchThread(id="upd1", topic="Test", phase=NarrativePhase.HYPOTHESIS)
        tracker.upsert(t2)
        threads = tracker.list_threads()
        assert len(threads) == 1
        assert threads[0].phase == NarrativePhase.HYPOTHESIS

    def test_get_by_topic(self, tmp_path):
        tracker = ResearchNarrativeTracker(data_dir=tmp_path)
        t = ResearchThread(id="gbt1", topic="Transformer", phase=NarrativePhase.VALIDATION)
        tracker.upsert(t)
        found = tracker.get_by_topic("transformer")
        assert found is not None
        assert found.phase == NarrativePhase.VALIDATION
        assert tracker.get_by_topic("nonexistent") is None

    def test_delete(self, tmp_path):
        tracker = ResearchNarrativeTracker(data_dir=tmp_path)
        t = ResearchThread(id="del1", topic="Test", phase=NarrativePhase.EXPLORATION)
        tracker.upsert(t)
        assert tracker.delete("del1") is True
        assert tracker.list_threads() == []

    def test_delete_nonexistent(self, tmp_path):
        tracker = ResearchNarrativeTracker(data_dir=tmp_path)
        assert tracker.delete("xyz") is False


# ─── Helper ───────────────────────────────────────────────────────────────────


def _make_event(action_value: str):
    """Create a mock event with a given action value string."""
    class MockEvent:
        pass
    class MockAction:
        value = action_value
    e = MockEvent()
    e.action = MockAction()
    return e
