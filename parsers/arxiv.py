"""arXiv API metadata fetching."""
import logging
import threading
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import List

import feedparser
import requests

from core import ARXIV_API, Paper

logger = logging.getLogger(__name__)
from core.cache import get_cached, set_cached
from core.retry import circuit_breaker

# ─── HTTP Session (lazy init for testability) ─────────────────────────────────
_http_session: requests.Session = None  # type: ignore


def _get_session() -> requests.Session:
    global _http_session
    if _http_session is None:
        _http_session = requests.Session()
    return _http_session

# ─── Rate limiting ────────────────────────────────────────────────────────────
# arXiv API guidelines: < 1 request per 3 seconds
_ARXIV_RATE_LIMIT_DELAY = 3.0  # seconds between requests
_last_arxiv_request_time = 0.0
_rate_limit_lock = threading.Lock()


def _rate_limit() -> None:
    """Enforce ~1 request per 3 seconds for the arXiv API (thread-safe)."""
    global _last_arxiv_request_time
    with _rate_limit_lock:
        now = time.monotonic()
        elapsed = now - _last_arxiv_request_time
        if elapsed < _ARXIV_RATE_LIMIT_DELAY:
            time.sleep(_ARXIV_RATE_LIMIT_DELAY - elapsed)
        _last_arxiv_request_time = time.monotonic()


def _fetch_with_rate_limit_and_backoff(url: str, timeout: int) -> requests.Response:
    """GET url with rate limiting + exponential backoff on 429."""
    _rate_limit()
    r = _get_session().get(url, timeout=timeout)
    if r.status_code == 429:
        # Exponential backoff: try 2, 4, 8 seconds
        for delay in [2.0, 4.0, 8.0]:
            logger.warning("arXiv API rate-limited (429). Retrying in %.1fs.", delay)
            time.sleep(delay)
            r = _get_session().get(url, timeout=timeout)
            if r.status_code != 429:
                break
    r.raise_for_status()
    return r


@circuit_breaker(failure_threshold=5, recovery_timeout=60.0)
def fetch_arxiv_metadata(arxiv_id: str, timeout: int = 30) -> Paper:
    cached = get_cached("arxiv", arxiv_id)
    if cached:
        return _dict_to_paper(cached)

    url = ARXIV_API.format(arxiv_id=arxiv_id)
    r = _fetch_with_rate_limit_and_backoff(url, timeout=timeout)
    feed = feedparser.parse(r.text)
    if not feed.entries:
        raise ValueError(f"arXiv API returned no entries for id: {arxiv_id}")

    # Reuse _parse_entry instead of duplicating the parsing logic
    e = feed.entries[0]
    paper_dict = _parse_entry(e)
    paper_dict["uid"] = arxiv_id  # ensure correct UID
    set_cached("arxiv", arxiv_id, paper_dict)

    return _dict_to_paper(paper_dict)


def _dict_to_paper(d: dict) -> Paper:
    return Paper(
        source=d["source"],
        uid=d["uid"],
        title=d["title"],
        authors=d["authors"],
        abstract=d["abstract"],
        published=d["published"],
        updated=d["updated"],
        abs_url=d["abs_url"],
        pdf_url=d["pdf_url"],
        primary_category=d.get("primary_category", ""),
        categories=d.get("categories", ""),
        comment=d.get("comment", ""),
        journal_ref=d.get("journal_ref", ""),
        doi=d.get("doi", ""),
    )


def _parse_entry(e) -> dict:
    """Parse a single feedparser entry into a paper dict."""
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
        aid = e.id.split("/")[-1]
        pdf_url = f"https://arxiv.org/pdf/{aid}.pdf"

    primary_cat = ""
    try:
        primary_cat = getattr(e, "arxiv_primary_category", {}).get("term", "")  # type: ignore
    except AttributeError:
        logger.debug(f"Failed to get primary category for entry {e.id}: {e}")
        primary_cat = ""

    all_cats = ""
    try:
        tags = getattr(e, "tags", []) or []
        cats = [t.get("term", "") for t in tags if t.get("term")]
        if cats:
            all_cats = ", ".join(cats)
    except AttributeError:
        logger.debug(f"Failed to get tags for entry {e.id}: {e}")
        all_cats = ""

    comment = (getattr(e, "arxiv_comment", None) or "").replace("\n", " ").strip()
    journal_ref = (getattr(e, "journal_ref", None) or "").replace("\n", " ").strip()
    doi = (getattr(e, "arxiv_doi", None) or "").strip()

    return {
        "source": "arxiv",
        "uid": e.id.split("/")[-1],
        "title": title,
        "authors": authors,
        "abstract": abstract,
        "published": published or "",
        "updated": updated or "",
        "abs_url": abs_url,
        "pdf_url": pdf_url,
        "primary_category": primary_cat or "",
        "categories": all_cats,
        "comment": comment,
        "journal_ref": journal_ref,
        "doi": doi,
    }


def fetch_arxiv_metadata_batch(arxiv_ids: List[str], timeout: int = 60) -> List[Paper]:
    """
    Fetch metadata for multiple arXiv papers in a single API call.

    Uses the arXiv `id_list` parameter to batch requests.
    Returns papers in the same order as input IDs.
    Falls back to individual requests for any IDs not in the combined response.
    """
    if not arxiv_ids:
        return []

    ids_to_fetch = []
    id_index = {}  # id -> original position
    papers: List[Paper] = []
    missing_ids = []

    # Check cache first
    for i, aid in enumerate(arxiv_ids):
        aid = aid.strip()
        if not aid:
            continue
        cached = get_cached("arxiv", aid)
        if cached:
            papers.append(_dict_to_paper(cached))
        else:
            ids_to_fetch.append(aid)
            id_index[aid] = i

    if not ids_to_fetch:
        return papers

    # Try batch fetch
    id_list_str = ",".join(ids_to_fetch)
    url = f"https://export.arxiv.org/api/query?id_list={id_list_str}"

    try:
        r = _fetch_with_rate_limit_and_backoff(url, timeout=timeout)
        feed = feedparser.parse(r.text)

        found_ids = set()
        for e in feed.entries:
            aid = e.id.split("/")[-1]
            found_ids.add(aid)
            entry_dict = _parse_entry(e)
            set_cached("arxiv", aid, entry_dict)
            papers.append(_dict_to_paper(entry_dict))

        # IDs not in response → fetch individually
        for aid in ids_to_fetch:
            if aid not in found_ids:
                missing_ids.append(aid)

    except Exception:
        logger.debug("Batch fetch failed, falling back to individual fetches")
        missing_ids = ids_to_fetch

    # Fallback: parallel individual fetches for missing IDs
    if missing_ids:
        with ThreadPoolExecutor(max_workers=min(len(missing_ids), 8)) as executor:
            futures = {executor.submit(fetch_arxiv_metadata, aid, timeout): aid for aid in missing_ids}
            for future in as_completed(futures):
                try:
                    paper = future.result()
                    papers.append(paper)
                except Exception as e:
                    logger.warning(f"Paper fetch failed for {aid}: {e}")

    # Sort by original input order
    uid_to_paper = {p.uid: p for p in papers}
    result = []
    for aid in arxiv_ids:
        aid = aid.strip()
        if not aid:
            continue
        if aid in uid_to_paper:
            result.append(uid_to_paper[aid])

    return result
