"""arXiv search by keyword query."""
from typing import List

import feedparser
import requests

from core import Paper

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

    feed = feedparser.parse(r.text)

    if not feed.entries:
        return []

    papers: List[Paper] = []
    for e in feed.entries:
        papers.append(_entry_to_paper(e))

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
