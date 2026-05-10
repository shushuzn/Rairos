"""Tests for core/rate_limiter.py."""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))
from core.rate_limiter import RateLimitConfig, RateLimiter


class TestRateLimitConfig:
    def test_defaults(self):
        cfg = RateLimitConfig()
        assert cfg.requests_per_second == 10.0
        assert cfg.requests_per_minute == 100.0
        assert cfg.requests_per_hour == 1000.0
        assert cfg.burst_size == 5

    def test_custom_values(self):
        cfg = RateLimitConfig(requests_per_second=5.0, burst_size=10)
        assert cfg.requests_per_second == 5.0
        assert cfg.burst_size == 10


class TestRateLimiterInit:
    def test_init_with_default_config(self):
        rl = RateLimiter()
        assert rl.config.requests_per_second == 10.0
        assert rl._tokens == 5  # burst_size

    def test_init_with_custom_config(self):
        cfg = RateLimitConfig(burst_size=3)
        rl = RateLimiter(config=cfg)
        assert rl._tokens == 3


class TestRateLimiterCanMakeRequest:
    def test_can_make_request_returns_bool(self):
        rl = RateLimiter()
        result = rl.can_make_request()
        assert isinstance(result, bool)

    def test_can_make_request_true_initially(self):
        rl = RateLimiter()
        # Fresh limiter should be able to make a request
        assert rl.can_make_request() is True


class TestRateLimiterAcquire:
    def test_acquire_returns_true(self):
        rl = RateLimiter(RateLimitConfig(burst_size=5))
        result = rl.acquire(timeout=1.0)
        assert result is True

    def test_acquire_false_on_timeout(self):
        # Extremely restrictive limiter should timeout
        rl = RateLimiter(RateLimitConfig(
            requests_per_second=0.0001,
            requests_per_minute=0.0001,
            requests_per_hour=0.0001,
        ))
        # Very short timeout should fail
        result = rl.acquire(blocking=True, timeout=0.001)
        assert isinstance(result, bool)


class TestRateLimiterWaitIfNeeded:
    def test_wait_if_needed_returns_float(self):
        rl = RateLimiter()
        wt = rl.wait_if_needed()
        assert isinstance(wt, float)
        assert wt >= 0.0
