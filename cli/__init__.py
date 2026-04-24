"""AI Research OS CLI."""
from db import Database
from cli._registry import main, _main_legacy
from cli._shared import (
    Colors, colored, print_success, print_error, print_warning,
    print_info, print_header, infer_tags_if_empty,
)

# Re-export _run_* functions for backward compatibility with tests
from cli.cmd.search import _run_search, _build_search_parser
from cli.cmd.list import _run_list
from cli.cmd.status import _run_status
from cli.cmd.stats import _run_stats
from cli.cmd.import_ import _run_import
from cli.cmd.export import _run_export
from cli.cmd.queue import _run_queue
from cli.cmd.cache import _run_cache
from cli.cmd.dedup import _run_dedup, _build_dedup_parser
from cli.cmd.dedup_semantic import _run_dedup_semantic
from cli.cmd.similar import _run_similar
from cli.cmd.kg import _run_kg
from cli.cmd.merge import _run_merge, _build_merge_parser, _pick_keep
from cli.cmd.citations import _run_citations
from cli.cmd.cite_graph import _run_cite_graph
from cli.cmd.cite_fetch import _run_cite_fetch
from cli.cmd.cite_import import _run_cite_import
from cli.cmd.cite_stats import _run_cite_stats
from cli.cmd.research import _run_research_cmd

__all__ = ["main"]
