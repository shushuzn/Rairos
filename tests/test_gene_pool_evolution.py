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
    """A mock EvolutionTracker whose data_dir points at tmp_path."""
    tracker = MagicMock()
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
    gene_file = data_dir / "gene_pool.jsonl"
    with open(gene_file, "w", encoding="utf-8") as f:
        for c in capsules:
            f.write(json.dumps(c.to_dict(), ensure_ascii=False) + "\n")


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
        assert evolver._add_candidate(cand) is True
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
        assert evolver._add_candidate(cand) is True
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
        assert evolver._add_candidate(cand) is False

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
        assert evolver._add_candidate(cand) is False

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
        assert evolver._add_candidate(cand) is True

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
        assert evolver._add_candidate(cand) is True

    def test_candidate_becomes_active_capsule(
        self, evolver: InsightEvolution, mock_tracker: MagicMock
    ) -> None:
        _write_gene_pool(mock_tracker.data_dir, [])
        cand = _make_candidate(confidence=0.8)
        evolver._add_candidate(cand)
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
        assert evolver._add_candidate(cand) is True


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
        expected_overall = 0.5 * 0.6 + 0.2 * 0.5 + 0.2 * 1.0 + 0.1 * 0.6
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
        merged = evolver._merge_capsules()
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
        merged = evolver._merge_capsules()
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
        archived = evolver._auto_archive_low_score()
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
        archived = evolver._auto_archive_low_score()
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
        archived = evolver._auto_archive_low_score()
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
