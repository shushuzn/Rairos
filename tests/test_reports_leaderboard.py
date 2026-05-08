"""Tests for reports._caps_by_theme and leaderboard.LeaderboardEntry."""

import pytest
from llm.reports import _caps_by_theme
from research_loop.leaderboard import LeaderboardEntry


# ── FakeCapsule for _caps_by_theme ────────────────────────────────────────────


class FakeCapsule:
    def __init__(self, capsule_id, title, topic):
        self.capsule_id = capsule_id
        self.action_gap_title = title
        self.trigger_topic = topic

    def __repr__(self):
        return f"FakeCapsule({self.capsule_id})"


# ── _caps_by_theme ────────────────────────────────────────────────────────────


class TestCapsByTheme:
    def test_empty(self):
        result = _caps_by_theme([], ["robot"])
        assert result == []

    def test_matching_keyword(self):
        caps = [FakeCapsule("c1", "Robot Learning", "rl")]
        result = _caps_by_theme(caps, ["robot"])
        assert len(result) == 1
        assert result[0].capsule_id == "c1"

    def test_matches_topic(self):
        caps = [FakeCapsule("c1", "Some Title", "embodied ai")]
        result = _caps_by_theme(caps, ["embodied"])
        assert len(result) == 1

    def test_no_match(self):
        caps = [FakeCapsule("c1", "NLP Paper", "language")]
        result = _caps_by_theme(caps, ["robot", "vla"])
        assert result == []

    def test_exclude(self):
        caps = [
            FakeCapsule("c1", "Robot Paper", "rl"),
            FakeCapsule("c2", "Robot Paper 2", "rl"),
        ]
        result = _caps_by_theme(caps, ["robot"], exclude={"c1"})
        assert len(result) == 1
        assert result[0].capsule_id == "c2"

    def test_case_insensitive(self):
        caps = [FakeCapsule("c1", "ROBOT Learning", "RL")]
        result = _caps_by_theme(caps, ["robot"])
        assert len(result) == 1

    def test_multiple_keywords_one_match(self):
        caps = [FakeCapsule("c1", "Diffusion Policy Paper", "rl")]
        result = _caps_by_theme(caps, ["robot", "diffusion policy"])
        assert len(result) == 1


# ── LeaderboardEntry ──────────────────────────────────────────────────────────


class TestComputeScore:
    def test_perfect(self):
        entry = LeaderboardEntry(
            arxiv_id="1234",
            passed=10,
            failed=0,
            skipped=0,
            pass_rate=1.0,
            coverage_ratio=1.0,
        )
        entry.compute_score()  # noqa: F841
        # combined = 1.0*0.7 + 1.0*0.3 = 1.0
        # stub_rate = 0 → low penalty 0.05 → calibrated = 1.0 * 0.95 = 0.95
        assert entry.combined_score == 1.0
        assert entry.stub_rate == 0.0
        assert entry.difficulty_penalty == 0.05  # PENALTY_LOW
        assert entry.combined_score * (1 - entry.difficulty_penalty) == pytest.approx(0.95)

    def test_all_failed(self):
        entry = LeaderboardEntry(
            arxiv_id="1234",
            passed=0,
            failed=10,
            skipped=0,
            pass_rate=0.0,
            coverage_ratio=0.5,
        )
        entry.compute_score()  # noqa: F841
        # combined = 0*0.7 + 0.5*0.3 = 0.15
        assert entry.combined_score == 0.15

    def test_high_stub_rate(self):
        entry = LeaderboardEntry(
            arxiv_id="1234",
            passed=2,
            failed=1,
            skipped=7,
            pass_rate=0.67,
            coverage_ratio=0.3,
        )
        entry.compute_score()  # noqa: F841
        # stub_rate = 7/10 = 0.70 → exactly at HIGH threshold = 0.40 penalty
        assert entry.stub_rate == 0.7
        assert entry.difficulty_penalty == 0.40  # PENALTY_HIGH

    def test_medium_stub_rate(self):
        entry = LeaderboardEntry(
            arxiv_id="1234",
            passed=5,
            failed=1,
            skipped=4,
            pass_rate=0.83,
            coverage_ratio=0.5,
        )
        entry.compute_score()
        # stub_rate = 4/10 = 0.40 → exactly at MEDIUM threshold = 0.20 penalty
        assert entry.stub_rate == 0.4
        assert entry.difficulty_penalty == 0.20  # PENALTY_MEDIUM

    def test_zero_total(self):
        entry = LeaderboardEntry(
            arxiv_id="1234",
            passed=0,
            failed=0,
            skipped=0,
            pass_rate=0.0,
            coverage_ratio=0.0,
        )
        entry.compute_score()
        assert entry.stub_rate == 0.0


class TestLeaderboardEntry:
    def test_to_dict(self):
        entry = LeaderboardEntry(arxiv_id="1234", title="Test", passed=5, failed=1)
        d = entry.to_dict()
        assert d["arxiv_id"] == "1234"
        assert d["passed"] == 5

    def test_from_dict(self):
        data = {"arxiv_id": "1234", "title": "Test", "passed": 5, "failed": 1, "skipped": 0}
        entry = LeaderboardEntry.from_dict(data)
        assert entry.arxiv_id == "1234"
        assert entry.passed == 5

    def test_from_dict_extra_keys(self):
        """Extra keys should be silently ignored."""
        data = {"arxiv_id": "1234", "title": "T", "unknown_field": "ignored"}
        entry = LeaderboardEntry.from_dict(data)
        assert entry.arxiv_id == "1234"

    def test_defaults(self):
        entry = LeaderboardEntry(arxiv_id="1234")
        assert entry.passed == 0
        assert entry.combined_score == 0.0
        assert entry.framework == "pytorch"
