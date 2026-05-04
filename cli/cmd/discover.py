"""CLI command: discover — find hidden cross-domain patterns.

Usage:
    airos discover     Run pattern discovery on Gene Pool + market data
"""

from __future__ import annotations

import argparse
from cli._shared import print_error, print_success


def _build_discover_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "discover",
        help="Discover hidden patterns across events, markets, and research",
        description="Cross-reference Gene Pool capsules, market data, and "
        "event timestamps to find correlations the system learned autonomously.",
    )
    p.set_defaults(func=_run_discover)
    return p


def _run_discover(args) -> None:
    from llm.discover import discover, render_discovery

    try:
        result = discover()
        print(render_discovery(result))
        if result.get("patterns_discovered", 0) > 0:
            print_success(f"{result['patterns_discovered']} new patterns discovered")
    except Exception as e:
        print_error(f"Discovery failed: {e}")
