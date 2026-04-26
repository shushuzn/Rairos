"""CLI command: pipeline — Full research pipeline: gap → hypothesis → experiment.

Runs gap analysis + hypothesis generation + optionally creates experiment records.
"""
from __future__ import annotations

import argparse

from cli._shared import (
    Colors,
    colored,
    get_db,
    print_error,
    print_info,
    print_success,
)
from llm.experiment_tracker import ExperimentTracker
from llm.gap_analyzer import GapAnalyzerV2, render_combined_report


def _build_pipeline_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "pipeline",
        help="Full research pipeline: gap analysis → hypothesis → experiment",
        description="Run gap analysis, generate hypotheses, and optionally create experiment records.",
    )
    p.add_argument(
        "topic",
        nargs="?",
        default="",
        help="Research topic or keyword",
    )
    p.add_argument(
        "--hypothesis-only",
        action="store_true",
        help="Run gap analysis + hypothesis only (skip experiment creation)",
    )
    p.add_argument(
        "--experiments",
        dest="create_experiments",
        action="store_true",
        help="Create experiment records from top hypotheses (default: yes)",
    )
    p.add_argument(
        "--top", "-n",
        type=int,
        default=3,
        dest="top_hypotheses",
        help="Number of top hypotheses to convert to experiments (default: 3)",
    )
    p.add_argument(
        "--min-papers",
        type=int,
        default=5,
        help="Minimum papers for gap analysis (default: 5)",
    )
    p.add_argument(
        "--model",
        type=str,
        default=None,
        help="LLM model override",
    )
    p.add_argument(
        "--json", "-j",
        action="store_true",
        help="Output combined report as JSON",
    )
    p.add_argument(
        "--no-llm",
        action="store_true",
        help="Skip LLM enhancement for gap analysis",
    )
    p.add_argument(
        "--verbose", "-v",
        action="store_true",
        help="Verbose output",
    )
    return p


def _run_pipeline(args: argparse.Namespace) -> int:
    db = get_db()
    db.init()

    if not args.topic:
        print_error("Please provide a research topic.")
        return 1

    print_info(f"Starting pipeline for: {args.topic}")

    # Step 1: Gap analysis + hypothesis generation
    analyzer = GapAnalyzerV2(db=db)
    gap_result, hypothesis_result = analyzer.analyze_with_hypotheses(
        topic=args.topic,
        min_papers=args.min_papers,
        use_llm=not args.no_llm,
        model=args.model,
    )

    # Step 2: Render report
    if args.json:
        import json
        output = {
            "topic": args.topic,
            "gaps": [
                {
                    "title": g.title,
                    "type": g.gap_type.value,
                    "severity": g.severity.value,
                    "description": g.description,
                }
                for g in gap_result.gaps
            ],
            "hypotheses": [
                {
                    "title": h.title,
                    "type": h.hypothesis_type.value,
                    "statement": h.core_statement,
                    "experiment": {
                        "baseline": h.experiment_design.baseline,
                        "variables": h.experiment_design.variables,
                        "controls": h.experiment_design.controls,
                        "metrics": h.experiment_design.evaluation_metrics,
                    } if h.experiment_design else None,
                }
                for h in hypothesis_result.hypotheses
            ],
        }
        print(json.dumps(output, ensure_ascii=False, indent=2))
    else:
        print(render_combined_report(gap_result, hypothesis_result))

    # Step 3: Create experiments from top hypotheses
    if not args.hypothesis_only and hypothesis_result.hypotheses:
        tracker = ExperimentTracker()
        created = []
        for h in hypothesis_result.hypotheses[:args.top_hypotheses]:
            ed = h.experiment_design
            if not ed:
                continue
            exp = tracker.run(
                name=h.title,
                description=h.core_statement,
                hypothesis_id=h.id,
                config={
                    "baseline": ed.baseline,
                    "variables": ed.variables,
                    "controls": ed.controls,
                    "evaluation_metrics": ed.evaluation_metrics,
                    "expected_results": ed.expected_results,
                    "hypothesis_type": h.hypothesis_type.value,
                    "gap_type": h.gap_type,
                    "based_on": h.based_on,
                },
                tags=[args.topic, h.hypothesis_type.value],
            )
            created.append(exp)
            print_success(
                f"  Created experiment [{exp.id}]: {colored(exp.name, Colors.OKBLUE)}"
            )

        if created:
            print_success(f"\n{len(created)} experiment(s) registered in experiment tracker.")
            print_info("Run `airos experiment` to list them, or `airos experiment --complete <id>` when done.")

    return 0
