"""CLI command registry and main entry point."""
from __future__ import annotations

import argparse
import logging
import sys
from typing import List, Optional

from core.basics import get_default_concept_dir, get_default_radar_dir

logger = logging.getLogger(__name__)

SUBCOMMANDS = {
    "search", "list", "status", "queue", "cache", "dedup", "merge", "stats",
    "import", "export", "citations", "cite-graph", "cite-import", "cite-fetch",
    "cite-stats", "dedup-semantic", "research", "similar", "kg", "paper2code",
    "evoskill", "rag",
}


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

    # Import builders lazily to avoid loading all commands on startup
    from cli.cmd.search import _build_search_parser
    from cli.cmd.research import _build_research_parser
    from cli.cmd.list import _build_list_parser
    from cli.cmd.status import _build_status_parser
    from cli.cmd.stats import _build_stats_parser
    from cli.cmd.import_ import _build_import_parser
    from cli.cmd.export import _build_export_parser
    from cli.cmd.queue import _build_queue_parser
    from cli.cmd.dedup import _build_dedup_parser
    from cli.cmd.dedup_semantic import _build_dedup_semantic_parser
    from cli.cmd.similar import _build_similar_parser
    from cli.cmd.kg import _build_kg_parser
    from cli.cmd.merge import _build_merge_parser
    from cli.cmd.citations import _build_citations_parser
    from cli.cmd.cite_graph import _build_cite_graph_parser
    from cli.cmd.cite_fetch import _build_cite_fetch_parser
    from cli.cmd.cite_import import _build_cite_import_parser
    from cli.cmd.cite_stats import _build_cite_stats_parser
    from cli.cmd.cache import _build_cache_parser
    from cli.cmd.paper2code import _build_paper2code_parser
    from cli.cmd.evoskill import _build_evoskill_parser
    from cli.cmd.rag import _build_rag_parser

    _build_search_parser(subparsers)
    _build_research_parser(subparsers)
    _build_list_parser(subparsers)
    _build_status_parser(subparsers)
    _build_queue_parser(subparsers)
    _build_cache_parser(subparsers)
    _build_dedup_parser(subparsers)
    _build_merge_parser(subparsers)
    _build_stats_parser(subparsers)
    _build_import_parser(subparsers)
    _build_export_parser(subparsers)
    _build_citations_parser(subparsers)
    _build_cite_graph_parser(subparsers)
    _build_cite_import_parser(subparsers)
    _build_cite_fetch_parser(subparsers)
    _build_cite_stats_parser(subparsers)
    _build_dedup_semantic_parser(subparsers)
    _build_similar_parser(subparsers)
    _build_kg_parser(subparsers)
    _build_paper2code_parser(subparsers)
    _build_evoskill_parser(subparsers)
    _build_rag_parser(subparsers)

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

    args = parser.parse_args(argv if argv is not None else sys.argv[1:])

    # Resolve model default from config
    if getattr(args, "model", None) is None:
        try:
            from config import DEFAULT_LLM_MODEL_CLI as _cli_default
            args.model = _cli_default
        except Exception:
            args.model = "qwen3.5-plus"

    # Dispatch to command handlers (use cli namespace for test mockability)
    import cli
    if args.subcmd == "search":
        return cli._run_search(args)
    elif args.subcmd == "list":
        return cli._run_list(args)
    elif args.subcmd == "status":
        return cli._run_status(args)
    elif args.subcmd == "queue":
        return cli._run_queue(args)
    elif args.subcmd == "cache":
        return cli._run_cache(args)
    elif args.subcmd == "dedup":
        return cli._run_dedup(args)
    elif args.subcmd == "merge":
        return cli._run_merge(args)
    elif args.subcmd == "stats":
        return cli._run_stats(args)
    elif args.subcmd == "import":
        return cli._run_import(args)
    elif args.subcmd == "export":
        return cli._run_export(args)
    elif args.subcmd == "citations":
        return cli._run_citations(args)
    elif args.subcmd == "cite-graph":
        return cli._run_cite_graph(args)
    elif args.subcmd == "cite-import":
        return cli._run_cite_import(args)
    elif args.subcmd == "cite-fetch":
        return cli._run_cite_fetch(args)
    elif args.subcmd == "cite-stats":
        return cli._run_cite_stats(args)
    elif args.subcmd == "dedup-semantic":
        return cli._run_dedup_semantic(args)
    elif args.subcmd == "research":
        return cli._run_research_cmd(args)
    elif args.subcmd == "similar":
        return cli._run_similar(args)
    elif args.subcmd == "kg":
        return cli._run_kg(args)
    elif args.subcmd == "watch":
        from core.watch_papers import watch_and_rebuild
        papers_json = args.papers_json or None
        watch_and_rebuild(
            papers_json=papers_json,
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
    return 0


def _main_legacy(argv: Optional[List[str]] = None) -> int:
    """Legacy single-argument flow (arxiv ID/DOI directly)."""
    from cli._shared import print_error
    from parsers.input_detection import is_probably_doi, normalize_arxiv_id, normalize_doi
    from pdf.extract import download_pdf, extract_pdf_structured, extract_pdf_text_hybrid
    from sections.segment import segment_structured, format_section_snippets, format_tables_markdown, format_math_markdown, segment_into_sections
    from llm.generate import ai_generate_pnote_draft
    from renderers.pnote import render_pnote
    from updaters.radar import update_radar
    from updaters.timeline import update_timeline
    from notes.mnote import pick_top3_pnotes_for_tag, ensure_or_update_mnote
    from notes.keyword_tags import infer_tags_if_empty
    from core import Paper, DOI_RESOLVER, today_iso
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
        paper = DOI_RESOLVER.resolve(doi)
    else:
        arxiv_id = normalize_arxiv_id(raw)
        paper = Paper.from_arxiv(arxiv_id)

    tags = [t.strip() for t in args.tags.split(",") if t.strip()] if args.tags else []
    if not tags:
        tags = infer_tags_if_empty(tags, paper)

    root = Path(args.root)
    ensure_research_tree(root)
    pdf_path = Path(args.pdf) if args.pdf else None

    if not pdf_path:
        try:
            pdf_path = download_pdf(paper.pdf_url)
        except Exception as e:
            print_error(f"Failed to download PDF: {e}")
            return 1

    try:
        if args.structured:
            result = extract_pdf_structured(str(pdf_path), max_pages=args.max_pages)
        else:
            result = extract_pdf_text_hybrid(
                str(pdf_path),
                ocr=args.ocr,
                ocr_lang=args.ocr_lang,
                ocr_zoom=args.ocr_zoom,
                no_pdfminer=args.no_pdfminer,
                max_pages=args.max_pages,
            )
    except Exception as e:
        print_error(f"Failed to extract PDF: {e}")
        return 1

    segments = segment_into_sections(result["text"]) if args.structured else []
    snippets = format_section_snippets(segments) if args.structured else result["text"][:500]

    paper.path = pdf_path
    paper.pid = safe_uid(paper.arxiv_id or paper.doi or paper.title)

    pnote_path = root / args.category / f"{slugify_title(paper.title)}.md"
    pnote_path.parent.mkdir(parents=True, exist_ok=True)
    with open(pnote_path, "w", encoding="utf-8") as f:
        f.write(render_pnote(paper, result["text"], snippets, tags))

    update_radar(paper, root / args.category)
    update_timeline(paper, root / "99-Timeline")

    cnote_dir = root / args.concept_dir
    cnote_dir.mkdir(parents=True, exist_ok=True)
    top_pnotes = pick_top3_pnotes_for_tag(tags[0] if tags else "", cnote_dir)
    ensure_or_update_mnote(top_pnotes, cnote_dir, paper)

    if args.ai:
        ai_generate_pnote_draft(pnote_path, paper, model=args.model)

    return 0
