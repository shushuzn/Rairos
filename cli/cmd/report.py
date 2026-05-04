"""CLI command: report — generate live situation report.

Usage:
    rairos report     Generate fresh report from current system state
"""

from __future__ import annotations

import argparse
from cli._shared import print_success, print_error


def _build_report_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "report",
        help="Generate live situation report from current system state",
        description="Always produces a fresh report with latest Gene Pool data.",
    )
    p.set_defaults(func=_run_report)
    return p


def _run_report(args) -> None:
    from llm.report import generate, save

    try:
        report = generate()
        path = save()
        print(report)
        print_success(f"Report saved to {path}")
    except Exception as e:
        print_error(f"Report generation failed: {e}")
