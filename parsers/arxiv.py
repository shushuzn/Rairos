"""arXiv API metadata fetching."""
from typing import List

import feedparser
import requests

from core import ARXIV_API, Paper
from core.cache import get_cached, set_cached


def fetch_arxiv_metadata(arxiv_id: str, timeout: int = 30) -> Paper:
    cached = get_cached("arxiv", arxiv_id)
    if cached:
        return _dict_to_paper(cached)

    url = ARXIV_API.format(arxiv_id=arxiv_id)
    r = requests.get(url, timeout=timeout)
    r.raise_for_status()
    feed = feedparser.parse(r.text)
    if not feed.entries:
        raise ValueError(f"arXiv API returned no entries for id: {arxiv_id}")

    e = feed.entries[0]
    title = (getattr(e, "title", "") or "").replace("\n", " ").strip()
    abstract = (getattr(e, "summary", "") or "").replace("\n", " ").strip()

    authors: List[str] = []
    for a in getattr(e, "authors", []) or []:
        name = getattr(a, "name", "").strip()
        if name:
            authors.append(name)

    published = (getattr(e, "published", "") or "")[:10]
    updated = (getattr(e, "updated", "") or "")[:10]
    abs_url = getattr(e, "link", "") or f"https://arxiv.org/abs/{arxiv_id}"

    pdf_url = ""
    for link_item in getattr(e, "links", []) or []:
        if getattr(link_item, "type", "") == "application/pdf":
            pdf_url = link_item.href
            break
    if not pdf_url:
        pdf_url = f"https://arxiv.org/pdf/{arxiv_id}.pdf"

    primary_cat = ""
    try:
        primary_cat = getattr(e, "arxiv_primary_category", {}).get("term", "")  # type: ignore
    except Exception:
        primary_cat = ""

    # All categories as comma-separated string
    all_cats = ""
    try:
        tags = getattr(e, "tags", []) or []
        cats = [t.get("term", "") for t in tags if t.get("term")]
        if cats:
            all_cats = ", ".join(cats)
    except Exception:
        all_cats = ""

    # Author comment (pages, version, etc.)
    comment = (getattr(e, "arxiv_comment", None) or "").replace("\n", " ").strip()

    # Journal reference (e.g. "Nature 2023")
    journal_ref = (getattr(e, "journal_ref", None) or "").replace("\n", " ").strip()

    # DOI (if present)
    doi = (getattr(e, "arxiv_doi", None) or "").strip()

    paper_dict = {
        "source": "arxiv",
        "uid": arxiv_id,
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
    set_cached("arxiv", arxiv_id, paper_dict)

    return Paper(
        source="arxiv",
        uid=arxiv_id,
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
        r = requests.get(url, timeout=timeout)
        r.raise_for_status()
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
        missing_ids = ids_to_fetch

    # Fallback: individual fetches for missing IDs
    for aid in missing_ids:
        try:
            paper = fetch_arxiv_metadata(aid, timeout=timeout)
            papers.append(paper)
        except Exception:
            # Last resort: return what we have
            pass

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
