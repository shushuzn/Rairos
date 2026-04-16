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
from pdf.extract import download_pdf, extract_pdf_text, extract_pdf_text_hybrid
from sections.segment import looks_like_heading, segment_into_sections, format_section_snippets
from llm.client import call_llm_chat_completions
from llm.generate import ai_generate_pnote_draft, ai_generate_cnote_draft
from renderers.pnote import render_pnote
from renderers.cnote import render_cnote
from renderers.mnote import render_mnote
from notes.frontmatter import parse_frontmatter, parse_tags_from_frontmatter, parse_date_from_frontmatter
from notes.pnotes import collect_pnotes, pnotes_by_tag, wikilink_for_pnote, read_pnote_metadata
from notes.cnote import ensure_cnote, upsert_link_under_heading, update_cnote_links
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

__all__ = [
    # core
    "Paper", "today_iso", "ensure_research_tree", "slugify_title", "safe_uid", "read_text", "write_text",
    # parsers
    "is_probably_doi", "normalize_doi", "normalize_arxiv_id", "fetch_arxiv_metadata", "fetch_crossref_metadata",
    # pdf
    "download_pdf", "extract_pdf_text", "extract_pdf_text_hybrid",
    # sections
    "looks_like_heading", "segment_into_sections", "format_section_snippets",
    # llm
    "call_llm_chat_completions", "ai_generate_pnote_draft", "ai_generate_cnote_draft",
    # renderers
    "render_pnote", "render_cnote", "render_mnote",
    # notes
    "parse_frontmatter", "parse_tags_from_frontmatter", "parse_date_from_frontmatter",
    "collect_pnotes", "pnotes_by_tag", "wikilink_for_pnote", "read_pnote_metadata",
    "ensure_cnote", "upsert_link_under_heading", "update_cnote_links",
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
