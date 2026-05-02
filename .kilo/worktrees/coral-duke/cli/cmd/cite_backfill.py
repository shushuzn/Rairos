"""CLI command: cite-backfill — Backfill missing citation data from OpenAlex.

Identifies papers in the database that have no forward citation records,
then fetches their forward citation chains from OpenAlex to populate
the citations table so that influence/trend commands work correctly.
"""
from __future__ import annotations

import argparse
import ssl
import sys
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from threading import Lock

import orjson as json

from cli._shared import get_db, Colors, colored, print_info, print_success


_OPENALEX_BASE = "https://api.openalex.org"
_OPENALEX_EMAIL = "ai-research-os@example.com"


def _build_openalex_ctx() -> ssl.SSLContext:
    ctx = ssl.create_default_context()
    ctx.check_hostname = False
    ctx.verify_mode = ssl.CERT_NONE
    return ctx


def _openalex_request(path: str, timeout: int = 15) -> dict:
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


def _arxiv_doi_to_openalex(arxiv_id: str) -> str | None:
    """Look up OpenAlex ID for an arXiv paper."""
    clean_id = arxiv_id.rsplit("v", 1)[0] if arxiv_id[-1] in "0123456789" else arxiv_id
    doi = f"10.48550/arXiv.{clean_id}"
    try:
        d = _openalex_request(f"/works?filter=doi:{doi}&per-page=1")
        results = d.get("results", [])
        if results:
            return results[0]["id"]
    except Exception:
        pass
    return None


def _get_forward_citations(openalex_id: str, per_page: int = 200) -> tuple[list[dict], int]:
    """Fetch all papers citing this paper. Returns (works, total_count)."""
    oid = openalex_id.rstrip("/").split("/")[-1]
    try:
        d = _openalex_request(
            f"/works?filter=cites:{oid}&per-page={per_page}&mailto={_OPENALEX_EMAIL}"
        )
        return d.get("results", []) or [], d.get("meta", {}).get("count", 0)
    except Exception as e:
        print(f"    Warning: citing lookup failed: {e}", file=sys.stderr)
        return [], 0


def _work_to_arxiv_id(work: dict) -> str | None:
    ids = work.get("ids", {}) or {}
    doi = ids.get("doi", "") or ""
    if "/arxiv." in doi.lower():
        return doi.lower().split("/arxiv.")[-1]
    return None


@dataclass
class GapStats:
    total_papers: int
    with_citations: int
    without_citations: int
    total_forward_cites_stored: int


def _compute_gap_stats(db) -> GapStats:
    """Compute statistics about citation coverage."""
    cur = db.conn.execute("SELECT COUNT(*) FROM papers")
    total = cur.fetchone()[0]

    cur = db.conn.execute("SELECT COUNT(DISTINCT target_id) FROM citations")
    with_cites = cur.fetchone()[0]

    subq = (
        "SELECT target_id, COUNT(*) AS forward_cites "
        "FROM citations GROUP BY target_id"
    )
    cur = db.conn.execute(f"SELECT SUM(forward_cites) FROM ({subq})")
    total_fwd = cur.fetchone()[0] or 0

    return GapStats(
        total_papers=total,
        with_citations=with_cites,
        without_citations=total - with_cites,
        total_forward_cites_stored=total_fwd,
    )


def _find_papers_without_citations(db) -> list[tuple[str, str]]:
    """Find papers that have no forward citation records.

    Returns list of (paper_id, published_year).
    Only considers papers published from year 2000 onward.
    """
    cur = db.conn.execute("""
        SELECT p.id, p.published
        FROM papers p
        LEFT JOIN citations c ON c.target_id = p.id
        WHERE c.target_id IS NULL
          AND (p.published >= '2000' OR p.published IS NULL)
        GROUP BY p.id
        HAVING COUNT(c.id) = 0
    """)
    results = []
    for row in cur.fetchall():
        pub = row[1] or ""
        if pub and pub >= "2000":
            results.append((row[0], pub[:4]))
        elif not pub:
            results.append((row[0], "0000"))
    return results


def _run_cite_backfill(args: argparse.Namespace) -> int:
    db = get_db()
    db.init()

    if args.stats:
        stats = _compute_gap_stats(db)
        print()
        print(colored("=== Citation Coverage Report ===", Colors.HEADER))
        print(f"  Total papers in database : {stats.total_papers}")
        print(f"  Papers with citations   : {stats.with_citations}")
        missing = stats.without_citations
        print(f"  Papers missing citations: {colored(str(missing), Colors.WARNING if missing > 0 else Colors.OKGREEN)}")
        print(f"  Total forward cites stored: {stats.total_forward_cites_stored}")
        print()
        if missing > 0:
            print("Run with --backfill to populate missing citation data from OpenAlex.")
        return 0

    if not args.backfill and not args.dry_run:
        print("Specify --backfill to actually fetch citation data.", file=sys.stderr)
        return 1

    papers = _find_papers_without_citations(db)
    if not papers:
        print_success("All papers already have citation data!")
        return 0

    paper_ids = [p[0] for p in papers]
    print_info(f"Found {len(papers)} papers without citation data")

    if args.dry_run:
        print()
        print(colored("=== Dry Run — Papers to Backfill ===", Colors.HEADER))
        for pid, year in papers[:20]:
            print(f"  {pid}  ({year})")
        if len(papers) > 20:
            print(f"  ... and {len(papers) - 20} more")
        print()
        print("Re-run without --dry-run to actually backfill.")
        return 0

    all_rows, _ = db.list_papers()
    known_ids = {p.id for p in all_rows}

    lock = Lock()
    added = [0]
    errors = [0]
    imported_papers = [0]

    def _backfill_one(paper_id: str) -> None:
        try:
            oid = _arxiv_doi_to_openalex(paper_id)
            if not oid:
                return

            citing_works, total_count = _get_forward_citations(oid)
            if not citing_works and total_count == 0:
                return

            for work in citing_works:
                citing_arxiv = _work_to_arxiv_id(work)
                if not citing_arxiv:
                    continue

                if citing_arxiv not in known_ids:
                    try:
                        ids = work.get("ids", {}) or {}
                        doi = ids.get("doi", "") or ""
                        title = work.get("title", "") or f"Imported {citing_arxiv}"
                        authors = ",".join(
                            a.get("display_name", "")
                            for a in (work.get("authorships") or [])
                        ) or ""
                        pub_date = work.get("publication_date") or ""
                        journal = (
                            work.get("primary_location", {})
                            .get("source", {})
                            .get("display_name", "")
                        ) or ""

                        db.upsert_paper(
                            paper_id=citing_arxiv,
                            source="openalex",
                            title=title,
                            authors=authors,
                            abstract="",
                            published=pub_date,
                            abs_url="",
                            pdf_url="",
                            journal=journal,
                            doi=doi,
                            reference_count=0,
                        )
                        known_ids.add(citing_arxiv)
                        with lock:
                            imported_papers[0] += 1
                    except Exception:
                        pass

                try:
                    db.add_citation(citing_arxiv, paper_id)
                    with lock:
                        added[0] += 1
                except Exception:
                    pass

        except Exception:
            with lock:
                errors[0] += 1

    print_info(f"Backfilling {len(paper_ids)} papers using {args.workers} workers...")

    with ThreadPoolExecutor(max_workers=min(args.workers, 10)) as ex:
        ex.map(_backfill_one, paper_ids)

    print()
    print_success(f"Done! Citations added: {added[0]}")
    if imported_papers[0] > 0:
        print_info(f"New papers imported: {imported_papers[0]}")
    if errors[0] > 0:
        print(f"Errors: {errors[0]}", file=sys.stderr)

    stats = _compute_gap_stats(db)
    print()
    print(colored("=== Updated Coverage ===", Colors.HEADER))
    print(f"  Papers now with citations: {stats.with_citations}/{stats.total_papers}")
    print(f"  Total forward cites stored: {stats.total_forward_cites_stored}")
    return 0


def _build_cite_backfill_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "cite-backfill",
        help="Backfill missing citation data from OpenAlex",
        description="Find papers without citation records and fetch their "
                    "forward citation chains from OpenAlex to populate the citations table.",
    )
    p.add_argument(
        "--stats", "-s",
        action="store_true",
        help="Show citation coverage statistics only",
    )
    p.add_argument(
        "--backfill", "-b",
        action="store_true",
        help="Actually fetch citation data from OpenAlex (default: dry-run)",
    )
    p.add_argument(
        "--dry-run",
        action="store_true",
        default=True,
        help="Preview papers that would be backfilled (default: True)",
    )
    p.add_argument(
        "--no-dry-run",
        action="store_false",
        dest="dry_run",
        help="Actually backfill (same as --backfill)",
    )
    p.add_argument(
        "--workers", "-w",
        type=int,
        default=5,
        help="Number of parallel workers (default: 5)",
    )
    return p
