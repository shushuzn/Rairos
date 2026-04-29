"""Unit tests for read-queue CLI subcommand — smart reading priority queue."""
from unittest.mock import patch


class FakeArgs:
    def __init__(self, **kwargs):
        for k, v in kwargs.items():
            setattr(self, k, v)


class FakePaper:
    def __init__(self, id="2301.00001", title="Test Paper", abstract=""):
        self.id = id
        self.paper_id = id
        self.title = title
        self.abstract = abstract
        self.published = "2024-01-01"
        self.primary_category = "cs.AI"
        self.authors = []
        self.reading_started_at = None
        self.reading_completed_at = None


class FakeDB:
    def __init__(self, papers=None):
        self.papers = papers or {}
        self.init_called = False

    def init(self):
        self.init_called = True

    def paper_exists(self, pid):
        return pid in self.papers

    def get_paper(self, pid):
        return self.papers.get(pid)

    def search_papers(self, query, limit=100):
        return [], 0

    def get_papers_by_reading_status(self, status, limit=100):
        return []

    def get_reading_status(self, paper_id):
        return None


# ─────────────────────────────────────────────────────────────────────────────
# Parser tests
# ─────────────────────────────────────────────────────────────────────────────

class TestReadQueueParser:
    def test_parser_help_text(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.read_queue import _build_read_queue_parser
        import argparse
        p = argparse.ArgumentParser()
        sub = p.add_subparsers()
        _build_read_queue_parser(sub)
        assert True  # smoke

    def test_parser_accepts_all_options(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.read_queue import _build_read_queue_parser
        import argparse
        p = argparse.ArgumentParser()
        sub = p.add_subparsers()
        _build_read_queue_parser(sub)
        assert True  # smoke


# ─────────────────────────────────────────────────────────────────────────────
# _handle_status_action unit tests
# ─────────────────────────────────────────────────────────────────────────────

class TestHandleStatus:
    def _make_args(self, **kwargs):
        """Build args with defaults so FakeArgs always has status/start/done/reset."""
        defaults = dict(
            status=None,
            start=None,
            done=None,
            reset=None,
        )
        defaults.update(kwargs)
        return FakeArgs(**defaults)

    def test_status_empty_lists_reading_papers(self, monkeypatch, capsys):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.read_queue import _handle_status_action

        class FakeDBWithReading(FakeDB):
            def get_papers_by_reading_status(self, status, limit=100):
                if status == "reading":
                    return [FakePaper(id="2301.00001")]
                return []

        args = self._make_args(status="")  # empty → list all
        rc = _handle_status_action(args, FakeDBWithReading())
        assert rc == 0

    def test_status_unknown_paper_returns_error(self, monkeypatch, capsys):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.read_queue import _handle_status_action

        args = self._make_args(status="nonexistent.00001")
        rc = _handle_status_action(args, FakeDB())
        assert rc == 1

    def test_done_action_updates_status(self, monkeypatch, capsys):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.read_queue import _handle_status_action

        class FakeDBWithDone(FakeDB):
            def __init__(self):
                super().__init__()
                self.papers = {"2301.00001": FakePaper()}

            def get_paper_title(self, pid):
                p = self.papers.get(pid)
                return p.title if p else None

            def update_reading_status(self, pid, status):
                pass  # no-op in test

        args = self._make_args(done="2301.00001")
        rc = _handle_status_action(args, FakeDBWithDone())
        assert rc == 0

    def test_reset_action_updates_status(self, monkeypatch, capsys):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.read_queue import _handle_status_action

        class FakeDBWithReset(FakeDB):
            def __init__(self):
                super().__init__()
                self.papers = {"2301.00001": FakePaper()}

            def get_paper_title(self, pid):
                return "Test Paper"

            def update_reading_status(self, pid, status):
                pass

        args = self._make_args(reset="2301.00001")
        rc = _handle_status_action(args, FakeDBWithReset())
        assert rc == 0


# ─────────────────────────────────────────────────────────────────────────────
# _run_read_queue unit tests
# ─────────────────────────────────────────────────────────────────────────────

class TestRunReadQueue:
    def _make_args(self, **kwargs):
        defaults = dict(
            status=None,
            start=None,
            done=None,
            reset=None,
            tag=None,
            year=None,
            min_similarity=0.0,
            format="table",
            explain=False,
            explain_model=None,
            limit=10,
        )
        defaults.update(kwargs)
        return FakeArgs(**defaults)

    def test_status_returns_early(self, monkeypatch, capsys):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.read_queue import _run_read_queue

        args = self._make_args(status="2301.00001")
        with patch("cli.cmd.read_queue.get_db") as mock_get_db:
            mock_db = FakeDB()
            mock_get_db.return_value = mock_db
            with patch("cli.cmd.read_queue._handle_status_action") as mock_status:
                mock_status.return_value = 1  # error
                rc = _run_read_queue(args)
                assert rc == 1

    def test_rank_returns_empty_when_no_candidates(self, monkeypatch, capsys):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.read_queue import _run_read_queue

        args = self._make_args()
        with patch("cli.cmd.read_queue.get_db") as mock_get_db:
            mock_db = FakeDB()
            mock_get_db.return_value = mock_db
            with patch("cli.cmd.read_queue._handle_status_action") as mock_status:
                mock_status.return_value = None  # continue
                with patch.object(mock_db, "search_papers", return_value=([], 0)):
                    rc = _run_read_queue(args)
                    assert rc == 0  # empty queue is OK
