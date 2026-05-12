"""Tests for core/performance_guarantee.py — PerformanceGuarantee dataclass."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

from core.performance_guarantee import PerformanceGuarantee


class TestPerformanceGuarantee:
    def test_creation(self):
        pg = PerformanceGuarantee(
            name="CPU",
            promise="< 30% CPU",
            measured_impact=15.5,
            status="OK",
        )
        assert pg.name == "CPU"
        assert pg.promise == "< 30% CPU"
        assert pg.measured_impact == 15.5
        assert pg.status == "OK"

    def test_status_values(self):
        for status in ("OK", "WARNING", "CRITICAL"):
            pg = PerformanceGuarantee(name="X", promise="Y", measured_impact=0.0, status=status)
            assert pg.status == status

    def test_fields_are_correct_types(self):
        pg = PerformanceGuarantee(name="X", promise="Y", measured_impact=0.0, status="OK")
        assert isinstance(pg.name, str)
        assert isinstance(pg.promise, str)
        assert isinstance(pg.measured_impact, float)
        assert isinstance(pg.status, str)

    def test_measured_impact_range(self):
        pg = PerformanceGuarantee(name="X", promise="Y", measured_impact=0.0, status="OK")
        assert pg.measured_impact == 0.0
        pg2 = PerformanceGuarantee(name="X", promise="Y", measured_impact=99.9, status="CRITICAL")
        assert pg2.measured_impact == 99.9
