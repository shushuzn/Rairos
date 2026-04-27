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
    p.add_argument(
        "--route", "-r",
        action="store_true",
        help="Use semantic routing to delegate to specialized commands",
    )
    return p


def _run_ask(args: argparse.Namespace) -> int:
    """Run ask command."""
    query = " ".join(args.query)

    # Semantic routing: delegate to specialized commands instead of RAG chat
    if args.route:
        from llm.semantic_router import SemanticRouter
        router = SemanticRouter()
        try:
            route = router.route(query)
        except Exception as e:
            print_error(f"Routing failed: {e}")
            return 1

        # QUESTION_ANSWER falls through to RAG chat; all others delegate
        if route.query_type.value != "question_answer":
            outputs = router.execute(route, query, exec_all=True)
            for name, out in outputs.items():
                print(f"=== {name} ===")
                print(out if out.strip() else "[no output]")
                print()
            return 0

    db = get_db()
    db.init()

    insight_manager = None if args.no_insights else InsightManager()
    chat = ResearchChat(db=db, insight_manager=insight_manager)

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
