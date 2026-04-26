"""CLI command: cite-graph."""
from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from typing import Dict, List, Tuple

from cli._shared import get_db
from cli._shared import (
    Colors, colored, print_success, print_error, print_warning, print_info, print_header,
)
from kg import KGManager


@dataclass
class CiteGraphNode:
    paper_id: str
    title: str
    depth: int  # 0 = root, 1 = direct, 2 = 2-hop
    direction: str  # "root", "forward" (papers citing it), "backward" (papers it cites)


def _extract_references_from_text(paper_id: str, text: str) -> dict[str, list[str]]:
    """Extract arXiv IDs, DOIs, PMIDs, and ISBNs from the plain text of a paper."""
    if not text or not text.strip():
        return {"arxiv_ids": [], "dois": [], "pmids": [], "isbns": []}

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
        help="Citation network: text, JSON, or interactive D3.js graph view",
    )
    sub = p.add_subparsers(dest="cite_subcmd", help="Citation subcommands")

    # cite-graph (text/json/mermaid output)
    p_text = sub.add_parser(
        "run",
        help="Text/JSON/Mermaid output of citation subgraph",
    )
    p_text.add_argument(
        "--paper", required=True, metavar="PAPER_ID",
        help="Root paper for the citation graph",
    )
    p_text.add_argument(
        "--depth", type=int, default=2, choices=[1, 2],
        help="Traversal depth: 1 = direct citations only, 2 = +2-hop (default: 2)",
    )
    p_text.add_argument(
        "--max-nodes", type=int, default=30,
        help="Maximum nodes per direction to show (default: 30)",
    )
    p_text.add_argument(
        "--format", choices=["text", "mermaid", "json"], default="text",
        help="Output format (default: text)",
    )
    p_text.add_argument(
        "--plain-text", metavar="TEXT",
        help="Build graph directly from plain text (extract references without DB import). "
             "Useful for papers not yet in the DB.",
    )
    p_text.add_argument(
        "--fetch-metadata", action="store_true",
        help="Fetch paper titles from arXiv/CrossRef APIs for plain-text mode. "
             "Implies --plain-text. Adds titles to graph nodes.",
    )

    # cite-graph view (D3.js interactive visualization)
    vp = sub.add_parser(
        "view",
        help="Open interactive D3.js citation network in browser",
    )
    vp.add_argument(
        "--paper", required=True, metavar="PAPER_ID",
        help="Root paper for the citation graph",
    )
    vp.add_argument(
        "--depth", type=int, default=1, choices=[1, 2],
        help="Traversal depth: 1 = direct citations only, 2 = +2-hop (default: 1)",
    )
    vp.add_argument(
        "--max-nodes", type=int, default=50,
        help="Maximum nodes per direction to show (default: 50)",
    )
    vp.add_argument(
        "--open", action="store_true", default=True, help="Open in default browser (default: on)",
    )
    vp.add_argument(
        "--no-open", dest="open", action="store_false", help="Write HTML to stdout instead of opening browser",
    )
    return p


def _run_cite_graph(args: argparse.Namespace) -> int:
    """Dispatch to subcommands: run (text/JSON) or view (D3.js)."""
    subcmd = getattr(args, "cite_subcmd", "run")
    if subcmd == "view":
        return _run_cite_graph_view(args)
    return _run_cite_graph_text(args)


def _run_cite_graph_text(args: argparse.Namespace) -> int:
    """Text/JSON/Mermaid citation subgraph output."""
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

        for pmid in pmids:
            rid = f"PMID:{pmid}"
            if rid not in nodes:
                nodes[rid] = CiteGraphNode(rid, "", 1, "backward")
                edges.append((root_id, rid, "backward"))

        for isbn in isbns:
            rid = f"ISBN:{isbn}"
            if rid not in nodes:
                nodes[rid] = CiteGraphNode(rid, "", 1, "backward")
                edges.append((root_id, rid, "backward"))

        all_refs = ref_ids + [f"DOI:{d}" for d in dois] + [f"PMID:{p}" for p in pmids] + [f"ISBN:{i}" for i in isbns]
        print(f"Root: {root_id}")
        print(f"References ({len(all_refs)}): {', '.join(all_refs[:args.max_nodes])}")
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


def _run_cite_graph_view(args: argparse.Namespace) -> int:
    """Render interactive D3.js citation graph and open in browser (or write to stdout)."""
    import tempfile, json, webbrowser, os
    from pathlib import Path
    from viz.d3_renderer import D3ForceGraph

    kg = KGManager()
    renderer = D3ForceGraph(kg)

    graph_data = renderer.to_citation_json(
        paper_id=args.paper,
        depth=args.depth,
        max_nodes=args.max_nodes,
    )

    if not graph_data["nodes"]:
        print(f"No citation data found for '{args.paper}'. Ensure the paper is in the KG with cite edges.", file=sys.stderr)
        return 1

    # Load citation template
    template_path = Path(__file__).parent.parent.parent / "viz" / "templates" / "cite_viz_template_d3.html"
    html_content = template_path.read_text(encoding="utf-8")

    # Inject data
    nodes_json = json.dumps(graph_data["nodes"], ensure_ascii=False)
    links_json = json.dumps(graph_data["links"], ensure_ascii=False)
    html_content = html_content.replace('INJECT_NODES', nodes_json)
    html_content = html_content.replace('INJECT_LINKS', links_json)
    html_content = html_content.replace('INJECT_ROOT_ID', f'"{graph_data["root"]}"')

    if not args.open:
        sys.stdout.write(html_content)
        return 0

    with tempfile.NamedTemporaryFile(
        mode="w", suffix=".html", delete=False, encoding="utf-8"
    ) as f:
        f.write(html_content)
        tmp_path = f.name

    webbrowser.open(f"file://{tmp_path}")
    citing = len([n for n in graph_data["nodes"] if n.get("is_citing")])
    cited_by = len([n for n in graph_data["nodes"] if n.get("is_cited_by")])
    print(f"Opened citation graph for '{args.paper}' in browser.")
    print(f"  {len(graph_data['nodes'])} papers · {citing} cites · {cited_by} cited by")
    print(f"(HTML also saved to: {tmp_path})")
    return 0
