"""AI Research OS - re-export from modules for backward compatibility."""
from core import Paper, today_iso
from core.basics import (
    ensure_research_tree,
    slugify_title,
    safe_uid,
    read_text,
    write_text,
)
from parsers.input_detection import is_probably_doi, normalize_doi, normalize_arxiv_id
from parsers.arxiv import fetch_arxiv_metadata, fetch_arxiv_metadata_batch
from parsers.crossref import fetch_crossref_metadata
from notes.frontmatter import parse_frontmatter, parse_tags_from_frontmatter, parse_date_from_frontmatter
from notes.pnotes import collect_pnotes, pnotes_by_tag, wikilink_for_pnote, read_pnote_metadata
from notes.cnote import ensure_cnote, upsert_link_under_heading, update_cnote_links, auto_fill_cnotes_with_ai
from notes.mnote import (
    pick_top3_pnotes_for_tag,
    mnote_filename,
    parse_current_abc,
    append_view_evolution_log,
    ensure_or_update_mnote,
)
from notes.keyword_tags import KEYWORD_TAGS, infer_tags_if_empty
from updaters.radar import ensure_radar, parse_radar_table, render_radar, update_radar
from updaters.timeline import ensure_timeline, update_timeline
from cli import main

# ─── Lazy module-level loading ────────────────────────────────────────────────
# Heavy modules (pdf, llm, renderers) loaded on first attribute access.
# This avoids ~1.2s of import overhead during test collection.
_LAZY_SUBMODULES = {
    "pdf.extract": ["download_pdf", "extract_pdf_text", "extract_pdf_text_hybrid"],
    "sections.segment": ["looks_like_heading", "segment_into_sections", "format_section_snippets"],
    "llm.client": ["call_llm_chat_completions"],
    "llm.generate": ["ai_generate_pnote_draft", "ai_generate_cnote_draft"],
    "renderers.pnote": ["render_pnote"],
    "renderers.cnote": ["render_cnote"],
    "renderers.mnote": ["render_mnote"],
}
_lazy_cache = {}


def __getattr__(name: str):
    """Lazy-load heavy submodules on first access."""
    if name in _lazy_cache:
        return _lazy_cache[name]

    for module_path, exports in _LAZY_SUBMODULES.items():
        if name in exports:
            import importlib
            mod = importlib.import_module(module_path)
            _lazy_cache[name] = getattr(mod, name)
            return _lazy_cache[name]

    raise AttributeError(f"module 'ai_research_os' has no attribute '{name}'")


__all__ = [
    # core
    "Paper", "today_iso", "ensure_research_tree", "slugify_title", "safe_uid", "read_text", "write_text",
    # parsers
    "is_probably_doi", "normalize_doi", "normalize_arxiv_id", "fetch_arxiv_metadata", "fetch_arxiv_metadata_batch", "fetch_crossref_metadata",
    # pdf (lazy-loaded)
    "download_pdf", "extract_pdf_text", "extract_pdf_text_hybrid",
    # sections (lazy-loaded)
    "looks_like_heading", "segment_into_sections", "format_section_snippets",
    # llm (lazy-loaded)
    "call_llm_chat_completions", "ai_generate_pnote_draft", "ai_generate_cnote_draft",
    # renderers (lazy-loaded)
    "render_pnote", "render_cnote", "render_mnote",
    # notes
    "parse_frontmatter", "parse_tags_from_frontmatter", "parse_date_from_frontmatter",
    "collect_pnotes", "pnotes_by_tag", "wikilink_for_pnote", "read_pnote_metadata",
    "ensure_cnote", "upsert_link_under_heading", "update_cnote_links", "auto_fill_cnotes_with_ai",
    "pick_top3_pnotes_for_tag", "mnote_filename", "parse_current_abc", "append_view_evolution_log", "ensure_or_update_mnote",
    "KEYWORD_TAGS", "infer_tags_if_empty",
    # updaters
    "ensure_radar", "parse_radar_table", "render_radar", "update_radar",
    "ensure_timeline", "update_timeline",
    # cli
    "main",
]

# Backward compatibility: allow `python ai_research_os.py` to still work
if __name__ == "__main__":
    import sys
    sys.exit(main())
