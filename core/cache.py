"""HTTP response cache for arXiv and Crossref API calls."""
import json
import time
from pathlib import Path
from typing import Any, Dict, Optional, Tuple

from config import CACHE_DIR, CACHE_TTL_SECONDS, MAX_CACHE_FILES, MEMORY_CACHE_MAX_SIZE

_CACHE_DIR = Path(CACHE_DIR)
_ARXIV_CACHE_DIR = _CACHE_DIR / "arxiv"
_CROSSREF_CACHE_DIR = _CACHE_DIR / "crossref"
_TTL_SECONDS: int = CACHE_TTL_SECONDS
_MAX_CACHE_FILES: int = MAX_CACHE_FILES  # max files per cache dir

# In-memory cache layer to reduce disk I/O
_MEMORY_CACHE: Dict[Tuple[str, str], Tuple[float, dict]] = {}  # (source, key) -> (timestamp, data)
_MEMORY_CACHE_MAX_SIZE = MEMORY_CACHE_MAX_SIZE  # Maximum number of items in memory cache


def _cache_dir(source: str) -> Path:
    d = _CACHE_DIR / source
    d.mkdir(parents=True, exist_ok=True)
    return d


def _cache_path(source: str, key: str) -> Path:
    safe = key.replace("/", "%2F").replace(":", "%3A").replace("?", "%3F").replace("#", "%23")
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


def _evict_memory_cache_if_needed() -> None:
    """Evict oldest items from memory cache if it exceeds the limit."""
    if len(_MEMORY_CACHE) > _MEMORY_CACHE_MAX_SIZE:
        # Sort items by timestamp and remove oldest 20%
        sorted_items = sorted(_MEMORY_CACHE.items(), key=lambda x: x[1][0])
        evict_count = max(1, len(_MEMORY_CACHE) // 5)
        for item in sorted_items[:evict_count]:
            del _MEMORY_CACHE[item[0]]


def get_cached(source: str, key: str) -> Optional[dict]:
    """Get cached data, checking memory cache first, then disk."""
    # Check memory cache first
    cache_key = (source, key)
    if cache_key in _MEMORY_CACHE:
        timestamp, data = _MEMORY_CACHE[cache_key]
        if time.time() - timestamp < _TTL_SECONDS:
            return data
        else:
            # Remove expired item from memory cache
            del _MEMORY_CACHE[cache_key]

    # Check disk cache
    p = _cache_path(source, key)
    if not p.exists():
        return None
    try:
        mtime = p.stat().st_mtime
        if time.time() - mtime > _TTL_SECONDS:
            return None
        data = json.loads(p.read_text(encoding="utf-8"))
        # Add to memory cache
        _MEMORY_CACHE[cache_key] = (time.time(), data)
        _evict_memory_cache_if_needed()
        return data
    except (OSError, json.JSONDecodeError):
        return None


def set_cached(source: str, key: str, data: dict) -> None:
    """Set cached data, updating both memory and disk caches."""
    # Update memory cache
    cache_key = (source, key)
    _MEMORY_CACHE[cache_key] = (time.time(), data)
    _evict_memory_cache_if_needed()

    # Update disk cache
    _evict_if_needed(source)
    p = _cache_path(source, key)
    try:
        p.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")
    except OSError:
        pass  # disk full or permission issue — non-fatal

def clear_cache(source: Optional[str] = None) -> None:
    """Clear cache for a specific source or all sources."""

    # Clear memory cache
    if source:
        # Remove only items for the specified source
        keys_to_remove = [k for k in _MEMORY_CACHE if k[0] == source]
        for key in keys_to_remove:
            del _MEMORY_CACHE[key]
    else:
        # Clear all memory cache
        _MEMORY_CACHE.clear()

    # Clear disk cache
    if source:
        cache_dir = _cache_dir(source)
        try:
            for f in cache_dir.glob("*.json"):
                f.unlink()
        except OSError:
            pass
    else:
        try:
            for subdir in _CACHE_DIR.iterdir():
                if subdir.is_dir():
                    for f in subdir.glob("*.json"):
                        f.unlink()
        except OSError:
            pass

def get_cache_stats() -> Dict[str, Any]:
    """Get cache statistics."""
    stats = {
        "memory_cache_size": len(_MEMORY_CACHE),
        "memory_cache_max_size": _MEMORY_CACHE_MAX_SIZE,
        "disk_cache_dir": str(_CACHE_DIR),
        "ttl_seconds": _TTL_SECONDS,
        "max_cache_files": _MAX_CACHE_FILES,
        "disk_cache_sizes": {}
    }

    # Calculate disk cache sizes
    try:
        for subdir in _CACHE_DIR.iterdir():
            if subdir.is_dir():
                files = list(subdir.glob("*.json"))
                stats["disk_cache_sizes"][subdir.name] = len(files)
    except OSError:
        pass

    return stats
