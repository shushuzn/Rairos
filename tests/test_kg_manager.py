"""Tests for kg/manager.py — KGManager core CRUD and graph operations."""
from __future__ import annotations

import tempfile
from pathlib import Path

import pytest

from kg.manager import KGManager


@pytest.fixture
def kg(tmp_path):
    """Fresh KGManager backed by a temporary DB."""
    db = tmp_path / "test_kg.db"
    return KGManager(db_path=str(db))


class TestKGManagerNodeOps:
    def test_add_node_returns_id(self, kg):
        nid = kg.add_node("Paper", "paper-1", "Paper One", title="Hello", year=2024)
        assert isinstance(nid, str)
        assert len(nid) == 36  # UUID

    def test_add_node_idempotent(self, kg):
        nid1 = kg.add_node("Paper", "paper-1", "Paper One")
        nid2 = kg.add_node("Paper", "paper-1", "Paper One")
        assert nid1 == nid2

    def test_get_node(self, kg):
        nid = kg.add_node("Paper", "paper-1", "Paper One", year=2024)
        node = kg.get_node(nid)
        assert node is not None
        assert node["type"] == "Paper"
        assert node["entity_id"] == "paper-1"
        assert node["label"] == "Paper One"
        assert node["properties"]["year"] == 2024

    def test_get_node_not_found(self, kg):
        node = kg.get_node("does-not-exist")
        assert node is None

    def test_get_node_by_entity(self, kg):
        kg.add_node("Paper", "paper-1", "Paper One")
        node = kg.get_node_by_entity("Paper", "paper-1")
        assert node is not None
        assert node["entity_id"] == "paper-1"

    def test_get_node_by_entity_not_found(self, kg):
        node = kg.get_node_by_entity("Paper", "ghost")
        assert node is None

    def test_upsert_node_updates_properties(self, kg):
        kg.add_node("Paper", "paper-1", "Paper One", year=2023)
        nid = kg.upsert_node("Paper", "paper-1", "Paper One Updated", year=2024)
        node = kg.get_node(nid)
        assert node["label"] == "Paper One Updated"
        assert node["properties"]["year"] == 2024

    def test_upsert_node_inserts_if_missing(self, kg):
        nid = kg.upsert_node("Paper", "paper-new", "New Paper", tags=["LLM"])
        node = kg.get_node(nid)
        assert node["entity_id"] == "paper-new"
        assert node["properties"]["tags"] == ["LLM"]

    def test_get_all_nodes_no_filter(self, kg):
        kg.add_node("Paper", "p1", "One")
        kg.add_node("Paper", "p2", "Two")
        kg.add_node("Tag", "t1", "LLM")
        nodes = kg.get_all_nodes()
        assert len(nodes) == 3

    def test_get_all_nodes_filtered_by_type(self, kg):
        kg.add_node("Paper", "p1", "One")
        kg.add_node("Paper", "p2", "Two")
        kg.add_node("Tag", "t1", "LLM")
        papers = kg.get_all_nodes(node_type="Paper")
        assert len(papers) == 2
        assert all(n["type"] == "Paper" for n in papers)


class TestKGManagerEdgeOps:
    def test_add_edge_returns_id(self, kg):
        nid1 = kg.add_node("Paper", "p1", "One")
        nid2 = kg.add_node("Paper", "p2", "Two")
        eid = kg.add_edge(nid1, nid2, "cite")
        assert isinstance(eid, str)

    def test_add_edge_idempotent(self, kg):
        nid1 = kg.add_node("Paper", "p1", "One")
        nid2 = kg.add_node("Paper", "p2", "Two")
        eid1 = kg.add_edge(nid1, nid2, "cite")
        eid2 = kg.add_edge(nid1, nid2, "cite")
        assert eid1 == eid2

    def test_get_edge(self, kg):
        nid1 = kg.add_node("Paper", "p1", "One")
        nid2 = kg.add_node("Paper", "p2", "Two")
        eid = kg.add_edge(nid1, nid2, "cite", weight=0.9)
        edge = kg.get_edge(eid)
        assert edge is not None
        assert edge["source_id"] == nid1
        assert edge["target_id"] == nid2
        assert edge["relation_type"] == "cite"
        assert edge["weight"] == 0.9

    def test_get_edges_by_node_out(self, kg):
        nid1 = kg.add_node("Paper", "p1", "One")
        nid2 = kg.add_node("Paper", "p2", "Two")
        kg.add_edge(nid1, nid2, "cite")
        edges = kg.get_edges_by_node(nid1, direction="out")
        assert len(edges) == 1
        assert edges[0]["target_id"] == nid2

    def test_get_edges_by_node_in(self, kg):
        nid1 = kg.add_node("Paper", "p1", "One")
        nid2 = kg.add_node("Paper", "p2", "Two")
        kg.add_edge(nid1, nid2, "cite")
        edges = kg.get_edges_by_node(nid2, direction="in")
        assert len(edges) == 1
        assert edges[0]["source_id"] == nid1

    def test_get_edges_by_node_both(self, kg):
        nid1 = kg.add_node("Paper", "p1", "One")
        nid2 = kg.add_node("Paper", "p2", "Two")
        kg.add_edge(nid1, nid2, "cite")
        edges = kg.get_edges_by_node(nid1, direction="both")
        assert len(edges) == 1
        edges2 = kg.get_edges_by_node(nid2, direction="both")
        assert len(edges2) == 1

    def test_get_edges_by_node_filtered_by_rel_type(self, kg):
        nid1 = kg.add_node("Paper", "p1", "One")
        nid2 = kg.add_node("Paper", "p2", "Two")
        nid3 = kg.add_node("Tag", "t1", "LLM")
        kg.add_edge(nid1, nid2, "cite")
        kg.add_edge(nid1, nid3, "same_tag")
        cite_edges = kg.get_edges_by_node(nid1, direction="out", rel_type="cite")
        assert len(cite_edges) == 1
        assert cite_edges[0]["relation_type"] == "cite"


class TestKGManagerGraphQueries:
    def test_find_neighbors_depth1(self, kg):
        nid1 = kg.add_node("Paper", "p1", "One")
        nid2 = kg.add_node("Paper", "p2", "Two")
        kg.add_edge(nid1, nid2, "cite")
        neighbors = kg.find_neighbors(nid1, depth=1)
        assert len(neighbors) == 1
        neighbor_node, edge, depth = neighbors[0]
        assert neighbor_node["entity_id"] == "p2"
        assert depth == 1

    def test_find_neighbors_depth2(self, kg):
        nid1 = kg.add_node("Paper", "p1", "One")
        nid2 = kg.add_node("Paper", "p2", "Two")
        nid3 = kg.add_node("Paper", "p3", "Three")
        kg.add_edge(nid1, nid2, "cite")
        kg.add_edge(nid2, nid3, "cite")
        neighbors = kg.find_neighbors(nid1, depth=2)
        entity_ids = {n[0]["entity_id"] for n in neighbors}
        assert entity_ids == {"p2", "p3"}

    def test_find_shortest_path(self, kg):
        nid1 = kg.add_node("Paper", "p1", "One")
        nid2 = kg.add_node("Paper", "p2", "Two")
        nid3 = kg.add_node("Paper", "p3", "Three")
        kg.add_edge(nid1, nid2, "cite")
        kg.add_edge(nid2, nid3, "cite")
        path = kg.find_shortest_path(nid1, nid3)
        assert path is not None
        assert len(path) == 3
        assert path[0] == nid1
        assert path[-1] == nid3

    def test_find_shortest_path_no_path(self, kg):
        nid1 = kg.add_node("Paper", "p1", "One")
        nid2 = kg.add_node("Paper", "p2", "Two")
        # no edge between them
        path = kg.find_shortest_path(nid1, nid2)
        assert path is None

    def test_find_shortest_path_same_node(self, kg):
        nid1 = kg.add_node("Paper", "p1", "One")
        path = kg.find_shortest_path(nid1, nid1)
        assert path == [nid1]

    def test_find_papers_by_tag(self, kg):
        paper_nid = kg.add_node("Paper", "paper-llm", "LLM Paper")
        tag_nid = kg.add_node("Tag", "LLM", "LLM")
        kg.add_edge(paper_nid, tag_nid, "same_tag")
        papers = kg.find_papers_by_tag("LLM")
        assert len(papers) == 1
        assert papers[0]["entity_id"] == "paper-llm"

    def test_find_papers_by_tag_not_found(self, kg):
        papers = kg.find_papers_by_tag("NonExistentTag")
        assert papers == []

    def test_find_mnotes_by_tag(self, kg):
        mnote_nid = kg.add_node("M-Note", "m1", "Note about LLM", abstract="LLM is hot")
        papers = kg.find_mnotes_by_tag("LLM")
        assert len(papers) == 1
        assert papers[0]["entity_id"] == "m1"

    def test_stats(self, kg):
        kg.add_node("Paper", "p1", "One")
        kg.add_node("Paper", "p2", "Two")
        kg.add_node("Tag", "t1", "LLM")
        nid1 = kg.add_node("Paper", "p1", "One")
        nid2 = kg.add_node("Paper", "p2", "Two")
        kg.add_edge(nid1, nid2, "cite")
        s = kg.stats()
        assert s["total_nodes"] == 3
        assert s["nodes_by_type"]["Paper"] == 2
        assert s["nodes_by_type"]["Tag"] == 1
        assert s["total_edges"] == 1
