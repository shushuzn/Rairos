"""CLI command: narrative — Research Narrative Tracker."""

from __future__ import annotations

import argparse

from cli._shared import print_info, print_error


def _build_narrative_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "narrative",
        help="Research Narrative Tracker — unified view across gaps, hypotheses, experiments",
        description="Aggregate all research state into narrative threads with phase tracking and publication readiness scoring.",
    )
    p.add_argument(
        "action",
        nargs="?",
        default="list",
        choices=["list", "show", "track", "update", "note", "dashboard"],
        help="Action to perform",
    )
    p.add_argument("--id", "-i", help="Narrative ID")
    p.add_argument("--title", "-t", help="Narrative title")
    p.add_argument("--text", "-T", help="Note text")
    p.add_argument("--hypothesis", "-H", help="Hypothesis ID to link")
    p.add_argument("--gap", "-g", help="Gap ID to link")
    p.add_argument("--experiment", "-e", help="Experiment ID to link")
    p.set_defaults(func=lambda a: _run_narrative(a))
    return p


def _run_narrative(args: argparse.Namespace) -> int:
    """Run narrative command with lazy import to avoid missing module crash."""
    try:
        from llm.research_narrative_tracker import (
            ResearchNarrativeTracker,
            render_dashboard,
        )
    except ImportError:
        print_error("Research Narrative Tracker module not available")
        print_error("  The llm.research_narrative_tracker module was removed.")
        print_info("  Please use the Rust CLI equivalent or check project status.")
        return 1

    tracker = ResearchNarrativeTracker()

    if args.action == "dashboard":
        dashboard = tracker.get_dashboard()
        print(render_dashboard(dashboard))
    elif args.action == "list":
        narratives = tracker.list_narratives()
        for n in narratives:
            print(f"  [{n.id}] {n.title}")
    elif args.action == "show":
        if not args.id:
            print_error("Usage: narrative show --id <narrative_id>")
            return 1
        n = tracker.get_narrative(args.id)
        if n:
            print(f"Title: {n.title}")
            print(f"ID: {n.id}")
        else:
            print_error(f"Narrative [{args.id}] not found")
    else:
        print_info("Narrative command — use: list, show, dashboard")
    return 0
