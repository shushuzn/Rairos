"""CLI command: compare — Compare multiple papers side-by-side."""
from __future__ import annotations

import argparse

from cli._shared import get_db, print_info, print_error
from llm.paper_comparison import PaperComparator


def _build_compare_parser(subparsers) -> argparse.ArgumentParser:
    """Build the compare subcommand parser."""
    p = subparsers.add_parser(
        "compare",
        help="Compare papers",
        description="Compare multiple papers side-by-side.",
    )
    p.add_argument("paper_ids", nargs="+", help="Paper IDs to compare")
    p.add_argument(
        "--aspect", "-a",
        action="append",
        choices=["methods", "datasets", "metrics", "authors", "year", "abstract"],
        help="Aspects to compare (can be repeated)",
    )
    p.add_argument("--markdown", "-m", action="store_true", help="Output as Markdown")
    p.add_argument("--diff", help="Generate diff between two papers (paper_id_a:paper_id_b)")
    return p


def _run_compare(args: argparse.Namespace) -> int:
    """Run compare command."""
    db = get_db()
    db.init()

    comparator = PaperComparator(db=db)

    # Diff mode
    if args.diff:
        parts = args.diff.split(':')
        if len(parts) != 2:
            print_error("Diff format: <paper_id_a>:<paper_id_b>")
            return 1

        pid_a, pid_b = parts
        paper_a = db.get_paper(pid_a) if hasattr(db, 'get_paper') else None
        paper_b = db.get_paper(pid_b) if hasattr(db, 'get_paper') else None

        if not paper_a:
            print_error(f"Paper [{pid_a}] not found")
            return 1
        if not paper_b:
            print_error(f"Paper [{pid_b}] not found")
            return 1

        print(comparator.render_diff(paper_a, paper_b))
        return 0

    # Comparison mode
    papers = []
    for pid in args.paper_ids:
        paper = db.get_paper(pid) if hasattr(db, 'get_paper') else None
        if paper:
            papers.append(pid)
        else:
            print_warning(f"Paper [{pid}] not found, skipping")

    if len(papers) < 2:
        print_error("Need at least 2 valid paper IDs")
        return 1

    print_info(f"Comparing {len(papers)} papers...")

    result = comparator.compare(papers, aspects=args.aspect or None)

    if args.markdown:
        print(comparator.render_markdown(result))
    else:
        print(comparator.render_text(result))

    return 0


def print_warning(msg: str):
    """Print warning message."""
    print(f"⚠️  {msg}")
