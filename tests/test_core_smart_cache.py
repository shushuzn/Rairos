"""Tests for core/smart_cache.py — CacheEntry dataclass and basic operations."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

from core.smart_cache import CacheEntry


class TestCacheEntry:
    def test_creation(self):
        entry = CacheEntry(
            key="test-key",
            data=b"hello world",
            created_at=1000.0,
            accessed_at=1001.0,
            access_count=1,
            size_bytes=11,
            priority=5,
            compressed=False,
        )
        assert entry.key == "test-key"
        assert entry.data == b"hello world"
        assert entry.created_at == 1000.0
        assert entry.accessed_at == 1001.0
        assert entry.access_count == 1
        assert entry.size_bytes == 11
        assert entry.priority == 5
        assert entry.compressed is False
        assert entry.ttl is None

    def test_with_ttl(self):
        entry = CacheEntry(
            key="x",
            data=b"data",
            created_at=1.0,
            accessed_at=1.0,
            access_count=0,
            size_bytes=4,
            priority=0,
            compressed=False,
            ttl=3600,
        )
        assert entry.ttl == 3600

    def test_fields_are_correct_types(self):
        entry = CacheEntry(
            key="k",
            data=b"d",
            created_at=1.0,
            accessed_at=1.0,
            access_count=0,
            size_bytes=1,
            priority=0,
            compressed=False,
        )
        assert isinstance(entry.key, str)
        assert isinstance(entry.data, bytes)
        assert isinstance(entry.created_at, float)
        assert isinstance(entry.accessed_at, float)
        assert isinstance(entry.access_count, int)
        assert isinstance(entry.size_bytes, int)
        assert isinstance(entry.priority, int)
        assert isinstance(entry.compressed, bool)
        # ttl can be None or int
        assert entry.ttl is None or isinstance(entry.ttl, int)

    def test_priority_range(self):
        e1 = CacheEntry(
            key="k", data=b"d", created_at=1.0, accessed_at=1.0,
            access_count=0, size_bytes=1, priority=0, compressed=False,
        )
        assert e1.priority == 0
        e2 = CacheEntry(
            key="k", data=b"d", created_at=1.0, accessed_at=1.0,
            access_count=0, size_bytes=1, priority=100, compressed=False,
        )
        assert e2.priority == 100

    def test_access_count(self):
        entry = CacheEntry(
            key="k", data=b"d", created_at=1.0, accessed_at=1.0,
            access_count=99, size_bytes=1, priority=0, compressed=False,
        )
        assert entry.access_count == 99

    def test_size_bytes(self):
        large_data = b"x" * 10000
        entry = CacheEntry(
            key="k",
            data=large_data,
            created_at=1.0,
            accessed_at=1.0,
            access_count=0,
            size_bytes=10000,
            priority=0,
            compressed=False,
        )
        assert entry.size_bytes == 10000
