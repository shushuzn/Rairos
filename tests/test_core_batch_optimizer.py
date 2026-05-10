"""Tests for core/batch_optimizer.py — BatchOptimizer."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

from core.batch_optimizer import BatchOptimizer, BatchResult


class TestBatchResult:
    def test_creation(self):
        result = BatchResult(
            success_count=5,
            failure_count=2,
            total_time=1.5,
            results=[1, 2, 3],
            errors=["err1", "err2"],
        )
        assert result.success_count == 5
        assert result.failure_count == 2
        assert result.total_time == 1.5
        assert result.results == [1, 2, 3]
        assert result.errors == ["err1", "err2"]


class TestBatchOptimizer:
    def test_process_batch_success(self):
        opt = BatchOptimizer(max_workers=2)
        items = [1, 2, 3, 4, 5]
        result = opt.process_batch(items, lambda x: x * 2)
        assert result.success_count == 5
        assert result.failure_count == 0
        assert set(result.results) == {2, 4, 6, 8, 10}
        assert result.errors == []

    def test_process_batch_with_failures(self):
        opt = BatchOptimizer(max_workers=2)
        items = [1, 2, 3]
        errors_collected = []

        def processor(x):
            if x == 2:
                raise ValueError("bad value")
            return x * 2

        def handler(e, item):
            errors_collected.append((e, item))

        result = opt.process_batch(items, processor, error_handler=handler)
        assert result.success_count == 2
        assert result.failure_count == 1
        assert set(result.results) == {2, 6}
        assert len(result.errors) == 1
        assert len(errors_collected) == 1

    def test_process_batch_empty(self):
        opt = BatchOptimizer(max_workers=2)
        result = opt.process_batch([], lambda x: x)
        assert result.success_count == 0
        assert result.failure_count == 0
        assert result.results == []
        assert result.errors == []

    def test_process_sequential_success(self):
        opt = BatchOptimizer(max_workers=2)
        items = ["a", "b", "c"]
        result = opt.process_sequential(items, lambda x: x.upper())
        assert result.success_count == 3
        assert result.failure_count == 0
        assert result.results == ["A", "B", "C"]
        assert result.errors == []

    def test_process_sequential_with_failures(self):
        opt = BatchOptimizer(max_workers=2)
        items = [1, 2, 3, 4]
        result = opt.process_sequential(items, lambda x: 10 // (x - 2))
        # x=3 raises ZeroDivisionError (10 // 1 = 10 OK; x=4 raises 10 // 2 = 5 OK)
        # Actually: x=1 -> 10//(-1)=-10 OK, x=2 -> ZeroDivisionError, x=3 -> 10//1=10 OK, x=4 -> 10//2=5 OK
        assert result.failure_count == 1
        assert result.success_count == 3

    def test_process_sequential_empty(self):
        opt = BatchOptimizer()
        result = opt.process_sequential([], lambda x: x)
        assert result.success_count == 0
        assert result.total_time >= 0
