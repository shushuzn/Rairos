"""Tests for kg/queries.py — KGQueries high-level graph queries."""
from __future__ import annotations

import pytest

from kg.manager import KGManager
from kg.queries import KGQueries


@pytest.fixture
def kq(tmp_path):
    db = tmp_path / "test_kg.db"
    kg = KGManager(db_path=str(db))
    return KGQueries(kg)


@pytest.fixture
def kq_populated(kq):
    """KGQueries with a simple 3-hop graph: p1 → p2 → p3, plus a tag."""
    p1 = kq.kg.add_node("Paper", "p1", "Paper 1", year=2023)
    p2 = kq.kg.add_node("Paper", "p2", "Paper 2", year=2022)
    p3 = kq.kg.add_node("Paper", "p3", "Paper 3", year=2021)
    t1 = kq.kg.add_node("Tag", "LLM", "LLM")

    kq.kg.add_edge(p1, p2, "cite")
    kq.kg.add_edge(p2, p3, "cite")
    kq.kg.add_edge(p1, t1, "same_tag")
    kq.kg.add_edge(p2, t1, "same_tag")

    return {"p1": p1, "p2": p2, "p3": p3, "t1": t1}


class TestKGPaperSubgraph:
    def test_returns_empty_for_unknown_paper(self, kq):
        result = kq.get_paper_subgraph("ghost-paper")
        assert result["nodes"] == []
        assert result["edges"] == []
        assert result["center"] is None

    def test_subgraph_depth1(self, kq, kq_populated):
        result = kq.get_paper_subgraph("p1", depth=1)
        assert result["center"]["entity_id"] == "p1"
        node_ids = {n["entity_id"] for n in result["nodes"]}
        # depth=1: direct neighbors only (p2 and the tag)
        assert "p1" in node_ids

    def test_subgraph_depth2(self, kq, kq_populated):
        result = kq.get_paper_subgraph("p1", depth=2)
        node_ids = {n["entity_id"] for n in result["nodes"]}
        # depth=2: p1, p2 (direct), p3 (via p2)
        assert "p1" in node_ids
        assert "p2" in node_ids
        assert "p3" in node_ids

    def test_subgraph_excludes_notes_when_flag_false(self, kq):
        p1 = kq.kg.add_node("Paper", "p1", "Paper 1")
        note = kq.kg.add_node("P-Note", "n1", "Note on p1")
        kq.kg.add_edge(p1, note, "has_note")
        result = kq.get_paper_subgraph("p1", include_notes=False)
        note_nodes = [n for n in result["nodes"] if n["type"] == "P-Note"]
        assert note_nodes == []

    def test_subgraph_includes_notes_when_flag_true(self, kq):
        p1 = kq.kg.add_node("Paper", "p1", "Paper 1")
        note = kq.kg.add_node("P-Note", "n1", "Note on p1")
        kq.kg.add_edge(p1, note, "has_note")
        result = kq.get_paper_subgraph("p1", include_notes=True)
        note_nodes = [n for n in result["nodes"] if n["type"] == "P-Note"]
        assert len(note_nodes) == 1


class TestKGTagEcosystem:
    def test_tag_ecosystem(self, kq, kq_populated):
        result = kq.get_tag_ecosystem("LLM")
        node_types = {n["type"] for n in result["nodes"]}
        # Tag ecosystem returns papers related to the tag
        assert "Paper" in node_types
        assert result["tag"] == "LLM"

    def test_tag_ecosystem_empty_for_unknown_tag(self, kq):
        result = kq.get_tag_ecosystem("NonExistentTag")
        assert result["nodes"] == []
        assert result["edges"] == []


class TestKGExportGraphJson:
    def test_export_all(self, kq, kq_populated):
        result = kq.export_graph_json()
        assert len(result["nodes"]) == 4
        assert len(result["edges"]) == 4

    def test_export_subset_by_node_ids(self, kq, kq_populated):
        result = kq.export_graph_json(node_ids=[kq_populated["p1"]])
        # only p1 + its edges
        assert len(result["nodes"]) == 1
        assert result["nodes"][0]["entity_id"] == "p1"
