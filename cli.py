#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""AI Research OS CLI entry point."""

import argparse
import json
import os
import sys
from pathlib import Path
from typing import List, Optional

from db import Database
from db.database import SearchResult
from core import DOI_RESOLVER, Paper, today_iso
from core.basics import ensure_research_tree, safe_uid, slugify_title
from llm.generate import ai_generate_pnote_draft
from notes.cnote import auto_fill_cnotes_with_ai, ensure_cnote, update_cnote_links
from notes.mnote import ensure_or_update_mnote, pick_top3_pnotes_for_tag
from notes.pnotes import pnotes_by_tag
from parsers.arxiv import fetch_arxiv_metadata
from parsers.crossref import fetch_crossref_metadata
from parsers.input_detection import is_probably_doi, normalize_arxiv_id, normalize_doi
from pdf.extract import download_pdf, extract_pdf_text_hybrid, extract_pdf_structured
from renderers.pnote import render_pnote
from sections.segment import format_section_snippets, segment_into_sections, segment_structured, format_tables_markdown, format_math_markdown
from updaters.radar import update_radar
from updaters.timeline import update_timeline

KEYWORD_TAGS = [
    (r"\bagent(s)?\b|tool\s*use|function\s*calling", "Agent"),
    (r"\brag\b|retrieval\-augmented|retrieval augmented", "RAG"),
    (r"\bmoe\b|mixture of experts", "MoE"),
    (r"\brlhf\b|preference optimization|dpo\b", "Alignment"),
    (r"\bevaluation\b|benchmark", "Evaluation"),
    (r"\bcompiler\b|kernel|cuda|inference", "Infrastructure"),
    (r"\bmultimodal\b|vision|audio", "Multimodal"),
    (r"\bcompression\b|quantization|distillation", "Optimization"),
    (r"\blong context\b|context length", "LongContext"),
    (r"\bsafety\b|jailbreak|red teaming", "Safety"),
]

import re


def infer_tags_if_empty(tags: List[str], paper: Paper) -> List[str]:
    if tags:
        return tags
    text = f"{paper.title}\n{paper.abstract}".lower()
    out = []
    for pat, tg in KEYWORD_TAGS:
        if re.search(pat, text, flags=re.I):
            out.append(tg)
    return out if out else ["Unsorted"]


def _build_search_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser("search", help="Search indexed papers")
    p.add_argument("query", nargs="?", default="", help="Search query (optional)")
    p.add_argument("--limit", type=int, default=10, help="Max results")
    p.add_argument("--offset", type=int, default=0, help="Skip N results")
    p.add_argument("--format", choices=["table", "json", "csv"], default="table")
    p.add_argument("--source", default="", help="Filter by source (e.g. arxiv, doi)")
    p.add_argument("--year", type=int, default=0, help="Filter by year")
    p.add_argument("--tag", dest="tags", action="append", default=[], help="Filter by tag (repeatable)")
    p.add_argument("--status", default="", help="Filter by parse status")
    p.add_argument("--sort", choices=["relevance", "year", "title"], default="relevance")
    return p


def _run_search(args: argparse.Namespace) -> int:
    db = Database()
    db.init()

    results, total = db.search_papers(
        query=args.query or "",
        limit=args.limit,
        offset=args.offset,
        source=args.source or None,
        parse_status=args.status or None,
        date_from=f"{args.year}-01-01" if args.year > 0 else None,
    )

    if args.format == "json":
        out = []
        for r in results:
            out.append({
                "paper_id": r.paper_id,
                "title": r.title,
                "authors": r.authors,
                "published": r.published,
                "primary_category": r.primary_category,
                "score": round(r.score, 3) if r.score else None,
                "snippet": r.snippet,
                "source": r.source,
                "abs_url": r.abs_url,
                "pdf_url": r.pdf_url,
                "parse_status": r.parse_status,
            })
        print(json.dumps({"total": total, "results": out}, ensure_ascii=False, indent=2))
    elif args.format == "csv":
        import csv
        import sys as _sys
        writer = csv.writer(_sys.stdout)
        writer.writerow(["paper_id", "title", "authors", "published", "primary_category", "score", "snippet", "source", "abs_url", "parse_status"])
        for r in results:
            writer.writerow([r.paper_id, r.title, r.authors, r.published, r.primary_category,
                             round(r.score, 3) if r.score else "", r.snippet, r.source, r.abs_url, r.parse_status or ""])
    else:
        print(f"Found {total} papers, showing {len(results)}:")
        for r in results:
            score_str = f"[{r.score:.2f}]" if r.score else "     "
            print(f"  {score_str} {r.title}")
            print(f"         {r.authors}")
            print(f"         {r.published} | {r.source} | {r.primary_category}")
            if r.snippet:
                print(f"         ...{r.snippet}...")
            print()
    return 0


def _build_list_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser("list", help="List indexed papers")
    p.add_argument("--status", default="", help="Filter by parse status")
    p.add_argument("--year", type=int, default=0, help="Filter by year")
    p.add_argument("--tag", dest="tags", action="append", default=[], help="Filter by tag (repeatable)")
    p.add_argument("--limit", type=int, default=20, help="Max results")
    p.add_argument("--offset", type=int, default=0, help="Skip N results")
    p.add_argument("--format", choices=["table", "json"], default="table")
    return p


def _run_list(args: argparse.Namespace) -> int:
    db = Database()
    db.init()
    papers, total = db.list_papers(
        parse_status=args.status or None,
        limit=args.limit,
        offset=args.offset,
        date_from=f"{args.year}-01-01" if args.year > 0 else None,
    )
    if args.format == "json":
        out = [{"paper_id": p.id, "title": p.title, "authors": p.authors,
                "published": p.published, "primary_category": p.primary_category,
                "source": p.source, "abs_url": p.abs_url} for p in papers]
        print(json.dumps(out, ensure_ascii=False, indent=2))
    else:
        for p in papers:
            print(f"  {p.id:>5}  {p.published}  {p.source:<6}  {p.title}")
    return 0


def _build_status_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser("status", help="Show database status")
    return p


def _run_status(args: argparse.Namespace) -> int:
    db = Database()
    db.init()
    papers = db.get_papers(limit=10000)
    print(f"Total papers: {len(papers)}")
    by_source: dict[str, int] = {}
    by_status: dict[str, int] = {}
    for p in papers:
        by_source[p.source or "?"] = by_source.get(p.source or "?", 0) + 1
        by_status[p.parse_status or "?"] = by_status.get(p.parse_status or "?", 0) + 1
    print("By source:", ", ".join(f"{k}={v}" for k, v in sorted(by_source.items())))
    print("By status:", ", ".join(f"{k}={v}" for k, v in sorted(by_status.items())))
    return 0


def _build_queue_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser("queue", help="Manage job queue")
    p.add_argument("--add", metavar="UID", help="Add a paper UID to the queue")
    p.add_argument("--list", action="store_true", help="List pending jobs")
    p.add_argument("--dequeue", action="store_true", help="Pop next job from queue")
    p.add_argument("--cancel", metavar="JOB_ID", type=int, help="Cancel a queued job by id")
    return p


def _build_dedup_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser("dedup", help="Find duplicate papers in the database")
    p.add_argument("--dry-run", action="store_true", help="Show duplicates without merging")
    return p


def _run_dedup(args: argparse.Namespace) -> int:
    db = Database()
    db.init()
    pairs = db.find_duplicates()
    if not pairs:
        print("No duplicates found")
        return 0
    for older, newer in pairs:
        print(f"Duplicate pair: {older.id} / {newer.id}")
        print(f"  Title: {older.title[:80]}")
        print(f"  DOI: {older.doi or '(none)'}")
    if args.dry_run:
        print(f"\n({len(pairs)} duplicate pair(s), dry-run — no changes made)")
    else:
        print(f"\nUse 'paper-cli merge TARGET_ID DUPLICATE_ID' to merge each pair")
    return 0


def _build_merge_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser("merge", help="Merge a duplicate paper into a target paper")
    p.add_argument("target_id", metavar="TARGET_ID", help="ID of the paper to keep")
    p.add_argument("duplicate_id", metavar="DUPLICATE_ID", help="ID of the duplicate paper to absorb and delete")
    return p


def _run_merge(args: argparse.Namespace) -> int:
    db = Database()
    db.init()
    target = db.get_paper(args.target_id)
    if target is None:
        print(f"Target paper {args.target_id} not found")
        return 1
    duplicate = db.get_paper(args.duplicate_id)
    if duplicate is None:
        print(f"Duplicate paper {args.duplicate_id} not found")
        return 1
    ok = db.merge_papers(args.target_id, args.duplicate_id)
    if ok:
        print(f"Merged {args.duplicate_id} into {args.target_id}")
        return 0
    else:
        print(f"Merge failed")
        return 1


def _run_queue(args: argparse.Namespace) -> int:
    db = Database()
    db.init()
    if args.list:
        jobs = db.get_papers(limit=100)
        pending = [p.uid for p in jobs if p.parse_status == "pending"]
        if pending:
            print("Pending:", ", ".join(pending))
        else:
            print("Queue empty")
    elif args.dequeue:
        job = db.dequeue_job()
        if job:
            print(f"Dequeued: {job['paper_id']} (id={job['id']})")
        else:
            print("Queue empty")
    elif args.add:
        db.enqueue_job(args.add, "parse")
        print(f"Added {args.add} to queue")
    elif args.cancel is not None:
        removed = db.cancel_job(args.cancel)
        if removed:
            print(f"Cancelled job {args.cancel}")
        else:
            print(f"No such job {args.cancel}")
    else:
        print("Use --list, --dequeue, --add UID, or --cancel JOB_ID")
    return 0


def _build_cache_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser("cache", help="Manage paper cache")
    p.add_argument("--get", metavar="UID", help="Get cached paper by UID")
    p.add_argument("--set", nargs=2, metavar=("UID", "PATH"), help="Cache a paper from JSON")
    p.add_argument("--clear", action="store_true", help="Clear all cache entries")
    p.add_argument("--stats", action="store_true", help="Show cache statistics")
    return p


def _run_cache(args: argparse.Namespace) -> int:
    db = Database()
    db.init()
    if args.stats:
        entries = db.get_cached_paper("__stats__")
        print(f"Cache size: {entries}")
    elif args.clear:
        deleted = db.clear_cache()
        print(f"Cache cleared ({deleted} entries)")
    elif args.get:
        cached = db.get_cached_paper(args.get)
        if cached:
            print(json.dumps(cached, indent=2, ensure_ascii=False))
        else:
            print(f"No cache entry for {args.get}")
    elif getattr(args, "set", None):
        uid, path = args.set
        try:
            with open(path, encoding="utf-8") as f:
                data = json.load(f)
            db.set_cached_paper(uid, data)
            print(f"Cached {uid} from {path}")
        except Exception as e:
            print(f"Failed to cache {uid}: {e}")
            return 1
    else:
        print("Use --stats, --clear, --get UID, or --set UID PATH")
    return 0


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="AI Research OS")
    subparsers = parser.add_subparsers(dest="subcmd", help="Subcommands")

    # Build all subcommand parsers
    _build_search_parser(subparsers)
    _build_list_parser(subparsers)
    _build_status_parser(subparsers)
    _build_queue_parser(subparsers)
    _build_cache_parser(subparsers)
    _build_dedup_parser(subparsers)
    _build_merge_parser(subparsers)

    # Check if first arg is a known subcommand
    raw_args = argv if argv is not None else sys.argv[1:]
    first = raw_args[0] if raw_args else ""

    SUBCOMMANDS = {"search", "list", "status", "queue", "cache", "dedup", "merge"}

    if first in SUBCOMMANDS:
        # New subcommand flow
        parser = argparse.ArgumentParser(description="AI Research OS")
        subparsers = parser.add_subparsers(dest="subcmd", help="Subcommands")
        _build_search_parser(subparsers)
        _build_list_parser(subparsers)
        _build_status_parser(subparsers)
        _build_queue_parser(subparsers)
        _build_cache_parser(subparsers)
        _build_dedup_parser(subparsers)
        _build_merge_parser(subparsers)
        args = parser.parse_args(argv if argv is not None else sys.argv[1:])

        if args.subcmd == "search":
            return _run_search(args)
        elif args.subcmd == "list":
            return _run_list(args)
        elif args.subcmd == "status":
            return _run_status(args)
        elif args.subcmd == "queue":
            return _run_queue(args)
        elif args.subcmd == "cache":
            return _run_cache(args)
        elif args.subcmd == "dedup":
            return _run_dedup(args)
        elif args.subcmd == "merge":
            return _run_merge(args)
        return 0
    else:
        # Legacy flow (arxiv ID / DOI as first positional arg)
        return _main_legacy(argv)


def _main_legacy(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="AI Research OS - Full Flow (P+C+M+Radar+Timeline + optional AI draft)")
    parser.add_argument("input", help="arXiv id/URL or DOI/doi.org URL")
    parser.add_argument("--root", default="AI-Research", help="Root folder for your research OS")
    parser.add_argument("--category", default="02-Models", help="Folder under root to place P-Note")
    parser.add_argument("--tags", default="", help="Comma-separated tags (recommended), e.g. LLM,Agent,RAG")
    parser.add_argument("--concept-dir", default="01-Foundations", help="Folder under root to place C-Notes")
    parser.add_argument("--comparison-dir", default="00-Radar", help="Folder under root to place M-Notes")
    parser.add_argument("--max-pages", type=int, default=None, help="Max PDF pages to extract")

    # Local PDF (paywalled/subscription papers)
    parser.add_argument("--pdf", default="", help="Path to a local PDF (manual download). If set, skip PDF download.")

    # OCR (scanned/image PDFs)
    parser.add_argument("--ocr", action="store_true", help="Enable OCR fallback per page (scanned PDFs).")
    parser.add_argument("--ocr-lang", default="chi_sim+eng", help="Tesseract language (default: chi_sim+eng).")
    parser.add_argument("--ocr-zoom", type=float, default=2.0, help="OCR render zoom (default: 2.0).")
    parser.add_argument("--no-pdfminer", action="store_true", help="Disable pdfminer fallback.")

    # Structured PDF extraction
    parser.add_argument("--structured", action="store_true", help="Use structured PDF extraction (tables/math separated).")

    # AI draft options
    parser.add_argument("--ai", action="store_true", help="Use AI to draft-fill P-Note sections (adds an AI draft block)")
    parser.add_argument("--ai-cnote", action="store_true", help="AI-fill all C-Notes from existing P-Notes (standalone mode; skips paper processing)")
    parser.add_argument("--ai-max-papers", type=int, default=10, help="Max P-notes to feed per C-note (default: 10)")
    parser.add_argument("--api-key", default="", help="LLM API key (or set OPENAI_API_KEY env)")
    parser.add_argument("--model", default="qwen3.5-plus", help="LLM model name (OpenAI-compatible)")
    parser.add_argument("--base-url", default="https://dashscope.aliyuncs.com/compatible-mode/v1", help="OpenAI-compatible base url")
    parser.add_argument("--ai-max-chars", type=int, default=24000, help="Max chars of extracted text sent to AI")
    args = parser.parse_args(argv if argv is not None else sys.argv[1:])

    # Standalone C-note AI fill mode (no paper processing needed)
    if args.ai_cnote:
        api_key = args.api_key or os.environ.get("OPENAI_API_KEY", "")
        if not api_key:
            print("ERROR: --api-key or OPENAI_API_KEY required for --ai-cnote")
            return 1
        root = Path(args.root).resolve()
        results = auto_fill_cnotes_with_ai(
            root=root,
            api_key=api_key,
            base_url=args.base_url,
            model=args.model,
            min_papers=1,
        )
        print(f"OK: C-note AI fill done ({len(results)} concepts)")
        for concept, status in results:
            print(f"  - {concept}: {status}")
        return 0

    raw_in = args.input.strip()
    root = Path(args.root).resolve()
    ensure_research_tree(root)

    category_dir = root / args.category
    category_dir.mkdir(parents=True, exist_ok=True)

    concept_dir = root / args.concept_dir
    concept_dir.mkdir(parents=True, exist_ok=True)

    comparison_dir = root / args.comparison_dir
    comparison_dir.mkdir(parents=True, exist_ok=True)

    paper: Optional[Paper] = None
    arxiv_id = normalize_arxiv_id(raw_in)

    # DOI flow: prioritize arXiv DOI before Crossref
    if is_probably_doi(raw_in):
        doi = normalize_doi(raw_in)
        arxiv_from_doi = normalize_arxiv_id(doi)
        if arxiv_from_doi:
            paper = fetch_arxiv_metadata(arxiv_from_doi)
        else:
            doi_paper, maybe_arxiv = fetch_crossref_metadata(doi)
            paper = fetch_arxiv_metadata(maybe_arxiv) if maybe_arxiv else doi_paper

    elif arxiv_id:
        paper = fetch_arxiv_metadata(arxiv_id)

    else:
        m = re.search(r"(\d{4}\.\d{4,5}(v\d+)?)", raw_in)
        if m:
            paper = fetch_arxiv_metadata(m.group(1))
        else:
            # Allow placeholder input (e.g. "test") with --pdf
            paper = Paper(
                source="doi",
                uid=raw_in,
                title=raw_in,
                authors=[],
                abstract="",
                published="",
                updated="",
                abs_url=DOI_RESOLVER + raw_in,
                pdf_url="",
                primary_category="",
            )

    tags = [t.strip() for t in args.tags.split(",") if t.strip()]
    tags = infer_tags_if_empty(tags, paper)

    note_date = paper.published or today_iso()
    year = (note_date[:4] if len(note_date) >= 4 else str(__import__('datetime').date.today().year))
    title_slug = slugify_title(paper.title)

    pnote_name = f"P - {year} - {title_slug}.md"
    pnote_path = category_dir / pnote_name

    # Default: download PDF to _assets/{uid}/
    assets_dir = category_dir / "_assets" / safe_uid(paper.uid)
    default_pdf_path = assets_dir / (safe_uid(paper.uid) + ".pdf")

    extracted_sections_md = ""
    extracted_text_for_ai = ""
    pdf_downloaded = False
    table_md = ""
    math_md = ""

    if args.pdf:
        pdf_path = Path(args.pdf).expanduser().resolve()
        if not pdf_path.exists():
            raise FileNotFoundError(f"--pdf not found: {pdf_path}")
        pdf_downloaded = True
        paper.pdf_url = str(pdf_path)
    else:
        pdf_path = default_pdf_path
        if paper.pdf_url:
            try:
                download_pdf(paper.pdf_url, pdf_path)
                pdf_downloaded = True
            except Exception as e:
                extracted_sections_md = f"_PDF 下载失败：{e}_"
        else:
            extracted_sections_md = "_未提供可直接下载的 PDF 链接（常见于 DOI-only 元数据），已跳过 PDF 抽取。_"

    if pdf_downloaded:
        try:
            if args.structured:
                sdoc = extract_pdf_structured(
                    pdf_path,
                    max_pages=args.max_pages,
                )
                sections = segment_structured(sdoc)
                extracted_sections_md = format_section_snippets(sections)
                table_md = format_tables_markdown(sdoc)
                math_md = format_math_markdown(sdoc)
                # Provide text to AI draft
                extracted_text_for_ai = "\n".join(b.text for b in sdoc.text_blocks)
            else:
                txt = extract_pdf_text_hybrid(
                    pdf_path,
                    max_pages=args.max_pages,
                    ocr=args.ocr,
                    ocr_lang=args.ocr_lang,
                    ocr_zoom=args.ocr_zoom,
                    use_pdfminer_fallback=(not args.no_pdfminer),
                )
                extracted_text_for_ai = txt
                sections = segment_into_sections(txt)
                extracted_sections_md = format_section_snippets(sections)
                table_md = ""
                math_md = ""
        except Exception as e:
            extracted_sections_md = f"_PDF 抽取失败：{e}_"
            table_md = ""
            math_md = ""

    # AI draft generation (optional)
    ai_draft_md = ""
    parsed_ai = None
    if args.ai:
        ctx = extracted_text_for_ai.strip() or (paper.abstract or "")
        ctx = ctx[: max(1000, args.ai_max_chars)]
        try:
            from llm.parse import parse_ai_pnote_draft
            raw_draft = ai_generate_pnote_draft(
                paper=paper,
                tags=tags,
                extracted_text=ctx[: args.ai_max_chars],
                base_url=args.base_url,
                api_key=args.api_key,
                model=args.model,
            )
            sections_dict, rubric_dict, raw_draft = parse_ai_pnote_draft(raw_draft)
            ai_draft_md = raw_draft  # full raw output for reference
            parsed_ai = (sections_dict, rubric_dict)
        except Exception as e:
            ai_draft_md = (
                "> AI Draft（生成失败，需人工核验）\n\n"
                f"- 错误：{e}\n"
                "- 建议：检查 OPENAI_API_KEY / --api-key / --base-url / --model\n"
            )

    from core.basics import write_text
    write_text(pnote_path, render_pnote(paper, tags, extracted_sections_md, ai_draft_md=ai_draft_md, table_md=table_md, math_md=math_md, parsed_ai=parsed_ai))

    # C-Notes create/update + link P-Note
    cnote_paths = []
    for t in tags:
        cpath = ensure_cnote(concept_dir, t)
        update_cnote_links(cpath, pnote_path)
        cnote_paths.append(cpath)

    # Radar update
    radar_path = update_radar(root, tags, note_date)

    # Timeline update
    timeline_path = update_timeline(root, year, pnote_path, paper.title)

    # M-Note trigger/update
    tag_map = pnotes_by_tag(root)
    mnote_paths = []
    for t in tags:
        top3 = pick_top3_pnotes_for_tag(t, tag_map)
        if top3:
            mpath = ensure_or_update_mnote(comparison_dir, t, top3)
            if mpath:
                from core.basics import read_text as _read_text
                cpath = concept_dir / f"C - {t}.md"
                cmd = _read_text(cpath)
                mlink = f"[[{mpath.stem}]]"
                from notes.cnote import upsert_link_under_heading
                cmd2 = upsert_link_under_heading(cmd, "关联笔记", mlink)
                from core.basics import write_text as _write_text
                _write_text(cpath, cmd2)
                mnote_paths.append(mpath)

    # Print summary
    print("OK: AI Research OS Flow Done")
    print(f"- P-Note: {pnote_path}")
    if pdf_downloaded:
        print(f"- PDF   : {pdf_path}")
    else:
        print("- PDF   : (not downloaded)")
    print(f"- Radar : {radar_path}")
    print(f"- Timeline: {timeline_path}")

    if cnote_paths:
        print("- C-Notes:")
        for p in cnote_paths:
            print(f"  - {p}")

    if mnote_paths:
        print("- M-Notes:")
        for p in mnote_paths:
            print(f"  - {p}")
    else:
        print("- M-Notes: (no tag reached 3 P-Notes yet)")

    if args.ai:
        print("- AI Draft: ENABLED (see P-Note section: 'AI 自动初稿（待核验）')")

    return 0


if __name__ == "__main__":
    sys.exit(main())
