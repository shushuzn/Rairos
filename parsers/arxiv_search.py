"""arXiv search by keyword query."""

import hashlib
import json
import logging
from datetime import datetime as _dt, timedelta, timezone
from typing import List

import feedparser
import requests  # type: ignore[import-untyped]

from core import Paper

logger = logging.getLogger(__name__)

# Module-level session for connection reuse
_http = requests.Session()


def search_arxiv(query: str, max_results: int = 5, timeout: int = 30) -> List[Paper]:
    """
    Search arXiv by keyword and return metadata for top papers.

    Args:
        query: Search query (supports arXiv advanced operators like AND, OR, TITLE, ABS)
        max_results: Number of papers to return (default 5)
        timeout: Request timeout in seconds

    Returns:
        List of Paper objects sorted by relevance (best match first)

    Raises:
        RuntimeError: If the search request fails
    """
    import urllib.parse

    # Encode query for URL
    encoded_query = urllib.parse.quote_plus(query)
    url = (
        f"https://export.arxiv.org/api/query?"
        f"search_query=all:{encoded_query}&"
        f"start=0&"
        f"max_results={max_results}&"
        f"sortBy=relevance&"
        f"sortOrder=descending"
    )

    try:
        r = _http.get(url, timeout=timeout)
        r.raise_for_status()
    except Exception as e:
        raise RuntimeError(f"arXiv search failed for query '{query}': {e}") from e

    try:
        feed = feedparser.parse(r.text)
    except Exception as e:
        raise RuntimeError(f"arXiv feed parse failed for query '{query}': {e}") from e

    if not feed.entries:
        return []

    papers: List[Paper] = []
    for entry in feed.entries:
        papers.append(_entry_to_paper(entry))

    return papers


def _entry_to_paper(e) -> Paper:
    """Convert a feedparser entry to a Paper object."""
    title = (getattr(e, "title", "") or "").replace("\n", " ").strip()
    abstract = (getattr(e, "summary", "") or "").replace("\n", " ").strip()

    authors: List[str] = []
    for a in getattr(e, "authors", []) or []:
        name = getattr(a, "name", "").strip()
        if name:
            authors.append(name)

    published = (getattr(e, "published", "") or "")[:10]
    updated = (getattr(e, "updated", "") or "")[:10]
    abs_url = getattr(e, "link", "") or f"https://arxiv.org/abs/{e.id.split('/')[-1]}"

    pdf_url = ""
    for link_item in getattr(e, "links", []) or []:
        if getattr(link_item, "type", "") == "application/pdf":
            pdf_url = link_item.href
            break
    if not pdf_url:
        pdf_url = f"https://arxiv.org/pdf/{e.id.split('/')[-1]}.pdf"

    primary_cat = ""
    try:
        primary_cat = getattr(e, "arxiv_primary_category", {}).get("term", "")  # type: ignore
    except Exception:
        primary_cat = ""

    all_cats = ""
    try:
        tags = getattr(e, "tags", []) or []
        cats = [t.get("term", "") for t in tags if t.get("term")]
        if cats:
            all_cats = ", ".join(cats)
    except Exception:
        all_cats = ""

    comment = (getattr(e, "arxiv_comment", None) or "").replace("\n", " ").strip()
    journal_ref = (getattr(e, "journal_ref", None) or "").replace("\n", " ").strip()
    doi = (getattr(e, "arxiv_doi", None) or "").strip()

    return Paper(
        source="arxiv",
        uid=e.id.split("/")[-1],
        title=title,
        authors=authors,
        abstract=abstract,
        published=published or "",
        updated=updated or "",
        abs_url=abs_url,
        pdf_url=pdf_url,
        primary_category=primary_cat or "",
        categories=all_cats,
        comment=comment,
        journal_ref=journal_ref,
        doi=doi,
    )

# ─── ArXiv Search Cache ────────────────────────────────────────────────────────

_CACHE_TTL = timedelta(hours=24)

def _get_cache_db():
    """Get a DB connection for cache (lazy init to avoid import cycles)."""
    from db.database import Database
    db = Database()
    db.init()
    return db

def _query_hash(query: str) -> str:
    """Normalize and hash a query for cache key."""
    normalized = " ".join(query.lower().split())
    return hashlib.sha256(normalized.encode()).hexdigest()[:32]

def _paper_to_dict(paper) -> dict:
    """Serialize a Paper object to dict for JSON storage."""
    return {
        "source": paper.source,
        "uid": paper.uid,
        "title": paper.title,
        "authors": paper.authors,
        "abstract": paper.abstract,
        "published": paper.published,
        "updated": paper.updated,
        "abs_url": paper.abs_url,
        "pdf_url": paper.pdf_url,
        "primary_category": paper.primary_category,
        "categories": paper.categories,
    }

def _dict_to_paper(d: dict):
    """Restore a Paper object from dict (e.g. from cache)."""
    from dataclasses import fields as _fields
    from core import Paper
    valid = {f.name for f in _fields(Paper)}
    return Paper(**{k: v for k, v in d.items() if k in valid})

def search_arxiv_cached(query: str, max_results: int = 5, timeout: int = 30, ttl_hours: int = 24) -> List:
    """Cached arXiv search.

    1. Check local SQLite cache for this query
    2. If fresh (within TTL) and has results → return cache
    3. If 429 or network error → return stale cache if available
    4. Otherwise call arXiv API and update cache
    """
    db = _get_cache_db()
    qhash = _query_hash(query)
    ttl = timedelta(hours=ttl_hours)
    now = _dt.utcnow()

    # Check cache
    row = db.conn.execute(
        "SELECT results_json, created_at, hit_count FROM arxiv_search_cache WHERE query_hash = ?",
        (qhash,),
    ).fetchone()

    if row:
        results_json, created_at_str, hit_count = row
        created_at = _dt.fromisoformat(created_at_str)
        age = now - created_at
        if age < ttl:
            # Fresh cache hit
            db.conn.execute(
                "UPDATE arxiv_search_cache SET hit_count = hit_count + 1 WHERE query_hash = ?",
                (qhash,),
            )
            db.conn.commit()
            results = json.loads(results_json)  # type: ignore[arg-type]
            return [_dict_to_paper(d) for d in results[:max_results]]

    # Cache miss or stale — call API
    try:
        papers = search_arxiv(query, max_results=max_results, timeout=timeout)
        results_dicts = [_paper_to_dict(p) for p in papers]
        db.conn.execute(
            """INSERT OR REPLACE INTO arxiv_search_cache
               (query_hash, query, results_json, created_at, hit_count)
               VALUES (?, ?, ?, ?, 1)""",
            (qhash, query, json.dumps(results_dicts), now.isoformat()),
        )
        db.conn.commit()
        return papers
    except RuntimeError as e:
        error_msg = str(e)
        # 429 or network error → try stale cache
        if "429" in error_msg or "timeout" in error_msg.lower() or "connection" in error_msg.lower():
            if row:
                # Return stale cache (even if expired)
                results = json.loads(row[0])  # type: ignore[arg-type]
                return [_dict_to_paper(d) for d in results[:max_results]]
        raise  # Re-raise other errors
    except Exception:
        # Unexpected error (TypeError, JSONDecodeError, AttributeError, etc.)
        # → try stale cache as last resort, otherwise return empty list
        if row:
            try:
                results = json.loads(row[0])
                return [_dict_to_paper(d) for d in results[:max_results]]
            except Exception:
                return []
        return []
