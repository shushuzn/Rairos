"""HTTP response cache for arXiv and Crossref API calls."""
import json
import os
import time
from pathlib import Path
from typing import Optional

_CACHE_DIR = Path.home() / ".cache" / "ai_research_os"
_ARXIV_CACHE_DIR = _CACHE_DIR / "arxiv"
_CROSSREF_CACHE_DIR = _CACHE_DIR / "crossref"
_TTL_SECONDS = 24 * 3600


def _cache_dir(source: str) -> Path:
    d = _CACHE_DIR / source
    d.mkdir(parents=True, exist_ok=True)
    return d


def _cache_path(source: str, key: str) -> Path:
    safe = key.replace("/", "%2F").replace(":", "%3A")
    return _cache_dir(source) / f"{safe}.json"


def get_cached(source: str, key: str) -> Optional[dict]:
    p = _cache_path(source, key)
    if not p.exists():
        return None
    try:
        mtime = p.stat().st_mtime
        if time.time() - mtime > _TTL_SECONDS:
            return None
        return json.loads(p.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None


def set_cached(source: str, key: str, data: dict) -> None:
    p = _cache_path(source, key)
    try:
        p.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")
    except OSError:
        pass  # disk full or permission issue — non-fatal
