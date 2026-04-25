"""CLI command: search."""
from __future__ import annotations

import argparse
import csv
import sys as _sys

import orjson as json

from cli._shared import get_db

from cli._shared import (
    Colors,
    colored,
    print_success,
)


def _build_search_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "search",
        help="Search indexed papers",
        prog="airos search",
        description="Search papers by query, year, source, tag, or parse status.",
        epilog="""\
Examples:
  %(prog)s "attention mechanism"                     # basic query
  %(prog)s --year 2024 --limit 20                 # recent papers
  %(prog)s --tag LLM --tag Agent --format json    # by tags, JSON output
  %(prog)s --source arxiv --sort year --order desc # by source, newest first
  %(prog)s --status parsed --format csv            # export parsed papers as CSV""",
    )

    p.add_argument("query", nargs="?", default="", help="Search query (optional)")

    p.add_argument("--limit", type=int, default=10, help="Max results")

    p.add_argument("--offset", type=int, default=0, help="Skip N results")

    p.add_argument("--format", choices=["table", "json", "csv"], default="table")

    p.add_argument("--source", default="", help="Filter by source (e.g. arxiv, doi)")

    p.add_argument("--year", type=int, default=0, help="Filter by year")

    p.add_argument("--tag", dest="tags", action="append", default=[], help="Filter by tag (repeatable)")

    p.add_argument("--status", default="", help="Filter by parse status")

    p.add_argument("--sort", choices=["relevance", "year", "title"], default="relevance")

    return p


def _run_search(args: argparse.Namespace) -> int:
    db = get_db()
    db.init()

    results, total = db.search_papers(
        query=args.query or "",
        limit=args.limit,
        offset=args.offset,
        source=args.source or None,
        parse_status=args.status or None,
        date_from=f"{args.year}-01-01" if args.year > 0 else None,
    )

    if args.format == "json":
        out = []
        for r in results:
            out.append({
                "paper_id": r.paper_id,
                "title": r.title,
                "authors": r.authors,
                "published": r.published,
                "primary_category": r.primary_category,
                "score": round(r.score, 3) if r.score else None,
                "snippet": r.snippet,
                "source": r.source,
                "abs_url": r.abs_url,
                "pdf_url": r.pdf_url,
                "parse_status": r.parse_status,
            })
        print(json.dumps({"total": total, "results": out}, option=json.OPT_INDENT_2).decode())

    elif args.format == "csv":
        writer = csv.writer(_sys.stdout)
        writer.writerow(["paper_id", "title", "authors", "published", "primary_category", "score", "snippet", "source", "abs_url", "parse_status"])
        for r in results:
            writer.writerow([r.paper_id, r.title, r.authors, r.published, r.primary_category,
                             round(r.score, 3) if r.score else "", r.snippet, r.source, r.abs_url, r.parse_status or ""])

    else:
        print_success(f"Found {total} papers, showing {len(results)}:")
        print()
        for r in results:
            score_str = f"[{r.score:.2f}]" if r.score else "     "
            print(f"  {colored(score_str, Colors.BOLD)} {r.title}")
            print(f"         {colored(r.authors, Colors.OKBLUE)}")
            print(f"         {r.published} | {colored(r.source, Colors.OKGREEN)} | {r.primary_category}")
            if r.snippet:
                print(f"         ...{r.snippet}...")
            print()

    return 0
