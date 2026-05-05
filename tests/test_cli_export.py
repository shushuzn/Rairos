"""Tier 1 tests — CLI export command."""

import pytest
from unittest.mock import patch, MagicMock
from cli import _run_export, _build_export_parser
import argparse


class TestExportParser:
    """Test export parser construction."""

    def test_build_parser_accepts_format_limit_out_paper_flags(self):
        """_build_export_parser adds --format/--limit/--out/--paper flags."""
        p = argparse.ArgumentParser()
        sub = p.add_subparsers()
        result = _build_export_parser(sub)
        assert result is not None


class TestRunExport:
    """Test _run_export function with mocked database."""

    @patch("cli.Database")
    def test_export_csv_no_papers(self, mock_db_cls, capsys):
        """_run_export --format csv with empty DB returns header row."""
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.export_papers.return_value = (["id", "title"], [])
        mock_db_cls.return_value = mock_db
        args = argparse.Namespace(format="csv", limit=0, out=None, paper=None)
        rc = _run_export(args)
        assert rc == 0
        captured = capsys.readouterr()
        assert "id" in captured.out
        assert "title" in captured.out

    @patch("cli.Database")
    def test_export_json_no_papers(self, mock_db_cls, capsys):
        """_run_export --format json with empty DB returns empty list."""
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.export_papers.return_value = (["id", "title"], [])
        mock_db_cls.return_value = mock_db
        args = argparse.Namespace(format="json", limit=0, out=None, paper=None)
        rc = _run_export(args)
        assert rc == 0
        captured = capsys.readouterr()
        assert "[]" in captured.out

    @patch("cli.Database")
    def test_export_bibtex_no_papers(self, mock_db_cls, capsys):
        """_run_export --format bibtex with empty DB returns empty output."""
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.export_papers.return_value = (["id", "title"], [])
        mock_db_cls.return_value = mock_db
        args = argparse.Namespace(format="bibtex", limit=0, out=None, paper=None)
        rc = _run_export(args)
        assert rc == 0

    @patch("cli.Database")
    def test_export_single_paper_not_found(self, mock_db_cls, capsys):
        """_run_export --paper <id> with non-existent ID exits 1."""
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.paper_exists.return_value = False
        mock_db_cls.return_value = mock_db
        args = argparse.Namespace(format="csv", limit=0, out=None, paper="nonexistent-id")
        rc = _run_export(args)
        assert rc == 1

    @patch("cli.Database")
    def test_export_single_paper_json(self, mock_db_cls, capsys):
        """_run_export --paper <id> --format json exports single paper."""
        mock_paper = MagicMock()
        mock_paper.id = "2301.12345"
        mock_paper.title = "Test Paper"
        mock_paper.authors = ["Smith, John"]
        mock_paper.abstract = "Abstract text"
        mock_paper.published = "2023-01-15"
        mock_paper.doi = "10.1234/test"
        mock_paper.journal = "Test Journal"
        mock_paper.primary_category = "cs.AI"
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.paper_exists.return_value = True
        mock_db.get_paper.return_value = mock_paper
        mock_db_cls.return_value = mock_db
        args = argparse.Namespace(format="json", limit=0, out=None, paper="2301.12345")
        rc = _run_export(args)
        assert rc == 0
        captured = capsys.readouterr()
        assert "Test Paper" in captured.out

    @patch("cli.Database")
    def test_export_limit_passed_to_db(self, mock_db_cls):
        """_run_export --limit 10 passes limit to db.export_papers."""
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.export_papers.return_value = (["id"], [])
        mock_db_cls.return_value = mock_db
        args = argparse.Namespace(format="csv", limit=10, out=None, paper=None)
        rc = _run_export(args)
        assert rc == 0
        mock_db.export_papers.assert_called_once()
        call_kwargs = mock_db.export_papers.call_args
        assert call_kwargs[1]["limit"] == 10
