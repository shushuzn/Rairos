"""CLI command: cite-fetch."""
from __future__ import annotations

import argparse
import logging
import ssl
import sys
import time
import urllib.request
import warnings
from typing import List, Optional, Tuple

import orjson as json

from cli._shared import get_db

logger = logging.getLogger(__name__)

_OPENALEX_BASE = "https://api.openalex.org"
_OPENALEX_EMAIL = "ai-research-os@example.com"


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
            return json.loads(resp.read())
    except Exception as e:
        raise RuntimeError(f"OpenAlex request failed for {path}: {e}") from e


def _arxiv_doi_to_openalex(arxiv_id: str) -> Optional[str]:
    """Query OpenAlex for a paper by arXiv ID, return OpenAlex ID or None."""
    # Strip version suffix so DOI is e.g. 10.48550/arXiv.2401.15391 (not .2401.15391v1)
    clean_id = arxiv_id.rsplit("v", 1)[0] if arxiv_id[-1] in "0123456789" else arxiv_id
    doi = f"10.48550/arXiv.{clean_id}"
    try:
        d = _openalex_request(f"/works?filter=doi:{doi}&per-page=1")
        results = d.get("results", [])
        if results:
            return results[0]["id"]
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
    if "/arxiv." in doi.lower():
        return doi.lower().split("/arxiv.")[-1]
    return None


def _work_to_paper_record(work: dict, source: str = "openalex") -> Optional["PaperRecord"]:
    """Convert an OpenAlex work dict to a PaperRecord. Accepts any paper with an ID."""
    from db.database import PaperRecord

    ids = work.get("ids", {}) or {}

    # Prefer arXiv ID; fall back to OpenAlex ID; last resort is DOI
    paper_id = _work_to_arxiv_id(work)
    if not paper_id:
        openalex_id = ids.get("openalex", "")
        if openalex_id:
            paper_id = openalex_id.rstrip("/").split("/")[-1]
        else:
            doi = ids.get("doi", "") or ""
            if doi:
                paper_id = doi.rstrip("/").split("/")[-1]
        if not paper_id:
            return None

    primary_location = (work.get("primary_location") or {})
    best_oa = (primary_location.get("best_oa_location") or {})
    landing = best_oa.get("landing_page_url") or ids.get("doi") or ""

    authors = [
        au.get("author", {}).get("display_name", "")
        for au in (work.get("authorships") or [])
        if au.get("author", {}).get("display_name")
    ]

    raw_date = work.get("publication_date") or ""
    year = raw_date[:4] if raw_date else ""

    topics = work.get("topics", [])
    topic_ids = ""
    if isinstance(topics, list):
        topic_ids = ",".join(t.get("display_name", "") for t in topics[:10])
    elif isinstance(topics, dict):
        topic_ids = topics.get("display_name", "") or ""

    return PaperRecord(
        id=paper_id,
        source=source,
        title=work.get("title", "") or "",
        authors=authors,
        abstract="",
        published=year,
        updated="",
        abs_url=landing,
        pdf_url=best_oa.get("pdf_url") or "",
        primary_category="",
        journal=work.get("host_venue", {}).get("display_name", "") if isinstance(work.get("host_venue"), dict) else "",
        volume="",
        issue="",
        page="",
        doi=ids.get("doi", "") or "",
        categories=topic_ids,
        reference_count=work.get("referenced_works_count", 0),
        pdf_path="",
        pdf_hash="",
    )


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
    from concurrent.futures import ThreadPoolExecutor

    db = get_db()
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
    total_imported = [0]

    def _fetch_ref(ref_oid: str, paper_id: str):
        try:
            oid = ref_oid.rstrip("/").split("/")[-1]
            ref_work = _openalex_request(f"/works/{oid}")
            ref_arxiv_id = _work_to_arxiv_id(ref_work)
            if not ref_arxiv_id:
                return

            pr = _work_to_paper_record(ref_work)
            if not pr:
                return

            with lock:
                if ref_arxiv_id not in known_ids:
                    db.upsert_paper(
                        paper_id=pr.id,
                        source=pr.source,
                        title=pr.title,
                        authors=pr.authors,
                        abstract=pr.abstract,
                        published=pr.published,
                        abs_url=pr.abs_url,
                        pdf_url=pr.pdf_url,
                        journal=pr.journal,
                        doi=pr.doi,
                        reference_count=pr.reference_count,
                    )
                    known_ids.add(ref_arxiv_id)
                    total_imported[0] += 1

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
            pr = _work_to_paper_record(citing_work)
            if not pr:
                return

            with lock:
                if pr.id not in known_ids:
                    db.upsert_paper(
                        paper_id=pr.id,
                        source=pr.source,
                        title=pr.title,
                        authors=pr.authors,
                        abstract=pr.abstract,
                        published=pr.published,
                        abs_url=pr.abs_url,
                        pdf_url=pr.pdf_url,
                        journal=pr.journal,
                        doi=pr.doi,
                        reference_count=pr.reference_count,
                    )
                    known_ids.add(pr.id)
                    total_imported[0] += 1

            added = db.add_citation(pr.id, paper_id)
            with lock:
                if added:
                    total_added[0] += 1

        except Exception as e:
            with lock:
                total_errors[0] += 1
            warnings.warn(f"citing: {e}", stacklevel=2)

    for paper_id in paper_ids:
        logger.debug("Processing paper", extra={"uid": paper_id})

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
    print(f"  Papers imported from OpenAlex: {total_imported[0]}", file=sys.stderr)
    print(f"  Errors: {total_errors[0]}", file=sys.stderr)
    if direction in ("to", "both"):
        print(f"  Note: {total_cited_by_count[0]} forward citations found (some may be outside DB)", file=sys.stderr)

    return 0
