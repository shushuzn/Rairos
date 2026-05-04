"""CLI command: signal — pattern-based event signal analysis.

Usage:
    airos signal 富查伊拉     Analyze event against Gene Pool patterns
    airos signal 石油         Check oil-related signal
    airos signal 美联储       Check Fed-related signal
"""

from __future__ import annotations

import argparse
from cli._shared import print_error


def _build_signal_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "signal",
        help="Analyze an event against historical Gene Pool patterns",
        description="Match a live event keyword against historical capsules, "
        "cross-reference market data, and generate a signal level.",
    )
    p.add_argument("keyword", help="Event keyword (e.g. 富查伊拉, 石油, 美联储)")
    p.set_defaults(func=_run_signal)
    return p


def _run_signal(args) -> None:
    from llm.signal import signal, render_signal

    try:
        r = signal(args.keyword)
        print(render_signal(r))
    except Exception as e:
        print_error(f"Signal analysis failed: {e}")
