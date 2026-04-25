"""CLI command: kg."""
from __future__ import annotations

import argparse
import json

from cli._shared import (
    Colors, colored, print_success, print_error, print_warning, print_info, print_header,
    cmd_infer_tags_if_empty as infer_tags_if_empty,
)
from kg import KGManager


def _build_kg_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "kg",
        help="Knowledge graph — query, visualize, and manage the research KG",
        prog="airos kg",
        description="Query, visualize, and manage the research knowledge graph.",
        epilog="""\
Examples:
  %(prog)s graph 2301.00001 --depth 3      # ego graph with 3 hops
  %(prog)s path 2301.00001 2301.00002     # shortest path between papers
  %(prog)s search --tag LLM                # find all LLM-related nodes
  %(prog)s stats                            # KG statistics
  %(prog)s rebuild --incremental            # rebuild KG incrementally""",
    )

    sub = p.add_subparsers(dest="kg_cmd", help="KG subcommands")

    # kg graph
    gp = sub.add_parser("graph", help="Show paper's ego graph (neighbors up to N hops)")
    gp.add_argument("paper_id", help="Paper UID")
    gp.add_argument("--depth", type=int, default=2, help="BFS depth (default 2)")
    gp.add_argument("--format", choices=["text", "json"], default="text")

    # kg path
    pp = sub.add_parser("path", help="Find shortest path between two nodes")
    pp.add_argument("idA", help="First node ID or paper UID")
    pp.add_argument("idB", help="Second node ID or paper UID")

    # kg stats
    sub.add_parser("stats", help="KG statistics (nodes/edges by type)")

    # kg rebuild
    rp = sub.add_parser("rebuild", help="Rebuild KG from papers.json")
    rp.add_argument("--papers-json", default="", help="Path to papers.json (default: auto-detect)")
    rp.add_argument("--incremental", action="store_true", help="Only process new/changed papers since last rebuild")

    # kg search
    fp = sub.add_parser("search", help="Find nodes by tag or type")
    fp.add_argument("--tag", help="Filter by tag")
    fp.add_argument("--type", help="Filter by node type (Paper/P-Note/C-Note/M-Note/Tag)")
    fp.add_argument("--format", choices=["table", "json"], default="table")

    # kg view
    vp = sub.add_parser("view", help="Open interactive D3.js force graph in browser")
    vp.add_argument("--paper", help="Paper UID — show ego graph (default: full graph)")
    vp.add_argument("--tag", help="Show all papers for a tag as graph")
    vp.add_argument("--depth", type=int, default=2, help="BFS depth for ego graph (default 2)")
    vp.add_argument("--max-nodes", type=int, default=300, help="Max nodes to render (default 300)")
    vp.add_argument("--open", action="store_true", default=True, help="Open in default browser (default: on)")
    vp.add_argument("--no-open", dest="open", action="store_false", help="Write HTML to stdout instead of opening browser")

    return p


def _run_kg(args: argparse.Namespace) -> int:
    kg = KGManager()

    if args.kg_cmd == "stats":
        stats = kg.stats()
        print("=== Knowledge Graph Stats ===")
        print(f"Total nodes : {stats['total_nodes']}")
        print(f"Total edges : {stats['total_edges']}")
        print("\nNodes by type:")
        for ntype, cnt in sorted(stats["nodes_by_type"].items()):
            print(f"  {ntype:12s}: {cnt:6d}")
        print("\nEdges by relation:")
        for rtype, cnt in sorted(stats["edges_by_type"].items()):
            print(f"  {rtype:12s}: {cnt:6d}")
        return 0

    elif args.kg_cmd == "graph":
        paper_node = kg.get_node_by_entity("Paper", args.paper_id)
        if paper_node is None:
            print(f"Paper '{args.paper_id}' not found in KG.")
            return 1
        neighbors = kg.find_neighbors(paper_node["id"], depth=args.depth)
        if args.format == "json":
            out = {
                "center": paper_node,
                "neighbors": [{"node": n, "edge": e, "depth": d} for n, e, d in neighbors],
            }
            print(json.dumps(out, option=json.OPT_INDENT_2).decode())
        else:
            print(f"=== KG Graph for '{args.paper_id}' (depth={args.depth}) ===")
            print(f"Center: [{paper_node['type']}] {paper_node['label']}")
            print(f"\n{len(neighbors)} neighbor(s):")
            for node, edge, depth in sorted(neighbors, key=lambda x: x[2]):
                print(f"  [depth={depth}] {node['type']:8s} | {edge['relation_type']:12s} | {node['label'][:50]}")
        return 0

    elif args.kg_cmd == "path":
        nodeA = kg.get_node_by_entity("Paper", args.idA) or kg.get_node(args.idA)
        nodeB = kg.get_node_by_entity("Paper", args.idB) or kg.get_node(args.idB)
        if not nodeA:
            print(f"Node A ('{args.idA}') not found.")
            return 1
        if not nodeB:
            print(f"Node B ('{args.idB}') not found.")
            return 1
        path = kg.find_shortest_path(nodeA["id"], nodeB["id"])
        if path is None:
            print(f"No path found between '{args.idA}' and '{args.idB}'.")
        else:
            print(f"Path ({len(path)} hops):")
            for i, nid in enumerate(path):
                node = kg.get_node(nid)
                label = node["label"][:50] if node else nid
                print(f"  {i+1}. [{node['type'] if node else '?'}] {label}")
        return 0

    elif args.kg_cmd == "rebuild":
        from pathlib import Path
        papers_json = args.papers_json
        if not papers_json:
            candidates = [Path("papers.json"), Path("data/papers.json")]
            for c in candidates:
                if c.exists():
                    papers_json = str(c)
                    break
        if not papers_json:
            print("papers.json not found. Use --papers-json to specify.")
            return 1
        from kg.integration import KGIntegration
        integ = KGIntegration(kg)
        print(f"Rebuilding KG from {papers_json} ...")
        integ.rebuild_from_papers_json(papers_json, incremental=args.incremental)
        stats = kg.stats()
        print(f"Done: {stats['total_nodes']} nodes, {stats['total_edges']} edges.")
        return 0

    elif args.kg_cmd == "search":
        nodes = []
        if args.tag:
            nodes = kg.find_papers_by_tag(args.tag)
        elif args.type:
            nodes = kg.get_all_nodes(node_type=args.type)
        else:
            nodes = kg.get_all_nodes()
        if not nodes:
            print("No nodes found.")
            return 0
        if args.format == "json":
            print(json.dumps(nodes, option=json.OPT_INDENT_2).decode())
        else:
            print(f"{len(nodes)} node(s):")
            for n in nodes[:50]:
                print(f"  [{n['type']:8s}] {n['label'][:55]}")
            if len(nodes) > 50:
                print(f"  ... and {len(nodes)-50} more")
        return 0

    elif args.kg_cmd == "view":
        return _run_kg_view(args)

    print(f"Unknown kg subcommand: {args.kg_cmd}")
    return 1


def _run_kg_view(args: argparse.Namespace) -> int:
    """Render interactive D3.js force graph and open in browser (or write to stdout)."""
    import tempfile, json, webbrowser, os
    from pathlib import Path
    from viz.d3_renderer import D3ForceGraph

    kg = KGManager()
    renderer = D3ForceGraph(kg)

    paper_uids = [args.paper] if args.paper else None
    tag = args.tag if hasattr(args, "tag") else None

    graph_data = renderer.to_json(
        paper_uids=paper_uids,
        tag=tag,
        max_nodes=args.max_nodes,
    )

    if not graph_data["nodes"]:
        print("No nodes found in the selected scope.", file=__import__("sys").stderr)
        return 1

    # Load template
    template_path = Path(__file__).parent.parent.parent / "viz" / "templates" / "kg_viz_template_d3.html"
    html_content = template_path.read_text(encoding="utf-8")

    # Inject data — JSON-escape for safe JS embedding
    nodes_json = json.dumps(graph_data["nodes"], ensure_ascii=False)
    links_json = json.dumps(graph_data["links"], ensure_ascii=False)
    html_content = html_content.replace('"INJECT_NODES"', nodes_json)
    html_content = html_content.replace('"INJECT_LINKS"', links_json)

    if not args.open:
        __import__("sys").stdout.write(html_content)
        return 0

    with tempfile.NamedTemporaryFile(
        mode="w", suffix=".html", delete=False, encoding="utf-8"
    ) as f:
        f.write(html_content)
        tmp_path = f.name

    webbrowser.open(f"file://{tmp_path}")
    print(f"Opened {len(graph_data['nodes'])} nodes, {len(graph_data['links'])} edges in browser.")
    print(f"(HTML also saved to: {tmp_path})")
    return 0
