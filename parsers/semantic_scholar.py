"""Semantic Scholar API search with retry logic."""

import logging
import time
from typing import List, Optional, Dict, Any

import httpx

from core import Paper

logger = logging.getLogger(__name__)

_S2_API = "https://api.semanticscholar.org/graph/v1"
_MAX_RETRIES = 3
_RETRY_DELAY = 1.0

# Module-level singleton client — avoids repeated TCP/TLS handshakes
_client: Optional[httpx.Client] = None


def _get_client() -> httpx.Client:
    """Return the shared HTTP client (lazy singleton)."""
    global _client
    if _client is None:
        _client = httpx.Client(timeout=30.0, follow_redirects=True)
    return _client


def search_semantic_scholar(
    query: str,
    max_results: int = 10,
    timeout: float = 30.0,
    fields: Optional[List[str]] = None,
) -> List["S2Paper"]:
    """
    Search Semantic Scholar by keyword with retry logic.

    Args:
        query: Search query
        max_results: Number of papers to return (default 10)
        timeout: Request timeout in seconds
        fields: List of fields to request

    Returns:
        List of S2Paper dicts sorted by relevance
    """
    if fields is None:
        fields = [
            "title",
            "authors",
            "abstract",
            "year",
            "venue",
            "citationCount",
            "openAccessPdf",
            "paperId",
        ]

    encoded_query = _urlencode(query)
    url = (
        f"{_S2_API}/paper/search?"
        f"query={encoded_query}&"
        f"limit={max_results}&"
        f"fields={','.join(fields)}"
    )

    last_error: Exception = RuntimeError("unknown error")
    for attempt in range(_MAX_RETRIES):
        try:
            client = _get_client()
            r = client.get(url)
            if r.status_code == 429:
                retry_after = int(r.headers.get("retry-after", "60"))
                logger.warning(f"Rate limited, waiting {retry_after}s")
                time.sleep(min(retry_after, 120))
                continue
            r.raise_for_status()
            data = r.json()
            return [S2Paper(p) for p in data.get("data", [])]
        except httpx.HTTPStatusError as e:
            last_error = e
            if r.status_code == 429:
                continue
            if attempt < _MAX_RETRIES - 1:
                time.sleep(_RETRY_DELAY * (attempt + 1))
            continue
        except httpx.HTTPError as e:
            last_error = e
            if attempt < _MAX_RETRIES - 1:
                time.sleep(_RETRY_DELAY * (attempt + 1))
            continue

    raise RuntimeError(
        f"Semantic Scholar search failed for '{query}' after {_MAX_RETRIES} attempts: {last_error}"
    )


def get_paper_by_id(paper_id: str, fields: Optional[List[str]] = None) -> Optional["S2Paper"]:
    """Fetch a paper by Semantic Scholar ID."""
    if fields is None:
        fields = [
            "title",
            "authors",
            "abstract",
            "year",
            "venue",
            "citationCount",
            "openAccessPdf",
            "paperId",
            "externalIds",
        ]

    url = f"{_S2_API}/paper/{paper_id}?fields={','.join(fields)}"
    try:
        client = _get_client()
        r = client.get(url)
        r.raise_for_status()
        return S2Paper(r.json())
    except Exception as e:
        logger.warning(f"Failed to fetch paper {paper_id}: {e}")
        return None


def get_citations(paper_id: str, limit: int = 100) -> List["S2Paper"]:
    """Fetch papers that cite the given paper."""
    url = (
        f"{_S2_API}/paper/{paper_id}/citations?"
        f"fields=title,authors,abstract,year,venue,citationCount,openAccessPdf,paperId&"
        f"limit={limit}"
    )
    try:
        client = _get_client()
        r = client.get(url)
        r.raise_for_status()
        data = r.json()
        return [S2Paper(c["citingPaper"]) for c in data.get("data", [])]
    except Exception as e:
        logger.warning(f"Failed to fetch citations for {paper_id}: {e}")
        return []


def get_references(paper_id: str, limit: int = 100) -> List["S2Paper"]:
    """Fetch papers referenced by the given paper."""
    url = (
        f"{_S2_API}/paper/{paper_id}/references?"
        f"fields=title,authors,abstract,year,venue,citationCount,openAccessPdf,paperId&"
        f"limit={limit}"
    )
    try:
        client = _get_client()
        r = client.get(url)
        r.raise_for_status()
        data = r.json()
        return [S2Paper(r["referencedPaper"]) for r in data.get("data", [])]
    except Exception as e:
        logger.warning(f"Failed to fetch references for {paper_id}: {e}")
        return []


def _urlencode(q: str) -> str:
    import urllib.parse

    return urllib.parse.quote_plus(q)


class S2Paper:
    """Wrapper for Semantic Scholar paper data."""

    __slots__ = ("_d",)

    def __init__(self, d: Dict[str, Any]):
        self._d = d

    @property
    def paper_id(self) -> str:
        return self._d.get("paperId", "")  # type: ignore[no-any-return]

    @property
    def title(self) -> str:
        return self._d.get("title", "") or ""

    @property
    def abstract(self) -> str:
        return self._d.get("abstract", "") or ""

    @property
    def year(self) -> Optional[int]:
        return self._d.get("year")

    @property
    def venue(self) -> str:
        return self._d.get("venue", "") or ""

    @property
    def citation_count(self) -> int:
        return self._d.get("citationCount", 0) or 0

    @property
    def authors(self) -> List[str]:
        return [a.get("name", "") for a in self._d.get("authors", []) or [] if a.get("name")]

    @property
    def open_access_pdf(self) -> Optional[str]:
        pdf = self._d.get("openAccessPdf")
        if pdf:
            return pdf.get("url") or pdf.get("status")  # type: ignore[no-any-return]
        return None

    @property
    def external_ids(self) -> Dict[str, str]:
        return self._d.get("externalIds", {}) or {}

    @property
    def arxiv_id(self) -> Optional[str]:
        ext = self.external_ids
        return ext.get("ArXiv") or ext.get("arXiv")

    def to_paper(self) -> Paper:
        """Convert to core.Paper."""
        from core import Paper as CorePaper

        ext = self.external_ids
        uid = self.arxiv_id or self.paper_id
        pdf_url = self.open_access_pdf or ""
        if not pdf_url and self.arxiv_id:
            pdf_url = f"https://arxiv.org/pdf/{self.arxiv_id}.pdf"

        return CorePaper(
            source="semantic_scholar",
            uid=uid,
            title=self.title,
            authors=self.authors,
            abstract=self.abstract,
            published=str(self.year) if self.year else "",
            updated="",
            abs_url=f"https://www.semanticscholar.org/paper/{self.paper_id}",
            pdf_url=pdf_url,
            primary_category=self.venue,
            categories="",
            comment="",
            journal_ref="",
            reference_count=self.citation_count,
            doi=ext.get("DOI", ""),
        )
