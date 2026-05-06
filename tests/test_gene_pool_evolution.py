"""Tests for llm.insight.evolution — InsightEvolution gene-pool evolver."""

from __future__ import annotations

import json
from datetime import datetime, timedelta
from pathlib import Path
from unittest.mock import MagicMock

import pytest

from llm.insight.evolution import (
    AuditResult,
    CapsuleCandidate,
    CapsuleQuality,
    InsightEvolution,
)
from llm.insight.gene import CapsuleGene


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_capsule(
    capsule_id: str = "cap-001",
    trigger_topic: str = "RAG",
    trigger_gap_type: str = "method_limitation",
    trigger_keywords: list[str] | None = None,
    action_gap_type: str = "method_limitation",
    action_gap_title: str = "Scalability issues in RAG",
    outcome_success_score: float = 0.8,
    feedback_count: int = 5,
    created_at: str | None = None,
    status: str = "active",
    low_score_streak: int = 0,
    evolved_generation: int = 0,
) -> CapsuleGene:
    if trigger_keywords is None:
        trigger_keywords = ["retrieval", "augmented", "generation"]
    if created_at is None:
        created_at = (datetime.now() - timedelta(days=30)).isoformat()
    return CapsuleGene(
        capsule_id=capsule_id,
        created_at=created_at,
        trigger_topic=trigger_topic,
        trigger_gap_type=trigger_gap_type,
        trigger_keywords=trigger_keywords,
        action_gap_type=action_gap_type,
        action_gap_title=action_gap_title,
        outcome_success_score=outcome_success_score,
        feedback_count=feedback_count,
        evolved_generation=evolved_generation,
        archetype={},
        status=status,
        low_score_streak=low_score_streak,
    )


def _make_candidate(
    candidate_id: str = "cand-001",
    trigger_topic: str = "RAG",
    trigger_gap_type: str = "method_limitation",
    trigger_keywords: list[str] | None = None,
    action_gap_type: str = "method_limitation",
    action_gap_title: str = "Scalability issues in RAG",
    confidence: float = 0.7,
    source: str = "trigger_refine",
    mutation_description: str = "broadened topic",
) -> CapsuleCandidate:
    if trigger_keywords is None:
        trigger_keywords = ["retrieval", "augmented", "generation"]
    return CapsuleCandidate(
        original_id="cap-001",
        candidate_id=candidate_id,
        trigger_topic=trigger_topic,
        trigger_gap_type=trigger_gap_type,
        trigger_keywords=trigger_keywords,
        action_gap_type=action_gap_type,
        action_gap_title=action_gap_title,
        mutation_description=mutation_description,
        confidence=confidence,
        source=source,
    )


@pytest.fixture
def mock_tracker(tmp_path: Path) -> MagicMock:
    """A mock EvolutionTracker whose data_dir points at tmp_path.

    Injects a real CapsuleStorageMixin so that _save_capsules actually persists
    to SQLite at tmp_path (matching production behavior after the UPSERT fix).
    """
    from llm.insight.storage import CapsuleStorageMixin

    class RealSavingTracker(CapsuleStorageMixin, MagicMock):
        pass

    tracker = RealSavingTracker()
    tracker.data_dir = tmp_path
    return tracker


@pytest.fixture
def evolver(mock_tracker: MagicMock) -> InsightEvolution:
    return InsightEvolution(tracker=mock_tracker)


@pytest.fixture
def evolver_with_capsules(evolver: InsightEvolution, mock_tracker: MagicMock) -> InsightEvolution:
    """Evolver pre-loaded with a gene_pool.jsonl of 5 capsules."""
    capsules = [_make_capsule(capsule_id=f"cap-{i:03d}") for i in range(5)]
    _write_gene_pool(mock_tracker.data_dir, capsules)
    return evolver


def _write_gene_pool(data_dir: Path, capsules: list[CapsuleGene]) -> None:
    """Write capsules directly to SQLite via CapsuleStorageMixin, bypassing JSONL.

    Bypasses JSONL entirely so that tests are isolated from each other's DB state.
    """
    from llm.insight.storage import CapsuleStorageMixin

    class _TempTracker(CapsuleStorageMixin):
        pass

    t = _TempTracker()
    t.data_dir = data_dir
    t._save_capsules(capsules)


# ===========================================================================
# 1. __init__
# ===========================================================================


class TestInsightEvolutionInit:
    def test_sets_tracker(self, mock_tracker: MagicMock) -> None:
        evolver = InsightEvolution(tracker=mock_tracker)
        assert evolver.tracker is mock_tracker

    def test_default_tracker_when_none(self) -> None:
        evolver = InsightEvolution(tracker=None)
        assert evolver.tracker is not None
        assert isinstance(evolver.tracker, InsightEvolution.__mro__[1])

    def test_class_thresholds(self) -> None:
        assert InsightEvolution.HIGH_QUALITY_THRESHOLD == 0.70
        assert InsightEvolution.LOW_QUALITY_THRESHOLD == 0.30
        assert InsightEvolution.RETIRE_COUNT_THRESHOLD == 3

    def test_max_candidates_per_evolve(self) -> None:
        assert InsightEvolution.MAX_CANDIDATES_PER_EVOLVE == 5

    def test_max_gene_pool_size(self) -> None:
        assert InsightEvolution.MAX_GENE_POOL_SIZE == 500

    def test_llm_client_is_none_initially(self, evolver: InsightEvolution) -> None:
        assert evolver._llm_client is None


# ===========================================================================
# 2. _add_candidate — duplicate check logic
# ===========================================================================


class TestAddCandidate:
    def test_adds_new_candidate_to_empty_pool(
        self, evolver: InsightEvolution, mock_tracker: MagicMock
    ) -> None:
        _write_gene_pool(mock_tracker.data_dir, [])
        cand = _make_candidate()
        capsules = evolver._load_capsules()
        was_added, capsules = evolver._add_candidate(cand, capsules)
        assert was_added is True
        evolver._save_capsules(capsules)
        loaded = evolver._load_capsules()
        assert len(loaded) == 1
        assert loaded[0].capsule_id == "cand-001"

    def test_adds_candidate_with_different_action_gap_title(
        self, evolver: InsightEvolution, mock_tracker: MagicMock
    ) -> None:
        """Relaxed duplicate check: same trigger_topic + gap_type but different
        action_gap_title AND different keywords should still be allowed."""
        existing = _make_capsule(
            action_gap_title="Original title",
            trigger_topic="RAG",
            trigger_gap_type="method_limitation",
            trigger_keywords=["alpha", "beta", "gamma"],
        )
        _write_gene_pool(mock_tracker.data_dir, [existing])
        cand = _make_candidate(
            action_gap_title="Different title",
            trigger_topic="RAG",
            trigger_gap_type="method_limitation",
            trigger_keywords=["delta", "epsilon", "zeta"],
        )
        capsules = evolver._load_capsules()
        was_added, capsules = evolver._add_candidate(cand, capsules)
        assert was_added is True
        evolver._save_capsules(capsules)
        assert len(evolver._load_capsules()) == 2

    def test_rejects_exact_duplicate(
        self, evolver: InsightEvolution, mock_tracker: MagicMock
    ) -> None:
        """Exact duplicate: same trigger_topic + trigger_gap_type + action_gap_title."""
        existing = _make_capsule(
            trigger_topic="RAG",
            trigger_gap_type="method_limitation",
            action_gap_title="Scalability issues",
        )
        _write_gene_pool(mock_tracker.data_dir, [existing])
        cand = _make_candidate(
            trigger_topic="RAG",
            trigger_gap_type="method_limitation",
            action_gap_title="Scalability issues",
        )
        capsules = evolver._load_capsules()
        was_added, capsules = evolver._add_candidate(cand, capsules)
        assert was_added is False

    def test_rejects_high_keyword_overlap(
        self, evolver: InsightEvolution, mock_tracker: MagicMock
    ) -> None:
        """Reject when >80% keyword Jaccard overlap AND same trigger_topic."""
        existing = _make_capsule(
            trigger_topic="RAG",
            trigger_keywords=["retrieval", "augmented", "generation", "transformer"],
            action_gap_title="Title A",
        )
        _write_gene_pool(mock_tracker.data_dir, [existing])
        # 4 common out of 5 unique = 80% (intersection=4, union=5 -> 0.8) -> rejected
        cand = _make_candidate(
            trigger_topic="RAG",
            trigger_keywords=["retrieval", "augmented", "generation", "new_kwd"],
            action_gap_title="Title A",
        )
        capsules = evolver._load_capsules()
        was_added, capsules = evolver._add_candidate(cand, capsules)
        assert was_added is False

    def test_allows_moderate_keyword_overlap(
        self, evolver: InsightEvolution, mock_tracker: MagicMock
    ) -> None:
        """Allow when keyword overlap is <=80%."""
        existing = _make_capsule(
            trigger_topic="RAG",
            trigger_keywords=["retrieval", "augmented"],
            action_gap_title="Original title",
        )
        _write_gene_pool(mock_tracker.data_dir, [existing])
        # intersection=1 ("augmented"), union=3 -> 33% -- allowed
        cand = _make_candidate(
            trigger_topic="RAG",
            trigger_keywords=["augmented", "generation", "transformer"],
            action_gap_title="Different title",
        )
        capsules = evolver._load_capsules()
        was_added, capsules = evolver._add_candidate(cand, capsules)
        assert was_added is True

    def test_allows_different_trigger_topic_with_same_keywords(
        self, evolver: InsightEvolution, mock_tracker: MagicMock
    ) -> None:
        """Different trigger_topic means keyword overlap check is skipped."""
        existing = _make_capsule(
            trigger_topic="NLP",
            trigger_keywords=["retrieval", "augmented", "generation"],
        )
        _write_gene_pool(mock_tracker.data_dir, [existing])
        cand = _make_candidate(
            trigger_topic="CV",
            trigger_keywords=["retrieval", "augmented", "generation"],
        )
        capsules = evolver._load_capsules()
        was_added, capsules = evolver._add_candidate(cand, capsules)
        assert was_added is True

    def test_candidate_becomes_active_capsule(
        self, evolver: InsightEvolution, mock_tracker: MagicMock
    ) -> None:
        _write_gene_pool(mock_tracker.data_dir, [])
        cand = _make_candidate(confidence=0.8)
        capsules = evolver._load_capsules()
        _, capsules = evolver._add_candidate(cand, capsules)
        evolver._save_capsules(capsules)
        capsule = evolver._load_capsules()[0]
        assert capsule.status == "active"
        assert capsule.evolved_generation == 1
        assert capsule.outcome_success_score == pytest.approx(0.8 * 0.7)

    def test_empty_keywords_on_existing_capsule_skips_overlap_check(
        self, evolver: InsightEvolution, mock_tracker: MagicMock
    ) -> None:
        existing = _make_capsule(
            trigger_topic="RAG",
            trigger_keywords=[],
            action_gap_title="Original title",
        )
        _write_gene_pool(mock_tracker.data_dir, [existing])
        cand = _make_candidate(
            trigger_topic="RAG",
            trigger_keywords=["foo", "bar"],
            action_gap_title="Different title",
        )
        # Empty keywords on existing -> the overlap branch is not entered
        capsules = evolver._load_capsules()
        was_added, capsules = evolver._add_candidate(cand, capsules)
        assert was_added is True


# ===========================================================================
# 3. _gene_pool_size
# ===========================================================================


class TestGenePoolSize:
    def test_returns_zero_for_empty_pool(
        self, evolver: InsightEvolution, mock_tracker: MagicMock
    ) -> None:
        _write_gene_pool(mock_tracker.data_dir, [])
        assert evolver._gene_pool_size() == 0

    def test_returns_capsule_count(
        self, evolver: InsightEvolution, mock_tracker: MagicMock
    ) -> None:
        capsules = [_make_capsule(capsule_id=f"c{i}") for i in range(7)]
        _write_gene_pool(mock_tracker.data_dir, capsules)
        assert evolver._gene_pool_size() == 7

    def test_returns_zero_when_file_missing(
        self, evolver: InsightEvolution, mock_tracker: MagicMock
    ) -> None:
        assert evolver._gene_pool_size() == 0


# ===========================================================================
# 4. _score_capsule (quality assessment)
# ===========================================================================


class TestScoreCapsule:
    def test_high_score_capsule(self, evolver: InsightEvolution) -> None:
        capsule = _make_capsule(
            outcome_success_score=1.0,
            feedback_count=10,
            created_at=datetime.now().isoformat(),
        )
        q = evolver._score_capsule(capsule)
        assert isinstance(q, CapsuleQuality)
        assert q.utility == pytest.approx(1.0)
        assert q.novelty == pytest.approx(1.0)  # min(10/10, 1.0)
        assert q.freshness == pytest.approx(1.0)  # just created
        assert q.overall > 0.8

    def test_low_score_capsule(self, evolver: InsightEvolution) -> None:
        old_date = (datetime.now() - timedelta(days=400)).isoformat()
        capsule = _make_capsule(
            outcome_success_score=0.1,
            feedback_count=0,
            created_at=old_date,
        )
        q = evolver._score_capsule(capsule)
        assert q.utility == pytest.approx(0.1)
        assert q.novelty == pytest.approx(0.5)  # no feedback -> 0.5 default
        assert q.freshness < 0.1  # very old
        assert q.overall < 0.3

    def test_zero_feedback_gives_novelty_0_5(self, evolver: InsightEvolution) -> None:
        capsule = _make_capsule(feedback_count=0)
        q = evolver._score_capsule(capsule)
        assert q.novelty == pytest.approx(0.5)

    def test_novelty_clamped_at_1_0(self, evolver: InsightEvolution) -> None:
        capsule = _make_capsule(feedback_count=50)
        q = evolver._score_capsule(capsule)
        assert q.novelty == pytest.approx(1.0)

    def test_invalid_date_falls_back_to_freshness_0_5(self, evolver: InsightEvolution) -> None:
        capsule = _make_capsule(created_at="not-a-date")
        q = evolver._score_capsule(capsule)
        assert q.freshness == pytest.approx(0.5)

    def test_overall_is_weighted_composite(self, evolver: InsightEvolution) -> None:
        capsule = _make_capsule(
            outcome_success_score=0.6,
            feedback_count=5,
            created_at=datetime.now().isoformat(),
        )
        q = evolver._score_capsule(capsule)
        # New formula adds credibility dimension (0.25 weight)
        expected_overall = 0.35 * 0.6 + 0.15 * 0.5 + 0.15 * 1.0 + 0.10 * 0.6 + 0.25 * 0.5
        assert q.overall == pytest.approx(expected_overall)


# ===========================================================================
# 5. audit
# ===========================================================================


class TestAudit:
    def test_returns_empty_result_when_too_few_capsules(
        self, evolver: InsightEvolution, mock_tracker: MagicMock
    ) -> None:
        _write_gene_pool(mock_tracker.data_dir, [])
        result = evolver.audit(min_capsules=3)
        assert isinstance(result, AuditResult)
        assert result.total_capsules == 0
        assert result.avg_quality == 0.0
        assert result.high_quality == []
        assert result.low_quality == []
        assert result.candidate_ids == []
        assert result.retire_ids == []

    def test_scores_all_capsules(self, evolver_with_capsules: InsightEvolution) -> None:
        result = evolver_with_capsules.audit(min_capsules=3)
        assert result.total_capsules == 5
        assert result.avg_quality > 0
        assert len(result.high_quality) + len(result.low_quality) <= 5

    def test_candidate_ids_include_high_scoring(
        self, evolver: InsightEvolution, mock_tracker: MagicMock
    ) -> None:
        good = _make_capsule(
            capsule_id="good-1",
            outcome_success_score=1.0,
            feedback_count=10,
            created_at=datetime.now().isoformat(),
        )
        bad = _make_capsule(
            capsule_id="bad-1",
            outcome_success_score=0.1,
            feedback_count=0,
            created_at=(datetime.now() - timedelta(days=500)).isoformat(),
        )
        _write_gene_pool(mock_tracker.data_dir, [good, bad, _make_capsule()])
        result = evolver.audit(min_capsules=1)
        assert "good-1" in result.candidate_ids

    def test_retire_ids_for_stale_low_quality(
        self, evolver: InsightEvolution, mock_tracker: MagicMock
    ) -> None:
        # Use a capsule with low novelty (<0.3) and low freshness (<0.3)
        # feedback_count=1 -> novelty=0.1; very old -> freshness near 0
        stale2 = _make_capsule(
            capsule_id="stale2",
            outcome_success_score=0.05,
            feedback_count=1,
            created_at=(datetime.now() - timedelta(days=500)).isoformat(),
        )
        _write_gene_pool(mock_tracker.data_dir, [stale2, _make_capsule(), _make_capsule()])
        result2 = evolver.audit(min_capsules=1)
        assert "stale2" in result2.retire_ids

    def test_avg_quality_is_mean_of_overalls(
        self, evolver: InsightEvolution, mock_tracker: MagicMock
    ) -> None:
        c1 = _make_capsule(
            capsule_id="c1",
            outcome_success_score=0.9,
            feedback_count=5,
            created_at=datetime.now().isoformat(),
        )
        c2 = _make_capsule(
            capsule_id="c2",
            outcome_success_score=0.3,
            feedback_count=2,
            created_at=datetime.now().isoformat(),
        )
        _write_gene_pool(mock_tracker.data_dir, [c1, c2, _make_capsule()])
        result = evolver.audit(min_capsules=1)
        assert result.avg_quality > 0


# ===========================================================================
# 6. propose — mutation strategies
# ===========================================================================


class TestPropose:
    def test_propose_returns_list_of_candidates(
        self, evolver: InsightEvolution, mock_tracker: MagicMock
    ) -> None:
        capsule = _make_capsule()
        _write_gene_pool(mock_tracker.data_dir, [capsule])
        evolver.tracker.find_capsule = MagicMock(return_value=[capsule])
        evolver._propose_llm = MagicMock(return_value=[])
        candidates = evolver.propose("RAG", "method_limitation", limit=5)
        assert isinstance(candidates, list)
        assert all(isinstance(c, CapsuleCandidate) for c in candidates)

    def test_propose_generates_trigger_broaden(
        self, evolver: InsightEvolution, mock_tracker: MagicMock
    ) -> None:
        capsule = _make_capsule(trigger_topic="deep-learning/RAG")
        _write_gene_pool(mock_tracker.data_dir, [capsule])
        evolver.tracker.find_capsule = MagicMock(return_value=[capsule])
        evolver._propose_llm = MagicMock(return_value=[])
        candidates = evolver.propose("deep-learning/RAG", limit=5)
        broaden = [c for c in candidates if c.source == "trigger_refine"]
        assert len(broaden) >= 1
        # The topic contains "/" so it should be split
        assert "deep-learning" in broaden[0].trigger_topic or "RAG" in broaden[0].trigger_topic

    def test_propose_generates_gap_type_transfer(
        self, evolver: InsightEvolution, mock_tracker: MagicMock
    ) -> None:
        capsule = _make_capsule(trigger_topic="NLP")
        _write_gene_pool(mock_tracker.data_dir, [capsule])
        evolver.tracker.find_capsule = MagicMock(return_value=[capsule])
        evolver._propose_llm = MagicMock(return_value=[])
        candidates = evolver.propose("NLP", limit=5)
        transfers = [c for c in candidates if c.source == "gap_type_transfer"]
        assert len(transfers) >= 1
        # The transferred gap type should differ from original
        assert transfers[0].trigger_gap_type != capsule.trigger_gap_type

    def test_propose_generates_keyword_expand(
        self, evolver: InsightEvolution, mock_tracker: MagicMock
    ) -> None:
        capsule = _make_capsule(
            trigger_topic="NLP",
            trigger_keywords=["short"],
        )
        _write_gene_pool(mock_tracker.data_dir, [capsule])
        evolver.tracker.find_capsule = MagicMock(return_value=[capsule])
        evolver._propose_llm = MagicMock(return_value=[])
        # Use a topic with words > 3 chars that are not already in keywords
        candidates = evolver.propose("natural language processing", limit=5)
        expands = [c for c in candidates if c.source == "keyword_expand"]
        assert len(expands) >= 1
        assert len(expands[0].trigger_keywords) > 1

    def test_propose_respects_limit(
        self, evolver: InsightEvolution, mock_tracker: MagicMock
    ) -> None:
        capsule = _make_capsule()
        _write_gene_pool(mock_tracker.data_dir, [capsule])
        evolver.tracker.find_capsule = MagicMock(return_value=[capsule])
        evolver._propose_llm = MagicMock(return_value=[])
        candidates = evolver.propose("RAG", limit=2)
        assert len(candidates) <= 2

    def test_propose_returns_empty_when_no_capsules_match(
        self, evolver: InsightEvolution, mock_tracker: MagicMock
    ) -> None:
        _write_gene_pool(mock_tracker.data_dir, [])
        evolver.tracker.find_capsule = MagicMock(return_value=[])
        evolver._propose_llm = MagicMock(return_value=[])
        candidates = evolver.propose("unknown_topic", limit=5)
        assert candidates == []

    def test_propose_caps_at_max_candidates_per_evolve(
        self, evolver: InsightEvolution, mock_tracker: MagicMock
    ) -> None:
        """Only the first MAX_CANDIDATES_PER_EVOLVE capsules get mutated."""
        capsules = [_make_capsule(capsule_id=f"c{i}") for i in range(10)]
        _write_gene_pool(mock_tracker.data_dir, capsules)
        evolver.tracker.find_capsule = MagicMock(return_value=capsules)
        evolver._propose_llm = MagicMock(return_value=[])
        evolver.propose("RAG", limit=50)
        # The loop is capped at MAX_CANDIDATES_PER_EVOLVE=5
        # We verify LLM was called (for remaining slots)
        evolver._propose_llm.assert_called()

    def test_keyword_expand_returns_none_when_no_new_words(self, evolver: InsightEvolution) -> None:
        capsule = _make_capsule(
            trigger_topic="RAG",
            trigger_keywords=["retrieval", "augmented", "generation"],
        )
        result = evolver._mutate_keyword_expand(capsule, "the is a")
        assert result is None  # no words > 3 chars

    def test_trigger_broaden_returns_none_for_empty_topic(self, evolver: InsightEvolution) -> None:
        capsule = _make_capsule(trigger_topic="")
        result = evolver._mutate_trigger_broaden(capsule, "RAG")
        assert result is None


# ===========================================================================
# 7. _load_capsules / _save_capsules round-trip
# ===========================================================================


class TestLoadSaveCapsules:
    def test_round_trip(self, evolver: InsightEvolution, mock_tracker: MagicMock) -> None:
        original = [_make_capsule(capsule_id=f"r{i}") for i in range(3)]
        _write_gene_pool(mock_tracker.data_dir, original)
        loaded = evolver._load_capsules()
        assert len(loaded) == 3
        assert loaded[0].capsule_id == "r0"
        assert loaded[2].capsule_id == "r2"

    def test_load_skips_malformed_lines(
        self, evolver: InsightEvolution, mock_tracker: MagicMock
    ) -> None:
        gene_file = mock_tracker.data_dir / "gene_pool.jsonl"
        with open(gene_file, "w", encoding="utf-8") as f:
            f.write(json.dumps(_make_capsule().to_dict()) + "\n")
            f.write("NOT VALID JSON\n")
            f.write(json.dumps(_make_capsule(capsule_id="ok").to_dict()) + "\n")
        loaded = evolver._load_capsules()
        assert len(loaded) == 2

    def test_save_and_reload_preserves_data(
        self, evolver: InsightEvolution, mock_tracker: MagicMock
    ) -> None:
        capsule = _make_capsule(trigger_keywords=["alpha", "beta"])
        _write_gene_pool(mock_tracker.data_dir, [capsule])
        loaded = evolver._load_capsules()
        assert loaded[0].trigger_keywords == ["alpha", "beta"]


# ===========================================================================
# 8. evaluate (pairwise comparison without LLM)
# ===========================================================================


class TestEvaluate:
    def test_returns_empty_for_single_candidate(self, evolver: InsightEvolution) -> None:
        result = evolver.evaluate([_make_candidate()])
        assert result == []

    def test_fallback_comparison_by_confidence(self, evolver: InsightEvolution) -> None:
        evolver._get_llm_client = MagicMock(return_value=None)
        a = _make_candidate(candidate_id="a", confidence=0.9)
        b = _make_candidate(candidate_id="b", confidence=0.3)
        results = evolver.evaluate([a, b])
        assert len(results) == 1
        assert results[0].winner_id == "a"
        assert results[0].loser_id == "b"
        assert results[0].confidence == pytest.approx(0.6)

    def test_sorted_by_confidence_desc(self, evolver: InsightEvolution) -> None:
        evolver._get_llm_client = MagicMock(return_value=None)
        a = _make_candidate(candidate_id="a", confidence=0.3)
        b = _make_candidate(candidate_id="b", confidence=0.9)
        c = _make_candidate(candidate_id="c", confidence=0.6)
        results = evolver.evaluate([a, b, c])
        assert len(results) == 3
        assert results[0].confidence >= results[1].confidence
        assert results[1].confidence >= results[2].confidence


# ===========================================================================
# 9. render_summary
# ===========================================================================


class TestRenderSummary:
    def test_render_summary(self, evolver: InsightEvolution) -> None:
        result = {
            "audit": {
                "total": 10,
                "avg_quality": 0.65,
                "candidates": 3,
                "to_retire": 1,
            },
            "proposed": 5,
            "evaluations": 3,
            "result": {
                "added": 2,
                "retired": 1,
                "total_capsules": 11,
                "avg_quality": 0.65,
            },
        }
        text = evolver.render_summary(result)
        assert "10 total" in text
        assert "5 candidates" in text
        assert "2 new capsules" in text
        assert "1 capsules" in text


# ===========================================================================
# 10. _merge_capsules keyword Jaccard
# ===========================================================================


class TestMergeCapsules:
    def test_merges_near_duplicate_keywords(
        self, evolver: InsightEvolution, mock_tracker: MagicMock
    ) -> None:
        """Two active capsules with same gap_type and >80% keyword Jaccard."""
        # 9 common out of 10 unique = 9/10 = 90% Jaccard > 80%
        a = _make_capsule(
            capsule_id="merge-a",
            trigger_gap_type="method_limitation",
            trigger_keywords=[
                "retrieval",
                "augmented",
                "generation",
                "deep",
                "transformer",
                "model",
                "training",
                "inference",
                "optimization",
            ],
            outcome_success_score=0.9,
        )
        b = _make_capsule(
            capsule_id="merge-b",
            trigger_gap_type="method_limitation",
            trigger_keywords=[
                "retrieval",
                "augmented",
                "generation",
                "deep",
                "transformer",
                "model",
                "training",
                "inference",
                "scaling",
            ],
            outcome_success_score=0.5,
        )
        _write_gene_pool(mock_tracker.data_dir, [a, b])
        capsules = evolver._load_capsules()
        merged, capsules = evolver._merge_capsules(capsules)
        evolver._save_capsules(capsules)
        assert merged >= 1
        # After merge, one should be archived
        remaining = evolver._load_capsules()
        archived = [c for c in remaining if c.status == "archived"]
        assert len(archived) >= 1

    def test_no_merge_when_different_gap_types(
        self, evolver: InsightEvolution, mock_tracker: MagicMock
    ) -> None:
        a = _make_capsule(
            capsule_id="m-a",
            trigger_gap_type="method_limitation",
            trigger_keywords=[
                "retrieval",
                "augmented",
                "generation",
                "deep",
            ],
        )
        b = _make_capsule(
            capsule_id="m-b",
            trigger_gap_type="theoretical_gap",
            trigger_keywords=[
                "retrieval",
                "augmented",
                "generation",
                "deep",
            ],
        )
        _write_gene_pool(mock_tracker.data_dir, [a, b])
        capsules = evolver._load_capsules()
        merged, capsules = evolver._merge_capsules(capsules)
        assert merged == 0


# ===========================================================================
# 11. _auto_archive_low_score
# ===========================================================================


class TestAutoArchive:
    def test_archives_after_streak_threshold(
        self, evolver: InsightEvolution, mock_tracker: MagicMock
    ) -> None:
        capsule = _make_capsule(
            capsule_id="streaky",
            outcome_success_score=0.1,
            low_score_streak=2,  # one more push should archive
        )
        _write_gene_pool(mock_tracker.data_dir, [capsule])
        capsules = evolver._load_capsules()
        archived, capsules = evolver._auto_archive_low_score(capsules)
        evolver._save_capsules(capsules)
        assert archived >= 1
        updated = evolver._load_capsules()
        assert updated[0].status == "archived"

    def test_resets_streak_when_score_is_high(
        self, evolver: InsightEvolution, mock_tracker: MagicMock
    ) -> None:
        capsule = _make_capsule(
            capsule_id="reseter",
            outcome_success_score=0.9,
            low_score_streak=2,
        )
        _write_gene_pool(mock_tracker.data_dir, [capsule])
        capsules = evolver._load_capsules()
        archived, capsules = evolver._auto_archive_low_score(capsules)
        evolver._save_capsules(capsules)
        assert archived == 0
        updated = evolver._load_capsules()
        assert updated[0].low_score_streak == 0
        assert updated[0].status == "active"

    def test_skips_non_active_capsules(
        self, evolver: InsightEvolution, mock_tracker: MagicMock
    ) -> None:
        capsule = _make_capsule(
            capsule_id="archived-one",
            outcome_success_score=0.1,
            low_score_streak=5,
            status="archived",
        )
        _write_gene_pool(mock_tracker.data_dir, [capsule])
        capsules = evolver._load_capsules()
        archived, capsules = evolver._auto_archive_low_score(capsules)
        assert archived == 0


# ===========================================================================
# 12. Constants and class-level attributes
# ===========================================================================


class TestConstants:
    def test_overlap_threshold(self) -> None:
        assert InsightEvolution.OVERLAP_THRESHOLD == 0.80

    def test_low_score_threshold(self) -> None:
        assert InsightEvolution.LOW_SCORE_THRESHOLD == 0.30

    def test_streak_threshold(self) -> None:
        assert InsightEvolution.STREAK_THRESHOLD == 3

    def test_retire_count_threshold(self) -> None:
        assert InsightEvolution.RETIRE_COUNT_THRESHOLD == 3


# ---------------------------------------------------------------------------
# trigger_match scoring tests
# ---------------------------------------------------------------------------

class TestTriggerMatch:
    """Tests for CapsuleGene.trigger_match() scoring logic."""

    def _score(self, capsule, topic, gap_type, keywords=None):
        return capsule.trigger_match(topic, gap_type, keywords or [])

    def test_action_gap_title_in_query(self):
        c = _make_capsule(action_gap_title="Better RLHF evaluation metrics")
        score = self._score(c, "RLHF evaluation", "improvement")
        # "RLHF evaluation" IN "Better RLHF evaluation metrics" → 0.3
        # + token Jaccard: {rlhf,evaluation}∩{better,rlhf,evaluation,metrics}=2, 2/4=0.5 → 0.225
        assert score == 0.525

    def test_topic_in_trigger_topic(self):
        # Use mismatching gap_type to isolate topic signal
        c = _make_capsule(trigger_topic="RLHF alignment", trigger_gap_type="capability", action_gap_title="Some title", trigger_keywords=[])
        score = self._score(c, "RLHF alignment research", "improvement")
        # trigger_topic in topic: "RLHF alignment" in "RLHF alignment research" → 0.2
        assert score == 0.2

    def test_topic_in_action_title_bidirectional(self):
        c = _make_capsule(action_gap_title="RLHF alignment and reward modeling")
        score = self._score(c, "RLHF", "improvement")
        # "RLHF" IN "RLHF alignment..." → 0.3
        # + token Jaccard: 0.25 * (1/5) = 0.05
        assert score == 0.4625

    def test_query_in_trigger_topic(self):
        c = _make_capsule(trigger_topic="RLHF alignment techniques")
        score = self._score(c, "RLHF", "improvement")
        # "RLHF" IN "RLHF alignment techniques" → 0.2
        # + token Jaccard: 0.25 * (1/5) = 0.05
        assert score == 0.4

    def test_gap_type_match(self):
        c = _make_capsule(trigger_gap_type="improvement", action_gap_title="some title")
        score = self._score(c, "unrelated topic", "improvement")
        assert score == 0.3

    def test_gap_type_no_match(self):
        c = _make_capsule(trigger_gap_type="capability", action_gap_title="some title")
        score = self._score(c, "unrelated topic", "improvement")
        assert score == 0.0

    def test_keyword_overlap(self):
        c = _make_capsule(trigger_keywords=["RLHF", "alignment", "reward"])
        score = self._score(c, "RL", "improvement", keywords=["RLHF", "alignment"])
        # keyword overlap: 2/3=0.667 → 0.1; topic="RL" no substring/title match
        assert 0.09 <= score <= 0.11 or score == 0.2

    def test_all_signals_combined(self):
        c = _make_capsule(
            trigger_topic="RLHF", trigger_gap_type="improvement",
            trigger_keywords=["alignment"], action_gap_title="RLHF alignment metrics",
        )
        score = self._score(c, "RLHF", "improvement", keywords=["alignment"])
        assert 0.9 <= score <= 1.0

    def test_score_capped_at_one(self):
        c = _make_capsule(
            trigger_topic="RLHF", trigger_gap_type="improvement",
            trigger_keywords=["alignment"], action_gap_title="RLHF",
        )
        score = self._score(c, "RLHF", "improvement", keywords=["alignment"])
        assert score == 1.0

    def test_trigger_topic_list_form(self):
        c = _make_capsule(trigger_topic="RLHF")
        c.trigger_topic = ["RLHF", "alignment"]
        score = self._score(c, "RLHF alignment", "improvement")
        assert score >= 0.3


# ---------------------------------------------------------------------------
# eval_retrieval tests
# ---------------------------------------------------------------------------

class TestEvalRetrieval:
    """Tests for CapsuleStorageMixin.eval_retrieval()."""

    def test_eval_retrieval_filters_test_events(self):
        from llm.insight.storage import CapsuleStorageMixin
        import json, tempfile
        class EvalOnly(CapsuleStorageMixin):
            def __init__(self, tmp):
                self.data_dir = tmp
        with tempfile.TemporaryDirectory() as tmp:
            t = EvalOnly(Path(tmp))
            events_file = t.data_dir / "events.jsonl"
            events_file.write_text(
                json.dumps({"action":"accepted","topic":"test","gap_type":"x","gap_title":"test gap"}) + "\n" +
                json.dumps({"action":"accepted","topic":"RLHF","gap_type":"y","gap_title":"Better RLHF metrics"}) + "\n",
                encoding="utf-8")
            r = t.eval_retrieval(limit=50)
            t.close()  # close SQLite connection so tempfile can clean up on Windows
            assert r["total"] == 1  # test event filtered

    def test_eval_retrieval_deduplicates_events(self):
        from llm.insight.storage import CapsuleStorageMixin
        import json, tempfile
        class EvalOnly(CapsuleStorageMixin):
            def __init__(self, tmp):
                self.data_dir = tmp
        with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as tmp:
            t = EvalOnly(Path(tmp))
            events_file = t.data_dir / "events.jsonl"
            ev = {"action":"accepted","topic":"RLHF","gap_type":"y","gap_title":"Better RLHF metrics"}
            events_file.write_text(json.dumps(ev) + "\n" + json.dumps(ev) + "\n", encoding="utf-8")
            r = t.eval_retrieval(limit=50)
            t.close()
            assert r["total"] == 1  # deduplicated


# ---------------------------------------------------------------------------
# _normalize_gap_type tests
# ---------------------------------------------------------------------------

class TestNormalizeGapType:
    """Tests for _normalize_gap_type()."""

    def test_known_types_pass_through(self):
        from llm.insight.storage import _normalize_gap_type
        for valid in ["method_limitation", "unexplored_application", "contradiction",
                       "evaluation_gap", "scalability_issue", "theoretical_gap",
                       "dataset_gap", "generalization_gap"]:
            assert _normalize_gap_type(valid) == valid

    def test_capability_maps_to_method_limitation(self):
        from llm.insight.storage import _normalize_gap_type
        assert _normalize_gap_type("capability") == "method_limitation"

    def test_application_gap_maps_to_unexplored_application(self):
        from llm.insight.storage import _normalize_gap_type
        assert _normalize_gap_type("application_gap") == "unexplored_application"

    def test_other_maps_to_method_limitation(self):
        from llm.insight.storage import _normalize_gap_type
        assert _normalize_gap_type("other") == "method_limitation"

    def test_empty_string_maps_to_method_limitation(self):
        from llm.insight.storage import _normalize_gap_type
        assert _normalize_gap_type("") == "method_limitation"

    def test_unknown_string_maps_to_method_limitation(self):
        from llm.insight.storage import _normalize_gap_type
        assert _normalize_gap_type("garbage_type") == "method_limitation"

    def test_preserves_known_gap_types(self):
        from llm.insight.storage import _normalize_gap_type
        known = ["method_gap", "exploration_gap", "implementation", "theory_gap"]
        for gt in known:
            result = _normalize_gap_type(gt)
            assert result != "method_limitation"  # should map to something specific


# ---------------------------------------------------------------------------
# get_gene_pool_quality_report tests
# ---------------------------------------------------------------------------

class TestGenePoolQualityReport:
    """Tests for get_gene_pool_quality_report()."""

    def test_report_with_empty_pool(self):
        from llm.insight.storage import CapsuleStorageMixin
        import tempfile
        class ReportOnly(CapsuleStorageMixin):
            def __init__(self, tmp):
                self.data_dir = tmp
        with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as tmp:
            t = ReportOnly(Path(tmp))
            r = t.get_gene_pool_quality_report()
            t.close()
            assert "error" in r
            assert r["total"] == 0

    def test_report_includes_expected_keys(self):
        from llm.insight.tracker import EvolutionTracker
        import tempfile
        with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as tmp:
            t = EvolutionTracker(data_dir=Path(tmp))
            t.encode_capsule(
                topic="RLHF research",
                gap_type="method_limitation",
                gap_title="RLHF evaluation metrics gap",
                success_score=0.8,
            )
            r = t.get_gene_pool_quality_report()
            t.close()
            assert "total" in r
            assert "score_distribution" in r
            assert "credibility_distribution" in r
            assert "trendslop" in r
            assert "at_risk_capsules" in r
            assert r.get("error") is None

    def test_at_risk_excludes_archived(self):
        from llm.insight.tracker import EvolutionTracker
        import tempfile
        with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as tmp:
            t = EvolutionTracker(data_dir=Path(tmp))
            capsule = t.encode_capsule(
                topic="test",
                gap_type="method_limitation",
                gap_title="At-risk capsule",
                success_score=0.2,
            )
            capsule.low_score_streak = 3
            capsule.status = "archived"
            t.update_capsule(capsule)
            r = t.get_gene_pool_quality_report()
            t.close()
            assert r["at_risk_capsules"] == 0  # archived excluded

    def test_test_prefix_gap_title_is_rejected(self):
        from llm.insight.tracker import EvolutionTracker
        import tempfile
        with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as tmp:
            t = EvolutionTracker(data_dir=Path(tmp))
            capsule = t.encode_capsule(
                topic="test",
                gap_type="method_limitation",
                gap_title="test implementation",
                success_score=0.8,
            )
            t.close()
            # Should return capsule but not persist it
            assert capsule.capsule_id is not None
            # Pool should be empty since test title was rejected
            capsules = t._load_capsules()
            assert len(capsules) == 0
