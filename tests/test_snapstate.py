"""Comprehensive tests for research_loop.snapstate module.
Covers:
  - PaperSnapshot / GapSnapshot / ResearchSession dataclass creation & defaults
  - to_dict / from_dict round-trips
  - Snapstate save / load / list / delete with tmp_path
  - new_session creation (with mock)
  - load_latest behavior
  - Edge cases: corrupt files, empty dir, missing files, unicode
NOTE: This project uses an autouse freeze_time fixture (conftest.py) that
freezes time.time() to 2024-06-15. However, ResearchSession.default_factory
captures the real time.time function at class-definition time, so timestamps
created during __init__ use real wall-clock time. Tests that compare these
two time sources are marked @pytest.mark.no_freeze.
"""

from __future__ import annotations
import json
import time
import sys
from pathlib import Path
from unittest.mock import patch, MagicMock
import pytest

# ---------------------------------------------------------------------------
# Ensure the project root is on sys.path so `research_loop.snapstate` resolves
# ---------------------------------------------------------------------------
_REPO_ROOT = str(Path(__file__).resolve().parent.parent)
if _REPO_ROOT not in sys.path:
    sys.path.insert(0, _REPO_ROOT)
from research_loop.snapstate import (
    PaperSnapshot,
    GapSnapshot,
    ResearchSession,
    Snapstate,
)


# =========================================================================
# Helpers
# =========================================================================
def _make_paper(**overrides) -> PaperSnapshot:
    defaults = dict(
        arxiv_id="2301.00001",
        title="Test Paper",
        abstract="An abstract.",
        url="https://arxiv.org/abs/2301.00001",
    )
    defaults.update(overrides)
    return PaperSnapshot(**defaults)


def _make_gap(**overrides) -> GapSnapshot:
    defaults = dict(
        gap_type="methodological",
        title="Some Gap",
        description="There is a gap here.",
    )
    defaults.update(overrides)
    return GapSnapshot(**defaults)


def _make_session(**overrides) -> ResearchSession:
    defaults = dict(
        session_id="abc12345",
        query="test query",
    )
    defaults.update(overrides)
    return ResearchSession(**defaults)


# =========================================================================
# 1. PaperSnapshot
# =========================================================================
class TestPaperSnapshot:
    def test_creation_with_required_fields(self):
        p = _make_paper()
        assert p.arxiv_id == "2301.00001"
        assert p.title == "Test Paper"
        assert p.abstract == "An abstract."
        assert p.url == "https://arxiv.org/abs/2301.00001"
        # default fields
        assert p.extracted_text == ""
        assert p.summary == ""
        assert p.gaps_found == []
        assert p.notes == ""

    def test_creation_with_all_fields(self):
        p = _make_paper(
            extracted_text="Full text here",
            summary="A short summary",
            gaps_found=["g1", "g2"],
            notes="my notes",
        )
        assert p.extracted_text == "Full text here"
        assert p.summary == "A short summary"
        assert p.gaps_found == ["g1", "g2"]
        assert p.notes == "my notes"

    def test_to_dict(self):
        p = _make_paper(gaps_found=["g1"])
        d = p.to_dict()
        assert isinstance(d, dict)
        assert d["arxiv_id"] == "2301.00001"
        assert d["gaps_found"] == ["g1"]
        assert set(d.keys()) == {
            "arxiv_id",
            "title",
            "abstract",
            "url",
            "extracted_text",
            "summary",
            "gaps_found",
            "notes",
        }

    def test_from_dict_round_trip(self):
        original = _make_paper(
            extracted_text="text", summary="sum", gaps_found=["a"], notes="n"
        )
        d = original.to_dict()
        restored = PaperSnapshot.from_dict(d)
        assert restored == original

    def test_from_dict_returns_same_type(self):
        d = _make_paper().to_dict()
        assert isinstance(PaperSnapshot.from_dict(d), PaperSnapshot)


# =========================================================================
# 2. GapSnapshot
# =========================================================================
class TestGapSnapshot:
    def test_creation_with_required_fields(self):
        g = _make_gap()
        assert g.gap_type == "methodological"
        assert g.title == "Some Gap"
        assert g.description == "There is a gap here."
        assert g.matched_papers == []
        assert g.archetype_match == 0.0
        assert g.accepted is False

    def test_creation_with_all_fields(self):
        g = _make_gap(
            matched_papers=["2301.00001", "2301.00002"],
            archetype_match=0.85,
            accepted=True,
        )
        assert g.matched_papers == ["2301.00001", "2301.00002"]
        assert g.archetype_match == 0.85
        assert g.accepted is True

    def test_to_dict(self):
        g = _make_gap(matched_papers=["x"], archetype_match=1.0, accepted=True)
        d = g.to_dict()
        assert isinstance(d, dict)
        assert d["gap_type"] == "methodological"
        assert d["matched_papers"] == ["x"]
        assert d["archetype_match"] == 1.0
        assert d["accepted"] is True

    def test_from_dict_round_trip(self):
        original = _make_gap(matched_papers=["p1"], archetype_match=0.5, accepted=True)
        restored = GapSnapshot.from_dict(original.to_dict())
        assert restored == original

    def test_from_dict_returns_same_type(self):
        d = _make_gap().to_dict()
        assert isinstance(GapSnapshot.from_dict(d), GapSnapshot)


# =========================================================================
# 3. ResearchSession
# =========================================================================
class TestResearchSession:
    def test_creation_with_required_fields(self):
        s = _make_session()
        assert s.session_id == "abc12345"
        assert s.query == "test query"
        # defaults
        assert s.iteration == 0
        assert s.max_iterations == 3
        assert s.papers == []
        assert s.gaps == []
        assert s.search_history == []
        assert s.hypotheses == []
        assert s.findings == []
        assert s.reflections == []
        assert s.archetype == {}
        assert s.status == "running"
        assert s.error == ""

    def test_creation_with_all_fields(self):
        s = _make_session(
            iteration=2,
            max_iterations=5,
            papers=[_make_paper()],
            gaps=[_make_gap()],
            search_history=["q1", "q2"],
            hypotheses=["h1"],
            findings=["f1"],
            reflections=["r1"],
            archetype={"dim1": 0.7},
            status="paused",
            error="some error",
        )
        assert s.iteration == 2
        assert s.max_iterations == 5
        assert len(s.papers) == 1
        assert len(s.gaps) == 1
        assert s.search_history == ["q1", "q2"]
        assert s.hypotheses == ["h1"]
        assert s.findings == ["f1"]
        assert s.reflections == ["r1"]
        assert s.archetype == {"dim1": 0.7}
        assert s.status == "paused"
        assert s.error == "some error"

    @pytest.mark.no_freeze
    def test_to_dict_updates_updated_at(self):
        """to_dict() refreshes updated_at to current time (must use real clock)."""
        s = _make_session()
        old_updated = s.updated_at
        time.sleep(0.01)
        d = s.to_dict()
        assert d["updated_at"] >= old_updated
        # The dict should contain all key fields
        assert "session_id" in d
        assert "papers" in d
        assert "gaps" in d

    def test_to_dict_contains_nested_dataclass_fields(self):
        p = _make_paper()
        g = _make_gap()
        s = _make_session(papers=[p], gaps=[g])
        d = s.to_dict()
        assert isinstance(d["papers"], list)
        assert d["papers"][0]["arxiv_id"] == "2301.00001"
        assert isinstance(d["gaps"], list)
        assert d["gaps"][0]["gap_type"] == "methodological"

    @pytest.mark.no_freeze
    def test_from_dict_pops_updated_at(self):
        """from_dict should remove updated_at before constructing, so the
        field gets its default_factory value (real time.time())."""
        s = _make_session()
        d = s.to_dict()
        d["updated_at"] = 9999999.0
        restored = ResearchSession.from_dict(d)
        # updated_at was popped, so the field gets default_factory (real clock).
        # It should be close to now, not 9999999.0
        assert restored.updated_at > 1_000_000_000  # sensible timestamp
        assert abs(restored.updated_at - time.time()) < 2  # close to now
        assert restored.session_id == "abc12345"

    def test_from_dict_round_trip_with_nested_objects(self):
        """Round-trip preserves data; nested PaperSnapshot/GapSnapshot become
        plain dicts after to_dict->from_dict (asdict flattens them)."""
        p = _make_paper()
        g = _make_gap()
        original = _make_session(
            papers=[p],
            gaps=[g],
            search_history=["q1"],
            hypotheses=["h1"],
            status="completed",
        )
        d = original.to_dict()
        restored = ResearchSession.from_dict(d)
        # Scalar fields match
        assert restored.session_id == original.session_id
        assert restored.query == original.query
        assert restored.iteration == original.iteration
        assert restored.max_iterations == original.max_iterations
        assert restored.status == original.status
        assert restored.search_history == original.search_history
        assert restored.hypotheses == original.hypotheses
        # Nested objects become plain dicts (asdict behavior)
        assert len(restored.papers) == 1
        assert restored.papers[0]["arxiv_id"] == "2301.00001"
        assert len(restored.gaps) == 1
        assert restored.gaps[0]["gap_type"] == "methodological"

    @pytest.mark.no_freeze
    def test_duration_returns_positive_number(self):
        """duration() = real time.time() - created_at (both real clock)."""
        s = _make_session()
        d = s.duration()
        assert isinstance(d, float)
        assert d >= 0.0


# =========================================================================
# 4. Snapstate -- basic init
# =========================================================================
class TestSnapstateInit:
    def test_creates_directory(self, tmp_path):
        sessions_dir = tmp_path / "sessions"
        ss = Snapstate(base_dir=sessions_dir)
        assert sessions_dir.exists()
        assert ss.base_dir == sessions_dir

    def test_default_base_dir_when_none(self):
        """When base_dir is None, it defaults to ~/.ai_research_os/sessions."""
        ss = Snapstate(base_dir=None)
        expected = Path.home() / ".ai_research_os" / "sessions"
        assert ss.base_dir == expected

    def test_idempotent_mkdir(self, tmp_path):
        """Calling init twice on existing dir doesn't raise."""
        d = tmp_path / "d"
        Snapstate(base_dir=d)
        Snapstate(base_dir=d)
        assert d.exists()


# =========================================================================
# 5. Snapstate -- save & load
# =========================================================================
class TestSnapstateSaveLoad:
    def test_save_returns_path(self, tmp_path):
        ss = Snapstate(base_dir=tmp_path)
        s = _make_session()
        path = ss.save(s)
        assert isinstance(path, Path)
        assert path.exists()
        assert path.suffix == ".json"

    def test_save_creates_json_file(self, tmp_path):
        ss = Snapstate(base_dir=tmp_path)
        s = _make_session(query="hello world")
        ss.save(s)
        json_files = list(tmp_path.glob("*.json"))
        assert len(json_files) == 1
        data = json.loads(json_files[0].read_text(encoding="utf-8"))
        assert data["session_id"] == "abc12345"
        assert data["query"] == "hello world"

    def test_save_no_tmp_files_left(self, tmp_path):
        ss = Snapstate(base_dir=tmp_path)
        ss.save(_make_session())
        tmp_files = list(tmp_path.glob("*.tmp"))
        assert len(tmp_files) == 0

    def test_load_existing_session(self, tmp_path):
        ss = Snapstate(base_dir=tmp_path)
        s = _make_session()
        ss.save(s)
        loaded = ss.load("abc12345")
        assert loaded is not None
        assert loaded.session_id == "abc12345"
        assert loaded.query == "test query"

    def test_load_nonexistent_returns_none(self, tmp_path):
        ss = Snapstate(base_dir=tmp_path)
        assert ss.load("no-such-id") is None

    def test_save_load_round_trip_preserves_data(self, tmp_path):
        ss = Snapstate(base_dir=tmp_path)
        original = _make_session(
            iteration=2,
            status="paused",
            papers=[_make_paper(), _make_paper(arxiv_id="2301.00002")],
            gaps=[_make_gap()],
            search_history=["q1", "q2", "q3"],
            hypotheses=["h1", "h2"],
        )
        ss.save(original)
        loaded = ss.load("abc12345")
        assert loaded.session_id == original.session_id
        assert loaded.query == original.query
        assert loaded.iteration == 2
        assert loaded.status == "paused"
        # Nested objects are plain dicts after save/load
        assert len(loaded.papers) == 2
        assert loaded.papers[0]["arxiv_id"] == "2301.00001"
        assert loaded.papers[1]["arxiv_id"] == "2301.00002"
        assert len(loaded.gaps) == 1
        assert loaded.search_history == ["q1", "q2", "q3"]
        assert loaded.hypotheses == ["h1", "h2"]

    def test_save_overwrites_existing(self, tmp_path):
        ss = Snapstate(base_dir=tmp_path)
        s1 = _make_session(query="first")
        ss.save(s1)
        s2 = _make_session(query="second")
        ss.save(s2)
        loaded = ss.load("abc12345")
        assert loaded.query == "second"

    @pytest.mark.no_freeze
    def test_save_updates_updated_at(self, tmp_path):
        """save() writes updated_at to disk; from_dict replays it.
        The loaded updated_at should be >= t_before (save happened after that)."""
        ss = Snapstate(base_dir=tmp_path)
        t_before = time.time()
        ss.save(_make_session())
        loaded = ss.load("abc12345")
        assert loaded is not None
        # updated_at is refreshed at load time (from_dict drops saved value),
        # so it will be >= t_before but may slightly exceed t_after.
        assert loaded.updated_at >= t_before


# =========================================================================
# 6. Snapstate -- load with corrupt files
# =========================================================================
class TestSnapstateLoadCorrupt:
    def test_load_corrupt_json_returns_none(self, tmp_path):
        ss = Snapstate(base_dir=tmp_path)
        # Write invalid JSON
        (tmp_path / "bad12345.json").write_text("NOT JSON {{{", encoding="utf-8")
        assert ss.load("bad12345") is None

    def test_load_missing_fields_returns_none(self, tmp_path):
        """A JSON file missing required fields should raise KeyError -> None."""
        ss = Snapstate(base_dir=tmp_path)
        # Write valid JSON but missing session_id / query
        (tmp_path / "inc12345.json").write_text(
            json.dumps({"only_this": True}), encoding="utf-8"
        )
        # from_dict will fail because session_id is missing -> caught -> None
        assert ss.load("inc12345") is None


# =========================================================================
# 7. Snapstate -- list_sessions
# =========================================================================
class TestSnapstateList:
    def test_list_empty_dir(self, tmp_path):
        ss = Snapstate(base_dir=tmp_path)
        assert ss.list_sessions() == []

    def test_list_single_session(self, tmp_path):
        ss = Snapstate(base_dir=tmp_path)
        ss.save(_make_session(query="find all about AI"))
        listing = ss.list_sessions()
        assert len(listing) == 1
        info = listing[0]
        assert info["session_id"] == "abc12345"
        assert info["query"] == "find all about AI"
        assert info["status"] == "running"
        assert info["iteration"] == 0
        assert isinstance(info["duration"], float)
        assert info["papers"] == 0
        assert info["gaps"] == 0

    def test_list_multiple_sessions(self, tmp_path):
        ss = Snapstate(base_dir=tmp_path)
        ss.save(_make_session(session_id="aaa", query="q1"))
        time.sleep(0.02)
        ss.save(_make_session(session_id="bbb", query="q2"))
        listing = ss.list_sessions()
        assert len(listing) == 2
        # Most recently modified should be first
        assert listing[0]["session_id"] == "bbb"
        assert listing[1]["session_id"] == "aaa"

    def test_list_includes_paper_gap_counts(self, tmp_path):
        ss = Snapstate(base_dir=tmp_path)
        s = _make_session(
            papers=[_make_paper(), _make_paper(arxiv_id="x")],
            gaps=[_make_gap()],
        )
        ss.save(s)
        info = ss.list_sessions()[0]
        assert info["papers"] == 2
        assert info["gaps"] == 1

    def test_list_handles_corrupt_file(self, tmp_path):
        ss = Snapstate(base_dir=tmp_path)
        ss.save(_make_session(session_id="good1"))
        # Corrupt file
        (tmp_path / "corrupt.json").write_text("garbage", encoding="utf-8")
        listing = ss.list_sessions()
        # Should have 2 entries: the corrupt one + the good one
        assert len(listing) == 2
        statuses = {s["status"] for s in listing}
        assert "corrupt" in statuses
        assert "running" in statuses

    def test_list_does_not_include_tmp_files(self, tmp_path):
        ss = Snapstate(base_dir=tmp_path)
        ss.save(_make_session())
        # Manually create a stray .tmp file
        (tmp_path / "stray.tmp").write_text("{}", encoding="utf-8")
        listing = ss.list_sessions()
        assert len(listing) == 1


# =========================================================================
# 8. Snapstate -- delete
# =========================================================================
class TestSnapstateDelete:
    def test_delete_existing_returns_true(self, tmp_path):
        ss = Snapstate(base_dir=tmp_path)
        ss.save(_make_session())
        assert ss.delete("abc12345") is True
        assert not (tmp_path / "abc12345.json").exists()

    def test_delete_nonexistent_returns_false(self, tmp_path):
        ss = Snapstate(base_dir=tmp_path)
        assert ss.delete("doesnt-exist") is False

    def test_delete_prevents_load(self, tmp_path):
        ss = Snapstate(base_dir=tmp_path)
        ss.save(_make_session())
        ss.delete("abc12345")
        assert ss.load("abc12345") is None

    def test_delete_one_of_many(self, tmp_path):
        ss = Snapstate(base_dir=tmp_path)
        ss.save(_make_session(session_id="aaa"))
        ss.save(_make_session(session_id="bbb"))
        ss.save(_make_session(session_id="ccc"))
        assert ss.delete("bbb") is True
        listing = ss.list_sessions()
        ids = [s["session_id"] for s in listing]
        assert "bbb" not in ids
        assert "aaa" in ids
        assert "ccc" in ids


# =========================================================================
# 9. Snapstate -- load_latest
# =========================================================================
class TestSnapstateLoadLatest:
    def test_load_latest_empty_returns_none(self, tmp_path):
        ss = Snapstate(base_dir=tmp_path)
        assert ss.load_latest() is None

    def test_load_latest_single(self, tmp_path):
        ss = Snapstate(base_dir=tmp_path)
        ss.save(_make_session(query="only one"))
        latest = ss.load_latest()
        assert latest is not None
        assert latest.session_id == "abc12345"
        assert latest.query == "only one"

    def test_load_latest_returns_most_recent(self, tmp_path):
        ss = Snapstate(base_dir=tmp_path)
        ss.save(_make_session(session_id="old1", query="first"))
        time.sleep(0.05)
        ss.save(_make_session(session_id="new1", query="second"))
        latest = ss.load_latest()
        assert latest is not None
        assert latest.session_id == "new1"
        assert latest.query == "second"

    def test_load_latest_after_delete(self, tmp_path):
        ss = Snapstate(base_dir=tmp_path)
        ss.save(_make_session(session_id="aaa"))
        time.sleep(0.05)
        ss.save(_make_session(session_id="bbb"))
        ss.delete("aaa")
        latest = ss.load_latest()
        assert latest is not None
        assert latest.session_id == "bbb"

    def test_load_latest_with_non_json_files(self, tmp_path):
        """Non-JSON files in the directory should be ignored by glob *.json."""
        ss = Snapstate(base_dir=tmp_path)
        ss.save(_make_session())
        (tmp_path / "notes.txt").write_text("not a session", encoding="utf-8")
        latest = ss.load_latest()
        assert latest is not None
        assert latest.session_id == "abc12345"


# =========================================================================
# 10. Snapstate -- new_session (mocked)
# =========================================================================
class TestSnapstateNewSession:
    def _mock_tracker(self, dims=None):
        """Build a mock evolution tracker.
        Note: must use 'is not not None' check (not 'or') because
        an empty dict {} is valid but falsy.
        """
        tracker = MagicMock()
        tracker.get_archetype.return_value = {
            "dimensions": dims
            if dims is not None
            else {
                "novelty": [0.0, 0.8],
                "feasibility": [0.0, 0.6],
            }
        }
        return tracker

    @patch("llm.insight_evolution.get_evolution_tracker")
    def test_new_session_creates_session(self, mock_get_tracker, tmp_path):
        mock_get_tracker.return_value = self._mock_tracker()
        ss = Snapstate(base_dir=tmp_path)
        s = ss.new_session("what is deep learning")
        assert isinstance(s, ResearchSession)
        assert s.query == "what is deep learning"
        assert isinstance(s.session_id, str)
        assert len(s.session_id) == 8

    @patch("llm.insight_evolution.get_evolution_tracker")
    def test_new_session_custom_max_iterations(self, mock_get_tracker, tmp_path):
        mock_get_tracker.return_value = self._mock_tracker()
        ss = Snapstate(base_dir=tmp_path)
        s = ss.new_session("test", max_iterations=7)
        assert s.max_iterations == 7

    @patch("llm.insight_evolution.get_evolution_tracker")
    def test_new_session_uses_archetype_when_provided(self, mock_get_tracker, tmp_path):
        mock_get_tracker.return_value = self._mock_tracker()
        ss = Snapstate(base_dir=tmp_path)
        custom_arch = {"dim_a": 0.99}
        s = ss.new_session("test", archetype=custom_arch)
        assert s.archetype == {"dim_a": 0.99}

    @patch("llm.insight_evolution.get_evolution_tracker")
    def test_new_session_falls_back_to_tracker_archetype(
        self, mock_get_tracker, tmp_path
    ):
        mock_get_tracker.return_value = self._mock_tracker()
        ss = Snapstate(base_dir=tmp_path)
        s = ss.new_session("test")
        assert "novelty" in s.archetype
        assert s.archetype["novelty"] == 0.8
        assert s.archetype["feasibility"] == 0.6

    @patch("llm.insight_evolution.get_evolution_tracker")
    def test_new_session_session_id_is_unique(self, mock_get_tracker, tmp_path):
        mock_get_tracker.return_value = self._mock_tracker()
        ss = Snapstate(base_dir=tmp_path)
        ids = {ss.new_session("q").session_id for _ in range(10)}
        assert len(ids) == 10

    @patch("llm.insight_evolution.get_evolution_tracker")
    def test_new_session_default_status(self, mock_get_tracker, tmp_path):
        mock_get_tracker.return_value = self._mock_tracker()
        ss = Snapstate(base_dir=tmp_path)
        s = ss.new_session("q")
        assert s.status == "running"
        assert s.iteration == 0

    @patch("llm.insight_evolution.get_evolution_tracker")
    def test_new_session_empty_dimensions(self, mock_get_tracker, tmp_path):
        """Tracker returns empty dimensions -> archetype should be empty dict."""
        mock_get_tracker.return_value = self._mock_tracker(dims={})
        ss = Snapstate(base_dir=tmp_path)
        s = ss.new_session("q")
        assert s.archetype == {}

    @patch("llm.insight_evolution.get_evolution_tracker")
    def test_new_session_calls_tracker(self, mock_get_tracker, tmp_path):
        """Verify that get_evolution_tracker is actually called."""
        mock_get_tracker.return_value = self._mock_tracker()
        ss = Snapstate(base_dir=tmp_path)
        ss.new_session("q")
        mock_get_tracker.assert_called_once()

    @patch("llm.insight_evolution.get_evolution_tracker")
    def test_new_session_default_query_and_iterations(self, mock_get_tracker, tmp_path):
        mock_get_tracker.return_value = self._mock_tracker()
        ss = Snapstate(base_dir=tmp_path)
        s = ss.new_session("my query")
        assert s.query == "my query"
        assert s.max_iterations == 3  # default


# =========================================================================
# 11. Integration: save -> load -> modify -> save -> list
# =========================================================================
class TestSnapstateIntegration:
    def test_full_lifecycle(self, tmp_path):
        ss = Snapstate(base_dir=tmp_path)
        # 1. Create and save
        s = _make_session(query="lifecycle test")
        ss.save(s)
        assert len(ss.list_sessions()) == 1
        # 2. Load
        loaded = ss.load("abc12345")
        assert loaded.query == "lifecycle test"
        assert loaded.status == "running"
        # 3. Modify
        loaded.status = "completed"
        loaded.iteration = 3
        loaded.papers.append(_make_paper())
        loaded.gaps.append(_make_gap())
        loaded.hypotheses.append("test hypothesis")
        # 4. Re-save
        ss.save(loaded)
        # 5. Reload and verify
        reloaded = ss.load("abc12345")
        assert reloaded.status == "completed"
        assert reloaded.iteration == 3
        assert len(reloaded.papers) == 1
        assert len(reloaded.gaps) == 1
        assert reloaded.hypotheses == ["test hypothesis"]

    def test_multiple_independent_sessions(self, tmp_path):
        ss = Snapstate(base_dir=tmp_path)
        for i in range(5):
            ss.save(_make_session(session_id=f"s{i:04d}", query=f"query {i}"))
        listing = ss.list_sessions()
        assert len(listing) == 5
        # Delete middle one
        assert ss.delete("s0002") is True
        listing = ss.list_sessions()
        assert len(listing) == 4
        ids = [s["session_id"] for s in listing]
        assert "s0002" not in ids

    def test_nested_paper_and_gap_round_trip(self, tmp_path):
        """Papers with gaps_found and gaps with matched_papers survive save/load."""
        ss = Snapstate(base_dir=tmp_path)
        p = _make_paper(gaps_found=["g001", "g002"])
        g = _make_gap(matched_papers=["2301.00001", "2301.00002"])
        s = _make_session(papers=[p], gaps=[g])
        ss.save(s)
        loaded = ss.load("abc12345")
        # Nested objects are dicts after save/load
        assert loaded.papers[0]["gaps_found"] == ["g001", "g002"]
        assert loaded.gaps[0]["matched_papers"] == ["2301.00001", "2301.00002"]


# =========================================================================
# 12. Edge cases
# =========================================================================
class TestEdgeCases:
    def test_empty_string_fields(self):
        p = PaperSnapshot(arxiv_id="", title="", abstract="", url="")
        d = p.to_dict()
        restored = PaperSnapshot.from_dict(d)
        assert restored.arxiv_id == ""

    def test_unicode_content(self, tmp_path):
        ss = Snapstate(base_dir=tmp_path)
        s = _make_session(
            query="研究课题：量子计算与人工智能",
            papers=[_make_paper(title="量子优势的证明", abstract="α β γ δ")],
        )
        ss.save(s)
        loaded = ss.load(s.session_id)
        assert loaded.query == "研究课题：量子计算与人工智能"
        # Nested papers are dicts after load
        assert loaded.papers[0]["title"] == "量子优势的证明"

    def test_large_session_with_many_papers(self, tmp_path):
        ss = Snapstate(base_dir=tmp_path)
        papers = [_make_paper(arxiv_id=f"2301.{i:05d}") for i in range(50)]
        gaps = [_make_gap(title=f"Gap {i}") for i in range(20)]
        s = _make_session(
            papers=papers,
            gaps=gaps,
            search_history=[f"q{i}" for i in range(50)],
        )
        ss.save(s)
        loaded = ss.load(s.session_id)
        assert len(loaded.papers) == 50
        assert len(loaded.gaps) == 20
        assert len(loaded.search_history) == 50

    def test_session_with_special_characters_in_query(self, tmp_path):
        ss = Snapstate(base_dir=tmp_path)
        q = "C++ & Python: a \"comparative\" study of <brackets> & 'quotes'"
        ss.save(_make_session(query=q))
        loaded = ss.load("abc12345")
        assert loaded.query == q

    def test_load_after_corrupt_overwrite(self, tmp_path):
        """If a valid file is overwritten with corrupt data, load returns None."""
        ss = Snapstate(base_dir=tmp_path)
        ss.save(_make_session())
        # Overwrite with corrupt
        (tmp_path / "abc12345.json").write_text("NOT VALID", encoding="utf-8")
        assert ss.load("abc12345") is None
