"""AI Research OS CLI."""

import importlib

from typing import Any, Dict

from cli._registry import main

__all__ = ["main"]


# Lazy re-exports for backward compatibility (tests and internal use).


# Each name is imported on first access, then cached.


_LAZY_EXPORTS = {
    "_run_chat": ("cli.cmd.chat", "_run_chat"),
    "_build_chat_parser": ("cli.cmd.chat", "_build_chat_parser"),
    "_run_chat_tui": ("cli.cmd.chat_tui", "run"),
    "_build_chat_tui_parser": ("cli.cmd.chat_tui", "_build_chat_tui_parser"),
    "_run_path": ("cli.cmd.path", "_run_path"),
    "_build_path_parser": ("cli.cmd.path", "_build_path_parser"),
    "_run_question": ("cli.cmd.question", "_run_question"),
    "_build_question_parser": ("cli.cmd.question", "_build_question_parser"),
    "_run_roadmap": ("cli.cmd.roadmap", "_run_roadmap"),
    "_build_roadmap_parser": ("cli.cmd.roadmap", "_build_roadmap_parser"),
    "_run_pipeline": ("cli.cmd.pipeline", "_run_pipeline"),
    "_build_pipeline_parser": ("cli.cmd.pipeline", "_build_pipeline_parser"),
    "_run_demo": ("cli.cmd.demo", "run_demo"),
    "_run_evoskill": ("cli.cmd.evoskill", "evoskill"),
    "_run_rag": ("cli.cmd.rag", "rag"),
    "_run_validate": ("cli.cmd.validate", "_run_validate"),
    "_run_narrative": ("cli.cmd.narrative", "_run_narrative"),
    "_build_narrative_parser": ("cli.cmd.narrative", "_build_narrative_parser"),
    "_run_route": ("cli.cmd.route", "_run_route"),
    "_build_route_parser": ("cli.cmd.route", "_build_route_parser"),
    "infer_tags_if_empty": ("cli._shared", "infer_tags_if_empty"),
    "Database": ("db", "Database"),
    # Module-level re-exports (used by tests for mock.patch)
    "argparse": ("argparse", None),
    "Colors": ("cli._shared", "Colors"),
    "colored": ("cli._shared", "colored"),
    "print_success": ("cli._shared", "print_success"),
    "print_error": ("cli._shared", "print_error"),
    "print_warning": ("cli._shared", "print_warning"),
    "print_info": ("cli._shared", "print_info"),
    "print_header": ("cli._shared", "print_header"),
}

_cache: Dict[str, Any] = {}


def __getattr__(name):

    if name in _cache:
        return _cache[name]

    if name in _LAZY_EXPORTS:
        mod_path, fn_name = _LAZY_EXPORTS[name]

        mod = importlib.import_module(mod_path)

        val = mod if fn_name is None else getattr(mod, fn_name)

        _cache[name] = val

        return val

    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
