"""CLI command: digest — Weekly research digest."""
from __future__ import annotations

import argparse

from cli._shared import print_info
from llm.weekly_digest import WeeklyDigest


def _build_digest_parser(subparsers) -> argparse.ArgumentParser:
    """Build the digest subcommand parser."""
    p = subparsers.add_parser(
        "digest",
        help="Weekly research digest",
        description="Generate weekly research summary.",
    )
    p.add_argument(
        "--days", "-d",
        type=int,
        default=7,
        help="Number of days to summarize (default: 7)",
    )
    p.add_argument(
        "--last-week",
        action="store_true",
        help="Summarize last 7 days",
    )
    p.add_argument(
        "--markdown", "-m",
        action="store_true",
        help="Output as Markdown",
    )
    p.add_argument(
        "--export",
        type=str,
        help="Export to file",
    )
    return p


def _run_digest(args: argparse.Namespace) -> int:
    """Run digest command."""
    digest = WeeklyDigest()

    days = 14 if args.last_week else (args.days or 7)
    data = digest.collect_week_data(days=days)

    if args.markdown:
        output = digest.render_markdown(data)
    else:
        output = digest.generate_summary(data)

    if args.export:
        with open(args.export, 'w', encoding='utf-8') as f:
            f.write(output)
        print_info(f"✓ Exported to {args.export}")
    else:
        print()
        print(output)

    return 0
