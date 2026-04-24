"""Tests for smart cache functionality."""
import pytest
import tempfile
from pathlib import Path
from core.smart_cache import SmartCache, get_smart_cache


@pytest.fixture
def temp_cache_dir():
    """Create a temporary cache directory."""
    with tempfile.TemporaryDirectory() as tmpdir:
        yield Path(tmpdir)


def test_smart_cache_set_get(temp_cache_dir):
    """Test basic set and get operations."""
    cache = SmartCache(temp_cache_dir, max_size_mb=10.0)

    cache.set("key1", {"data": "value1"})
    result = cache.get("key1")

    assert result is not None
    assert result["data"] == "value1"


def test_smart_cache_miss(temp_cache_dir):
    """Test cache miss."""
    cache = SmartCache(temp_cache_dir, max_size_mb=10.0)

    result = cache.get("nonexistent")
    assert result is None


def test_smart_cache_delete(temp_cache_dir):
    """Test deleting a cache entry."""
    cache = SmartCache(temp_cache_dir, max_size_mb=10.0)

    cache.set("key1", {"data": "value1"})
    cache.delete("key1")
    result = cache.get("key1")

    assert result is None


def test_smart_cache_clear(temp_cache_dir):
    """Test clearing all cache entries."""
    cache = SmartCache(temp_cache_dir, max_size_mb=10.0)

    cache.set("key1", {"data": "value1"})
    cache.set("key2", {"data": "value2"})
    cache.clear()

    assert cache.get("key1") is None
    assert cache.get("key2") is None


def test_smart_cache_stats(temp_cache_dir):
    """Test cache statistics."""
    cache = SmartCache(temp_cache_dir, max_size_mb=10.0)

    cache.set("key1", {"data": "value1"})
    cache.get("key1")  # hit
    cache.get("key2")  # miss

    stats = cache.get_stats()

    assert stats["total_entries"] == 1
    assert stats["hits"] == 1
    assert stats["misses"] == 1
    assert stats["hit_rate_percent"] > 0


def test_smart_cache_priority(temp_cache_dir):
    """Test cache priority."""
    cache = SmartCache(temp_cache_dir, max_size_mb=1.0)  # Small size to trigger eviction

    # Set multiple entries with different priorities
    cache.set("low", {"data": "low"}, priority=0)
    cache.set("high", {"data": "high"}, priority=10)

    # Both should exist
    assert cache.get("low") is not None
    assert cache.get("high") is not None


def test_smart_cache_compression(temp_cache_dir):
    """Test cache compression for large data."""
    cache = SmartCache(
        temp_cache_dir,
        max_size_mb=10.0,
        compression_threshold_kb=0.1  # Very low threshold for testing
    )

    # Create large data
    large_data = {"data": "x" * 10000}

    cache.set("large", large_data)
    result = cache.get("large")

    assert result is not None
    assert result["data"] == "x" * 10000


def test_smart_cache_eviction(temp_cache_dir):
    """Test cache eviction when size limit is reached."""
    cache = SmartCache(temp_cache_dir, max_size_mb=0.001)  # Very small limit

    # Set many entries
    for i in range(100):
        cache.set(f"key{i}", {"data": f"value{i}" * 10})

    # Some entries should have been evicted
    stats = cache.get_stats()
    assert stats["evictions"] > 0


def test_smart_cache_ttl(temp_cache_dir):
    """Test cache TTL functionality."""
    cache = SmartCache(temp_cache_dir, max_size_mb=10.0, default_ttl=3600)

    cache.set("key1", {"data": "value1"}, ttl=3600)

    # Should exist immediately
    assert cache.get("key1") is not None

    # Test that TTL is stored correctly
    stats = cache.get_stats()
    assert stats["total_entries"] == 1


def test_smart_cache_cleanup_expired(temp_cache_dir):
    """Test cleanup of expired entries."""
    cache = SmartCache(temp_cache_dir, max_size_mb=10.0, default_ttl=3600)

    cache.set("key1", {"data": "value1"}, ttl=3600)
    cache.set("key2", {"data": "value2"}, ttl=3600)

    # Both should exist
    assert cache.get("key1") is not None
    assert cache.get("key2") is not None

    # Test that cleanup can be called
    removed = cache.cleanup_expired()
    assert removed >= 0


def test_smart_cache_multiple_types(temp_cache_dir):
    """Test caching different data types."""
    cache = SmartCache(temp_cache_dir, max_size_mb=10.0)

    cache.set("string", "test string")
    cache.set("int", 42)
    cache.set("float", 3.14)
    cache.set("bool", True)
    cache.set("list", [1, 2, 3])
    cache.set("nested", {"a": {"b": {"c": 1}}})

    assert cache.get("string") == "test string"
    assert cache.get("int") == 42
    assert cache.get("float") == 3.14
    assert cache.get("bool") is True
    assert cache.get("list") == [1, 2, 3]
    assert cache.get("nested") == {"a": {"b": {"c": 1}}}


def test_get_smart_cache_global():
    """Test getting global smart cache instance."""
    cache1 = get_smart_cache()
    cache2 = get_smart_cache()

    # Should return the same instance
    assert cache1 is cache2
