"""HTTP response cache for arXiv and Crossref API calls."""
import json
import os
import time
from pathlib import Path
from typing import Optional

_CACHE_DIR = Path(os.getenv("AIROS_CACHE_DIR", str(Path.home() / ".cache" / "ai_research_os")))
_ARXIV_CACHE_DIR = _CACHE_DIR / "arxiv"
_CROSSREF_CACHE_DIR = _CACHE_DIR / "crossref"
_TTL_SECONDS: int = int(os.getenv("AIROS_CACHE_TTL_SECONDS", str(24 * 3600)))
_MAX_CACHE_FILES: int = int(os.getenv("AIROS_MAX_CACHE_FILES", "2000"))  # max files per cache dir


def _cache_dir(source: str) -> Path:
    d = _CACHE_DIR / source
    d.mkdir(parents=True, exist_ok=True)
    return d


def _cache_path(source: str, key: str) -> Path:
    safe = key.replace("/", "%2F").replace(":", "%3A")
    return _cache_dir(source) / f"{safe}.json"


def _evict_if_needed(source: str) -> None:
    """Evict oldest files if cache exceeds MAX_CACHE_FILES (simple LRU-ish eviction)."""
    cache_path = _cache_dir(source)
    try:
        files = sorted(cache_path.glob("*.json"), key=lambda f: f.stat().st_mtime)
    except OSError:
        return
    if len(files) > _MAX_CACHE_FILES:
        # Remove oldest 10%
        evict_count = max(1, len(files) // 10)
        for f in files[:evict_count]:
            try:
                f.unlink()
            except OSError:
                pass


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
    _evict_if_needed(source)
    p = _cache_path(source, key)
    try:
        p.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")
    except OSError:
        pass  # disk full or permission issue — non-fatal
