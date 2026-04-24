"""CLI command: import."""
from __future__ import annotations

import argparse
import sys
from concurrent.futures import ThreadPoolExecutor
from typing import Tuple

from cli._shared import get_db
from cli._shared import print_success, print_warning, print_error


def _build_import_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser("import", help="Add papers to the database by ID")
    p.add_argument("ids", nargs="*", metavar="ID", help="arXiv IDs, DOIs, or paper UIDs to add")
    p.add_argument("--source", default="import", help="Source label (default: import)")
    p.add_argument("--skip-existing", action="store_true", help="Skip IDs already in database")
    p.add_argument("--file", metavar="FILE", help="Read IDs from file (one per line), or '-' for stdin")
    return p


def _run_import(args: argparse.Namespace) -> int:
    db = get_db()
    db.init()

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
            print_error("Error: no IDs provided (use positional IDs, --file, or pipe into stdin)")
            return 1

    added, skipped, failed = 0, 0, 0
    existing = db.get_papers_bulk(paper_ids)
    existing_ids = {pid for pid in paper_ids if pid.strip() in existing}
    missing_ids = [pid.strip() for pid in paper_ids if pid.strip() not in existing]

    for paper_id in existing_ids:
        skipped += 1
        print_warning(f"Skipped (exists): {paper_id}")

    if missing_ids:
        def _upsert_one(enumerated: Tuple[int, str]) -> Tuple[int, str, bool, str]:
            idx, pid = enumerated
            try:
                db.upsert_paper(pid, args.source)
                return idx, pid, True, ""
            except Exception as e:
                return idx, pid, False, str(e)

        with ThreadPoolExecutor(max_workers=min(len(missing_ids), 8)) as ex:
            results = sorted(ex.map(_upsert_one, enumerate(missing_ids)), key=lambda r: r[0])
        for _, pid, ok, err in results:
            if ok:
                added += 1
                print_success(f"Added: {pid}")
            else:
                failed += 1
                print_error(f"Failed: {pid} — {err}")

    print_success(f"\nImport done: {added} added, {skipped} skipped, {failed} failed")
    return 0
