"""Tests for viz/d3_renderer.py."""

import pytest
from unittest.mock import MagicMock, patch


class TestD3ForceGraph:
    """Tests for D3ForceGraph."""

    def _make_kg(self, nodes=None, edges=None):
        """Build a mock KGManager with optional nodes/edges."""
        kg = MagicMock()
        if nodes is not None:
            kg.get_nodes_bulk.return_value = {n["id"]: n for n in nodes}
        if edges is not None:
            kg.get_edges_bulk.return_value = edges
        return kg

    def test_init_with_no_kg(self):
        """D3ForceGraph accepts None kg (uses default KGManager)."""
        from viz.d3_renderer import D3ForceGraph
        with patch("viz.d3_renderer.KGManager") as MockKG:
            g = D3ForceGraph()
            MockKG.assert_called_once()

    def test_init_with_kg_instance(self):
        """D3ForceGraph accepts a KGManager instance."""
        from viz.d3_renderer import D3ForceGraph
        kg = MagicMock()
        g = D3ForceGraph(kg=kg)
        assert g.kg is kg

    def test_to_json_paper_uids_returns_d3_format(self):
        """to_json with paper_uids returns {nodes, links} with D3 field names."""
        from viz.d3_renderer import D3ForceGraph
        kg = self._make_kg()
        g = D3ForceGraph(kg=kg)

        with patch("kg.queries.KGQueries") as MockQ:
            MockQ.return_value.get_paper_subgraph.return_value = {
                "nodes": [
                    {"id": "n1", "label": "Paper A", "type": "Paper", "entity_id": "uid1"}
                ],
                "edges": [
                    {"id": "e1", "source_id": "n1", "target_id": "n1",
                     "relation_type": "cite", "weight": 1.0}
                ],
            }
            result = g.to_json(paper_uids=["uid1"])

        assert "nodes" in result
        assert "links" in result
        node = result["nodes"][0]
        assert "id" in node
        assert "label" in node
        assert "type" in node
        assert "entity_id" in node
        link = result["links"][0]
        assert "source" in link
        assert "target" in link
        assert "relation" in link
        assert "weight" in link

    def test_to_json_paper_uids_deduplicates_nodes(self):
        """Duplicate nodes across subgraphs are deduplicated."""
        from viz.d3_renderer import D3ForceGraph
        kg = self._make_kg()
        g = D3ForceGraph(kg=kg)

        with patch("kg.queries.KGQueries") as MockQ:
            node = {"id": "shared", "label": "Shared", "type": "Paper", "entity_id": "uid1"}
            MockQ.return_value.get_paper_subgraph.side_effect = [
                {"nodes": [node], "edges": []},
                {"nodes": [node], "edges": []},
            ]
            result = g.to_json(paper_uids=["uid1", "uid2"])

        ids = [n["id"] for n in result["nodes"]]
        assert ids.count("shared") == 1

    def test_to_json_paper_uids_respects_max_nodes(self):
        """Results are truncated to max_nodes."""
        from viz.d3_renderer import D3ForceGraph
        kg = self._make_kg()
        g = D3ForceGraph(kg=kg)

        with patch("kg.queries.KGQueries") as MockQ:
            MockQ.return_value.get_paper_subgraph.return_value = {
                "nodes": [{"id": f"n{i}", "label": f"P{i}", "type": "Paper", "entity_id": f"u{i}"}
                          for i in range(10)],
                "edges": [],
            }
            result = g.to_json(paper_uids=["uid1"], max_nodes=3)

        assert len(result["nodes"]) == 3

    def test_to_json_tag_returns_bulk_edges(self):
        """to_json with tag uses get_edges_bulk (not N+1 get_edges_by_node)."""
        from viz.d3_renderer import D3ForceGraph
        paper = {"id": "p1", "label": "Paper", "type": "Paper", "entity_id": "uid1"}
        kg = MagicMock()
        kg.find_papers_by_tag.return_value = [paper]
        kg.get_edges_bulk.return_value = [
            {"id": "e1", "source_id": "p1", "target_id": "p1",
             "relation_type": "same_tag", "weight": 1.0}
        ]
        g = D3ForceGraph(kg=kg)
        result = g.to_json(tag="machine-learning")

        kg.find_papers_by_tag.assert_called_once_with("machine-learning")
        kg.get_edges_bulk.assert_called_once()
        assert result["nodes"][0]["id"] == "p1"
        assert result["links"][0]["source"] == "p1"

    def test_to_json_tag_empty_when_no_papers(self):
        """Tag query returning no papers gives empty result."""
        from viz.d3_renderer import D3ForceGraph
        kg = MagicMock()
        kg.find_papers_by_tag.return_value = []
        g = D3ForceGraph(kg=kg)
        result = g.to_json(tag="nonexistent")
        assert result["nodes"] == []
        assert result["links"] == []

    def test_to_json_full_graph_returns_all_nodes(self):
        """to_json without args returns full graph."""
        from viz.d3_renderer import D3ForceGraph
        kg = MagicMock()
        kg.get_all_nodes.return_value = [
            {"id": "n1", "label": "A", "type": "Paper", "entity_id": "u1"},
            {"id": "n2", "label": "B", "type": "Tag", "entity_id": "tag1"},
        ]
        kg.get_edges_bulk.return_value = [
            {"id": "e1", "source_id": "n1", "target_id": "n2",
             "relation_type": "same_tag", "weight": 1.0}
        ]
        g = D3ForceGraph(kg=kg)
        result = g.to_json()

        kg.get_all_nodes.assert_called_once()
        kg.get_edges_bulk.assert_called_once()
        assert len(result["nodes"]) == 2
        assert len(result["links"]) == 1

    def test_to_json_full_graph_respects_max_nodes(self):
        """Full graph export is capped at max_nodes."""
        from viz.d3_renderer import D3ForceGraph
        kg = MagicMock()
        kg.get_all_nodes.return_value = [
            {"id": f"n{i}", "label": f"N{i}", "type": "Paper", "entity_id": f"u{i}"}
            for i in range(10)
        ]
        kg.get_edges_bulk.return_value = []
        g = D3ForceGraph(kg=kg)
        result = g.to_json(max_nodes=5)
        assert len(result["nodes"]) == 5

    def test_to_json_label_truncated_to_60_chars(self):
        """Node label is sliced to 60 characters."""
        from viz.d3_renderer import D3ForceGraph
        kg = self._make_kg(nodes=[
            {"id": "n1", "label": "A" * 100, "type": "Paper", "entity_id": "u1"}
        ], edges=[])
        g = D3ForceGraph(kg=kg)

        with patch("kg.queries.KGQueries") as MockQ:
            MockQ.return_value.get_paper_subgraph.return_value = {
                "nodes": [{"id": "n1", "label": "A" * 100, "type": "Paper", "entity_id": "u1"}],
                "edges": [],
            }
            result = g.to_json(paper_uids=["u1"])

        assert len(result["nodes"][0]["label"]) == 60

    def test_to_json_missing_node_fields_default_to_empty(self):
        """Nodes without label/type/entity_id get empty-string defaults."""
        from viz.d3_renderer import D3ForceGraph
        kg = self._make_kg(nodes=[{"id": "n1"}], edges=[])
        g = D3ForceGraph(kg=kg)

        with patch("kg.queries.KGQueries") as MockQ:
            MockQ.return_value.get_paper_subgraph.return_value = {
                "nodes": [{"id": "n1"}],
                "edges": [],
            }
            result = g.to_json(paper_uids=["u1"])

        assert result["nodes"][0]["label"] == ""
        assert result["nodes"][0]["type"] == "Paper"
        assert result["nodes"][0]["entity_id"] == ""

    def test_to_json_missing_edge_fields_use_defaults(self):
        """Edges without relation_type or weight use defaults."""
        from viz.d3_renderer import D3ForceGraph
        kg = self._make_kg(nodes=[], edges=[])
        g = D3ForceGraph(kg=kg)

        with patch("kg.queries.KGQueries") as MockQ:
            MockQ.return_value.get_paper_subgraph.return_value = {
                "nodes": [{"id": "n1", "label": "A", "type": "Paper", "entity_id": "u1"}],
                "edges": [{"id": "e1", "source_id": "n1", "target_id": "n1"}],
            }
            result = g.to_json(paper_uids=["u1"])

        assert result["links"][0]["relation"] == ""
        assert result["links"][0]["weight"] == 1.0

    def test_to_json_edges_filtered_to_valid_nodes(self):
        """Edges whose endpoint nodes were truncated are excluded."""
        from viz.d3_renderer import D3ForceGraph
        kg = self._make_kg(nodes=[], edges=[])
        g = D3ForceGraph(kg=kg)

        with patch("kg.queries.KGQueries") as MockQ:
            MockQ.return_value.get_paper_subgraph.return_value = {
                "nodes": [{"id": "n1", "label": "A", "type": "Paper", "entity_id": "u1"}],
                "edges": [
                    {"id": "e1", "source_id": "n1", "target_id": "n2",
                     "relation_type": "cite", "weight": 1.0}
                ],
            }
            result = g.to_json(paper_uids=["u1"], max_nodes=1)

        assert len(result["links"]) == 0

    def test_to_json_uses_bulk_api_not_n_plus_one(self):
        """The tag branch calls get_edges_bulk once, not get_edges_by_node per node."""
        from viz.d3_renderer import D3ForceGraph
        kg = MagicMock()
        kg.find_papers_by_tag.return_value = [
            {"id": "p1", "label": "Paper1", "type": "Paper", "entity_id": "u1"},
            {"id": "p2", "label": "Paper2", "type": "Paper", "entity_id": "u2"},
        ]
        kg.get_edges_bulk.return_value = []
        g = D3ForceGraph(kg=kg)
        g.to_json(tag="ml")

        kg.get_edges_bulk.assert_called_once()
        kg.get_edges_by_node.assert_not_called()
