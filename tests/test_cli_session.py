"""Tests for session command — parser and subcommand functions."""
import argparse
from unittest.mock import MagicMock

from llm.research_session import ResearchSessionTracker
from cli.cmd.session import (
    _build_session_parser,
    _session_start,
    _session_list,
    _session_current,
)


class TestResearchSessionTracker:
    """Unit tests for ResearchSessionTracker initialization."""

    def test_tracker_default_init(self, tmp_path):
        tracker = ResearchSessionTracker(memory_dir=tmp_path)
        assert tracker.sessions_file == tmp_path / "research_sessions.jsonl"

    def test_tracker_with_memory_dir(self, tmp_path):
        tracker = ResearchSessionTracker(memory_dir=tmp_path)
        assert tracker.memory_dir == tmp_path


class TestBuildSessionParser:
    """Test session subcommand parser builds without error."""

    def test_parser_builds_successfully(self):
        parser = _build_session_parser(MagicMock())
        assert parser is not None

    def test_start_subparser_accepts_topic(self):
        parent = argparse.ArgumentParser()
        sub = parent.add_subparsers(dest="action")
        parser = _build_session_parser(sub)
        args = parser.parse_args(["start", "--topic", "LLM"])
        assert args.topic == "LLM"

    def test_list_subparser_accepts_days_and_limit(self):
        parent = argparse.ArgumentParser()
        sub = parent.add_subparsers(dest="action")
        parser = _build_session_parser(sub)
        args = parser.parse_args(["list", "--days", "30", "--limit", "5"])
        assert args.days == 30
        assert args.limit == 5

    def test_current_subparser(self):
        parent = argparse.ArgumentParser()
        sub = parent.add_subparsers(dest="action")
        parser = _build_session_parser(sub)
        args = parser.parse_args(["current"])
        assert args.action == "current"


class TestSessionSubcommands:
    """Test session subcommand functions with mock tracker."""

    def test_session_start(self, tmp_path, monkeypatch):
        tracker = ResearchSessionTracker(memory_dir=tmp_path)
        mock_session = MagicMock()
        mock_session.title = "Test Session"
        monkeypatch.setattr(tracker, "start_session", lambda title: mock_session)
        args = argparse.Namespace(
            action="start",
            topic="RLHF",
            title="Test Session",
        )
        result = _session_start(tracker, args)
        assert result == 0

    def test_session_list(self, tmp_path, monkeypatch):
        tracker = ResearchSessionTracker(memory_dir=tmp_path)
        monkeypatch.setattr(tracker, "get_recent_sessions", lambda days, limit: [])
        args = argparse.Namespace(
            action="list",
            days=7,
            limit=10,
        )
        result = _session_list(tracker, args)
        assert result == 0

    def test_session_current(self, tmp_path, monkeypatch):
        tracker = ResearchSessionTracker(memory_dir=tmp_path)
        monkeypatch.setattr(tracker, "get_current_session", lambda: None)
        result = _session_current(tracker)
        assert result == 0
