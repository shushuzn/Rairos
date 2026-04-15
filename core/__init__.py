"""Core data structures and constants."""
from dataclasses import dataclass
from pathlib import Path
from typing import List

ARXIV_API = "https://export.arxiv.org/api/query?id_list={arxiv_id}"
CROSSREF_WORKS = "https://api.crossref.org/works/{doi}"
DOI_RESOLVER = "https://doi.org/"

RADAR_FILE = "Radar.md"
TIMELINE_FILE = "Timeline.md"


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


def today_iso() -> str:
    import datetime as dt
    return dt.date.today().isoformat()
