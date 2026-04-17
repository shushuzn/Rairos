"""Unit tests for CLI search, list, and status subcommands."""
import argparse
import json
import sys
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest

from cli import _run_search, _run_list, _run_status


# ─────────────────────────────────────────────────────────────────────────────
# Helpers
# ─────────────────────────────────────────────────────────────────────────────

class FakeSearchResult:
    """Fake SearchResult matching db.database.SearchResult."""
    def __init__(
        self,
        paper_id="2301.00001",
        title="Attention Is All You Need",
        authors="Vaswani et al.",
        published="2017-06-12",
        primary_category="cs.CL",
        score=7.43,
        snippet="**attention** mechanism",
        source="arxiv",
        abs_url="https://arxiv.org/abs/1706.03762",
        pdf_url="https://arxiv.org/1706.03762.pdf",
        parse_status="done",
    ):
        self.paper_id = paper_id
        self.title = title
        self.authors = authors
        self.published = published
        self.primary_category = primary_category
        self.score = score
        self.snippet = snippet
        self.source = source
        self.abs_url = abs_url
        self.pdf_url = pdf_url
        self.parse_status = parse_status


class FakePaper:
    """Fake Paper matching db.database.Paper."""
    def __init__(
        self,
        id="2301.00001",
        title="Attention Is All You Need",
        authors="Vaswani et al.",
        published="2017-06-12",
        primary_category="cs.CL",
        source="arxiv",
        abs_url="https://arxiv.org/abs/1706.03762",
        pdf_url="https://arxiv.org/1706.03762.pdf",
        parse_status="done",
    ):
        self.id = id
        self.title = title
        self.authors = authors
        self.published = published
        self.primary_category = primary_category
        self.source = source
        self.abs_url = abs_url
        self.pdf_url = pdf_url
        self.parse_status = parse_status


def make_args(**kwargs):
    ns = argparse.Namespace()
    for k, v in kwargs.items():
        setattr(ns, k, v)
    return ns


# ─────────────────────────────────────────────────────────────────────────────
# _run_search tests
# ─────────────────────────────────────────────────────────────────────────────

class TestRunSearchTable:
    """Test _run_search with table format (default)."""

    @patch("cli.Database")
    def test_table_header_shows_total(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.search_papers.return_value = ([FakeSearchResult()], 1)
        mock_db_cls.return_value = mock_db

        args = make_args(
            query="attention",
            limit=10,
            offset=0,
            format="table",
            source="",
            year=0,
            tags=[],
            status="",
            sort="relevance",
        )
        _run_search(args)

        captured = capsys.readouterr().out
        assert "Found 1 papers" in captured
        assert "Attention Is All You Need" in captured
        assert "Vaswani et al." in captured
        assert "2017-06-12" in captured

    @patch("cli.Database")
    def test_table_shows_score(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.search_papers.return_value = ([FakeSearchResult(score=7.43)], 1)
        mock_db_cls.return_value = mock_db

        args = make_args(query="", limit=10, offset=0, format="table",
                         source="", year=0, tags=[], status="", sort="relevance")
        _run_search(args)

        captured = capsys.readouterr().out
        assert "[7.43]" in captured

    @patch("cli.Database")
    def test_table_shows_snippet(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.search_papers.return_value = ([FakeSearchResult(
            snippet="**attention** mechanism and **transformer** architecture"
        )], 1)
        mock_db_cls.return_value = mock_db

        args = make_args(query="", limit=10, offset=0, format="table",
                         source="", year=0, tags=[], status="", sort="relevance")
        _run_search(args)

        captured = capsys.readouterr().out
        assert "..." in captured
        assert "**attention**" in captured

    @patch("cli.Database")
    def test_no_results(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.search_papers.return_value = ([], 0)
        mock_db_cls.return_value = mock_db

        args = make_args(query="nonexistent", limit=10, offset=0, format="table",
                         source="", year=0, tags=[], status="", sort="relevance")
        _run_search(args)

        captured = capsys.readouterr().out
        assert "Found 0 papers" in captured

    @patch("cli.Database")
    def test_multiple_results(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        results = [
            FakeSearchResult(paper_id="2301.00001", title="Paper One", score=5.0),
            FakeSearchResult(paper_id="2301.00002", title="Paper Two", score=3.0),
        ]
        mock_db.search_papers.return_value = (results, 2)
        mock_db_cls.return_value = mock_db

        args = make_args(query="", limit=10, offset=0, format="table",
                         source="", year=0, tags=[], status="", sort="relevance")
        _run_search(args)

        captured = capsys.readouterr().out
        assert "Paper One" in captured
        assert "Paper Two" in captured
        assert "[5.00]" in captured

    @patch("cli.Database")
    def test_calls_search_papers_with_correct_args(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.search_papers.return_value = ([], 0)
        mock_db_cls.return_value = mock_db

        args = make_args(
            query="transformer",
            limit=5,
            offset=10,
            format="table",
            source="arxiv",
            year=2024,
            tags=["nlp"],
            status="done",
            sort="relevance",
        )
        _run_search(args)

        mock_db.search_papers.assert_called_once()
        call_kwargs = mock_db.search_papers.call_args[1]
        assert call_kwargs["query"] == "transformer"
        assert call_kwargs["limit"] == 5
        assert call_kwargs["offset"] == 10
        assert call_kwargs["source"] == "arxiv"
        assert call_kwargs["parse_status"] == "done"
        assert call_kwargs["date_from"] == "2024-01-01"

    @patch("cli.Database")
    def test_empty_query_allowed(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.search_papers.return_value = ([], 0)
        mock_db_cls.return_value = mock_db

        args = make_args(query="", limit=10, offset=0, format="table",
                         source="", year=0, tags=[], status="", sort="relevance")
        _run_search(args)

        mock_db.search_papers.assert_called_once()
        assert mock_db.search_papers.call_args[1]["query"] == ""

    @patch("cli.Database")
    def test_returns_zero(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.search_papers.return_value = ([], 0)
        mock_db_cls.return_value = mock_db

        args = make_args(query="x", limit=10, offset=0, format="table",
                         source="", year=0, tags=[], status="", sort="relevance")
        result = _run_search(args)
        assert result == 0


class TestRunSearchJson:
    """Test _run_search with JSON format."""

    @patch("cli.Database")
    def test_json_output(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.search_papers.return_value = ([FakeSearchResult()], 1)
        mock_db_cls.return_value = mock_db

        args = make_args(query="", limit=10, offset=0, format="json",
                         source="", year=0, tags=[], status="", sort="relevance")
        _run_search(args)

        captured = capsys.readouterr().out
        data = json.loads(captured)
        assert data["total"] == 1
        assert len(data["results"]) == 1
        assert data["results"][0]["title"] == "Attention Is All You Need"
        assert data["results"][0]["score"] == 7.43
        assert "**attention**" in data["results"][0]["snippet"]

    @patch("cli.Database")
    def test_json_score_rounded(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.search_papers.return_value = ([FakeSearchResult(score=7.438)], 1)
        mock_db_cls.return_value = mock_db

        args = make_args(query="", limit=10, offset=0, format="json",
                         source="", year=0, tags=[], status="", sort="relevance")
        _run_search(args)

        captured = capsys.readouterr().out
        data = json.loads(captured)
        assert data["results"][0]["score"] == 7.438

    @patch("cli.Database")
    def test_json_null_score_when_none(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.search_papers.return_value = ([FakeSearchResult(score=None)], 1)
        mock_db_cls.return_value = mock_db

        args = make_args(query="", limit=10, offset=0, format="json",
                         source="", year=0, tags=[], status="", sort="relevance")
        _run_search(args)

        captured = capsys.readouterr().out
        data = json.loads(captured)
        assert data["results"][0]["score"] is None


class TestRunSearchCsv:
    """Test _run_search with CSV format."""

    @patch("cli.Database")
    def test_csv_header(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.search_papers.return_value = ([FakeSearchResult()], 1)
        mock_db_cls.return_value = mock_db

        args = make_args(query="", limit=10, offset=0, format="csv",
                         source="", year=0, tags=[], status="", sort="relevance")
        _run_search(args)

        captured = capsys.readouterr().out.replace("\r", "")
        lines = captured.strip().split("\n")
        assert lines[0] == "paper_id,title,authors,published,primary_category,score,snippet,source,abs_url,parse_status"

    @patch("cli.Database")
    def test_csv_row(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.search_papers.return_value = ([FakeSearchResult()], 1)
        mock_db_cls.return_value = mock_db

        args = make_args(query="", limit=10, offset=0, format="csv",
                         source="", year=0, tags=[], status="", sort="relevance")
        _run_search(args)

        captured = capsys.readouterr().out.replace("\r", "")
        lines = captured.strip().split("\n")
        assert "2301.00001" in lines[1]
        assert "Attention Is All You Need" in lines[1]
        assert "7.43" in lines[1]


# ─────────────────────────────────────────────────────────────────────────────
# _run_list tests
# ─────────────────────────────────────────────────────────────────────────────

class TestRunList:
    """Test _run_list."""

    @patch("cli.Database")
    def test_list_table_output(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.list_papers.return_value = ([FakePaper(), FakePaper()], 2)
        mock_db_cls.return_value = mock_db

        args = make_args(status="", year=0, tags=[], limit=20, offset=0, format="table")
        _run_list(args)

        captured = capsys.readouterr().out
        assert "2301.00001" in captured
        assert "Attention Is All You Need" in captured

    @patch("cli.Database")
    def test_list_json_output(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.list_papers.return_value = ([FakePaper()], 1)
        mock_db_cls.return_value = mock_db

        args = make_args(status="", year=0, tags=[], limit=20, offset=0, format="json")
        _run_list(args)

        captured = capsys.readouterr().out
        data = json.loads(captured)
        assert len(data) == 1
        assert data[0]["title"] == "Attention Is All You Need"

    @patch("cli.Database")
    def test_calls_list_papers_with_filters(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.list_papers.return_value = ([], 0)
        mock_db_cls.return_value = mock_db

        args = make_args(status="done", year=2024, tags=["nlp"], limit=5, offset=10, format="table")
        _run_list(args)

        mock_db.list_papers.assert_called_once()
        call_kwargs = mock_db.list_papers.call_args[1]
        assert call_kwargs["parse_status"] == "done"
        assert call_kwargs["limit"] == 5
        assert call_kwargs["offset"] == 10
        assert call_kwargs["date_from"] == "2024-01-01"

    @patch("cli.Database")
    def test_list_empty(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.list_papers.return_value = ([], 0)
        mock_db_cls.return_value = mock_db

        args = make_args(status="", year=0, tags=[], limit=20, offset=0, format="table")
        _run_list(args)

        captured = capsys.readouterr().out
        assert captured.strip() == ""

    @patch("cli.Database")
    def test_returns_zero(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.list_papers.return_value = ([], 0)
        mock_db_cls.return_value = mock_db

        args = make_args(status="", year=0, tags=[], limit=20, offset=0, format="table")
        result = _run_list(args)
        assert result == 0


# ─────────────────────────────────────────────────────────────────────────────
# _run_status tests
# ─────────────────────────────────────────────────────────────────────────────

class TestRunStatus:
    """Test _run_status."""

    @patch("cli.Database")
    def test_status_output(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        # _run_status calls get_papers() not get_stats()
        mock_db.get_papers.return_value = [
            FakePaper(source="arxiv", parse_status="done"),
            FakePaper(source="arxiv", parse_status="done"),
            FakePaper(source="doi", parse_status="pending"),
        ]
        mock_db_cls.return_value = mock_db

        args = make_args()
        _run_status(args)

        captured = capsys.readouterr().out
        assert "Total papers: 3" in captured
        assert "arxiv=2" in captured
        assert "done=2" in captured
        assert "pending=1" in captured

    @patch("cli.Database")
    def test_status_empty_db(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.get_papers.return_value = []
        mock_db_cls.return_value = mock_db

        args = make_args()
        _run_status(args)

        captured = capsys.readouterr().out
        assert "Total papers: 0" in captured

    @patch("cli.Database")
    def test_returns_zero(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.get_papers.return_value = []
        mock_db_cls.return_value = mock_db

        args = make_args()
        result = _run_status(args)
        assert result == 0
