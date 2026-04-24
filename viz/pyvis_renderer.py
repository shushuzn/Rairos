"""PyVis-based interactive KG visualization."""

import json

try:
    from pyvis.network import Network
    _HAS_PYVIS = True
except ImportError:
    _HAS_PYVIS = False

from kg.manager import KGManager
from typing import Optional


# Node colour map by type
_TYPE_COLORS = {
    "Paper":    "#4A90E2",  # blue
    "P-Note":   "#50C878",  # green
    "C-Note":   "#F4D03F",  # yellow
    "M-Note":   "#E74C3C",  # red
    "Tag":      "#9B59B6",  # purple
    "Author":   "#E67E22",  # orange
}


class KGVizRenderer:
    """Renders KG as interactive PyVis HTML."""

    def __init__(self, kg: Optional[KGManager] = None):
        self.kg = kg or KGManager()

    def _base_net(self, height: str = "600px", bgcolor: str = "#ffffff") -> "Network":
        net = Network(height=height, bgcolor=bgcolor, directed=False, select_menu=True)
        net.set_options("""
        {
            "nodes": {"font": {"size": 14, "face": "arial"}},
            "edges": {"width": 1, "color": {"inherit": true}},
            "physics": {"enabled": true, "forceAtlas2Based": {"gravitationalConstant": -50}, "solver": "forceAtlas2Based"}
        }
        """)
        return net

    def _add_nodes_from_graph(self, net: "Network", graph: dict):
        for node in graph.get("nodes", []):
            ntype = node.get("type", "Paper")
            color = _TYPE_COLORS.get(ntype, "#AAAAAA")
            label = node.get("label", "")[:60]
            title = json.dumps(node, ensure_ascii=False)
            net.add_node(node["id"], label=label, title=title,
                         color=color, type=ntype)

        seen = set()
        for edge in graph.get("edges", []):
            eid = edge["id"]
            if eid in seen:
                continue
            seen.add(eid)
            net.add_edge(
                edge["source_id"], edge["target_id"],
                title=edge.get("relation_type", ""),
                width=edge.get("weight", 1.0),
            )

    def paper_graph(self, paper_uid: str, depth: int = 2, height: str = "600px") -> "str":
        """Ego graph around a paper as HTML string."""
        if not _HAS_PYVIS:
            return self._paper_graph_fallback(paper_uid, depth)

        from kg.queries import KGQueries
        q = KGQueries(self.kg)
        subgraph = q.get_paper_subgraph(paper_uid, depth=depth)

        net = self._base_net(height=height)
        self._add_nodes_from_graph(net, subgraph)

        center_node = subgraph.get("center")
        if center_node:
            net.set_options("""
            {
                "nodes": {"font": {"size": 14}},
                "edges": {"color": {"inherit": true}, "width": 1},
                "physics": {"enabled": true, "forceAtlas2Based": {"gravitationalConstant": -80}, "solver": "forceAtlas2Based"}
            }
            """)

        return net.generate_html()  # type: ignore[no-any-return]

    def tag_graph(self, tag: str, height: str = "600px") -> "str":
        """All papers + citations for a tag as HTML string."""
        if not _HAS_PYVIS:
            return self._tag_graph_fallback(tag)

        from kg.queries import KGQueries
        q = KGQueries(self.kg)
        ecosystem = q.get_tag_ecosystem(tag)

        net = self._base_net(height=height)
        self._add_nodes_from_graph(net, ecosystem)
        return net.generate_html()  # type: ignore[no-any-return]

    def full_graph(self, max_nodes: int = 500, height: str = "800px") -> "str":
        """Global KG graph (limited) as HTML string."""
        if not _HAS_PYVIS:
            return "<p>PyVis not installed. Run: pip install pyvis</p>"

        from kg.queries import KGQueries
        q = KGQueries(self.kg)
        export = q.export_graph_json()

        # Limit nodes
        nodes = export["nodes"][:max_nodes]
        nids = {n["id"] for n in nodes}
        edges = [e for e in export["edges"] if e["source_id"] in nids and e["target_id"] in nids]

        net = self._base_net(height=height)
        for node in nodes:
            ntype = node.get("type", "Paper")
            color = _TYPE_COLORS.get(ntype, "#AAAAAA")
            net.add_node(node["id"], label=node.get("label", "")[:40],
                         color=color, type=ntype)
        for edge in edges:
            net.add_edge(edge["source_id"], edge["target_id"],
                         title=edge.get("relation_type", ""))
        return net.generate_html()  # type: ignore[no-any-return]

    # ─── Fallback (no PyVis) ─────────────────────────────────────────

    def _paper_graph_fallback(self, paper_uid: str, depth: int) -> str:
        paper_node = self.kg.get_node_by_entity("Paper", paper_uid)
        if not paper_node:
            return f"<p>Paper '{paper_uid}' not found in KG.</p>"
        neighbors = self.kg.find_neighbors(paper_node["id"], depth=depth)
        lines = [f"<h3>Ego Graph: {paper_node['label'][:60]}</h3>"]
        lines.append(f"<p>Center: [{paper_node['type']}] {paper_node['label']}</p>")
        lines.append(f"<p>{len(neighbors)} neighbor(s):</p><ul>")
        for node, edge, d in neighbors:
            lines.append(f"<li>[depth={d}] {node['type']} | {edge['relation_type']} | {node['label'][:60]}</li>")
        lines.append("</ul>")
        return "\n".join(lines)

    def _tag_graph_fallback(self, tag: str) -> str:
        paper_nodes = self.kg.find_papers_by_tag(tag)
        if not paper_nodes:
            return f"<p>No papers found for tag '{tag}'.</p>"
        lines = [f"<h3>Tag Ecosystem: {tag}</h3>"]
        lines.append(f"<p>{len(paper_nodes)} paper(s):</p><ul>")
        for n in paper_nodes:
            lines.append(f"<li>[Paper] {n['label'][:60]}</li>")
        lines.append("</ul>")
        return "\n".join(lines)
