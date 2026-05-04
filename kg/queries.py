"""High-level graph query functions built on KGManager."""

from kg.manager import KGManager
from typing import Optional


class KGQueries:
    """Advanced query layer on top of KGManager."""

    def __init__(self, kg: KGManager):
        self.kg = kg

    def get_paper_subgraph(
        self,
        paper_uid: str,
        depth: int = 2,
        include_notes: bool = True,
    ) -> dict:
        """Get a paper's ego subgraph."""
        paper_node = self.kg.get_node_by_entity("Paper", paper_uid)
        if paper_node is None:
            return {"nodes": [], "edges": [], "center": None}

        nodes = [paper_node]
        edges = []
        visited = {paper_node["id"]}
        queue = [(paper_node["id"], 0)]
        seen_edge_ids = set()

        while queue:
            current_id, d = queue.pop(0)
            if d >= depth:
                continue

            node_edges = self.kg.get_edges_bulk([current_id], direction="both")
            for edge in node_edges:
                if edge["id"] not in seen_edge_ids:
                    seen_edge_ids.add(edge["id"])
                    edges.append(edge)

                neighbor_id = edge["target_id"] if edge["source_id"] == current_id else edge["source_id"]
                if neighbor_id in visited:
                    continue
                visited.add(neighbor_id)

                neighbors = self.kg.get_nodes_bulk([neighbor_id])
                neighbor = neighbors.get(neighbor_id)
                if neighbor is None:
                    continue
                if not include_notes and neighbor["type"] in ("P-Note", "C-Note", "M-Note"):
                    continue

                nodes.append(neighbor)
                queue.append((neighbor_id, d + 1))

        return {"nodes": nodes, "edges": edges, "center": paper_node}

    def get_tag_ecosystem(self, tag: str) -> dict:
        """Get all papers and M-Notes related to a tag."""
        paper_nodes = self.kg.find_papers_by_tag(tag)
        mnote_nodes = self.kg.find_mnotes_by_tag(tag)
        node_ids = {n["id"] for n in paper_nodes + mnote_nodes}
        if not node_ids:
            return {"nodes": [], "edges": [], "tag": tag}
        # Bulk fetch all edges in one query instead of N queries
        edges = [
            e for e in self.kg.get_edges_bulk(list(node_ids), direction="both")
            if e["source_id"] in node_ids and e["target_id"] in node_ids
        ]
        return {"nodes": paper_nodes + mnote_nodes, "edges": edges, "tag": tag}

    def export_graph_json(self, node_ids: Optional[list[str]] = None) -> dict:
        """Export subgraph as JSON-serializable dict for D3.js / PyVis."""
        if node_ids:
            nodes = [n for n in self.kg.get_nodes_bulk(node_ids).values() if n is not None]
        else:
            nodes = self.kg.get_all_nodes()  # type: ignore[assignment]

        if not nodes:
            return {"nodes": [], "edges": []}

        nid_set = {n["id"] for n in nodes}
        all_edges = self.kg.get_edges_bulk(list(nid_set), direction="both")
        edges = [e for e in all_edges if e["source_id"] in nid_set and e["target_id"] in nid_set]

        return {"nodes": nodes, "edges": edges}
