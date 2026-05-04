"""Tests for research_loop.snapstate — session persistence."""

from __future__ import annotations

import json
import time
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest

from research_loop.snapstate import (
    GapSnapshot,
    PaperSnapshot,
    ResearchSession,
    Snapstate,
)


# ---------------------------------------------------------------------------
# PaperSnapshot
# ---------------------------------------------------------------------------
class TestPaperSnapshot:
    def test_creation(self):
        ps = PaperSnapshot(arxiv_id="2106.09685", title="LoRA", abstract="abs")
        assert ps.arxiv_id == "2106.09685"
        assert ps.title == "LoRA"

    def test_defaults(self):
        ps = PaperSnapshot(arxiv_id="x")
        assert ps.title == ""
        assert ps.abstract == ""
        assert ps.keywords == []

    def test_to_dict_round_trip(self):
        ps = PaperSnapshot(arxiv_id="1234", title="T", abstract="A", keywords=["k1"])
        d = ps.to_dict()
        ps2 = PaperSnapshot.from_dict(d)
        assert ps2.arxiv_id == "1234"
        assert ps2.keywords == ["k1"]

    def test_from_dict_missing_fields(self):
        ps = PaperSnapshot.from_dict({"arxiv_id": "x"})
        assert ps.title == ""

    def test_from_dict_extra_fields_ignored(self):
        ps = PaperSnapshot.from_dict({"arxiv_id": "x", "extra": True})
        assert ps.arxiv_id == "x"


# ---------------------------------------------------------------------------
# GapSnapshot
# ---------------------------------------------------------------------------
class TestGapSnapshot:
    def test_creation(self):
        gs = GapSnapshot(gap_type="method_limitation", title="Gap", description="d")
        assert gs.gap_type == "method_limitation"
        assert gs.accepted is False

    def test_defaults(self):
        gs = GapSnapshot(gap_type="improvement", title="G")
        assert gs.description == ""
        assert gs.archetype_match == 0.0

    def test_to_dict_round_trip(self):
        gs = GapSnapshot(
            gap_type="improvement",
            title="T",
            description="D",
            accepted=True,
            archetype_match=0.8,
        )
        d = gs.to_dict()
        gs2 = GapSnapshot.from_dict(d)
        assert gs2.accepted is True
        assert gs2.archetype_match == 0.8


# ---------------------------------------------------------------------------
# ResearchSession
# ---------------------------------------------------------------------------
class TestResearchSession:
    def test_creation(self):
        rs = ResearchSession(session_id="s1", query="test query")
        assert rs.session_id == "s1"
        assert rs.query == "test query"
        assert rs.iterations == []

    def test_to_dict_round_trip(self):
        rs = ResearchSession(session_id="s1", query="q")
        d = rs.to_dict()
        assert "session_id" in d
        assert "created_at" in d
        rs2 = ResearchSession.from_dict(d)
        assert rs2.session_id == "s1"

    def test_duration(self):
        rs = ResearchSession(
            session_id="s1",
            query="q",
            created_at=100.0,
            updated_at=110.0,
        )
        assert rs.duration == pytest.approx(10.0, abs=0.1)


# ---------------------------------------------------------------------------
# Snapstate
# ---------------------------------------------------------------------------
class TestSnapstate:
    def test_init_creates_dir(self, tmp_path):
        s = Snapstate(base_dir=tmp_path / "sessions")
        assert (tmp_path / "sessions").exists()

    def test_save_and_load(self, tmp_path):
        s = Snapstate(base_dir=tmp_path)
        rs = ResearchSession(session_id="s1", query="test")
        s.save(rs)
        loaded = s.load("s1")
        assert loaded is not None
        assert loaded.session_id == "s1"
        assert loaded.query == "test"

    def test_load_nonexistent(self, tmp_path):
        s = Snapstate(base_dir=tmp_path)
        assert s.load("nope") is None

    def test_list_sessions_empty(self, tmp_path):
        s = Snapstate(base_dir=tmp_path)
        assert s.list_sessions() == []

    def test_list_sessions(self, tmp_path):
        s = Snapstate(base_dir=tmp_path)
        s.save(ResearchSession(session_id="a", query="q1"))
        s.save(ResearchSession(session_id="b", query="q2"))
        sessions = s.list_sessions()
        assert len(sessions) == 2

    def test_delete(self, tmp_path):
        s = Snapstate(base_dir=tmp_path)
        s.save(ResearchSession(session_id="s1", query="q"))
        assert s.delete("s1") is True
        assert s.load("s1") is None

    def test_delete_nonexistent(self, tmp_path):
        s = Snapstate(base_dir=tmp_path)
        assert s.delete("nope") is False

    def test_load_latest_empty(self, tmp_path):
        s = Snapstate(base_dir=tmp_path)
        assert s.load_latest() is None

    def test_load_latest(self, tmp_path):
        s = Snapstate(base_dir=tmp_path)
        s.save(ResearchSession(session_id="a", query="q1"))
        time.sleep(0.01)
        s.save(ResearchSession(session_id="b", query="q2"))
        latest = s.load_latest()
        assert latest is not None
        assert latest.session_id == "b"

    def test_overwrite(self, tmp_path):
        s = Snapstate(base_dir=tmp_path)
        s.save(ResearchSession(session_id="s1", query="old"))
        s.save(ResearchSession(session_id="s1", query="new"))
        loaded = s.load("s1")
        assert loaded.query == "new"

    def test_load_corrupt_json(self, tmp_path):
        s = Snapstate(base_dir=tmp_path)
        (tmp_path / "s1.json").write_text("NOT JSON {{{", encoding="utf-8")
        assert s.load("s1") is None

    def test_unicode_content(self, tmp_path):
        s = Snapstate(base_dir=tmp_path)
        rs = ResearchSession(session_id="s1", query="中文测试 🚀")
        s.save(rs)
        loaded = s.load("s1")
        assert loaded.query == "中文测试 🚀"
