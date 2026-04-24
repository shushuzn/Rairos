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
        help="Find papers similar to a given paper using embedding similarity",
    )
    p.add_argument(
        "paper_id",
        nargs="?",
        default="",
        help="Paper ID (e.g. 2301.001)",
    )
    p.add_argument(
        "--threshold", type=float, default=0.85,
        help="Minimum cosine similarity (default: 0.85)",
    )
    p.add_argument(
        "--limit", type=int, default=10,
        help="Max similar papers to return (default: 10)",
    )
    p.add_argument(
        "--format", choices=["table", "json"], default="table",
        help="Output format (default: table)",
    )
    return p


def _run_similar(args: argparse.Namespace) -> int:
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
