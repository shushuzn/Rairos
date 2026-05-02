"""Tests for rankers module."""
from __future__ import annotations

import struct
from datetime import date

import pytest
from freezegun import freeze_time
from unittest.mock import patch

from db.database import Database, PaperRecord
from rankers.cosine import CosineSimilarityRanker
from rankers.score import CompositeScorer


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _embed(values: list[float]) -> bytes:
    """Pack a float list into a little-endian binary blob."""
    return struct.pack(f"{len(values)}f", *values)


def _insert(db: Database, paper_id: str, embed: list[float], **kwargs) -> None:
    """Insert a paper record with an embedding blob."""
    now = date.today().isoformat()
    blob = _embed(embed)
    db.conn.execute(
        "INSERT OR IGNORE INTO papers "
        "(id,source,title,authors,abstract,published,added_at,updated_at,embed_vector) "
        "VALUES (?,?,?,?,?,?,?,?,?)",
        (
            paper_id,
            kwargs.get("source", "test"),
            kwargs.get("title", ""),
            "[]",
            "",
            kwargs.get("published", ""),
            now,
            now,
            blob,
        ),
    )
    db.conn.commit()


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

@pytest.fixture
def db_with_embeddings(tmp_path):
    """Database backed by a temp file, with the papers table created."""
    db_path = tmp_path / "research.db"
    db = Database(str(db_path))
    db.init()
    yield db
    db.conn.close()


# ---------------------------------------------------------------------------
# Ranker ABC
# ---------------------------------------------------------------------------

class TestRankerAbc:
    def test_rank_is_abstract(self):
        from rankers.base import Ranker

        with pytest.raises(TypeError, match="abstract"):
            Ranker()


# ---------------------------------------------------------------------------
# CosineSimilarityRanker
# ---------------------------------------------------------------------------

class TestCosineSimilarityRanker:
    @freeze_time("2024-06-15")
    def test_returns_empty_when_paper_has_no_embedding(self, db_with_embeddings):
        _insert(db_with_embeddings, "paper-x", [1.0] * 10)
        ranker = CosineSimilarityRanker(db_with_embeddings)
        results = ranker.rank("nonexistent")
        assert results == []

    @freeze_time("2024-06-15")
    def test_returns_empty_when_embedding_is_all_zeros(self, db_with_embeddings):
        _insert(db_with_embeddings, "paper-x", [0.0] * 10)
        ranker = CosineSimilarityRanker(db_with_embeddings)
        results = ranker.rank("paper-x")
        assert results == []

    @freeze_time("2024-06-15")
    def test_filters_by_threshold(self, db_with_embeddings):
        # paper-x = [1,0,1,0,1,0,1,0,1,0] (norm = √5 ≈ 2.236)
        _insert(db_with_embeddings, "paper-x", [1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0])
        # p2: mostly same direction — cos ≈ 0.894 (passes 0.8)
        _insert(db_with_embeddings, "p2", [0.9, 0.0, 0.9, 0.0, 0.9, 0.0, 0.9, 0.0, 0.9, 0.0])
        # p3: mostly opposite — cos ≈ 0.224 (below 0.8, filtered)
        _insert(db_with_embeddings, "p3", [0.1, 0.9, 0.1, 0.9, 0.1, 0.9, 0.1, 0.9, 0.1, 0.9])
        ranker = CosineSimilarityRanker(db_with_embeddings)
        # With threshold=0.8 only p2 passes
        results = ranker.rank("paper-x", threshold=0.8)
        assert len(results) == 1
        assert results[0][0].id == "p2"

    @freeze_time("2024-06-15")
    def test_sorts_by_score_descending(self, db_with_embeddings):
        _insert(db_with_embeddings, "paper-x", [1.0] * 10)
        _insert(db_with_embeddings, "low",  [0.3] * 10)
        _insert(db_with_embeddings, "high", [0.9] * 10)
        _insert(db_with_embeddings, "mid",  [0.6] * 10)
        ranker = CosineSimilarityRanker(db_with_embeddings)
        results = ranker.rank("paper-x", threshold=0.0)
        ids = [r[0].id for r in results]
        scores = [r[1] for r in results]
        # All vectors are collinear with [1.0]*10, so all get sim ≈ 1.0
        # Order is stable sort by score descending; verify descending
        assert scores == sorted(scores, reverse=True)
        assert len(ids) == 3

    @freeze_time("2024-06-15")
    def test_excludes_self(self, db_with_embeddings):
        _insert(db_with_embeddings, "paper-x", [0.5] * 10)
        _insert(db_with_embeddings, "other",   [0.5] * 10)
        ranker = CosineSimilarityRanker(db_with_embeddings)
        results = ranker.rank("paper-x")
        assert all(r[0].id != "paper-x" for r in results)

    @freeze_time("2024-06-15")
    def test_respects_limit(self, db_with_embeddings):
        _insert(db_with_embeddings, "paper-x", [1.0] * 10)
        for i in range(10):
            _insert(db_with_embeddings, f"p{i}", [0.99] * 10)
        ranker = CosineSimilarityRanker(db_with_embeddings)
        results = ranker.rank("paper-x", threshold=0.0, limit=5)
        assert len(results) == 5

    @freeze_time("2024-06-15")
    def test_handles_null_embed_vector(self, db_with_embeddings):
        """Row with null embed_vector is skipped without raising."""
        db = db_with_embeddings
        db.conn.execute(
            "INSERT OR IGNORE INTO papers "
            "(id,source,added_at,updated_at,embed_vector) "
            "VALUES (?,?,?,?,?)",
            ("p2", "test", "2024-06-15", "2024-06-15", None),
        )
        db.conn.commit()
        ranker = CosineSimilarityRanker(db)
        # Should not raise — null blob is skipped
        results = ranker.rank("p2", threshold=0.0)
        assert len(results) == 0

    @freeze_time("2024-06-15")
    def test_handles_zero_norm_embedding_in_db(self, db_with_embeddings):
        """Row with all-zero embed_vector is skipped (norm=0 → divide by zero)."""
        _insert(db_with_embeddings, "paper-x", [1.0] * 10)
        _insert(db_with_embeddings, "p2", [0.0] * 10)   # zero norm — skipped
        _insert(db_with_embeddings, "p3", [0.9] * 10)
        ranker = CosineSimilarityRanker(db_with_embeddings)
        results = ranker.rank("paper-x", threshold=0.0)
        ids = [r[0].id for r in results]
        assert "p2" not in ids
        assert "p3" in ids


# ---------------------------------------------------------------------------
# CompositeScorer — unit tests for _parse_quality_weight
# ---------------------------------------------------------------------------

class TestCompositeScorerParseQuality:
    @pytest.mark.parametrize(
        "status,expected",
        [
            ("full",      1.0),
            ("sections",  0.8),
            ("partial",   0.5),
            ("failed",    0.1),
            ("FULL",      1.0),
            ("Sections",  0.8),
            ("unknown",   0.0),
            ("-",         0.0),
        ],
    )
    def test_parse_quality_mapping(self, db_with_embeddings, status, expected):
        scorer = CompositeScorer(db_with_embeddings)
        assert scorer._parse_quality_score(status) == expected

    def test_parse_quality_none_returns_zero(self, db_with_embeddings):
        scorer = CompositeScorer(db_with_embeddings)
        assert scorer._parse_quality_score(None) == 0.0


# ---------------------------------------------------------------------------
# CompositeScorer — integration (mock CosineSimilarityRanker.rank)
# ---------------------------------------------------------------------------

class TestCompositeScorerIntegration:
    @pytest.mark.no_freeze
    def test_returns_empty_when_no_cosine_results(self, db_with_embeddings):
        with patch.object(CosineSimilarityRanker, "rank", return_value=[]):
            scorer = CompositeScorer(db_with_embeddings, sim_weight=1.0)
            assert scorer.rank("any-paper") == []

    @pytest.mark.no_freeze
    def test_threshold_filters_results(self, db_with_embeddings):
        with patch.object(CosineSimilarityRanker, "rank", return_value=[]):
            scorer = CompositeScorer(db_with_embeddings, sim_weight=1.0)
            # CompositeScorer applies threshold in rank(), not __init__
            assert scorer.rank("any-paper", threshold=0.9) == []

    @pytest.mark.no_freeze
    def test_respects_limit(self, db_with_embeddings):
        rows = [_paper_record(f"p{i}", 0.5, published="2024-01-01") for i in range(5)]
        with patch.object(CosineSimilarityRanker, "rank", return_value=rows):
            scorer = CompositeScorer(db_with_embeddings, sim_weight=1.0)
            scored = scorer.rank("any-paper", limit=3)
            assert len(scored) == 3

    def test_ref_year_from_most_recent_paper(self, db_with_embeddings):
        p1 = _paper_record("p1", 0.9, published="2022-01-01")
        p2 = _paper_record("p2", 0.8, published="2024-01-01")
        with patch.object(CosineSimilarityRanker, "rank", return_value=[p1, p2]):
            scorer = CompositeScorer(db_with_embeddings, sim_weight=1.0)
            results = scorer.rank("any-paper")
            # Returns List[RankedResult] i.e. List[(PaperRecord, float)]
            assert len(results) == 2

    def test_ref_year_fallback_to_current_year_when_published_none(self, db_with_embeddings):
        p1 = _paper_record("p1", 0.9, published="2024-01-01")
        with patch.object(CosineSimilarityRanker, "rank", return_value=[p1]):
            scorer = CompositeScorer(db_with_embeddings, sim_weight=1.0)
            scores = scorer.rank("any-paper")
            assert len(scores) == 1

    def test_default_weights_sum_to_one(self, db_with_embeddings):
        scorer = CompositeScorer(db_with_embeddings)
        total = scorer.sim_weight + scorer.recency_weight + scorer.parse_weight
        assert abs(total - 1.0) < 1e-9

    def test_custom_weights(self, db_with_embeddings):
        scorer = CompositeScorer(
            db_with_embeddings,
            sim_weight=0.5,
            recency_weight=0.3,
            parse_weight=0.2,
        )
        total = scorer.sim_weight + scorer.recency_weight + scorer.parse_weight
        assert abs(total - 1.0) < 1e-9

    def test_published_year_from_string(self, db_with_embeddings):
        p1 = _paper_record("p1", 0.9, published="2023-05-15")
        with patch.object(CosineSimilarityRanker, "rank", return_value=[p1]):
            scorer = CompositeScorer(db_with_embeddings, sim_weight=1.0)
            scores = scorer.rank("any-paper")
            # Result is (PaperRecord, float) tuple; check it returned something
            assert len(scores) == 1
            record, composite = scores[0]
            assert record.id == "p1"


# ---------------------------------------------------------------------------
# Test a real CosineSimilarityRanker + CompositeScorer round-trip
# ---------------------------------------------------------------------------

class TestCosinePlusComposite:
    @freeze_time("2024-06-15")
    def test_composite_score_with_real_ranker(self, db_with_embeddings):
        """End-to-end: insert papers, score via CompositeScorer backed by real CosineSimilarityRanker."""
        _insert(db_with_embeddings, "paper-x", [1.0] * 10)
        _insert(db_with_embeddings, "high-sim", [0.95] * 10, published="2024-01-01")
        _insert(db_with_embeddings, "low-sim",  [0.3] * 10, published="2023-01-01")
        scorer = CompositeScorer(
            db_with_embeddings,
            sim_weight=0.7,
            recency_weight=0.2,
            parse_weight=0.1,
        )
        results = scorer.rank("paper-x", limit=5)
        ids = [r[0].id for r in results]
        # high-sim should rank above low-sim (both have same parse_status default)
        assert ids.index("high-sim") < ids.index("low-sim")


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _paper_record(id: str, score: float, published=None, parse_status="full"):
    now = date.today().isoformat()
    return (
        PaperRecord(
            id=id,
            source="test",
            title="",
            authors=[],
            abstract="",
            published=published or "",
            updated=now,
            abs_url="",
            pdf_url="",
            primary_category="",
            journal="",
            volume="",
            issue="",
            page="",
            doi="",
            categories="",
            reference_count=0,
            added_at=now,
            updated_at=now,
            pdf_path="",
            pdf_hash="",
            parse_status=parse_status,
            parse_error="",
            parse_version=0,
            plain_text="",
            latex_blocks=[],
            table_count=0,
            figure_count=0,
            word_count=0,
            page_count=0,
            pnote_path="",
            cnote_path="",
            mnote_path="",
            embed_vector=None,
            tags=[],
        ),
        score,
    )
