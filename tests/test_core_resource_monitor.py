"""Tests for core/resource_monitor.py — ResourceStats and DiskInfo dataclasses."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

from core.resource_monitor import ResourceStats, DiskInfo


class TestResourceStats:
    def test_creation(self):
        rs = ResourceStats(
            timestamp=1700000000.0,
            cpu_percent=45.5,
            memory_used_mb=8192.0,
            memory_percent=75.0,
            disk_used_gb=500.0,
            disk_percent=60.0,
        )
        assert rs.timestamp == 1700000000.0
        assert rs.cpu_percent == 45.5
        assert rs.memory_used_mb == 8192.0
        assert rs.memory_percent == 75.0
        assert rs.disk_used_gb == 500.0
        assert rs.disk_percent == 60.0

    def test_defaults(self):
        rs = ResourceStats(
            timestamp=1.0,
            cpu_percent=0.0,
            memory_used_mb=0.0,
            memory_percent=0.0,
            disk_used_gb=0.0,
            disk_percent=0.0,
        )
        assert rs.disk_io_reads == 0
        assert rs.disk_io_writes == 0
        assert rs.network_sent_mb == 0.0
        assert rs.network_recv_mb == 0.0

    def test_all_optional_fields(self):
        rs = ResourceStats(
            timestamp=1.0,
            cpu_percent=10.0,
            memory_used_mb=100.0,
            memory_percent=50.0,
            disk_used_gb=200.0,
            disk_percent=40.0,
            disk_io_reads=1000,
            disk_io_writes=500,
            network_sent_mb=50.5,
            network_recv_mb=123.4,
        )
        assert rs.disk_io_reads == 1000
        assert rs.disk_io_writes == 500
        assert rs.network_sent_mb == 50.5
        assert rs.network_recv_mb == 123.4

    def test_fields_are_correct_types(self):
        rs = ResourceStats(
            timestamp=1.0,
            cpu_percent=0.0,
            memory_used_mb=0.0,
            memory_percent=0.0,
            disk_used_gb=0.0,
            disk_percent=0.0,
        )
        assert isinstance(rs.timestamp, float)
        assert isinstance(rs.cpu_percent, float)
        assert isinstance(rs.memory_used_mb, float)
        assert isinstance(rs.memory_percent, float)
        assert isinstance(rs.disk_used_gb, float)
        assert isinstance(rs.disk_percent, float)


class TestDiskInfo:
    def test_creation(self):
        di = DiskInfo(
            path=Path("/"),
            total_gb=1000.0,
            used_gb=600.0,
            free_gb=400.0,
            percent=60.0,
        )
        assert di.path == Path("/")
        assert di.total_gb == 1000.0
        assert di.used_gb == 600.0
        assert di.free_gb == 400.0
        assert di.percent == 60.0

    def test_fields_are_correct_types(self):
        di = DiskInfo(
            path=Path("C:/"),
            total_gb=500.0,
            used_gb=250.0,
            free_gb=250.0,
            percent=50.0,
        )
        assert isinstance(di.path, Path)
        assert isinstance(di.total_gb, float)
        assert isinstance(di.used_gb, float)
        assert isinstance(di.free_gb, float)
        assert isinstance(di.percent, float)
