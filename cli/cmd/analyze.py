"""CLI command: analyze — Run full research pipeline (trend→story→validate→hypothesize)."""
from __future__ import annotations

import argparse

from cli._shared import get_db, print_info
from llm.pipeline import ResearchPipeline, PipelineStage


def _build_analyze_parser(subparsers) -> argparse.ArgumentParser:
    """Build the analyze subcommand parser."""
    p = subparsers.add_parser(
        "analyze",
        help="Run full research pipeline",
        description="Orchestrate gap→trend→story→hypothesis analysis.",
    )
    p.add_argument(
        "topic",
        nargs="?",
        default=None,
        help="Research topic to analyze",
    )
    p.add_argument(
        "--quick", "-q",
        action="store_true",
        help="Quick mode (fewer papers, faster)",
    )
    p.add_argument(
        "--json", "-j",
        action="store_true",
        help="JSON output",
    )
    p.add_argument(
        "--no-llm",
        action="store_true",
        help="Disable LLM enhancement",
    )
    p.add_argument(
        "--model", "-M",
        type=str,
        default=None,
        help="LLM model to use",
    )
    p.add_argument(
        "--stages", "-s",
        nargs="+",
        choices=["trend", "story", "validate", "hypothesize"],
        help="Specific stages to run",
    )
    p.add_argument(
        "--only-gap",
        action="store_true",
        help="Run only gap detection (validate stage)",
    )
    p.add_argument(
        "--only-hypothesis",
        action="store_true",
        help="Run only hypothesis generation",
    )
    return p


def _run_analyze(args: argparse.Namespace) -> int:
    """Run research pipeline command."""
    db = get_db()
    db.init()

    pipeline = ResearchPipeline(db=db)

    # Parse stages
    stages = None
    if args.only_gap:
        stages = [PipelineStage.VALIDATE]
    elif args.only_hypothesis:
        stages = [PipelineStage.HYPOTHESIZE]
    elif args.stages:
        stage_map = {
            "trend": PipelineStage.TREND,
            "story": PipelineStage.STORY,
            "validate": PipelineStage.VALIDATE,
            "hypothesize": PipelineStage.HYPOTHESIZE,
        }
        stages = [stage_map[s] for s in args.stages]

    if not args.topic:
        print("❌ 请提供 topic")
        return 1

    print_info(f"🔬 Running research pipeline: {args.topic}")

    result = pipeline.run(
        topic=args.topic,
        stages=stages,
        quick=args.quick,
        use_llm=not args.no_llm,
        model=args.model,
    )

    if args.json:
        import json
        # Build JSON-friendly result
        data = {
            "topic": result.topic,
            "errors": result.errors,
        }
        print(json.dumps(data, ensure_ascii=False, indent=2))
    else:
        print()
        print(pipeline.render_result(result))

    return 0
