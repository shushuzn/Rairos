"""
Performance Profiler and Analysis Tools.

Helps identify performance bottlenecks and optimize critical paths.
"""
import time
import functools
import logging
from typing import Callable, Dict, List, Optional, Any
from dataclasses import dataclass

from contextlib import contextmanager
import cProfile
import pstats
import io

logger = logging.getLogger(__name__)


@dataclass
class FunctionProfile:
    """Profile data for a single function."""
    name: str
    call_count: int = 0
    total_time: float = 0.0
    min_time: float = float('inf')
    max_time: float = 0.0
    avg_time: float = 0.0
    last_called: float = 0.0

    def update(self, elapsed: float):
        """Update profile with new timing data."""
        self.call_count += 1
        self.total_time += elapsed
        self.min_time = min(self.min_time, elapsed)
        self.max_time = max(self.max_time, elapsed)
        self.avg_time = self.total_time / self.call_count
        self.last_called = time.time()


class PerformanceProfiler:
    """Track and analyze function performance."""

    def __init__(self):
        self._profiles: Dict[str, FunctionProfile] = {}
        self._enabled = True
        self._start_time = time.time()

    def enable(self):
        """Enable profiling."""
        self._enabled = True

    def disable(self):
        """Disable profiling."""
        self._enabled = False

    def profile_function(self, name: Optional[str] = None):
        """Decorator to profile a function."""
        def decorator(func: Callable) -> Callable:
            profile_name = name or f"{func.__module__}.{func.__qualname__}"

            @functools.wraps(func)
            def wrapper(*args, **kwargs):
                if not self._enabled:
                    return func(*args, **kwargs)

                start = time.perf_counter()
                try:
                    result = func(*args, **kwargs)
                    return result
                finally:
                    elapsed = time.perf_counter() - start
                    self._record_call(profile_name, elapsed)

            return wrapper
        return decorator

    def _record_call(self, name: str, elapsed: float):
        """Record a function call."""
        if name not in self._profiles:
            self._profiles[name] = FunctionProfile(name=name)
        self._profiles[name].update(elapsed)

    @contextmanager
    def profile_block(self, name: str):
        """Context manager to profile a code block."""
        if not self._enabled:
            yield
            return

        start = time.perf_counter()
        try:
            yield
        finally:
            elapsed = time.perf_counter() - start
            self._record_call(name, elapsed)

    def get_profile(self, name: str) -> Optional[FunctionProfile]:
        """Get profile for a specific function."""
        return self._profiles.get(name)

    def get_all_profiles(self) -> List[FunctionProfile]:
        """Get all profiles sorted by total time."""
        profiles = list(self._profiles.values())
        return sorted(profiles, key=lambda p: p.total_time, reverse=True)

    def get_slowest_functions(self, count: int = 10) -> List[FunctionProfile]:
        """Get the slowest functions."""
        return self.get_all_profiles()[:count]

    def get_most_called(self, count: int = 10) -> List[FunctionProfile]:
        """Get the most frequently called functions."""
        profiles = list(self._profiles.values())
        return sorted(profiles, key=lambda p: p.call_count, reverse=True)[:count]

    def get_report(self) -> str:
        """Generate a performance report."""
        lines = [
            "=" * 70,
            "PERFORMANCE PROFILE REPORT",
            "=" * 70,
            "",
            f"Total profiling time: {time.time() - self._start_time:.2f}s",
            f"Total functions tracked: {len(self._profiles)}",
            "",
            "TOP 10 SLOWEST FUNCTIONS (by total time):",
            "-" * 70,
        ]

        slowest = self.get_slowest_functions(10)
        for i, profile in enumerate(slowest, 1):
            lines.append(
                f"{i:2d}. {profile.name}"
            )
            lines.append(
                f"    Total: {profile.total_time:.3f}s | "
                f"Calls: {profile.call_count} | "
                f"Avg: {profile.avg_time*1000:.2f}ms | "
                f"Min: {profile.min_time*1000:.2f}ms | "
                f"Max: {profile.max_time*1000:.2f}ms"
            )

        lines.append("")
        lines.append("TOP 10 MOST CALLED FUNCTIONS:")
        lines.append("-" * 70)

        most_called = self.get_most_called(10)
        for i, profile in enumerate(most_called, 1):
            lines.append(
                f"{i:2d}. {profile.name} - {profile.call_count} calls "
                f"({profile.total_time:.3f}s total)"
            )

        lines.append("")
        lines.append("=" * 70)

        return "\n".join(lines)

    def reset(self):
        """Reset all profiling data."""
        self._profiles.clear()
        self._start_time = time.time()

    def get_stats_dict(self) -> Dict[str, Any]:
        """Get profiling statistics as a dictionary."""
        return {
            "total_functions": len(self._profiles),
            "total_time": time.time() - self._start_time,
            "slowest": [
                {
                    "name": p.name,
                    "total_time": p.total_time,
                    "call_count": p.call_count,
                    "avg_time": p.avg_time,
                    "min_time": p.min_time,
                    "max_time": p.max_time,
                }
                for p in self.get_slowest_functions(10)
            ],
            "most_called": [
                {
                    "name": p.name,
                    "call_count": p.call_count,
                    "total_time": p.total_time,
                }
                for p in self.get_most_called(10)
            ]
        }


# Global profiler instance
_profiler: Optional[PerformanceProfiler] = None


def get_profiler() -> PerformanceProfiler:
    """Get the global performance profiler."""
    global _profiler
    if _profiler is None:
        _profiler = PerformanceProfiler()
    return _profiler


def profile(name: Optional[str] = None) -> Callable:
    """Decorator to profile a function using the global profiler."""
    return get_profiler().profile_function(name)  # type: ignore[no-any-return]


@contextmanager
def profile_block(name: str):
    """Context manager to profile a code block."""
    with get_profiler().profile_block(name):
        yield


class ProfilerContext:
    """Context manager for cProfile-based profiling."""

    def __init__(self, enabled: bool = True):
        self.enabled = enabled
        self.profiler = cProfile.Profile()
        self.stats: Optional[pstats.Stats] = None

    def __enter__(self):
        if self.enabled:
            self.profiler.enable()
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        if self.enabled:
            self.profiler.disable()

    def get_stats(self, sort_by: str = 'cumulative', limit: int = 20) -> str:
        """Get profiling stats as string."""
        s = io.StringIO()
        ps = pstats.Stats(self.profiler, stream=s)
        ps.sort_stats(sort_by)
        ps.print_stats(limit)
        return s.getvalue()


@contextmanager
def cprofile_block(sort_by: str = 'cumulative', limit: int = 20):
    """Context manager for cProfile-based profiling."""
    with ProfilerContext() as ctx:
        yield ctx
    print(ctx.get_stats(sort_by, limit))


class MemoryProfiler:
    """Track memory usage of operations."""

    def __init__(self):
        self._snapshots: List[Dict[str, Any]] = []
        self._start_memory = self._get_current_memory()

    @staticmethod
    def _get_current_memory() -> float:
        """Get current memory usage in MB."""
        try:
            import psutil
            process = psutil.Process()
            return process.memory_info().rss / (1024 * 1024)  # type: ignore[no-any-return]
        except (ImportError, OSError):
            return 0.0

    @contextmanager
    def track(self, name: str):
        """Track memory usage of a code block."""
        start_mem = self._get_current_memory()
        start_time = time.time()

        yield

        end_mem = self._get_current_memory()
        elapsed = time.time() - start_time

        snapshot = {
            "name": name,
            "start_memory_mb": start_mem,
            "end_memory_mb": end_mem,
            "memory_delta_mb": end_mem - start_mem,
            "elapsed_time": elapsed,
            "timestamp": time.time(),
        }

        self._snapshots.append(snapshot)

        if float(snapshot["memory_delta_mb"]) > 1.0:  # type: ignore[arg-type]  # Log if > 1MB increase
            logger.warning(
                f"Memory increase in '{name}': "
                f"+{snapshot['memory_delta_mb']:.2f} MB "
                f"({start_mem:.1f} MB -> {end_mem:.1f} MB)"
            )

    def get_snapshots(self) -> List[Dict[str, Any]]:
        """Get all memory snapshots."""
        return self._snapshots.copy()

    def get_peak_increase(self) -> float:
        """Get peak memory increase."""
        if not self._snapshots:
            return 0.0
        return max(float(s["memory_delta_mb"]) for s in self._snapshots)

    def get_report(self) -> str:
        """Generate memory profiling report."""
        if not self._snapshots:
            return "No memory snapshots recorded."

        lines = [
            "=" * 70,
            "MEMORY PROFILING REPORT",
            "=" * 70,
            "",
        ]

        for snapshot in self._snapshots:
            delta = snapshot["memory_delta_mb"]
            delta_str = f"+{delta:.2f}" if delta >= 0 else f"{delta:.2f}"

            lines.append(
                f"{snapshot['name']}: {delta_str} MB "
                f"({snapshot['start_memory_mb']:.1f} -> {snapshot['end_memory_mb']:.1f})"
            )

        lines.append("")
        lines.append(f"Peak increase: +{self.get_peak_increase():.2f} MB")
        lines.append("")
        lines.append("=" * 70)

        return "\n".join(lines)


# Global memory profiler
_memory_profiler: Optional[MemoryProfiler] = None


def get_memory_profiler() -> MemoryProfiler:
    """Get the global memory profiler."""
    global _memory_profiler
    if _memory_profiler is None:
        _memory_profiler = MemoryProfiler()
    return _memory_profiler
