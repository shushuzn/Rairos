"""Tests for viz module — D3 and PyVis renderers."""
from __future__ import annotations

import pytest

from viz.d3_renderer import D3ForceGraph
from viz.pyvis_renderer import KGVizRenderer


class MockKGManager:
    """Minimal mock of KGManager for viz tests."""

    def __init__(self, nodes=None, edges=None):
        self._nodes = nodes or []
        self._edges = edges or []

    def get_node_by_entity(self, ntype, eid):
        for n in self._nodes:
            if n.get("type") == ntype and n.get("entity_id") == eid:
                return n
        return None

    def get_node(self, nid):
        for n in self._nodes:
            if n["id"] == nid:
                return n
        return None

    def get_all_nodes(self):
        return self._nodes

    def get_edges_by_node(self, nid, direction="both"):
        return [
            e for e in self._edges
            if e["source_id"] == nid or e["target_id"] == nid
        ]

    def find_papers_by_tag(self, tag):
        return [n for n in self._nodes if n.get("type") == "Paper" and tag in n.get("tags", [])]

    def find_mnotes_by_tag(self, tag):
        return [n for n in self._nodes if n.get("type") == "M-Note" and tag in n.get("tags", [])]

    def find_neighbors(self, nid, depth=2):
        results = []
        for e in self._edges:
            if e["source_id"] == nid:
                neighbor = self.get_node(e["target_id"])
                if neighbor:
                    results.append((neighbor, e, 1))
            elif e["target_id"] == nid:
                neighbor = self.get_node(e["source_id"])
                if neighbor:
                    results.append((neighbor, e, 1))
        return results


@pytest.fixture
def sample_kg():
    nodes = [
        {"id": "n1", "type": "Paper", "entity_id": "p1", "label": "Paper One", "tags": ["llm"]},
        {"id": "n2", "type": "Paper", "entity_id": "p2", "label": "Paper Two", "tags": ["llm"]},
        {"id": "n3", "type": "P-Note", "entity_id": "pn1", "label": "Note One"},
    ]
    edges = [
        {"id": "e1", "source_id": "n1", "target_id": "n2", "relation_type": "cites", "weight": 1.0},
        {"id": "e2", "source_id": "n1", "target_id": "n3", "relation_type": "has_note", "weight": 0.5},
    ]
    return MockKGManager(nodes=nodes, edges=edges)


class TestD3ForceGraph:
    def test_to_json_empty(self):
        kg = MockKGManager()
        d3 = D3ForceGraph(kg)
        result = d3.to_json()
        assert result == {"nodes": [], "links": []}

    def test_to_json_full_graph(self, sample_kg):
        d3 = D3ForceGraph(sample_kg)
        result = d3.to_json()
        assert len(result["nodes"]) == 3
        assert len(result["links"]) == 2
        node_ids = {n["id"] for n in result["nodes"]}
        assert "n1" in node_ids
        for link in result["links"]:
            assert link["source"] in node_ids
            assert link["target"] in node_ids

    def test_to_json_paper_uids(self, sample_kg):
        d3 = D3ForceGraph(sample_kg)
        result = d3.to_json(paper_uids=["p1"])
        assert any(n["id"] == "n1" for n in result["nodes"])
        # Links should only be between included nodes
        for link in result["links"]:
            assert link["source"] in {n["id"] for n in result["nodes"]}
            assert link["target"] in {n["id"] for n in result["nodes"]}

    def test_to_json_tag_filter(self, sample_kg):
        d3 = D3ForceGraph(sample_kg)
        result = d3.to_json(tag="llm")
        assert all(n["type"] == "Paper" for n in result["nodes"])

    def test_max_nodes_limit(self, sample_kg):
        d3 = D3ForceGraph(sample_kg)
        result = d3.to_json(max_nodes=1)
        assert len(result["nodes"]) == 1
        assert len(result["links"]) == 0


class TestKGVizRenderer:
    def test_paper_graph_fallback(self, sample_kg):
        viz = KGVizRenderer(sample_kg)
        html = viz._paper_graph_fallback("p1", depth=1)
        assert "Paper One" in html
        assert "neighbor" in html.lower()

    def test_paper_graph_fallback_not_found(self, sample_kg):
        viz = KGVizRenderer(sample_kg)
        html = viz._paper_graph_fallback("missing", depth=1)
        assert "not found" in html

    def test_tag_graph_fallback(self, sample_kg):
        viz = KGVizRenderer(sample_kg)
        html = viz._tag_graph_fallback("llm")
        assert "llm" in html
        assert "Paper One" in html

    def test_tag_graph_fallback_empty(self, sample_kg):
        viz = KGVizRenderer(sample_kg)
        html = viz._tag_graph_fallback("none")
        assert "No papers found" in html
