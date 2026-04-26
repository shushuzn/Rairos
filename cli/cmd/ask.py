"""CLI command: ask — Research chat."""
from __future__ import annotations

import argparse

from cli._shared import get_db, print_info, print_error
from llm.research_chat import ResearchChat
from llm.insight_cards import InsightManager


def _build_ask_parser(subparsers) -> argparse.ArgumentParser:
    """Build the ask subcommand parser."""
    p = subparsers.add_parser(
        "ask",
        help="Research chat",
        description="Ask research questions with context awareness.",
    )
    p.add_argument(
        "query",
        nargs="+",
        help="Your research question",
    )
    p.add_argument(
        "--context", "-c",
        help="Limit to papers with tag/topic",
    )
    p.add_argument(
        "--verbose", "-v",
        action="store_true",
        help="Show retrieved context",
    )
    p.add_argument(
        "--no-insights",
        action="store_true",
        help="Skip insights in context",
    )
    p.add_argument(
        "--max-papers",
        type=int,
        default=10,
        help="Max papers to retrieve (default: 10)",
    )
    return p


def _run_ask(args: argparse.Namespace) -> int:
    """Run ask command."""
    db = get_db()
    db.init()

    insight_manager = None if args.no_insights else InsightManager()

    chat = ResearchChat(db=db, insight_manager=insight_manager)

    query = " ".join(args.query)
    print_info(f"\n🔍 {query}\n")

    # Build context
    ctx = chat.build_context(query, topic_hint=args.context)

    if args.verbose:
        print(f"[Context] Topic: {ctx.topic}")
        print(f"[Context] Papers: {len(ctx.papers)}")
        print(f"[Context] Insights: {len(ctx.insights)}")
        if ctx.papers:
            print("\n[Retrieved Papers]")
            for p in ctx.papers[:5]:
                print(f"  - {p.title[:60]}")
        print()

    # Generate response
    response = chat.chat(query, context=ctx)
    print(response)

    return 0
