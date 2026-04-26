"""CLI command: dashboard — Research progress dashboard."""
from __future__ import annotations

import argparse

from cli._shared import print_info, get_db
from llm.dashboard import Dashboard


def _build_dashboard_parser(subparsers) -> argparse.ArgumentParser:
    """Build the dashboard subcommand parser."""
    p = subparsers.add_parser(
        "dashboard",
        help="Research progress dashboard",
        description="View aggregated research progress summary.",
    )
    p.add_argument(
        "--questions", "-q",
        action="store_true",
        help="Focus on questions",
    )
    p.add_argument(
        "--experiments", "-e",
        action="store_true",
        help="Focus on experiments",
    )
    p.add_argument(
        "--papers", "-p",
        action="store_true",
        help="Focus on papers",
    )
    p.add_argument(
        "--json", "-j",
        action="store_true",
        help="JSON output",
    )
    p.add_argument(
        "--export-md",
        type=str,
        help="Export as Markdown file",
    )
    p.add_argument(
        "--no-papers",
        action="store_true",
        help="Skip paper statistics (faster)",
    )
    return p


def _run_dashboard(args: argparse.Namespace) -> int:
    """Run dashboard command."""
    db = get_db()
    db.init()

    dashboard = Dashboard(db=db)
    data = dashboard.collect(include_papers=not args.no_papers)

    if args.json:
        print(dashboard.render_json(data))
    elif args.export_md:
        # Markdown export
        lines = ["# Research Dashboard", ""]
        lines.append(f"Generated: {data.generated_at[:10]}", "")
        lines.append(dashboard.render_text(data))
        with open(args.export_md, 'w', encoding='utf-8') as f:
            f.write('\n'.join(lines))
        print(f"✓ Exported to {args.export_md}")
    else:
        print()
        print(dashboard.render_text(data))

    return 0
