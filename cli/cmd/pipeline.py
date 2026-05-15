     1|"""CLI command: pipeline — Full research pipeline: gap → hypothesis → experiment.
     2|
     3|Runs gap analysis + hypothesis generation + optionally creates experiment records.
     4|"""
# [LEGACY] Research pipeline orchestration — multi-step LLM pipeline

     5|
     6|from __future__ import annotations
     7|
     8|import argparse
     9|
    10|from cli._shared import (
    11|    Colors,
    12|    colored,
    13|    get_db,
    14|    print_error,
    15|    print_info,
    16|    print_success,
    17|)
    18|from llm.experiment_tracker import ExperimentTracker
    19|from llm.research.gap_analyzer import GapAnalyzerV2, render_combined_report
    20|from llm.insight import EvolutionTracker
    21|
    22|
    23|def _build_pipeline_parser(subparsers) -> argparse.ArgumentParser:
    24|    p = subparsers.add_parser(
    25|        "pipeline",
    26|        help="Full research pipeline: gap analysis → hypothesis → experiment",
    27|        description="Run gap analysis, generate hypotheses, and optionally create experiment records.",
    28|    )
    29|    p.add_argument(
    30|        "topic",
    31|        nargs="?",
    32|        default="",
    33|        help="Research topic or keyword",
    34|    )
    35|    p.add_argument(
    36|        "--hypothesis-only",
    37|        action="store_true",
    38|        help="Run gap analysis + hypothesis only (skip experiment creation)",
    39|    )
    40|    p.add_argument(
    41|        "--experiments",
    42|        dest="create_experiments",
    43|        action="store_true",
    44|        help="Create experiment records from top hypotheses (default: yes)",
    45|    )
    46|    p.add_argument(
    47|        "--top",
    48|        "-n",
    49|        type=int,
    50|        default=3,
    51|        dest="top_hypotheses",
    52|        help="Number of top hypotheses to convert to experiments (default: 3)",
    53|    )
    54|    p.add_argument(
    55|        "--min-papers",
    56|        type=int,
    57|        default=5,
    58|        help="Minimum papers for gap analysis (default: 5)",
    59|    )
    60|    p.add_argument(
    61|        "--model",
    62|        type=str,
    63|        default=None,
    64|        help="LLM model override",
    65|    )
    66|    p.add_argument(
    67|        "--json",
    68|        "-j",
    69|        action="store_true",
    70|        help="Output combined report as JSON",
    71|    )
    72|    p.add_argument(
    73|        "--no-llm",
    74|        action="store_true",
    75|        help="Skip LLM enhancement for gap analysis",
    76|    )
    77|    p.add_argument(
    78|        "--verbose",
    79|        "-v",
    80|        action="store_true",
    81|        help="Verbose output",
    82|    )
    83|    return p  # type: ignore[no-any-return]
    84|
    85|
    86|def _run_pipeline(args: argparse.Namespace) -> int:
    87|    db = get_db()
    88|    db.init()
    89|
    90|    if not args.topic:
    91|        print_error("Please provide a research topic.")
    92|        return 1
    93|
    94|    print_info(f"Starting pipeline for: {args.topic}")
    95|
    96|    # Step 1: Gap analysis + hypothesis generation
    97|    tracker = EvolutionTracker()
    98|    analyzer = GapAnalyzerV2(db=db, evolution_tracker=tracker)
    99|    gap_result, hypothesis_result = analyzer.analyze_with_hypotheses(
   100|        topic=args.topic,
   101|        min_papers=args.min_papers,
   102|        use_llm=not args.no_llm,
   103|        model=args.model,
   104|    )
   105|
   106|    # Step 2: Render report
   107|    if args.json:
   108|        import json
   109|
   110|        output = {
   111|            "topic": args.topic,
   112|            "gaps": [
   113|                {
   114|                    "title": g.title,
   115|                    "type": g.gap_type.value,
   116|                    "severity": g.severity.value,
   117|                    "description": g.description,
   118|                }
   119|                for g in gap_result.gaps
   120|            ],
   121|            "hypotheses": [
   122|                {
   123|                    "title": h.title,
   124|                    "type": h.hypothesis_type.value,
   125|                    "statement": h.core_statement,
   126|                    "experiment": {
   127|                        "baseline": h.experiment_design.baseline,
   128|                        "variables": h.experiment_design.variables,
   129|                        "controls": h.experiment_design.controls,
   130|                        "metrics": h.experiment_design.evaluation_metrics,
   131|                    }
   132|                    if h.experiment_design
   133|                    else None,
   134|                }
   135|                for h in hypothesis_result.hypotheses
   136|            ],
   137|        }
   138|        print(json.dumps(output, ensure_ascii=False, indent=2))
   139|    else:
   140|        print(render_combined_report(gap_result, hypothesis_result))
   141|
   142|    # Step 3: Create experiments from top hypotheses
   143|    if not args.hypothesis_only and hypothesis_result.hypotheses:
   144|        exp_tracker = ExperimentTracker()
   145|        created = []
   146|        for h in hypothesis_result.hypotheses[: args.top_hypotheses]:
   147|            ed = h.experiment_design
   148|            if not ed:
   149|                continue
   150|            tracker.record_hypothesis_generated(
   151|                topic=args.topic,
   152|                gap_type=h.gap_type,
   153|                gap_title=h.title,
   154|                hypothesis_id=h.id,
   155|            )
   156|            exp = exp_tracker.run(
   157|                name=h.title,
   158|                description=h.core_statement,
   159|                hypothesis_id=h.id,
   160|                config={
   161|                    "baseline": ed.baseline,
   162|                    "variables": ed.variables,
   163|                    "controls": ed.controls,
   164|                    "evaluation_metrics": ed.evaluation_metrics,
   165|                    "expected_results": ed.expected_results,
   166|                    "hypothesis_type": h.hypothesis_type.value,
   167|                    "gap_type": h.gap_type,
   168|                    "based_on": h.based_on,
   169|                },
   170|                tags=[args.topic, h.hypothesis_type.value],
   171|            )
   172|            created.append(exp)
   173|            print_success(f"  Created experiment [{exp.id}]: {colored(exp.name, Colors.OKBLUE)}")
   174|
   175|        if created:
   176|            print_success(f"\n{len(created)} experiment(s) registered in experiment tracker.")
   177|            print_info(
   178|                "Run `airos experiment` to list them, or `airos experiment --complete <id>` when done."
   179|            )
   180|
   181|    return 0
   182|
