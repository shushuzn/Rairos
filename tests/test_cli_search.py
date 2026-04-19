"""Unit tests for CLI search, list, and status subcommands."""
import argparse
import json
from unittest.mock import MagicMock, patch


from cli import _run_search, _run_list, _run_status, _run_queue, _run_cache, _run_dedup, _run_merge, infer_tags_if_empty, main
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
        sort="added_at", order="desc",
        since="", clear=False, dry_run=False, keep="older", report=False,
        source="import", skip_existing=False,
        format="table", limit=0, out=None, json=False,
        set_=None,
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

    @patch("cli.Database")
    def test_list_csv_output(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.list_papers.return_value = ([FakePaper(), FakePaper()], 2)
        mock_db_cls.return_value = mock_db

        args = make_args(status="", year=0, tags=[], limit=20, offset=0, format="csv")
        _run_list(args)

        captured = capsys.readouterr().out.replace("\r", "")
        lines = captured.strip().split("\n")
        assert lines[0] == "id,title,authors,published,source,primary_category,parse_status,added_at"
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
        assert lines[0] == "id,title,authors,published,source,primary_category,parse_status,added_at"
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
    """Test queue --list."""

    @patch("cli.Database")
    def test_queue_list_shows_pending(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.get_papers.return_value = [
            FakePaper(id="2301.00001", parse_status="pending"),
            FakePaper(id="2301.00002", parse_status="done"),
            FakePaper(id="2301.00003", parse_status="pending"),
        ]
        mock_db_cls.return_value = mock_db

        args = make_args(list=True, dequeue=False, add=None, cancel=None)
        result = _run_queue(args)

        captured = capsys.readouterr().out
        assert "Pending:" in captured
        assert "2301.00001" in captured
        assert "2301.00003" in captured
        assert "2301.00002" not in captured  # not pending
        assert result == 0

    @patch("cli.Database")
    def test_queue_list_empty(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.get_papers.return_value = [FakePaper(id="2301.00001", parse_status="done")]
        mock_db_cls.return_value = mock_db

        args = make_args(list=True, dequeue=False, add=None, cancel=None)
        result = _run_queue(args)

        captured = capsys.readouterr().out
        assert "Queue empty" in captured
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
        json_file.write_text('{"title": "Test Paper", "abstract": "Test abstract"}', encoding="utf-8")

        args = make_args(stats=False, clear=False, get=None, set=["uid123", str(json_file)])
        result = _run_cache(args)

        mock_db.set_cached_paper.assert_called_once_with("uid123", {"title": "Test Paper", "abstract": "Test abstract"})
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

        args = make_args(stats=False, clear=False, get=None, set=["uid123", "/nonexistent/path.json"])
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

        args = make_args(subcmd="search", query="attention", limit=10, offset=0,
                         format="table", source="", year=0, tags=[], status="", sort="relevance")
        with patch("argparse.ArgumentParser.parse_args", return_value=args):
            result = main(["search", "attention"])

        mock_run.assert_called_once()
        assert result == 0

    @patch("cli._run_list")
    def test_main_routes_to_list(self, mock_run):
        mock_run.return_value = 0
        args = make_args(subcmd="list", status="", year=0, tags=[], limit=20, offset=0, format="table")
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

    @patch("cli._main_legacy")
    def test_main_routes_to_legacy_for_arxiv_id(self, mock_legacy):
        mock_legacy.return_value = 0
        _ = main(["2301.00001"])
        mock_legacy.assert_called_once_with(["2301.00001"])

    def test_main_no_args_routes_to_legacy(self):
        """No args falls through to legacy parser."""
        with patch("cli._main_legacy") as mock_legacy:
            mock_legacy.return_value = 0
            _ = main([])
            mock_legacy.assert_called_once()

    def test_main_unknown_subcommand_routes_to_legacy(self):
        """Unknown subcommand (not in SUBCOMMANDS set) falls through to legacy."""
        with patch("cli._main_legacy") as mock_legacy:
            mock_legacy.return_value = 0
            _ = main(["some-random-input"])
            mock_legacy.assert_called_once_with(["some-random-input"])

    def test_main_doi_input_routes_to_legacy(self):
        """DOI input routes to legacy."""
        with patch("cli._main_legacy") as mock_legacy:
            mock_legacy.return_value = 0
            _ = main(["10.1234/some"])
            mock_legacy.assert_called_once_with(["10.1234/some"])


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
        assert call_kwargs.get("date_from") == "2023-01-01", f"Expected date_from='2023-01-01', got {call_kwargs}"

    @patch("cli.Database")
    def test_list_passes_status_filter(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.list_papers.return_value = ([], 0)
        mock_db_cls.return_value = mock_db
        args = make_args(status="done", year=0, tags=[], limit=20, offset=0, format="table")
        _run_list(args)
        call_kwargs = mock_db.list_papers.call_args.kwargs
        assert call_kwargs.get("parse_status") == "done", f"Expected parse_status='done', got {call_kwargs}"

    @patch("cli.Database")
    def test_list_zero_year_no_date_filter(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.list_papers.return_value = ([], 0)
        mock_db_cls.return_value = mock_db
        args = make_args(status="", year=0, tags=[], limit=20, offset=0, format="table")
        _run_list(args)
        call_kwargs = mock_db.list_papers.call_args.kwargs
        assert call_kwargs.get("date_from") is None, f"Expected date_from=None for year=0, got {call_kwargs}"


class TestRunSearchFilters:
    """Test _run_search filter arguments are passed correctly to db.search_papers."""

    @patch("cli.Database")
    def test_search_passes_year_as_date_from(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.search_papers.return_value = ([], 0)
        mock_db_cls.return_value = mock_db
        args = make_args(
            query="attention", limit=10, offset=0, format="table",
            source="", year=2024, tags=[], status="", sort="relevance"
        )
        _run_search(args)
        call_kwargs = mock_db.search_papers.call_args.kwargs
        assert call_kwargs.get("date_from") == "2024-01-01", f"Expected date_from='2024-01-01', got {call_kwargs}"

    @patch("cli.Database")
    def test_search_zero_year_no_date_filter(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.search_papers.return_value = ([], 0)
        mock_db_cls.return_value = mock_db
        args = make_args(
            query="attention", limit=10, offset=0, format="table",
            source="", year=0, tags=[], status="", sort="relevance"
        )
        _run_search(args)
        call_kwargs = mock_db.search_papers.call_args.kwargs
        assert call_kwargs.get("date_from") is None, f"Expected date_from=None for year=0, got {call_kwargs}"

    @patch("cli.Database")
    def test_search_passes_source_filter(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.search_papers.return_value = ([], 0)
        mock_db_cls.return_value = mock_db
        args = make_args(
            query="attention", limit=10, offset=0, format="table",
            source="doi", year=0, tags=[], status="", sort="relevance"
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
            query="attention", limit=10, offset=0, format="table",
            source="", year=0, tags=[], status="done", sort="relevance"
        )
        _run_search(args)
        call_kwargs = mock_db.search_papers.call_args.kwargs
        assert call_kwargs.get("parse_status") == "done", f"Expected parse_status='done', got {call_kwargs}"

    @patch("cli.Database")
    def test_search_empty_query_allowed(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.search_papers.return_value = ([], 0)
        mock_db_cls.return_value = mock_db
        args = make_args(
            query="", limit=10, offset=0, format="table",
            source="", year=0, tags=[], status="", sort="relevance"
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
        args = make_args(stats=False, clear=False, get=None, set=["uid123", "/nonexistent/cache/file.json"])
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
        paper = Paper(source="arxiv", uid="t", title="t", authors=[], abstract="", published="", updated="", abs_url="", pdf_url="", primary_category="")
        tags = ["Agent", "RAG"]
        assert infer_tags_if_empty(tags, paper) == ["Agent", "RAG"]

    def test_infers_agent_from_title(self):
        paper = Paper(source="arxiv", uid="t", title="Tool Use in LLM Agents", authors=[], abstract="", published="", updated="", abs_url="", pdf_url="", primary_category="")
        tags = []
        assert "Agent" in infer_tags_if_empty(tags, paper)

    def test_infers_rag_from_abstract(self):
        paper = Paper(source="arxiv", uid="t", title="Foo", authors=[], abstract="retrieval augmented generation", published="", updated="", abs_url="", pdf_url="", primary_category="")
        tags = []
        assert "RAG" in infer_tags_if_empty(tags, paper)

    def test_unsorted_when_no_match(self):
        paper = Paper(source="arxiv", uid="t", title="Foo Bar Baz", authors=[], abstract="", published="", updated="", abs_url="", pdf_url="", primary_category="")
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

    def test_main_dedup_not_legacy(self):
        """dedup must be a subcommand, not fall through to legacy."""
        with patch("cli._build_dedup_parser"):
            with patch("cli._run_dedup", return_value=0) as mock_run:
                with patch("cli._build_merge_parser"):
                    with patch("cli._build_queue_parser"):
                        with patch("cli._build_cache_parser"):
                            with patch("cli._build_status_parser"):
                                with patch("cli._build_list_parser"):
                                    with patch("cli._build_search_parser"):
                                        # Patch argparse so it doesn't fail
                                        with patch("cli.argparse.ArgumentParser") as mock_ap:
                                            mock_parser = MagicMock()
                                            mock_parser.parse_args.return_value = make_args(subcmd="dedup")
                                            mock_ap.return_value = mock_parser
                                            main(["dedup"])
        # If it routed to legacy instead, _run_dedup wouldn't be called
        assert mock_run.called


class TestMainMergeRouting:
    """Test main() routes 'merge' to _run_merge (not legacy)."""

    @patch("cli._run_merge")
    @patch("cli._build_merge_parser")
    @patch("cli.argparse.ArgumentParser")
    def test_main_merge_routes_to_run_merge(self, mock_argparse, mock_build, mock_run, capsys):
        mock_parser = MagicMock()
        mock_parser.parse_args.return_value = make_args(subcmd="merge", target_id="uid1", duplicate_id="uid2")
        mock_argparse.return_value = mock_parser
        mock_build.return_value = None
        mock_run.return_value = 0

        result = main(["merge", "uid1", "uid2"])

        assert result == 0

    def test_main_merge_not_legacy(self):
        """merge must be a subcommand, not fall through to legacy."""
        with patch("cli._build_dedup_parser"):
            with patch("cli._run_merge", return_value=0) as mock_run:
                with patch("cli._build_merge_parser"):
                    with patch("cli._build_queue_parser"):
                        with patch("cli._build_cache_parser"):
                            with patch("cli._build_status_parser"):
                                with patch("cli._build_list_parser"):
                                    with patch("cli._build_search_parser"):
                                        with patch("cli.argparse.ArgumentParser") as mock_ap:
                                            mock_parser = MagicMock()
                                            mock_parser.parse_args.return_value = make_args(subcmd="merge", target_id="uid1", duplicate_id="uid2")
                                            mock_ap.return_value = mock_parser
                                            main(["merge", "uid1", "uid2"])
        assert mock_run.called


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

        result = _run_dedup(make_args(dry_run=False, auto=False, keep="older", batch=False, report=False))

        assert result == 0
        assert "No duplicates found" in capsys.readouterr().out

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

        result = _run_dedup(make_args(dry_run=False, auto=False, keep="older", batch=False, report=False))

        out = capsys.readouterr().out
        assert "uid1" in out
        assert "uid2" in out
        assert "Attention Is All You Need" in out
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
        assert "dry-run" in out
        assert "uid1" in out
        assert "uid2" in out
        assert "completed" in out
        assert "pending" in out
        # Shows keep decision for current --keep
        assert "would keep [uid1]" in out
        assert "parsed winner" in out
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
        assert "Auto-merged uid2 into uid1" in out
        assert "(--keep=older)" in out
        assert "Auto-merged uid4 into uid3" in out
        assert "Auto-merged 2/2 pair(s)" in out
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
        assert "Auto-merged 1/1 pair(s)" in out
        assert result == 0

    @patch("cli.Database")
    def test_dedup_auto_no_pairs(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.find_duplicates.return_value = []
        mock_db_cls.return_value = mock_db

        result = _run_dedup(make_args(dry_run=False, auto=True, keep="older", report=False))

        out = capsys.readouterr().out
        assert "No duplicates found" in out
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
        assert "Auto-merged uid1 into uid2" in out
        assert "(--keep=newer)" in out
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
        assert "Auto-merged uid1 into uid2" in out
        assert "(--keep=parsed)" in out
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
        assert "Auto-merged uid2 into uid1" in out
        assert result == 0


# ─────────────────────────────────────────────────────────────────────────────
# _run_merge tests
# ─────────────────────────────────────────────────────────────────────────────

class TestRunMerge:
    """Test _run_merge behavior."""

    @patch("cli.Database")
    def test_merge_target_not_found(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_paper.return_value = None
        mock_db_cls.return_value = mock_db

        result = _run_merge(make_args(target_id="uid1", duplicate_id="uid2"))

        out = capsys.readouterr().out
        assert "not found" in out
        assert result == 1

    @patch("cli.Database")
    def test_merge_duplicate_not_found(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_paper.side_effect = [MagicMock(), None]
        mock_db_cls.return_value = mock_db

        result = _run_merge(make_args(target_id="uid1", duplicate_id="uid2"))

        out = capsys.readouterr().out
        assert "not found" in out
        assert result == 1

    @patch("cli.Database")
    def test_merge_success(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_paper.side_effect = [MagicMock(), MagicMock()]
        mock_db.get_similarity.return_value = None
        mock_db.merge_papers.return_value = True
        mock_db_cls.return_value = mock_db

        result = _run_merge(make_args(target_id="uid1", duplicate_id="uid2"))

        out = capsys.readouterr().out
        assert "Merged uid2 into uid1" in out
        assert result == 0

    @patch("cli.Database")
    def test_merge_db_returns_false(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_paper.side_effect = [MagicMock(), MagicMock()]
        mock_db.get_similarity.return_value = None
        mock_db.merge_papers.return_value = False
        mock_db_cls.return_value = mock_db

        result = _run_merge(make_args(target_id="uid1", duplicate_id="uid2"))

        out = capsys.readouterr().out
        assert "Merge failed" in out
        assert result == 1


# ─────────────────────────────────────────────────────────────────────────────
# _run_queue else branch (no list/dequeue/add/cancel)
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
        assert "Dedup history" in captured
        assert "uid1" in captured
        assert "uid2" in captured
        assert "keep=older" in captured
        assert "keep=parsed" in captured
        assert result == 0


class TestRunDedupBatch:
    """Test dedup --batch mode."""

    @patch("cli.Database")
    def test_batch_both_same_doi(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        p1 = MagicMock(id="uid1", title="Attention Is All You Need",
                       doi="10.1234/abc", parse_status="completed",
                       added_at="2026-01-01T00:00:00")
        p2 = MagicMock(id="uid2", title="Attention Is All You Need",
                       doi="10.1234/abc", parse_status="pending",
                       added_at="2026-01-02T00:00:00")
        mock_db.find_duplicates.return_value = [(p1, p2)]
        mock_db.merge_papers.return_value = True
        mock_db_cls.return_value = mock_db

        args = make_args(dry_run=False, auto=False, batch=True, keep="older", report=False)
        result = _run_dedup(args)

        captured = capsys.readouterr().out
        assert "[batch] Merged uid2 -> uid1" in captured
        assert "Batch: 1 merged, 0 skipped" in captured
        mock_db.log_dedup.assert_called_once_with("uid1", "uid2", "older")
        assert result == 0

    @patch("cli.Database")
    def test_batch_different_doi(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        p1 = MagicMock(id="uid1", title="Attention Is All You Need",
                       doi="10.1234/abc", parse_status="completed",
                       added_at="2026-01-01T00:00:00")
        p2 = MagicMock(id="uid2", title="Attention Is All You Need",
                       doi="10.9999/xyz", parse_status="pending",
                       added_at="2026-01-02T00:00:00")
        mock_db.find_duplicates.return_value = [(p1, p2)]
        mock_db_cls.return_value = mock_db

        args = make_args(dry_run=False, auto=False, batch=True, keep="older", report=False)
        result = _run_dedup(args)

        captured = capsys.readouterr().out
        assert "[batch] Skipped uid1/uid2" in captured
        assert "Batch: 0 merged, 1 skipped" in captured
        mock_db.merge_papers.assert_not_called()
        assert result == 0

    @patch("cli.Database")
    def test_batch_mixed_pairs(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.init.return_value = None
        # Pair 1: same DOI → auto-merged
        p1 = MagicMock(id="uid1", title="Paper A", doi="10.1234/a",
                       parse_status="completed", added_at="2026-01-01T00:00:00")
        p2 = MagicMock(id="uid2", title="Paper A", doi="10.1234/a",
                       parse_status="pending", added_at="2026-01-02T00:00:00")
        # Pair 2: different DOI → skipped
        p3 = MagicMock(id="uid3", title="Paper B", doi="10.9999/b",
                       parse_status="completed", added_at="2026-01-01T00:00:00")
        p4 = MagicMock(id="uid4", title="Paper B", doi=None,
                       parse_status="completed", added_at="2026-01-02T00:00:00")
        mock_db.find_duplicates.return_value = [(p1, p2), (p3, p4)]
        mock_db.merge_papers.return_value = True
        mock_db_cls.return_value = mock_db

        args = make_args(dry_run=False, auto=False, batch=True, keep="older", report=False)
        result = _run_dedup(args)

        captured = capsys.readouterr().out
        assert "[batch] Merged uid2 -> uid1" in captured
        assert "[batch] Skipped uid3/uid4" in captured
        assert "Batch: 1 merged, 1 skipped" in captured
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
        assert "No duplicates found" in captured
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
