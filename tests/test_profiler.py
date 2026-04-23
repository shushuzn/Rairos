"""Tests for performance profiling functionality."""
import pytest
import time
from core.profiler import (
    PerformanceProfiler,
    FunctionProfile,
    get_profiler,
    profile,
    profile_block,
    MemoryProfiler,
    get_memory_profiler,
)


def test_function_profile_creation():
    """Test FunctionProfile creation."""
    profile = FunctionProfile(name="test_function")
    
    assert profile.name == "test_function"
    assert profile.call_count == 0
    assert profile.total_time == 0.0


def test_function_profile_update():
    """Test updating a function profile."""
    profile = FunctionProfile(name="test_function")
    
    profile.update(0.5)
    assert profile.call_count == 1
    assert profile.total_time == 0.5
    assert profile.min_time == 0.5
    assert profile.max_time == 0.5
    assert profile.avg_time == 0.5
    
    profile.update(0.3)
    assert profile.call_count == 2
    assert profile.total_time == 0.8
    assert profile.min_time == 0.3
    assert profile.max_time == 0.5
    assert profile.avg_time == 0.4


def test_profiler_enable_disable():
    """Test profiler enable/disable."""
    profiler = PerformanceProfiler()
    
    profiler.disable()
    assert profiler._enabled is False
    
    profiler.enable()
    assert profiler._enabled is True


def test_profiler_decorator():
    """Test profiler decorator."""
    profiler = PerformanceProfiler()
    profiler.reset()
    
    @profiler.profile_function("test_func")
    def test_func():
        time.sleep(0.01)
        return "result"
    
    result = test_func()
    
    assert result == "result"
    profile = profiler.get_profile("test_func")
    assert profile is not None
    assert profile.call_count == 1
    assert profile.total_time >= 0  # Just check it's tracked


def test_profiler_context_manager():
    """Test profiler context manager."""
    profiler = PerformanceProfiler()
    profiler.reset()
    
    with profiler.profile_block("test_block"):
        time.sleep(0.01)
    
    profile = profiler.get_profile("test_block")
    assert profile is not None
    assert profile.call_count == 1
    assert profile.total_time >= 0  # Just check it's tracked


def test_profiler_get_slowest_functions():
    """Test getting slowest functions."""
    profiler = PerformanceProfiler()
    profiler.reset()
    
    profiler._record_call("slow", 1.0)
    profiler._record_call("fast", 0.1)
    profiler._record_call("medium", 0.5)
    
    slowest = profiler.get_slowest_functions(2)
    assert len(slowest) == 2
    assert slowest[0].name == "slow"
    assert slowest[1].name == "medium"


def test_profiler_get_most_called():
    """Test getting most called functions."""
    profiler = PerformanceProfiler()
    profiler.reset()
    
    for _ in range(10):
        profiler._record_call("frequent", 0.01)
    
    for _ in range(5):
        profiler._record_call("rare", 0.01)
    
    most_called = profiler.get_most_called(2)
    assert len(most_called) == 2
    assert most_called[0].name == "frequent"
    assert most_called[1].name == "rare"


def test_profiler_report():
    """Test profiler report generation."""
    profiler = PerformanceProfiler()
    profiler.reset()
    
    profiler._record_call("test1", 0.5)
    profiler._record_call("test2", 0.3)
    
    report = profiler.get_report()
    
    assert "PERFORMANCE PROFILE REPORT" in report
    assert "test1" in report
    assert "test2" in report


def test_profiler_reset():
    """Test profiler reset."""
    profiler = PerformanceProfiler()
    
    profiler._record_call("test", 0.5)
    assert len(profiler._profiles) == 1
    
    profiler.reset()
    assert len(profiler._profiles) == 0


def test_get_profiler():
    """Test getting global profiler."""
    profiler1 = get_profiler()
    profiler2 = get_profiler()
    
    assert profiler1 is profiler2


def test_global_profile_decorator():
    """Test global profile decorator."""
    profiler = get_profiler()
    profiler.reset()
    profiler.enable()
    
    @profile("decorated_func")
    def my_func():
        time.sleep(0.01)
        return "done"
    
    result = my_func()
    assert result == "done"
    
    profile_data = profiler.get_profile("decorated_func")
    assert profile_data is not None
    assert profile_data.call_count == 1


def test_global_profile_block():
    """Test global profile block."""
    profiler = get_profiler()
    profiler.reset()
    
    with profile_block("block_test"):
        time.sleep(0.01)
    
    profile_data = profiler.get_profile("block_test")
    assert profile_data is not None


def test_memory_profiler():
    """Test memory profiler."""
    profiler = get_memory_profiler()
    
    with profiler.track("test_operation"):
        time.sleep(0.01)
    
    snapshots = profiler.get_snapshots()
    assert len(snapshots) == 1
    assert snapshots[0]["name"] == "test_operation"


def test_memory_profiler_peak():
    """Test memory profiler peak calculation."""
    profiler = MemoryProfiler()
    
    profiler._snapshots = [
        {"memory_delta_mb": 1.0},
        {"memory_delta_mb": 5.0},
        {"memory_delta_mb": 2.0},
    ]
    
    assert profiler.get_peak_increase() == 5.0


def test_memory_profiler_report():
    """Test memory profiler report."""
    profiler = get_memory_profiler()
    
    # Add a snapshot first
    with profiler.track("test"):
        pass
    
    report = profiler.get_report()
    assert "MEMORY PROFILING REPORT" in report


def test_profiler_stats_dict():
    """Test profiler stats dictionary."""
    profiler = PerformanceProfiler()
    profiler.reset()
    
    profiler._record_call("test1", 0.5)
    profiler._record_call("test2", 0.3)
    
    stats = profiler.get_stats_dict()
    
    assert stats["total_functions"] == 2
    assert "slowest" in stats
    assert "most_called" in stats
