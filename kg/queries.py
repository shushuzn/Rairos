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
        edges = []  # type: ignore[var-annotated]
        visited = {paper_node["id"]}
        queue = [(paper_node["id"], 0)]

        while queue:
            current_id, d = queue.pop(0)
            if d >= depth:
                continue

            node_edges = self.kg.get_edges_by_node(current_id, direction="both")
            for edge in node_edges:
                if edge["id"] not in [e["id"] for e in edges]:
                    edges.append(edge)

                neighbor_id = edge["target_id"] if edge["source_id"] == current_id else edge["source_id"]
                if neighbor_id in visited:
                    continue
                visited.add(neighbor_id)

                neighbor = self.kg.get_node(neighbor_id)
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
        edges = []
        for nid in node_ids:
            for edge in self.kg.get_edges_by_node(nid, direction="both"):
                if edge["source_id"] in node_ids and edge["target_id"] in node_ids:
                    edges.append(edge)
        return {"nodes": paper_nodes + mnote_nodes, "edges": edges, "tag": tag}

    def export_graph_json(self, node_ids: Optional[list[str]] = None) -> dict:
        """Export subgraph as JSON-serializable dict for D3.js / PyVis."""
        if node_ids:
            nodes = [self.kg.get_node(nid) for nid in node_ids if self.kg.get_node(nid)]
        else:
            nodes = self.kg.get_all_nodes()  # type: ignore[assignment]

        seen = set()
        edges = []
        for node in nodes:
            for edge in self.kg.get_edges_by_node(node["id"], direction="both"):  # type: ignore[index]
                if edge["id"] not in seen:
                    seen.add(edge["id"])
                    edges.append(edge)

        return {"nodes": nodes, "edges": edges}
