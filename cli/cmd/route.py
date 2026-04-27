"""CLI command: route — semantic command routing."""
from __future__ import annotations

import argparse
import json
import sys

from cli._shared import print_info, print_warning
from llm.semantic_router import SemanticRouter


def _build_route_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "route",
        help="Route a natural-language query to the appropriate CLI command",
        description=(
            "Classify a research query and optionally execute the routed command(s). "
            "Uses LLM classification with graceful fallback to embedding similarity "
            "and keyword matching."
        ),
    )
    p.add_argument(
        "query",
        nargs="*",
        default=[],
        help="Research query to route",
    )
    p.add_argument(
        "--json", "-j",
        action="store_true",
        help="Output full route object as JSON",
    )
    p.add_argument(
        "--exec", "-e",
        action="store_true",
        help="Execute the routed command(s) and print outputs",
    )
    p.add_argument(
        "--all", "-a",
        action="store_true",
        help="Execute all routed commands (for multi-intent queries, implies --exec)",
    )
    p.add_argument(
        "--model", "-m",
        type=str,
        default=None,
        help="LLM model for classification (default: qwen3.5-plus)",
    )
    return p


def _run_route(args: argparse.Namespace) -> int:
    if not args.query:
        print_warning("Usage: airos route <query>")
        return 1

    query = " ".join(args.query)
    router = SemanticRouter(model=args.model)

    try:
        route = router.route(query)
    except Exception as e:
        print_warning(f"Routing failed: {e}")
        return 1

    exec_all = args.all or args.exec

    if args.json:
        print(json.dumps(route.to_dict(), indent=2, ensure_ascii=False))
        return 0

    # Human-readable output
    _print_route(route, query)

    if exec_all:
        print()
        try:
            outputs = router.execute(route, query, exec_all=True)
            for name, out in outputs.items():
                print(f"=== {name} ===")
                print(out if out.strip() else "[no output]")
                print()
        except Exception as e:
            print_warning(f"Execution failed: {e}")

    return 0


def _print_route(route, query: str) -> None:
    bar = "█" * int(route.confidence * 10) + "░" * (10 - int(route.confidence * 10))
    print(f"🔍 Query: {query}")
    print(f"   Type:          {route.query_type.value}")
    print(f"   Command:       {route.primary_command}")
    print(f"   Confidence:    {bar} {route.confidence:.0%}")
    if route.multi_intent:
        print(f"   Secondary:     {route.secondary_query_type.value if route.secondary_query_type else '—'}")
        print(f"   Sub-commands:  {' → '.join(route.sub_commands)}")
    if route.reasoning:
        print(f"   Reasoning:     {route.reasoning}")
