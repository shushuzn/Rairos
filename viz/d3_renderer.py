"""D3.js-compatible force graph export."""

from kg.manager import KGManager
from typing import Optional


class D3ForceGraph:
    """Export KG as D3.js-compatible nodes+links JSON."""

    def __init__(self, kg: Optional[KGManager] = None):
        self.kg = kg or KGManager()

    def to_json(self, paper_uids: Optional[list[str]] = None,
                tag: Optional[str] = None,
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
            all_edges = self.kg.get_edges_bulk(list(nids), direction="both") if nids else []
            seen = set()
            edges = []
            for edge in all_edges:
                if edge["id"] in seen:
                    continue
                seen.add(edge["id"])
                if edge["source_id"] in nids and edge["target_id"] in nids:
                    edges.append(edge)

        else:
            all_nodes = self.kg.get_all_nodes()
            nodes = all_nodes[:max_nodes]
            nids = {n["id"] for n in nodes}
            all_edges = self.kg.get_edges_bulk(list(nids), direction="both") if nids else []
            seen = set()
            edges = []
            for edge in all_edges:
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

    def to_citation_json(self, paper_id: str, depth: int = 1,
                         max_nodes: int = 100) -> dict:
        """Build citation graph for a paper from KG cite edges.

        Args:
            paper_id: Root paper UID (with or without arXiv: prefix)
            depth: 1 = direct citations only, 2 = +2-hop
            max_nodes: Max nodes per direction

        Returns:
            D3.js-compatible {nodes, links} dict with citation metadata.
        """
        # Resolve root node
        root_node = self.kg.get_node_by_entity("Paper", paper_id)
        if root_node is None:
            # Try with arXiv: prefix
            if not paper_id.startswith("arXiv:"):
                root_node = self.kg.get_node_by_entity("Paper", f"arXiv:{paper_id}")
            else:
                root_node = self.kg.get_node_by_entity("Paper", paper_id)
        if root_node is None:
            return {"nodes": [], "links": [], "root": paper_id}

        nodes = {}  # id -> node dict
        links = []  # list of link dicts

        def add_paper_node(uid: str, label: str, is_root: bool = False):
            nid = uid if uid.startswith("arXiv:") else f"arXiv:{uid}"
            if nid not in nodes:
                nodes[nid] = {
                    "id": nid,
                    "label": label[:60] if label else nid,
                    "type": "Paper",
                    "entity_id": uid,
                    "is_root": is_root,
                    "is_citing": False,  # will be set below
                    "is_cited_by": False,
                }
            if is_root:
                nodes[nid]["is_root"] = True
            return nid

        # Root
        root_nid = add_paper_node(
            root_node["entity_id"], root_node["label"], is_root=True
        )

        # BFS for cite edges
        visited = {root_nid}
        queue = [(root_nid, 0)]
        cite_rel = "cite"

        while queue:
            current_nid, current_depth = queue.pop(0)
            if current_depth >= depth:
                continue

            # Find all edges from this node (outbound = cites)
            outbound = self.kg.get_edges_by_node(current_nid, direction="out", rel_type=cite_rel)
            citing_count = 0
            for edge in outbound:
                if citing_count >= max_nodes:
                    break
                tgt = edge["target_id"]
                if tgt in visited:
                    continue
                visited.add(tgt)
                tgt_node = self.kg.get_node(tgt)
                if tgt_node:
                    add_paper_node(tgt_node["entity_id"], tgt_node["label"])
                    nodes[tgt]["is_citing"] = True
                links.append({
                    "source": current_nid,
                    "target": tgt,
                    "relation": "cites",
                    "weight": edge.get("weight", 1.0),
                })
                citing_count += 1
                queue.append((tgt, current_depth + 1))

            # Find all edges TO this node (inbound = cited by)
            inbound = self.kg.get_edges_by_node(current_nid, direction="in", rel_type=cite_rel)
            cited_by_count = 0
            for edge in inbound:
                if cited_by_count >= max_nodes:
                    break
                src = edge["source_id"]
                if src in visited:
                    continue
                visited.add(src)
                src_node = self.kg.get_node(src)
                if src_node:
                    add_paper_node(src_node["entity_id"], src_node["label"])
                    nodes[src]["is_cited_by"] = True
                links.append({
                    "source": src,
                    "target": current_nid,
                    "relation": "cited_by",
                    "weight": edge.get("weight", 1.0),
                })
                cited_by_count += 1
                queue.append((src, current_depth + 1))

        return {
            "nodes": list(nodes.values()),
            "links": links,
            "root": root_nid,
        }

    def to_similar_json(self, paper_id: str, threshold: float = 0.85,
                        max_nodes: int = 30) -> dict:
        """Build similarity graph from paper embeddings.

        Args:
            paper_id: Root paper ID
            threshold: Minimum cosine similarity (default 0.85)
            max_nodes: Max similar papers to include (default 30)

        Returns:
            D3.js-compatible {nodes, links, root} dict.
            Root paper + similar papers with similarity-weighted edges.
        """
        from db.database import Database

        db = Database()
        db.init()

        # Get root paper
        root_paper = db.get_paper(paper_id)
        if root_paper is None:
            return {"nodes": [], "links": [], "root": paper_id}

        # Find similar papers
        similar = db.find_similar(paper_id, threshold=threshold, limit=max_nodes)

        nodes = {}
        links = []

        # Root node
        nid = paper_id if paper_id.startswith("arXiv:") else f"arXiv:{paper_id}"
        nodes[nid] = {
            "id": nid,
            "label": root_paper.title[:60],
            "type": "Paper",
            "entity_id": paper_id,
            "is_root": True,
        }

        # Similar papers
        for sim_paper, score in similar:
            sim_nid = sim_paper.id if sim_paper.id.startswith("arXiv:") else f"arXiv:{sim_paper.id}"
            nodes[sim_nid] = {
                "id": sim_nid,
                "label": sim_paper.title[:60],
                "type": "Paper",
                "entity_id": sim_paper.id,
                "is_root": False,
                "similarity": round(float(score), 4),
            }
            links.append({
                "source": nid,
                "target": sim_nid,
                "relation": "similar",
                "weight": float(score),
            })

        return {
            "nodes": list(nodes.values()),
            "links": links,
            "root": nid,
        }
