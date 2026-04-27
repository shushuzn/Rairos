"""CLI command: friction — Research friction report."""
from __future__ import annotations

import argparse
from typing import Optional

from llm.friction_tracker import FrictionTracker, FrictionType

from cli._shared import (
    Colors,
    colored,
    print_error,
    print_info,
    print_success,
    print_warning,
)


def _build_friction_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "friction",
        help="Show research friction (bottlenecks) report",
        description="Detect and report research friction points — commands that fail, "
                    "workflows that get abandoned, searches that come up empty.",
    )
    p.add_argument(
        "--type", "-t",
        dest="friction_type",
        choices=["command", "workflow", "retrieval", "cognitive", "navigation"],
        default=None,
        help="Filter by friction type",
    )
    p.add_argument(
        "--days", "-d",
        type=int,
        default=30,
        help="Time window in days (default: 30)",
    )
    p.add_argument(
        "--json", "-j",
        action="store_true",
        help="Output as JSON",
    )
    p.add_argument(
        "--limit", "-n",
        type=int,
        default=20,
        help="Max events to show (default: 20)",
    )
    return p


def run(args: argparse.Namespace) -> int:
    """Run the friction command."""
    tracker = FrictionTracker()

    if args.friction_type:
        ftype = FrictionType(args.friction_type)
    else:
        ftype = None

    summary = tracker.get_summary(since_days=args.days)
    events = tracker.get_events(
        friction_type=ftype,
        since_days=args.days,
        limit=args.limit,
    )

    if args.json:
        import json
        print(json.dumps({"summary": summary, "events": [e.to_dict() for e in events]}, indent=2))
        return 0

    # Human-readable output
    print()
    print(colored("═" * 60, Colors.HEADER))
    print(colored("  Research Friction Report", Colors.BOLD + Colors.HEADER))
    print(colored(f"  Last {args.days} days", Colors.OKBLUE))
    print(colored("═" * 60, Colors.HEADER))
    print()

    if summary["total_events"] == 0:
        print_info("No friction events recorded yet.")
        print("  Run some commands — friction is tracked automatically in the background.")
        print()
        return 0

    print_success(f"Total events: {summary['total_events']}")
    print(f"  Abandon rate: {summary['abandon_rate']:.1%}")
    print()

    # By type
    if summary["by_type"]:
        print(colored("By Type:", Colors.BOLD))
        for t, count in sorted(summary["by_type"].items(), key=lambda x: -x[1]):
            bar = "█" * min(count, 30)
            print(f"  {t:<12} {bar} {count}")
        print()

    # By severity
    if summary["by_severity"]:
        print(colored("By Severity:", Colors.BOLD))
        for s, count in sorted(summary["by_severity"].items(), key=lambda x: -x[1]):
            label = getattr(FrictionType, s.upper(), s)
            print(f"  {s:<10} {count}")
        print()

    # Top commands
    if summary["top_commands"]:
        print(colored("Top Friction Commands:", Colors.BOLD))
        for cmd, count in summary["top_commands"]:
            print(f"  {cmd:<20} {count} events")
        print()

    # Recent events
    if events:
        print(colored(f"Recent Events (last {len(events)}):", Colors.BOLD))
        for e in events[: args.limit]:
            ts = e.timestamp[:10] if len(e.timestamp) >= 10 else e.timestamp
            status = colored("[ABANDONED]", Colors.FAIL) if e.abandoned else ""
            print(f"  {ts}  {e.friction_type:<12} {e.command or '-':<15} {e.error or e.notes[:40] or '-'} {status}")

    print()
    return 0
