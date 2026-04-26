"""AI Research OS CLI."""
import importlib
from cli._registry import main, _main_legacy

__all__ = ["main"]


# Lazy re-exports for backward compatibility (tests and internal use).
# Each name is imported on first access, then cached.
_LAZY_EXPORTS = {
    "_run_search":        ("cli.cmd.search",       "_run_search"),
    "_build_search_parser": ("cli.cmd.search",     "_build_search_parser"),
    "_run_list":         ("cli.cmd.list",        "_run_list"),
    "_build_list_parser":  ("cli.cmd.list",       "_build_list_parser"),
    "_run_status":       ("cli.cmd.status",      "_run_status"),
    "_build_status_parser": ("cli.cmd.status",    "_build_status_parser"),
    "_run_stats":        ("cli.cmd.stats",       "_run_stats"),
    "_build_stats_parser":  ("cli.cmd.stats",     "_build_stats_parser"),
    "_run_import":       ("cli.cmd.import_",     "_run_import"),
    "_build_import_parser": ("cli.cmd.import_",   "_build_import_parser"),
    "_run_export":       ("cli.cmd.export",      "_run_export"),
    "_build_export_parser":  ("cli.cmd.export",   "_build_export_parser"),
    "_run_queue":        ("cli.cmd.queue",       "_run_queue"),
    "_build_queue_parser":  ("cli.cmd.queue",     "_build_queue_parser"),
    "_run_cache":        ("cli.cmd.cache",       "_run_cache"),
    "_build_cache_parser":  ("cli.cmd.cache",     "_build_cache_parser"),
    "_run_dedup":        ("cli.cmd.dedup",      "_run_dedup"),
    "_build_dedup_parser":  ("cli.cmd.dedup",    "_build_dedup_parser"),
    "_run_dedup_semantic": ("cli.cmd.dedup_semantic", "_run_dedup_semantic"),
    "_build_dedup_semantic_parser": ("cli.cmd.dedup_semantic", "_build_dedup_semantic_parser"),
    "_run_similar":      ("cli.cmd.similar",    "_run_similar"),
    "_build_similar_parser": ("cli.cmd.similar", "_build_similar_parser"),
    "_run_kg":           ("cli.cmd.kg",         "_run_kg"),
    "_build_kg_parser":   ("cli.cmd.kg",         "_build_kg_parser"),
    "_run_merge":        ("cli.cmd.merge",       "_run_merge"),
    "_build_merge_parser":  ("cli.cmd.merge",     "_build_merge_parser"),
    "_pick_keep":        ("cli.cmd.merge",       "_pick_keep"),
    "_run_citations":    ("cli.cmd.citations",   "_run_citations"),
    "_build_citations_parser": ("cli.cmd.citations", "_build_citations_parser"),
    "_run_cite_graph":   ("cli.cmd.cite_graph",  "_run_cite_graph"),
    "_build_cite_graph_parser": ("cli.cmd.cite_graph", "_build_cite_graph_parser"),
    "_run_cite_fetch":   ("cli.cmd.cite_fetch",  "_run_cite_fetch"),
    "_build_cite_fetch_parser": ("cli.cmd.cite_fetch", "_build_cite_fetch_parser"),
    "_run_cite_import":  ("cli.cmd.cite_import", "_run_cite_import"),
    "_build_cite_import_parser": ("cli.cmd.cite_import", "_build_cite_import_parser"),
    "_run_cite_stats":   ("cli.cmd.cite_stats",  "_run_cite_stats"),
    "_build_cite_stats_parser": ("cli.cmd.cite_stats", "_build_cite_stats_parser"),
    "_run_research_cmd": ("cli.cmd.research",    "_run_research_cmd"),
    "_build_research_parser": ("cli.cmd.research", "_build_research_parser"),
    "_arxiv_doi_to_openalex": ("cli.cmd.cite_fetch", "_arxiv_doi_to_openalex"),
    "_get_ollama_embedding_batch": ("cli.cmd.dedup_semantic", "_get_ollama_embedding_batch"),
    "_extract_references_from_text": ("cli.cmd.cite_graph", "_extract_references_from_text"),
    "_run_repl":             ("cli.cmd.repl",              "_run_repl"),
    "_build_repl_parser":    ("cli.cmd.repl",              "_build_repl_parser"),
    "_run_read_queue":       ("cli.cmd.read_queue",        "_run_read_queue"),
    "_build_read_queue_parser": ("cli.cmd.read_queue",     "_build_read_queue_parser"),
    "_run_chat":             ("cli.cmd.chat",              "_run_chat"),
    "_build_chat_parser":     ("cli.cmd.chat",              "_build_chat_parser"),
    "_run_path":             ("cli.cmd.path",              "_run_path"),
    "_build_path_parser":     ("cli.cmd.path",              "_build_path_parser"),
    "_run_gap":             ("cli.cmd.gap",              "_run_gap"),
    "_build_gap_parser":     ("cli.cmd.gap",              "_build_gap_parser"),
    "_run_hypothesize":      ("cli.cmd.hypothesize",      "_run_hypothesize"),
    "_build_hypothesize_parser": ("cli.cmd.hypothesize",  "_build_hypothesize_parser"),
    "_run_story":            ("cli.cmd.story",            "_run_story"),
    "_build_story_parser":   ("cli.cmd.story",            "_build_story_parser"),
    "_run_analyze":          ("cli.cmd.analyze",          "_run_analyze"),
    "_run_review":           ("cli.cmd.review",           "_run_review"),
    "_build_review_parser":   ("cli.cmd.review",           "_build_review_parser"),
    "_run_question":          ("cli.cmd.question",          "_run_question"),
    "_build_question_parser":   ("cli.cmd.question",          "_build_question_parser"),
    "_build_analyze_parser":  ("cli.cmd.analyze",          "_build_analyze_parser"),
    "_run_slides":           ("cli.cmd.slides",            "_run_slides"),
    "_build_slides_parser":   ("cli.cmd.slides",            "_build_slides_parser"),
    "_run_evolution":         ("cli.cmd.evolution",          "_run_evolution"),
    "_build_evolution_parser":("cli.cmd.evolution",          "_build_evolution_parser"),
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

_cache = {}


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
