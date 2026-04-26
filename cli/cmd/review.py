"""CLI command: review — Generate literature review."""
from __future__ import annotations

import argparse

from cli._shared import get_db, print_info
from llm.review_generator import ReviewGenerator


def _build_review_parser(subparsers) -> argparse.ArgumentParser:
    """Build the review subcommand parser."""
    p = subparsers.add_parser(
        "review",
        help="Generate literature review",
        description="Generate structured literature review for a research topic.",
    )
    p.add_argument(
        "topic",
        nargs="?",
        default=None,
        help="Research topic",
    )
    p.add_argument(
        "--depth",
        choices=["short", "full"],
        default="full",
        help="Review depth (short or full)",
    )
    p.add_argument(
        "--sections",
        nargs="+",
        choices=["overview", "streams", "controversies", "timeline", "gaps"],
        help="Specific sections to generate",
    )
    p.add_argument(
        "--max-papers",
        type=int,
        default=50,
        help="Maximum papers to analyze",
    )
    p.add_argument(
        "--json", "-j",
        action="store_true",
        help="JSON output",
    )
    p.add_argument(
        "--export-md",
        type=str,
        default=None,
        help="Export as Markdown file",
    )
    return p


def _run_review(args: argparse.Namespace) -> int:
    """Run literature review command."""
    db = get_db()
    db.init()

    generator = ReviewGenerator(db=db)

    if not args.topic:
        print("❌ 请提供 topic")
        return 1

    print_info(f"📚 Generating literature review: {args.topic}")

    review = generator.generate(
        topic=args.topic,
        max_papers=args.max_papers,
        depth=args.depth,
        sections=args.sections,
    )

    if args.json:
        print(generator.render_json(review))
    elif args.export_md:
        with open(args.export_md, 'w', encoding='utf-8') as f:
            f.write(generator.render_markdown(review))
        print(f"✓ Exported to {args.export_md}")
    else:
        print()
        print(generator.render_markdown(review))

    return 0
