"""Input normalization utilities."""
import re
from typing import Optional


def is_probably_doi(s: str) -> bool:
    s = s.strip()
    return bool(re.search(r"(https?://(dx\.)?doi\.org/)?10\.\d{4,9}/\S+", s, flags=re.I))


def normalize_doi(s: str) -> Optional[str]:
    if not s:
        return None
    s = s.strip()
    s = re.sub(r"^https?://(dx\.)?doi\.org/", "", s, flags=re.I)
    return s.strip().rstrip(".")


def normalize_arxiv_id(s: str) -> Optional[str]:
    if not s:
        return None
    s = s.strip()

    # arXiv URL formats
    m = re.search(r"(?:arxiv\.org/(?:abs|pdf)/)(\d{4}\.\d{4,5})(v\d+)?", s, flags=re.I)
    if m:
        return (m.group(1) + (m.group(2) or "")).strip()

    # New-style id directly
    if re.fullmatch(r"\d{4}\.\d{4,5}(v\d+)?", s):
        return s

    # Old-style id
    if re.fullmatch(r"[a-zA-Z\-]+/\d{7}(v\d+)?", s):
        return s

    # arXiv DOI format
    m2 = re.search(r"10\.48550/arXiv\.(\d{4}\.\d{4,5})(v\d+)?", s, flags=re.I)
    if m2:
        return (m2.group(1) + (m2.group(2) or "")).strip()

    return None
