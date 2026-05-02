"""Core data structures and constants."""
from dataclasses import dataclass
from typing import List

from core._constants import (
    ARXIV_API,
    CROSSREF_WORKS,
    DOI_RESOLVER,
    RADAR_FILE,
    TIMELINE_FILE,
)

__all__ = [
    "Paper",
    "today_iso",
    "ARXIV_API",
    "CROSSREF_WORKS",
    "DOI_RESOLVER",
    "RADAR_FILE",
    "TIMELINE_FILE",
]


@dataclass
class Paper:
    source: str  # "arxiv" or "doi"
    uid: str  # arXiv id or DOI
    title: str
    authors: List[str]
    abstract: str
    published: str  # YYYY-MM-DD best-effort
    updated: str  # YYYY-MM-DD best-effort
    abs_url: str  # landing page
    pdf_url: str  # direct pdf when known
    primary_category: str = ""
    # --- extended fields (all optional) ---
    journal: str = ""  # journal / container title
    volume: str = ""
    issue: str = ""
    page: str = ""
    doi: str = ""  # DOI when source is arxiv
    comment: str = ""  # arXiv author comment (e.g. page count, v1/v2)
    journal_ref: str = ""  #正式发表期刊引用
    categories: str = ""  # comma-separated full category list
    reference_count: int = 0  # citation / reference count


def today_iso() -> str:
    import datetime as dt
    return dt.date.today().isoformat()
