"""Tests for core/profiler.py."""
import time
import sys
from pathlib import Path
from unittest.mock import patch, MagicMock
import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))
from core.profiler import (
    PerformanceProfiler,
    FunctionProfile,
    MemoryProfiler,
    ProfilerContext,
    get_profiler,
    profile,
    profile_block,
)


class TestFunctionProfile:
    """Tests for FunctionProfile dataclass."""

    def test_initialization(self):
        fp = FunctionProfile(name="test_func")
        assert fp.name == "test_func"
        assert fp.call_count == 0
        assert fp.total_time == 0.0
        assert fp.min_time == float("inf")
        assert fp.max_time == 0.0
        assert fp.avg_time == 0.0

    def test_update_single_call(self):
        fp = FunctionProfile(name="test_func")
        fp.update(0.05)
        assert fp.call_count == 1
        assert fp.total_time == 0.05
        assert fp.min_time == 0.05
        assert fp.max_time == 0.05
        assert fp.avg_time == 0.05

    def test_update_multiple_calls(self):
        fp = FunctionProfile(name="test_func")
        fp.update(0.03)
        fp.update(0.07)
        fp.update(0.05)
        assert fp.call_count == 3
        assert fp.total_time == pytest.approx(0.15)
        assert fp.min_time == pytest.approx(0.03)
        assert fp.max_time == pytest.approx(0.07)
        assert fp.avg_time == pytest.approx(0.05)


class TestPerformanceProfiler:
    """Tests for PerformanceProfiler."""

    def test_enable_disable(self):
        profiler = PerformanceProfiler()
        assert profiler._enabled is True
        profiler.disable()
        assert profiler._enabled is False
        profiler.enable()
        assert profiler._enabled is True

    def test_profile_function_decorator(self):
        profiler = PerformanceProfiler()

        @profiler.profile_function(name="my_func")
        def my_func(x):
            return x * 2

        result = my_func(5)
        assert result == 10
        p = profiler.get_profile("my_func")
        assert p is not None
        assert p.call_count == 1

    def test_profile_function_disabled(self):
        profiler = PerformanceProfiler()
        profiler.disable()

        @profiler.profile_function(name="disabled_func")
        def disabled_func():
            return 42

        result = disabled_func()
        assert result == 42
        assert profiler.get_profile("disabled_func") is None

    def test_profile_block(self):
        profiler = PerformanceProfiler()
        with profiler.profile_block("block1"):
            time.sleep(0.05)
        p = profiler.get_profile("block1")
        assert p is not None
        assert p.call_count == 1
        assert p.total_time >= 0

    def test_profile_block_disabled(self):
        profiler = PerformanceProfiler()
        profiler.disable()
        with profiler.profile_block("disabled_block"):
            pass
        assert profiler.get_profile("disabled_block") is None

    def test_get_all_profiles_sorted(self):
        profiler = PerformanceProfiler()
        with profiler.profile_block("slow"):
            time.sleep(0.05)
        with profiler.profile_block("fast"):
            pass
        profiles = profiler.get_all_profiles()
        assert profiles[0].name == "slow"
        assert profiles[1].name == "fast"

    def test_get_slowest_functions(self):
        profiler = PerformanceProfiler()
        for i in range(15):
            with profiler.profile_block(f"func_{i}"):
                time.sleep(0.001 * (15 - i))
        slowest = profiler.get_slowest_functions(5)
        assert len(slowest) == 5
        assert slowest[0].name == "func_0"

    def test_get_most_called(self):
        profiler = PerformanceProfiler()

        @profiler.profile_function(name="hot_func")
        def hot_func():
            pass

        for _ in range(100):
            hot_func()

        @profiler.profile_function(name="cold_func")
        def cold_func():
            pass

        cold_func()
        most_called = profiler.get_most_called(5)
        assert most_called[0].name == "hot_func"
        assert most_called[0].call_count == 100

    def test_reset(self):
        profiler = PerformanceProfiler()
        with profiler.profile_block("to_reset"):
            pass
        assert profiler.get_profile("to_reset") is not None
        profiler.reset()
        assert len(profiler._profiles) == 0

    def test_get_stats_dict(self):
        profiler = PerformanceProfiler()
        with profiler.profile_block("stat_func"):
            pass
        stats = profiler.get_stats_dict()
        assert "total_functions" in stats
        assert "slowest" in stats
        assert "most_called" in stats
        assert stats["total_functions"] == 1


class TestMemoryProfiler:
    """Tests for MemoryProfiler (psutil optional)."""

    def test_get_current_memory_returns_float(self):
        profiler = MemoryProfiler()
        mem = profiler._get_current_memory()
        assert isinstance(mem, float)
        assert mem >= 0.0

    def test_track_context_manager(self):
        profiler = MemoryProfiler()
        with profiler.track("test_operation"):
            x = [0] * 1000
        snapshots = profiler.get_snapshots()
        assert len(snapshots) == 1
        assert snapshots[0]["name"] == "test_operation"

    def test_get_peak_increase(self):
        profiler = MemoryProfiler()
        assert profiler.get_peak_increase() == 0.0
        with profiler.track("op1"):
            pass
        assert profiler.get_peak_increase() >= 0.0

    def test_get_report_empty(self):
        profiler = MemoryProfiler()
        report = profiler.get_report()
        assert "No memory snapshots" in report

    def test_get_report_with_data(self):
        profiler = MemoryProfiler()
        with profiler.track("test_op"):
            pass
        report = profiler.get_report()
        assert "MEMORY PROFILING REPORT" in report
        assert "test_op" in report


class TestProfilerContext:
    """Tests for ProfilerContext (cProfile wrapper)."""

    def test_enter_exit(self):
        ctx = ProfilerContext()
        assert ctx.enabled is True
        with ctx:
            pass
        assert ctx.stats is None  # Not computed until get_stats called

    def test_disabled_context(self):
        ctx = ProfilerContext(enabled=False)
        with ctx:
            pass
        # When disabled, no profiling data - get_stats raises TypeError
        # (pstats.Stats cannot construct from disabled profiler)
        with pytest.raises(TypeError):
            ctx.get_stats()


class TestGlobalFunctions:
    """Tests for module-level functions."""

    def test_get_profiler_singleton(self):
        p1 = get_profiler()
        p2 = get_profiler()
        assert p1 is p2

    def test_profile_decorator(self):
        @profile(name="decorated_func")
        def decorated_func():
            return 123

        result = decorated_func()
        assert result == 123
        p = get_profiler().get_profile("decorated_func")
        assert p is not None
        get_profiler().reset()  # Clean up

    def test_profile_block_global(self):
        with profile_block("global_block"):
            time.sleep(0.001)
        p = get_profiler().get_profile("global_block")
        assert p is not None
        get_profiler().reset()  # Clean up
