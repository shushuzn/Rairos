#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""AI Research OS CLI entry point."""

import argparse
import json
import os
import ssl
import sys
import time
import urllib.request
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, List, Literal, Optional, Tuple

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
    p.add_argument("ids", nargs="+", metavar="ID", help="arXiv IDs, DOIs, or paper UIDs to add")
    p.add_argument("--source", default="import", help="Source label (default: import)")
    p.add_argument("--skip-existing", action="store_true", help="Skip IDs already in database")
    return p


def _run_import(args: argparse.Namespace) -> int:
    db = Database()
    db.init()
    added, skipped, failed = 0, 0, 0
    for paper_id in args.ids:
        p = db.get_paper(paper_id)
        if p is not None:
            if args.skip_existing:
                skipped += 1
                print(f"Skipped (exists): {paper_id}")
                continue
            else:
                skipped += 1
                print(f"Skipped (exists): {paper_id}")
                continue
        # Attempt to upsert — use raw arXiv ID or DOI as id
        try:
            paper_id_clean = paper_id.strip()
            db.upsert_paper(paper_id_clean, args.source)
            added += 1
            print(f"Added: {paper_id_clean}")
        except Exception as e:
            failed += 1
            print(f"Failed: {paper_id} — {e}")
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
        keep_older = newer.parse_status == older.parse_status and newer.added_at == older.added_at
        parsed_rank = {"completed": 4, "running": 3, "pending": 2, "failed": 1}
        rank = lambda p: parsed_rank.get(p.parse_status, 0)
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
    print(f"Use 'paper-cli merge TARGET_ID DUPLICATE_ID' to merge each pair")
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
    rank = lambda p: status_rank.get(p.parse_status, 0)
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
        choices=["older", "newer", "parsed"],
        default="older",
        help="Which paper to keep: 'older' (default), 'newer', or 'parsed' (better parse_status)",
    )
    p.add_argument(
        "--dry-run",
        action="store_true",
        help="Show what would be merged without making changes",
    )
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
    # Build fake "older/newer" objects for _pick_keep based on added_at
    older = target if target.added_at <= duplicate.added_at else duplicate
    newer = duplicate if target.added_at <= duplicate.added_at else target
    keep, drop = _pick_keep(older, newer, args.keep)
    if args.dry_run:
        print(f"Would merge {drop.id} into {keep.id} (--keep={args.keep})")
        print(f"  keeping : [{keep.id}] {keep.title[:70]}")
        print(f"  deleting: [{drop.id}] {drop.title[:70]}")
        return 0
    ok = db.merge_papers(keep.id, drop.id)
    if ok:
        db.log_dedup(keep.id, drop.id, args.keep)
        print(f"Merged {drop.id} into {keep.id} (--keep={args.keep})")
        return 0
    else:
        print(f"Merge failed")
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

    paper_id = args.citation_from or args.citation_to
    direction: Literal["from", "to", "both"] = (
        "from" if args.citation_from else "to"
    )

    citations = db.get_citations(paper_id, direction)
    source_title = db.get_paper_title(paper_id)

    if args.format == "csv":
        print("direction,source_id,source_title,target_id,target_title")
        dir_label = "backward" if direction == "from" else "forward"
        for c in citations:
            src = c.source_id
            tgt = c.target_id
            src_t = db.get_paper_title(src) if direction == "both" else source_title
            tgt_t = db.get_paper_title(tgt)
            print(f"{dir_label},{src},{src_t},{tgt},{tgt_t}")
        return 0

    # Text output
    if not source_title:
        print(f"Error: paper {paper_id} not found in the database")
        return 1
    if not citations:
        print(f"No citations found for {paper_id}")
        return 0

    if direction == "from":
        print(f"BACKWARD CITATIONS — {paper_id}: {source_title}")
        print(f"({len(citations)} references)")
        print()
        for c in citations:
            t = db.get_paper_title(c.target_id)
            print(f"  {c.target_id}  {t or '(unknown)'}")
    elif direction == "to":
        print(f"FORWARD CITATIONS — {paper_id}: {source_title}")
        print(f"({len(citations)} citing papers)")
        print()
        for c in citations:
            title = db.get_paper_title(c.source_id)
            print(f"  {c.source_id}  {title or '(unknown)'}")
    else:
        print(f"ALL CITATIONS for {paper_id}: {source_title}")
        print(f"({len(citations)} total)")
        print()
        for c in citations:
            if c.source_id == paper_id:
                title = db.get_paper_title(c.target_id)
                print(f"  -> {c.target_id}  {title or '(unknown)'}")
            else:
                title = db.get_paper_title(c.source_id)
                print(f"  <- {c.source_id}  {title or '(unknown)'}")

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
    except Exception:
        pass
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
    except Exception:
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
    """Fetch citation data from OpenAlex and populate the citations table."""
    db = Database()
    db.init()

    delay = max(0, args.delay)
    direction = args.direction
    dry_run = args.dry_run
    skip_external = args.skip_external
    max_per_paper = args.max_per_paper

    # Collect papers to process
    paper_ids: list[str]
    if args.paper_id:
        if not db.paper_exists(args.paper_id):
            print(f"Error: paper {args.paper_id!r} not found in database", file=sys.stderr)
            return 1
        paper_ids = [args.paper_id]
    else:
        # Process all papers
        all_papers, _total = db.list_papers()
        paper_ids = [p.id for p in all_papers]
        if not paper_ids:
            print("No papers in database. Nothing to do.")
            return 0

    # Build set of known paper IDs for fast lookup
    all_papers, _total = db.list_papers()
    known_ids: set[str] = {p.id for p in all_papers}

    total_added = 0
    total_skipped_external = 0
    total_errors = 0
    total_cited_by_count = 0  # forward citations found (may not all be imported)

    for paper_id in paper_ids:
        print(f"Processing {paper_id}...", file=sys.stderr)

        # Step 1: Get OpenAlex ID for this paper
        openalex_id = _arxiv_doi_to_openalex(paper_id)
        if not openalex_id:
            print(f"  [skip] {paper_id}: not found in OpenAlex", file=sys.stderr)
            continue

        # Step 2: Fetch backward citations (papers this paper references)
        if direction in ("from", "both"):
            referenced_works, ref_count = _get_openalex_references(openalex_id)
            if dry_run:
                # Just show count without fetching individual references
                print(f"  [dry-run] backward: {ref_count} referenced works", file=sys.stderr)
            else:
                for ref_openalex_id in (referenced_works[:max_per_paper] if max_per_paper > 0 else referenced_works):
                    try:
                        ref_work = _openalex_request(f"/works/{ref_openalex_id.rstrip('/').split('/')[-1]}")
                        ref_arxiv_id = _work_to_arxiv_id(ref_work)
                        if not ref_arxiv_id:
                            total_skipped_external += 1
                            continue
                        if ref_arxiv_id not in known_ids:
                            total_skipped_external += 1
                            continue
                        added = db.add_citation(paper_id, ref_arxiv_id)
                        if added:
                            total_added += 1
                    except Exception as e:
                        total_errors += 1
                        print(f"  [error] fetching ref {ref_openalex_id}: {e}", file=sys.stderr)
            if delay > 0:
                time.sleep(delay)

        # Step 3: Fetch forward citations (papers citing this paper)
        if direction in ("to", "both"):
            citing_works, citing_count = _get_openalex_citing(openalex_id)
            total_cited_by_count += citing_count
            if dry_run:
                print(f"  [dry-run] forward: {citing_count} citing works", file=sys.stderr)
            else:
                cited_count = 0
                for citing_work in citing_works:
                    if max_per_paper > 0 and cited_count >= max_per_paper:
                        break
                    citing_arxiv_id = _work_to_arxiv_id(citing_work)
                    if not citing_arxiv_id:
                        total_skipped_external += 1
                        cited_count += 1
                        continue
                    if citing_arxiv_id not in known_ids:
                        total_skipped_external += 1
                        cited_count += 1
                        continue
                    added = db.add_citation(citing_arxiv_id, paper_id)
                    if added:
                        total_added += 1
                    cited_count += 1
                if delay > 0:
                    time.sleep(delay)

    # Summary
    print(f"\nSummary:", file=sys.stderr)
    if dry_run:
        print(f"  [dry-run mode — nothing written]", file=sys.stderr)
    print(f"  Citations added to DB: {total_added}", file=sys.stderr)
    print(f"  Skipped (not in DB or external): {total_skipped_external}", file=sys.stderr)
    print(f"  Errors: {total_errors}", file=sys.stderr)
    if direction in ("to", "both"):
        print(f"  Note: {total_cited_by_count} forward citations found in OpenAlex (some may be outside DB)", file=sys.stderr)

    return 0


# ── Citation import ──────────────────────────────────────────────────────────

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
    return p


def _run_cite_import(args: argparse.Namespace) -> int:
    raw = args.json_input
    if not raw:
        print("Error: json_input required (JSON string or @filepath)", file=sys.stderr)
        return 1

    # Load JSON
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
    total_skip_dup = 0
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

        if args.dry_run:
            for tgt in valid_targets:
                print(f"  [dry-run] add citation: {source} -> {tgt}")
            total_new += len(valid_targets)
        else:
            n = db.add_citations_batch(source, valid_targets)
            total_new += n

    # Summary
    print(f"Import complete.")
    print(f"  new citations : {total_new}")
    print(f"  skipped (missing papers): {total_skip_missing}")
    if errors and not args.dry_run:
        print(f"  warnings/errors : {len(errors)}")
        for e in errors[:10]:
            print(f"    - {e}")
        if len(errors) > 10:
            print(f"    ... and {len(errors) - 10} more")
        return 1

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
    _build_stats_parser(subparsers)
    _build_import_parser(subparsers)
    _build_export_parser(subparsers)
    _build_citations_parser(subparsers)
    _build_cite_import_parser(subparsers)
    _build_cite_fetch_parser(subparsers)

    # Check if first arg is a known subcommand
    raw_args = argv if argv is not None else sys.argv[1:]
    first = raw_args[0] if raw_args else ""

    SUBCOMMANDS = {"search", "list", "status", "queue", "cache", "dedup", "merge", "stats", "import", "export", "citations", "cite-import", "cite-fetch"}

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
        _build_stats_parser(subparsers)
        _build_import_parser(subparsers)
        _build_export_parser(subparsers)
        _build_citations_parser(subparsers)
        _build_cite_import_parser(subparsers)
        _build_cite_fetch_parser(subparsers)
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
        elif args.subcmd == "cite-import":
            return _run_cite_import(args)
        elif args.subcmd == "cite-fetch":
            return _run_cite_fetch(args)
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
