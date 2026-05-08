"""Tests for rankers/base.py — Ranker abstract base class."""

import pytest
from rankers.base import Ranker, RankedResult


class TestRankedResult:
    def test_is_tuple(self):
        result: RankedResult = ("paper_id_123", 0.95)
        assert result[0] == "paper_id_123"
        assert result[1] == 0.95


class TestRanker:
    def test_rank_is_abstract(self):
        with pytest.raises(TypeError):
            Ranker()

    def test_concrete_ranker(self):
        class ConcreteRanker(Ranker):
            def rank(self, paper_id: str, threshold: float = 0.0, limit: int = 20):
                return [(f"{paper_id}_related", 0.9)]

        r = ConcreteRanker()
        results = r.rank("paper_1")
        assert len(results) == 1
        assert results[0][0] == "paper_1_related"
        assert results[0][1] == 0.9

    def test_rank_default_threshold(self):
        class ConcreteRanker(Ranker):
            def rank(self, paper_id: str, threshold: float = 0.0, limit: int = 20):
                return [(f"{paper_id}_a", 0.5), (f"{paper_id}_b", 0.1)]

        r = ConcreteRanker()
        results = r.rank("paper_x")
        assert len(results) == 2

    def test_ranked_result_type_annotation(self):
        result: RankedResult = ("id", 0.5)
        assert isinstance(result, tuple)
