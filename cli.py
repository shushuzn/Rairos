#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""AI Research OS CLI entry point."""

import argparse
import json
import logging
import os
import re
import ssl
import sys
import time
import urllib.request
import warnings
from pathlib import Path
from dataclasses import dataclass
from typing import Any, Dict, List, Literal, Optional, Tuple

from db import Database
from db.database import ExperimentTableRecord
from core import DOI_RESOLVER, Paper, today_iso
from core.basics import ensure_research_tree, get_default_concept_dir, get_default_radar_dir, safe_uid, slugify_title
from llm.generate import ai_generate_pnote_draft
from notes.cnote import auto_fill_cnotes_with_ai, ensure_cnote, update_cnote_links
from notes.mnote import ensure_or_update_mnote, pick_top3_pnotes_for_tag
from notes.pnotes import pnotes_by_tag
from parsers.arxiv import fetch_arxiv_metadata
from parsers.crossref import fetch_crossref_metadata
from parsers.input_detection import is_probably_doi, normalize_arxiv_id, normalize_doi
from pdf.extract import download_pdf, extract_pdf_text_hybrid, extract_pdf_structured
from research_loop import run_research
from renderers.pnote import render_pnote
from sections.segment import format_section_snippets, segment_into_sections, segment_structured, format_tables_markdown, format_math_markdown
from updaters.radar import update_radar
from updaters.timeline import update_timeline

_KEYWORD_TAG_PATTERNS = [
    (re.compile(r"\bagent(s)?\b|tool\s*use|function\s*calling", re.I), "Agent"),
    (re.compile(r"\brag\b|retrieval\-augmented|retrieval augmented", re.I), "RAG"),
    (re.compile(r"\bmoe\b|mixture of experts", re.I), "MoE"),
    (re.compile(r"\brlhf\b|preference optimization|dpo\b", re.I), "Alignment"),
    (re.compile(r"\bevaluation\b|benchmark", re.I), "Evaluation"),
    (re.compile(r"\bcompiler\b|kernel|cuda|inference", re.I), "Infrastructure"),
    (re.compile(r"\bmultimodal\b|vision|audio", re.I), "Multimodal"),
    (re.compile(r"\bcompression\b|quantization|distillation", re.I), "Optimization"),
    (re.compile(r"\blong context\b|context length", re.I), "LongContext"),
    (re.compile(r"\bsafety\b|jailbreak|red teaming", re.I), "Safety"),
]


def infer_tags_if_empty(tags: List[str], paper: Paper) -> List[str]:
    if tags:
        return tags
    text = f"{paper.title}\n{paper.abstract}".lower()
    out = []
    for pat, tg in _KEYWORD_TAG_PATTERNS:
        if pat.search(text):
            out.append(tg)
    return out if out else ["Unsorted"]


def _build_research_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "research",
        help="Autonomous research loop: search arXiv → download → extract → AI summarize",
    )
    p.add_argument("query", nargs="?", default="", help="Research topic or keyword")
    p.add_argument("--limit", type=int, default=5, help="Max papers to process (default 5)")
    p.add_argument(
        "--output",
        type=str,
        default="",
        help="Output directory (default: ~/ai_research/<query-slug>/)",
    )
    p.add_argument(
        "--no-ai",
        dest="no_ai",
        action="store_true",
        default=False,
        help="Skip AI draft generation (metadata only)",
    )
    p.add_argument(
        "--no-pdf",
        dest="no_pdf",
        action="store_true",
        default=False,
        help="Skip PDF download (use abstract only)",
    )
    p.add_argument("--tag", dest="tags", action="append", default=[], help="Tags to assign (repeatable)")
    p.add_argument(
        "--model",
        type=str,
        default="gpt-4o-mini",
        help="LLM model for AI drafts (default gpt-4o-mini)",
    )
    p.add_argument(
        "--base-url",
        type=str,
        default="",
        help="LLM API base URL (default: OPENAI_BASE_URL env var)",
    )
    p.add_argument(
        "--api-key",
        type=str,
        default="",
        help="LLM API key (default: OPENAI_API_KEY env var)",
    )
    p.add_argument(
        "--no-skip",
        dest="no_skip",
        action="store_true",
        default=False,
        help="Re-generate even if note already exists",
    )
    p.add_argument("-v", "--verbose", action="store_true", default=False, help="Verbose output")
    p.add_argument("--lang", type=str, default="zh", choices=["en", "zh", "e", "z"], help="Output language (default: zh)")
    p.add_argument(
        "--async",
        dest="async_mode",
        action="store_true",
        default=False,
        help="Use async I/O for concurrent PDF download and LLM streaming",
    )
    p.add_argument(
        "--progress",
        dest="progress",
        action="store_true",
        default=False,
        help="Print LLM streaming tokens in real time (implies --async)",
    )
    return p


def _run_research_cmd(args: argparse.Namespace) -> int:
    import asyncio
    import sys

    from core.i18n import set_lang
    from research_loop import arun_research

    set_lang(args.lang)

    # --progress implies --async
    if args.progress:
        args.async_mode = True

    output_dir = Path(args.output) if args.output else None
    tags = args.tags if args.tags else []
    skip_existing = not args.no_skip

    # progress_callback prints each streaming chunk to stdout
    progress_cb = sys.stdout.write if args.progress else None

    if args.async_mode:
        paths = asyncio.run(
            arun_research(
                query=args.query,
                limit=args.limit,
                output_dir=output_dir,
                download_pdfs=not args.no_pdf,
                skip_existing=skip_existing,
                tags=tags,
                model=args.model,
                base_url=args.base_url or None,
                api_key=(args.api_key or None) if not args.no_ai else "",
                verbose=args.verbose,
                progress_callback=progress_cb,
            )
        )
    else:
        paths = run_research(
            query=args.query,
            limit=args.limit,
            output_dir=output_dir,
            download_pdfs=not args.no_pdf,
            skip_existing=skip_existing,
            tags=tags,
            model=args.model,
            base_url=args.base_url or None,
            api_key=(args.api_key or None) if not args.no_ai else "",
            verbose=args.verbose,
        )

    if not paths:
        print("No notes generated.")
        return 1

    print(f"\nGenerated {len(paths)} note(s):")
    for p in paths:
        print(f"  {p}")
    return 0


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
    p.add_argument("--format", choices=["table", "json", "csv"], default="table")
    p.add_argument(
        "--sort",
        choices=["added_at", "published", "title", "parse_status"],
        default="added_at",
        help="Sort field (default: added_at)",
    )
    p.add_argument("--order", choices=["asc", "desc"], default="desc", help="Sort order")
    return p


def _run_list(args: argparse.Namespace) -> int:
    db = Database()
    db.init()
    papers, total = db.list_papers(
        parse_status=args.status or None,
        limit=args.limit,
        offset=args.offset,
        date_from=f"{args.year}-01-01" if args.year > 0 else None,
        sort_by=args.sort,
        sort_order=args.order,
    )
    if args.format == "csv":
        import csv
        import sys as _sys
        writer = csv.writer(_sys.stdout)
        writer.writerow(["id", "title", "authors", "published", "source", "primary_category", "parse_status", "added_at"])
        for p in papers:
            writer.writerow([p.id, p.title, p.authors, p.published, p.source,
                             p.primary_category, p.parse_status or "", p.added_at])
    elif args.format == "json":
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


def _build_stats_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser("stats", help="Show database statistics summary")
    p.add_argument("--json", action="store_true", help="Output as JSON")
    return p


def _run_stats(args: argparse.Namespace) -> int:
    db = Database()
    db.init()
    s = db.get_stats()
    if args.json:
        print(json.dumps(s, indent=2, ensure_ascii=False))
    else:
        print("Papers:")
        print(f"  total : {s['total_papers']}")
        print(f"  by source : {', '.join(f'{k}={v}' for k, v in sorted(s['by_source'].items()))}")
        print(f"  by status : {', '.join(f'{k}={v}' for k, v in sorted(s['by_status'].items()))}")
        print("Queue:")
        print(f"  queued  : {s['queue_queued']}")
        print(f"  running : {s['queue_running']}")
        print("Cache:")
        print(f"  entries : {s['cache_entries']}")
    print("Dedup:")
    print(f"  records : {s['dedup_records']}")
    return 0


def _build_import_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser("import", help="Add papers to the database by ID")
    p.add_argument("ids", nargs="*", metavar="ID", help="arXiv IDs, DOIs, or paper UIDs to add")
    p.add_argument("--source", default="import", help="Source label (default: import)")
    p.add_argument("--skip-existing", action="store_true", help="Skip IDs already in database")
    p.add_argument("--file", metavar="FILE", help="Read IDs from file (one per line), or '-' for stdin")
    return p


def _run_import(args: argparse.Namespace) -> int:
    db = Database()
    db.init()

    # Resolve paper IDs: from file or positional args
    _has_file = bool(getattr(args, "file", None))
    if _has_file:
        import io as _io
        if args.file == "-":
            _source = _io.StringIO(sys.stdin.read())
        else:
            _source = open(args.file, encoding="utf-8")
        try:
            raw = _source.read()
        finally:
            if _source is not sys.stdin:
                _source.close()
        paper_ids = [line.strip() for line in raw.splitlines() if line.strip()]
    else:
        paper_ids = getattr(args, "ids", []) or []

    if not paper_ids:
        if not _has_file and not getattr(args, "ids", []):
            print("Error: no IDs provided (use positional IDs, --file, or pipe into stdin)", file=sys.stderr)
            return 1
        # file provided but empty — not an error, just nothing to do

    added, skipped, failed = 0, 0, 0
    # Bulk check which papers already exist (single query instead of N)
    existing = db.get_papers_bulk(paper_ids)

    # Separate existing vs missing
    existing_ids = {pid for pid in paper_ids if pid.strip() in existing}
    missing_ids = [pid.strip() for pid in paper_ids if pid.strip() not in existing]

    for paper_id in existing_ids:
        if args.skip_existing:
            skipped += 1
            print(f"Skipped (exists): {paper_id}")
        else:
            skipped += 1
            print(f"Skipped (exists): {paper_id}")

    # Parallel bulk upsert for missing papers
    if missing_ids:
        def _upsert_one(pid: str) -> Tuple[str, bool, str]:
            try:
                db.upsert_paper(pid, args.source)
                return pid, True, ""
            except Exception as e:
                return pid, False, str(e)

        from concurrent.futures import ThreadPoolExecutor
        with ThreadPoolExecutor(max_workers=min(len(missing_ids), 8)) as ex:
            for pid, ok, err in ex.map(_upsert_one, missing_ids):
                if ok:
                    added += 1
                    print(f"Added: {pid}")
                else:
                    failed += 1
                    print(f"Failed: {pid} — {err}")

    print(f"\nImport done: {added} added, {skipped} skipped, {failed} failed")
    return 0


def _build_export_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser("export", help="Export all papers to CSV or JSON")
    p.add_argument("--format", choices=["csv", "json"], default="csv", help="Output format (default: csv)")
    p.add_argument("--limit", type=int, default=0, help="Limit number of papers (0 = all)")
    p.add_argument("--out", metavar="FILE", help="Write to file instead of stdout")
    return p


def _run_export(args: argparse.Namespace) -> int:
    db = Database()
    db.init()
    fields, rows = db.export_papers(format=args.format, limit=args.limit)
    import csv as _csv
    import io as _io
    output = _io.StringIO()
    if args.format == "csv":
        writer = _csv.DictWriter(output, fieldnames=fields)
        writer.writeheader()
        writer.writerows(rows)
        content = output.getvalue()
    else:
        import json as _json
        content = _json.dumps(rows, ensure_ascii=False, indent=2)
    if args.out:
        Path(args.out).write_text(content, encoding="utf-8")
        print(f"Exported {len(rows)} papers to {args.out}")
    else:
        print(content)
    return 0


def _build_queue_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser("queue", help="Manage job queue")
    p.add_argument("--add", metavar="UID", help="Add a paper UID to the queue")
    p.add_argument("--list", action="store_true", help="List pending jobs")
    p.add_argument("--dequeue", action="store_true", help="Pop next job from queue")
    p.add_argument("--cancel", metavar="JOB_ID", type=int, help="Cancel a queued job by id")
    p.add_argument("--clear", action="store_true", help="Clear all queued jobs")
    return p


def _build_dedup_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser("dedup", help="Find duplicate papers in the database")
    g = p.add_mutually_exclusive_group()
    g.add_argument("--dry-run", action="store_true", help="Show duplicates without merging")
    g.add_argument("--auto", action="store_true", help="Automatically merge every duplicate pair")
    g.add_argument("--batch", action="store_true", help="Auto-merge safe pairs (same DOI), skip the rest")
    p.add_argument(
        "--keep",
        choices=["older", "newer", "parsed"],
        default="older",
        help="Which paper to keep: 'older' (default, keeps paper with earlier added_at), "
             "'newer' (keeps paper with later added_at), or 'parsed' (keeps paper with better parse_status)",
    )
    p.add_argument("--report", action="store_true", help="Show dedup history log")
    p.add_argument(
        "--since",
        metavar="YYYY-MM-DD",
        default="",
        help="Only consider papers added on or after this date",
    )
    return p


def _run_dedup(args: argparse.Namespace) -> int:
    db = Database()
    db.init()
    if args.report:
        logs = db.get_dedup_log()
        if not logs:
            print("No dedup history")
            return 0
        print(f"Dedup history ({len(logs)} record(s)):")
        for r in logs:
            print(f"  [{r['id']}] {r['logged_at']}  keep={r['keep_policy']}  kept={r['target_id']}  merged={r['duplicate_id']}")
            print(f"    kept title:   {r['target_title'][:70]}")
            print(f"    merged title: {r['duplicate_title'][:70]}")
        return 0
    pairs = db.find_duplicates(since=args.since or None)
    if not pairs:
        print("No duplicates found")
        return 0
    for older, newer in pairs:
        parsed_rank = {"completed": 4, "running": 3, "pending": 2, "failed": 1}
        def rank(p, _rank=parsed_rank):
            return _rank.get(p.parse_status, 0)
        parsed_winner = older if rank(older) >= rank(newer) else newer
        print(f"Duplicate pair: {older.id} / {newer.id}")
        print(f"  Title: {older.title[:80]}")
        print(f"  DOI: {older.doi or '(none)'}")
        print(f"  [{older.id}] status={older.parse_status:<10} added_at={older.added_at}")
        print(f"  [{newer.id}] status={newer.parse_status:<10} added_at={newer.added_at}")
        if args.dry_run:
            # Show keep decision for current --keep setting
            target, dup = _pick_keep(older, newer, args.keep)
            print(f"  --> would keep [{target.id}], merge [{dup.id}] (--keep={args.keep})")
            print(f"  --> parsed winner: [{parsed_winner.id}] (status={parsed_winner.parse_status})")
        print()
    if args.dry_run:
        print(f"({len(pairs)} duplicate pair(s), dry-run — no changes made)")
        return 0
    if args.auto:
        merged = 0
        for older, newer in pairs:
            target, duplicate = _pick_keep(older, newer, args.keep)
            ok = db.merge_papers(target.id, duplicate.id)
            if ok:
                db.log_dedup(target.id, duplicate.id, args.keep)
                print(f"Auto-merged {duplicate.id} into {target.id} (--keep={args.keep})")
                merged += 1
            else:
                print(f"Failed to merge {duplicate.id} into {target.id}")
        print(f"\nAuto-merged {merged}/{len(pairs)} pair(s)")
        return 0
    if args.batch:
        merged, skipped = 0, 0
        for older, newer in pairs:
            if older.doi and older.doi == newer.doi:
                target, duplicate = _pick_keep(older, newer, args.keep)
                ok = db.merge_papers(target.id, duplicate.id)
                if ok:
                    db.log_dedup(target.id, duplicate.id, args.keep)
                    print(f"[batch] Merged {duplicate.id} -> {target.id} (same DOI)")
                    merged += 1
                else:
                    print(f"[batch] Failed: {duplicate.id} -> {target.id}")
                    skipped += 1
            else:
                print(f"[batch] Skipped {older.id}/{newer.id} (no matching DOI)")
                skipped += 1
        print(f"\nBatch: {merged} merged, {skipped} skipped ({len(pairs)} total pairs)")
        return 0
    print("Use 'paper-cli merge TARGET_ID DUPLICATE_ID' to merge each pair")
    return 0


def _build_dedup_semantic_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "dedup-semantic",
        help="Find near-duplicate papers using semantic embeddings",
    )
    p.add_argument(
        "--threshold",
        type=float,
        default=0.85,
        help="Minimum cosine similarity threshold (default: 0.85)",
    )
    p.add_argument(
        "--limit",
        type=int,
        default=20,
        help="Maximum similar papers to find per query (default: 20)",
    )
    p.add_argument(
        "--paper",
        metavar="PAPER_ID",
        help="Check similarity for a specific paper only",
    )
    p.add_argument(
        "--generate",
        action="store_true",
        help="Generate embeddings for papers that don't have them yet",
    )
    p.add_argument(
        "--dry-run",
        action="store_true",
        help="Show what would happen without making changes",
    )
    p.add_argument(
        "--stats",
        action="store_true",
        help="Show embedding coverage statistics",
    )
    p.add_argument(
        "--format",
        choices=["text", "csv"],
        default="text",
        help="Output format: 'text' (default) or 'csv'",
    )
    return p


def _build_similar_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "similar",
        help="Find papers similar to a given paper using embedding similarity",
    )
    p.add_argument(
        "paper_id",
        nargs="?",
        default="",
        help="Paper ID (e.g. 2301.001)",
    )
    p.add_argument(
        "--threshold", type=float, default=0.85,
        help="Minimum cosine similarity (default: 0.85)",
    )
    p.add_argument(
        "--limit", type=int, default=10,
        help="Max similar papers to return (default: 10)",
    )
    p.add_argument(
        "--format", choices=["table", "json"], default="table",
        help="Output format (default: table)",
    )
    return p


def _run_similar(args: argparse.Namespace) -> int:
    db = Database()
    db.init()

    if not args.paper_id:
        stats = db.get_embedding_stats()
        papers_without_emb = stats.get("total_with_text", 0) - stats.get("with_embedding", 0)
        print("Semantic similarity search requires embeddings.")
        print(f"Database stats: {stats.get('with_embedding', 0)} papers have embeddings "
              f"({papers_without_emb} still need them).")
        print()
        print("To generate embeddings for papers without them, use:")
        print("  ai_research_os import --embed <paper_id>")
        print("Or run the embedding pipeline on your research root.")
        return 0

    sims = db.find_similar(args.paper_id, threshold=args.threshold, limit=args.limit)

    if not sims:
        print(f"No papers found with similarity >= {args.threshold} to '{args.paper_id}'.")
        return 0

    if args.format == "json":
        out = []
        for paper, score in sims:
            out.append({
                "paper_id": paper.paper_id,
                "title": paper.title,
                "authors": paper.authors,
                "published": paper.published,
                "similarity": round(score, 4),
                "source": paper.source,
                "primary_category": paper.primary_category,
            })
        print(json.dumps(out, ensure_ascii=False, indent=2))
    else:
        print(f"Found {len(sims)} papers similar to '{args.paper_id}' (threshold={args.threshold}):\n")
        for paper, score in sims:
            bar_len = int(score * 10)
            bar = "=" * bar_len + "-" * (10 - bar_len)
            print(f"  [{score:.3f}] {bar}  {paper.paper_id}")
            print(f"         {paper.title}")
            if paper.authors:
                print(f"         {paper.authors[:60]}")
            print(f"         {paper.published or 'n.d.'} | {paper.source or ''} | {paper.primary_category or ''}")
            print()

    return 0



def _get_ollama_embedding(text: str, model: str = "nomic-embed-text") -> Optional[List[float]]:
    """Fetch embedding from local Ollama. Returns None on failure."""
    try:
        req = urllib.request.Request(
            "http://localhost:11434/api/embeddings",
            data=json.dumps({"model": model, "prompt": text}).encode(),
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=60) as resp:
            data = json.loads(resp.read().decode())
            return data.get("embedding")
    except Exception as e:
        print(f"  [WARN] Ollama embedding failed: {e}", file=sys.stderr)
        return None


def _get_ollama_embedding_batch(
    texts: List[str],
    model: str = "nomic-embed-text",
    batch_size: int = 32,
) -> List[Optional[List[float]]]:
    """Fetch embeddings for multiple texts in one Ollama API call.

    Falls back to individual /api/embeddings calls if the batch endpoint
    returns a non-JSON response (e.g. 502 Bad Gateway from an older model
    that does not support multi-prompt batching).
    Returns list parallel to input; None for failed items.
    """
    results: List[Optional[List[float]]] = [None] * len(texts)
    for i in range(0, len(texts), batch_size):
        batch = texts[i : i + batch_size]
        try:
            req = urllib.request.Request(
                "http://localhost:11434/api/embed",
                data=json.dumps({"model": model, "prompt": batch}).encode(),
                headers={"Content-Type": "application/json"},
                method="POST",
            )
            with urllib.request.urlopen(req, timeout=120) as resp:
                raw = resp.read().decode()
                try:
                    data = json.loads(raw)
                except json.JSONDecodeError:
                    # Batch endpoint not supported (old model or 502) — fall back to single
                    for j, text in enumerate(batch):
                        single = _get_ollama_embedding(text, model)
                        results[i + j] = single
                    continue
                embeddings = data.get("embeddings") or []
                for j, emb in enumerate(embeddings):
                    results[i + j] = emb
        except Exception:
            # Network or server error — fall back to single calls
            for j, text in enumerate(batch):
                single = _get_ollama_embedding(text, model)
                results[i + j] = single
    return results



def _generate_missing_embeddings(
    db: "Database",
    delay: float = 0.0,
    batch_size: int = 32,
    max_workers: int = 8,
) -> Tuple[int, int]:
    """Generate embeddings for papers missing them using batch Ollama API.

    Returns (generated, failed).
    """
    from concurrent.futures import ThreadPoolExecutor

    papers = db.get_papers_without_embeddings(limit=1000)
    if not papers:
        return 0, 0

    # Prepare texts
    paper_ids = []
    texts = []
    for paper in papers:
        text = (paper.title or "") + ("\n\n" + paper.abstract if paper.abstract else "")
        if text.strip():
            paper_ids.append(paper.id)
            texts.append(text)

    if not texts:
        return 0, 0

    # Batch Ollama calls with thread pool for I/O parallelism
    generated, failed = 0, 0
    lock = __import__("threading").Lock()

    def store_batch(batch_texts: List[str], batch_ids: List[str]):
        nonlocal generated, failed
        embeddings = _get_ollama_embedding_batch(batch_texts, batch_size=batch_size)
        local_gen, local_fail = 0, 0
        for pid, emb in zip(batch_ids, embeddings):
            if emb is not None:
                try:
                    db.set_embedding(pid, emb)
                    local_gen += 1
                except Exception:
                    local_fail += 1
            else:
                local_fail += 1
        with lock:
            generated += local_gen
            failed += local_fail

    # Split into batches for Ollama, process batches in parallel
    for i in range(0, len(texts), batch_size * max_workers):
        chunk_ids = paper_ids[i : i + batch_size * max_workers]
        chunk_texts = texts[i : i + batch_size * max_workers]
        batches = [
            (chunk_texts[j : j + batch_size], chunk_ids[j : j + batch_size])
            for j in range(0, len(chunk_texts), batch_size)
        ]
        with ThreadPoolExecutor(max_workers=min(len(batches), max_workers)) as ex:
            for batch_texts, batch_ids in batches:
                ex.submit(store_batch, batch_texts, batch_ids)
        if delay > 0:
            time.sleep(delay)

    return generated, failed




def _run_dedup_semantic(args: argparse.Namespace) -> int:
    db = Database()
    db.init()

    if args.stats:
        s = db.get_embedding_stats()
        print("Embedding coverage:")
        print(f"  Papers with embedding : {s['with_embedding']}")
        print(f"  Papers with text     : {s['total_with_text']}")
        if s["total_with_text"] > 0:
            pct = s["with_embedding"] / s["total_with_text"] * 100
            print(f"  Coverage             : {pct:.1f}%")
        return 0

    if args.generate:
        print("Generating missing embeddings...")
        gen, fail = _generate_missing_embeddings(db)
        print(f"Generated: {gen}, Failed: {fail}")
        return 0

    if args.paper:
        if not db.paper_exists(args.paper):
            print(f"Paper '{args.paper}' not found")
            return 1
        paper = db.get_paper(args.paper)
        sims = db.find_similar(args.paper, threshold=args.threshold, limit=args.limit)
        if not sims:
            print(f"No similar papers found for '{args.paper}' (threshold={args.threshold})")
            return 0
        if args.format == "csv":
            print("paper_a,paper_b,similarity,title_a,title_b")
            for sim_paper, score in sims:
                t1 = paper.title.replace('"', '""')
                t2 = sim_paper.title.replace('"', '""')
                print(f"{args.paper},{sim_paper.id},{score:.4f},\"{t1}\",\"{t2}\"")
        else:
            print(f"Similar papers for '{args.paper}' (threshold={args.threshold}):")
            for sim_paper, score in sims:
                print(f"  [{score:.4f}] {sim_paper.id}  {sim_paper.title[:70]}")
        return 0

    # Global: check all papers
    papers, _total = db.list_papers(limit=10000)
    found = 0
    seen: set = set()
    if args.format == "csv":
        print("paper_a,paper_b,similarity,title_a,title_b")
    for paper in papers:
        if paper.id in seen or not paper.title:
            continue
        sims = db.find_similar(paper.id, threshold=args.threshold, limit=5)
        for sim_paper, score in sims:
            pair_key = tuple(sorted([paper.id, sim_paper.id]))
            if pair_key in seen:
                continue
            seen.add(pair_key)
            if args.format == "csv":
                t1 = paper.title.replace('"', '""')
                t2 = sim_paper.title.replace('"', '""')
                print(f"{paper.id},{sim_paper.id},{score:.4f},\"{t1}\",\"{t2}\"")
            else:
                print(f"[{score:.4f}] {paper.id} <-> {sim_paper.id}")
                print(f"  A: {paper.title[:70]}")
                print(f"  B: {sim_paper.title[:70]}")
                print()
            found += 1
    if found == 0:
        if args.format != "csv":
            print("No duplicate pairs found")
    else:
        if args.format != "csv":
            print(f"({found} duplicate pair(s) found, threshold={args.threshold})")
        else:
            print(f"# {found} duplicate pair(s) found, threshold={args.threshold}")
    return 0


def _pick_keep(older: Any, newer: Any, strategy: str) -> Tuple[Any, Any]:
    """Return (target, duplicate) based on keep strategy."""
    if strategy == "older":
        return (older, newer)
    if strategy == "newer":
        return (newer, older)
    # "parsed": keep the one with better parse_status
    # Ranking: completed > running > pending > failed
    status_rank = {"completed": 4, "running": 3, "pending": 2, "failed": 1}
    def rank(p):
        return status_rank.get(p.parse_status, 0)
    p1, p2 = older, newer
    if rank(p1) > rank(p2):
        return (p1, p2)
    if rank(p2) > rank(p1):
        return (p2, p1)
    # tie: prefer older by added_at
    return (older, newer)


def _build_merge_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser("merge", help="Merge a duplicate paper into a target paper")
    p.add_argument(
        "--keep",
        choices=["older", "newer", "parsed", "semantic"],
        default="older",
        help="Which paper to keep: 'older' (default), 'newer', 'parsed' (better parse_status), or 'semantic' (high similarity + parse_status)",
    )
    p.add_argument(
        "--dry-run",
        action="store_true",
        help="Show what would be merged without making changes",
    )
    p.add_argument(
        "--auto",
        action="store_true",
        help="Automatically find and merge all duplicate pairs with similarity >= 0.95",
    )
    p.add_argument("target_id", metavar="TARGET_ID", nargs="?", help="ID of the paper to keep")
    p.add_argument("duplicate_id", metavar="DUPLICATE_ID", nargs="?", help="ID of the duplicate paper to absorb and delete")
    return p


def _run_merge(args: argparse.Namespace) -> int:
    db = Database()
    db.init()

    if getattr(args, "auto", False):
        # Auto-mode: scan all papers and merge high-similarity pairs
        papers, _ = db.list_papers(limit=10000)
        seen: set = set()
        merged_count = 0
        skipped_count = 0

        for paper in papers:
            if paper.id in seen or not paper.title:
                continue
            sims = db.find_similar(paper.id, threshold=0.95, limit=10)
            for sim_paper, _score in sims:
                pair_key = tuple(sorted([paper.id, sim_paper.id]))
                if pair_key in seen:
                    continue
                seen.add(pair_key)

                # Determine keep/drop using semantic logic
                target, duplicate = paper, sim_paper
                sim = db.get_similarity(target.id, duplicate.id)
                if sim is None or sim < 0.95:
                    continue  # shouldn't happen given threshold, but guard

                older = target if target.added_at <= duplicate.added_at else duplicate
                newer = duplicate if target.added_at <= duplicate.added_at else target
                keep, drop = _pick_keep(target, duplicate, "parsed")

                if args.dry_run:
                    print(f"Would merge {drop.id} into {keep.id}")
                    print(f"  keeping : [{keep.id}] {keep.title[:70]}")
                    print(f"  deleting: [{drop.id}] {drop.title[:70]}")
                    print(f"  semantic similarity: {sim:.3f}")
                    print()
                    skipped_count += 1
                else:
                    ok = db.merge_papers(keep.id, drop.id)
                    if ok:
                        db.log_dedup(keep.id, drop.id, "semantic-auto")
                        print(f"Merged {drop.id} into {keep.id} (similarity={sim:.3f})")
                        seen.add(tuple(sorted([keep.id, drop.id])))  # prevent re-merging
                        merged_count += 1
                    else:
                        print(f"Merge failed for {drop.id} -> {keep.id}")

        if args.dry_run:
            print(f"({skipped_count} pair(s) would be merged, dry-run)")
        else:
            print(f"Auto-merge complete: {merged_count} pair(s) merged")
        return 0

    # Original two-paper merge
    if args.target_id is None or args.duplicate_id is None:
        print("merge requires TARGET_ID and DUPLICATE_ID (or use --auto)")
        return 1
    target = db.get_paper(args.target_id)
    if target is None:
        print(f"Target paper {args.target_id} not found")
        return 1
    duplicate = db.get_paper(args.duplicate_id)
    if duplicate is None:
        print(f"Duplicate paper {args.duplicate_id} not found")
        return 1
    # Build fake "older/newer" objects for _pick_keep based on added_at
    older = target if target.added_at <= duplicate.added_at else duplicate
    newer = duplicate if target.added_at <= duplicate.added_at else target
    sim = db.get_similarity(target.id, duplicate.id)
    if args.keep == "semantic":
        # Auto-select based on cosine similarity
        if sim is None or sim < 0.8:
            print(f"Note: low similarity, falling back to 'parsed' (similarity: {f'{sim:.3f}' if sim is not None else 'N/A'})")
            keep, drop = _pick_keep(target, duplicate, "parsed")
        else:
            # Both are likely the same paper — prefer better parse_status
            print(f"Auto-selected: similarity {sim:.3f} >= 0.8")
            keep, drop = _pick_keep(target, duplicate, "parsed")
    else:
        keep, drop = _pick_keep(older, newer, args.keep)

    if args.dry_run:
        print(f"Would merge {drop.id} into {keep.id} (--keep={args.keep})")
        print(f"  keeping : [{keep.id}] {keep.title[:70]}")
        print(f"  deleting: [{drop.id}] {drop.title[:70]}")
        if sim is not None:
            print(f"  semantic similarity: {sim:.3f}")
        else:
            print("  semantic similarity: no embeddings available")
        return 0
    ok = db.merge_papers(keep.id, drop.id)
    if ok:
        db.log_dedup(keep.id, drop.id, args.keep)
        print(f"Merged {drop.id} into {keep.id} (--keep={args.keep})")
        if sim is not None:
            print(f"  semantic similarity: {sim:.3f}")
        return 0
    else:
        print("Merge failed")
        return 1


def _run_queue(args: argparse.Namespace) -> int:
    db = Database()
    db.init()
    if args.list:
        jobs = db.get_papers(limit=100)
        pending = [p.id for p in jobs if p.parse_status == "pending"]
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
    elif args.clear:
        n = db.clear_pending_papers()
        print(f"Cleared {n} pending paper(s)")
    else:
        print("Use --list, --dequeue, --add UID, --cancel JOB_ID, or --clear")
    return 0


def _build_citations_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "citations",
        help="Show citation relationships for a paper",
    )
    p.add_argument(
        "--from",
        metavar="PAPER_ID",
        dest="citation_from",
        help="Show papers cited by PAPER_ID (backward citations)",
    )
    p.add_argument(
        "--to",
        metavar="PAPER_ID",
        dest="citation_to",
        help="Show papers that cite PAPER_ID (forward citations)",
    )
    p.add_argument(
        "--format",
        choices=["text", "csv"],
        default="text",
        help="Output format (default: text)",
    )
    return p


def _run_citations(args: argparse.Namespace) -> int:
    if not args.citation_from and not args.citation_to:
        print("Error: must specify --from or --to", file=sys.stderr)
        return 1

    db = Database()
    db.init()

    # Bidirectional filter: --from A --to B shows papers connecting A and B
    if args.citation_from and args.citation_to:
        paper_from = args.citation_from
        paper_to = args.citation_to
        from_title = db.get_paper_title(paper_from)
        to_title = db.get_paper_title(paper_to)

        if not from_title:
            print(f"Error: paper {paper_from} not found in the database")
            return 1
        if not to_title:
            print(f"Error: paper {paper_to} not found in the database")
            return 1

        # Papers cited by A (A's backward citations)
        backward_from = db.get_citations(paper_from, "from")
        # Papers that cite B (B's forward citations)
        forward_to = db.get_citations(paper_to, "to")

        # Check for direct edge: A -> B
        direct = any(c.target_id == paper_to for c in backward_from)

        # Papers that A cites AND that also have forward citations to B
        forward_to_sources = {c.source_id for c in forward_to}
        via_papers = [c for c in backward_from if c.target_id in forward_to_sources]

        if args.format == "csv":
            print("from_id,from_title,to_id,to_title,type")
            if direct:
                print(f"{paper_from},{from_title},{paper_to},{to_title},direct")
            if via_papers:
                # Bulk fetch titles for via papers (N+1 fix)
                via_ids = list({c.target_id for c in via_papers})
                paper_map = db.get_papers_bulk(via_ids)
                title_map = {pid: (paper_map[pid].title or '') for pid in via_ids if pid in paper_map}
                for c in via_papers:
                    t = title_map.get(c.target_id, '')
                    print(f"{paper_from},{from_title},{c.target_id},{t},via")
        else:
            print(f"CITATION BRIDGE: {paper_from} <-> {paper_to}")
            print(f"  {paper_from}: {from_title}")
            print(f"  {paper_to}: {to_title}")
            print()
            if direct:
                print(f"  DIRECT: {paper_from} cites {paper_to}")
            if via_papers:
                # Bulk fetch titles for via papers (N+1 fix)
                via_ids = list({c.target_id for c in via_papers})
                paper_map = db.get_papers_bulk(via_ids)
                title_map = {pid: (paper_map[pid].title or '') for pid in via_ids if pid in paper_map}
                print(f"  INDIRECT ({len(via_papers)} connections):")
                for c in via_papers:
                    t = title_map.get(c.target_id, '?')
                    print(f"    {paper_from} -> {c.target_id} ({t}) -> {paper_to}")
            if not direct and not via_papers:
                print("  No citation path found between these papers.")

        return 0

    # Single-direction mode
    paper_id = args.citation_from or args.citation_to
    direction: Literal["from", "to", "both"] = (
        "from" if args.citation_from else "to"
    )

    citations = db.get_citations(paper_id, direction)
    source_title = db.get_paper_title(paper_id)

    if args.format == "csv":
        # Bulk fetch all titles needed for CSV output
        all_ids = set()
        for c in citations:
            all_ids.add(c.source_id)
            all_ids.add(c.target_id)
        paper_map = db.get_papers_bulk(list(all_ids))
        title_map = {pid: (paper_map[pid].title or '') for pid in paper_map}
        print("direction,source_id,source_title,target_id,target_title")
        dir_label = "backward" if direction == "from" else "forward"
        for c in citations:
            src = c.source_id
            tgt = c.target_id
            src_t = title_map.get(src, '') if direction == "both" else source_title
            tgt_t = title_map.get(tgt, '')
            print(f"{dir_label},{src},{src_t},{tgt},{tgt_t}")
        return 0

    # Text output
    if not source_title:
        print(f"Error: paper {paper_id} not found in the database")
        return 1
    if not citations:
        print(f"No citations found for {paper_id}")
        return 0

    # Bulk fetch all titles needed for text output
    all_ids = set()
    for c in citations:
        all_ids.add(c.source_id)
        all_ids.add(c.target_id)
    paper_map = db.get_papers_bulk(list(all_ids))
    title_map = {pid: (paper_map[pid].title or '') for pid in paper_map}

    if direction == "from":
        print(f"BACKWARD CITATIONS — {paper_id}: {source_title}")
        print(f"({len(citations)} references)")
        print()
        for c in citations:
            t = title_map.get(c.target_id, '')
            print(f"  {c.target_id}  {t or '(unknown)'}")
    elif direction == "to":
        print(f"FORWARD CITATIONS — {paper_id}: {source_title}")
        print(f"({len(citations)} citing papers)")
        print()
        for c in citations:
            title = title_map.get(c.source_id, '')
            print(f"  {c.source_id}  {title or '(unknown)'}")
    else:
        print(f"ALL CITATIONS for {paper_id}: {source_title}")
        print(f"({len(citations)} total)")
        print()
        for c in citations:
            if c.source_id == paper_id:
                title = title_map.get(c.target_id, '')
                print(f"  -> {c.target_id}  {title or '(unknown)'}")
            else:
                title = title_map.get(c.source_id, '')
                print(f"  <- {c.source_id}  {title or '(unknown)'}")

    return 0


# ── Citation graph ─────────────────────────────────────────────────────────────────

@dataclass
class CiteGraphNode:
    paper_id: str
    title: str
    depth: int  # 0 = root, 1 = direct, 2 = 2-hop
    direction: str  # "root", "forward" (papers citing it), "backward" (papers it cites)


def _build_cite_graph_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "cite-graph",
        help="Visualize the local citation subgraph around a paper (1-2 hops)",
    )
    p.add_argument(
        "--paper", required=True, metavar="PAPER_ID",
        help="Root paper for the citation graph",
    )
    p.add_argument(
        "--depth", type=int, default=2, choices=[1, 2],
        help="Traversal depth: 1 = direct citations only, 2 = +2-hop (default: 2)",
    )
    p.add_argument(
        "--max-nodes", type=int, default=30,
        help="Maximum nodes per direction to show (default: 30)",
    )
    p.add_argument(
        "--format", choices=["text", "mermaid", "json"], default="text",
        help="Output format (default: text)",
    )
    p.add_argument(
        "--plain-text", metavar="TEXT",
        help="Build graph directly from plain text (extract references without DB import). "
             "Useful for papers not yet in the DB.",
    )
    p.add_argument(
        "--fetch-metadata", action="store_true",
        help="Fetch paper titles from arXiv/CrossRef APIs for plain-text mode. "
             "Implies --plain-text. Adds titles to graph nodes.",
    )
    return p


def _run_cite_graph(args: argparse.Namespace) -> int:
    db = Database()
    db.init()

    root_id = args.paper
    plain_text = getattr(args, "plain_text", None)

    # ── Plain-text mode: extract references from raw text ─────────────────────
    fetch_meta = getattr(args, "fetch_metadata", False)
    if plain_text is not None or fetch_meta:
        # If --fetch-metadata but no --plain-text provided, require an arXiv root ID
        if fetch_meta and plain_text is None:
            print("Error: --fetch-metadata requires --plain-text", file=sys.stderr)
            return 1

        if plain_text is not None:
            result = _extract_references_from_text(root_id, plain_text)
        else:
            result = {"arxiv_ids": [], "dois": [], "pmids": [], "isbns": []}

        arxiv_ids = result["arxiv_ids"]
        dois = result["dois"]
        pmids = result["pmids"]
        isbns = result["isbns"]

        if not arxiv_ids and not dois and not pmids and not isbns:
            print(f"No references found in plain text for {root_id!r}")
            return 0

        # Build in-memory graph from extracted references
        nodes: Dict[str, CiteGraphNode] = {}
        edges: List[Tuple[str, str, str]] = []

        nodes[root_id] = CiteGraphNode(root_id, root_id, 0, "root")

        ref_ids: list[str] = []
        for aid in arxiv_ids:
            rid = f"arXiv:{aid}"
            if rid not in nodes:
                nodes[rid] = CiteGraphNode(rid, "", 1, "backward")
                edges.append((root_id, rid, "backward"))
            ref_ids.append(rid)

        for doi in dois:
            did = f"doi:{doi}"
            if did not in nodes:
                nodes[did] = CiteGraphNode(did, "", 1, "backward")
                edges.append((root_id, did, "backward"))
            ref_ids.append(did)

        for pmid in pmids:
            pid = f"pmid:{pmid}"
            if pid not in nodes:
                nodes[pid] = CiteGraphNode(pid, "", 1, "backward")
                edges.append((root_id, pid, "backward"))
            ref_ids.append(pid)

        for isbn in isbns:
            iid = f"isbn:{isbn}"
            if iid not in nodes:
                nodes[iid] = CiteGraphNode(iid, "", 1, "backward")
                edges.append((root_id, iid, "backward"))
            ref_ids.append(iid)

        # ── Metadata fetch: populate titles from arXiv / CrossRef / PubMed / ISBN ─────────
        if fetch_meta:
            items = [(pid, n) for pid, n in nodes.items() if not n.title and n.depth > 0]
            if items:
                def _fetch_title_for_pid(pid: str) -> Tuple[str, Optional[str]]:
                    """Fetch title for a single paper ID. Returns (pid, title)."""
                    if pid.startswith("arXiv:"):
                        return pid, _fetch_arxiv_title(pid[6:])
                    elif pid.startswith("doi:"):
                        return pid, _fetch_doi_title(pid[4:])
                    elif pid.startswith("pmid:"):
                        return pid, _fetch_pmid_title(pid[5:])
                    elif pid.startswith("isbn:"):
                        return pid, _fetch_isbn_title(pid[5:])
                    return pid, None

                # Parallel fetch — no per-request sleep; total wall time ≈ max(individual_times)
                from concurrent.futures import ThreadPoolExecutor
                import threading
                lock = threading.Lock()
                results: List[Tuple[str, Optional[str]]] = []
                with ThreadPoolExecutor(max_workers=min(len(items), 8)) as ex:
                    futures = {ex.submit(_fetch_title_for_pid, pid): pid for pid, _ in items}
                    for future in futures:
                        results.append(future.result())
                for pid, title in results:
                    if title:
                        with lock:
                            nodes[pid].title = title

        # JSON: include title in node output
        if args.format == "json":
            out = {
                "root": root_id,
                "mode": "plain-text",
                "metadata_fetched": fetch_meta,
                "nodes": [
                    {"id": pid, "title": n.title, "depth": n.depth, "direction": n.direction}
                    for pid, n in nodes.items()
                ],
                "edges": [
                    {"from": f, "to": t, "direction": d} for f, t, d in edges
                ],
                "stats": {
                    "total_refs": len(ref_ids),
                    "arxiv_count": len(arxiv_ids),
                    "doi_count": len(dois),
                    "pmid_count": len(pmids),
                    "isbn_count": len(isbns),
                    "titles_fetched": sum(1 for n in nodes.values() if n.title and n.depth > 0),
                },
            }
            print(json.dumps(out, ensure_ascii=False, indent=2))
            return 0

        if args.format == "mermaid":
            print("```mermaid")
            print("graph TD")
            for pid, n in nodes.items():
                safe_id = pid.replace("-", "_").replace(".", "_").replace(":", "_")
                label = n.title[:40] + ("..." if len(n.title) > 40 else "") if n.title else pid
                print(f'    {safe_id}["{label}"]')
            print()
            for f, t, _d in edges:
                sf = f.replace("-", "_").replace(".", "_").replace(":", "_")
                st = t.replace("-", "_").replace(".", "_").replace(":", "_")
                arrow = "-->"
                print(f"    {sf} {arrow} {st}")
            print("```")
            return 0

        # Text format: show summary header, then graph
        print(f"References extracted from {root_id!r}:")
        if arxiv_ids:
            print(f"  arXiv IDs ({len(arxiv_ids)}): {', '.join(arxiv_ids[:20])}")
            if len(arxiv_ids) > 20:
                print(f"  ... and {len(arxiv_ids) - 20} more")
        if dois:
            print(f"  DOIs ({len(dois)}): {', '.join(dois[:10])}")
            if len(dois) > 10:
                print(f"  ... and {len(dois) - 10} more")
        if pmids:
            print(f"  PMIDs ({len(pmids)}): {', '.join(pmids[:10])}")
            if len(pmids) > 10:
                print(f"  ... and {len(pmids) - 10} more")
        if isbns:
            print(f"  ISBNs ({len(isbns)}): {', '.join(isbns[:10])}")
            if len(isbns) > 10:
                print(f"  ... and {len(isbns) - 10} more")
        meta_status = " (metadata fetched)" if fetch_meta else ""
        print()
        print(f"CITATION GRAPH (plain-text mode){meta_status} — {root_id}")
        print(f"References: {len(ref_ids)}  |  arXiv: {len(arxiv_ids)}  |  DOI: {len(dois)}  |  PMID: {len(pmids)}  |  ISBN: {len(isbns)}")
        print()
        print(f"  {root_id}  (ROOT)")
        print()
        print(f"  CITES ({len(ref_ids)} references)  ──")
        for rid in sorted(ref_ids, key=lambda x: x[0]):
            n = nodes[rid]
            title_str = f" — {n.title[:45]}" if n.title else ""
            print(f"    ●──? {rid}{title_str}")
        print()
        print(f"[{len(nodes)} nodes, {len(edges)} edges]")
        return 0

    # ── DB mode: build graph from citation edges in DB ──────────────────────
    if not db.paper_exists(root_id):
        print(f"Error: paper {root_id!r} not found in database", file=sys.stderr)
        return 1

    root_title = db.get_paper_title(root_id)
    depth = args.depth
    max_nodes = args.max_nodes

    # BFS: collect nodes by depth
    nodes: Dict[str, CiteGraphNode] = {}
    edges: List[Tuple[str, str, str]] = []

    def add_node(paper_id: str, title: str, node_depth: int, direction: str) -> None:
        if paper_id not in nodes and len(nodes) < max_nodes * 3:
            nodes[paper_id] = CiteGraphNode(paper_id, title, node_depth, direction)

    add_node(root_id, root_title, 0, "root")

    # Depth 0 → 1: direct forward (papers citing root) and backward (papers root cites)
    forward_1 = db.get_citations(root_id, "to")  # papers that cite root
    backward_1 = db.get_citations(root_id, "from")  # papers root cites

    for c in forward_1[:max_nodes]:
        add_node(c.source_id, "", 1, "forward")
        edges.append((c.source_id, root_id, "forward"))

    for c in backward_1[:max_nodes]:
        add_node(c.target_id, "", 1, "backward")
        edges.append((root_id, c.target_id, "backward"))

    # Depth 1 → 2: expand each depth-1 node
    if depth >= 2:
        depth1_ids = [pid for pid, n in nodes.items() if n.depth == 1]
        for d1_id in depth1_ids:
            if len(nodes) >= max_nodes * 3:
                break
            # forward 1-hop: papers that cite this node
            for c in db.get_citations(d1_id, "to")[:3]:
                if c.source_id not in nodes:
                    add_node(c.source_id, "", 2, "forward")
                    edges.append((c.source_id, d1_id, "forward"))
            # backward 1-hop: papers this node cites
            for c in db.get_citations(d1_id, "from")[:3]:
                if c.target_id not in nodes:
                    add_node(c.target_id, "", 2, "backward")
                    edges.append((d1_id, c.target_id, "backward"))

    # Batch-fetch titles for all nodes (excluding root which already has title)
    all_ids = list(nodes.keys())
    papers_map = db.get_papers_bulk(all_ids)
    for pid, _node in nodes.items():
        if pid in papers_map:
            nodes[pid].title = papers_map[pid].title or ""

    if args.format == "json":
        out = {
            "root": root_id,
            "title": root_title,
            "nodes": [
                {"id": pid, "title": n.title, "depth": n.depth, "direction": n.direction}
                for pid, n in nodes.items()
            ],
            "edges": [
                {"from": f, "to": t, "direction": d} for f, t, d in edges
            ],
        }
        print(json.dumps(out, ensure_ascii=False, indent=2))
        return 0

    if args.format == "mermaid":
        print("```mermaid")
        print("graph TD")
        for pid, n in nodes.items():
            safe_id = pid.replace("-", "_").replace(".", "_")
            label = n.title[:40] + ("..." if len(n.title) > 40 else "") if n.title else pid
            print(f'    {safe_id}["{label}"]')
        print()
        for f, t, d in edges:
            sf = f.replace("-", "_").replace(".", "_")
            st = t.replace("-", "_").replace(".", "_")
            arrow = "-->" if d == "forward" else "-.->"
            print(f"    {sf} {arrow} {st}")
        print("```")
        return 0

    # ── Text / ASCII output ───────────────────────────────────────────────────
    print(f"CITATION GRAPH — {root_id}: {root_title}")
    print(f"Depth: {depth}  |  Nodes: {len(nodes)}  |  Edges: {len(edges)}")
    print()

    # Show root
    print(f"  {root_id}  (ROOT)")
    if root_title:
        print(f"    {root_title[:60]}")
    print()

    # Group nodes by direction and depth
    depth1_forward = [(pid, n) for pid, n in nodes.items() if n.depth == 1 and n.direction == "forward"]
    depth1_backward = [(pid, n) for pid, n in nodes.items() if n.depth == 1 and n.direction == "backward"]
    depth2_forward = [(pid, n) for pid, n in nodes.items() if n.depth == 2 and n.direction == "forward"]
    depth2_backward = [(pid, n) for pid, n in nodes.items() if n.depth == 2 and n.direction == "backward"]

    if depth1_forward:
        print(f"  CITED BY ({len(depth1_forward)} papers citing ROOT)  ──")
        for pid, n in sorted(depth1_forward, key=lambda x: x[0]):
            title_str = f" — {n.title[:45]}" if n.title else ""
            print(f"    ●──? {pid}{title_str}")
        print()

    if depth1_backward:
        print(f"  CITES ({len(depth1_backward)} papers cited by ROOT)  ──")
        for pid, n in sorted(depth1_backward, key=lambda x: x[0]):
            title_str = f" — {n.title[:45]}" if n.title else ""
            print(f"    ●──? {pid}{title_str}")
        print()

    if depth2_forward and depth >= 2:
        print(f"  2-HOP CITED BY ({len(depth2_forward)} papers citing depth-1)  ──")
        for pid, n in sorted(depth2_forward, key=lambda x: x[0]):
            title_str = f" — {n.title[:40]}" if n.title else ""
            print(f"    ?──? {pid}{title_str}")
        print()

    if depth2_backward and depth >= 2:
        print(f"  2-HOP CITES ({len(depth2_backward)} papers cited by depth-1)  ──")
        for pid, n in sorted(depth2_backward, key=lambda x: x[0]):
            title_str = f" — {n.title[:40]}" if n.title else ""
            print(f"    ?──? {pid}{title_str}")
        print()

    print(f"[{len(nodes)} nodes, {len(edges)} edges, depth={depth}]")
    return 0


# ── Citation fetch from OpenAlex ───────────────────────────────────────────────

# Rate-limit: 10 requests/second with polite pool
_OPENALEX_BASE = "https://api.openalex.org"
_OPENALEX_EMAIL = "ai-research-os@example.com"


def _openalex_get(path: str, timeout: int = 15) -> dict:
    """Fetch a path from OpenAlex API with SSL bypass for Windows proxy."""
    url = f"{_OPENALEX_BASE}{path}"
    # Windows proxy interferes with OpenAlex SSL — bypass it
    ctx = ssl.create_default_context()
    ctx.check_hostname = False
    ctx.verify_mode = ssl.CERT_NONE
    proxy_handler = urllib.request.ProxyHandler({})
    opener = urllib.request.build_opener(proxy_handler, urllib.request.HTTPSHandler(context=ctx))
    req = urllib.request.Request(url, headers={
        "User-Agent": f"ai_research_os/1.0 (mailto:{_OPENALEX_EMAIL})",
        "Accept": "application/json",
    })
    try:
        with opener.open(req, timeout=timeout) as resp:
            return json.loads(resp.read().decode("utf-8"))
    except Exception as e:
        raise RuntimeError(f"OpenAlex request failed for {path}: {e}") from e


def _build_openalex_ctx() -> ssl.SSLContext:
    """Create SSL context that bypasses Windows proxy for API calls."""
    ctx = ssl.create_default_context()
    ctx.check_hostname = False
    ctx.verify_mode = ssl.CERT_NONE
    return ctx


def _fetch_arxiv_title(arxiv_id: str, timeout: int = 15) -> Optional[str]:
    """
    Fetch paper title from arXiv API using urllib + SSL bypass.
    Returns title string or None on failure.
    """
    url = f"http://export.arxiv.org/api/query?id_list={arxiv_id}"
    ctx = _build_openalex_ctx()
    proxy_handler = urllib.request.ProxyHandler({})
    opener = urllib.request.build_opener(proxy_handler, urllib.request.HTTPSHandler(context=ctx))
    req = urllib.request.Request(url, headers={"User-Agent": "ai_research_os/1.0"})
    try:
        with opener.open(req, timeout=timeout) as resp:
            data = resp.read().decode("utf-8")
        import xml.etree.ElementTree as ET
        root = ET.fromstring(data)
        ns = {"atom": "http://www.w3.org/2005/Atom"}
        entries = root.findall("atom:entry", ns)
        if entries:
            title_el = entries[0].find("atom:title", ns)
            if title_el is not None and title_el.text:
                return title_el.text.replace("\n", " ").strip()
    except Exception as e:
        warnings.warn(f"arXiv title fetch failed for {arxiv_id}: {e}", stacklevel=2)
    return None


def _fetch_doi_title(doi: str, timeout: int = 15) -> Optional[str]:
    """
    Fetch paper title from CrossRef API using urllib + SSL bypass.
    Returns title string or None on failure.
    """
    from parsers.crossref import fetch_crossref_metadata
    try:
        p, _ = fetch_crossref_metadata(doi, timeout=timeout)
        if p.title and p.title != doi:
            return p.title
    except Exception as e:
        warnings.warn(f"CrossRef title fetch failed for {doi}: {e}", stacklevel=2)
    return None


def _fetch_pmid_title(pmid: str, timeout: int = 15) -> Optional[str]:
    """
    Fetch paper title from NCBI Entrez API (E-utilities) using urllib + SSL bypass.
    Returns title string or None on failure.
    """
    url = (
        f"https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi"
        f"?db=pubmed&id={pmid}&retmode=json"
    )
    ctx = _build_openalex_ctx()
    proxy_handler = urllib.request.ProxyHandler({})
    opener = urllib.request.build_opener(proxy_handler, urllib.request.HTTPSHandler(context=ctx))
    req = urllib.request.Request(url, headers={"User-Agent": "ai_research_os/1.0"})
    try:
        with opener.open(req, timeout=timeout) as resp:
            data = json.loads(resp.read().decode("utf-8"))
        result = data.get("result", {})
        pmid_data = result.get(pmid, {})
        title = pmid_data.get("title", "")
        if title:
            return title
    except Exception as e:
        warnings.warn(f"PMID title fetch failed for {pmid}: {e}", stacklevel=2)
    return None


def _fetch_isbn_title(isbn: str, timeout: int = 15) -> Optional[str]:
    """
    Fetch book title from Open Library API using urllib + SSL bypass.
    Returns title string or None on failure.
    """
    url = f"https://openlibrary.org/api/books?bibkeys=ISBN:{isbn}&format=json&jscmd=data"
    ctx = _build_openalex_ctx()
    proxy_handler = urllib.request.ProxyHandler({})
    opener = urllib.request.build_opener(proxy_handler, urllib.request.HTTPSHandler(context=ctx))
    req = urllib.request.Request(url, headers={"User-Agent": "ai_research_os/1.0"})
    try:
        with opener.open(req, timeout=timeout) as resp:
            data = json.loads(resp.read().decode("utf-8"))
        key = f"ISBN:{isbn}"
        if key in data:
            book_data = data[key]
            title = book_data.get("title", "")
            if title:
                return title
    except Exception as e:
        warnings.warn(f"ISBN title fetch failed for {isbn}: {e}", stacklevel=2)
    return None


def _openalex_request(path: str, timeout: int = 15) -> dict:
    """Fetch a path from OpenAlex API, bypassing Windows proxy SSL issues."""
    url = f"{_OPENALEX_BASE}{path}"
    ctx = _build_openalex_ctx()
    proxy_handler = urllib.request.ProxyHandler({})
    opener = urllib.request.build_opener(proxy_handler, urllib.request.HTTPSHandler(context=ctx))
    req = urllib.request.Request(url, headers={
        "User-Agent": f"ai_research_os/1.0 (mailto:{_OPENALEX_EMAIL})",
        "Accept": "application/json",
    })
    try:
        with opener.open(req, timeout=timeout) as resp:
            return json.loads(resp.read().decode("utf-8"))
    except Exception as e:
        raise RuntimeError(f"OpenAlex request failed for {path}: {e}") from e


def _arxiv_doi_to_openalex(arxiv_id: str) -> Optional[str]:
    """Query OpenAlex for a paper by arXiv ID, return OpenAlex ID or None."""
    # OpenAlex DOI format: https://doi.org/10.48550/arXiv.YYMM.NNNNN
    doi = f"10.48550/arXiv.{arxiv_id}"
    try:
        d = _openalex_request(f"/works?filter=doi:{doi}&per-page=1")
        results = d.get("results", [])
        if results:
            return results[0]["id"]  # e.g. https://openalex.org/W2626778328
    except Exception as e:
        warnings.warn(f"arXiv DOI to OpenAlex lookup failed for {arxiv_id}: {e}", stacklevel=2)
    return None


def _get_openalex_references(openalex_id: str) -> Tuple[List[str], int]:
    """Fetch referenced works for an OpenAlex work. Returns (work_ids, total_count)."""
    oid = openalex_id.rstrip("/").split("/")[-1]
    data = _openalex_request(f"/works/{oid}")
    refs = data.get("referenced_works") or []
    count = data.get("referenced_works_count", len(refs))
    return refs, count


def _get_openalex_citing(openalex_id: str, per_page: int = 200) -> Tuple[list[dict], int]:
    """Get all papers citing this paper (forward citations). Returns (list of work dicts, total count)."""
    oid = openalex_id.rstrip("/").split("/")[-1]
    try:
        d = _openalex_request(f"/works?filter=cites:{oid}&per-page={per_page}&mailto={_OPENALEX_EMAIL}")
        return d.get("results", []) or [], d.get("meta", {}).get("count", 0)
    except Exception as e:
        warnings.warn(f"OpenAlex citing lookup failed for {openalex_id}: {e}", stacklevel=2)
        return [], 0


def _work_to_arxiv_id(work: dict) -> Optional[str]:
    """Extract arXiv ID from OpenAlex work IDs dict. Returns None if not an arXiv paper."""
    ids = work.get("ids", {}) or {}
    doi = ids.get("doi", "") or ""
    # DOI format: https://doi.org/10.48550/arXiv.2301.00001
    if "/arXiv." in doi:
        return doi.split("/arXiv.")[-1]
    return None


def _build_cite_fetch_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "cite-fetch",
        help="Fetch citation data from OpenAlex for papers in the database",
    )
    p.add_argument(
        "paper_id",
        nargs="?",
        help="arXiv paper ID (e.g. 2301.00001). If omitted, processes all papers.",
    )
    p.add_argument(
        "--direction",
        choices=["from", "to", "both"],
        default="both",
        help="Which citations to fetch: 'from'=references, 'to'=citing papers, 'both'=all (default: both)",
    )
    p.add_argument(
        "--dry-run",
        action="store_true",
        help="Show what would be fetched and imported without writing to DB",
    )
    p.add_argument(
        "--skip-external",
        action="store_true",
        help="Only import citations where both source and target are in the local DB",
    )
    p.add_argument(
        "--delay",
        type=float,
        default=0.11,
        help="Delay between API requests in seconds (default: 0.11 = ~9 req/s)",
    )
    p.add_argument(
        "--max-per-paper",
        type=int,
        default=0,
        help="Max citations to fetch per paper (0 = unlimited, default: 0)",
    )
    return p


def _run_cite_fetch(args: argparse.Namespace) -> int:
    """Fetch citation data from OpenAlex and populate the citations table (parallel)."""
    import threading

    db = Database()
    db.init()

    delay = max(0, args.delay)
    direction = args.direction
    dry_run = args.dry_run
    max_per_paper = args.max_per_paper
    max_workers = min(getattr(args, 'workers', 10), 20)

    paper_ids: list[str]
    if args.paper_id:
        if not db.paper_exists(args.paper_id):
            print(f"Error: paper {args.paper_id!r} not found in database", file=sys.stderr)
            return 1
        paper_ids = [args.paper_id]
    else:
        all_papers, _total = db.list_papers()
        paper_ids = [p.id for p in all_papers]
        if not paper_ids:
            print("No papers in database. Nothing to do.")
            return 0

    all_papers, _total = db.list_papers()
    known_ids: set[str] = {p.id for p in all_papers}

    lock = threading.Lock()
    total_added = [0]
    total_skipped_external = [0]
    total_errors = [0]
    total_cited_by_count = [0]

    def _fetch_ref(ref_oid: str, paper_id: str):
        try:
            oid = ref_oid.rstrip("/").split("/")[-1]
            ref_work = _openalex_request(f"/works/{oid}")
            ref_arxiv_id = _work_to_arxiv_id(ref_work)
            if not ref_arxiv_id or ref_arxiv_id not in known_ids:
                with lock:
                    total_skipped_external[0] += 1
                return
            added = db.add_citation(paper_id, ref_arxiv_id)
            with lock:
                if added:
                    total_added[0] += 1
        except Exception as e:
            with lock:
                total_errors[0] += 1
            warnings.warn(f"ref {ref_oid}: {e}", stacklevel=2)

    def _fetch_citing(citing_work: dict, paper_id: str):
        try:
            citing_arxiv_id = _work_to_arxiv_id(citing_work)
            if not citing_arxiv_id or citing_arxiv_id not in known_ids:
                with lock:
                    total_skipped_external[0] += 1
                return
            added = db.add_citation(citing_arxiv_id, paper_id)
            with lock:
                if added:
                    total_added[0] += 1
        except Exception as e:
            with lock:
                total_errors[0] += 1
            warnings.warn(f"citing: {e}", stacklevel=2)

    for paper_id in paper_ids:
        print(f"Processing {paper_id}...", file=sys.stderr)
        openalex_id = _arxiv_doi_to_openalex(paper_id)
        if not openalex_id:
            print(f"  [skip] {paper_id}: not found in OpenAlex", file=sys.stderr)
            continue

        if direction in ("from", "both"):
            referenced_works, ref_count = _get_openalex_references(openalex_id)
            ref_ids = referenced_works[:max_per_paper] if max_per_paper > 0 else referenced_works
            if dry_run:
                print(f"  [dry-run] backward: {ref_count} referenced works", file=sys.stderr)
            elif ref_ids:
                with ThreadPoolExecutor(max_workers=max_workers) as ex:
                    for ref_oid in ref_ids:
                        ex.submit(_fetch_ref, ref_oid, paper_id)
                if delay > 0:
                    time.sleep(delay)

        if direction in ("to", "both"):
            citing_works, citing_count = _get_openalex_citing(openalex_id)
            with lock:
                total_cited_by_count[0] += citing_count
            if dry_run:
                print(f"  [dry-run] forward: {citing_count} citing works", file=sys.stderr)
            elif citing_works:
                limit = max_per_paper if max_per_paper > 0 else len(citing_works)
                with ThreadPoolExecutor(max_workers=max_workers) as ex:
                    for citing_work in citing_works[:limit]:
                        ex.submit(_fetch_citing, citing_work, paper_id)
                if delay > 0:
                    time.sleep(delay)

    print("\nSummary:", file=sys.stderr)
    if dry_run:
        print("  [dry-run mode — nothing written]", file=sys.stderr)
    print(f"  Citations added to DB: {total_added[0]}", file=sys.stderr)
    print(f"  Skipped (not in DB or external): {total_skipped_external[0]}", file=sys.stderr)
    print(f"  Errors: {total_errors[0]}", file=sys.stderr)
    if direction in ("to", "both"):
        print(f"  Note: {total_cited_by_count[0]} forward citations found (some may be outside DB)", file=sys.stderr)
    return 0


# ── Citation reference extraction ──────────────────────────────────────────────

_REFS_SECTION_PAT = re.compile(
    r"(?i)\n(?:references|bibliography|works cited|literature cited)\s*\n",
    re.M,
)
_ARXIV_PAT = re.compile(
    r"(?i)\barxiv:?\.?\s*(\d{4}\.\d{4,5}(?:v\d+)?)\b"
)
_DOI_PAT = re.compile(
    r"(?i)(?:doi:|https?://(?:dx\.)?doi\.org/)(10\.\d{4,}/\S+)"
)
_PMID_PAT = re.compile(
    r"\b(?:PMID|pmid)[:\s]*(\d{6,9})\b"
)
_ISBN_PAT = re.compile(
    r"(?i)\b(?:ISBN)[:\s]*([0-9\-X]+)\b"
)
_NUM_BRACKET_PAT = re.compile(r"\[\d+\]")


def _extract_references_from_text(paper_id: str, text: str) -> dict[str, list[str]]:
    """
    Extract arXiv IDs, DOIs, PMIDs, and ISBNs from the plain text of a paper.

    Returns dict with keys "arxiv_ids", "dois", "pmids", and "isbns", each a list
    of unique IDs found in the references section (or the whole text if no
    section header is found). Returns empty lists if no references section is
    found or the text is empty.
    """

    if not text or not text.strip():
        return {"arxiv_ids": [], "dois": [], "pmids": [], "isbns": []}

    # Try to isolate the references section
    match = _REFS_SECTION_PAT.search(text)
    if match:
        refs_text = text[match.start() :]  # noqa: F841
    else:
        pass  # refs_text = text  # fall back to whole text

    # Extract arXiv IDs (search the whole text, not just refs section,
    # so inline citations in body text are also captured)
    arxiv_ids: set[str] = set()
    for m in _ARXIV_PAT.finditer(text):
        arxiv_ids.add(m.group(1))

    # Extract DOIs (strip leading doi: or https://doi.org/)
    dois: set[str] = set()
    for m in _DOI_PAT.finditer(text):
        raw = m.group(1).rstrip(" .")
        if raw.startswith("10."):
            dois.add(raw)

    # Extract PMIDs
    pmids: set[str] = set()
    for m in _PMID_PAT.finditer(text):
        pmids.add(m.group(1))

    # Extract ISBNs (13-digit and 10-digit, normalize by removing hyphens/spaces)
    isbns: set[str] = set()
    for m in _ISBN_PAT.finditer(text):
        raw = m.group(1).replace("-", "").replace(" ", "")
        if len(raw) == 13 or len(raw) == 10:
            if len(raw) == 10:
                # Normalize 10-digit ISBN to uppercase
                raw = raw.upper()
            isbns.add(raw)

    return {
        "arxiv_ids": sorted(arxiv_ids),
        "dois": sorted(dois),
        "pmids": sorted(pmids),
        "isbns": sorted(isbns),
    }


def _build_cite_import_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "cite-import",
        help="Import citation links from a JSON file or inline JSON",
    )
    p.add_argument(
        "json_input",
        nargs="?",
        help="JSON string or @filename (file prefixed with @) containing citation data",
    )
    p.add_argument(
        "--dry-run",
        action="store_true",
        help="Show what would be imported without writing to DB",
    )
    p.add_argument(
        "--skip-missing",
        action="store_true",
        help="Skip source/target papers that don't exist in the DB",
    )
    p.add_argument(
        "--extract",
        action="store_true",
        help="Extract citation references from a paper's plain_text (requires --paper)",
    )
    p.add_argument(
        "--paper",
        metavar="PAPER_ID",
        dest="extract_paper",
        help="Paper ID for --extract mode (extract references from this paper's plain_text)",
    )
    p.add_argument(
        "--dedup",
        action="store_true",
        help="Use upsert mode to report duplicate citation edges instead of silently skipping them",
    )
    return p


def _run_cite_import(args: argparse.Namespace) -> int:
    # ── --extract mode ───────────────────────────────────────────────────────
    if getattr(args, "extract", False):
        paper_id = getattr(args, "extract_paper", None)
        if not paper_id:
            print("Error: --paper PAPER_ID required with --extract", file=sys.stderr)
            return 1

        db = Database()
        db.init()

        paper = db.get_paper(paper_id)
        if not paper:
            print(f"Error: paper {paper_id!r} not found in DB", file=sys.stderr)
            return 1

        if not paper.plain_text:
            print(f"Error: paper {paper_id!r} has no plain_text to extract from", file=sys.stderr)
            return 1

        result = _extract_references_from_text(paper_id, paper.plain_text)
        arxiv_ids = result["arxiv_ids"]
        dois = result["dois"]
        pmid_ids = result["pmids"]
        isbn_ids = result["isbns"]

        if not arxiv_ids and not dois and not pmid_ids and not isbn_ids:
            print(f"No references found in {paper_id!r}")
            return 0

        print(f"Extracted from {paper_id!r}:")
        if arxiv_ids:
            print(f"  arXiv IDs ({len(arxiv_ids)}): {', '.join(arxiv_ids)}")
        if dois:
            print(f"  DOIs ({len(dois)}): {', '.join(dois)}")
        if pmid_ids:
            print(f"  PMIDs ({len(pmid_ids)}): {', '.join(pmid_ids)}")
        if isbn_ids:
            print(f"  ISBNs ({len(isbn_ids)}): {', '.join(isbn_ids)}")

        # Build citation edge list (source paper cites each reference)
        # arXiv IDs that exist in DB become citation edges
        db_ids: list[str] = []
        missing: list[str] = []
        for aid in arxiv_ids:
            full = f"arXiv:{aid}"
            if db.paper_exists(full):
                db_ids.append(full)
            else:
                missing.append(aid)

        if db_ids:
            if args.dry_run:
                print(f"\n[dry-run] Would import {len(db_ids)} citation edge(s):")
                print(f"  {paper_id} --> {db_ids}")
            else:
                if getattr(args, "dedup", False):
                    new_n, dup_n = db.upsert_citations(paper_id, db_ids)
                    print(f"\nImported {new_n} new edge(s), {dup_n} duplicate(s) skipped")
                else:
                    n = db.add_citations_batch(paper_id, db_ids)
                    print(f"\nImported {n} citation edge(s) from arXiv IDs ({len(missing)} not in DB)")
        else:
            print(f"\n0 arXiv IDs found in DB (all {len(missing)} references are new papers)")
            return 0

        if missing:
            print(f"  Missing (not in DB): {', '.join(missing[:20])}")
            if len(missing) > 20:
                print(f"  ... and {len(missing) - 20} more")

        return 0

    # ── JSON import mode ─────────────────────────────────────────────────────
    raw = args.json_input
    if not raw:
        print("Error: json_input required (JSON string or @filepath)", file=sys.stderr)
        return 1

    if raw.startswith("@"):
        path = raw[1:]
        try:
            with open(path, encoding="utf-8") as f:
                data = json.load(f)
        except Exception as e:
            print(f"Error: cannot read {path}: {e}", file=sys.stderr)
            return 1
    else:
        try:
            data = json.loads(raw)
        except json.JSONDecodeError as e:
            print(f"Error: invalid JSON: {e}", file=sys.stderr)
            return 1

    # Normalise to list
    if isinstance(data, dict):
        data = [data]
    elif not isinstance(data, list):
        print("Error: JSON must be a list of objects or a single object", file=sys.stderr)
        return 1

    db = Database()
    db.init()

    total_new = 0
    total_skip_missing = 0
    errors = []

    for i, item in enumerate(data):
        if not isinstance(item, dict):
            errors.append(f"[{i}] item is not an object, skipping")
            continue
        source = str(item.get("source") or item.get("source_id") or "")
        targets = item.get("targets") or item.get("target_ids") or []
        if not isinstance(targets, list):
            targets = [targets]
        if not source:
            errors.append(f"[{i}] missing 'source' field, skipping")
            continue
        if not targets:
            errors.append(f"[{i}] empty 'targets' for source={source}, skipping")
            continue

        # Check source exists
        if not db.paper_exists(source):
            msg = f"source paper {source!r} not in DB"
            if args.skip_missing:
                total_skip_missing += 1
                if args.dry_run:
                    print(f"  [dry-run] skip (missing): {source}")
                else:
                    print(f"Error: {msg}", file=sys.stderr)
            else:
                errors.append(f"[{i}] {msg}")
            continue

        valid_targets = []
        for tgt in targets:
            tgt = str(tgt)
            if not db.paper_exists(tgt):
                msg = f"target paper {tgt!r} not in DB"
                if args.skip_missing:
                    total_skip_missing += 1
                    if args.dry_run:
                        print(f"  [dry-run] skip (missing): {tgt}")
                    else:
                        print(f"Error: {msg}", file=sys.stderr)
                else:
                    errors.append(f"[{i}] {msg}")
                continue
            valid_targets.append(tgt)

        if not valid_targets:
            continue

        # Upsert
        if args.dry_run:
            for tgt in valid_targets:
                print(f"  [dry-run] add citation: {source} -> {tgt}")
            total_new += len(valid_targets)
        else:
            n = db.add_citations_batch(source, valid_targets)
            total_new += n

    if errors:
        print(f"  warnings/errors : {len(errors)}")
        for e in errors[:10]:
            print(f"    - {e}")
        if len(errors) > 10:
            print(f"    ... and {len(errors) - 10} more")
        return 1

    print("Import complete.")
    print(f"  new citations : {total_new}")
    print(f"  skipped (missing papers): {total_skip_missing}")
    return 0

# ── Citation stats ─────────────────────────────────────────────────────────

def _build_cite_stats_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "cite-stats",
        help="Show citation statistics for papers in the database",
    )
    p.add_argument(
        "--paper",
        metavar="PAPER_ID",
        dest="stats_paper",
        help="Show stats for a specific paper (forward/backward counts)",
    )
    p.add_argument(
        "--top",
        type=int,
        metavar="N",
        help="Show top N most-cited papers (by forward citations)",
    )
    p.add_argument(
        "--format",
        choices=["text", "csv"],
        default="text",
        help="Output format (default: text)",
    )
    return p


def _run_cite_stats(args: argparse.Namespace) -> int:
    db = Database()
    db.init()

    # Single-paper view
    if args.stats_paper:
        paper_id = args.stats_paper
        title = db.get_paper_title(paper_id)
        if db.paper_exists(paper_id) is False:
            print(f"Error: paper {paper_id!r} not found in database", file=sys.stderr)
            return 1
        counts = db.get_citation_count(paper_id)
        print(f"Paper: {paper_id}")
        print(f"Title : {title or '(no title)'}")
        print(f"Cites  (backward): {counts['backward']} papers")
        print(f"CitedBy (forward) : {counts['forward']} papers")
        return 0

    # Global stats
    cur = db.conn.cursor()

    # Total citation pairs
    cur.execute("SELECT COUNT(*) FROM citations")
    total_citations = cur.fetchone()[0]

    # Papers that have any citation data
    cur.execute("SELECT COUNT(DISTINCT source_id) FROM citations")
    citing_papers = cur.fetchone()[0]
    cur.execute("SELECT COUNT(DISTINCT target_id) FROM citations")
    cited_papers = cur.fetchone()[0]

    # Papers with no citation data (orphans)
    cur.execute("SELECT COUNT(*) FROM papers")
    total_papers = cur.fetchone()[0]
    cur.execute("SELECT COUNT(DISTINCT id) FROM papers WHERE id IN (SELECT source_id FROM citations UNION SELECT target_id FROM citations)")
    papers_with_any = cur.fetchone()[0]
    orphan_papers = total_papers - papers_with_any

    if args.format == "csv":
        print("metric,value")
        print(f"total_citations,{total_citations}")
        print(f"papers_with_any_citation,{papers_with_any}")
        print(f"orphan_papers,{orphan_papers}")
        print(f"citing_papers,{citing_papers}")
        print(f"cited_papers,{cited_papers}")
        return 0

    print("=== Citation Statistics ===")
    print(f"Total citation pairs   : {total_citations}")
    print(f"Papers with citation  : {papers_with_any} / {total_papers}")
    print(f"  - citing (backward): {citing_papers}")
    print(f"  - cited  (forward) : {cited_papers}")
    print(f"Orphan papers (no refs): {orphan_papers}")

    # Graph density
    if total_papers > 1:
        max_edges = total_papers * (total_papers - 1)
        density = (total_citations / max_edges) * 100 if max_edges > 0 else 0
        print(f"Graph density          : {density:.4f}%")

    # Top cited (most cited by others = forward citations)
    if args.top:
        n = args.top
        cur.execute("""
            SELECT target_id, COUNT(*) as cnt, p.title
            FROM citations c
            JOIN papers p ON p.id = c.target_id
            GROUP BY target_id
            ORDER BY cnt DESC, target_id
            LIMIT ?
        """, (n,))
        rows = cur.fetchall()
        print(f"\n=== Top {n} Most Cited (forward citations) ===")
        for i, (paper_id, cnt, title) in enumerate(rows, 1):
            title_short = (title or "(no title)")[:60]
            print(f"  {i:2}. [{cnt:4}x] {paper_id}  {title_short}")

        # Top citing (most references to others = backward citations)
        cur.execute("""
            SELECT source_id, COUNT(*) as cnt, p.title
            FROM citations c
            JOIN papers p ON p.id = c.source_id
            GROUP BY source_id
            ORDER BY cnt DESC, source_id
            LIMIT ?
        """, (n,))
        rows = cur.fetchall()
        print(f"\n=== Top {n} Most Citing (backward citations) ===")
        for i, (paper_id, cnt, title) in enumerate(rows, 1):
            title_short = (title or "(no title)")[:60]
            print(f"  {i:2}. [{cnt:4}x] {paper_id}  {title_short}")

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
    logging.basicConfig(level=logging.WARNING, format="%(levelname)s: %(message)s")

    SUBCOMMANDS = {
        "search", "list", "status", "queue", "cache", "dedup", "merge", "stats",
        "import", "export", "citations", "cite-graph", "cite-import", "cite-fetch",
        "cite-stats", "dedup-semantic", "research", "similar",
    }
    raw_args = argv if argv is not None else sys.argv[1:]
    first = raw_args[0] if raw_args else ""

    if first not in SUBCOMMANDS:
        return _main_legacy(argv)

    parser = argparse.ArgumentParser(description="AI Research OS")
    subparsers = parser.add_subparsers(dest="subcmd", help="Subcommands")

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
    elif args.subcmd == "stats":
        return _run_stats(args)
    elif args.subcmd == "import":
        return _run_import(args)
    elif args.subcmd == "export":
        return _run_export(args)
    elif args.subcmd == "citations":
        return _run_citations(args)
    elif args.subcmd == "cite-graph":
        return _run_cite_graph(args)
    elif args.subcmd == "cite-import":
        return _run_cite_import(args)
    elif args.subcmd == "cite-fetch":
        return _run_cite_fetch(args)
    elif args.subcmd == "cite-stats":
        return _run_cite_stats(args)
    elif args.subcmd == "dedup-semantic":
        return _run_dedup_semantic(args)
    elif args.subcmd == "research":
        return _run_research_cmd(args)
    elif args.subcmd == "similar":
        return _run_similar(args)
    return 0


def _main_legacy(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="AI Research OS - Full Flow (P+C+M+Radar+Timeline + optional AI draft)")
    parser.add_argument("input", help="arXiv id/URL or DOI/doi.org URL")
    parser.add_argument("--root", default="AI-Research", help="Root folder for your research OS")
    parser.add_argument("--category", default="02-Models", help="Folder under root to place P-Note")
    parser.add_argument("--tags", default="", help="Comma-separated tags (recommended), e.g. LLM,Agent,RAG")
    parser.add_argument("--concept-dir", default=get_default_concept_dir(), help="Folder under root to place C-Notes")
    parser.add_argument("--comparison-dir", default=get_default_radar_dir(), help="Folder under root to place M-Notes")
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
                # Save tables to DB
                if sdoc.tables:
                    db = Database()
                    db.init()
                    table_records = []
                    for tbl in sdoc.tables:
                        lines = [l for l in tbl.text.split("\n") if l.strip() and "---" not in l]
                        header_cells = [c.strip() for c in lines[0].split("|") if c.strip()] if lines else []
                        data_rows = []
                        for row_line in lines[1:]:
                            cells = [c.strip() for c in row_line.split("|") if c.strip()]
                            if cells:
                                data_rows.append(cells)
                        table_records.append(ExperimentTableRecord(
                            id=0,
                            paper_id=paper.uid,
                            table_caption="",
                            page=tbl.page,
                            headers=header_cells,
                            rows=data_rows,
                            bbox_x0=tbl.bbox[0] if tbl.bbox else 0,
                            bbox_y0=tbl.bbox[1] if tbl.bbox else 0,
                            bbox_x1=tbl.bbox[2] if tbl.bbox else 0,
                            bbox_y1=tbl.bbox[3] if tbl.bbox else 0,
                            created_at=__import__("datetime").datetime.utcnow().isoformat(),
                        ))
                    db.upsert_experiment_tables(paper.uid, table_records)
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
