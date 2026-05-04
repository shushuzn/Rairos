"""CLI command: intel — unified intelligence report.

Usage:
    airos intel                    Full situation report
    airos intel --topic "伊朗"      Focus on a topic
    airos intel --verbose           Detailed breakdown
"""

from __future__ import annotations

import argparse
from cli._shared import print_error


def _build_intel_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "intel",
        help="Unified intelligence report from all data sources",
        description="Aggregate Jin10 news, market quotes, Gene Pool state, "
        "watch daemon status, and related academic papers into one report.",
    )
    p.add_argument("--topic", "-t", type=str, default="",
        help="Focus topic (default: global)")
    p.add_argument("--verbose", "-v", action="store_true",
        help="Include detailed breakdowns")
    p.set_defaults(func=_run_intel)
    return p


def _run_intel(args) -> None:
    from llm.intelligence import intelligence, render_report

    topic = getattr(args, "topic", "")
    verbose = getattr(args, "verbose", False)

    try:
        report = intelligence(topic=topic, verbose=verbose)
        print(render_report(report))
    except Exception as e:
        print_error(f"Intel failed: {e}")
