"""CLI command: cite-import."""
from __future__ import annotations

import argparse
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
        print("Error: json_input required (JSON string or @filename)", file=sys.stderr)
        return 1
    import json
    data_str = args.json_input
    if data_str.startswith("@"):
        with open(data_str[1:], encoding="utf-8") as f:
            data = json.load(f)
    else:
        data = json.loads(data_str)
    edges = data if isinstance(data, list) else data.get("citations", [])
    if not edges:
        print("No citation edges found in input")
        return 0
    db = get_db()
    db.init()
    if args.skip_missing:
        source_ids = list(set(e.get("source") or e.get("from") for e in edges if e.get("source") or e.get("from")))
        target_ids = list(set(e.get("target") or e.get("to") for e in edges if e.get("target") or e.get("to")))
        all_ids = source_ids + target_ids
        existing = db.get_papers_bulk(all_ids)
        edges = [e for e in edges if (e.get("source") or e.get("from")) in existing and (e.get("target") or e.get("to")) in existing]
    if args.dry_run:
        print(f"[dry-run] Would import {len(edges)} citation edge(s)")
        return 0
    added = 0
    for e in edges:
        src = e.get("source") or e.get("from")
        tgt = e.get("target") or e.get("to")
        if src and tgt:
            n = db.add_citation(src, tgt)
            if n:
                added += 1
    print(f"Imported {added}/{len(edges)} citation edge(s)")
    return 0
