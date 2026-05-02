"""Tests for kg/integration.py rebuild — full and incremental modes."""
from __future__ import annotations

import json
import pytest

from kg.manager import KGManager
from kg.integration import KGIntegration


@pytest.fixture
def kg_integ(tmp_path):
    """Fresh KGManager + KGIntegration backed by a temporary DB."""
    db = tmp_path / "test_kg.db"
    kg = KGManager(db_path=str(db))
    return kg, KGIntegration(kg), tmp_path


def _write_papers_json(tmp_path, papers, citation_graph=None):
    """Helper: write a papers.json file and return its path."""
    path = tmp_path / "papers.json"
    path.write_text(json.dumps({
        "papers": papers,
        "citation_graph": citation_graph or {},
    }), encoding="utf-8")
    return path


class TestRebuildFull:
    def test_rebuild_processes_all_papers(self, kg_integ):
        kg, integ, tmp_path = kg_integ
        papers = {
            "p1": {"title": "Paper One", "tags": ["LLM"], "authors": [], "year": 2024},
            "p2": {"title": "Paper Two", "tags": ["RL"], "authors": [], "year": 2023},
        }
        path = _write_papers_json(tmp_path, papers)

        integ.rebuild_from_papers_json(path)

        # Both papers indexed
        assert kg.get_node_by_entity("Paper", "p1") is not None
        assert kg.get_node_by_entity("Paper", "p2") is not None
        assert kg.get_node_by_entity("Tag", "LLM") is not None
        assert kg.get_node_by_entity("Tag", "RL") is not None

    def test_rebuild_stores_rebuild_meta(self, kg_integ):
        kg, integ, tmp_path = kg_integ
        papers = {"p1": {"title": "P1", "tags": [], "authors": [], "year": 2024}}
        path = _write_papers_json(tmp_path, papers)

        integ.rebuild_from_papers_json(path)

        meta = kg.get_rebuild_meta()
        assert meta["last_rebuild_at"] is not None
        assert meta["indexed_paper_uids"] == {"p1"}

    def test_rebuild_idempotent_on_second_run(self, kg_integ):
        kg, integ, tmp_path = kg_integ
        papers = {"p1": {"title": "P1", "tags": [], "authors": [], "year": 2024}}
        path = _write_papers_json(tmp_path, papers)

        integ.rebuild_from_papers_json(path)
        stats1 = kg.stats()["total_nodes"]
        integ.rebuild_from_papers_json(path)  # second full run
        stats2 = kg.stats()["total_nodes"]

        # Should be same — no duplicate nodes
        assert stats1 == stats2


class TestRebuildIncremental:
    def test_incremental_skips_already_indexed(self, kg_integ):
        kg, integ, tmp_path = kg_integ
        papers = {"p1": {"title": "P1", "tags": [], "authors": [], "year": 2024}}
        path = _write_papers_json(tmp_path, papers)

        # First: full rebuild
        integ.rebuild_from_papers_json(path)

        # Second: incremental — p1 already indexed, nothing new
        integ.rebuild_from_papers_json(path, incremental=True)

        # p1 still only appears once
        assert len(kg.get_all_nodes(node_type="Paper")) == 1

    def test_incremental_only_processes_new_papers(self, kg_integ):
        kg, integ, tmp_path = kg_integ
        papers_v1 = {"p1": {"title": "P1", "tags": [], "authors": [], "year": 2024}}
        path = _write_papers_json(tmp_path, papers_v1)

        # First rebuild
        integ.rebuild_from_papers_json(path)

        # Add new paper
        papers_v2 = {
            "p1": {"title": "P1", "tags": [], "authors": [], "year": 2024},
            "p2": {"title": "P2", "tags": ["LLM"], "authors": [], "year": 2023},
        }
        path = _write_papers_json(tmp_path, papers_v2)

        integ.rebuild_from_papers_json(path, incremental=True)

        assert kg.get_node_by_entity("Paper", "p1") is not None
        assert kg.get_node_by_entity("Paper", "p2") is not None
        assert kg.get_node_by_entity("Tag", "LLM") is not None

    def test_incremental_citations_only_for_new_papers(self, kg_integ):
        kg, integ, tmp_path = kg_integ
        papers_v1 = {
            "p1": {"title": "P1", "tags": [], "authors": [], "year": 2024},
            "p2": {"title": "P2", "tags": [], "authors": [], "year": 2023},
        }
        citations_v1 = {
            "p1": {"cited": [], "citing": ["p2"]},
            "p2": {"cited": [], "citing": []},
        }
        path = _write_papers_json(tmp_path, papers_v1, citations_v1)
        integ.rebuild_from_papers_json(path)

        papers_v2 = {
            "p1": {"title": "P1", "tags": [], "authors": [], "year": 2024},
            "p2": {"title": "P2", "tags": [], "authors": [], "year": 2023},
            "p3": {"title": "P3", "tags": [], "authors": [], "year": 2022},
        }
        citations_v2 = {
            "p1": {"cited": [], "citing": ["p2"]},
            "p2": {"cited": ["p1"], "citing": []},
            "p3": {"cited": [], "citing": []},
        }
        path = _write_papers_json(tmp_path, papers_v2, citations_v2)

        # Incremental — only p3 is new
        integ.rebuild_from_papers_json(path, incremental=True)

        assert kg.get_node_by_entity("Paper", "p3") is not None

    def test_incremental_fallback_to_full_when_no_prior_meta(self, kg_integ):
        kg, integ, tmp_path = kg_integ
        papers = {"p1": {"title": "P1", "tags": [], "authors": [], "year": 2024}}
        path = _write_papers_json(tmp_path, papers)

        # Incremental with no prior rebuild — should process all (same as full)
        integ.rebuild_from_papers_json(path, incremental=True)

        assert kg.get_node_by_entity("Paper", "p1") is not None
        meta = kg.get_rebuild_meta()
        assert "p1" in meta["indexed_paper_uids"]

    def test_incremental_meta_updated_after_run(self, kg_integ):
        kg, integ, tmp_path = kg_integ
        papers_v1 = {"p1": {"title": "P1", "tags": [], "authors": [], "year": 2024}}
        path = _write_papers_json(tmp_path, papers_v1)
        integ.rebuild_from_papers_json(path)

        papers_v2 = {
            "p1": {"title": "P1", "tags": [], "authors": [], "year": 2024},
            "p2": {"title": "P2", "tags": [], "authors": [], "year": 2023},
        }
        path = _write_papers_json(tmp_path, papers_v2)
        integ.rebuild_from_papers_json(path, incremental=True)

        meta = kg.get_rebuild_meta()
        assert meta["indexed_paper_uids"] == {"p1", "p2"}
