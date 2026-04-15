"""arXiv API metadata fetching."""
import re
from typing import List

import feedparser
import requests

from core import ARXIV_API, Paper


def fetch_arxiv_metadata(arxiv_id: str, timeout: int = 30) -> Paper:
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
    for l in getattr(e, "links", []) or []:
        if getattr(l, "type", "") == "application/pdf":
            pdf_url = l.href
            break
    if not pdf_url:
        pdf_url = f"https://arxiv.org/pdf/{arxiv_id}.pdf"

    primary_cat = ""
    try:
        primary_cat = getattr(e, "arxiv_primary_category", {}).get("term", "")  # type: ignore
    except Exception:
        primary_cat = ""

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
    )
