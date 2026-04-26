"""CLI command: hypothesize — Generate research hypotheses from gaps."""
from __future__ import annotations

import argparse

from cli._shared import get_db, print_info
from llm.hypothesis_generator import HypothesisGenerator


def _build_hypothesize_parser(subparsers) -> argparse.ArgumentParser:
    """Build the hypothesize subcommand parser."""
    p = subparsers.add_parser(
        "hypothesize",
        help="Generate testable research hypotheses from gaps",
        description="Generate research hypotheses with experiment designs and risk assessments.",
    )
    p.add_argument(
        "topic",
        nargs="?",
        default=None,
        help="Research topic for hypothesis generation",
    )
    p.add_argument(
        "--gap", "-g",
        type=str,
        default="",
        help="Gap context from gap analysis",
    )
    p.add_argument(
        "--trend", "-t",
        type=str,
        default="",
        help="Trend context from trend analysis",
    )
    p.add_argument(
        "--story", "-s",
        type=str,
        default="",
        help="Story context from story weaving",
    )
    p.add_argument(
        "--no-llm",
        action="store_true",
        help="Disable LLM enhancement",
    )
    p.add_argument(
        "--creative",
        action="store_true",
        help="Generate creative cross-domain hypotheses",
    )
    p.add_argument(
        "--json", "-j",
        action="store_true",
        help="Output as JSON",
    )
    p.add_argument(
        "--model", "-M",
        type=str,
        default=None,
        help="LLM model to use",
    )
    p.add_argument(
        "--top", "-n",
        type=int,
        default=5,
        help="Number of hypotheses to generate (default: 5)",
    )
    return p


def _run_hypothesize(args: argparse.Namespace) -> int:
    """Run hypothesis generation command."""
    db = get_db()
    db.init()

    generator = HypothesisGenerator(db=db)

    if not args.topic:
        print("❌ 请提供 topic")
        return 1

    print_info(f"🎯 Generating hypotheses for: {args.topic}")

    result = generator.generate(
        topic=args.topic,
        gap_context=args.gap,
        trend_context=args.trend,
        story_context=args.story,
        use_llm=not args.no_llm,
        model=args.model,
        creative=args.creative,
    )

    if args.json:
        print(generator.render_json(result))
    else:
        print()
        print(generator.render_result(result))

    return 0
