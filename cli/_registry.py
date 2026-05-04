"""CLI command registry and main entry point."""

from __future__ import annotations


import argparse


import importlib


import logging


import sys


from typing import List, Optional, cast


from pathlib import Path


logger = logging.getLogger(__name__)


# ─── Custom help formatter ───────────────────────────────────────────────────────


class _WarpHelpFormatter(argparse.RawDescriptionHelpFormatter):
    """Format help with Warp-style tips."""

    def __init__(self, prog):

        super().__init__(prog, max_help_position=40, width=80)

    def _add_warp_tip(self):
        """Return Warp tip text for end of help."""

        return """


[#A5D5FE]Tip:[/#A5D5FE] Use [#B4FA72]--format warp[/#B4FA72] for Warp terminal style:


  airos stats --format warp


  airos search "attention" --format warp


  airos cache --llm


"""

    def format_help(self):

        help_text = super().format_help()

        return help_text + self._add_warp_tip()


# All available subcommands — derived from _SUBCOMMAND_TABLE


_SUBCOMMAND_TABLE = [
    ("search", "cli.cmd.search", "_build_search_parser"),
    ("research", "cli.cmd.research", "_build_research_parser"),
    ("list", "cli.cmd.list", "_build_list_parser"),
    ("status", "cli.cmd.status", "_build_status_parser"),
    ("queue", "cli.cmd.queue", "_build_queue_parser"),
    ("cache", "cli.cmd.cache", "_build_cache_parser"),
    ("dedup", "cli.cmd.dedup", "_build_dedup_parser"),
    ("dedup-semantic", "cli.cmd.dedup_semantic", "_build_dedup_semantic_parser"),
    ("similar", "cli.cmd.similar", "_build_similar_parser"),
    ("kg", "cli.cmd.kg", "_build_kg_parser"),
    ("merge", "cli.cmd.merge", "_build_merge_parser"),
    ("stats", "cli.cmd.stats", "_build_stats_parser"),
    ("import", "cli.cmd.import_", "_build_import_parser"),
    ("export", "cli.cmd.export", "_build_export_parser"),
    ("citations", "cli.cmd.citations", "_build_citations_parser"),
    ("cite-graph", "cli.cmd.cite_graph", "_build_cite_graph_parser"),
    ("cite-import", "cli.cmd.cite_import", "_build_cite_import_parser"),
    ("cite-fetch", "cli.cmd.cite_fetch", "_build_cite_fetch_parser"),
    ("cite-stats", "cli.cmd.cite_stats", "_build_cite_stats_parser"),
    ("cite-backfill", "cli.cmd.cite_backfill", "_build_cite_backfill_parser"),
    ("paper2code", "cli.cmd.paper.paper2code", "_build_paper2code_parser"),
    ("evoskill", "cli.cmd.evoskill", "_build_evoskill_parser"),
    ("rag", "cli.cmd.rag", "_build_rag_parser"),
    ("agent", "cli.cmd.agent", "_build_agent_parser"),
    ("route", "cli.cmd.route", "_build_route_parser"),
    ("visual", "cli.cmd.visual", "_build_visual_parser"),
    ("repl", "cli.cmd.repl", "_build_repl_parser"),
    ("read-queue", "cli.cmd.read_queue", "_build_read_queue_parser"),
    ("chat", "cli.cmd.chat", "_build_chat_parser"),
    ("path", "cli.cmd.path", "_build_path_parser"),
    ("gap", "cli.cmd.gap", "_build_gap_parser"),
    ("trend", "cli.cmd.trend", "_build_trend_parser"),
    ("influence", "cli.cmd.influence", "_build_influence_parser"),
    ("hypothesize", "cli.cmd.hypothesize", "_build_hypothesize_parser"),
    ("lean", "cli.cmd.lean", "_build_lean_parser"),
    ("validate", "cli.cmd.validate", "_build_validate_parser"),
    ("story", "cli.cmd.story", "_build_story_parser"),
    ("slides", "cli.cmd.slides", "_build_slides_parser"),
    ("evolution", "cli.cmd.evolution", "_build_evolution_parser"),
    ("analyze", "cli.cmd.analyze", "_build_analyze_parser"),
    ("review", "cli.cmd.review", "_build_review_parser"),
    ("question", "cli.cmd.question", "_build_question_parser"),
    ("roadmap", "cli.cmd.roadmap", "_build_roadmap_parser"),
    ("experiment", "cli.cmd.experiment", "_build_experiment_parser"),
    ("pipeline", "cli.cmd.pipeline", "_build_pipeline_parser"),
    ("dashboard", "cli.cmd.dashboard", "_build_dashboard_parser"),
    ("journal", "cli.cmd.journal", "_build_journal_parser"),
    ("digest", "cli.cmd.digest", "_build_digest_parser"),
    ("citation-chain", "cli.cmd.citation_chain", "_build_citation_chain_parser"),
    ("compare", "cli.cmd.compare", "_build_compare_parser"),
    ("replicate", "cli.cmd.replicate", "_build_replicate_parser"),
    ("insight", "cli.cmd.insight", "_build_insight_parser"),
    ("ask", "cli.cmd.ask", "_build_ask_parser"),
    ("session", "cli.cmd.session", "_build_session_parser"),
    ("argue", "cli.cmd.argue", "_build_argue_parser"),
    ("narrative", "cli.cmd.narrative", "_build_narrative_parser"),
    ("friction", "cli.cmd.friction", "_build_friction_parser"),
    ("chat-tui", "cli.cmd.chat_tui", "_build_chat_tui_parser"),
    ("subscribe", "cli.cmd.subscribe", "_build_subscribe_parser"),
    ("litreview", "cli.cmd.litreview", "_build_litreview_parser"),
    ("benchmark", "cli.cmd.benchmark", "_build_benchmark_parser"),
    ("postprocess", "cli.cmd.postprocess", "_build_postprocess_parser"),
    ("ingest", "cli.cmd.ingest", "_build_ingest_parser"),
    ("daemon", "cli.cmd.daemon", "_build_daemon_parser"),
    ("demo", "cli.cmd.demo", "_build_demo_parser"),
    ("scout", "cli.cmd.scout", "_build_scout_parser"),
    ("jin10", "cli.cmd.jin10", "_build_jin10_parser"),
    ("intel", "cli.cmd.intel", "_build_intel_parser"),
    ("signal", "cli.cmd.signal", "_build_signal_parser"),
]


SUBCOMMANDS = {name for name, _, _ in _SUBCOMMAND_TABLE}


def _build_all_parsers(subparsers) -> None:
    """Build all subcommand parsers via lazy dynamic import."""

    for _name, module_path, builder_name in _SUBCOMMAND_TABLE:
        mod = importlib.import_module(module_path)

        getattr(mod, builder_name)(subparsers)

    # Watch command (inline)

    p = subparsers.add_parser("watch", help="Watch papers.json and auto-rebuild KG on changes")

    p.add_argument(
        "--papers-json",
        default="",
        help="Path to papers.json (default: auto-detect)",
    )

    p.add_argument(
        "--poll-interval",
        type=float,
        default=5.0,
        help="Poll interval in seconds (default: 5)",
    )

    p.add_argument(
        "--no-incremental",
        action="store_true",
        help="Run full rebuild instead of incremental",
    )


def main(argv: Optional[List[str]] = None) -> int:
    """Main CLI entry point."""

    logging.basicConfig(level=logging.WARNING, format="%(levelname)s: %(message)s")

    raw_args = argv if argv is not None else sys.argv[1:]

    first = raw_args[0] if raw_args else ""

    # Handle --help and -h before subcommand check

    if first in ("-h", "--help") or "--help" in raw_args or "-h" in raw_args:
        pass  # Fall through to parser building below

    parser = argparse.ArgumentParser(
        description="[#FF8272]AI Research OS[/#FF8272] — Self-Evolving Research System",
        formatter_class=_WarpHelpFormatter,
    )

    subparsers = parser.add_subparsers(dest="subcmd", help="Subcommands")

    # Build all parsers

    _build_all_parsers(subparsers)

    args = parser.parse_args(argv if argv is not None else sys.argv[1:])

    # Resolve model default from config

    if getattr(args, "model", None) is None:
        try:
            from config import DEFAULT_LLM_MODEL_CLI as _cli_default

            args.model = _cli_default

        except (ImportError, AttributeError):
            args.model = "qwen3.5-plus"

    # Lazy dispatch — attribute name so test mocks on cli._run_X take effect

    dispatch = {
        "search": "_run_search",
        "list": "_run_list",
        "status": "_run_status",
        "queue": "_run_queue",
        "cache": "_run_cache",
        "dedup": "_run_dedup",
        "merge": "_run_merge",
        "stats": "_run_stats",
        "import": "_run_import",
        "export": "_run_export",
        "citations": "_run_citations",
        "cite-graph": "_run_cite_graph",
        "cite-import": "_run_cite_import",
        "cite-fetch": "_run_cite_fetch",
        "cite-stats": "_run_cite_stats",
        "dedup-semantic": "_run_dedup_semantic",
        "research": "_run_research_cmd",
        "similar": "_run_similar",
        "kg": "_run_kg",
        "read-queue": "_run_read_queue",
        "chat": "_run_chat",
        "slides": "_run_slides",
        "hypothesize": "_run_hypothesize",
        "gap": "_run_gap",
        "trend": "_run_trend",
        "influence": "_run_influence",
        "cite-backfill": "_run_cite_backfill",
        "analyze": "_run_analyze",
        "review": "_run_review",
        "question": "_run_question",
        "roadmap": "_run_roadmap",
        "experiment": "_run_experiment",
        "pipeline": "_run_pipeline",
        "dashboard": "_run_dashboard",
        "journal": "_run_journal",
        "digest": "_run_digest",
        "lean": "_run_lean",
        "citation-chain": "_run_citation_chain",
        "compare": "_run_compare",
        "replicate": "_run_replicate",
        "insight": "_run_insight",
        "ask": "_run_ask",
        "session": "_run_session",
        "argue": "_run_argue",
        "narrative": "_run_narrative",
        "route": "_run_route",
        "friction": "_run_friction",
        "chat-tui": "_run_chat_tui",
        "subscribe": "_run_subscribe",
        "litreview": "_run_litreview",
        "benchmark": "_run_benchmark",
        "postprocess": "_run_postprocess",
        "ingest": "_run_ingest",
    }

    if args.subcmd in dispatch:
        import cli as _cli

        func = getattr(_cli, dispatch[args.subcmd])  # type: ignore[assignment]

        return cast(int, func(args))

    elif args.subcmd == "watch":
        from core.watch_papers import watch_and_rebuild

        watch_and_rebuild(
            papers_json=args.papers_json or None,
            interval=args.poll_interval,
            incremental=not args.no_incremental,
        )

        return 0

    elif args.subcmd == "evoskill":
        from cli.cmd.evoskill import evoskill as evoskill_cmd

        return evoskill_cmd.main(args.argv if hasattr(args, "argv") else [])

    elif args.subcmd == "rag":
        from cli.cmd.rag import rag as rag_cmd

        return rag_cmd.main(args.argv if hasattr(args, "argv") else [])

    elif args.subcmd == "slides":
        from cli.cmd.slides import slides as slides_cmd

        return slides_cmd.main(args.argv if hasattr(args, "argv") else [])

    elif args.subcmd == "evolution":
        from cli.cmd.evolution import evolution_main

        return cast(
            int,
            evolution_main(
                show_stats=args.stats,
                show_patterns=args.patterns,
                show_feedback=args.feedback,
                show_report=args.report,
                show_sessions=args.sessions,
                report_days=args.days,
                clear=args.clear,
                export=args.export,
            ),
        )

    elif args.subcmd == "visual":
        from cli.cmd.visual import _show_visual_status

        return cast(int, _show_visual_status())

    elif args.subcmd == "repl":
        import cli as _cli

        return cast(int, _cli._run_repl(args))

    return 0
