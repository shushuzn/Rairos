"""Retry utilities with exponential backoff and circuit breaker."""
from __future__ import annotations

import logging
import random
import time
import threading
from functools import wraps
from typing import Callable, Optional, Sequence, Dict, Any

logger = logging.getLogger(__name__)


# ─── Retry Statistics ────────────────────────────────────────────────────


class RetryStats:
    """Track retry statistics for monitoring."""

    def __init__(self):
        self._stats: Dict[str, Dict[str, Any]] = {}
        self._lock = threading.Lock()

    def record_attempt(self, func_name: str, attempt: int, success: bool, error: str = None):  # type: ignore[assignment]
        """Record a retry attempt."""
        with self._lock:
            if func_name not in self._stats:
                self._stats[func_name] = {
                    "total_attempts": 0,
                    "total_failures": 0,
                    "total_retries": 0,
                    "total_success": 0,
                    "errors": {}
                }

            self._stats[func_name]["total_attempts"] += 1
            if not success:
                self._stats[func_name]["total_failures"] += 1
                self._stats[func_name]["total_retries"] += attempt
                if error:
                    error_type = type(error).__name__
                    self._stats[func_name]["errors"][error_type] = \
                        self._stats[func_name]["errors"].get(error_type, 0) + 1
            else:
                self._stats[func_name]["total_success"] += 1

    def get_stats(self, func_name: str = None) -> Dict[str, Any]:  # type: ignore[assignment]
        """Get statistics for a function or all functions."""
        with self._lock:
            if func_name:
                return self._stats.get(func_name, {})
            return dict(self._stats)

    def reset(self, func_name: str = None):  # type: ignore[assignment]
        """Reset statistics."""
        with self._lock:
            if func_name and func_name in self._stats:
                del self._stats[func_name]
            elif func_name is None:
                self._stats.clear()


# Global retry statistics tracker
_retry_stats = RetryStats()


def get_retry_stats() -> RetryStats:
    """Get the global retry statistics tracker."""
    return _retry_stats


# ─── Retry Decorator ──────────────────────────────────────────────────


def retry(
    max_attempts: int = 3,
    base_delay: float = 1.0,
    max_delay: float = 30.0,
    exceptions: Sequence[type[Exception]] = (Exception,),
    on_retry: Callable[[Exception, int], None] | None = None,
    jitter: float = 0.0,
    track_stats: bool = False,
) -> Callable:
    """
    Decorator that retries a function with exponential backoff and optional jitter.

    Args:
        max_attempts: Maximum number of attempts (including the first).
        base_delay: Initial delay in seconds.
        max_delay: Maximum delay cap in seconds.
        exceptions: Tuple of exception types to catch and retry.
        on_retry: Optional callback fired before each retry with (exception, attempt).
        jitter: Random jitter factor (0.0-1.0) added to delay for better distribution.
        track_stats: Whether to track retry statistics.
    """

    def decorator(fn: Callable) -> Callable:
        func_name = f"{fn.__module__}.{fn.__qualname__}"

        @wraps(fn)
        def wrapper(*args, **kwargs):
            last_exc: Optional[Exception] = None
            for attempt in range(1, max_attempts + 1):
                try:
                    result = fn(*args, **kwargs)
                    if track_stats:
                        _retry_stats.record_attempt(func_name, attempt, success=True)
                    return result
                except exceptions as e:
                    last_exc = e
                    if track_stats:
                        _retry_stats.record_attempt(func_name, attempt, success=False, error=e)
                    if attempt == max_attempts:
                        break
                    delay = min(base_delay * (2 ** (attempt - 1)), max_delay)

                    # Add jitter if specified
                    if jitter > 0:
                        delay += delay * jitter * random.random()

                    logger.warning(
                        "[retry] %s attempt %d/%d failed: %s. Retrying in %.1fs",
                        fn.__qualname__,
                        attempt,
                        max_attempts,
                        e,
                        delay,
                    )
                    if on_retry is not None:
                        on_retry(e, attempt)
                    time.sleep(delay)
            # Re-raise the last exception with original traceback
            raise last_exc from last_exc.__cause__ if last_exc else None

        return wrapper

    return decorator


# ─── Circuit Breaker ──────────────────────────────────────────────────


class CircuitBreaker:
    """
    Circuit breaker that prevents repeated calls to a failing service.

    States:
        CLOSED → normal operation, calls pass through
        OPEN   → calls fail immediately (CircuitOpen)
        HALF-OPEN → one test call allowed
    """

    CLOSED = "closed"
    OPEN = "open"
    HALF_OPEN = "half-open"

    def __init__(
        self,
        failure_threshold: int = 5,
        recovery_timeout: float = 60.0,
        expected_exception: type[Exception] = Exception,
    ):
        self.failure_threshold = failure_threshold
        self.recovery_timeout = recovery_timeout
        self.expected_exception = expected_exception

        self._state = self.CLOSED
        self._failure_count = 0
        self._last_failure_time: Optional[float] = None
        self._lock = threading.RLock()

    @property
    def state(self) -> str:
        with self._lock:
            if self._state == self.OPEN:
                if (
                    self._last_failure_time is not None
                    and time.time() - self._last_failure_time >= self.recovery_timeout
                ):
                    self._state = self.HALF_OPEN
            return self._state

    def record_success(self) -> None:
        with self._lock:
            self._failure_count = 0
            self._state = self.CLOSED

    def record_failure(self) -> None:
        with self._lock:
            self._failure_count += 1
            self._last_failure_time = time.time()
            if self._failure_count >= self.failure_threshold:
                self._state = self.OPEN
                logger.error(
                    "[circuit_breaker] OPEN after %d failures (threshold=%d)",
                    self._failure_count,
                    self.failure_threshold,
                )

    def call(self, fn: Callable, *args, **kwargs):
        """Execute fn through the circuit breaker."""
        state = self.state
        if state == self.OPEN:
            raise CircuitOpen(f"Circuit breaker is OPEN. Retry after {self.recovery_timeout}s")

        try:
            result = fn(*args, **kwargs)
            self.record_success()
            return result
        except self.expected_exception:
            self.record_failure()
            raise


class CircuitOpen(Exception):
    """Raised when a circuit breaker is open."""


# ─── Circuit Breaker Decorator ────────────────────────────────────────


def circuit_breaker(
    _fn: Optional[Callable[..., Any]] = None,
    *,
    failure_threshold: int = 5,
    recovery_timeout: float = 60.0,
    expected_exception: type[Exception] = Exception,
) -> Callable:
    """
    Decorator that wraps a function with a circuit breaker.

    Usage:
        @circuit_breaker()
        def my_func():
            ...

        # or with options:
        @circuit_breaker(failure_threshold=3, recovery_timeout=30.0)
        def my_func():
            ...

    Thread-safe. Each decorated function gets its own CircuitBreaker instance.
    """
    _breakers: dict[str, CircuitBreaker] = {}
    _breakers_lock = threading.Lock()

    def decorator(fn: Callable) -> Callable:
        key = f"{fn.__module__}.{fn.__qualname__}"

        def get_breaker() -> CircuitBreaker:
            with _breakers_lock:
                if key not in _breakers:
                    _breakers[key] = CircuitBreaker(
                        failure_threshold=failure_threshold,
                        recovery_timeout=recovery_timeout,
                        expected_exception=expected_exception,
                    )
                return _breakers[key]

        @wraps(fn)
        def wrapper(*args, **kwargs):
            breaker = get_breaker()
            if breaker.state == CircuitBreaker.OPEN:
                raise CircuitOpen(
                    f"[circuit_breaker] {fn.__qualname__} is OPEN. "
                    f"Retry after {breaker.recovery_timeout:.0f}s."
                )
            try:
                result = fn(*args, **kwargs)
                breaker.record_success()
                return result
            except expected_exception:
                breaker.record_failure()
                raise

        return wrapper

    if _fn is not None:
        return decorator(_fn)
    return decorator
