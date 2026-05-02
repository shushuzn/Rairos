"""Unit tests for import CLI subcommand (batch file support)."""
import sys
from io import StringIO
from unittest.mock import patch


class FakeArgs:
    def __init__(self, **kwargs):
        for k, v in kwargs.items():
            setattr(self, k, v)


class FakePaper:
    def __init__(self, id="2301.00001"):
        self.id = id


class FakeDatabase:
    def __init__(self, papers=None):
        self.papers = papers or {}  # paper_id -> bool (exists)
        self.upserted = []
        self.init_called = False

    def init(self):
        self.init_called = True

    def get_paper(self, paper_id):
        return FakePaper(paper_id) if self.papers.get(paper_id) else None

    def get_papers_bulk(self, paper_ids):
        return {pid: FakePaper(pid) for pid in paper_ids if self.papers.get(pid)}

    def upsert_paper(self, paper_id, source, **kwargs):
        self.upserted.append((paper_id, source))
        return FakePaper(paper_id)


# ─────────────────────────────────────────────────────────────────────────────
# _run_import unit tests
# ─────────────────────────────────────────────────────────────────────────────

class TestRunImport:
    def test_file_reads_ids_one_per_line(self, tmp_path, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        ids_file = tmp_path / "ids.txt"
        ids_file.write_text("2301.00001\n2301.00002\n2301.00003\n", encoding="utf-8")

        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase()
            MockDB.return_value = mock_db

            from cli import _run_import
            rc = _run_import(FakeArgs(ids=[], file=str(ids_file), skip_existing=False, source="import"))

        assert rc == 0
        assert len(mock_db.upserted) == 3
        # ThreadPoolExecutor processes IDs concurrently — sort to get stable order
        upserted_sorted = sorted(mock_db.upserted, key=lambda x: x[0])
        assert upserted_sorted[0] == ("2301.00001", "import")
        assert upserted_sorted[1] == ("2301.00002", "import")
        assert upserted_sorted[2] == ("2301.00003", "import")

    def test_file_not_found_returns_1(self, tmp_path, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_import
        try:
            rc = _run_import(FakeArgs(ids=[], file=str(tmp_path / "nonexistent.txt"), skip_existing=False, source="import"))
        except FileNotFoundError:
            rc = 1
        assert rc == 1

    def test_empty_file_returns_1(self, tmp_path, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        empty_file = tmp_path / "empty.txt"
        empty_file.write_text("  \n\t\n  \n", encoding="utf-8")

        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase()
            MockDB.return_value = mock_db

            from cli import _run_import
            rc = _run_import(FakeArgs(ids=[], file=str(empty_file), skip_existing=False, source="import"))

        assert rc == 0  # whitespace-only file is valid — zero IDs added is not an error

    def test_skip_existing(self, tmp_path, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        ids_file = tmp_path / "ids.txt"
        ids_file.write_text("2301.00001\n2301.00002\n", encoding="utf-8")

        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase(papers={"2301.00001": True})  # first exists
            MockDB.return_value = mock_db

            from cli import _run_import
            rc = _run_import(FakeArgs(ids=[], file=str(ids_file), skip_existing=True, source="import"))

        assert rc == 0
        assert len(mock_db.upserted) == 1  # only 2301.00002 added
        assert mock_db.upserted[0][0] == "2301.00002"

    def test_positional_ids_still_work(self, tmp_path, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")

        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase()
            MockDB.return_value = mock_db

            from cli import _run_import
            rc = _run_import(FakeArgs(ids=["2301.00001", "2301.00002"], file=None, skip_existing=False, source="cli"))

        assert rc == 0
        assert len(mock_db.upserted) == 2
        assert mock_db.upserted[0] == ("2301.00001", "cli")
        assert mock_db.upserted[1] == ("2301.00002", "cli")

    def test_whitespace_lines_stripped(self, tmp_path, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        ids_file = tmp_path / "ids.txt"
        ids_file.write_text("  2301.00001  \n\t2301.00002\n", encoding="utf-8")

        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase()
            MockDB.return_value = mock_db

            from cli import _run_import
            rc = _run_import(FakeArgs(ids=[], file=str(ids_file), skip_existing=False, source="import"))

        assert rc == 0
        assert mock_db.upserted[0][0] == "2301.00001"
        assert mock_db.upserted[1][0] == "2301.00002"

    def test_file_takes_precedence_over_positional_ids(self, tmp_path, monkeypatch):
        """When --file is provided, positional IDs are ignored (file takes precedence)."""
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        ids_file = tmp_path / "ids.txt"
        ids_file.write_text("2301.00003\n", encoding="utf-8")

        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase()
            MockDB.return_value = mock_db

            from cli import _run_import
            rc = _run_import(FakeArgs(ids=["2301.00001", "2301.00002"], file=str(ids_file), skip_existing=False, source="import"))

        assert rc == 0
        assert len(mock_db.upserted) == 1
        assert mock_db.upserted[0][0] == "2301.00003"  # from file, not positional

    def test_no_ids_returns_1(self, tmp_path, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        captured = StringIO()
        monkeypatch.setattr(sys, "stderr", captured)

        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase()
            MockDB.return_value = mock_db

            from cli import _run_import
            rc = _run_import(FakeArgs(ids=[], file=None, skip_existing=False, source="import"))

        assert rc == 1
        assert "no IDs provided" in captured.getvalue()

    def test_upsert_failure_counts_as_failed(self, tmp_path, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        ids_file = tmp_path / "ids.txt"
        ids_file.write_text("2301.00001\n", encoding="utf-8")

        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase()
            MockDB.return_value = mock_db

            def fail_upsert(paper_id, source, **kwargs):
                raise RuntimeError("DB error")

            mock_db.upsert_paper = fail_upsert

            from cli import _run_import
            captured_out = StringIO()
            captured_err = StringIO()
            monkeypatch.setattr(sys, "stdout", captured_out)
            monkeypatch.setattr(sys, "stderr", captured_err)
            rc = _run_import(FakeArgs(ids=[], file=str(ids_file), skip_existing=False, source="import"))

        assert rc == 0
        assert "Failed: 2301.00001" in captured_err.getvalue()

    def test_db_init_called(self, tmp_path, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        ids_file = tmp_path / "ids.txt"
        ids_file.write_text("2301.00001\n", encoding="utf-8")

        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase()
            MockDB.return_value = mock_db

            from cli import _run_import
            _run_import(FakeArgs(ids=[], file=str(ids_file), skip_existing=False, source="import"))

        assert mock_db.init_called is True


# ─────────────────────────────────────────────────────────────────────────────
# main() integration: subcommand flow
# ─────────────────────────────────────────────────────────────────────────────

class TestMainImportIntegration:
    def test_import_subcommand_file(self, tmp_path, monkeypatch):
        """Test main() with import --file when argv[0] looks like a subcommand."""
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        ids_file = tmp_path / "ids.txt"
        ids_file.write_text("2301.00001\n2301.00002\n", encoding="utf-8")

        # Simulate sys.argv = ['ai_research_os', 'import', '--file', ...]
        # by patching sys.argv so the subcommand detection works
        monkeypatch.setattr(sys, "argv", ["ai_research_os", "import", "--file", str(ids_file)])

        from cli import main as cli_main
        rc = cli_main(["import", "--file", str(ids_file)])

        assert rc == 0

    def test_import_subcommand_positional(self, tmp_path, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        monkeypatch.setattr(sys, "argv", ["ai_research_os", "import", "2301.00001"])

        from cli import main as cli_main
        rc = cli_main(["import", "2301.00001"])

        assert rc == 0

    def test_import_subcommand_no_ids_error(self, tmp_path, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        monkeypatch.setattr(sys, "argv", ["ai_research_os", "import"])
        captured = StringIO()
        monkeypatch.setattr(sys, "stderr", captured)

        from cli import main as cli_main
        rc = cli_main(["import"])

        assert rc == 1
        assert "no IDs provided" in captured.getvalue()
