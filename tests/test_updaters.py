"""Tests for updaters module (timeline and radar)."""
from __future__ import annotations

import tempfile
from pathlib import Path
from unittest.mock import patch

import pytest

from updaters.radar import (
    _heat,
    ensure_radar,
    flush_radar,
    parse_radar_table,
    render_radar,
    update_radar,
)
from updaters.timeline import ensure_timeline, update_timeline


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _mkp(base: Path, rel: str) -> Path:
    return base / rel


# ---------------------------------------------------------------------------
# Timeline
# ---------------------------------------------------------------------------

class TestEnsureTimeline:
    def test_creates_file_if_missing(self, tmp_path):
        root = tmp_path / "research"
        root.mkdir()
        p = ensure_timeline(root)
        assert p.exists()
        content = p.read_text(encoding="utf-8")
        assert "# Timeline" in content

    def test_returns_existing_file(self, tmp_path):
        root = tmp_path / "research"
        root.mkdir()
        existing = root / "00-Radar" / "Timeline.md"
        existing.parent.mkdir(parents=True, exist_ok=True)
        existing.write_text("# Timeline\n\ncustom content\n")
        p = ensure_timeline(root)
        assert p == existing
        assert "custom content" in p.read_text(encoding="utf-8")


class TestUpdateTimeline:
    def test_adds_new_year_section(self, tmp_path):
        root = tmp_path / "research"
        root.mkdir()
        p = ensure_timeline(root)  # create empty timeline

        note_path = _mkp(root, "01-Foundations/test-note.md")

        with patch("updaters.timeline.wikilink_for_pnote", return_value="[[test-note]]"):
            result = update_timeline(root, "2024", note_path, "Test Paper Title")

        content = result.read_text(encoding="utf-8")
        assert "## 2024" in content
        assert "[[test-note]]" in content
        assert "Test Paper Title" in content

    def test_appends_bullet_to_existing_year(self, tmp_path):
        root = tmp_path / "research"
        root.mkdir()
        p = ensure_timeline(root)

        note1 = _mkp(root, "01-Foundations/note1.md")
        note2 = _mkp(root, "01-Foundations/note2.md")

        with patch("updaters.timeline.wikilink_for_pnote", return_value="[[note1]]"):
            update_timeline(root, "2024", note1, "Paper One")
        with patch("updaters.timeline.wikilink_for_pnote", return_value="[[note2]]"):
            update_timeline(root, "2024", note2, "Paper Two")

        content = p.read_text(encoding="utf-8")
        assert content.count("[[note1]]") == 1
        assert content.count("[[note2]]") == 1
        assert "## 2024" in content

    def test_skips_duplicate_bullet(self, tmp_path):
        root = tmp_path / "research"
        root.mkdir()
        p = ensure_timeline(root)
        note_path = _mkp(root, "01-Foundations/dup.md")

        with patch("updaters.timeline.wikilink_for_pnote", return_value="[[dup]]"):
            update_timeline(root, "2024", note_path, "Paper")
            update_timeline(root, "2024", note_path, "Paper")

        content = p.read_text(encoding="utf-8")
        assert content.count("[[dup]]") == 1

    def test_handles_new_year_between_existing_years(self, tmp_path):
        """New year sections are appended at the end (not sorted in-place)."""
        root = tmp_path / "research"
        root.mkdir()
        p = ensure_timeline(root)

        note_2023 = _mkp(root, "01-Foundations/n2023.md")
        note_2025 = _mkp(root, "01-Foundations/n2025.md")
        note_2024 = _mkp(root, "01-Foundations/n2024.md")

        with patch("updaters.timeline.wikilink_for_pnote", return_value="[[n2023]]"):
            update_timeline(root, "2023", note_2023, "Paper 2023")
        with patch("updaters.timeline.wikilink_for_pnote", return_value="[[n2025]]"):
            update_timeline(root, "2025", note_2025, "Paper 2025")
        # 2024 appended last (update_timeline does not sort sections)
        with patch("updaters.timeline.wikilink_for_pnote", return_value="[[n2024]]"):
            update_timeline(root, "2024", note_2024, "Paper 2024")

        content = p.read_text(encoding="utf-8")
        assert "## 2023" in content
        assert "## 2024" in content
        assert "## 2025" in content
        # 2024 should appear after 2025 (appended at end)
        assert content.index("## 2025") < content.index("## 2024")


# ---------------------------------------------------------------------------
# Radar — parse_radar_table + render_radar
# ---------------------------------------------------------------------------

class TestParseRadarTable:
    def test_parses_valid_table(self):
        md = (
            "# Radar\n\n"
            "| 主题 | 热度 | 证据质量 | 成本变化 | 我的信心 | 最近更新 |\n"
            "| -- | -- | ---- | ---- | ---- | ---- |\n"
            "| LLM | 5 | 高 | 下降 | 中 | 2024-06-15 |\n"
            "| RL | 3 | 中 | 持平 | 低 | 2024-05-01 |\n"
        )
        header, rows = parse_radar_table(md)
        assert "# Radar" in header
        assert len(rows) == 2
        assert rows[0]["主题"] == "LLM"
        assert rows[0]["热度"] == "5"
        assert rows[1]["主题"] == "RL"
        assert rows[1]["最近更新"] == "2024-05-01"

    def test_handles_missing_or_empty_table(self):
        md = "# Radar\n\nNo table here.\n"
        header, rows = parse_radar_table(md)
        assert "No table" in header
        assert rows == []

    def test_skips_rows_with_fewer_than_6_cols(self):
        md = (
            "# Radar\n\n"
            "| 主题 | 热度 |\n"
            "| -- | -- |\n"
            "| LLM | 5 |  extra |\n"  # only 2 cols
        )
        _, rows = parse_radar_table(md)
        assert rows == []


class TestRenderRadar:
    def test_renders_header_and_rows(self):
        header = "# Radar\n"
        rows = [
            {"主题": "LLM", "热度": "5", "证据质量": "高", "成本变化": "下降", "我的信心": "中", "最近更新": "2024-06-15"},
        ]
        out = render_radar(header, rows)
        assert "| 主题 |" in out
        assert "| LLM | 5 | 高 | 下降 | 中 | 2024-06-15 |" in out

    def test_renders_multiple_rows_in_input_order(self):
        """render_radar preserves the order of rows passed in (sorting happens in update_radar)."""
        header = "# Radar\n"
        rows = [
            {"主题": "A", "热度": "2", "证据质量": "", "成本变化": "", "我的信心": "", "最近更新": ""},
            {"主题": "B", "热度": "10", "证据质量": "", "成本变化": "", "我的信心": "", "最近更新": ""},
            {"主题": "C", "热度": "5", "证据质量": "", "成本变化": "", "我的信心": "", "最近更新": ""},
        ]
        out = render_radar(header, rows)
        lines = out.splitlines()
        # Skip header, blank, column-header row, separator
        data_lines = lines[4:]
        topics = [l.split("|")[1].strip() for l in data_lines]
        # render_radar preserves input order (A, B, C); sorting is done by update_radar
        assert topics == ["A", "B", "C"]


# ---------------------------------------------------------------------------
# Radar — update_radar
# ---------------------------------------------------------------------------

class TestUpdateRadar:
    def test_creates_radar_file_if_missing(self, tmp_path):
        root = tmp_path / "research"
        root.mkdir()
        p = update_radar(root, ["LLM"], "2024-06-15", flush=True)
        assert p.exists()
        content = p.read_text(encoding="utf-8")
        assert "LLM" in content
        assert "热度" in content

    def test_increments_existing_tag_heat(self, tmp_path):
        root = tmp_path / "research"
        root.mkdir()
        # First update
        update_radar(root, ["LLM"], "2024-06-15", flush=True)
        # Second update — heat should go from "1" to "2"
        update_radar(root, ["LLM"], "2024-06-16", flush=True)

        content = (root / "00-Radar" / "Radar.md").read_text(encoding="utf-8")
        # LLM row should have heat "2"
        for line in content.splitlines():
            if "LLM" in line and line.startswith("|"):
                parts = [c.strip() for c in line.strip().strip("|").split("|")]
                if parts[0] == "LLM":
                    assert parts[1] == "2", f"Expected heat 2, got {parts[1]} in line: {line}"
                    assert parts[5] == "2024-06-16"
                    return
        pytest.fail("LLM row not found in radar")

    def test_adds_new_tag_to_existing_table(self, tmp_path):
        root = tmp_path / "research"
        root.mkdir()
        update_radar(root, ["LLM"], "2024-06-15", flush=True)
        update_radar(root, ["RL"], "2024-06-16", flush=True)

        content = (root / "00-Radar" / "Radar.md").read_text(encoding="utf-8")
        assert "LLM" in content
        assert "RL" in content

    def test_flush_false_accumulates_in_memory(self, tmp_path):
        root = tmp_path / "research"
        root.mkdir()
        ensure_radar(root)  # create empty file first

        update_radar(root, ["LLM"], "2024-06-15", flush=False)
        update_radar(root, ["LLM"], "2024-06-16", flush=False)
        # File should NOT be written yet (flush=False)
        content = (root / "00-Radar" / "Radar.md").read_text(encoding="utf-8")
        assert "LLM" not in content or content.count("LLM") == 0  # either empty or just the header

        # Now flush
        flush_radar(root)
        content = (root / "00-Radar" / "Radar.md").read_text(encoding="utf-8")
        assert "LLM" in content


# ---------------------------------------------------------------------------
# Radar — _heat helper
# ---------------------------------------------------------------------------

class TestHeatHelper:
    def test_returns_int_heat(self):
        r = {"热度": "42"}
        assert _heat(r) == 42

    def test_returns_zero_for_missing_key(self):
        r = {}
        assert _heat(r) == 0

    def test_returns_zero_for_non_numeric(self):
        r = {"热度": "abc"}
        assert _heat(r) == 0
