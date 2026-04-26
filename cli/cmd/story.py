"""CLI command: story — Research story weaving."""
from __future__ import annotations

import argparse

from cli._shared import get_db, print_info
from llm.story_weaver import StoryWeaver


def _build_story_parser(subparsers) -> argparse.ArgumentParser:
    """Build the story subcommand parser."""
    p = subparsers.add_parser(
        "story",
        help="Weave research papers into narrative stories",
        description="Generate narrative understanding from research papers.",
    )
    p.add_argument(
        "topic",
        nargs="?",
        default=None,
        help="Research topic to weave into story",
    )
    p.add_argument(
        "--compare", "-c",
        nargs=2,
        metavar=("TOPIC_A", "TOPIC_B"),
        help="Compare two research storylines",
    )
    p.add_argument(
        "--papers", "-p",
        nargs="+",
        help="Specific paper IDs to analyze",
    )
    p.add_argument(
        "--no-llm",
        action="store_true",
        help="Disable LLM narrative generation",
    )
    p.add_argument(
        "--mermaid", "-m",
        action="store_true",
        help="Output as Mermaid flowchart",
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
        "--max-papers", "-n",
        type=int,
        default=20,
        help="Maximum papers to analyze (default: 20)",
    )
    return p


def _run_story(args: argparse.Namespace) -> int:
    """Run story weaving command."""
    db = get_db()
    db.init()

    weaver = StoryWeaver(db=db)

    # Compare mode
    if args.compare:
        topic_a, topic_b = args.compare
        print_info(f"📖 Comparing storylines: {topic_a} vs {topic_b}")
        comparison = weaver.compare(topic_a, topic_b, use_llm=not args.no_llm)
        print()
        print(comparison)
        return 0

    # Single topic mode
    if not args.topic:
        print("❌ 请提供 topic 或使用 --compare 比较两个主题")
        return 1

    print_info(f"📖 Weaving story for: {args.topic}")

    result = weaver.weave(
        topic=args.topic,
        use_llm=not args.no_llm,
        model=args.model,
        max_papers=args.max_papers,
    )

    if args.json:
        import json
        data = {
            "topic": result.topic,
            "chapters": [
                {
                    "title": c.title,
                    "time_range": c.time_range,
                    "paper_count": len(c.papers),
                    "summary": c.summary,
                }
                for c in result.chapters
            ],
            "themes": result.themes,
            "contradictions": result.contradictions,
            "summary": result.summary,
        }
        print(json.dumps(data, ensure_ascii=False, indent=2))
    elif args.mermaid:
        print(weaver.render_mermaid(result))
    else:
        print()
        print(weaver.render_result(result))

    return 0
