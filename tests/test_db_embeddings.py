"""Tests for DB embedding methods and dedup-semantic CLI."""

from __future__ import annotations

import pytest

from db import Database


# ─────────────────────────────────────────────────────────────────────────────
# Fixtures
# ─────────────────────────────────────────────────────────────────────────────


@pytest.fixture
def db(tmp_path):
    d = Database(tmp_path / "research.db")
    d.init()
    return d


# ─────────────────────────────────────────────────────────────────────────────
# Test: set_embedding + get_embedding roundtrip
# ─────────────────────────────────────────────────────────────────────────────


class TestEmbeddingRoundtrip:
    def test_set_and_get_embedding(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="Attention Is All You Need")
        vector = [0.1] * 768
        ok = db.set_embedding("2301.00001", vector)
        assert ok is True

        retrieved = db.get_embedding("2301.00001")
        assert retrieved is not None
        assert len(retrieved) == 768
        assert abs(retrieved[0] - 0.1) < 1e-6

    def test_get_embedding_missing_paper(self, db):
        assert db.get_embedding("does-not-exist") is None

    def test_get_embedding_no_vector_set(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="No Embedding")
        assert db.get_embedding("2301.00001") is None

    def test_set_embedding_updates_existing(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="Test")
        v1 = [0.1] * 768
        v2 = [0.9] * 768
        db.set_embedding("2301.00001", v1)
        db.set_embedding("2301.00001", v2)
        retrieved = db.get_embedding("2301.00001")
        assert abs(retrieved[0] - 0.9) < 1e-6

    def test_set_embedding_missing_paper_returns_false(self, db):
        ok = db.set_embedding("nonexistent", [0.1] * 768)
        assert ok is False


# ─────────────────────────────────────────────────────────────────────────────
# Test: find_similar
# ─────────────────────────────────────────────────────────────────────────────


class TestFindSimilar:
    def test_find_similar_same_paper_excluded(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="Paper A")
        db.upsert_paper("2301.00002", "arxiv", title="Paper B")
        # Both have same embedding → similarity = 1.0
        vector = [0.5] * 768
        db.set_embedding("2301.00001", vector)
        db.set_embedding("2301.00002", vector)

        results = db.find_similar("2301.00001", threshold=0.85, limit=20)
        assert len(results) == 1
        assert results[0][0] == "2301.00002"

    def test_find_similar_below_threshold(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="Paper A")
        db.upsert_paper("2301.00002", "arxiv", title="Paper B")
        # Orthogonal vectors → similarity ≈ 0
        v1 = [1.0] + [0.0] * 767
        v2 = [0.0] * 768
        v2[10] = 1.0
        db.set_embedding("2301.00001", v1)
        db.set_embedding("2301.00002", v2)

        results = db.find_similar("2301.00001", threshold=0.85, limit=20)
        assert len(results) == 0

    def test_find_similar_respects_limit(self, db):
        for i in range(5):
            db.upsert_paper(f"2301.{i:05d}", "arxiv", title=f"Paper {i}")
            db.set_embedding(f"2301.{i:05d}", [0.5] * 768)

        results = db.find_similar("2301.00000", threshold=0.0, limit=2)
        assert len(results) == 2

    def test_find_similar_no_embedding_for_query(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="Paper A")
        db.upsert_paper("2301.00002", "arxiv", title="Paper B")
        db.set_embedding("2301.00002", [0.5] * 768)

        results = db.find_similar("2301.00001", threshold=0.85, limit=20)
        assert results == []

    def test_find_similar_result_sorted_by_score(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="Query")
        db.upsert_paper("2301.00002", "arxiv", title="Med")
        db.upsert_paper("2301.00003", "arxiv", title="Close")
        db.upsert_paper("2301.00004", "arxiv", title="Closer")
        # Query embedding
        q = [1.0] * 768
        # Different similarity levels
        db.set_embedding("2301.00001", q)
        db.set_embedding("2301.00002", [0.3 * x for x in q])  # ~0.3
        db.set_embedding("2301.00003", [0.6 * x for x in q])  # ~0.6
        db.set_embedding("2301.00004", [0.9 * x for x in q])  # ~0.9

        results = db.find_similar("2301.00001", threshold=0.0, limit=10)
        scores = [r[1] for r in results]
        assert scores == sorted(scores, reverse=True)


# ─────────────────────────────────────────────────────────────────────────────
# Test: get_similarity
# ─────────────────────────────────────────────────────────────────────────────


class TestGetSimilarity:
    def test_get_similarity_identical_vectors(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="A")
        db.upsert_paper("2301.00002", "arxiv", title="B")
        v = [0.5] * 768
        db.set_embedding("2301.00001", v)
        db.set_embedding("2301.00002", v)
        sim = db.get_similarity("2301.00001", "2301.00002")
        assert sim is not None
        assert abs(sim - 1.0) < 1e-4

    def test_get_similarity_orthogonal_vectors(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="A")
        db.upsert_paper("2301.00002", "arxiv", title="B")
        v1 = [1.0] + [0.0] * 767
        v2 = [0.0] * 768
        v2[0] = 0.0
        v2[1] = 1.0
        db.set_embedding("2301.00001", v1)
        db.set_embedding("2301.00002", v2)
        sim = db.get_similarity("2301.00001", "2301.00002")
        assert abs(sim) < 1e-4

    def test_get_similarity_one_missing(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="A")
        db.upsert_paper("2301.00002", "arxiv", title="B")
        db.set_embedding("2301.00001", [0.5] * 768)
        assert db.get_similarity("2301.00001", "2301.00002") is None

    def test_get_similarity_both_missing(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="A")
        db.upsert_paper("2301.00002", "arxiv", title="B")
        assert db.get_similarity("2301.00001", "2301.00002") is None


# ─────────────────────────────────────────────────────────────────────────────
# Test: get_embedding_stats
# ─────────────────────────────────────────────────────────────────────────────


class TestEmbeddingStats:
    def test_stats_all_embedded(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="A")
        db.upsert_paper("2301.00002", "arxiv", title="B")
        db.set_embedding("2301.00001", [0.1] * 768)
        db.set_embedding("2301.00002", [0.2] * 768)
        s = db.get_embedding_stats()
        assert s["with_embedding"] == 2
        assert s["total_with_text"] == 2

    def test_stats_partial(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="A")
        db.upsert_paper("2301.00002", "arxiv", title="B")  # no embedding
        db.set_embedding("2301.00001", [0.1] * 768)
        s = db.get_embedding_stats()
        assert s["with_embedding"] == 1
        assert s["total_with_text"] == 2

    def test_stats_empty_db(self, db):
        s = db.get_embedding_stats()
        assert s["with_embedding"] == 0
        assert s["total_with_text"] == 0


# ─────────────────────────────────────────────────────────────────────────────
# Test: get_papers_without_embeddings
# ─────────────────────────────────────────────────────────────────────────────


class TestPapersWithoutEmbeddings:
    def test_returns_papers_without_vector(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="No Embed")
        db.upsert_paper("2301.00002", "arxiv", title="Has Embed")
        db.set_embedding("2301.00002", [0.1] * 768)
        papers = db.get_papers_without_embeddings(limit=100)
        assert len(papers) == 1
        assert papers[0].id == "2301.00001"

    def test_respects_limit(self, db):
        for i in range(5):
            db.upsert_paper(f"2301.{i:05d}", "arxiv", title=f"Paper {i}")
        papers = db.get_papers_without_embeddings(limit=2)
        assert len(papers) == 2

    def test_excludes_empty_title(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="")
        papers = db.get_papers_without_embeddings(limit=10)
        assert len(papers) == 0
