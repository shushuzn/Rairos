"""Unit tests for CLI citations subcommand."""
import argparse
import csv
import io
from unittest.mock import MagicMock, patch

import pytest

from cli import _run_citations


# ─────────────────────────────────────────────────────────────────────────────
# Helpers
# ─────────────────────────────────────────────────────────────────────────────

class FakeCitationRecord:
    """Fake CitationRecord matching db.database.CitationRecord."""
    def __init__(self, source_id, target_id, source_title="", target_title=""):
        self.source_id = source_id
        self.target_id = target_id
        self.source_title = source_title
        self.target_title = target_title


def make_args(**kwargs):
    defaults = dict(citation_from=None, citation_to=None, format="text")
    defaults.update(kwargs)
    ns = argparse.Namespace()
    for k, v in defaults.items():
        setattr(ns, k, v)
    return ns


# ─────────────────────────────────────────────────────────────────────────────
# Test: citations --from (backward citations)
# ─────────────────────────────────────────────────────────────────────────────

class TestRunCitationsFrom:
    """Test _run_citations with --from (backward citations / references)."""

    @patch("cli.Database")
    def test_from_shows_citations(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_paper_title.side_effect = lambda pid: {
            "2301.00001": "Attention Is All You Need",
            "2107.12345": "BERT",
            "2106.09876": "GPT-3",
        }.get(pid, "")
        mock_db.get_citations.return_value = [
            FakeCitationRecord(source_id="2301.00001", target_id="2107.12345"),
            FakeCitationRecord(source_id="2301.00001", target_id="2106.09876"),
        ]
        mock_db_cls.return_value = mock_db

        args = make_args(citation_from="2301.00001", format="text")
        result = _run_citations(args)

        captured = capsys.readouterr().out
        assert "2301.00001" in captured          # paper_id in header
        assert "Attention Is All You Need" in captured  # source paper title in header
        assert "2107.12345" in captured          # target paper in citation line
        assert "BERT" in captured                # target paper title in citation line
        assert "2106.09876" in captured          # another target
        assert "GPT-3" in captured
        assert result == 0

    @patch("cli.Database")
    def test_from_no_citations(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_paper_title.return_value = "Some Paper"
        mock_db.get_citations.return_value = []
        mock_db_cls.return_value = mock_db

        args = make_args(citation_from="2301.00001", format="text")
        result = _run_citations(args)

        captured = capsys.readouterr().out
        assert "No citations found" in captured
        assert result == 0

    @patch("cli.Database")
    def test_from_paper_not_found(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_paper_title.return_value = None
        mock_db_cls.return_value = mock_db

        args = make_args(citation_from="nonexistent.99999", format="text")
        result = _run_citations(args)

        captured = capsys.readouterr().out
        assert "not found in the database" in captured
        assert result == 1


# ─────────────────────────────────────────────────────────────────────────────
# Test: citations --to (forward citations)
# ─────────────────────────────────────────────────────────────────────────────

class TestRunCitationsTo:
    """Test _run_citations with --to (forward citations / bibliography)."""

    @patch("cli.Database")
    def test_to_shows_citing_papers(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_paper_title.return_value = "BERT"
        mock_db.get_citations.return_value = [
            FakeCitationRecord(source_id="2301.00001", target_id="2107.12345",
                              source_title="Attention Is All You Need", target_title="BERT"),
        ]
        mock_db_cls.return_value = mock_db

        args = make_args(citation_to="2107.12345", format="text")
        result = _run_citations(args)

        captured = capsys.readouterr().out
        assert "2107.12345" in captured    # target paper_id in header
        assert "BERT" in captured           # target paper title in header
        assert "2301.00001" in captured    # source paper in citation line
        # For forward citations, we show (source_id, target_title) — not source_title
        assert result == 0

    @patch("cli.Database")
    def test_to_no_citing_papers(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_paper_title.return_value = "BERT"
        mock_db.get_citations.return_value = []
        mock_db_cls.return_value = mock_db

        args = make_args(citation_to="2107.12345", format="text")
        result = _run_citations(args)

        captured = capsys.readouterr().out
        assert "No citations found" in captured
        assert result == 0


# ─────────────────────────────────────────────────────────────────────────────
# Test: citations --format csv
# ─────────────────────────────────────────────────────────────────────────────

class TestRunCitationsCsv:
    """Test _run_citations with --format csv."""

    @patch("cli.Database")
    def test_csv_output(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_paper_title.return_value = "Attention Is All You Need"
        mock_db.get_citations.return_value = [
            FakeCitationRecord(source_id="2301.00001", target_id="2107.12345",
                              source_title="A", target_title="BERT"),
        ]
        mock_db_cls.return_value = mock_db

        args = make_args(citation_from="2301.00001", format="csv")
        result = _run_citations(args)

        captured = capsys.readouterr().out
        reader = csv.DictReader(io.StringIO(captured))
        rows = list(reader)

        assert len(rows) == 1
        assert rows[0]["source_id"] == "2301.00001"
        assert rows[0]["target_id"] == "2107.12345"
        assert result == 0

    @patch("cli.Database")
    def test_csv_header_only_when_empty(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_paper_title.return_value = "Attention Is All You Need"
        mock_db.get_citations.return_value = []
        mock_db_cls.return_value = mock_db

        args = make_args(citation_from="2301.00001", format="csv")
        result = _run_citations(args)

        captured = capsys.readouterr().out
        reader = csv.DictReader(io.StringIO(captured))
        rows = list(reader)
        assert rows == []
        assert result == 0


# ─────────────────────────────────────────────────────────────────────────────
# Test: citations (no flags)
# ─────────────────────────────────────────────────────────────────────────────

class TestRunCitationsNoFlags:
    """Test _run_citations when no flags are provided."""

    def test_no_flags_shows_error(self, capsys):
        args = make_args(format="text")
        result = _run_citations(args)

        captured = capsys.readouterr().err
        assert "Error: must specify --from or --to" in captured
        assert result == 1


# ─────────────────────────────────────────────────────────────────────────────
# Test: citation count in output
# ─────────────────────────────────────────────────────────────────────────────

class TestRunCitationsCount:
    """Test that citation counts are displayed correctly."""

    @patch("cli.Database")
    def test_from_shows_count(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_paper_title.return_value = "Attention Is All You Need"
        mock_db.get_citations.return_value = [
            FakeCitationRecord(source_id="2301.00001", target_id=str(i),
                              source_title="A", target_title=f"Paper {i}")
            for i in range(1, 4)
        ]
        mock_db_cls.return_value = mock_db

        args = make_args(citation_from="2301.00001", format="text")
        result = _run_citations(args)

        captured = capsys.readouterr().out
        assert "3 references" in captured
        assert result == 0

    @patch("cli.Database")
    def test_to_shows_count(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_paper_title.return_value = "BERT"
        mock_db.get_citations.return_value = [
            FakeCitationRecord(source_id=str(i), target_id="2107.12345",
                              source_title=f"Paper {i}", target_title="BERT")
            for i in range(1, 3)
        ]
        mock_db_cls.return_value = mock_db

        args = make_args(citation_to="2107.12345", format="text")
        result = _run_citations(args)

        captured = capsys.readouterr().out
        assert "2 citing papers" in captured
        assert result == 0
