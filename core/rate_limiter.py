"""
API Rate Limiter and Request Manager.

Helps control API call frequency to avoid rate limits and quota exhaustion.
"""
import time
import threading
import logging
from typing import Dict, Optional, Callable, Any
from dataclasses import dataclass
from collections import deque
from contextlib import contextmanager

logger = logging.getLogger(__name__)


@dataclass
class RateLimitConfig:
    """Rate limit configuration for an API."""
    requests_per_second: float = 10.0
    requests_per_minute: float = 100.0
    requests_per_hour: float = 1000.0
    burst_size: int = 5  # Max requests that can be made in quick succession


class RateLimiter:
    """
    Token bucket rate limiter with multiple time windows.
    
    Tracks requests across second, minute, and hour windows to ensure
    API calls stay within defined limits.
    """

    def __init__(self, config: Optional[RateLimitConfig] = None):
        self.config = config or RateLimitConfig()

        # Token bucket state
        self._tokens = self.config.burst_size
        self._last_update = time.time()
        self._lock = threading.Lock()

        # Request history for sliding window
        self._second_history: deque = deque()
        self._minute_history: deque = deque()
        self._hour_history: deque = deque()

        # Statistics
        self._total_requests = 0
        self._total_wait_time = 0.0
        self._total_rejected = 0

    def _refill_tokens(self):
        """Refill tokens based on elapsed time."""
        now = time.time()
        elapsed = now - self._last_update

        # Refill tokens based on rate
        refill = elapsed * self.config.requests_per_second
        self._tokens = min(self.config.burst_size, self._tokens + refill)
        self._last_update = now

    def _clean_history(self):
        """Remove expired entries from history."""
        now = time.time()

        # Clean second history (older than 1 second)
        while self._second_history and now - self._second_history[0] > 1.0:
            self._second_history.popleft()

        # Clean minute history (older than 1 minute)
        while self._minute_history and now - self._minute_history[0] > 60.0:
            self._minute_history.popleft()

        # Clean hour history (older than 1 hour)
        while self._hour_history and now - self._hour_history[0] > 3600.0:
            self._hour_history.popleft()

    def can_make_request(self) -> bool:
        """Check if a request can be made without waiting."""
        self._clean_history()

        # Check all limits
        if len(self._second_history) >= self.config.requests_per_second:
            return False

        if len(self._minute_history) >= self.config.requests_per_minute:
            return False

        if len(self._hour_history) >= self.config.requests_per_hour:
            return False

        return True

    def acquire(self, blocking: bool = True, timeout: Optional[float] = None) -> bool:
        """
        Acquire permission to make a request.
        
        Args:
            blocking: If True, wait until a request can be made
            timeout: Maximum time to wait (None = wait forever)
            
        Returns:
            True if request was allowed, False if timeout or rejected
        """
        start_time = time.time()

        while True:
            with self._lock:
                self._refill_tokens()
                self._clean_history()

                now = time.time()

                # Check if we can make a request
                if self.can_make_request():
                    # Record the request
                    self._second_history.append(now)
                    self._minute_history.append(now)
                    self._hour_history.append(now)
                    self._tokens -= 1
                    self._total_requests += 1
                    return True

                # Calculate wait time
                if self._second_history:
                    wait_time = 1.0 - (now - self._second_history[0])
                elif self._minute_history:
                    wait_time = 60.0 - (now - self._minute_history[0])
                else:
                    wait_time = 1.0 / self.config.requests_per_second

                wait_time = max(0.01, min(wait_time, 10.0))  # Cap wait time

                # Check timeout
                if not blocking:
                    self._total_rejected += 1
                    return False

                if timeout is not None and (time.time() - start_time) >= timeout:
                    self._total_rejected += 1
                    return False

            # Wait before retrying
            time.sleep(wait_time)
            self._total_wait_time += wait_time

    def wait_if_needed(self) -> float:
        """
        Wait if necessary to respect rate limits.
        
        Returns:
            Time waited in seconds
        """
        start = time.time()
        self.acquire(blocking=True)
        return time.time() - start

    def get_stats(self) -> Dict[str, Any]:
        """Get rate limiter statistics."""
        self._clean_history()

        return {
            "total_requests": self._total_requests,
            "total_wait_time": self._total_wait_time,
            "total_rejected": self._total_rejected,
            "current_second_requests": len(self._second_history),
            "current_minute_requests": len(self._minute_history),
            "current_hour_requests": len(self._hour_history),
            "tokens_available": self._tokens,
            "limits": {
                "per_second": self.config.requests_per_second,
                "per_minute": self.config.requests_per_minute,
                "per_hour": self.config.requests_per_hour,
                "burst_size": self.config.burst_size,
            }
        }

    def reset_stats(self):
        """Reset statistics."""
        self._total_requests = 0
        self._total_wait_time = 0.0
        self._total_rejected = 0


class APIRateLimitManager:
    """
    Manager for multiple API rate limiters.
    
    Allows different configurations for different API endpoints.
    """

    def __init__(self):
        self._limiters: Dict[str, RateLimiter] = {}
        self._lock = threading.Lock()

    def get_limiter(self, endpoint: str, config: Optional[RateLimitConfig] = None) -> RateLimiter:
        """Get or create a rate limiter for an endpoint."""
        with self._lock:
            if endpoint not in self._limiters:
                self._limiters[endpoint] = RateLimiter(config)
            return self._limiters[endpoint]

    def wait_for_endpoint(self, endpoint: str, config: Optional[RateLimitConfig] = None) -> float:
        """Wait for rate limit for a specific endpoint."""
        limiter = self.get_limiter(endpoint, config)
        return limiter.wait_if_needed()

    def can_call_endpoint(self, endpoint: str) -> bool:
        """Check if an endpoint can be called immediately."""
        if endpoint not in self._limiters:
            return True
        return self._limiters[endpoint].can_make_request()

    def get_all_stats(self) -> Dict[str, Dict[str, Any]]:
        """Get statistics for all endpoints."""
        return {
            endpoint: limiter.get_stats()
            for endpoint, limiter in self._limiters.items()
        }


# Global rate limit manager
_rate_limit_manager: Optional[APIRateLimitManager] = None


def get_rate_limit_manager() -> APIRateLimitManager:
    """Get the global rate limit manager."""
    global _rate_limit_manager
    if _rate_limit_manager is None:
        _rate_limit_manager = APIRateLimitManager()
    return _rate_limit_manager


@contextmanager
def rate_limited(endpoint: str, config: Optional[RateLimitConfig] = None):
    """
    Context manager for rate-limited API calls.
    
    Usage:
        with rate_limited("arxiv"):
            # Make API call
            pass
    """
    manager = get_rate_limit_manager()
    wait_time = manager.wait_for_endpoint(endpoint, config)
    yield wait_time


def create_limiter(
    requests_per_second: float = 10.0,
    requests_per_minute: float = 100.0,
    requests_per_hour: float = 1000.0,
    burst_size: int = 5
) -> RateLimiter:
    """Create a new rate limiter with custom settings."""
    config = RateLimitConfig(
        requests_per_second=requests_per_second,
        requests_per_minute=requests_per_minute,
        requests_per_hour=requests_per_hour,
        burst_size=burst_size
    )
    return RateLimiter(config)


# Decorator for rate-limited functions
def rate_limit(
    endpoint: str = "default",
    requests_per_second: float = 10.0,
    requests_per_minute: float = 100.0,
    requests_per_hour: float = 1000.0,
    burst_size: int = 5
) -> Callable:
    """
    Decorator to rate-limit a function.
    
    Usage:
        @rate_limit("arxiv", requests_per_minute=30)
        def fetch_arxiv_data():
            # Make API call
            pass
    """
    def decorator(func: Callable) -> Callable:
        config = RateLimitConfig(
            requests_per_second=requests_per_second,
            requests_per_minute=requests_per_minute,
            requests_per_hour=requests_per_hour,
            burst_size=burst_size
        )
        manager = get_rate_limit_manager()

        def wrapper(*args, **kwargs):
            manager.wait_for_endpoint(endpoint, config)
            return func(*args, **kwargs)

        return wrapper

    return decorator
