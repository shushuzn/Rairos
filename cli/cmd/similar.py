"""CLI command: similar."""
from __future__ import annotations

import argparse
import sys

from cli._shared import get_db
from cli._shared import (
    Colors, colored, print_success, print_error, print_warning, print_info, print_header,
)


def _build_similar_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "similar",
        help="Similarity search: table output or interactive D3.js graph",
    )
    sub = p.add_subparsers(dest="similar_subcmd", help="Similarity subcommands")

    # similar run (text/JSON table output)
    p_run = sub.add_parser(
        "run",
        help="Text/JSON table of similar papers",
    )
    p_run.add_argument(
        "paper_id",
        nargs="?",
        default="",
        help="Paper ID (e.g. 2301.001)",
    )
    p_run.add_argument(
        "--threshold", type=float, default=0.85,
        help="Minimum cosine similarity (default: 0.85)",
    )
    p_run.add_argument(
        "--limit", type=int, default=10,
        help="Max similar papers to return (default: 10)",
    )
    p_run.add_argument(
        "--format", choices=["table", "json"], default="table",
        help="Output format (default: table)",
    )

    # similar view (D3.js interactive visualization)
    vp = sub.add_parser(
        "view",
        help="Open interactive D3.js similarity graph in browser",
    )
    vp.add_argument(
        "paper_id",
        help="Paper ID (e.g. 2301.001)",
    )
    vp.add_argument(
        "--threshold", type=float, default=0.85,
        help="Minimum cosine similarity (default: 0.85)",
    )
    vp.add_argument(
        "--limit", type=int, default=20,
        help="Max similar papers to show (default: 20)",
    )
    vp.add_argument(
        "--open", action="store_true", default=True,
        help="Open in default browser (default: on)",
    )
    vp.add_argument(
        "--no-open", dest="open", action="store_false",
        help="Write HTML to stdout instead of opening browser",
    )
    return p


def _run_similar(args: argparse.Namespace) -> int:
    """Dispatch to subcommands: run (text/JSON) or view (D3.js)."""
    subcmd = getattr(args, "similar_subcmd", "run")
    if subcmd == "view":
        return _run_similar_view(args)
    return _run_similar_text(args)


def _run_similar_text(args: argparse.Namespace) -> int:
    """Text/JSON table of similar papers."""
    db = get_db()
    db.init()

    if not args.paper_id:
        stats = db.get_embedding_stats()
        papers_without_emb = stats.get("total_with_text", 0) - stats.get("with_embedding", 0)
        print("Semantic similarity search requires embeddings.")
        print(f"Database stats: {stats.get('with_embedding', 0)} papers have embeddings "
              f"({papers_without_emb} still need them).")
        print()
        print("To generate embeddings for papers without them, use:")
        print("  ai_research_os research --generate")
        return 1

    if not db.paper_exists(args.paper_id):
        print(f"Paper {args.paper_id!r} not found in database", file=sys.stderr)
        return 1

    paper = db.get_paper(args.paper_id)
    sims = db.find_similar(args.paper_id, threshold=args.threshold, limit=args.limit)

    if not sims:
        print(f"No similar papers found for {args.paper_id!r}")
        return 0

    if args.format == "json":
        import json
        result = [{"id": p.id, "title": p.title, "score": float(s)} for p, s in sims]
        print(json.dumps(result, indent=2))
    else:
        print(f"Similar papers to {args.paper_id!r} ({paper.title[:60]}):")
        for sim_paper, score in sims:
            print(f"  [{score:.4f}] {sim_paper.id}  {sim_paper.title[:70]}")

    return 0


def _run_similar_view(args: argparse.Namespace) -> int:
    """Render interactive D3.js similarity graph and open in browser."""
    import tempfile, json, webbrowser
    from pathlib import Path
    from viz.d3_renderer import D3ForceGraph

    db = get_db()
    db.init()

    if not db.paper_exists(args.paper_id):
        print(f"Paper {args.paper_id!r} not found in database", file=sys.stderr)
        return 1

    renderer = D3ForceGraph()
    graph_data = renderer.to_similar_json(
        paper_id=args.paper_id,
        threshold=args.threshold,
        max_nodes=args.limit,
    )

    if not graph_data["nodes"]:
        print(f"No similar papers found for '{args.paper_id}'. "
              "Ensure the paper has an embedding and similar papers exist.", file=sys.stderr)
        return 1

    # Load template
    template_path = Path(__file__).parent.parent.parent / "viz" / "templates" / "similar_viz_template_d3.html"
    html_content = template_path.read_text(encoding="utf-8")

    # Inject data
    nodes_json = json.dumps(graph_data["nodes"], ensure_ascii=False)
    links_json = json.dumps(graph_data["links"], ensure_ascii=False)
    html_content = html_content.replace("INJECT_NODES", nodes_json)
    html_content = html_content.replace("INJECT_LINKS", links_json)
    html_content = html_content.replace("INJECT_ROOT_ID", f'"{graph_data["root"]}"')

    if not args.open:
        sys.stdout.write(html_content)
        return 0

    with tempfile.NamedTemporaryFile(
        mode="w", suffix=".html", delete=False, encoding="utf-8"
    ) as f:
        f.write(html_content)
        tmp_path = f.name

    webbrowser.open(f"file://{tmp_path}")
    sim_count = len([n for n in graph_data["nodes"] if not n.get("is_root")])
    print(f"Opened similarity graph for '{args.paper_id}' in browser.")
    print(f"  {len(graph_data['nodes'])} papers · {sim_count} similar (threshold: {args.threshold})")
    print(f"(HTML also saved to: {tmp_path})")
    return 0
