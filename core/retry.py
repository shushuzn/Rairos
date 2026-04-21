"""Retry utilities with exponential backoff and circuit breaker."""
from __future__ import annotations

import logging
import time
import threading
from functools import wraps
from typing import Callable, Sequence

logger = logging.getLogger(__name__)


# ─── Retry Decorator ──────────────────────────────────────────────────────────


def retry(
    max_attempts: int = 3,
    base_delay: float = 1.0,
    max_delay: float = 30.0,
    exceptions: Sequence[type[Exception]] = (Exception,),
    on_retry: Callable[[Exception, int], None] | None = None,
) -> Callable:
    """
    Decorator that retries a function with exponential backoff.

    Args:
        max_attempts: Maximum number of attempts (including the first).
        base_delay: Initial delay in seconds.
        max_delay: Maximum delay cap in seconds.
        exceptions: Tuple of exception types to catch and retry.
        on_retry: Optional callback fired before each retry with (exception, attempt).
    """

    def decorator(fn: Callable) -> Callable:
        @wraps(fn)
        def wrapper(*args, **kwargs):
            last_exc: Exception | None = None
            for attempt in range(1, max_attempts + 1):
                try:
                    return fn(*args, **kwargs)
                except exceptions as e:  # noqa: PERF203
                    last_exc = e
                    if attempt == max_attempts:
                        break
                    delay = min(base_delay * (2 ** (attempt - 1)), max_delay)
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


# ─── Circuit Breaker ──────────────────────────────────────────────────────────


class CircuitBreaker:
    """
    Circuit breaker that prevents repeated calls to a failing service.

    States:
        CLOSED  → normal operation, calls pass through
        OPEN    → calls fail immediately (CircuitOpen)
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
        self._last_failure_time: float | None = None
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
        except self.expected_exception as e:  # noqa: F841
            self.record_failure()
            raise


class CircuitOpen(Exception):
    """Raised when a circuit breaker is open."""


# ─── Circuit Breaker Decorator ────────────────────────────────────────────────


def circuit_breaker(
    _fn: Callable | None = None,
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
