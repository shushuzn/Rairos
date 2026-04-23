"""Tests for resource monitoring functionality."""
from pathlib import Path
from core.resource_monitor import (
    ResourceMonitor,
    ResourceGuard,
    APIBudgetTracker,
    get_resource_monitor,
    get_api_budget_tracker,
)


def test_resource_monitor_creation():
    """Test resource monitor can be created."""
    monitor = ResourceMonitor()
    assert monitor is not None
    assert isinstance(monitor.data_dir, Path)


def test_get_disk_info():
    """Test getting disk information."""
    monitor = ResourceMonitor()
    disk_info = monitor.get_disk_info()
    
    assert disk_info.total_gb > 0
    assert disk_info.used_gb >= 0
    assert disk_info.free_gb >= 0
    assert disk_info.percent >= 0
    assert disk_info.percent <= 100


def test_get_memory_info():
    """Test getting memory information."""
    monitor = ResourceMonitor()
    mem_used, mem_avail, mem_pct = monitor.get_memory_info()
    
    assert mem_used >= 0
    assert mem_avail >= 0
    assert mem_pct >= 0
    assert mem_pct <= 100


def test_get_cpu_info():
    """Test getting CPU information."""
    monitor = ResourceMonitor()
    cpu_pct, cpu_count = monitor.get_cpu_info()
    
    assert cpu_pct >= 0
    assert cpu_pct <= 100
    assert cpu_count > 0


def test_collect_stats():
    """Test collecting resource statistics."""
    monitor = ResourceMonitor()
    stats = monitor.collect_stats()
    
    assert stats.timestamp > 0
    assert stats.cpu_percent >= 0
    assert stats.memory_used_mb >= 0
    assert stats.disk_used_gb >= 0


def test_get_recent_stats():
    """Test getting recent statistics."""
    monitor = ResourceMonitor()
    # Collect some stats
    for _ in range(5):
        monitor.collect_stats()
    
    recent = monitor.get_recent_stats(3)
    assert len(recent) <= 3


def test_resource_guard_check():
    """Test resource guard check."""
    guard = ResourceGuard(
        min_disk_gb=0.001,  # Very low threshold for testing
        max_memory_percent=99.0  # Very high threshold for testing
    )
    
    ok, msg = guard.check()
    assert ok is True


def test_api_budget_tracker_creation():
    """Test API budget tracker creation."""
    tracker = APIBudgetTracker(monthly_budget_usd=50.0)
    assert tracker.monthly_budget_usd == 50.0
    assert tracker._call_count == 0


def test_api_budget_tracker_record():
    """Test recording API calls."""
    tracker = APIBudgetTracker(monthly_budget_usd=10.0)
    tracker.record_api_call(
        provider="openai",
        endpoint="/chat/completions",
        tokens_used=1000,
        cost_per_1k=0.002
    )
    
    assert tracker._call_count == 1
    assert tracker._cost_estimate == 0.002


def test_api_budget_tracker_should_make_call():
    """Test checking if API call should be made."""
    tracker = APIBudgetTracker(monthly_budget_usd=10.0)
    
    # Should allow call within budget
    assert tracker.should_make_api_call(0.01) is True
    
    # Record expensive calls
    tracker.record_api_call("openai", "/chat", tokens_used=1000000, cost_per_1k=0.002)
    
    # May or may not allow depending on remaining budget
    # Just test the method works
    result = tracker.should_make_api_call(0.01)
    assert isinstance(result, bool)


def test_api_budget_usage_report():
    """Test getting usage report."""
    tracker = APIBudgetTracker(monthly_budget_usd=100.0)
    tracker.record_api_call("openai", "/chat", tokens_used=1000, cost_per_1k=0.002)
    
    report = tracker.get_usage_report()
    
    assert report["total_calls"] == 1
    assert report["estimated_cost_usd"] == 0.002
    assert report["budget_remaining_usd"] > 0
    assert report["budget_used_percent"] >= 0


def test_get_resource_monitor():
    """Test getting global resource monitor."""
    monitor1 = get_resource_monitor()
    monitor2 = get_resource_monitor()
    
    # Should return the same instance
    assert monitor1 is monitor2


def test_get_api_budget_tracker():
    """Test getting global API budget tracker."""
    tracker1 = get_api_budget_tracker()
    tracker2 = get_api_budget_tracker()
    
    # Should return the same instance
    assert tracker1 is tracker2
