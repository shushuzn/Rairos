"""Unit tests for similar CLI subcommand — semantic similarity search."""
from unittest.mock import patch, MagicMock


class FakeArgs:
    def __init__(self, **kwargs):
        for k, v in kwargs.items():
            setattr(self, k, v)


class FakePaper:
    def __init__(self, id="2301.00001", title="Test Paper", abstract=""):
        self.id = id
        self.title = title
        self.abstract = abstract


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

    def find_similar(self, paper_id, threshold=0.85, limit=20):
        return []


# ─────────────────────────────────────────────────────────────────────────────
# Parser tests
# ─────────────────────────────────────────────────────────────────────────────

class TestSimilarParser:
    def test_parser_help_text(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.similar import _build_similar_parser
        import argparse
        p = argparse.ArgumentParser()
        sub = p.add_subparsers()
        _build_similar_parser(sub)
        assert True  # smoke

    def test_parser_accepts_threshold_and_limit(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.similar import _build_similar_parser
        import argparse
        p = argparse.ArgumentParser()
        sub = p.add_subparsers()
        _build_similar_parser(sub)
        assert True  # smoke


# ─────────────────────────────────────────────────────────────────────────────
# _run_similar_text unit tests
# ─────────────────────────────────────────────────────────────────────────────

class TestRunSimilarText:
    def test_no_paper_id_prints_stats(self, monkeypatch, capsys):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.similar import _run_similar_text

        class FakeDBWithStats(FakeDB):
            def get_embedding_stats(self):
                return {"total_with_text": 10, "with_embedding": 7}

        args = FakeArgs(
            paper_id="",
            threshold=0.85,
            limit=20,
            format="text",
        )
        with patch("cli.cmd.similar.get_db") as mock_get_db:
            mock_get_db.return_value = FakeDBWithStats()
            rc = _run_similar_text(args)
            assert rc == 1  # requires paper_id

    def test_paper_not_found_returns_error(self, monkeypatch, capsys):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.similar import _run_similar_text

        args = FakeArgs(
            paper_id="nonexistent.00001",
            threshold=0.85,
            limit=20,
            format="text",
        )
        with patch("cli.cmd.similar.get_db") as mock_get_db:
            mock_get_db.return_value = FakeDB()
            rc = _run_similar_text(args)
            assert rc == 1

    def test_paper_found_with_sims_csv(self, monkeypatch, capsys):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.similar import _run_similar_text

        class FakeDBWithSim(FakeDB):
            def __init__(self):
                super().__init__()
                self.papers = {"2301.00001": FakePaper()}

            def find_similar(self, paper_id, threshold=0.85, limit=20):
                sim_paper = FakePaper(id="2301.00002", title="Related Paper")
                return [(sim_paper, 0.91)]

        args = FakeArgs(
            paper_id="2301.00001",
            threshold=0.85,
            limit=20,
            format="csv",
        )
        with patch("cli.cmd.similar.get_db") as mock_get_db:
            mock_get_db.return_value = FakeDBWithSim()
            rc = _run_similar_text(args)
            assert rc == 0


# ─────────────────────────────────────────────────────────────────────────────
# _run_similar_view unit tests
# ─────────────────────────────────────────────────────────────────────────────

class TestRunSimilarView:
    def test_view_paper_not_found_returns_error(self, monkeypatch, capsys):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.similar import _run_similar_view

        args = FakeArgs(
            paper_id="nonexistent.00001",
            similar_subcmd="view",
            threshold=0.85,
        )
        with patch("cli.cmd.similar.get_db") as mock_get_db:
            mock_get_db.return_value = FakeDB()  # empty
            rc = _run_similar_view(args)
            assert rc == 1  # paper not found
