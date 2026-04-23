"""D3.js-compatible force graph export."""

from kg.manager import KGManager


class D3ForceGraph:
    """Export KG as D3.js-compatible nodes+links JSON."""

    def __init__(self, kg: KGManager | None = None):
        self.kg = kg or KGManager()

    def to_json(self, paper_uids: list[str] | None = None,
                tag: str | None = None,
                max_nodes: int = 500) -> dict:
        """Return D3.js compatible {nodes, links} dict.

        If paper_uids provided, export those papers' ego subgraphs.
        If tag provided, export all papers with that tag.
        Otherwise export full graph (up to max_nodes).
        """
        from kg.queries import KGQueries
        q = KGQueries(self.kg)

        if paper_uids:
            all_nodes = []
            all_edges = []
            for uid in paper_uids:
                sg = q.get_paper_subgraph(uid, depth=2)
                all_nodes.extend(sg["nodes"])
                all_edges.extend(sg["edges"])
            # Dedupe
            seen_n = {}
            for n in all_nodes:
                seen_n[n["id"]] = n
            seen_e = {}
            for e in all_edges:
                seen_e[e["id"]] = e
            nodes = list(seen_n.values())[:max_nodes]
            nids = {n["id"] for n in nodes}
            edges = [e for e in seen_e.values()
                     if e["source_id"] in nids and e["target_id"] in nids]

        elif tag:
            paper_nodes = self.kg.find_papers_by_tag(tag)
            nodes = paper_nodes[:max_nodes]
            nids = {n["id"] for n in nodes}
            edges = []
            seen = set()
            for nid in nids:
                for edge in self.kg.get_edges_by_node(nid, direction="both"):
                    if edge["id"] in seen:
                        continue
                    seen.add(edge["id"])
                    if edge["source_id"] in nids and edge["target_id"] in nids:
                        edges.append(edge)

        else:
            all_nodes = self.kg.get_all_nodes()
            nodes = all_nodes[:max_nodes]
            nids = {n["id"] for n in nodes}
            edges = []
            seen = set()
            for nid in nids:
                for edge in self.kg.get_edges_by_node(nid, direction="both"):
                    if edge["id"] in seen:
                        continue
                    seen.add(edge["id"])
                    if edge["source_id"] in nids and edge["target_id"] in nids:
                        edges.append(edge)

        # D3 format
        d3_nodes = []
        for n in nodes:
            d3_nodes.append({
                "id": n["id"],
                "label": n.get("label", "")[:60],
                "type": n.get("type", "Paper"),
                "entity_id": n.get("entity_id", ""),
            })

        d3_links = []
        for e in edges:
            d3_links.append({
                "source": e["source_id"],
                "target": e["target_id"],
                "relation": e.get("relation_type", ""),
                "weight": e.get("weight", 1.0),
            })

        return {"nodes": d3_nodes, "links": d3_links}
