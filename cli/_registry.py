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
    ("chat", "cli.cmd.chat", "_build_chat_parser"),
    ("chat-tui", "cli.cmd.chat_tui", "_build_chat_tui_parser"),
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
        "chat": "_run_chat",
        "chat-tui": "_run_chat_tui",
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

    return 0
    return 0
