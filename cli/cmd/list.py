"""CLI command: list."""
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


def _build_list_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "list",
        help="List indexed papers",
        prog="airos list",
        description="List papers with filtering, sorting, and multiple output formats.",
        epilog="""\
Examples:
  %(prog)s                                    # recent 20 papers (default)
  %(prog)s --limit 50 --sort published --order desc  # all-time newest
  %(prog)s --year 2024 --tag LLM             # by year and tag
  %(prog)s --format json | jq '.[] | .title' # pipe to jq
  %(prog)s --format csv > papers.csv          # export to CSV""",
    )

    p.add_argument("--status", default="", help="Filter by parse status")

    p.add_argument("--year", type=int, default=0, help="Filter by year")

    p.add_argument("--tag", dest="tags", action="append", default=[], help="Filter by tag (repeatable)")

    p.add_argument("--limit", type=int, default=20, help="Max results")

    p.add_argument("--offset", type=int, default=0, help="Skip N results")

    p.add_argument("--format", choices=["table", "json", "csv"], default="table")

    p.add_argument(
        "--sort",
        choices=["added_at", "published", "title", "parse_status"],
        default="added_at",
        help="Sort field (default: added_at)",
    )

    p.add_argument("--order", choices=["asc", "desc"], default="desc", help="Sort order")

    return p


def _run_list(args: argparse.Namespace) -> int:
    db = get_db()
    db.init()
    papers, total = db.list_papers(
        parse_status=args.status or None,
        limit=args.limit,
        offset=args.offset,
        date_from=f"{args.year}-01-01" if args.year > 0 else None,
        sort_by=args.sort,
        sort_order=args.order,
    )

    if args.format == "csv":
        writer = csv.writer(_sys.stdout)
        writer.writerow(["id", "title", "authors", "published", "source", "primary_category", "parse_status", "added_at"])
        for p in papers:
            writer.writerow([p.id, p.title, p.authors, p.published, p.source,
                             p.primary_category, p.parse_status or "", p.added_at])

    elif args.format == "json":
        out = [{"paper_id": p.id, "title": p.title, "authors": p.authors,
                "published": p.published, "primary_category": p.primary_category,
                "source": p.source, "abs_url": p.abs_url} for p in papers]
        print(json.dumps(out, option=json.OPT_INDENT_2).decode())

    else:
        for p in papers:
            print(f"  {p.id:>5}  {p.published}  {p.source:<6}  {p.title}")

    return 0
