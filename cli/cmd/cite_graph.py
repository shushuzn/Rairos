"""CLI command: cite-graph."""
from __future__ import annotations

import argparse
import sys
from typing import Dict, List, Tuple

from cli._shared import get_db
from cli._shared import (
    Colors, colored, print_success, print_error, print_warning, print_info, print_header,
)


class CiteGraphNode:
    paper_id: str
    title: str
    depth: int  # 0 = root, 1 = direct, 2 = 2-hop
    direction: str  # "root", "forward" (papers citing it), "backward" (papers it cites)


def _extract_references_from_text(paper_id: str, text: str) -> dict[str, list[str]]:
    """Extract arXiv IDs, DOIs, PMIDs, and ISBNs from the plain text of a paper."""
    if not text or not text.strip():
        return {"arxiv_ids": [], "dois": [], "pmids": [], "isbns": []}

    import re
    arXiv_PAT = re.compile(r'\barXiv:\s*(\d+\.\d+\b)', re.IGNORECASE)
    DOI_PAT = re.compile(r'\b10\.\d{4,}/[^\s]+', re.IGNORECASE)
    PMID_PAT = re.compile(r'\bPMID:\s*(\d{6,})\b', re.IGNORECASE)
    ISBN_PAT = re.compile(r'\bISBN(?:-13)?:?\s*([0-9-X]{10,})\b', re.IGNORECASE)

    _REFS_SECTION_PAT = re.compile(r'(?:\n|^)[ ]*(?:\d+\.?\s*)?(?:References|Bibliography|Citations)', re.IGNORECASE)
    match = _REFS_SECTION_PAT.search(text)
    if match:
        text = text[match.start():]

    arxiv_ids = list(set(arXiv_PAT.findall(text)))
    dois = list(set(DOI_PAT.findall(text)))
    pmids = list(set(PMID_PAT.findall(text)))
    isbns = list(set(ISBN_PAT.findall(text)))

    return {"arxiv_ids": arxiv_ids, "dois": dois, "pmids": pmids, "isbns": isbns}


def _build_cite_graph_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "cite-graph",
        help="Visualize the local citation subgraph around a paper (1-2 hops)",
    )
    p.add_argument(
        "--paper", required=True, metavar="PAPER_ID",
        help="Root paper for the citation graph",
    )
    p.add_argument(
        "--depth", type=int, default=2, choices=[1, 2],
        help="Traversal depth: 1 = direct citations only, 2 = +2-hop (default: 2)",
    )
    p.add_argument(
        "--max-nodes", type=int, default=30,
        help="Maximum nodes per direction to show (default: 30)",
    )
    p.add_argument(
        "--format", choices=["text", "mermaid", "json"], default="text",
        help="Output format (default: text)",
    )
    p.add_argument(
        "--plain-text", metavar="TEXT",
        help="Build graph directly from plain text (extract references without DB import). "
             "Useful for papers not yet in the DB.",
    )
    p.add_argument(
        "--fetch-metadata", action="store_true",
        help="Fetch paper titles from arXiv/CrossRef APIs for plain-text mode. "
             "Implies --plain-text. Adds titles to graph nodes.",
    )
    return p


def _run_cite_graph(args: argparse.Namespace) -> int:
    db = get_db()
    db.init()

    root_id = args.paper
    plain_text = getattr(args, "plain_text", None)

    fetch_meta = getattr(args, "fetch_metadata", False)

    if plain_text is not None or fetch_meta:
        if fetch_meta and plain_text is None:
            print("Error: --fetch-metadata requires --plain-text", file=sys.stderr)
            return 1

        if plain_text is not None:
            result = _extract_references_from_text(root_id, plain_text)
        else:
            result = {"arxiv_ids": [], "dois": [], "pmids": [], "isbns": []}

        arxiv_ids = result["arxiv_ids"]
        dois = result["dois"]
        pmids = result["pmids"]
        isbns = result["isbns"]

        if not arxiv_ids and not dois and not pmids and not isbns:
            print(f"No references found in plain text for {root_id!r}")
            return 0

        nodes: Dict[str, CiteGraphNode] = {}
        edges: List[Tuple[str, str, str]] = []

        nodes[root_id] = CiteGraphNode(root_id, root_id, 0, "root")

        ref_ids: list[str] = []
        for aid in arxiv_ids:
            rid = f"arXiv:{aid}"
            if rid not in nodes:
                nodes[rid] = CiteGraphNode(rid, "", 1, "backward")
                edges.append((root_id, rid, "backward"))
            ref_ids.append(rid)

        for doi in dois:
            rid = f"DOI:{doi}"
            if rid not in nodes:
                nodes[rid] = CiteGraphNode(rid, "", 1, "backward")
                edges.append((root_id, rid, "backward"))

        print(f"Root: {root_id}")
        print(f"References ({len(ref_ids)}): {', '.join(ref_ids[:args.max_nodes])}")
        return 0

    # DB mode
    nodes, edges = db.get_citation_subgraph(root_id, depth=args.depth, max_nodes=args.max_nodes)

    if args.format == "mermaid":
        print("graph TD")
        for n in nodes.values():
            print(f'    {n.paper_id.replace("-", "_")}["{n.title[:30] if n.title else n.paper_id}"]')
        for src, tgt, _ in edges:
            src_s = src.replace("-", "_")
            tgt_s = tgt.replace("-", "_")
            print(f"    {src_s} --> {tgt_s}")
    elif args.format == "json":
        import json
        print(json.dumps({"nodes": [{"id": n.paper_id, "title": n.title, "depth": n.depth, "direction": n.direction} for n in nodes.values()], "edges": [{"from": e[0], "to": e[1]} for e in edges]}, indent=2))
    else:
        print(f"Citation graph for {root_id} (depth={args.depth}):")
        for n in sorted(nodes.values(), key=lambda x: (x.depth, x.direction)):
            label = f"{'[ROOT]' if n.depth == 0 else f'[D{n.depth}]'}"
            print(f"  {label} {n.paper_id}  {n.title[:60] if n.title else ''}")
        print(f"\n{len(nodes)} nodes, {len(edges)} edges")

    return 0
