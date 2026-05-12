"""Tests for core/search_optimizer.py — SearchOptimizer."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

from core.search_optimizer import SearchOptimizer


class TestSearchOptimizer:
    def test_optimize_query_strips_and_lowercases(self):
        opt = SearchOptimizer()
        result = opt.optimize_query("  Machine Learning  ")
        assert result == "machine learning"

    def test_optimize_query_records_history(self):
        opt = SearchOptimizer()
        opt.optimize_query("test query")
        assert "test query" in opt.search_history

    def test_optimize_query_bounded_history(self):
        opt = SearchOptimizer()
        opt.MAX_HISTORY = 5
        for i in range(10):
            opt.optimize_query(f"query-{i}")
        assert len(opt.search_history) == 5
        assert opt.search_history[-1] == "query-9"
        assert "query-0" not in opt.search_history

    def test_expand_query_no_match(self):
        opt = SearchOptimizer()
        result = opt.expand_query("unknownword")
        assert result == ["unknownword"]

    def test_expand_query_with_synonyms(self):
        opt = SearchOptimizer()
        result = opt.expand_query("ml")
        assert "ml" in result
        assert "machine learning" in result

    def test_expand_query_nlp(self):
        opt = SearchOptimizer()
        result = opt.expand_query("nlp")
        assert "nlp" in result
        assert "natural language processing" in result

    def test_expand_query_cv(self):
        opt = SearchOptimizer()
        result = opt.expand_query("cv")
        assert "cv" in result
        assert "computer vision" in result

    def test_expand_query_multiple_words(self):
        opt = SearchOptimizer()
        result = opt.expand_query("ml ai")
        assert len(result) >= 3  # ml expansions + ai expansions

    def test_rank_results_title_match_scores_higher(self):
        opt = SearchOptimizer()
        results = [
            {"title": "machine learning paper", "abstract": "something else"},
            {"title": "unrelated title", "abstract": "machine learning in abstract"},
        ]
        ranked = opt.rank_results(results, "machine learning")
        assert ranked[0]["title"] == "machine learning paper"
        assert ranked[1]["title"] == "unrelated title"

    def test_rank_results_empty(self):
        opt = SearchOptimizer()
        ranked = opt.rank_results([], "test")
        assert ranked == []

    def test_rank_results_no_match(self):
        opt = SearchOptimizer()
        results = [{"title": "foo", "abstract": "bar"}]
        ranked = opt.rank_results(results, "xyz")
        # All get score 0, sort is stable so original order
        assert ranked[0]["title"] == "foo"

    def test_rank_results_missing_fields(self):
        opt = SearchOptimizer()
        results = [{}, {"title": "test"}]
        ranked = opt.rank_results(results, "test")
        assert ranked[0]["title"] == "test"

    def test_get_suggestions_empty(self):
        opt = SearchOptimizer()
        suggestions = opt.get_suggestions("test")
        assert suggestions == []

    def test_get_suggestions_from_history(self):
        opt = SearchOptimizer()
        opt.search_history = [
            "machine learning basics",
            "machine learning advanced",
            "deep learning",
        ]
        suggestions = opt.get_suggestions("machine")
        assert len(suggestions) <= 5
        assert "machine learning basics" in suggestions

    def test_get_suggestions_limit(self):
        opt = SearchOptimizer()
        opt.search_history = ["test", "test a", "test b", "test c", "test d", "test e", "test f"]
        suggestions = opt.get_suggestions("test")
        assert len(suggestions) == 5

    def test_get_suggestions_case_insensitive(self):
        opt = SearchOptimizer()
        opt.search_history = ["Machine Learning"]
        suggestions = opt.get_suggestions("machine")
        assert len(suggestions) == 1
