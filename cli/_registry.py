"""CLI command registry and main entry point."""

from __future__ import annotations


import argparse


import importlib


import logging


import sys


from typing import List, Optional, cast


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
    ("dedup-semantic", "cli.cmd.dedup_semantic", "_build_dedup_semantic_parser"),
    ("kg", "cli.cmd.kg", "_build_kg_parser"),
    ("paper2code", "cli.cmd.paper.paper2code", "_build_paper2code_parser"),
    ("trace", "cli.cmd.paper.trace", "_build_paper_trace_parser"),
    ("evoskill", "cli.cmd.evoskill", "_build_evoskill_parser"),
    ("rag", "cli.cmd.rag", "_build_rag_parser"),
    ("route", "cli.cmd.route", "_build_route_parser"),
    ("read-queue", "cli.cmd.read_queue", "_build_read_queue_parser"),
    ("chat", "cli.cmd.chat", "_build_chat_parser"),
    ("path", "cli.cmd.path", "_build_path_parser"),
    ("validate", "cli.cmd.validate", "_build_validate_parser"),
    ("slides", "cli.cmd.slides", "_build_slides_parser"),
    ("question", "cli.cmd.question", "_build_question_parser"),
    ("roadmap", "cli.cmd.roadmap", "_build_roadmap_parser"),
    ("pipeline", "cli.cmd.pipeline", "_build_pipeline_parser"),
    ("narrative", "cli.cmd.narrative", "_build_narrative_parser"),
    ("chat-tui", "cli.cmd.chat_tui", "_build_chat_tui_parser"),
    ("postprocess", "cli.cmd.postprocess", "_build_postprocess_parser"),
    ("demo", "cli.cmd.demo", "_build_demo_parser"),
    ("jin10", "cli.cmd.jin10", "_build_jin10_parser"),
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

    # Opt-in structured JSON logs via environment variable
    import os  # noqa: F401 - used in this function

    if os.getenv("RAIROS_JSON_LOGS", "").lower() in ("1", "true", "yes"):
        from core.observability import setup_observability

        setup_observability(
            level=os.getenv("RAIROS_LOG_LEVEL", "INFO").upper(),
            json_logs=True,
        )
    else:
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
        "dedup-semantic": "_run_dedup_semantic",
        "read-queue": "_run_read_queue",
        "chat": "_run_chat",
        "slides": "_run_slides",
        "cite-backfill": "_run_cite_backfill",
        "question": "_run_question",
        "roadmap": "_run_roadmap",
        "pipeline": "_run_pipeline",
        "kg": "_run_kg",
        "narrative": "_run_narrative",
        "route": "_run_route",
        "chat-tui": "_run_chat_tui",
        "postprocess": "_run_postprocess",
        "jin10": "_run_jin10",
        "demo": "_run_demo",
        "paper2code": "_run_paper2code",
        "validate": "_run_validate",
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

        return evoskill_cmd.main(args.argv if hasattr(args, "argv") else [])  # type: ignore[no-any-return]

    elif args.subcmd == "rag":
        from cli.cmd.rag import rag as rag_cmd

        return rag_cmd.main(args.argv if hasattr(args, "argv") else [])  # type: ignore[no-any-return]

    elif args.subcmd == "slides":
        from cli.cmd.slides import slides as slides_cmd

        return slides_cmd.main(args.argv if hasattr(args, "argv") else [])  # type: ignore[no-any-return]

    elif args.subcmd == "repl":
        import cli as _cli

        return cast(int, _cli._run_repl(args))

    return 0
