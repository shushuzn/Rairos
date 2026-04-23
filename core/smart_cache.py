"""
Smart Cache Manager with compression, prioritization, and cost optimization.

Inspired by cloud optimization principles:
- Reduce remote API calls (minimize costs)
- Optimize disk I/O (SSD-aware caching)
- Intelligent cache expiration
- Cache prioritization (keep valuable data longer)
"""
import json
import zlib
import time
import logging
from pathlib import Path
from typing import Any, Dict, Optional
from dataclasses import dataclass
from collections import OrderedDict

logger = logging.getLogger(__name__)


@dataclass
class CacheEntry:
    """Cache entry with metadata."""
    key: str
    data: bytes
    created_at: float
    accessed_at: float
    access_count: int
    size_bytes: int
    priority: int  # Higher = keep longer
    compressed: bool
    ttl: Optional[int] = None  # Time-to-live in seconds (None = use default)


class SmartCache:
    """
    Intelligent cache manager with compression and prioritization.
    
    Features:
    - LRU + priority-based eviction
    - Automatic compression for large entries
    - Size limits and quotas
    - Cache statistics and metrics
    - TTL-based expiration
    """
    
    def __init__(
        self,
        cache_dir: Path,
        max_size_mb: float = 500.0,
        compression_threshold_kb: float = 10.0,
        default_ttl: int = 86400,  # 24 hours
        compression_level: int = 6
    ):
        self.cache_dir = cache_dir
        self.max_size_bytes = int(max_size_mb * 1024 * 1024)
        self.compression_threshold_bytes = int(compression_threshold_kb * 1024)
        self.default_ttl = default_ttl
        self.compression_level = compression_level
        
        # In-memory index for fast access
        self._index: OrderedDict[str, CacheEntry] = OrderedDict()
        
        # Statistics
        self._stats = {
            "hits": 0,
            "misses": 0,
            "evictions": 0,
            "compressions": 0,
            "decompressions": 0,
            "bytes_saved": 0,
            "total_writes": 0,
        }
        
        # Load existing index
        self._load_index()
    
    def _load_index(self):
        """Load cache index from disk."""
        index_file = self.cache_dir / ".cache_index.json"
        if index_file.exists():
            try:
                with open(index_file, 'r', encoding='utf-8') as f:
                    data = json.load(f)
                    
                for key, meta in data.items():
                    # Reconstruct minimal CacheEntry
                    entry = CacheEntry(
                        key=key,
                        data=b"",  # Don't load data in memory
                        created_at=meta["created_at"],
                        accessed_at=meta["accessed_at"],
                        access_count=meta["access_count"],
                        size_bytes=meta["size_bytes"],
                        priority=meta["priority"],
                        compressed=meta["compressed"],
                        ttl=meta.get("ttl")
                    )
                    self._index[key] = entry
                    
                logger.info(f"Loaded {len(self._index)} cache entries from index")
            except Exception as e:
                logger.warning(f"Failed to load cache index: {e}")
    
    def _save_index(self):
        """Save cache index to disk."""
        index_file = self.cache_dir / ".cache_index.json"
        try:
            # Only save metadata, not actual data
            data = {
                key: {
                    "created_at": entry.created_at,
                    "accessed_at": entry.accessed_at,
                    "access_count": entry.access_count,
                    "size_bytes": entry.size_bytes,
                    "priority": entry.priority,
                    "compressed": entry.compressed,
                    "ttl": entry.ttl
                }
                for key, entry in self._index.items()
            }
            
            with open(index_file, 'w', encoding='utf-8') as f:
                json.dump(data, f)
        except Exception as e:
            logger.warning(f"Failed to save cache index: {e}")
    
    def _compress(self, data: bytes) -> bytes:
        """Compress data using zlib."""
        return zlib.compress(data, self.compression_level)
    
    def _decompress(self, data: bytes) -> bytes:
        """Decompress data."""
        return zlib.decompress(data)
    
    def _get_cache_path(self, key: str) -> Path:
        """Get cache file path for a key."""
        # Use first 2 chars for subdirectory to avoid too many files in one dir
        subdir = key[:2]
        cache_file = self.cache_dir / subdir / f"{key}.cache"
        return cache_file
    
    def _evict_if_needed(self):
        """Evict cache entries if size limit is exceeded."""
        while self._get_total_size() > self.max_size_bytes and self._index:
            # Find entry to evict (LRU + lowest priority)
            evict_key = None
            evict_score = float('inf')
            
            for key, entry in self._index.items():
                # Score = accessed_at (older = higher priority for eviction)
                # But priority reduces eviction likelihood
                score = entry.accessed_at - (entry.priority * 1000)
                if score < evict_score:
                    evict_score = score
                    evict_key = key
            
            if evict_key:
                self._remove_entry(evict_key)
                self._stats["evictions"] += 1
    
    def _get_total_size(self) -> int:
        """Get total size of all cache entries."""
        return sum(entry.size_bytes for entry in self._index.values())
    
    def _remove_entry(self, key: str):
        """Remove a cache entry."""
        if key not in self._index:
            return
        
        # Remove file from disk
        cache_path = self._get_cache_path(key)
        if cache_path.exists():
            try:
                cache_path.unlink()
                # Try to remove empty subdirectory
                subdir = cache_path.parent
                if subdir != self.cache_dir and not any(subdir.iterdir()):
                    subdir.rmdir()
            except Exception as e:
                logger.warning(f"Failed to remove cache file: {e}")
        
        # Remove from index
        del self._index[key]
    
    def set(
        self,
        key: str,
        data: Any,
        ttl: Optional[int] = None,
        priority: int = 0
    ):
        """
        Store data in cache.
        
        Args:
            key: Cache key
            data: Data to cache (will be JSON serialized)
            ttl: Time-to-live in seconds (None = use default)
            priority: Cache priority (higher = keep longer)
        """
        # Serialize data
        serialized = json.dumps(data, ensure_ascii=False).encode('utf-8')
        
        # Check if compression is beneficial
        compressed = len(serialized) >= self.compression_threshold_bytes
        if compressed:
            serialized = self._compress(serialized)
            self._stats["compressions"] += 1
            self._stats["bytes_saved"] += len(serialized) - len(serialized)
        
        # Create cache entry
        now = time.time()
        entry = CacheEntry(
            key=key,
            data=serialized,
            created_at=now,
            accessed_at=now,
            access_count=1,
            size_bytes=len(serialized),
            priority=priority,
            compressed=compressed,
            ttl=ttl
        )
        
        # Ensure cache directory exists
        cache_path = self._get_cache_path(key)
        cache_path.parent.mkdir(parents=True, exist_ok=True)
        
        # Write to disk
        try:
            with open(cache_path, 'wb') as f:
                f.write(serialized)
            
            # Update index
            self._index[key] = entry
            
            # Evict if needed
            self._evict_if_needed()
            
            # Periodically save index
            if self._stats["total_writes"] % 100 == 0:
                self._save_index()
            
            self._stats["total_writes"] += 1
            
        except Exception as e:
            logger.error(f"Failed to write cache: {e}")
    
    def get(self, key: str) -> Optional[Any]:
        """
        Retrieve data from cache.
        
        Returns None if key doesn't exist or is expired.
        """
        if key not in self._index:
            self._stats["misses"] += 1
            return None
        
        entry = self._index[key]
        
        # Check TTL
        ttl = entry.ttl if entry.ttl is not None else self.default_ttl
        if ttl and (time.time() - entry.created_at) > ttl:
            self._remove_entry(key)
            self._stats["misses"] += 1
            return None
        
        # Read from disk
        cache_path = self._get_cache_path(key)
        if not cache_path.exists():
            self._remove_entry(key)
            self._stats["misses"] += 1
            return None
        
        try:
            with open(cache_path, 'rb') as f:
                data = f.read()
            
            # Decompress if needed
            if entry.compressed:
                data = self._decompress(data)
                self._stats["decompressions"] += 1
            
            # Deserialize
            result = json.loads(data.decode('utf-8'))
            
            # Update access statistics (move to end for LRU)
            entry.accessed_at = time.time()
            entry.access_count += 1
            self._index.move_to_end(key)
            
            self._stats["hits"] += 1
            return result
            
        except Exception as e:
            logger.warning(f"Failed to read cache: {e}")
            self._remove_entry(key)
            self._stats["misses"] += 1
            return None
    
    def delete(self, key: str):
        """Delete a cache entry."""
        self._remove_entry(key)
    
    def clear(self):
        """Clear all cache entries."""
        for key in list(self._index.keys()):
            self._remove_entry(key)
        self._index.clear()
    
    def get_stats(self) -> Dict[str, Any]:
        """Get cache statistics."""
        total_size = self._get_total_size()
        total_entries = len(self._index)
        
        hit_rate = 0.0
        total_requests = self._stats["hits"] + self._stats["misses"]
        if total_requests > 0:
            hit_rate = (self._stats["hits"] / total_requests) * 100
        
        return {
            "total_entries": total_entries,
            "total_size_mb": total_size / (1024 * 1024),
            "max_size_mb": self.max_size_bytes / (1024 * 1024),
            "usage_percent": (total_size / self.max_size_bytes) * 100 if self.max_size_bytes > 0 else 0,
            "hits": self._stats["hits"],
            "misses": self._stats["misses"],
            "hit_rate_percent": hit_rate,
            "evictions": self._stats["evictions"],
            "compressions": self._stats["compressions"],
            "decompressions": self._stats["decompressions"],
            "bytes_saved": self._stats["bytes_saved"],
            "total_writes": self._stats["total_writes"],
        }
    
    def cleanup_expired(self) -> int:
        """Remove all expired cache entries. Returns count of removed entries."""
        now = time.time()
        removed = 0
        
        keys_to_remove = []
        for key, entry in self._index.items():
            ttl = entry.ttl if entry.ttl is not None else self.default_ttl
            if ttl and (now - entry.created_at) > ttl:
                keys_to_remove.append(key)
        
        for key in keys_to_remove:
            self._remove_entry(key)
            removed += 1
        
        if removed > 0:
            logger.info(f"Cleaned up {removed} expired cache entries")
            self._save_index()
        
        return removed


# Global smart cache instance
_smart_cache: Optional[SmartCache] = None


def get_smart_cache(
    cache_dir: Optional[Path] = None,
    max_size_mb: float = 500.0
) -> SmartCache:
    """Get or create the global smart cache."""
    global _smart_cache
    if _smart_cache is None:
        cache_dir = cache_dir or Path.home() / ".cache" / "ai_research_os"
        _smart_cache = SmartCache(cache_dir, max_size_mb=max_size_mb)
    return _smart_cache
