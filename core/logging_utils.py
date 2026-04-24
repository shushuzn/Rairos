"""Logging and monitoring utilities for AI Research OS."""
import logging
import time
from functools import wraps
from typing import Callable, Any, Dict
from contextlib import contextmanager

# Configure module logger
logger = logging.getLogger(__name__)


class PerformanceMonitor:
    """Monitor and track performance metrics."""

    def __init__(self):
        self.metrics: Dict[str, list] = {}

    def record(self, name: str, value: float) -> None:
        """Record a metric value."""
        if name not in self.metrics:
            self.metrics[name] = []
        self.metrics[name].append(value)

    def get_stats(self, name: str) -> Dict[str, float]:
        """Get statistics for a metric."""
        if name not in self.metrics or not self.metrics[name]:
            return {}

        values = self.metrics[name]
        return {
            "count": len(values),
            "min": min(values),
            "max": max(values),
            "avg": sum(values) / len(values),
            "total": sum(values),
        }

    def get_all_stats(self) -> Dict[str, Dict[str, float]]:
        """Get statistics for all metrics."""
        return {name: self.get_stats(name) for name in self.metrics}

    def reset(self) -> None:
        """Reset all metrics."""
        self.metrics.clear()

    def reset_metric(self, name: str) -> None:
        """Reset a specific metric."""
        if name in self.metrics:
            del self.metrics[name]


# Global performance monitor instance
_monitor = PerformanceMonitor()


def get_monitor() -> PerformanceMonitor:
    """Get the global performance monitor."""
    return _monitor


@contextmanager
def track_time(name: str):
    """Context manager to track execution time of a code block."""
    start = time.time()
    try:
        yield
    finally:
        duration = time.time() - start
        _monitor.record(name, duration)
        logger.debug(f"{name} took {duration:.3f}s")


def timed(func: Callable) -> Callable:
    """Decorator to track function execution time."""
    @wraps(func)
    def wrapper(*args, **kwargs) -> Any:
        start = time.time()
        try:
            result = func(*args, **kwargs)
            return result
        finally:
            duration = time.time() - start
            metric_name = f"{func.__module__}.{func.__name__}"
            _monitor.record(metric_name, duration)
            logger.debug(f"{metric_name} took {duration:.3f}s")
    return wrapper


def get_performance_report() -> str:
    """Generate a performance report for all tracked metrics."""
    stats = _monitor.get_all_stats()
    if not stats:
        return "No performance metrics recorded."

    lines = ["=== Performance Report ===", ""]
    for name, metric_stats in stats.items():
        if metric_stats:
            lines.append(f"{name}:")
            lines.append(f"  Count: {metric_stats['count']}")
            lines.append(f"  Avg:   {metric_stats['avg']:.3f}s")
            lines.append(f"  Min:   {metric_stats['min']:.3f}s")
            lines.append(f"  Max:   {metric_stats['max']:.3f}s")
            lines.append(f"  Total: {metric_stats['total']:.3f}s")
            lines.append("")

    return "\n".join(lines)


def setup_logging(level: str = "INFO", log_file: str = None) -> None:  # type: ignore[assignment]
    """Setup logging configuration."""
    log_level = getattr(logging, level.upper(), logging.INFO)

    handlers = [logging.StreamHandler()]
    if log_file:
        handlers.append(logging.FileHandler(log_file))  # type: ignore[arg-type]

    logging.basicConfig(
        level=log_level,
        format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
        handlers=handlers
    )

    # Set library loggers to WARNING to reduce noise
    logging.getLogger("urllib3").setLevel(logging.WARNING)
    logging.getLogger("requests").setLevel(logging.WARNING)
