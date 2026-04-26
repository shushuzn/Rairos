"""Tests for session CLI commands."""
import pytest
from unittest.mock import MagicMock, patch
from io import StringIO

from cli.cmd.session import (
    _build_session_parser,
    _session_start,
    _session_list,
    _session_current,
    _session_end,
)
from llm.research_session import ResearchIntent


class TestSessionParser:
    """Test session command parser."""

    def test_build_session_parser_succeeds(self):
        """Test that _build_session_parser executes without error."""
        import argparse
        parser = argparse.ArgumentParser()
        subparsers = parser.add_subparsers()
        # Should not raise
        result = _build_session_parser(subparsers)
        # Verify it returns a parser
        assert result is not None
        # Verify session subparser was added
        assert "session" in subparsers.choices


class TestSessionStart:
    """Test session start functionality."""

    @patch("cli.cmd.session.ResearchSessionTracker")
    def test_start_creates_session(self, mock_tracker_class):
        """Test starting a session."""
        mock_tracker = MagicMock()
        mock_session = MagicMock()
        mock_session.title = "Test Session"
        mock_session.id = "sess-123"
        mock_tracker.start_session.return_value = mock_session
        mock_tracker_class.return_value = mock_tracker

        args = MagicMock()
        args.title = "Test Session"
        args.topic = None

        result = _session_start(mock_tracker, args)
        assert result == 0
        mock_tracker.start_session.assert_called_once_with(title="Test Session")


class TestSessionList:
    """Test session list functionality."""

    @patch("cli.cmd.session.ResearchSessionTracker")
    def test_list_empty_sessions(self, mock_tracker_class):
        """Test listing with no sessions."""
        mock_tracker = MagicMock()
        mock_tracker.get_recent_sessions.return_value = []
        mock_tracker_class.return_value = mock_tracker

        args = MagicMock()
        args.days = 7
        args.limit = 10

        result = _session_list(mock_tracker, args)
        assert result == 0

    @patch("cli.cmd.session.ResearchSessionTracker")
    def test_list_with_sessions(self, mock_tracker_class):
        """Test listing with sessions."""
        mock_tracker = MagicMock()

        mock_session = MagicMock()
        mock_session.title = "RAG Research"
        mock_session.started_at = "2024-01-15T10:00:00"
        mock_session.intent = ResearchIntent.LEARNING
        mock_session.queries = ["q1", "q2"]
        mock_session.duration_minutes = 30
        mock_session.tags = ["RAG", "NLP"]
        mock_session.insights = ["insight1"]

        mock_tracker.get_recent_sessions.return_value = [mock_session]
        mock_tracker_class.return_value = mock_tracker

        args = MagicMock()
        args.days = 7
        args.limit = 10

        result = _session_list(mock_tracker, args)
        assert result == 0


class TestSessionCurrent:
    """Test session current functionality."""

    @patch("cli.cmd.session.ResearchSessionTracker")
    def test_current_no_session(self, mock_tracker_class):
        """Test current with no active session."""
        mock_tracker = MagicMock()
        mock_tracker.get_current_session.return_value = None
        mock_tracker_class.return_value = mock_tracker

        result = _session_current(mock_tracker)
        assert result == 0

    @patch("cli.cmd.session.ResearchSessionTracker")
    def test_current_with_session(self, mock_tracker_class):
        """Test current with active session."""
        mock_tracker = MagicMock()

        mock_session = MagicMock()
        mock_session.title = "Active Session"
        mock_session.id = "sess-456"
        mock_session.duration_minutes = 45
        mock_session.queries = ["q1"]
        mock_session.intent = ResearchIntent.EXPLORING
        mock_session.tags = ["AI"]
        mock_session.insights = ["insight1", "insight2"]

        mock_tracker.get_current_session.return_value = mock_session
        mock_tracker_class.return_value = mock_tracker

        result = _session_current(mock_tracker)
        assert result == 0


class TestSessionEnd:
    """Test session end functionality."""

    @patch("cli.cmd.session.ResearchSessionTracker")
    def test_end_no_session(self, mock_tracker_class):
        """Test ending when no session active."""
        mock_tracker = MagicMock()
        mock_tracker.end_session.return_value = None
        mock_tracker_class.return_value = mock_tracker

        result = _session_end(mock_tracker)
        assert result == 0

    @patch("cli.cmd.session.ResearchSessionTracker")
    def test_end_with_session(self, mock_tracker_class):
        """Test ending active session."""
        mock_tracker = MagicMock()

        mock_session = MagicMock()
        mock_session.title = "Ending Session"
        mock_session.duration_minutes = 60
        mock_session.queries = ["q1", "q2", "q3"]

        mock_tracker.end_session.return_value = mock_session
        mock_tracker_class.return_value = mock_tracker

        result = _session_end(mock_tracker)
        assert result == 0
