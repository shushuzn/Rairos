"""CLI command: roadmap — Generate research roadmaps."""
from __future__ import annotations

import argparse

from cli._shared import print_info, print_error
from llm.roadmap_generator import RoadmapGenerator
from llm.question_tracker import QuestionTracker


def _build_roadmap_parser(subparsers) -> argparse.ArgumentParser:
    """Build the roadmap subcommand parser."""
    p = subparsers.add_parser(
        "roadmap",
        help="Generate research roadmaps",
        description="Generate structured research roadmaps from questions and hypotheses.",
    )
    p.add_argument(
        "--question", "-q",
        type=str,
        help="Question ID to generate roadmap for",
    )
    p.add_argument(
        "--text", "-t",
        type=str,
        help="Research question text (if not using --question)",
    )
    p.add_argument(
        "--json", "-j",
        action="store_true",
        help="JSON output",
    )
    p.add_argument(
        "--export-md",
        type=str,
        help="Export as Markdown file",
    )
    return p


def _run_roadmap(args: argparse.Namespace) -> int:
    """Run roadmap generation command."""
    generator = RoadmapGenerator()
    tracker = QuestionTracker()

    # Determine question
    question_text = ""
    question_id = ""

    if args.question:
        # Fetch question from tracker
        q = tracker.get(args.question)
        if q:
            question_text = q.question
            question_id = q.id
        else:
            print_error(f"问题 [{args.question}] 不存在")
            return 1
    elif args.text:
        question_text = args.text
    else:
        print_error("请提供 --question <id> 或 --text <问题>")
        return 1

    print_info(f"📋 生成研究路线图...")

    roadmap = generator.generate(
        question=question_text,
        question_id=question_id,
    )

    if args.json:
        print(generator.render_json(roadmap))
    elif args.export_md:
        with open(args.export_md, 'w', encoding='utf-8') as f:
            f.write(generator.render_markdown(roadmap))
        print(f"✓ 导出到 {args.export_md}")
    else:
        print()
        print(generator.render_text(roadmap))

    return 0
