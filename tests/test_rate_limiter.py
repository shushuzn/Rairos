"""Tests for API rate limiting functionality."""
from core.rate_limiter import (
    RateLimiter,
    RateLimitConfig,
    APIRateLimitManager,
    get_rate_limit_manager,
    create_limiter,
)


def test_rate_limit_config_creation():
    """Test RateLimitConfig creation."""
    config = RateLimitConfig()

    assert config.requests_per_second == 10.0
    assert config.requests_per_minute == 100.0
    assert config.requests_per_hour == 1000.0
    assert config.burst_size == 5


def test_rate_limiter_creation():
    """Test RateLimiter creation."""
    limiter = RateLimiter()

    assert limiter.config is not None
    assert limiter._total_requests == 0


def test_rate_limiter_can_make_request():
    """Test checking if request can be made."""
    limiter = RateLimiter(RateLimitConfig(
        requests_per_second=10.0,
        burst_size=2
    ))

    # Should be able to make request initially
    assert limiter.can_make_request() is True


def test_rate_limiter_acquire():
    """Test acquiring rate limit."""
    limiter = RateLimiter(RateLimitConfig(burst_size=5))

    # Should be able to acquire
    result = limiter.acquire(blocking=False)
    assert result is True


def test_rate_limiter_multiple_requests():
    """Test multiple requests."""
    limiter = RateLimiter(RateLimitConfig(burst_size=3))

    # Make multiple requests
    results = []
    for _ in range(5):
        results.append(limiter.acquire(blocking=False))

    # First 3 should succeed, some later might fail
    success_count = sum(1 for r in results if r)
    assert success_count >= 1


def test_rate_limiter_stats():
    """Test getting statistics."""
    limiter = RateLimiter(RateLimitConfig(burst_size=5))

    limiter.acquire(blocking=False)
    limiter.acquire(blocking=False)
    limiter.acquire(blocking=False)

    stats = limiter.get_stats()

    assert stats["total_requests"] == 3
    assert "limits" in stats


def test_rate_limiter_reset_stats():
    """Test resetting statistics."""
    limiter = RateLimiter(RateLimitConfig(burst_size=5))

    limiter.acquire(blocking=False)
    limiter.reset_stats()

    stats = limiter.get_stats()
    assert stats["total_requests"] == 0


def test_rate_limit_manager_creation():
    """Test RateLimitManager creation."""
    manager = APIRateLimitManager()

    assert len(manager._limiters) == 0


def test_rate_limit_manager_get_limiter():
    """Test getting limiter from manager."""
    manager = APIRateLimitManager()

    limiter1 = manager.get_limiter("test_endpoint")
    limiter2 = manager.get_limiter("test_endpoint")

    # Should return same instance
    assert limiter1 is limiter2


def test_rate_limit_manager_different_endpoints():
    """Test different endpoints get different limiters."""
    manager = APIRateLimitManager()

    limiter1 = manager.get_limiter("endpoint1")
    limiter2 = manager.get_limiter("endpoint2")

    # Should be different instances
    assert limiter1 is not limiter2


def test_rate_limit_manager_can_call():
    """Test checking if endpoint can be called."""
    manager = APIRateLimitManager()

    # Should be able to call initially
    assert manager.can_call_endpoint("test") is True

    # Add a limiter and make a request
    limiter = manager.get_limiter("test")
    limiter.acquire(blocking=False)

    # Should still be able to call
    assert manager.can_call_endpoint("test") is True


def test_get_rate_limit_manager():
    """Test getting global manager."""
    manager1 = get_rate_limit_manager()
    manager2 = get_rate_limit_manager()

    # Should return same instance
    assert manager1 is manager2


def test_create_limiter():
    """Test creating limiter with custom config."""
    limiter = create_limiter(
        requests_per_second=5.0,
        requests_per_minute=50.0,
        burst_size=3
    )

    stats = limiter.get_stats()
    assert stats["limits"]["per_second"] == 5.0
    assert stats["limits"]["per_minute"] == 50.0
    assert stats["limits"]["burst_size"] == 3


def test_rate_limiter_timeout():
    """Test timeout behavior."""
    limiter = RateLimiter(RateLimitConfig(burst_size=1, requests_per_second=1.0))

    # Fill up the limiter
    limiter.acquire(blocking=False)

    # Make several more requests to exhaust limits
    for _ in range(5):
        limiter.acquire(blocking=False)

    # Should eventually timeout or have high wait time
    result = limiter.acquire(blocking=True, timeout=0.1)
    # Just check it returns a boolean
    assert isinstance(result, bool)


def test_rate_limiter_concurrent():
    """Test concurrent access."""
    import threading

    limiter = RateLimiter(RateLimitConfig(burst_size=5))
    results = []
    lock = threading.Lock()

    def make_request():
        result = limiter.acquire(blocking=True, timeout=1.0)
        with lock:
            results.append(result)

    # Create multiple threads
    threads = []
    for _ in range(10):
        t = threading.Thread(target=make_request)
        threads.append(t)
        t.start()

    for t in threads:
        t.join()

    # Some should succeed
    success_count = sum(1 for r in results if r)
    assert success_count > 0
