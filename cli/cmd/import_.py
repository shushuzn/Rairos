"""CLI command: import."""
from __future__ import annotations

import argparse
import json
import sys
import os
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime
from pathlib import Path
from typing import Tuple, Set, Optional

from cli._shared import get_db
from cli._shared import print_success, print_warning, print_error, print_info

CHECKPOINT_VERSION = 1


def _fetch_paper_metadata(paper_id: str) -> Optional[dict]:
    """Fetch paper metadata from arXiv or DOI."""
    from parsers.arxiv import fetch_arxiv_metadata
    from parsers.crossref import fetch_crossref_metadata
    from parsers.input_detection import is_probably_doi, normalize_doi

    try:
        if is_probably_doi(paper_id):
            # DOI lookup
            doi = normalize_doi(paper_id)
            paper, _ = fetch_crossref_metadata(doi)
            return {
                "title": paper.title or "",
                "authors": paper.authors or [],
                "abstract": paper.abstract or "",
                "published": paper.published or "",
                "abs_url": f"https://doi.org/{doi}",
                "pdf_url": "",
                "primary_category": "",
                "doi": doi,
            }
        else:
            # arXiv lookup
            paper = fetch_arxiv_metadata(paper_id)
            return {
                "title": paper.title or "",
                "authors": paper.authors or [],
                "abstract": paper.abstract or "",
                "published": paper.published or "",
                "abs_url": paper.abs_url or "",
                "pdf_url": paper.pdf_url or "",
                "primary_category": paper.primary_category or "",
                "doi": paper.doi or "",
            }
    except Exception as e:
        print_warning(f"Failed to fetch metadata for {paper_id}: {e}")
        return None


def _load_checkpoint(checkpoint_path: Path) -> dict:
    """Load checkpoint file."""
    if not checkpoint_path.exists():
        return {"version": CHECKPOINT_VERSION, "processed": [], "failed": [], "total": 0}
    try:
        with open(checkpoint_path, encoding="utf-8") as f:
            return json.load(f)
    except (json.JSONDecodeError, IOError):
        return {"version": CHECKPOINT_VERSION, "processed": [], "failed": [], "total": 0}


def _save_checkpoint(checkpoint_path: Path, processed: list, failed: list, total: int) -> None:
    """Save checkpoint file atomically."""
    temp_path = checkpoint_path.with_suffix(".tmp")
    data = {
        "version": CHECKPOINT_VERSION,
        "processed": processed,
        "failed": failed,
        "total": total,
        "saved_at": datetime.now().isoformat(),
    }
    with open(temp_path, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2)
    temp_path.replace(checkpoint_path)


def _build_import_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "import",
        help="Add papers to the database by ID",
        prog="airos import",
        description="Import papers by arXiv ID, DOI, or paper UID. Supports batch import from file.",
        epilog="""\
Examples:
  %(prog)s 2301.00001 2301.00002           # import two arXiv papers
  %(prog)s 10.1038/nature12373             # import by DOI
  %(prog)s --file ids.txt                   # batch import from file
  %(prog)s --file - --skip-existing         # stdin, skip existing papers
  %(prog)s --checkpoint resume.json --resume # resume interrupted import""",
    )
    p.add_argument("ids", nargs="*", metavar="ID", help="arXiv IDs, DOIs, or paper UIDs to add")
    p.add_argument("--source", default="import", help="Source label (default: import)")
    p.add_argument("--skip-existing", action="store_true", help="Skip IDs already in database")
    p.add_argument("--file", metavar="FILE", help="Read IDs from file (one per line), or '-' for stdin")
    p.add_argument("--checkpoint", metavar="FILE", help="Save/resume progress to checkpoint file")
    p.add_argument("--resume", action="store_true", help="Resume from checkpoint (skip processed IDs)")
    return p


def _run_import(args: argparse.Namespace) -> int:
    db = get_db()
    db.init()

    _has_file = bool(getattr(args, "file", None))
    checkpoint_path = Path(args.checkpoint) if getattr(args, "checkpoint", None) else None
    resume_mode = getattr(args, "resume", False)

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
            print_error("Error: no IDs provided (use positional IDs, --file, or pipe into stdin)")
            return 1

    # Load checkpoint for resume
    processed_ids: Set[str] = set()
    failed_ids: list = []
    if resume_mode and checkpoint_path:
        checkpoint = _load_checkpoint(checkpoint_path)
        processed_ids = set(checkpoint.get("processed", []))
        failed_ids = checkpoint.get("failed", [])
        print_info(f"Resuming: {len(processed_ids)} processed, {len(failed_ids)} failed")

    added, skipped, new_failed = 0, 0, 0
    existing = db.get_papers_bulk(paper_ids)

    # Build missing list, excluding already processed if resuming
    missing_ids = []
    for pid in paper_ids:
        pid = pid.strip()
        if not pid:
            continue
        if pid in existing:
            skipped += 1
            print_warning(f"Skipped (exists): {pid}")
        elif resume_mode and pid in processed_ids:
            skipped += 1
            print_warning(f"Skipped (processed): {pid}")
        else:
            missing_ids.append(pid)

    if missing_ids:
        def _upsert_one(enumerated: Tuple[int, str]) -> Tuple[int, str, bool, str]:
            idx, pid = enumerated
            try:
                # Fetch metadata first
                metadata = _fetch_paper_metadata(pid)
                if metadata:
                    db.upsert_paper(
                        paper_id=pid,
                        source=args.source,
                        title=metadata.get("title", ""),
                        authors=metadata.get("authors", []),
                        abstract=metadata.get("abstract", ""),
                        published=metadata.get("published", ""),
                        abs_url=metadata.get("abs_url", ""),
                        pdf_url=metadata.get("pdf_url", ""),
                        primary_category=metadata.get("primary_category", ""),
                        doi=metadata.get("doi", ""),
                    )
                else:
                    # Fallback: just insert empty record
                    db.upsert_paper(pid, args.source)
                return idx, pid, True, ""
            except Exception as e:
                return idx, pid, False, str(e)

        processed_this_run = []
        with ThreadPoolExecutor(max_workers=min(len(missing_ids), 8)) as ex:
            results = sorted(ex.map(_upsert_one, enumerate(missing_ids)), key=lambda r: r[0])
        for _, pid, ok, err in results:
            if ok:
                added += 1
                processed_this_run.append(pid)
                print_success(f"Added: {pid}")
            else:
                new_failed += 1
                failed_ids.append(pid)
                print_error(f"Failed: {pid} — {err}")

            # Save checkpoint after each batch
            if checkpoint_path and (len(processed_this_run) % 10 == 0 or len(processed_this_run) == len(missing_ids)):
                all_processed = list(processed_ids) + processed_this_run
                _save_checkpoint(checkpoint_path, all_processed, failed_ids, len(paper_ids))

    # Final checkpoint save
    if checkpoint_path and missing_ids:
        all_processed = list(processed_ids) + [pid for pid in missing_ids if pid not in failed_ids]
        _save_checkpoint(checkpoint_path, all_processed, failed_ids, len(paper_ids))

    total_failed = len(failed_ids)
    print_success(f"\nImport done: {added} added, {skipped} skipped, {total_failed} failed")
    if checkpoint_path:
        print_info(f"Checkpoint saved: {checkpoint_path}")
    return 0
