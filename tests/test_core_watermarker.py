"""Tests for core/watermarker.py."""
import sys
from pathlib import Path
from unittest.mock import patch
import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))
from core.watermarker import WaterMarker, get_marker


class TestWaterMarker:
    def setup_method(self):
        """Reset singleton before each test."""
        import core.watermarker as wm
        wm._marker = None

    def teardown_method(self):
        """Reset singleton after each test."""
        import core.watermarker as wm
        wm._marker = None

    def test_add_mark(self):
        marker = WaterMarker()
        marker.add_mark("info", "test content")
        marks = marker.get_marks()
        assert len(marks) == 1
        assert marks[0]["type"] == "info"
        assert marks[0]["content"] == "test content"
        assert "time" in marks[0]

    def test_add_multiple_marks(self):
        marker = WaterMarker()
        marker.add_mark("info", "first")
        marker.add_mark("warn", "second")
        marker.add_mark("error", "third")
        assert len(marker.get_marks()) == 3
        assert marker.get_marks()[0]["type"] == "info"
        assert marker.get_marks()[2]["content"] == "third"

    def test_get_marks_returns_list(self):
        marker = WaterMarker()
        marker.add_mark("info", "test")
        marks = marker.get_marks()
        assert isinstance(marks, list)

    def test_clear(self):
        marker = WaterMarker()
        marker.add_mark("info", "test")
        assert len(marker.get_marks()) == 1
        marker.clear()
        assert len(marker.get_marks()) == 0

    def test_mark_contains_time_isoformat(self):
        marker = WaterMarker()
        marker.add_mark("info", "test")
        mark = marker.get_marks()[0]
        assert "time" in mark
        # ISO format: contains T
        assert "T" in mark["time"]

    def test_empty_marks(self):
        marker = WaterMarker()
        assert marker.get_marks() == []


class TestGetMarker:
    def setup_method(self):
        import core.watermarker as wm
        wm._marker = None

    def teardown_method(self):
        import core.watermarker as wm
        wm._marker = None

    def test_get_marker_returns_singleton(self):
        m1 = get_marker()
        m2 = get_marker()
        assert m1 is m2

    def test_get_marker_returns_watermarker_instance(self):
        marker = get_marker()
        assert isinstance(marker, WaterMarker)

    def test_singleton_persists_across_calls(self):
        m1 = get_marker()
        m1.add_mark("info", "persisted")
        m2 = get_marker()
        assert len(m2.get_marks()) == 1
        assert m2.get_marks()[0]["content"] == "persisted"

    def test_singleton_after_clear(self):
        m1 = get_marker()
        m1.add_mark("info", "before clear")
        m1.clear()
        m2 = get_marker()
        assert len(m2.get_marks()) == 0
        assert m1 is m2
