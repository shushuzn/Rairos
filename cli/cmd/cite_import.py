"""CLI command: cite-import."""
from __future__ import annotations

import argparse
import json as json_lib
import sys
import re as _re
from typing import List

from cli._shared import get_db
from cli._shared import (
    Colors, colored, print_success, print_error, print_warning, print_info, print_header,
)


def _extract_references_from_text(paper_id: str, text: str) -> dict[str, list[str]]:
    """Extract arXiv IDs, DOIs, PMIDs, and ISBNs from the plain text of a paper."""
    if not text or not text.strip():
        return {"arxiv_ids": [], "dois": [], "pmids": [], "isbns": []}

    arXiv_PAT = _re.compile(r'\barXiv:\s*(\d+\.\d+\b)', _re.IGNORECASE)
    DOI_PAT = _re.compile(r'\b10\.\d{4,}/[^\s]+', _re.IGNORECASE)
    PMID_PAT = _re.compile(r'\bPMID:\s*(\d{6,})\b', _re.IGNORECASE)
    ISBN_PAT = _re.compile(r'\bISBN(?:-13)?:?\s*([0-9-X]{10,})\b', _re.IGNORECASE)

    _REFS_SECTION_PAT = _re.compile(r'(?:\n|^)[ ]*(?:\d+\.?\s*)?(?:References|Bibliography|Citations)', _re.IGNORECASE)
    match = _REFS_SECTION_PAT.search(text)
    if match:
        text = text[match.start():]

    arxiv_ids = list(set(arXiv_PAT.findall(text)))
    dois = list(set(DOI_PAT.findall(text)))
    pmids = list(set(PMID_PAT.findall(text)))
    isbns = list(set(ISBN_PAT.findall(text)))

    return {"arxiv_ids": arxiv_ids, "dois": dois, "pmids": pmids, "isbns": isbns}


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
    p.add_argument("--dry-run", action="store_true", help="Show what would be imported without writing to DB")
    p.add_argument("--skip-missing", action="store_true", help="Skip source/target papers that don't exist in the DB")
    p.add_argument("--extract", action="store_true", help="Extract citation references from a paper's plain_text (requires --paper)")
    p.add_argument("--paper", metavar="PAPER_ID", dest="extract_paper", help="Paper ID for --extract mode")
    p.add_argument("--dedup", action="store_true", help="Use upsert mode to report duplicate citation edges")
    return p


def _run_cite_import(args: argparse.Namespace) -> int:
    if getattr(args, "extract", False):
        paper_id = getattr(args, "extract_paper", None)
        if not paper_id:
            print("Error: --paper PAPER_ID required with --extract", file=sys.stderr)
            return 1
        db = get_db()
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
        db_ids: list[str] = []
        for aid in arxiv_ids:
            full = f"arXiv:{aid}"
            if db.paper_exists(full):
                db_ids.append(full)
        if db_ids:
            if args.dry_run:
                print(f"\n[dry-run] Would import {len(db_ids)} citation edge(s):")
            else:
                if getattr(args, "dedup", False):
                    new_n, dup_n = db.upsert_citations(paper_id, db_ids)
                    print(f"\nImported {new_n} new edge(s), {dup_n} duplicate(s) skipped")
                else:
                    n = db.add_citations_batch(paper_id, db_ids)
                    print(f"\nImported {n} citation edge(s)")
        return 0

    if not args.json_input:
        print("Error: json_input required (JSON string or @filepath)", file=sys.stderr)
        return 1

    raw = args.json_input
    if raw.startswith("@"):
        path = raw[1:]
        try:
            with open(path, encoding="utf-8") as f:
                data = json_lib.loads(f.read())
        except Exception as e:
            print(f"Error reading {path}: {e}", file=sys.stderr)
            return 1
    else:
        try:
            data = json_lib.loads(raw)
        except json_lib.JSONDecodeError as e:
            print(f"Error: invalid JSON: {e}", file=sys.stderr)
            return 1

    # Normalise to list
    if isinstance(data, dict):
        data = [data]
    elif not isinstance(data, list):
        print("Error: JSON must be a list of objects or a single object", file=sys.stderr)
        return 1

    db = get_db()
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
        for err in errors[:10]:
            print(f"    - {err}")
        if len(errors) > 10:
            print(f"    ... and {len(errors) - 10} more")
        return 1

    print("Import complete.")
    print(f"  new citations : {total_new}")
    print(f"  skipped (missing papers): {total_skip_missing}")
    return 0
