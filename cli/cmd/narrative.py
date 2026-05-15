     1|"""CLI command: narrative — Research Narrative Tracker."""
     2|
     3|from __future__ import annotations
     4|
     5|import argparse
     6|
     7|from cli._shared import print_info, print_error
     8|
     9|
    10|def _build_narrative_parser(subparsers) -> argparse.ArgumentParser:
    11|    p = subparsers.add_parser(
    12|        "narrative",
    13|        help="Research Narrative Tracker — unified view across gaps, hypotheses, experiments",
    14|        description="Aggregate all research state into narrative threads with phase tracking and publication readiness scoring.",
    15|    )
    16|    p.add_argument(
    17|        "action",
    18|        nargs="?",
    19|        default="list",
    20|        choices=["list", "show", "track", "update", "note", "dashboard"],
    21|        help="Action to perform",
    22|    )
    23|    p.add_argument("--id", "-i", help="Narrative ID")
    24|    p.add_argument("--title", "-t", help="Narrative title")
    25|    p.add_argument("--text", "-T", help="Note text")
    26|    p.add_argument("--hypothesis", "-H", help="Hypothesis ID to link")
    27|    p.add_argument("--gap", "-g", help="Gap ID to link")
    28|    p.add_argument("--experiment", "-e", help="Experiment ID to link")
    29|    p.set_defaults(func=lambda a: _run_narrative(a))
    30|    return p
    31|
    32|
    33|def _run_narrative(args: argparse.Namespace) -> int:
    34|    """Run narrative command with lazy import to avoid missing module crash."""
# [LEGACY] Research narrative tracker — depends on llm/research/narrative_tracker.py

    35|    try:
    36|        from llm.research_narrative_tracker import (
    37|            ResearchNarrativeTracker,
    38|            render_dashboard,
    39|        )
    40|    except ImportError:
    41|        print_error("Research Narrative Tracker module not available")
    42|        print_error("  The llm.research_narrative_tracker module was removed.")
    43|        print_info("  Please use the Rust CLI equivalent or check project status.")
    44|        return 1
    45|
    46|    tracker = ResearchNarrativeTracker()
    47|
    48|    if args.action == "dashboard":
    49|        dashboard = tracker.get_dashboard()
    50|        print(render_dashboard(dashboard))
    51|    elif args.action == "list":
    52|        narratives = tracker.list_narratives()
    53|        for n in narratives:
    54|            print(f"  [{n.id}] {n.title}")
    55|    elif args.action == "show":
    56|        if not args.id:
    57|            print_error("Usage: narrative show --id <narrative_id>")
    58|            return 1
    59|        n = tracker.get_narrative(args.id)
    60|        if n:
    61|            print(f"Title: {n.title}")
    62|            print(f"ID: {n.id}")
    63|        else:
    64|            print_error(f"Narrative [{args.id}] not found")
    65|    else:
    66|        print_info("Narrative command — use: list, show, dashboard")
    67|    return 0
    68|
