"""CLI command registry and main entry point."""
from __future__ import annotations

import argparse
import importlib
import logging
import sys
from typing import List, Optional, Callable, Dict

from pathlib import Path

from core.basics import get_default_concept_dir, get_default_radar_dir

logger = logging.getLogger(__name__)

# All available subcommands — derived from _SUBCOMMAND_TABLE
_SUBCOMMAND_TABLE = [
    ("search",         "cli.cmd.search",          "_build_search_parser"),
    ("research",       "cli.cmd.research",         "_build_research_parser"),
    ("list",          "cli.cmd.list",             "_build_list_parser"),
    ("status",        "cli.cmd.status",           "_build_status_parser"),
    ("queue",         "cli.cmd.queue",            "_build_queue_parser"),
    ("cache",         "cli.cmd.cache",            "_build_cache_parser"),
    ("dedup",         "cli.cmd.dedup",            "_build_dedup_parser"),
    ("dedup-semantic","cli.cmd.dedup_semantic",   "_build_dedup_semantic_parser"),
    ("similar",       "cli.cmd.similar",          "_build_similar_parser"),
    ("kg",            "cli.cmd.kg",               "_build_kg_parser"),
    ("merge",         "cli.cmd.merge",            "_build_merge_parser"),
    ("stats",         "cli.cmd.stats",           "_build_stats_parser"),
    ("import",        "cli.cmd.import_",          "_build_import_parser"),
    ("export",        "cli.cmd.export",           "_build_export_parser"),
    ("citations",      "cli.cmd.citations",         "_build_citations_parser"),
    ("cite-graph",    "cli.cmd.cite_graph",       "_build_cite_graph_parser"),
    ("cite-import",   "cli.cmd.cite_import",       "_build_cite_import_parser"),
    ("cite-fetch",    "cli.cmd.cite_fetch",       "_build_cite_fetch_parser"),
    ("cite-stats",    "cli.cmd.cite_stats",       "_build_cite_stats_parser"),
    ("paper2code",    "cli.cmd.paper2code",       "_build_paper2code_parser"),
    ("evoskill",      "cli.cmd.evoskill",         "_build_evoskill_parser"),
    ("rag",           "cli.cmd.rag",              "_build_rag_parser"),
    ("visual",        "cli.cmd.visual",           "_build_visual_parser"),
    ("repl",          "cli.cmd.repl",             "_build_repl_parser"),
    ("read-queue",    "cli.cmd.read_queue",       "_build_read_queue_parser"),
]
SUBCOMMANDS = {name for name, _, _ in _SUBCOMMAND_TABLE}

def _build_all_parsers(subparsers) -> None:
    """Build all subcommand parsers via lazy dynamic import."""
    import importlib
    for name, module_path, builder_name in _SUBCOMMAND_TABLE:
        mod = importlib.import_module(module_path)
        getattr(mod, builder_name)(subparsers)

    # Watch command (inline)
    p = subparsers.add_parser("watch", help="Watch papers.json and auto-rebuild KG on changes")
    p.add_argument(
        "--papers-json", default="",
        help="Path to papers.json (default: auto-detect)",
    )
    p.add_argument(
        "--poll-interval", type=float, default=5.0,
        help="Poll interval in seconds (default: 5)",
    )
    p.add_argument(
        "--no-incremental", action="store_true",
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
    elif first not in SUBCOMMANDS:
        import cli
        return cli._main_legacy(argv)

    parser = argparse.ArgumentParser(description="AI Research OS")
    subparsers = parser.add_subparsers(dest="subcmd", help="Subcommands")

    # Build all parsers
    _build_all_parsers(subparsers)

    args = parser.parse_args(argv if argv is not None else sys.argv[1:])

    # Resolve model default from config
    if getattr(args, "model", None) is None:
        try:
            from config import DEFAULT_LLM_MODEL_CLI as _cli_default
            args.model = _cli_default
        except Exception:
            args.model = "qwen3.5-plus"

    # Lazy dispatch — attribute name so test mocks on cli._run_X take effect
    dispatch = {
        "search": "_run_search", "list": "_run_list", "status": "_run_status",
        "queue": "_run_queue", "cache": "_run_cache", "dedup": "_run_dedup",
        "merge": "_run_merge", "stats": "_run_stats", "import": "_run_import",
        "export": "_run_export", "citations": "_run_citations",
        "cite-graph": "_run_cite_graph", "cite-import": "_run_cite_import",
        "cite-fetch": "_run_cite_fetch", "cite-stats": "_run_cite_stats",
        "dedup-semantic": "_run_dedup_semantic", "research": "_run_research_cmd",
        "similar": "_run_similar", "kg": "_run_kg",
        "read-queue": "_run_read_queue",
    }
    if args.subcmd in dispatch:
        import cli as _cli
        return getattr(_cli, dispatch[args.subcmd])(args)
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
    elif args.subcmd == "visual":
        from cli.cmd.visual import visual as visual_cmd
        return visual_cmd.main(args.argv if hasattr(args, "argv") else [])
    elif args.subcmd == "repl":
        import cli as _cli
        return getattr(_cli, "_run_repl")(args)
    return 0


def _main_legacy(argv: Optional[List[str]] = None) -> int:
    """Legacy single-argument flow (arxiv ID/DOI directly)."""
    from cli._shared import print_error
    from parsers.input_detection import is_probably_doi, normalize_arxiv_id, normalize_doi
    from parsers.arxiv import fetch_arxiv_metadata
    from parsers.crossref import fetch_crossref_metadata
    from sections.segment import segment_into_sections
    from renderers.pnote import render_pnote
    from updaters.radar import update_radar
    from updaters.timeline import update_timeline
    from notes.cnote import ensure_cnote, update_cnote_links
    from notes.mnote import ensure_or_update_mnote
    from notes.pnotes import pnotes_by_tag
    from notes.keyword_tags import infer_tags_if_empty
    from core import today_iso
    from core.basics import ensure_research_tree, get_default_concept_dir, get_default_radar_dir, safe_uid, slugify_title

    parser = argparse.ArgumentParser(
        description="AI Research OS - Full Flow (P+C+M+Radar+Timeline + optional AI draft)"
    )
    parser.add_argument("input", help="arXiv id/URL or DOI/doi.org URL")
    parser.add_argument("--root", default="AI-Research", help="Root folder for your research OS")
    parser.add_argument("--category", default="02-Models", help="Folder under root to place P-Note")
    parser.add_argument("--tags", default="", help="Comma-separated tags (recommended), e.g. LLM,Agent,RAG")
    parser.add_argument("--concept-dir", default=get_default_concept_dir(), help="Folder under root to place C-Notes")
    parser.add_argument("--comparison-dir", default=get_default_radar_dir(), help="Folder under root to place M-Notes")
    parser.add_argument("--max-pages", type=int, default=None, help="Max PDF pages to extract")
    parser.add_argument("--pdf", default="", help="Path to a local PDF (manual download). If set, skip PDF download.")
    parser.add_argument("--ocr", action="store_true", help="Enable OCR fallback per page (scanned PDFs).")
    parser.add_argument("--ocr-lang", default="chi_sim+eng", help="Tesseract language (default: chi_sim+eng).")
    parser.add_argument("--ocr-zoom", type=float, default=2.0, help="OCR render zoom (default: 2.0).")
    parser.add_argument("--no-pdfminer", action="store_true", help="Disable pdfminer fallback.")
    parser.add_argument("--structured", action="store_true", help="Use structured PDF extraction (tables/math separated).")
    parser.add_argument("--ai", action="store_true", help="Use AI to draft-fill P-Note sections (adds an AI draft block)")
    parser.add_argument("--ai-cnote", action="store_true", help="AI-fill all C-Notes from existing P-Notes (standalone mode; skips paper processing)")
    parser.add_argument("--model", default=None, help="LLM model override")
    parser.add_argument("--lang", default=None, help="Language code (e.g., zh, en)")

    args = parser.parse_args(argv)

    if args.lang:
        from core.i18n import set_lang
        set_lang(args.lang)

    raw = args.input.strip()
    if is_probably_doi(raw):
        doi = normalize_doi(raw)
        paper, _ = fetch_crossref_metadata(doi)
    else:
        arxiv_id = normalize_arxiv_id(raw)
        paper = fetch_arxiv_metadata(arxiv_id)

    tags = [t.strip() for t in args.tags.split(",") if t.strip()] if args.tags else []
    if not tags:
        tags = infer_tags_if_empty(tags, paper)

    root = Path(args.root)
    ensure_research_tree(root)
    pdf_path = Path(args.pdf) if args.pdf else None

    # Compute pid early for PDF filename
    paper.pid = safe_uid(paper.uid or paper.doi or paper.title)

    if not pdf_path:
        # Try to download PDF if URL is available
        if paper.pdf_url:
            try:
                from pdf.extract import download_pdf
                pdf_path = root / "cache" / f"{paper.pid}.pdf"
                download_pdf(paper.pdf_url, pdf_path)
            except Exception as e:
                print_error(f"Failed to download PDF: {e}")
                # Graceful degradation: continue without PDF
                pdf_path = None
        else:
            pdf_path = None

    # Extract text from PDF if available
    if pdf_path and pdf_path.exists():
        try:
            from pdf.extract import extract_pdf_structured, extract_pdf_text_hybrid
            if args.structured:
                result = extract_pdf_structured(str(pdf_path), max_pages=args.max_pages)
            else:
                extracted_text = extract_pdf_text_hybrid(
                    str(pdf_path),
                    max_pages=args.max_pages,
                    ocr=args.ocr,
                    ocr_lang=args.ocr_lang,
                    ocr_zoom=args.ocr_zoom,
                    use_pdfminer_fallback=not args.no_pdfminer,
                )
        except Exception as e:
            print_error(f"Failed to extract PDF: {e}")
            extracted_text = ""
    else:
        extracted_text = ""

    if args.structured:
        from sections.segment import segment_structured, format_section_snippets
        segments = segment_structured(extracted_text) if isinstance(extracted_text, str) else []
        snippets = format_section_snippets(segments)
    else:
        segments = []
        snippets = extracted_text[:500] if extracted_text else ""

    paper.path = pdf_path

    # P-note filename format: "P - {year} - {slugified_title}.md"
    year = paper.published[:4] if paper.published else today_iso()[:4]
    pnote_path = root / args.category / f"P - {year} - {slugify_title(paper.title)}.md"
    pnote_path.parent.mkdir(parents=True, exist_ok=True)
    with open(pnote_path, "w", encoding="utf-8") as f:
        f.write(render_pnote(paper, tags, extracted_text, parsed_ai=None))

    note_date = paper.published[:4] if paper.published else today_iso()
    update_radar(root, tags, note_date)
    update_timeline(root, year, pnote_path, paper.title)

    cnote_dir = root / args.concept_dir
    cnote_dir.mkdir(parents=True, exist_ok=True)

    # Create C-notes for each tag
    for tag in tags:
        cpath = ensure_cnote(cnote_dir, tag)
        update_cnote_links(cpath, pnote_path)

    # Build tag_map from existing pnotes
    tag_map = pnotes_by_tag(root)
    # Add current paper's pnote to the map
    first_tag = tags[0] if tags else ""
    if first_tag:
        if first_tag not in tag_map:
            tag_map[first_tag] = []
        tag_map[first_tag].append((paper.title, pnote_path))
    # Pick top3 and create mnote if 3+ papers share a tag
    top3 = None
    if first_tag and len(tag_map.get(first_tag, [])) >= 3:
        top3 = [p for _, p in sorted(tag_map[first_tag], key=lambda x: x[0])[:3]]
    if top3:
        ensure_or_update_mnote(cnote_dir, first_tag, top3)

    if args.ai:
        ai_generate_pnote_draft(pnote_path, paper, model=args.model)

    return 0
