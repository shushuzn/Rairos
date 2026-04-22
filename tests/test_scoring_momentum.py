"""Tests for scoring/momentum.py — ResearchMomentum scoring."""
from __future__ import annotations

import json

import pytest

from kg.manager import KGManager
from scoring.momentum import ResearchMomentum


@pytest.fixture
def rm(tmp_path):
    db = tmp_path / "test_kg.db"
    kg = KGManager(db_path=str(db))
    rm_obj = ResearchMomentum(kg=kg)
    rm_obj._scores_path = tmp_path / "scores.json"
    return rm_obj


@pytest.fixture
def rm_populated(rm):
    """KG with two papers sharing a tag, one citing the other."""
    p1 = rm.kg.add_node("Paper", "p1", "Paper 1", year=2024)
    p2 = rm.kg.add_node("Paper", "p2", "Paper 2", year=2023)
    t1 = rm.kg.add_node("Tag", "LLM", "LLM")
    rm.kg.add_edge(p2, p1, "cite")
    rm.kg.add_edge(p1, t1, "same_tag")
    rm.kg.add_edge(p2, t1, "same_tag")
    return {"p1": p1, "p2": p2, "t1": t1, "kg": rm.kg}


@pytest.fixture
def radar_file(tmp_path, monkeypatch):
    """Create a data/ directory with radar.json for _load_radar tests."""
    data_dir = tmp_path / "data"
    data_dir.mkdir()
    radar = data_dir / "radar.json"
    radar.write_text(json.dumps({"LLM": {"score": 80}}), encoding="utf-8")
    monkeypatch.chdir(tmp_path)


class TestResearchMomentumScoring:
    def test_score_paper_returns_zero_for_unknown(self, rm):
        assert rm.score_paper("ghost") == 0.0

    def test_score_paper_is_cached(self, rm, rm_populated):
        score1 = rm.score_paper("p1")
        score2 = rm.score_paper("p1")
        assert score1 == score2
        assert "p1" in rm._scores

    def test_score_paper_unknown_node_returns_zero(self, rm):
        assert rm.score_paper("nonexistent") == 0.0

    def test_compute_score_returns_float_between_0_and_100(self, rm, rm_populated):
        node = rm.kg.get_node_by_entity("Paper", "p1")
        score = rm._compute_score(node)
        assert 0.0 <= score <= 100.0

    def test_compute_score_cited_paper_has_citation_component(self, rm, rm_populated):
        node = rm.kg.get_node_by_entity("Paper", "p1")
        score = rm._compute_score(node)
        assert score > 0.0

    def test_compute_score_uncited_paper(self, rm, rm_populated):
        node = rm.kg.get_node_by_entity("Paper", "p2")
        score = rm._compute_score(node)
        assert 0.0 <= score <= 100.0

    def test_raw_cite_score(self, rm, rm_populated):
        node = rm.kg.get_node_by_entity("Paper", "p1")
        score = rm._raw_cite_score(node)
        assert score == 1

    def test_raw_cite_score_none_node(self, rm):
        assert rm._raw_cite_score(None) == 0.0


class TestResearchMomentumTagScoring:
    def test_score_tag_unknown(self, rm):
        result = rm.score_tag("UnknownTag")
        assert result["raw_score"] == 0.0
        assert result["papers_count"] == 0
        assert result["momentum_label"] == "niche"

    def test_score_tag_with_papers(self, rm, rm_populated):
        result = rm.score_tag("LLM")
        assert result["papers_count"] == 2
        assert result["raw_score"] >= 0.0

    def test_score_tag_heat_trend_rising(self, rm_populated, radar_file):
        """Tag with radar score > 70 should be labeled 'rising'/'hot'."""
        import scoring.momentum as m
        orig_load = m.ResearchMomentum._load_radar
        def fake_load(self):
            return {"LLM": {"score": 80}}
        m.ResearchMomentum._load_radar = fake_load
        try:
            rm = ResearchMomentum(kg=rm_populated["kg"])
            result = rm.score_tag("LLM")
            assert result["heat_trend"] == "rising"
            assert result["momentum_label"] == "hot"
        finally:
            m.ResearchMomentum._load_radar = orig_load

    def test_score_tag_heat_trend_unknown_for_no_history(self, rm):
        result = rm.score_tag("GhostTag")
        assert result["heat_trend"] == "unknown"
        assert result["momentum_label"] == "niche"


class TestResearchMomentumLeaderboard:
    def test_get_top_papers(self, rm, rm_populated):
        top = rm.get_top_papers(top_n=5)
        assert isinstance(top, list)
        assert all(isinstance(x, tuple) for x in top)
        assert len(top) == 2

    def test_get_top_papers_filtered_by_tag(self, rm, rm_populated):
        top = rm.get_top_papers(tag="LLM", top_n=5)
        assert len(top) == 2

    def test_get_tag_leaderboard(self, rm, rm_populated):
        lb = rm.get_tag_leaderboard()
        assert isinstance(lb, list)
        assert len(lb) == 1


class TestResearchMomentumRefresh:
    def test_refresh_all_clears_and_recomputes(self, rm, rm_populated):
        rm._scores.clear()
        rm.refresh_all()
        assert "p1" in rm._scores
        assert "p2" in rm._scores

    def test_save_and_load_scores(self, rm, rm_populated):
        rm.save_scores()
        rm2 = ResearchMomentum(kg=rm.kg)
        rm2._scores_path = rm._scores_path
        assert "p1" in rm2._scores
