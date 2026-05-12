"""
Local Resource Monitor and Manager.

Inspired by cloud optimization principles:
- Minimize remote API calls (reduce costs)
- Optimize local disk I/O (like SSD optimization in the cloud)
- Monitor resource usage (avoid resource exhaustion)
- Efficient memory management
"""

import time
import os
import logging
from pathlib import Path
from typing import Dict, List, Optional, Tuple
from dataclasses import dataclass
from datetime import datetime

try:
    import rairos_sysinfo as _sysinfo
    _SYSINFO_AVAILABLE = True
except ImportError:
    _SYSINFO_AVAILABLE = False
    import psutil as _psutil_null  # type: ignore[import-untyped]

logger = logging.getLogger(__name__)


@dataclass
class ResourceStats:
    """Resource usage statistics."""

    timestamp: float
    cpu_percent: float
    memory_used_mb: float
    memory_percent: float
    disk_used_gb: float
    disk_percent: float
    disk_io_reads: int = 0
    disk_io_writes: int = 0
    network_sent_mb: float = 0.0
    network_recv_mb: float = 0.0


@dataclass
class DiskInfo:
    """Disk usage information."""

    path: Path
    total_gb: float
    used_gb: float
    free_gb: float
    percent: float


class ResourceMonitor:
    """Monitor local system resources."""

    def __init__(self, data_dir: Optional[Path] = None):
        self.data_dir = data_dir or self._get_default_data_dir()
        # Set disk/net io start to None since rairos_sysinfo stubs return zeros
        self._disk_io_start = None
        self._net_io_start = None
        self._history: List[ResourceStats] = []
        self._max_history = 1000  # Keep last 1000 samples

    @staticmethod
    def _get_default_data_dir() -> Path:
        """Get default data directory."""
        return Path.home() / ".cache" / "ai_research_os"

    def get_disk_info(self, path: Optional[Path] = None) -> DiskInfo:
        """Get disk usage information for a path."""
        target = path or self.data_dir
        si = _sysinfo.SysInfo()
        usage = si.disk_usage(str(target))
        return DiskInfo(
            path=target,
            total_gb=usage["total_gb"],
            used_gb=usage["used_gb"],
            free_gb=usage["free_gb"],
            percent=usage["percent"],
        )

    def get_memory_info(self) -> Tuple[float, float, float]:
        """Get memory information (used_mb, available_mb, percent)."""
        si = _sysinfo.SysInfo()
        mem = si.virtual_memory()
        return (
            mem["used_mb"],  # used_mb
            mem["available_mb"],  # available_mb
            mem["percent"],  # percent
        )

    def get_cpu_info(self) -> Tuple[float, int]:
        """Get CPU information (percent, count)."""
        si = _sysinfo.SysInfo()
        return (si.cpu_percent(0.1), os.cpu_count() or 1)

    def get_io_stats(self) -> Dict[str, float]:
        """Get disk I/O statistics (stub — disk I/O counters not available on all platforms)."""
        # sysinfo does not expose disk I/O counters on all platforms; return zeros
        return {"read_count": 0, "write_count": 0, "read_mb": 0.0, "write_mb": 0.0}

    def get_network_stats(self) -> Dict[str, float]:
        """Get network I/O statistics (stub — net I/O counters not available on all platforms)."""
        # sysinfo does not expose network I/O counters on all platforms; return zeros
        return {"sent_mb": 0.0, "recv_mb": 0.0}

    def collect_stats(self) -> ResourceStats:
        """Collect all resource statistics."""
        mem_used, mem_avail, mem_pct = self.get_memory_info()
        cpu_pct, cpu_count = self.get_cpu_info()
        disk_info = self.get_disk_info()
        io_stats = self.get_io_stats()
        net_stats = self.get_network_stats()

        stats = ResourceStats(
            timestamp=time.time(),
            cpu_percent=cpu_pct,
            memory_used_mb=mem_used,
            memory_percent=mem_pct,
            disk_used_gb=disk_info.used_gb,
            disk_percent=disk_info.percent,
            disk_io_reads=int(io_stats.get("read_count", 0)),
            disk_io_writes=int(io_stats.get("write_count", 0)),
            network_sent_mb=net_stats.get("sent_mb", 0.0),
            network_recv_mb=net_stats.get("recv_mb", 0.0),
        )

        # Store in history
        self._history.append(stats)
        if len(self._history) > self._max_history:
            self._history.pop(0)

        return stats

    def get_recent_stats(self, count: int = 10) -> List[ResourceStats]:
        """Get recent resource statistics."""
        return self._history[-count:]

    def get_average_stats(self, count: int = 100) -> Dict[str, float]:
        """Get average resource statistics over recent samples."""
        samples = self._history[-count:]
        if not samples:
            return {}

        return {
            "avg_cpu_percent": sum(s.cpu_percent for s in samples) / len(samples),
            "avg_memory_percent": sum(s.memory_percent for s in samples) / len(samples),
            "avg_disk_percent": sum(s.disk_percent for s in samples) / len(samples),
            "total_disk_io_reads": sum(s.disk_io_reads for s in samples),
            "total_disk_io_writes": sum(s.disk_io_writes for s in samples),
            "total_network_sent_mb": sum(s.network_sent_mb for s in samples),
            "total_network_recv_mb": sum(s.network_recv_mb for s in samples),
        }

    def get_resource_report(self) -> str:
        """Generate a human-readable resource report."""
        stats = self.collect_stats()
        avg_stats = self.get_average_stats()
        disk_info = self.get_disk_info()

        lines = [
            "=== Resource Usage Report ===",
            f"Timestamp: {datetime.fromtimestamp(stats.timestamp).isoformat()}",
            "",
            "Current Usage:",
            f"  CPU:     {stats.cpu_percent:.1f}%",
            f"  Memory:  {stats.memory_used_mb:.0f} MB ({stats.memory_percent:.1f}%)",
            f"  Disk:    {stats.disk_used_gb:.2f} GB ({stats.disk_percent:.1f}%)",
            "",
            "Disk Info:",
            f"  Total:   {disk_info.total_gb:.2f} GB",
            f"  Used:    {disk_info.used_gb:.2f} GB",
            f"  Free:    {disk_info.free_gb:.2f} GB",
            "",
            "Recent Averages (last 100 samples):",
            f"  CPU:     {avg_stats.get('avg_cpu_percent', 0):.1f}%",
            f"  Memory:  {avg_stats.get('avg_memory_percent', 0):.1f}%",
            f"  Disk:    {avg_stats.get('avg_disk_percent', 0):.1f}%",
            "",
            "I/O Statistics:",
            f"  Disk Reads:  {avg_stats.get('total_disk_io_reads', 0):,}",
            f"  Disk Writes: {avg_stats.get('total_disk_io_writes', 0):,}",
            f"  Network Sent:    {avg_stats.get('total_network_sent_mb', 0):.2f} MB",
            f"  Network Recv:   {avg_stats.get('total_network_recv_mb', 0):.2f} MB",
        ]

        return "\n".join(lines)


# Global resource monitor instance
_resource_monitor: Optional[ResourceMonitor] = None


def get_resource_monitor(data_dir: Optional[Path] = None) -> ResourceMonitor:
    """Get or create the global resource monitor."""
    global _resource_monitor
    if _resource_monitor is None:
        _resource_monitor = ResourceMonitor(data_dir)
    return _resource_monitor


class ResourceGuard:
    """Context manager to ensure resource availability before operations."""

    def __init__(
        self,
        min_disk_gb: float = 1.0,
        max_memory_percent: float = 90.0,
        monitor: Optional[ResourceMonitor] = None,
    ):
        self.min_disk_gb = min_disk_gb
        self.max_memory_percent = max_memory_percent
        self.monitor = monitor or get_resource_monitor()

    def check(self) -> Tuple[bool, str]:
        """Check if resources are available. Returns (ok, message)."""
        disk_info = self.monitor.get_disk_info()
        mem_used, mem_avail, mem_pct = self.monitor.get_memory_info()

        # Check disk space
        if disk_info.free_gb < self.min_disk_gb:
            return False, f"Low disk space: only {disk_info.free_gb:.2f} GB free"

        # Check memory
        if mem_pct > self.max_memory_percent:
            return False, f"High memory usage: {mem_pct:.1f}%"

        return True, "Resources OK"

    def __enter__(self):
        ok, msg = self.check()
        if not ok:
            raise ResourceWarning(msg)
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        pass


class APIBudgetTracker:
    """Track API usage and costs (inspired by minimizing cloud costs)."""

    def __init__(self, monthly_budget_usd: float = 100.0):
        self.monthly_budget_usd = monthly_budget_usd
        self.api_calls: List[Dict] = []
        self._call_count = 0
        self._cost_estimate = 0.0

    def record_api_call(
        self, provider: str, endpoint: str, tokens_used: int = 0, cost_per_1k: float = 0.0
    ):
        """Record an API call."""
        self._call_count += 1
        call_cost = (tokens_used / 1000) * cost_per_1k
        self._cost_estimate += call_cost

        self.api_calls.append(
            {
                "timestamp": time.time(),
                "provider": provider,
                "endpoint": endpoint,
                "tokens": tokens_used,
                "cost": call_cost,
                "total_cost": self._cost_estimate,
            }
        )

    def get_usage_report(self) -> Dict:
        """Get usage report."""
        return {
            "total_calls": self._call_count,
            "estimated_cost_usd": self._cost_estimate,
            "budget_remaining_usd": self.monthly_budget_usd - self._cost_estimate,
            "budget_used_percent": (self._cost_estimate / self.monthly_budget_usd) * 100
            if self.monthly_budget_usd > 0
            else 0,
            "recent_calls": len(self.api_calls[-100:]),  # Last 100 calls
        }

    def should_make_api_call(self, estimated_cost: float = 0.01) -> bool:
        """Check if we should make an API call based on budget."""
        return (self._cost_estimate + estimated_cost) <= self.monthly_budget_usd


# Global API budget tracker
_api_budget_tracker: Optional[APIBudgetTracker] = None


def get_api_budget_tracker(monthly_budget_usd: float = 100.0) -> APIBudgetTracker:
    """Get or create the global API budget tracker."""
    global _api_budget_tracker
    if _api_budget_tracker is None:
        _api_budget_tracker = APIBudgetTracker(monthly_budget_usd)
    return _api_budget_tracker
