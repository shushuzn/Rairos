"""Tests for cli/cmd/dedup.py _pick_keep function."""
import pytest
from dataclasses import dataclass


@dataclass
class MockPaper:
    parse_status: str


class TestPickKeep:
    """Test _pick_keep from cli/cmd/dedup.py."""

    def _pick_keep(self):
        from cli.cmd.dedup import _pick_keep

        return _pick_keep

    def test_keep_older_strategy(self):
        pick = self._pick_keep()
        older = MockPaper(parse_status="completed")
        newer = MockPaper(parse_status="pending")
        winner, loser = pick(older, newer, "older")
        assert winner is older
        assert loser is newer

    def test_keep_newer_strategy(self):
        pick = self._pick_keep()
        older = MockPaper(parse_status="completed")
        newer = MockPaper(parse_status="pending")
        winner, loser = pick(older, newer, "newer")
        assert winner is newer
        assert loser is older

    def test_keep_parsed_completed_vs_pending(self):
        pick = self._pick_keep()
        pending = MockPaper(parse_status="pending")
        completed = MockPaper(parse_status="completed")
        winner, loser = pick(pending, completed, "parsed")
        assert winner is completed
        assert loser is pending

    def test_keep_parsed_running_vs_failed(self):
        pick = self._pick_keep()
        failed = MockPaper(parse_status="failed")
        running = MockPaper(parse_status="running")
        winner, loser = pick(failed, running, "parsed")
        assert winner is running
        assert loser is failed

    def test_keep_parsed_equal_rank_keeps_older(self):
        pick = self._pick_keep()
        # Equal rank: both pending
        older = MockPaper(parse_status="pending")
        newer = MockPaper(parse_status="pending")
        winner, loser = pick(older, newer, "parsed")
        # Equal rank → older wins (consistent with >=)
        assert winner is older
        assert loser is newer

    def test_unknown_status_gets_lowest_rank(self):
        pick = self._pick_keep()
        unknown = MockPaper(parse_status="unknown_status")
        completed = MockPaper(parse_status="completed")
        winner, loser = pick(unknown, completed, "parsed")
        assert winner is completed
        assert loser is unknown
