"""AI Research OS CLI."""
import argparse

from db import Database
from cli._registry import main, _main_legacy
from cli._shared import (
    Colors, colored, print_success, print_error, print_warning,
    print_info, print_header, infer_tags_if_empty,
)

# Re-export _run_* and _build_* functions for backward compatibility with tests
from cli.cmd.search import _run_search, _build_search_parser
from cli.cmd.list import _run_list, _build_list_parser
from cli.cmd.status import _run_status, _build_status_parser
from cli.cmd.stats import _run_stats, _build_stats_parser
from cli.cmd.import_ import _run_import, _build_import_parser
from cli.cmd.export import _run_export, _build_export_parser
from cli.cmd.queue import _run_queue, _build_queue_parser
from cli.cmd.cache import _run_cache, _build_cache_parser
from cli.cmd.dedup import _run_dedup, _build_dedup_parser
from cli.cmd.dedup_semantic import _run_dedup_semantic, _build_dedup_semantic_parser
from cli.cmd.similar import _run_similar, _build_similar_parser
from cli.cmd.kg import _run_kg, _build_kg_parser
from cli.cmd.merge import _run_merge, _build_merge_parser, _pick_keep
from cli.cmd.citations import _run_citations, _build_citations_parser
from cli.cmd.cite_graph import _run_cite_graph, _build_cite_graph_parser
from cli.cmd.cite_fetch import _run_cite_fetch, _build_cite_fetch_parser
from cli.cmd.cite_import import _run_cite_import, _build_cite_import_parser
from cli.cmd.cite_stats import _run_cite_stats, _build_cite_stats_parser
from cli.cmd.research import _run_research_cmd, _build_research_parser

# Re-export underscore-prefixed helpers for backward compatibility with tests
from cli.cmd.cite_fetch import _arxiv_doi_to_openalex
from cli.cmd.cite_graph import _extract_references_from_text as _extract_references_from_text_cite_graph
from cli.cmd.cite_import import _extract_references_from_text as _extract_references_from_text_cite_import
from cli.cmd.dedup_semantic import _get_ollama_embedding_batch, _run_dedup_semantic
from cli.cmd.dedup import _run_dedup, _build_dedup_parser
from cli.cmd.search import _run_search, _build_search_parser
from cli.cmd.list import _run_list, _build_list_parser
from cli.cmd.status import _run_status, _build_status_parser
from cli.cmd.queue import _run_queue, _build_queue_parser
from cli.cmd.cache import _run_cache, _build_cache_parser
from cli.cmd.merge import _run_merge, _build_merge_parser, _pick_keep
from cli.cmd.cite_graph import _run_cite_graph
from cli._registry import _main_legacy

# Alias for backward compat: _extract_references_from_text resolves to cite_graph version
_extract_references_from_text = _extract_references_from_text_cite_graph

__all__ = ["main"]
