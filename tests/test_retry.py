"""Tests for core/retry.py — decorator API."""
from __future__ import annotations

import time

import pytest

from core.retry import CircuitBreaker, CircuitOpen, retry


# retry is a decorator — use as @retry(...)
class TestRetry:
    def test_succeeds_first_try(self):
        @retry(max_attempts=3, base_delay=0.01)
        def succeed():
            return 42
        assert succeed() == 42

    def test_retries_on_failure_then_succeeds(self):
        calls = [0]

        @retry(max_attempts=5, base_delay=0.01)
        def flaky():
            calls[0] += 1
            if calls[0] < 3:
                raise ConnectionError("transient")
            return "done"
        assert flaky() == "done"
        assert calls[0] == 3

    def test_raises_after_all_attempts_fail(self):
        @retry(max_attempts=3, base_delay=0.01)
        def always_fail():
            raise ConnectionError("permanent")
        with pytest.raises(ConnectionError):
            always_fail()


class TestCircuitBreaker:
    def test_default_state_is_closed(self):
        cb = CircuitBreaker()
        assert cb._state == cb.CLOSED

    def test_call_success(self):
        cb = CircuitBreaker()
        assert cb.call(lambda: "ok") == "ok"

    def test_failure_count_increases(self):
        cb = CircuitBreaker(failure_threshold=5, recovery_timeout=60.0)
        try:
            cb.call(lambda: (_ for _ in ()).throw(ValueError("fail")))
        except ValueError:
            pass
        assert cb._failure_count == 1

    def test_circuit_opens_at_threshold(self):
        cb = CircuitBreaker(failure_threshold=2, recovery_timeout=60.0)
        for _ in range(2):
            try:
                cb.call(lambda: (_ for _ in ()).throw(ValueError("fail")))
            except ValueError:
                pass
        assert cb._state == cb.OPEN

    def test_open_raises_circuit_open(self):
        cb = CircuitBreaker(failure_threshold=1, recovery_timeout=60.0)
        try:
            cb.call(lambda: (_ for _ in ()).throw(ValueError("fail")))
        except ValueError:
            pass
        assert cb._state == cb.OPEN
        with pytest.raises(CircuitOpen):
            cb.call(lambda: "ok")

    def test_half_open_after_recovery_timeout(self):
        cb = CircuitBreaker(failure_threshold=2, recovery_timeout=0.5)
        for _ in range(2):
            try:
                cb.call(lambda: (_ for _ in ()).throw(ValueError("fail")))
            except ValueError:
                pass
        assert cb._state == cb.OPEN
        time.sleep(0.6)
        assert cb.call(lambda: "recovered") == "recovered"
        assert cb._state == cb.CLOSED
