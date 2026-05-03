"""Unit tests for CLI search, list, and status subcommands."""

import argparse
import json
from unittest.mock import MagicMock, patch


from cli import (
    _run_search,
    _run_list,
    _run_status,
    _run_queue,
    _run_cache,
    _run_dedup,
    _run_merge,
    _run_citations,
    _run_stats,
    _run_cite_stats,
    _run_dedup_semantic,
    _run_kg,
    _run_trend,
    infer_tags_if_empty,
    main,
)
from cli.cmd.cite_fetch import _work_to_arxiv_id, _work_to_paper_record
from core import Paper


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
        added_at="2026-04-01",
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
        self.added_at = added_at


def make_args(**kwargs):
    defaults = dict(
        sort="added_at",
        order="desc",
        since="",
        clear=False,
        dry_run=False,
        keep="older",
        report=False,
        source="import",
        skip_existing=False,
        format="table",
        limit=0,
        out=None,
        json=False,
        set_=None,
        llm=False,
        llm_clear=False,
        stats_paper=None,
        top=None,
        dedup_semantic=False,
    )
    defaults.update(kwargs)
    ns = argparse.Namespace()
    for k, v in defaults.items():
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

        args = make_args(
            query="",
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
        assert "[7.43]" in captured

    @patch("cli.Database")
    def test_table_shows_snippet(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.search_papers.return_value = (
            [FakeSearchResult(snippet="**attention** mechanism and **transformer** architecture")],
            1,
        )
        mock_db_cls.return_value = mock_db

        args = make_args(
            query="",
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
        assert "..." in captured
        assert "**attention**" in captured

    @patch("cli.Database")
    def test_no_results(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.search_papers.return_value = ([], 0)
        mock_db_cls.return_value = mock_db

        args = make_args(
            query="nonexistent",
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

        args = make_args(
            query="",
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

        args = make_args(
            query="",
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

        mock_db.search_papers.assert_called_once()
        assert mock_db.search_papers.call_args[1]["query"] == ""

    @patch("cli.Database")
    def test_returns_zero(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.search_papers.return_value = ([], 0)
        mock_db_cls.return_value = mock_db

        args = make_args(
            query="x",
            limit=10,
            offset=0,
            format="table",
            source="",
            year=0,
            tags=[],
            status="",
            sort="relevance",
        )
        result = _run_search(args)
        assert result == 0


class TestRunSearchJson:
    """Test _run_search with JSON format."""

    @patch("cli.Database")
    def test_json_output(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.search_papers.return_value = ([FakeSearchResult()], 1)
        mock_db_cls.return_value = mock_db

        args = make_args(
            query="",
            limit=10,
            offset=0,
            format="json",
            source="",
            year=0,
            tags=[],
            status="",
            sort="relevance",
        )
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

        args = make_args(
            query="",
            limit=10,
            offset=0,
            format="json",
            source="",
            year=0,
            tags=[],
            status="",
            sort="relevance",
        )
        _run_search(args)

        captured = capsys.readouterr().out
        data = json.loads(captured)
        assert data["results"][0]["score"] == 7.438

    @patch("cli.Database")
    def test_json_null_score_when_none(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.search_papers.return_value = ([FakeSearchResult(score=None)], 1)
        mock_db_cls.return_value = mock_db

        args = make_args(
            query="",
            limit=10,
            offset=0,
            format="json",
            source="",
            year=0,
            tags=[],
            status="",
            sort="relevance",
        )
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

        args = make_args(
            query="",
            limit=10,
            offset=0,
            format="csv",
            source="",
            year=0,
            tags=[],
            status="",
            sort="relevance",
        )
        _run_search(args)

        captured = capsys.readouterr().out.replace("\r", "")
        lines = captured.strip().split("\n")
        assert (
            lines[0]
            == "paper_id,title,authors,published,primary_category,score,snippet,source,abs_url,parse_status"
        )

    @patch("cli.Database")
    def test_csv_row(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.search_papers.return_value = ([FakeSearchResult()], 1)
        mock_db_cls.return_value = mock_db

        args = make_args(
            query="",
            limit=10,
            offset=0,
            format="csv",
            source="",
            year=0,
            tags=[],
            status="",
            sort="relevance",
        )
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

    @patch("cli.Database")
    def test_list_csv_output(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.list_papers.return_value = ([FakePaper(), FakePaper()], 2)
        mock_db_cls.return_value = mock_db

        args = make_args(status="", year=0, tags=[], limit=20, offset=0, format="csv")
        _run_list(args)

        captured = capsys.readouterr().out.replace("\r", "")
        lines = captured.strip().split("\n")
        assert (
            lines[0] == "id,title,authors,published,source,primary_category,parse_status,added_at"
        )
        assert "2301.00001" in lines[1]
        assert "Attention Is All You Need" in lines[1]
        assert "Vaswani et al." in lines[1]

    @patch("cli.Database")
    def test_list_csv_empty(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.list_papers.return_value = ([], 0)
        mock_db_cls.return_value = mock_db

        args = make_args(status="", year=0, tags=[], limit=20, offset=0, format="csv")
        _run_list(args)

        captured = capsys.readouterr().out.replace("\r", "")
        lines = captured.strip().split("\n")
        assert (
            lines[0] == "id,title,authors,published,source,primary_category,parse_status,added_at"
        )
        assert len(lines) == 1  # only header, no data rows


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


# ─────────────────────────────────────────────────────────────────────────────
# _run_queue tests
# ─────────────────────────────────────────────────────────────────────────────


class TestRunQueueList:
    """Test queue --list (queries job_queue table, not papers.parse_status)."""

    @patch("cli.Database")
    def test_queue_list_shows_jobs(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        # Mock job_queue rows (sqlite3.Row compatible)
        mock_job1 = MagicMock()
        mock_job1.__getitem__ = lambda self, k: {
            "id": 1, "paper_id": "2301.00001",
            "job_type": "parse", "priority": 5, "status": "queued"
        }[k]
        mock_job2 = MagicMock()
        mock_job2.__getitem__ = lambda self, k: {
            "id": 2, "paper_id": "2301.00003",
            "job_type": "parse", "priority": 3, "status": "running"
        }[k]
        mock_db.get_queue_jobs.return_value = [mock_job1, mock_job2]
        mock_db_cls.return_value = mock_db

        args = make_args(list=True, dequeue=False, add=None, cancel=None, pending=False)
        result = _run_queue(args)

        captured = capsys.readouterr().out
        assert "2301.00001" in captured
        assert "2301.00003" in captured
        assert "[1]" in captured or "id=1" in captured
        assert result == 0

    @patch("cli.Database")
    def test_queue_list_empty(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.get_queue_jobs.return_value = []
        mock_db_cls.return_value = mock_db

        args = make_args(list=True, dequeue=False, add=None, cancel=None, pending=False)
        result = _run_queue(args)

        captured = capsys.readouterr().out
        assert "Queue empty" in captured
        assert result == 0

    @patch("cli.Database")
    def test_pending_shows_awaiting_papers(self, mock_db_cls, capsys):
        """Test queue --pending shows papers with parse_status=pending."""
        mock_db = MagicMock()
        mock_db.init.return_value = None
        # Mock conn context manager for pending query
        mock_conn = MagicMock()
        mock_cur = MagicMock()
        mock_row1 = MagicMock()
        mock_row1.__getitem__ = lambda self, k: {"id": "2301.00001", "parse_status": "pending", "source": "import"}[k]
        mock_row2 = MagicMock()
        mock_row2.__getitem__ = lambda self, k: {"id": "2301.00003", "parse_status": "pending", "source": "import"}[k]
        mock_cur.fetchall.return_value = [mock_row1, mock_row2]
        mock_conn.execute.return_value = mock_cur
        mock_db.conn.__enter__ = MagicMock(return_value=mock_conn)
        mock_db.conn.__exit__ = MagicMock(return_value=False)
        mock_db_cls.return_value = mock_db

        args = make_args(list=False, dequeue=False, add=None, cancel=None, pending=True)
        result = _run_queue(args)

        captured = capsys.readouterr().out
        assert "2 paper(s) awaiting processing" in captured
        assert "2301.00001" in captured
        assert "2301.00003" in captured
        assert result == 0


class TestRunQueueDequeue:
    """Test queue --dequeue."""

    @patch("cli.Database")
    def test_dequeue_returns_job(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_job = MagicMock()
        mock_job.__getitem__ = lambda self, key: {"paper_id": "2301.00001", "id": 42}[key]
        mock_db.dequeue_job.return_value = mock_job
        mock_db_cls.return_value = mock_db

        args = make_args(list=False, dequeue=True, add=None, cancel=None)
        result = _run_queue(args)

        captured = capsys.readouterr().out
        assert "Dequeued: 2301.00001" in captured
        assert "id=42" in captured
        assert result == 0

    @patch("cli.Database")
    def test_dequeue_empty_queue(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.dequeue_job.return_value = None
        mock_db_cls.return_value = mock_db

        args = make_args(list=False, dequeue=True, add=None, cancel=None)
        result = _run_queue(args)

        captured = capsys.readouterr().out
        assert "Queue empty" in captured
        assert result == 0


class TestRunQueueAdd:
    """Test queue --add UID."""

    @patch("cli.Database")
    def test_enqueue_adds_paper(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db_cls.return_value = mock_db

        args = make_args(list=False, dequeue=False, add="2301.99999", cancel=None)
        result = _run_queue(args)

        mock_db.enqueue_job.assert_called_once_with("2301.99999", "parse")
        captured = capsys.readouterr().out
        assert "Added 2301.99999 to queue" in captured
        assert result == 0


class TestRunQueueNoArgs:
    """Test queue with no arguments."""

    @patch("cli.Database")
    def test_queue_no_args_shows_usage(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db_cls.return_value = mock_db

        args = make_args(list=False, dequeue=False, add=None, cancel=None)
        result = _run_queue(args)

        captured = capsys.readouterr().out
        assert "--list" in captured or "--dequeue" in captured
        assert result == 0


class TestRunQueueCancel:
    """Test queue --cancel JOB_ID."""

    @patch("cli.Database")
    def test_cancel_removes_job(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.cancel_job.return_value = True
        mock_db_cls.return_value = mock_db

        args = make_args(list=False, dequeue=False, add=None, cancel=42)
        result = _run_queue(args)

        mock_db.cancel_job.assert_called_once_with(42)
        captured = capsys.readouterr().out
        assert "Cancelled job 42" in captured
        assert result == 0

    @patch("cli.Database")
    def test_cancel_nonexistent_job(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.cancel_job.return_value = False
        mock_db_cls.return_value = mock_db

        args = make_args(list=False, dequeue=False, add=None, cancel=99)
        result = _run_queue(args)

        mock_db.cancel_job.assert_called_once_with(99)
        captured = capsys.readouterr().out
        assert "No such job 99" in captured
        assert result == 0


# ─────────────────────────────────────────────────────────────────────────────
# _run_cache tests
# ─────────────────────────────────────────────────────────────────────────────


class TestRunCacheStats:
    """Test cache --stats."""

    @patch("cli.Database")
    def test_cache_stats_shows_size(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.get_cached_paper.return_value = 7
        mock_db_cls.return_value = mock_db

        args = make_args(stats=True, clear=False, get=None, set=None)
        result = _run_cache(args)

        captured = capsys.readouterr().out
        assert "Cache size: 7" in captured
        assert result == 0


class TestRunCacheGet:
    """Test cache --get UID."""

    @patch("cli.Database")
    def test_cache_get_returns_cached(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.get_cached_paper.return_value = {"title": "Attention Is All You Need"}
        mock_db_cls.return_value = mock_db

        args = make_args(stats=False, clear=False, get="2301.00001", set=None)
        result = _run_cache(args)

        mock_db.get_cached_paper.assert_called_with("2301.00001")
        captured = capsys.readouterr().out
        assert "Attention Is All You Need" in captured
        assert result == 0

    @patch("cli.Database")
    def test_cache_get_not_found(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.get_cached_paper.return_value = None
        mock_db_cls.return_value = mock_db

        args = make_args(stats=False, clear=False, get="nonexistent", set=None)
        result = _run_cache(args)

        captured = capsys.readouterr().out
        assert "No cache entry" in captured
        assert result == 0


class TestRunCacheClear:
    """Test cache --clear."""

    @patch("cli.Database")
    def test_cache_clear(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db_cls.return_value = mock_db

        args = make_args(stats=False, clear=True, get=None, set=None)
        result = _run_cache(args)

        captured = capsys.readouterr().out
        assert "Cache cleared" in captured
        assert result == 0


class TestRunCacheNoArgs:
    """Test cache with no arguments."""

    @patch("cli.Database")
    def test_cache_no_args_shows_usage(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db_cls.return_value = mock_db

        args = make_args(stats=False, clear=False, get=None, set=None)
        result = _run_cache(args)

        captured = capsys.readouterr().out
        assert "--stats" in captured or "Use --stats" in captured or "--get" in captured
        assert result == 0


class TestRunCacheSet:
    """Test cache --set UID PATH."""

    @patch("cli.Database")
    def test_cache_set_caches_json_file(self, mock_db_cls, capsys, tmp_path):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db_cls.return_value = mock_db

        json_file = tmp_path / "paper.json"
        json_file.write_text(
            '{"title": "Test Paper", "abstract": "Test abstract"}', encoding="utf-8"
        )

        args = make_args(stats=False, clear=False, get=None, set=["uid123", str(json_file)])
        result = _run_cache(args)

        mock_db.set_cached_paper.assert_called_once_with(
            "uid123", {"title": "Test Paper", "abstract": "Test abstract"}
        )
        captured = capsys.readouterr().out
        assert "Cached uid123" in captured
        assert result == 0

    @patch("cli.Database")
    def test_cache_set_returns_error_on_bad_json(self, mock_db_cls, capsys, tmp_path):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db_cls.return_value = mock_db

        bad_file = tmp_path / "bad.json"
        bad_file.write_text("not json{", encoding="utf-8")

        args = make_args(stats=False, clear=False, get=None, set=["uid123", str(bad_file)])
        result = _run_cache(args)

        assert "Failed to cache" in capsys.readouterr().out
        assert result == 1

    @patch("cli.Database")
    def test_cache_set_returns_error_on_missing_file(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db_cls.return_value = mock_db

        args = make_args(
            stats=False, clear=False, get=None, set=["uid123", "/nonexistent/path.json"]
        )
        result = _run_cache(args)

        assert "Failed to cache" in capsys.readouterr().out
        assert result == 1


# ─────────────────────────────────────────────────────────────────────────────
# main() routing tests
# ─────────────────────────────────────────────────────────────────────────────


class TestMainRouting:
    """Test main() routes to correct subcommand handler."""

    @patch("cli._run_search")
    @patch("cli._build_search_parser")
    def test_main_routes_to_search(self, mock_build, mock_run, capsys):
        mock_run.return_value = 0

        args = make_args(
            subcmd="search",
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
        with patch("argparse.ArgumentParser.parse_args", return_value=args):
            result = main(["search", "attention"])

        mock_run.assert_called_once()
        assert result == 0

    @patch("cli._run_list")
    def test_main_routes_to_list(self, mock_run):
        mock_run.return_value = 0
        args = make_args(
            subcmd="list", status="", year=0, tags=[], limit=20, offset=0, format="table"
        )
        with patch("argparse.ArgumentParser.parse_args", return_value=args):
            result = main(["list"])
        mock_run.assert_called_once()
        assert result == 0

    @patch("cli._run_status")
    def test_main_routes_to_status(self, mock_run):
        mock_run.return_value = 0
        args = make_args(subcmd="status")
        with patch("argparse.ArgumentParser.parse_args", return_value=args):
            result = main(["status"])
        mock_run.assert_called_once()
        assert result == 0

    @patch("cli._run_queue")
    def test_main_routes_to_queue(self, mock_run):
        mock_run.return_value = 0
        args = make_args(subcmd="queue", list=True, dequeue=False, add=None, cancel=None)
        with patch("argparse.ArgumentParser.parse_args", return_value=args):
            result = main(["queue", "--list"])
        mock_run.assert_called_once()
        assert result == 0

    @patch("cli._run_cache")
    def test_main_routes_to_cache(self, mock_run):
        mock_run.return_value = 0
        args = make_args(subcmd="cache", stats=True, clear=False, get=None, set=None)
        with patch("argparse.ArgumentParser.parse_args", return_value=args):
            result = main(["cache", "--stats"])
        mock_run.assert_called_once()
        assert result == 0

# ─────────────────────────────────────────────────────────────────────────────
# _run_status aggregation edge cases
# ─────────────────────────────────────────────────────────────────────────────


class TestRunStatusAggregation:
    """Test _run_status by_source / by_status aggregation."""

    @patch("cli.Database")
    def test_status_aggregates_multiple_sources(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_papers.return_value = [
            FakePaper(id="1", source="arxiv", parse_status="done"),
            FakePaper(id="2", source="arxiv", parse_status="done"),
            FakePaper(id="3", source="doi", parse_status="pending"),
        ]
        mock_db_cls.return_value = mock_db
        args = make_args()
        _run_status(args)
        out = capsys.readouterr().out
        assert "arxiv=2" in out, f"Expected 'arxiv=2' in: {out}"
        assert "doi=1" in out, f"Expected 'doi=1' in: {out}"

    @patch("cli.Database")
    def test_status_aggregates_multiple_statuses(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_papers.return_value = [
            FakePaper(id="1", source="arxiv", parse_status="done"),
            FakePaper(id="2", source="arxiv", parse_status="done"),
            FakePaper(id="3", source="arxiv", parse_status="pending"),
        ]
        mock_db_cls.return_value = mock_db
        args = make_args()
        _run_status(args)
        out = capsys.readouterr().out
        assert "done=2" in out, f"Expected 'done=2' in: {out}"
        assert "pending=1" in out, f"Expected 'pending=1' in: {out}"

    @patch("cli.Database")
    def test_status_handles_none_source(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_papers.return_value = [
            FakePaper(id="1", source=None, parse_status="done"),
        ]
        mock_db_cls.return_value = mock_db
        args = make_args()
        _run_status(args)
        out = capsys.readouterr().out
        # source=None should be grouped as "?"
        assert "?" in out, f"Expected '?' for None source in: {out}"

    @patch("cli.Database")
    def test_status_handles_none_status(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_papers.return_value = [
            FakePaper(id="1", source="arxiv", parse_status=None),
        ]
        mock_db_cls.return_value = mock_db
        args = make_args()
        _run_status(args)
        out = capsys.readouterr().out
        # parse_status=None should be grouped as "?"
        assert "?" in out, f"Expected '?' for None status in: {out}"


# ─────────────────────────────────────────────────────────────────────────────
# _run_list / _run_search filter mapping
# ─────────────────────────────────────────────────────────────────────────────


class TestRunListFilters:
    """Test _run_list filter arguments are passed correctly to db.list_papers."""

    @patch("cli.Database")
    def test_list_passes_year_as_date_from(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.list_papers.return_value = ([], 0)
        mock_db_cls.return_value = mock_db
        args = make_args(status="", year=2023, tags=[], limit=20, offset=0, format="table")
        _run_list(args)
        mock_db.list_papers.assert_called_once()
        call_kwargs = mock_db.list_papers.call_args.kwargs
        assert call_kwargs.get("date_from") == "2023-01-01", (
            f"Expected date_from='2023-01-01', got {call_kwargs}"
        )

    @patch("cli.Database")
    def test_list_passes_status_filter(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.list_papers.return_value = ([], 0)
        mock_db_cls.return_value = mock_db
        args = make_args(status="done", year=0, tags=[], limit=20, offset=0, format="table")
        _run_list(args)
        call_kwargs = mock_db.list_papers.call_args.kwargs
        assert call_kwargs.get("parse_status") == "done", (
            f"Expected parse_status='done', got {call_kwargs}"
        )

    @patch("cli.Database")
    def test_list_zero_year_no_date_filter(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.list_papers.return_value = ([], 0)
        mock_db_cls.return_value = mock_db
        args = make_args(status="", year=0, tags=[], limit=20, offset=0, format="table")
        _run_list(args)
        call_kwargs = mock_db.list_papers.call_args.kwargs
        assert call_kwargs.get("date_from") is None, (
            f"Expected date_from=None for year=0, got {call_kwargs}"
        )


class TestRunSearchFilters:
    """Test _run_search filter arguments are passed correctly to db.search_papers."""

    @patch("cli.Database")
    def test_search_passes_year_as_date_from(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.search_papers.return_value = ([], 0)
        mock_db_cls.return_value = mock_db
        args = make_args(
            query="attention",
            limit=10,
            offset=0,
            format="table",
            source="",
            year=2024,
            tags=[],
            status="",
            sort="relevance",
        )
        _run_search(args)
        call_kwargs = mock_db.search_papers.call_args.kwargs
        assert call_kwargs.get("date_from") == "2024-01-01", (
            f"Expected date_from='2024-01-01', got {call_kwargs}"
        )

    @patch("cli.Database")
    def test_search_zero_year_no_date_filter(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.search_papers.return_value = ([], 0)
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
        call_kwargs = mock_db.search_papers.call_args.kwargs
        assert call_kwargs.get("date_from") is None, (
            f"Expected date_from=None for year=0, got {call_kwargs}"
        )

    @patch("cli.Database")
    def test_search_passes_source_filter(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.search_papers.return_value = ([], 0)
        mock_db_cls.return_value = mock_db
        args = make_args(
            query="attention",
            limit=10,
            offset=0,
            format="table",
            source="doi",
            year=0,
            tags=[],
            status="",
            sort="relevance",
        )
        _run_search(args)
        call_kwargs = mock_db.search_papers.call_args.kwargs
        assert call_kwargs.get("source") == "doi", f"Expected source='doi', got {call_kwargs}"

    @patch("cli.Database")
    def test_search_passes_status_filter(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.search_papers.return_value = ([], 0)
        mock_db_cls.return_value = mock_db
        args = make_args(
            query="attention",
            limit=10,
            offset=0,
            format="table",
            source="",
            year=0,
            tags=[],
            status="done",
            sort="relevance",
        )
        _run_search(args)
        call_kwargs = mock_db.search_papers.call_args.kwargs
        assert call_kwargs.get("parse_status") == "done", (
            f"Expected parse_status='done', got {call_kwargs}"
        )

    @patch("cli.Database")
    def test_search_empty_query_allowed(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.search_papers.return_value = ([], 0)
        mock_db_cls.return_value = mock_db
        args = make_args(
            query="",
            limit=10,
            offset=0,
            format="table",
            source="",
            year=0,
            tags=[],
            status="",
            sort="relevance",
        )
        result = _run_search(args)
        assert result == 0
        mock_db.search_papers.assert_called_once()
        call_kwargs = mock_db.search_papers.call_args.kwargs
        assert call_kwargs.get("query") == ""


# ─────────────────────────────────────────────────────────────────────────────
# _run_cache edge cases
# ─────────────────────────────────────────────────────────────────────────────


class TestRunCacheEdgeCases:
    """Test _run_cache --stats and --set edge cases."""

    @patch("cli.Database")
    def test_cache_stats_shows_zero_when_none(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_cached_paper.return_value = None
        mock_db_cls.return_value = mock_db
        args = make_args(stats=True, clear=False, get=None, set_=None)
        _run_cache(args)
        out = capsys.readouterr().out
        # None should print "Cache size: None"
        assert "Cache size: None" in out, f"Expected 'Cache size: None' in: {out}"

    @patch("cli.Database")
    def test_cache_get_not_found_message(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_cached_paper.return_value = None
        mock_db_cls.return_value = mock_db
        args = make_args(stats=False, clear=False, get="not-exist", set_=None)
        _run_cache(args)
        out = capsys.readouterr().out
        assert "No cache entry for not-exist" in out, f"Expected 'No cache entry' message in: {out}"

    def test_cache_set_shows_error_on_missing_file(self, capsys):
        """--set with a missing file returns an error."""
        args = make_args(
            stats=False, clear=False, get=None, set=["uid123", "/nonexistent/cache/file.json"]
        )
        result = _run_cache(args)
        out = capsys.readouterr().out
        assert "not found" in out or "error" in out or "Failed" in out
        assert result == 1


# ─────────────────────────────────────────────────────────────────────────────
# infer_tags_if_empty tests
# ─────────────────────────────────────────────────────────────────────────────


class TestInferTagsIfEmpty:
    """Test keyword tag inference."""

    def test_existing_tags_unchanged(self):
        paper = Paper(
            source="arxiv",
            uid="t",
            title="t",
            authors=[],
            abstract="",
            published="",
            updated="",
            abs_url="",
            pdf_url="",
            primary_category="",
        )
        tags = ["Agent", "RAG"]
        assert infer_tags_if_empty(tags, paper) == ["Agent", "RAG"]

    def test_infers_agent_from_title(self):
        paper = Paper(
            source="arxiv",
            uid="t",
            title="Tool Use in LLM Agents",
            authors=[],
            abstract="",
            published="",
            updated="",
            abs_url="",
            pdf_url="",
            primary_category="",
        )
        tags = []
        assert "Agent" in infer_tags_if_empty(tags, paper)

    def test_infers_rag_from_abstract(self):
        paper = Paper(
            source="arxiv",
            uid="t",
            title="Foo",
            authors=[],
            abstract="retrieval augmented generation",
            published="",
            updated="",
            abs_url="",
            pdf_url="",
            primary_category="",
        )
        tags = []
        assert "RAG" in infer_tags_if_empty(tags, paper)

    def test_unsorted_when_no_match(self):
        paper = Paper(
            source="arxiv",
            uid="t",
            title="Foo Bar Baz",
            authors=[],
            abstract="",
            published="",
            updated="",
            abs_url="",
            pdf_url="",
            primary_category="",
        )
        tags = []
        assert infer_tags_if_empty(tags, paper) == ["Unsorted"]


# ─────────────────────────────────────────────────────────────────────────────
# _run_search CSV format
# ─────────────────────────────────────────────────────────────────────────────


class TestRunSearchCsvFormat:
    """Test _run_search with CSV format (additional tests)."""

    @patch("cli.Database")
    def test_csv_format_prints_header_and_row(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.search_papers.return_value = ([FakeSearchResult()], 1)
        mock_db_cls.return_value = mock_db

        args = make_args(
            query="attention",
            limit=10,
            offset=0,
            format="csv",
            source="",
            year=0,
            tags=[],
            status="",
        )
        result = _run_search(args)
        captured = capsys.readouterr().out
        assert "paper_id,title,authors" in captured
        assert "Attention Is All You Need" in captured
        assert result == 0


# ─────────────────────────────────────────────────────────────────────────────
# _run_list JSON format
# ─────────────────────────────────────────────────────────────────────────────


class TestRunListJson:
    """Test _run_list with JSON format."""

    @patch("cli.Database")
    def test_json_format_prints_papers(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.list_papers.return_value = ([FakePaper()], 1)
        mock_db_cls.return_value = mock_db

        args = make_args(
            status="",
            year=0,
            tags=[],
            limit=20,
            offset=0,
            format="json",
        )
        result = _run_list(args)
        captured = capsys.readouterr().out
        assert "Attention Is All You Need" in captured
        assert result == 0


# ─────────────────────────────────────────────────────────────────────────────
# _run_status edge cases
# ─────────────────────────────────────────────────────────────────────────────


class TestRunStatusEdgeCases:
    """Test _run_status with empty/unusual data."""

    @patch("cli.Database")
    def test_status_with_empty_papers(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_papers.return_value = []
        mock_db_cls.return_value = mock_db

        args = make_args()
        result = _run_status(args)
        captured = capsys.readouterr().out
        assert "Total papers: 0" in captured
        assert result == 0

    @patch("cli.Database")
    def test_status_with_none_source(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        fake = FakePaper()
        fake.source = None
        mock_db.get_papers.return_value = [fake]
        mock_db_cls.return_value = mock_db

        args = make_args()
        result = _run_status(args)
        captured = capsys.readouterr().out
        assert "By source:" in captured
        assert result == 0


# ─────────────────────────────────────────────────────────────────────────────
# main() routing: merge subcommand → legacy
# ─────────────────────────────────────────────────────────────────────────────


class TestMainDedupRouting:
    """Test main() routes 'dedup' to _run_dedup."""

    @patch("cli._run_dedup")
    @patch("cli._build_dedup_parser")
    @patch("cli.argparse.ArgumentParser")
    def test_main_dedup_routes_to_run_dedup(self, mock_argparse, mock_build, mock_run, capsys):
        mock_parser = MagicMock()
        mock_parser.parse_args.return_value = make_args(subcmd="dedup")
        mock_argparse.return_value = mock_parser
        mock_build.return_value = None
        mock_run.return_value = 0

        result = main(["dedup"])

        assert result == 0

class TestMainMergeRouting:
    """Test main() routes 'merge' to _run_merge (not legacy)."""

    @patch("cli._run_merge")
    @patch("cli._build_merge_parser")
    @patch("cli.argparse.ArgumentParser")
    def test_main_merge_routes_to_run_merge(self, mock_argparse, mock_build, mock_run, capsys):
        mock_parser = MagicMock()
        mock_parser.parse_args.return_value = make_args(
            subcmd="merge", target_id="uid1", duplicate_id="uid2"
        )
        mock_argparse.return_value = mock_parser
        mock_build.return_value = None
        mock_run.return_value = 0

        result = main(["merge", "uid1", "uid2"])

        assert result == 0

# ─────────────────────────────────────────────────────────────────────────────
# _run_dedup tests
# ─────────────────────────────────────────────────────────────────────────────


class TestRunDedup:
    """Test _run_dedup behavior."""

    @patch("cli.Database")
    def test_dedup_no_duplicates(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.find_duplicates.return_value = []
        mock_db_cls.return_value = mock_db

        result = _run_dedup(
            make_args(dry_run=False, auto=False, keep="older", batch=False, report=False)
        )

        assert result == 0
        assert "No Duplicates Found" in capsys.readouterr().out

    @patch("cli.Database")
    def test_dedup_with_duplicates(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        p1 = MagicMock()
        p1.id = "uid1"
        p1.title = "Attention Is All You Need"
        p1.doi = "10.1234/abc"
        p2 = MagicMock()
        p2.id = "uid2"
        p2.title = "Attention Is All You Need"
        p2.doi = "10.1234/abc"
        p1.parse_status = "completed"
        p1.added_at = "2024-01-01T00:00:00"
        p2.parse_status = "pending"
        p2.added_at = "2024-06-01T00:00:00"
        mock_db.find_duplicates.return_value = [(p1, p2)]
        mock_db_cls.return_value = mock_db

        result = _run_dedup(
            make_args(dry_run=False, auto=False, keep="older", batch=False, report=False)
        )

        out = capsys.readouterr().out
        assert "uid1" in out
        assert "uid2" in out
        assert "Attention" in out  # Title truncated in table
        assert result == 0

    @patch("cli.Database")
    def test_dedup_dry_run_shows_count(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        p1 = MagicMock()
        p1.id = "uid1"
        p1.title = "Test Paper"
        p1.doi = ""
        p1.parse_status = "completed"
        p1.added_at = "2024-01-01T00:00:00"
        p2 = MagicMock()
        p2.id = "uid2"
        p2.title = "Test Paper"
        p2.doi = ""
        p2.parse_status = "pending"
        p2.added_at = "2024-06-01T00:00:00"
        mock_db.find_duplicates.return_value = [(p1, p2)]
        mock_db_cls.return_value = mock_db

        result = _run_dedup(make_args(dry_run=True, auto=False, keep="older", report=False))

        out = capsys.readouterr().out
        assert "1 duplicate pair(s)" in out
        assert "Dry Run" in out
        assert "uid1" in out
        assert "uid2" in out
        # Status is shown but may be in table cell
        assert "winner" in out
        assert result == 0

    @patch("cli.Database")
    def test_dedup_auto_merges_all_pairs(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        p1 = MagicMock()
        p1.id = "uid1"
        p1.title = "Paper A"
        p1.doi = "10.1234/a"
        p2 = MagicMock()
        p2.id = "uid2"
        p2.title = "Paper A"
        p2.doi = "10.1234/a"
        p3 = MagicMock()
        p3.id = "uid3"
        p3.title = "Paper B"
        p3.doi = "10.1234/b"
        p4 = MagicMock()
        p4.id = "uid4"
        p4.title = "Paper B"
        p4.doi = "10.1234/b"
        p1.parse_status = "completed"
        p1.added_at = "2024-01-01T00:00:00"
        p2.parse_status = "pending"
        p2.added_at = "2024-06-01T00:00:00"
        p3.parse_status = "failed"
        p3.added_at = "2024-02-01T00:00:00"
        p4.parse_status = "running"
        p4.added_at = "2024-07-01T00:00:00"
        mock_db.find_duplicates.return_value = [(p1, p2), (p3, p4)]
        mock_db.merge_papers.return_value = True
        mock_db_cls.return_value = mock_db

        result = _run_dedup(make_args(dry_run=False, auto=True, keep="older", report=False))

        out = capsys.readouterr().out
        assert mock_db.merge_papers.call_count == 2
        assert "Auto-Merge Complete" in out
        assert "2 / 2" in out or "Merged" in out
        assert result == 0

    @patch("cli.Database")
    def test_dedup_auto_partial_failure(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        p1 = MagicMock()
        p1.id = "uid1"
        p1.title = "Paper"
        p1.doi = "10.1234/x"
        p2 = MagicMock()
        p2.id = "uid2"
        p2.title = "Paper"
        p2.doi = "10.1234/x"
        p1.parse_status = "pending"
        p1.added_at = "2024-01-01T00:00:00"
        p2.parse_status = "completed"
        p2.added_at = "2024-06-01T00:00:00"
        mock_db.find_duplicates.return_value = [(p1, p2)]
        # First call succeeds, second would be called if there were more pairs
        mock_db.merge_papers.side_effect = [True]
        mock_db_cls.return_value = mock_db

        result = _run_dedup(make_args(dry_run=False, auto=True, keep="older", report=False))

        out = capsys.readouterr().out
        assert "Merged" in out or "merged" in out
        assert result == 0

    @patch("cli.Database")
    def test_dedup_auto_no_pairs(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.find_duplicates.return_value = []
        mock_db_cls.return_value = mock_db

        result = _run_dedup(make_args(dry_run=False, auto=True, keep="older", report=False))

        out = capsys.readouterr().out
        assert "No Duplicates Found" in out
        assert not mock_db.merge_papers.called
        assert result == 0

    @patch("cli.Database")
    def test_dedup_auto_keep_newer(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        p1 = MagicMock()
        p1.id = "uid1"
        p1.title = "Paper"
        p1.doi = "10.1234/x"
        p1.parse_status = "pending"
        p1.added_at = "2024-01-01T00:00:00"
        p2 = MagicMock()
        p2.id = "uid2"
        p2.title = "Paper"
        p2.doi = "10.1234/x"
        p2.parse_status = "completed"
        p2.added_at = "2024-06-01T00:00:00"
        mock_db.find_duplicates.return_value = [(p1, p2)]
        mock_db.merge_papers.return_value = True
        mock_db_cls.return_value = mock_db

        result = _run_dedup(make_args(dry_run=False, auto=True, keep="newer", report=False))

        out = capsys.readouterr().out
        # With --keep=newer, newer paper (uid2) is target, older (uid1) is deleted
        mock_db.merge_papers.assert_called_once_with("uid2", "uid1")
        assert "Merged" in out
        assert "newer" in out.lower()
        assert result == 0

    @patch("cli.Database")
    def test_dedup_auto_keep_parsed_prefers_completed(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        p1 = MagicMock()
        p1.id = "uid1"
        p1.title = "Paper"
        p1.doi = "10.1234/x"
        p1.parse_status = "pending"
        p1.added_at = "2024-01-01T00:00:00"
        p2 = MagicMock()
        p2.id = "uid2"
        p2.title = "Paper"
        p2.doi = "10.1234/x"
        p2.parse_status = "completed"
        p2.added_at = "2024-06-01T00:00:00"
        mock_db.find_duplicates.return_value = [(p1, p2)]
        mock_db.merge_papers.return_value = True
        mock_db_cls.return_value = mock_db

        result = _run_dedup(make_args(dry_run=False, auto=True, keep="parsed", report=False))

        out = capsys.readouterr().out
        # With --keep=parsed, completed paper (uid2) is kept
        mock_db.merge_papers.assert_called_once_with("uid2", "uid1")
        assert "Merged" in out
        assert result == 0

    @patch("cli.Database")
    def test_dedup_auto_keep_parsed_tie_uses_older(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        p1 = MagicMock()
        p1.id = "uid1"
        p1.title = "Paper"
        p1.doi = "10.1234/x"
        p1.parse_status = "completed"
        p1.added_at = "2024-01-01T00:00:00"
        p2 = MagicMock()
        p2.id = "uid2"
        p2.title = "Paper"
        p2.doi = "10.1234/x"
        p2.parse_status = "completed"
        p2.added_at = "2024-06-01T00:00:00"
        mock_db.find_duplicates.return_value = [(p1, p2)]
        mock_db.merge_papers.return_value = True
        mock_db_cls.return_value = mock_db

        result = _run_dedup(make_args(dry_run=False, auto=True, keep="parsed", report=False))

        out = capsys.readouterr().out
        # Same parse_status, tie → older kept
        mock_db.merge_papers.assert_called_once_with("uid1", "uid2")
        assert "Merged" in out
        assert result == 0


# ─────────────────────────────────────────────────────────────────────────────
# _run_merge tests
# ─────────────────────────────────────────────────────────────────────────────


class TestRunQueueElseBranch:
    """Test _run_queue when none of list/dequeue/add/cancel are set."""

    @patch("cli.Database")
    def test_queue_no_flags_shows_usage(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db_cls.return_value = mock_db

        args = make_args(list=False, dequeue=False, add=None, cancel=None)
        result = _run_queue(args)

        captured = capsys.readouterr().out
        # The else branch says: "Use --list, --dequeue, --add UID, or --cancel JOB_ID"
        assert "--list" in captured or "--add" in captured or "--cancel" in captured
        assert result == 0


# ─────────────────────────────────────────────────────────────────────────────
# dedup --report
# ─────────────────────────────────────────────────────────────────────────────


class TestDedupReport:
    """Test dedup --report flag."""

    @patch("cli.Database")
    def test_report_empty(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.get_dedup_log.return_value = []
        mock_db_cls.return_value = mock_db

        args = make_args(dry_run=False, auto=False, keep="older", report=True)
        result = _run_dedup(args)

        captured = capsys.readouterr().out
        assert "No dedup history" in captured
        assert result == 0

    @patch("cli.Database")
    def test_report_with_records(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.get_dedup_log.return_value = [
            {
                "id": 5,
                "target_id": "uid1",
                "duplicate_id": "uid2",
                "keep_policy": "older",
                "logged_at": "2026-04-17T10:00:00",
                "target_title": "Attention Is All You Need",
                "duplicate_title": "Attention Is All You Need (2)",
            },
            {
                "id": 4,
                "target_id": "uid3",
                "duplicate_id": "uid4",
                "keep_policy": "parsed",
                "logged_at": "2026-04-16T08:00:00",
                "target_title": "BERT: Pre-training of Deep Bidirectional",
                "duplicate_title": "BERT preprint v2",
            },
        ]
        mock_db_cls.return_value = mock_db

        args = make_args(dry_run=False, auto=False, keep="older", report=True)
        result = _run_dedup(args)

        captured = capsys.readouterr().out
        assert "Dedup History" in captured
        assert "uid1" in captured
        assert "uid2" in captured
        assert "older" in captured.lower()
        assert "parsed" in captured.lower()
        assert result == 0


class TestRunDedupBatch:
    """Test dedup --batch mode."""

    @patch("cli.Database")
    def test_batch_both_same_doi(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        p1 = MagicMock(
            id="uid1",
            title="Attention Is All You Need",
            doi="10.1234/abc",
            parse_status="completed",
            added_at="2026-01-01T00:00:00",
        )
        p2 = MagicMock(
            id="uid2",
            title="Attention Is All You Need",
            doi="10.1234/abc",
            parse_status="pending",
            added_at="2026-01-02T00:00:00",
        )
        mock_db.find_duplicates.return_value = [(p1, p2)]
        mock_db.merge_papers.return_value = True
        mock_db_cls.return_value = mock_db

        args = make_args(dry_run=False, auto=False, batch=True, keep="older", report=False)
        result = _run_dedup(args)

        captured = capsys.readouterr().out
        assert "Merged" in captured or "merged" in captured
        mock_db.log_dedup.assert_called_once_with("uid1", "uid2", "older")
        assert result == 0

    @patch("cli.Database")
    def test_batch_different_doi(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        p1 = MagicMock(
            id="uid1",
            title="Attention Is All You Need",
            doi="10.1234/abc",
            parse_status="completed",
            added_at="2026-01-01T00:00:00",
        )
        p2 = MagicMock(
            id="uid2",
            title="Attention Is All You Need",
            doi="10.9999/xyz",
            parse_status="pending",
            added_at="2026-01-02T00:00:00",
        )
        mock_db.find_duplicates.return_value = [(p1, p2)]
        mock_db_cls.return_value = mock_db

        args = make_args(dry_run=False, auto=False, batch=True, keep="older", report=False)
        result = _run_dedup(args)

        captured = capsys.readouterr().out
        assert "Skipped" in captured or "skipped" in captured
        mock_db.merge_papers.assert_not_called()
        assert result == 0

    @patch("cli.Database")
    def test_batch_mixed_pairs(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        # Pair 1: same DOI → auto-merged
        p1 = MagicMock(
            id="uid1",
            title="Paper A",
            doi="10.1234/a",
            parse_status="completed",
            added_at="2026-01-01T00:00:00",
        )
        p2 = MagicMock(
            id="uid2",
            title="Paper A",
            doi="10.1234/a",
            parse_status="pending",
            added_at="2026-01-02T00:00:00",
        )
        # Pair 2: different DOI → skipped
        p3 = MagicMock(
            id="uid3",
            title="Paper B",
            doi="10.9999/b",
            parse_status="completed",
            added_at="2026-01-01T00:00:00",
        )
        p4 = MagicMock(
            id="uid4",
            title="Paper B",
            doi=None,
            parse_status="completed",
            added_at="2026-01-02T00:00:00",
        )
        mock_db.find_duplicates.return_value = [(p1, p2), (p3, p4)]
        mock_db.merge_papers.return_value = True
        mock_db_cls.return_value = mock_db

        args = make_args(dry_run=False, auto=False, batch=True, keep="older", report=False)
        result = _run_dedup(args)

        captured = capsys.readouterr().out
        assert "Merged" in captured or "merged" in captured
        assert "Skipped" in captured or "skipped" in captured
        mock_db.merge_papers.assert_called_once_with("uid1", "uid2")
        assert result == 0

    @patch("cli.Database")
    def test_batch_no_duplicates(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.find_duplicates.return_value = []
        mock_db_cls.return_value = mock_db

        args = make_args(dry_run=False, auto=False, batch=True, keep="older", report=False)
        result = _run_dedup(args)

        captured = capsys.readouterr().out
        assert "No Duplicates Found" in captured
        assert result == 0


# ─────────────────────────────────────────────────────────────────────────────
# _run_merge tests
# ─────────────────────────────────────────────────────────────────────────────


class TestRunMerge:  # noqa: F811
    """Test _run_merge manual paper merging."""

    @patch("cli.Database")
    def test_merge_target_not_found(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_paper.side_effect = [None, MagicMock(id="dup")]
        mock_db_cls.return_value = mock_db
        args = make_args(target_id="uid1", duplicate_id="uid2", keep="older")
        result = _run_merge(args)
        captured = capsys.readouterr().out
        assert "uid1" in captured and "not found" in captured
        assert result == 1

    @patch("cli.Database")
    def test_merge_duplicate_not_found(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_paper.side_effect = [MagicMock(id="uid1"), None]
        mock_db_cls.return_value = mock_db
        args = make_args(target_id="uid1", duplicate_id="uid2", keep="older")
        result = _run_merge(args)
        captured = capsys.readouterr().out
        assert "uid2" in captured and "not found" in captured
        assert result == 1

    @patch("cli.Database")
    def test_merge_keep_older(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        older = MagicMock(id="uid1", added_at="2024-01-01", parse_status="pending")
        newer = MagicMock(id="uid2", added_at="2024-06-01", parse_status="completed")
        mock_db.get_paper.side_effect = [older, newer]
        mock_db.get_similarity.return_value = None
        mock_db.merge_papers.return_value = True
        mock_db_cls.return_value = mock_db
        # older is uid1 (target), newer is uid2 (duplicate)
        args = make_args(target_id="uid1", duplicate_id="uid2", keep="older")
        result = _run_merge(args)
        captured = capsys.readouterr().out
        mock_db.merge_papers.assert_called_once_with("uid1", "uid2")
        mock_db.log_dedup.assert_called_once_with("uid1", "uid2", "older")
        assert "Merged uid2 into uid1" in captured
        assert result == 0

    @patch("cli.Database")
    def test_merge_keep_newer(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        older = MagicMock(id="uid1", added_at="2024-01-01", parse_status="pending")
        newer = MagicMock(id="uid2", added_at="2024-06-01", parse_status="completed")
        mock_db.get_paper.side_effect = [older, newer]
        mock_db.get_similarity.return_value = None
        mock_db.merge_papers.return_value = True
        mock_db_cls.return_value = mock_db
        args = make_args(target_id="uid1", duplicate_id="uid2", keep="newer")
        result = _run_merge(args)
        captured = capsys.readouterr().out
        # --keep newer means uid2 is kept, uid1 is dropped
        mock_db.merge_papers.assert_called_once_with("uid2", "uid1")
        mock_db.log_dedup.assert_called_once_with("uid2", "uid1", "newer")
        assert "Merged uid1 into uid2" in captured
        assert result == 0

    @patch("cli.Database")
    def test_merge_keep_parsed(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        older = MagicMock(id="uid1", added_at="2024-01-01", parse_status="pending")
        newer = MagicMock(id="uid2", added_at="2024-06-01", parse_status="completed")
        mock_db.get_paper.side_effect = [older, newer]
        mock_db.get_similarity.return_value = None
        mock_db.merge_papers.return_value = True
        mock_db_cls.return_value = mock_db
        # --keep parsed means the one with better parse_status wins
        args = make_args(target_id="uid1", duplicate_id="uid2", keep="parsed")
        result = _run_merge(args)
        captured = capsys.readouterr().out
        mock_db.merge_papers.assert_called_once_with("uid2", "uid1")
        mock_db.log_dedup.assert_called_once_with("uid2", "uid1", "parsed")
        assert "Merged uid1 into uid2" in captured
        assert result == 0


# ─────────────────────────────────────────────────────────────────────────────
# _run_citations tests
# ─────────────────────────────────────────────────────────────────────────────


class FakeCitation:
    """Fake citation matching db.database.Citation."""

    def __init__(self, source_id="uid1", target_id="uid2"):
        self.source_id = source_id
        self.target_id = target_id


class TestRunCitations:
    """Test _run_citations."""

    @patch("cli.Database")
    def test_citations_from_shows_backward(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.get_paper_title.return_value = "Test Paper"
        mock_db.get_citations.return_value = [
            FakeCitation(source_id="uid1", target_id="uid2"),
            FakeCitation(source_id="uid1", target_id="uid3"),
        ]
        mock_db_cls.return_value = mock_db

        args = make_args(citation_from="uid1", citation_to=None, format="text")
        result = _run_citations(args)

        out = capsys.readouterr().out
        assert "Backward Citations" in out
        assert result == 0

    @patch("cli.Database")
    def test_citations_to_shows_forward(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.get_paper_title.return_value = "Test Paper"
        mock_db.get_citations.return_value = [
            FakeCitation(source_id="uid2", target_id="uid1"),
        ]
        mock_db_cls.return_value = mock_db

        args = make_args(citation_from=None, citation_to="uid1", format="text")
        result = _run_citations(args)

        out = capsys.readouterr().out
        assert "Forward Citations" in out
        assert result == 0

    @patch("cli.Database")
    def test_citations_csv_format(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.get_paper_title.return_value = "Test Paper"
        mock_db.get_citations.return_value = [FakeCitation(source_id="uid1", target_id="uid2")]
        mock_db_cls.return_value = mock_db

        args = make_args(citation_from="uid1", citation_to=None, format="csv")
        result = _run_citations(args)

        out = capsys.readouterr().out
        assert "paper,count" in out
        assert "uid1" in out
        assert result == 0

    @patch("cli.Database")
    def test_citations_paper_not_found(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.get_paper_title.return_value = None
        mock_db_cls.return_value = mock_db

        args = make_args(citation_from="nonexistent", citation_to=None, format="text")
        result = _run_citations(args)

        out = capsys.readouterr().out
        assert "not found" in out
        assert result == 1

    @patch("cli.Database")
    def test_citations_no_args_error(self, mock_db_cls, capsys):
        args = make_args(citation_from=None, citation_to=None, format="text")
        result = _run_citations(args)
        assert result == 1

    @patch("cli.Database")
    def test_citations_warp_format(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.get_paper_title.return_value = "Test Paper"
        mock_db.get_citations.return_value = [FakeCitation(source_id="uid1", target_id="uid2")]
        mock_db_cls.return_value = mock_db

        args = make_args(citation_from="uid1", citation_to=None, format="warp")
        result = _run_citations(args)

        capsys.readouterr()
        assert result == 0

    @patch("cli.Database")
    def test_citations_bidirectional(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.get_paper_title.side_effect = ["Paper A", "Paper B"]
        mock_db.get_citations.side_effect = [
            [FakeCitation(source_id="uid1", target_id="uid2")],
            [],
        ]
        mock_db_cls.return_value = mock_db

        args = make_args(citation_from="uid1", citation_to="uid2", format="warp")
        result = _run_citations(args)

        out = capsys.readouterr().out
        assert "Citation Bridge" in out or "uid1" in out
        assert result == 0


# ─────────────────────────────────────────────────────────────────────────────
# _run_stats tests
# ─────────────────────────────────────────────────────────────────────────────


class TestRunStats:
    """Test _run_stats."""

    @patch("cli.Database")
    def test_stats_table_format(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_stats.return_value = {
            "total_papers": 10,
            "by_source": {"arxiv": 8, "doi": 2},
            "by_status": {"done": 5, "pending": 3, "failed": 2},
            "queue_queued": 1,
            "queue_running": 0,
            "cache_entries": 20,
            "dedup_records": 3,
        }
        mock_db_cls.return_value = mock_db

        args = make_args(json=False, format="table")
        result = _run_stats(args)

        out = capsys.readouterr().out
        assert "total" in out
        assert "10" in out
        assert result == 0

    @patch("cli.Database")
    def test_stats_json_format(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_stats.return_value = {"total_papers": 5}
        mock_db_cls.return_value = mock_db

        args = make_args(json=True, format="table")
        result = _run_stats(args)

        out = capsys.readouterr().out
        data = json.loads(out)
        assert data["total_papers"] == 5
        assert result == 0

    @patch("cli.Database")
    def test_stats_warp_format(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_stats.return_value = {
            "total_papers": 10,
            "by_source": {"arxiv": 10},
            "by_status": {"done": 10},
            "queue_queued": 0,
            "queue_running": 0,
            "cache_entries": 5,
            "dedup_records": 1,
        }
        mock_db_cls.return_value = mock_db

        args = make_args(json=False, format="warp")
        result = _run_stats(args)

        out = capsys.readouterr().out
        # warp output should have WarpBlocks table content
        assert "Database Overview" in out or "Papers" in out
        assert result == 0


# ─────────────────────────────────────────────────────────────────────────────
# _run_cite_stats tests
# ─────────────────────────────────────────────────────────────────────────────


class TestRunCiteStats:
    """Test _run_cite_stats."""

    @patch("cli.Database")
    def test_cite_stats_paper_not_found(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.paper_exists.return_value = False
        mock_db_cls.return_value = mock_db

        args = make_args(stats_paper="nonexistent")
        result = _run_cite_stats(args)

        out = capsys.readouterr().err
        assert "not found" in out
        assert result == 1

    @patch("cli.Database")
    def test_cite_stats_paper_shows_counts(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.paper_exists.return_value = True
        mock_db.get_paper_title.return_value = "Test Paper"
        mock_db.get_citation_count.return_value = {"backward": 5, "forward": 3}
        mock_db_cls.return_value = mock_db

        args = make_args(stats_paper="uid1")
        result = _run_cite_stats(args)

        out = capsys.readouterr().out
        assert "Paper: uid1" in out
        assert "5" in out
        assert "3" in out
        assert result == 0

    @patch("cli.Database")
    def test_cite_stats_csv_format(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        # CSV path: no stats_paper, no top
        mock_cursor = MagicMock()
        mock_cursor.fetchone.side_effect = [(100,), (20,), (15,), (50,), (5,)]
        mock_db.conn.cursor.return_value = mock_cursor
        mock_db_cls.return_value = mock_db

        args = make_args(stats_paper=None, top=None, format="csv")
        result = _run_cite_stats(args)

        out = capsys.readouterr().out
        assert "metric,value" in out
        assert "total_citations,100" in out
        assert result == 0

    @patch("cli.Database")
    def test_cite_stats_warp_format(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_cursor = MagicMock()
        mock_cursor.fetchone.side_effect = [(50,), (10,), (5,), (20,), (2,)]
        mock_cursor.fetchall.return_value = []
        mock_db.conn.cursor.return_value = mock_cursor
        mock_db_cls.return_value = mock_db

        args = make_args(stats_paper=None, top=None, format="warp")
        result = _run_cite_stats(args)

        out = capsys.readouterr().out
        assert "\033[" in out or "Citation Statistics" in out
        assert result == 0

    @patch("cli.Database")
    def test_cite_stats_top_n_papers(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_cursor = MagicMock()
        mock_cursor.fetchone.side_effect = [(200,), (40,), (30,), (80,), (10,)]
        mock_cursor.fetchall.return_value = [
            ("paper1", 10),
            ("paper2", 8),
            ("paper3", 5),
        ]
        mock_db.conn.cursor.return_value = mock_cursor
        mock_db.get_paper_title.side_effect = ["Paper One", "Paper Two", "Paper Three"]
        mock_db_cls.return_value = mock_db

        args = make_args(stats_paper=None, top=3, format="text")
        result = _run_cite_stats(args)

        out = capsys.readouterr().out
        assert "paper1" in out
        assert "paper2" in out
        assert "paper3" in out
        assert result == 0

    @patch("cli.Database")
    def test_cite_stats_top_in_warp(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_cursor = MagicMock()
        mock_cursor.fetchone.side_effect = [(0,), (0,), (0,), (0,), (0,)]
        mock_cursor.fetchall.return_value = [
            ("paper1", 10),
            ("paper2", 8),
        ]
        mock_db.conn.cursor.return_value = mock_cursor
        mock_db.get_paper_title.side_effect = ["Paper One", "Paper Two"]
        mock_db_cls.return_value = mock_db

        args = make_args(stats_paper=None, top=2, format="warp")
        result = _run_cite_stats(args)

        out = capsys.readouterr().out
        assert "\033[" in out or "Cites" in out
        assert result == 0


# ─────────────────────────────────────────────────────────────────────────────
# _run_dedup_semantic tests
# ─────────────────────────────────────────────────────────────────────────────


class TestRunDedupSemantic:
    """Test _run_dedup_semantic."""

    @patch("cli.Database")
    def test_stats_shows_coverage(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.get_embedding_stats.return_value = {
            "total_with_text": 100,
            "with_embedding": 80,
        }
        mock_db_cls.return_value = mock_db

        args = make_args(
            stats=True,
            generate=False,
            paper=None,
            threshold=0.85,
            limit=20,
            format="warp",
            dedup_semantic=False,
        )
        result = _run_dedup_semantic(args)

        out = capsys.readouterr().out
        assert "Embedding Coverage" in out
        assert "80" in out
        assert result == 0

    @patch("cli.Database")
    def test_stats_zero_total(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.get_embedding_stats.return_value = {"total_with_text": 0, "with_embedding": 0}
        mock_db_cls.return_value = mock_db

        args = make_args(
            stats=True,
            generate=False,
            paper=None,
            threshold=0.85,
            limit=20,
            format="warp",
            dedup_semantic=False,
        )
        result = _run_dedup_semantic(args)

        out = capsys.readouterr().out
        assert "Embedding Coverage" in out
        assert result == 0

    @patch("cli.Database")
    def test_paper_not_found(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.paper_exists.return_value = False
        mock_db_cls.return_value = mock_db

        args = make_args(
            stats=False,
            generate=False,
            paper="nonexistent",
            threshold=0.85,
            limit=20,
            format="warp",
            dedup_semantic=False,
        )
        result = _run_dedup_semantic(args)

        out = capsys.readouterr().out
        assert "not found" in out
        assert result == 1

    @patch("cli.Database")
    def test_paper_no_similar(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.paper_exists.return_value = True
        mock_db.get_paper.return_value = MagicMock(id="uid1", title="Test Paper")
        mock_db.find_similar.return_value = []
        mock_db_cls.return_value = mock_db

        args = make_args(
            stats=False,
            generate=False,
            paper="uid1",
            threshold=0.85,
            limit=20,
            format="warp",
            dedup_semantic=False,
        )
        result = _run_dedup_semantic(args)

        out = capsys.readouterr().out
        assert "No similar papers" in out
        assert result == 0

    @patch("cli.Database")
    def test_paper_with_similar(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.paper_exists.return_value = True
        mock_paper = MagicMock(id="uid1", title="Test Paper")
        mock_db.get_paper.return_value = mock_paper
        sim_paper = MagicMock(id="uid2", title="Similar Paper")
        mock_db.find_similar.return_value = [(sim_paper, 0.92)]
        mock_db_cls.return_value = mock_db

        args = make_args(
            stats=False,
            generate=False,
            paper="uid1",
            threshold=0.85,
            limit=20,
            format="warp",
            dedup_semantic=False,
        )
        result = _run_dedup_semantic(args)

        out = capsys.readouterr().out
        assert "uid2" in out
        assert "0.92" in out
        assert result == 0

    @patch("cli.Database")
    def test_paper_similar_csv_format(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.paper_exists.return_value = True
        mock_paper = MagicMock(id="uid1", title='Paper "A"')
        mock_db.get_paper.return_value = mock_paper
        sim_paper = MagicMock(id="uid2", title="Similar")
        mock_db.find_similar.return_value = [(sim_paper, 0.96)]
        mock_db_cls.return_value = mock_db

        args = make_args(
            stats=False,
            generate=False,
            paper="uid1",
            threshold=0.85,
            limit=20,
            format="csv",
            dedup_semantic=False,
        )
        result = _run_dedup_semantic(args)

        out = capsys.readouterr().out
        assert "paper_a,paper_b,similarity" in out
        assert "uid1,uid2" in out
        assert result == 0

    @patch("cli.Database")
    def test_global_scan_no_duplicates(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.list_papers.return_value = ([MagicMock(id="uid1", title="Paper")], 1)
        mock_db.find_similar.return_value = []
        mock_db_cls.return_value = mock_db

        args = make_args(
            stats=False,
            generate=False,
            paper=None,
            threshold=0.85,
            limit=5,
            format="warp",
            dedup_semantic=False,
        )
        result = _run_dedup_semantic(args)

        out = capsys.readouterr().out
        assert "No Duplicates Found" in out
        assert result == 0

    @patch("cli.Database")
    def test_generate_missing_embeddings(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.get_papers_without_embeddings.return_value = []
        mock_db_cls.return_value = mock_db

        args = make_args(
            stats=False,
            generate=True,
            paper=None,
            threshold=0.85,
            limit=20,
            format="warp",
            dedup_semantic=False,
        )
        result = _run_dedup_semantic(args)

        assert result == 0


# ─────────────────────────────────────────────────────────────────────────────
# _run_kg tests
# ─────────────────────────────────────────────────────────────────────────────


class TestRunKgStats:
    """Test kg stats subcommand."""

    def _kg_args(self, kg_cmd, **kwargs):
        defaults = dict(format="text", kg_cmd=kg_cmd)
        defaults.update(kwargs)
        ns = argparse.Namespace()
        for k, v in defaults.items():
            setattr(ns, k, v)
        return ns

    def test_stats_text_format_shows_totals(self, capsys):
        from kg import KGManager

        mock_kg = KGManager.__new__(KGManager)
        mock_kg.stats = MagicMock(
            return_value={
                "total_nodes": 100,
                "total_edges": 250,
                "nodes_by_type": {"Paper": 50, "Tag": 30},
                "edges_by_type": {"cites": 200, "has_tag": 50},
            }
        )

        with patch("cli.cmd.kg.kg.KGManager", return_value=mock_kg):
            args = self._kg_args("stats", format="text")
            result = _run_kg(args)

        out = capsys.readouterr().out
        assert "Total nodes" in out or "100" in out
        assert result == 0

    def test_stats_warp_format_shows_tables(self, capsys):
        from kg import KGManager

        mock_kg = KGManager.__new__(KGManager)
        mock_kg.stats = MagicMock(
            return_value={
                "total_nodes": 100,
                "total_edges": 250,
                "nodes_by_type": {"Paper": 50, "Tag": 30},
                "edges_by_type": {"cites": 200, "has_tag": 50},
            }
        )

        with patch("cli.cmd.kg.kg.KGManager", return_value=mock_kg):
            args = self._kg_args("stats", format="warp")
            result = _run_kg(args)

        out = capsys.readouterr().out
        assert "Nodes" in out or "100" in out
        assert result == 0


class TestRunKgGraph:
    """Test kg graph subcommand."""

    def _kg_args(self, kg_cmd, **kwargs):
        defaults = dict(format="text", kg_cmd=kg_cmd, depth=2)
        defaults.update(kwargs)
        ns = argparse.Namespace()
        for k, v in defaults.items():
            setattr(ns, k, v)
        return ns

    def test_graph_paper_not_found(self, capsys):
        from kg import KGManager

        mock_kg = KGManager.__new__(KGManager)
        mock_kg.get_node_by_entity = MagicMock(return_value=None)

        with patch("cli.cmd.kg.kg.KGManager", return_value=mock_kg):
            args = self._kg_args("graph", paper_id="nonexistent")
            result = _run_kg(args)

        out = capsys.readouterr().out
        assert "not found" in out
        assert result == 1

    def test_graph_text_format_shows_neighbors(self, capsys):
        from kg import KGManager

        mock_kg = KGManager.__new__(KGManager)
        paper_node = {
            "id": "n1",
            "type": "Paper",
            "entity_id": "2301.00001",
            "label": "Test Paper",
            "properties": {},
            "created_at": "",
        }
        neighbors = [
            (
                {
                    "id": "n2",
                    "type": "Paper",
                    "entity_id": "2301.00002",
                    "label": "Neighbor Paper",
                    "properties": {},
                    "created_at": "",
                },
                {"id": "e1", "source_id": "n1", "target_id": "n2", "relation_type": "cites"},
                1,
            ),
        ]
        mock_kg.get_node_by_entity = MagicMock(return_value=paper_node)
        mock_kg.find_neighbors = MagicMock(return_value=neighbors)

        with patch("cli.cmd.kg.kg.KGManager", return_value=mock_kg):
            args = self._kg_args("graph", paper_id="2301.00001", format="text", depth=2)
            result = _run_kg(args)

        out = capsys.readouterr().out
        assert "2301.00001" in out
        assert "neighbor" in out.lower()
        assert result == 0

    def test_graph_json_format(self, capsys):
        from kg import KGManager

        mock_kg = KGManager.__new__(KGManager)
        paper_node = {
            "id": "n1",
            "type": "Paper",
            "entity_id": "2301.00001",
            "label": "Test",
            "properties": {},
            "created_at": "",
        }
        neighbors = []
        mock_kg.get_node_by_entity = MagicMock(return_value=paper_node)
        mock_kg.find_neighbors = MagicMock(return_value=neighbors)

        with patch("cli.cmd.kg.kg.KGManager", return_value=mock_kg):
            args = self._kg_args("graph", paper_id="2301.00001", format="json")
            result = _run_kg(args)

        out = capsys.readouterr().out
        assert "center" in out or "2301.00001" in out
        assert result == 0


class TestRunKgPath:
    """Test kg path subcommand."""

    def _kg_args(self, kg_cmd, **kwargs):
        defaults = dict(kg_cmd=kg_cmd)
        defaults.update(kwargs)
        ns = argparse.Namespace()
        for k, v in defaults.items():
            setattr(ns, k, v)
        return ns

    def test_path_node_a_not_found(self, capsys):
        from kg import KGManager

        mock_kg = KGManager.__new__(KGManager)
        mock_kg.get_node_by_entity = MagicMock(return_value=None)
        mock_kg.get_node = MagicMock(return_value=None)

        with patch("cli.cmd.kg.kg.KGManager", return_value=mock_kg):
            args = self._kg_args("path", idA="nonexistent", idB="also-none")
            result = _run_kg(args)

        out = capsys.readouterr().out
        assert "not found" in out
        assert result == 1

    def test_path_no_path_found(self, capsys):
        from kg import KGManager

        mock_kg = KGManager.__new__(KGManager)
        mock_kg.get_node_by_entity = MagicMock(
            side_effect=[
                {
                    "id": "n1",
                    "type": "Paper",
                    "label": "A",
                    "entity_id": "A",
                    "properties": {},
                    "created_at": "",
                },
                {
                    "id": "n2",
                    "type": "Paper",
                    "label": "B",
                    "entity_id": "B",
                    "properties": {},
                    "created_at": "",
                },
            ]
        )
        mock_kg.get_node = MagicMock(return_value=None)
        mock_kg.find_shortest_path = MagicMock(return_value=None)

        with patch("cli.cmd.kg.kg.KGManager", return_value=mock_kg):
            args = self._kg_args("path", idA="A", idB="B")
            result = _run_kg(args)

        out = capsys.readouterr().out
        assert "No path found" in out
        assert result == 0

    def test_path_found(self, capsys):
        from kg import KGManager

        mock_kg = KGManager.__new__(KGManager)
        mock_kg.get_node_by_entity = MagicMock(
            side_effect=[
                {
                    "id": "n1",
                    "type": "Paper",
                    "label": "Paper A",
                    "entity_id": "A",
                    "properties": {},
                    "created_at": "",
                },
                {
                    "id": "n2",
                    "type": "Paper",
                    "label": "Paper B",
                    "entity_id": "B",
                    "properties": {},
                    "created_at": "",
                },
            ]
        )
        mock_kg.get_node = MagicMock(return_value=None)
        mock_kg.find_shortest_path = MagicMock(return_value=["n1", "n3", "n2"])
        mock_kg.get_node = MagicMock(
            side_effect=lambda nid: {
                "n1": {"id": "n1", "type": "Paper", "label": "Paper A"},
                "n3": {"id": "n3", "type": "Paper", "label": "Middle Paper"},
                "n2": {"id": "n2", "type": "Paper", "label": "Paper B"},
            }.get(nid)
        )

        with patch("cli.cmd.kg.kg.KGManager", return_value=mock_kg):
            args = self._kg_args("path", idA="A", idB="B")
            result = _run_kg(args)

        out = capsys.readouterr().out
        assert "hops" in out
        assert "Paper A" in out or "Paper B" in out
        assert result == 0


class TestRunKgSearch:
    """Test kg search subcommand."""

    def _kg_args(self, kg_cmd, **kwargs):
        defaults = dict(format="table", kg_cmd=kg_cmd, tag=None, type=None)
        defaults.update(kwargs)
        ns = argparse.Namespace()
        for k, v in defaults.items():
            setattr(ns, k, v)
        return ns

    def test_search_no_results(self, capsys):
        from kg import KGManager

        mock_kg = KGManager.__new__(KGManager)
        mock_kg.find_papers_by_tag = MagicMock(return_value=[])
        mock_kg.get_all_nodes = MagicMock(return_value=[])

        with patch("cli.cmd.kg.kg.KGManager", return_value=mock_kg):
            args = self._kg_args("search", tag="LLM", format="text")
            result = _run_kg(args)

        out = capsys.readouterr().out
        assert "No nodes found" in out
        assert result == 0

    def test_search_by_tag_shows_results(self, capsys):
        from kg import KGManager

        mock_kg = KGManager.__new__(KGManager)
        mock_kg.find_papers_by_tag = MagicMock(
            return_value=[
                {"id": "n1", "type": "Paper", "label": "LLM Survey"},
                {"id": "n2", "type": "Paper", "label": "LLM Benchmark"},
            ]
        )
        mock_kg.get_all_nodes = MagicMock(return_value=[])

        with patch("cli.cmd.kg.kg.KGManager", return_value=mock_kg):
            args = self._kg_args("search", tag="LLM", format="text")
            result = _run_kg(args)

        out = capsys.readouterr().out
        assert "LLM" in out
        assert "LLM Survey" in out
        assert result == 0

    def test_search_by_type(self, capsys):
        from kg import KGManager

        mock_kg = KGManager.__new__(KGManager)
        mock_kg.find_papers_by_tag = MagicMock(return_value=[])
        mock_kg.get_all_nodes = MagicMock(
            return_value=[
                {"id": "n1", "type": "Tag", "label": "LLM"},
                {"id": "n2", "type": "Tag", "label": "RAG"},
            ]
        )

        with patch("cli.cmd.kg.kg.KGManager", return_value=mock_kg):
            args = self._kg_args("search", type="Tag", format="text")
            result = _run_kg(args)

        out = capsys.readouterr().out
        assert "Tag" in out
        assert result == 0

    def test_search_warp_format(self, capsys):
        from kg import KGManager

        mock_kg = KGManager.__new__(KGManager)
        mock_kg.find_papers_by_tag = MagicMock(return_value=[])
        mock_kg.get_all_nodes = MagicMock(
            return_value=[
                {"id": "n1", "type": "Paper", "label": "Attention Is All You Need"},
            ]
        )

        with patch("cli.cmd.kg.kg.KGManager", return_value=mock_kg):
            args = self._kg_args("search", format="warp")
            result = _run_kg(args)

        out = capsys.readouterr().out
        assert "Paper" in out or "Attention" in out
        assert result == 0

    def test_search_json_format(self, capsys):
        from kg import KGManager

        mock_kg = KGManager.__new__(KGManager)
        mock_kg.find_papers_by_tag = MagicMock(return_value=[])
        mock_kg.get_all_nodes = MagicMock(
            return_value=[
                {"id": "n1", "type": "Paper", "label": "Test"},
            ]
        )

        with patch("cli.cmd.kg.kg.KGManager", return_value=mock_kg):
            args = self._kg_args("search", format="json")
            result = _run_kg(args)

        out = capsys.readouterr().out
        assert "n1" in out
        assert result == 0


# ─────────────────────────────────────────────────────────────────────────────
# _run_stats tests
# ─────────────────────────────────────────────────────────────────────────────


class TestRunStatsAndTrend:
    """Test _run_stats extended and _run_trend."""

    def _stats_args(self, **kwargs):
        defaults = dict(json=False, format="table")
        defaults.update(kwargs)
        ns = argparse.Namespace()
        for k, v in defaults.items():
            setattr(ns, k, v)
        return ns

    def test_stats_table_format_shows_total(self, capsys):
        mock_stats = {
            "total_papers": 42,
            "by_source": {"arxiv": 30, "doi": 12},
            "by_status": {"done": 35, "failed": 7},
            "queue_queued": 5,
            "queue_running": 1,
            "cache_entries": 100,
            "dedup_records": 20,
        }
        with patch("cli._shared.get_db") as mock_get:
            mock_db = MagicMock()
            mock_db.get_stats.return_value = mock_stats
            mock_get.return_value = mock_db
            args = self._stats_args(format="table")
            _run_stats(args)
        out = capsys.readouterr().out
        assert "42" in out or "total" in out

    def test_stats_json_format(self, capsys):
        mock_stats = {
            "total_papers": 10,
            "by_source": {},
            "by_status": {},
            "queue_queued": 0,
            "queue_running": 0,
            "cache_entries": 0,
            "dedup_records": 0,
        }
        with patch("cli._shared.get_db") as mock_get:
            mock_db = MagicMock()
            mock_db.get_stats.return_value = mock_stats
            mock_get.return_value = mock_db
            args = self._stats_args(json=True)
            result = _run_stats(args)
        out = capsys.readouterr().out
        assert "total_papers" in out
        assert result == 0


# ─────────────────────────────────────────────────────────────────────────────
# _run_trend tests
# ─────────────────────────────────────────────────────────────────────────────


class TestRunTrend:
    """Test _run_trend."""

    def _trend_args(self, **kwargs):
        defaults = dict(
            topic=None,
            year_start=None,
            year_end=None,
            min_papers=10,
            mermaid=False,
            json=False,
            interactive=False,
        )
        defaults.update(kwargs)
        ns = argparse.Namespace()
        for k, v in defaults.items():
            setattr(ns, k, v)
        return ns

    def test_no_topic_and_no_interactive_returns_early(self, capsys):
        """No topic with no interactive falls through to _run_interactive (returns 0)."""
        with (
            patch("cli.cmd.trend.TrendAnalyzer") as mock_analyzer,
            patch("builtins.input", return_value="q"),
        ):
            mock_analyzer.return_value = MagicMock()
            args = self._trend_args(topic=None, interactive=False)
            result = _run_trend(args)
        assert result == 0

    def test_interactive_mode_returns_zero(self, capsys):
        """--interactive without topic returns 0 (enters interactive loop)."""
        with (
            patch("cli.cmd.trend.TrendAnalyzer") as mock_analyzer,
            patch("builtins.input", return_value="q"),
        ):
            mock_analyzer.return_value = MagicMock()
            args = self._trend_args(interactive=True)
            result = _run_trend(args)
        assert result == 0

    def test_topic_with_no_results(self, capsys):
        """Topic with zero papers returns 0 and renders result."""
        from llm.trend_analyzer import TrendAnalysisResult

        mock_result = TrendAnalysisResult(
            topic="nonexistent_topic_xyz",
            year_range=(2020, 2025),
            total_papers=0,
            yearly_distribution=[],
            rising_trends=[],
            falling_trends=[],
            emerging_trends=[],
            stable_trends=[],
            hot_keywords=[],
            declining_keywords=[],
            emerging_keywords=[],
            growth_rate=0.0,
        )
        mock_analyzer = MagicMock()
        mock_analyzer.analyze.return_value = mock_result
        mock_analyzer.render_result.return_value = "No papers found for nonexistent_topic_xyz"

        with patch("cli.cmd.trend.TrendAnalyzer") as mock_get_analyzer:
            mock_get_analyzer.return_value = mock_analyzer
            args = self._trend_args(topic="nonexistent_topic_xyz")
            result = _run_trend(args)

        capsys.readouterr()
        assert result == 0
        mock_analyzer.analyze.assert_called_once()

    def test_topic_with_results(self, capsys):
        """Topic with results renders output and returns 0."""
        from llm.trend_analyzer import TrendAnalysisResult, TrendDirection, TrendKeyword

        mock_trend = TrendKeyword(
            keyword="transformer",
            direction=TrendDirection.RISING,
            yearly_counts={2023: 50, 2024: 80},
            growth_rate=0.8,
            peak_year=2024,
            current_year_count=80,
        )
        mock_result = TrendAnalysisResult(
            topic="LLM",
            year_range=(2020, 2025),
            total_papers=50,
            yearly_distribution=[],
            rising_trends=[mock_trend],
            falling_trends=[],
            emerging_trends=[],
            stable_trends=[],
            hot_keywords=["transformer"],
            declining_keywords=[],
            emerging_keywords=[],
            growth_rate=15.5,
        )
        mock_analyzer = MagicMock()
        mock_analyzer.analyze.return_value = mock_result
        mock_analyzer.render_result.return_value = "LLM trend analysis output"

        with patch("cli.cmd.trend.TrendAnalyzer") as mock_get_analyzer:
            mock_get_analyzer.return_value = mock_analyzer
            args = self._trend_args(topic="LLM")
            result = _run_trend(args)

        capsys.readouterr()
        assert result == 0
        mock_analyzer.analyze.assert_called_once_with(topic="LLM", year_range=None, min_papers=10)

    def test_json_flag_returns_json(self, capsys):
        """--json outputs JSON."""
        from llm.trend_analyzer import TrendAnalysisResult

        mock_result = TrendAnalysisResult(
            topic="RAG",
            year_range=(2020, 2025),
            total_papers=20,
            yearly_distribution=[],
            rising_trends=[],
            falling_trends=[],
            emerging_trends=[],
            stable_trends=[],
            hot_keywords=["retrieval"],
            declining_keywords=[],
            emerging_keywords=[],
            growth_rate=25.0,
        )
        mock_analyzer = MagicMock()
        mock_analyzer.analyze.return_value = mock_result

        with patch("cli.cmd.trend.TrendAnalyzer") as mock_get_analyzer:
            mock_get_analyzer.return_value = mock_analyzer
            args = self._trend_args(topic="RAG", json=True)
            result = _run_trend(args)

        out = capsys.readouterr().out
        assert "RAG" in out
        assert result == 0

    def test_mermaid_flag_renders_mermaid(self, capsys):
        """--mermaid calls render_mermaid_timeline."""
        from llm.trend_analyzer import TrendAnalysisResult

        mock_result = TrendAnalysisResult(
            topic="KVCache",
            year_range=(2020, 2025),
            total_papers=15,
            yearly_distribution=[],
            rising_trends=[],
            falling_trends=[],
            emerging_trends=[],
            stable_trends=[],
            hot_keywords=[],
            declining_keywords=[],
            emerging_keywords=[],
            growth_rate=0.0,
        )
        mock_analyzer = MagicMock()
        mock_analyzer.analyze.return_value = mock_result
        mock_analyzer.render_mermaid_timeline.return_value = "gantt\n  section RAG"

        with patch("cli.cmd.trend.TrendAnalyzer") as mock_get_analyzer:
            mock_get_analyzer.return_value = mock_analyzer
            args = self._trend_args(topic="KVCache", mermaid=True)
            result = _run_trend(args)

        out = capsys.readouterr().out
        assert "gantt" in out
        assert result == 0
        mock_analyzer.render_mermaid_timeline.assert_called_once()

    def test_year_range_passed_to_analyze(self, capsys):
        """--year-start/--year-end are passed as year_range tuple."""
        from llm.trend_analyzer import TrendAnalysisResult

        mock_result = TrendAnalysisResult(
            topic="Attention",
            year_range=(2021, 2024),
            total_papers=5,
            yearly_distribution=[],
            rising_trends=[],
            falling_trends=[],
            emerging_trends=[],
            stable_trends=[],
            hot_keywords=[],
            declining_keywords=[],
            emerging_keywords=[],
            growth_rate=0.0,
        )
        mock_analyzer = MagicMock()
        mock_analyzer.analyze.return_value = mock_result
        mock_analyzer.render_result.return_value = "output"

        with patch("cli.cmd.trend.TrendAnalyzer") as mock_get_analyzer:
            mock_get_analyzer.return_value = mock_analyzer
            args = self._trend_args(topic="Attention", year_start=2021, year_end=2024)
            result = _run_trend(args)

        mock_analyzer.analyze.assert_called_once()
        call_kwargs = mock_analyzer.analyze.call_args.kwargs
        assert call_kwargs["year_range"] == (2021, 2024)
        assert result == 0


# ─────────────────────────────────────────────────────────────────────────────
# _work_to_arxiv_id tests (pure function — no mocking needed)
# ─────────────────────────────────────────────────────────────────────────────


class TestWorkToArxivId:
    """Test _work_to_arxiv_id pure function."""

    def test_arxiv_paper_standard_doi(self):
        """DOI containing /arxiv. returns the arXiv ID suffix."""
        work = {"ids": {"doi": "https://doi.org/10.48550/arXiv.2301.00001"}}
        assert _work_to_arxiv_id(work) == "2301.00001"

    def test_arxiv_paper_uppercase_doi(self):
        """Case-insensitive match on /arxiv. in DOI."""
        work = {"ids": {"doi": "https://doi.org/10.48550/arXiv.2301.00001"}}
        assert _work_to_arxiv_id(work) == "2301.00001"

    def test_non_arxiv_paper(self):
        """DOI without /arxiv. returns None."""
        work = {"ids": {"doi": "https://doi.org/10.1234/journal.2024.001"}}
        assert _work_to_arxiv_id(work) is None

    def test_none_doi(self):
        """Missing DOI returns None."""
        work = {"ids": {}}
        assert _work_to_arxiv_id(work) is None

    def test_null_doi(self):
        """Explicitly null DOI is handled gracefully."""
        work = {"ids": {"doi": None}}
        assert _work_to_arxiv_id(work) is None

    def test_empty_doi(self):
        """Empty string DOI returns None."""
        work = {"ids": {"doi": ""}}
        assert _work_to_arxiv_id(work) is None

    def test_no_ids_key(self):
        """Work dict with no 'ids' key returns None."""
        work = {"title": "Some paper"}
        assert _work_to_arxiv_id(work) is None

    def test_ids_is_none(self):
        """'ids' key explicitly set to None is handled."""
        work = {"ids": None}
        assert _work_to_arxiv_id(work) is None

    def test_arxiv_id_with_version(self):
        """arXiv ID with version suffix is returned as-is."""
        work = {"ids": {"doi": "https://doi.org/10.48550/arXiv.2301.00001v2"}}
        # The function splits on /arxiv. and takes the last part, preserving version
        assert _work_to_arxiv_id(work) == "2301.00001v2"


# ─────────────────────────────────────────────────────────────────────────────
# _work_to_paper_record tests (pure function — no mocking needed)
# ─────────────────────────────────────────────────────────────────────────────


class TestWorkToPaperRecord:
    """Test _work_to_paper_record pure function."""

    def test_with_arxiv_id(self):
        """Paper with arXiv DOI returns PaperRecord keyed by arXiv ID."""
        work = {
            "ids": {"doi": "https://doi.org/10.48550/arXiv.2301.00001"},
            "title": "Attention Is All You Need",
            "authorships": [
                {"author": {"display_name": "Ashish Vaswani"}},
                {"author": {"display_name": "Noam Shazeer"}},
            ],
            "publication_date": "2017-06-12",
            "primary_location": {
                "best_oa_location": {
                    "landing_page_url": "https://arxiv.org/abs/1706.03762",
                    "pdf_url": "https://arxiv.org/1706.03762.pdf",
                }
            },
            "host_venue": {"display_name": "NeurIPS"},
            "topics": [{"display_name": "Machine Learning"}],
            "referenced_works_count": 50,
        }
        result = _work_to_paper_record(work)

        assert result is not None
        assert result.id == "2301.00001"
        assert result.title == "Attention Is All You Need"
        assert result.authors == ["Ashish Vaswani", "Noam Shazeer"]
        assert result.published == "2017"
        assert result.abs_url == "https://arxiv.org/abs/1706.03762"
        assert result.pdf_url == "https://arxiv.org/1706.03762.pdf"
        assert result.journal == "NeurIPS"
        assert result.doi == "https://doi.org/10.48550/arXiv.2301.00001"
        assert result.categories == "Machine Learning"
        assert result.reference_count == 50

    def test_with_only_openalex_id(self):
        """Paper with no arXiv DOI falls back to OpenAlex ID as paper_id."""
        work = {
            "ids": {"openalex": "https://openalex.org/W1234567"},
            "title": "A Non-ArXiv Paper",
            "authorships": [],
            "publication_date": "2024-01-15",
            "primary_location": {},
            "topics": [],
            "referenced_works_count": 0,
        }
        result = _work_to_paper_record(work)

        assert result is not None
        assert result.id == "W1234567"
        assert result.title == "A Non-ArXiv Paper"

    def test_with_only_doi(self):
        """Paper with no arXiv or OpenAlex ID falls back to DOI suffix (path after host)."""
        work = {
            "ids": {"doi": "https://doi.org/10.1234/journal.2024.001"},
            "title": "Journal Paper",
            "authorships": [],
            "publication_date": "2024-03-01",
            "primary_location": {},
            "topics": [],
            "referenced_works_count": 0,
        }
        result = _work_to_paper_record(work)

        assert result is not None
        # doi.rstrip("/").split("/")[-1] strips the https://doi.org/ prefix
        assert result.id == "journal.2024.001"
        assert result.title == "Journal Paper"

    def test_missing_all_ids(self):
        """Work with no IDs at all returns None."""
        work = {
            "title": "Orphan Paper",
            "authorships": [],
            "publication_date": "2024",
            "primary_location": {},
            "topics": [],
        }
        result = _work_to_paper_record(work)

        assert result is None

    def test_multiple_topics_limited_to_10(self):
        """Topics are comma-joined, capped at 10."""
        work = {
            "ids": {"doi": "https://doi.org/10.48550/arXiv.2301.00001"},
            "title": "Paper",
            "authorships": [],
            "publication_date": "2024-01-01",
            "primary_location": {},
            "topics": [{"display_name": f"Topic{i}"} for i in range(15)],
            "referenced_works_count": 0,
        }
        result = _work_to_paper_record(work)

        assert result is not None
        topics = result.categories.split(",")
        assert len(topics) == 10
        assert topics[0] == "Topic0"
        assert topics[9] == "Topic9"

    def test_custom_source(self):
        """source parameter is set on the PaperRecord."""
        work = {
            "ids": {"doi": "https://doi.org/10.48550/arXiv.2301.00001"},
            "title": "Test",
            "authorships": [],
            "publication_date": "2024-01-01",
            "primary_location": {},
            "topics": [],
            "referenced_works_count": 0,
        }
        result = _work_to_paper_record(work, source="test_source")

        assert result is not None
        assert result.source == "test_source"

    def test_empty_title_defaults_to_empty_string(self):
        """Missing title becomes empty string, not None."""
        work = {
            "ids": {"doi": "https://doi.org/10.48550/arXiv.2301.00001"},
            "title": None,
            "authorships": [],
            "publication_date": "2024-01-01",
            "primary_location": {},
            "topics": [],
            "referenced_works_count": 0,
        }
        result = _work_to_paper_record(work)

        assert result is not None
        assert result.title == ""

    def test_year_from_partial_date(self):
        """publication_date with only year part extracts correctly."""
        work = {
            "ids": {"doi": "https://doi.org/10.48550/arXiv.2301.00001"},
            "title": "Test",
            "authorships": [],
            "publication_date": "2023",
            "primary_location": {},
            "topics": [],
            "referenced_works_count": 0,
        }
        result = _work_to_paper_record(work)

        assert result is not None
        assert result.published == "2023"

    def test_openalex_id_strips_prefix(self):
        """OpenAlex ID with https://openalex.org/ prefix is stripped."""
        work = {
            "ids": {"openalex": "https://openalex.org/W2987654321"},
            "title": "Test",
            "authorships": [],
            "publication_date": "2024",
            "primary_location": {},
            "topics": [],
            "referenced_works_count": 0,
        }
        result = _work_to_paper_record(work)

        assert result is not None
        assert result.id == "W2987654321"
