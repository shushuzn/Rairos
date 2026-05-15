"""AI Research OS CLI."""

import importlib


from typing import Any, Dict


from cli._registry import main


__all__ = ["main"]


# Lazy re-exports for backward compatibility (tests and internal use).


# Each name is imported on first access, then cached.


_LAZY_EXPORTS = {
    "_run_dedup_semantic": ("cli.cmd.dedup_semantic", "_run_dedup_semantic"),
    "_build_dedup_semantic_parser": ("cli.cmd.dedup_semantic", "_build_dedup_semantic_parser"),
    "_run_kg": ("cli.cmd.kg.kg", "_run_kg"),
    "_build_kg_parser": ("cli.cmd.kg.kg", "_build_kg_parser"),
    "_run_cite_graph": ("cli.cmd.cite_graph", "_run_cite_graph"),
    "_build_cite_graph_parser": ("cli.cmd.cite_graph", "_build_cite_graph_parser"),
    "_run_cite_fetch": ("cli.cmd.cite_fetch", "_run_cite_fetch"),
    "_build_cite_fetch_parser": ("cli.cmd.cite_fetch", "_build_cite_fetch_parser"),
    "_run_cite_backfill": ("cli.cmd.cite_backfill", "_run_cite_backfill"),
    "_build_cite_backfill_parser": ("cli.cmd.cite_backfill", "_build_cite_backfill_parser"),
    "_arxiv_doi_to_openalex": ("cli.cmd.cite_fetch", "_arxiv_doi_to_openalex"),
    "_get_ollama_embedding_batch": ("cli.cmd.dedup_semantic", "_get_ollama_embedding_batch"),
    "_extract_references_from_text": ("cli.cmd.cite_graph", "_extract_references_from_text"),
    "_run_read_queue": ("cli.cmd.read_queue", "_run_read_queue"),
    "_build_read_queue_parser": ("cli.cmd.read_queue", "_build_read_queue_parser"),
    "_run_chat": ("cli.cmd.chat", "_run_chat"),
    "_build_chat_parser": ("cli.cmd.chat", "_build_chat_parser"),
    "_run_chat_tui": ("cli.cmd.chat_tui", "run"),
    "_build_chat_tui_parser": ("cli.cmd.chat_tui", "_build_chat_tui_parser"),
    "_run_path": ("cli.cmd.path", "_run_path"),
    "_build_path_parser": ("cli.cmd.path", "_build_path_parser"),
    "_run_gap": ("cli.cmd.gap", "_run_gap"),
    "_build_gap_parser": ("cli.cmd.gap", "_build_gap_parser"),
    "_run_hypothesize": ("cli.cmd.hypothesize", "_run_hypothesize"),
    "_build_hypothesize_parser": ("cli.cmd.hypothesize", "_build_hypothesize_parser"),
    "_run_question": ("cli.cmd.question", "_run_question"),
    "_build_question_parser": ("cli.cmd.question", "_build_question_parser"),
    "_run_roadmap": ("cli.cmd.roadmap", "_run_roadmap"),
    "_build_roadmap_parser": ("cli.cmd.roadmap", "_build_roadmap_parser"),
    "_run_experiment": ("cli.cmd.experiment", "_run_experiment"),
    "_build_experiment_parser": ("cli.cmd.experiment", "_build_experiment_parser"),
    "_run_pipeline": ("cli.cmd.pipeline", "_run_pipeline"),
    "_build_pipeline_parser": ("cli.cmd.pipeline", "_build_pipeline_parser"),
    "_run_dashboard": ("cli.cmd.dashboard", "_run_web"),
    "_build_dashboard_parser": ("cli.cmd.dashboard", "_build_web_parser"),
    "_build_lean_parser": ("cli.cmd.lean", "_build_lean_parser"),
    "_run_lean": ("cli.cmd.lean", "_run_lean"),
    "_build_citation_chain_parser": ("cli.cmd.citation_chain", "_build_citation_chain_parser"),
    "_build_insight_parser": ("cli.cmd.insight", "_build_insight_parser"),
    "_build_session_parser": ("cli.cmd.session", "_build_session_parser"),
    "_run_session": ("cli.cmd.session", "_run_session"),
    "_run_slides": ("cli.cmd.slides", "_run_slides"),
    "_build_slides_parser": ("cli.cmd.slides", "_build_slides_parser"),
    "_run_evolution": ("cli.cmd.evolution", "_run_evolution"),
    "_build_evolution_parser": ("cli.cmd.evolution", "_build_evolution_parser"),
    "_run_jin10": ("cli.cmd.jin10", "_run_jin10"),
    "_run_demo": ("cli.cmd.demo", "run_demo"),
    "_run_evoskill": ("cli.cmd.evoskill", "evoskill"),
    "_run_rag": ("cli.cmd.rag", "rag"),
    "_run_visual": ("cli.cmd.visual", "visual_extract"),
    "_run_paper2code": ("cli.cmd.paper.paper2code", "paper2code"),
    "_run_validate": ("cli.cmd.validate", "_run_validate"),
    "_run_narrative": ("cli.cmd.narrative", "_run_narrative"),
    "_build_narrative_parser": ("cli.cmd.narrative", "_build_narrative_parser"),
    "_run_route": ("cli.cmd.route", "_run_route"),
    "_build_route_parser": ("cli.cmd.route", "_build_route_parser"),
    "_run_friction": ("cli.cmd.friction", "run"),
    "_build_friction_parser": ("cli.cmd.friction", "_build_friction_parser"),
    "_run_ingest": ("cli.cmd.ingest", "_run_ingest"),
    "_build_ingest_parser": ("cli.cmd.ingest", "_build_ingest_parser"),
    "_run_postprocess": ("cli.cmd.postprocess", "_run_postprocess"),
    "_build_postprocess_parser": ("cli.cmd.postprocess", "_build_postprocess_parser"),
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
