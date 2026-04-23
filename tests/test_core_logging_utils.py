"""Tests for logging and monitoring utilities."""
import pytest
import time
from core.logging_utils import (
    PerformanceMonitor,
    get_monitor,
)


def test_performance_monitor_record():
    """Test recording metrics."""
    monitor = PerformanceMonitor()
    monitor.record("test_metric", 1.5)
    monitor.record("test_metric", 2.5)
    
    stats = monitor.get_stats("test_metric")
    assert stats["count"] == 2
    assert stats["min"] == 1.5
    assert stats["max"] == 2.5
    assert stats["avg"] == 2.0


def test_performance_monitor_get_stats_empty():
    """Test getting stats for non-existent metric."""
    monitor = PerformanceMonitor()
    stats = monitor.get_stats("nonexistent")
    assert stats == {}


def test_performance_monitor_reset():
    """Test resetting all metrics."""
    monitor = PerformanceMonitor()
    monitor.record("metric1", 1.0)
    monitor.record("metric2", 2.0)
    
    monitor.reset()
    assert len(monitor.metrics) == 0


def test_performance_monitor_reset_metric():
    """Test resetting a specific metric."""
    monitor = PerformanceMonitor()
    monitor.record("metric1", 1.0)
    monitor.record("metric2", 2.0)
    
    monitor.reset_metric("metric1")
    assert "metric1" not in monitor.metrics
    assert "metric2" in monitor.metrics


def test_get_monitor():
    """Test getting global monitor."""
    monitor = get_monitor()
    assert isinstance(monitor, PerformanceMonitor)


def test_performance_monitor_multiple_records():
    """Test multiple records."""
    monitor = PerformanceMonitor()
    monitor.record("op1", 1.0)
    monitor.record("op1", 2.0)
    monitor.record("op2", 3.0)
    
    stats1 = monitor.get_stats("op1")
    assert stats1["count"] == 2
    assert stats1["total"] == 3.0
    
    stats2 = monitor.get_stats("op2")
    assert stats2["count"] == 1
    assert stats2["total"] == 3.0


def test_performance_monitor_all_stats():
    """Test get_all_stats method."""
    monitor = PerformanceMonitor()
    monitor.record("operation1", 1.5)
    monitor.record("operation2", 2.5)
    
    all_stats = monitor.get_all_stats()
    assert "operation1" in all_stats
    assert "operation2" in all_stats
    assert all_stats["operation1"]["count"] == 1
    assert all_stats["operation2"]["count"] == 1
