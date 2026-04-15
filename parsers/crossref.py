"""Crossref API metadata fetching."""
import datetime as dt
import re
from typing import List, Optional, Tuple

import requests

from core import CROSSREF_WORKS, DOI_RESOLVER, Paper
from parsers.input_detection import normalize_arxiv_id


def _best_effort_date_from_crossref(item: dict) -> str:
    for key in ["published-print", "published-online", "published", "issued", "created", "deposited"]:
        obj = item.get(key)
        if not obj:
            continue
        dp = obj.get("date-parts")
        if isinstance(dp, list) and dp and isinstance(dp[0], list) and dp[0]:
            parts = dp[0]
            y = parts[0]
            m = parts[1] if len(parts) > 1 else 1
            d = parts[2] if len(parts) > 2 else 1
            try:
                return dt.date(int(y), int(m), int(d)).isoformat()
            except Exception:
                continue
    return ""


def _authors_from_crossref(item: dict) -> List[str]:
    out = []
    for a in item.get("author", []) or []:
        given = (a.get("given") or "").strip()
        family = (a.get("family") or "").strip()
        name = (given + " " + family).strip()
        if name:
            out.append(name)
    return out


def _title_from_crossref(item: dict) -> str:
    t = item.get("title") or []
    if isinstance(t, list) and t:
        return str(t[0]).strip()
    if isinstance(t, str):
        return t.strip()
    return ""


def _abstract_from_crossref(item: dict) -> str:
    ab = item.get("abstract") or ""
    if not ab:
        return ""
    ab = re.sub(r"<[^>]+>", "", ab)
    ab = re.sub(r"\s+", " ", ab).strip()
    return ab


def _try_find_arxiv_id_in_crossref(item: dict, doi: str) -> Optional[str]:
    arx = normalize_arxiv_id(doi)
    if arx:
        return arx

    rel = item.get("relation") or {}
    blob = str(rel)
    m = re.search(r"arxiv\.org/(?:abs|pdf)/(\d{4}\.\d{4,5})(v\d+)?", blob, flags=re.I)
    if m:
        return (m.group(1) + (m.group(2) or "")).strip()

    for k in ["alternative-id", "archive", "URL", "link"]:
        v = item.get(k)
        if not v:
            continue
        blob2 = str(v)
        m2 = re.search(r"arxiv\.org/(?:abs|pdf)/(\d{4}\.\d{4,5})(v\d+)?", blob2, flags=re.I)
        if m2:
            return (m2.group(1) + (m2.group(2) or "")).strip()

        m3 = re.search(r"(\d{4}\.\d{4,5})(v\d+)?", blob2)
        if m3 and "arxiv" in blob2.lower():
            return (m3.group(1) + (m3.group(2) or "")).strip()

    return None


def fetch_crossref_metadata(doi: str, timeout: int = 30) -> Tuple[Paper, Optional[str]]:
    """
    Crossref failures (404/network) will NOT crash.
    Returns (Paper, maybe_arxiv_id).
    """
    url = CROSSREF_WORKS.format(doi=doi)

    try:
        r = requests.get(
            url,
            timeout=timeout,
            headers={"User-Agent": "AI-Research-OS/1.0"},
        )
        if r.status_code == 404:
            raise ValueError("Crossref 404 (DOI not found in Crossref)")
        r.raise_for_status()
        data = r.json()
        item = data.get("message") or {}

        title = _title_from_crossref(item) or doi
        authors = _authors_from_crossref(item)
        abstract = _abstract_from_crossref(item)
        published = _best_effort_date_from_crossref(item)

        abs_url = (item.get("URL") or "").strip() or (DOI_RESOLVER + doi)

        pdf_url = ""
        for l in item.get("link", []) or []:
            if (l.get("content-type") or "").lower() == "application/pdf" and l.get("URL"):
                pdf_url = l["URL"].strip()
                break

        maybe_arxiv = _try_find_arxiv_id_in_crossref(item, doi)

        # Journal info
        journal = (item.get("container-title") or [""])[0] or ""
        volume = item.get("volume") or ""
        issue = item.get("issue") or ""
        page = item.get("page") or ""
        ref_count = item.get("is-referenced-by-count") or 0

        p = Paper(
            source="doi",
            uid=doi,
            title=title,
            authors=authors,
            abstract=abstract,
            published=published or "",
            updated="",
            abs_url=abs_url,
            pdf_url=pdf_url,
            primary_category="",
            journal=journal,
            volume=volume,
            issue=issue,
            page=page,
            reference_count=ref_count,
        )
        return p, maybe_arxiv

    except Exception:
        # Graceful downgrade: DOI-only metadata (minimal), still try to parse arXiv id from DOI string.
        maybe_arxiv = normalize_arxiv_id(doi)
        p = Paper(
            source="doi",
            uid=doi,
            title=doi,
            authors=[],
            abstract="",
            published="",
            updated="",
            abs_url=DOI_RESOLVER + doi,
            pdf_url="",
            primary_category="",
        )
        return p, maybe_arxiv
