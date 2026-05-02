"""Unit tests for cite-import CLI subcommand."""

from io import StringIO

from cli import main as cli_main


class FakeArgs:
    def __init__(self, **kwargs):
        for k, v in kwargs.items():
            setattr(self, k, v)


class FakeDatabase:
    def __init__(self, papers=None):
        self.papers = papers or {}  # paper_id -> bool (exists)
        self.citations_added = []

    def init(self):
        pass

    def paper_exists(self, paper_id):
        return self.papers.get(paper_id, False)

    def add_citations_batch(self, source_id, target_ids):
        count = 0
        for t in target_ids:
            self.citations_added.append((source_id, t))
            count += 1
        return count


class TestCiteImportValidation:
    def test_missing_json_input_returns_1(self, tmp_path, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        rc = cli_main(["cite-import"])
        assert rc == 1

    def test_invalid_json_returns_1(self, tmp_path, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        rc = cli_main(["cite-import", "not json at all"])
        assert rc == 1

    def test_json_not_list_or_dict_returns_1(self, tmp_path, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        rc = cli_main(["cite-import", '"just a string"'])
        assert rc == 1


class TestCiteImportDryRun:
    def _mock_db(self):
        return FakeDatabase({
            "2301.00001": True,
            "2302.00001": True,
            "2303.00003": True,
        })

    def test_dry_run_prints_actions(self, tmp_path, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        fake_db = self._mock_db()
        monkeypatch.setattr("cli.Database", lambda: fake_db)

        captured = StringIO()
        monkeypatch.setattr("sys.stdout", captured)
        rc = cli_main([
            "cite-import", "--dry-run",
            '[{"source": "2301.00001", "targets": ["2302.00001", "2303.00003"]}]'
        ])
        out = captured.getvalue()
        assert rc == 0
        assert "[dry-run] add citation: 2301.00001 -> 2302.00001" in out
        assert "[dry-run] add citation: 2301.00001 -> 2303.00003" in out
        assert fake_db.citations_added == []

    def test_dry_run_with_missing_source_shows_skip(self, tmp_path, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        fake_db = FakeDatabase({"2302.00001": True})
        monkeypatch.setattr("cli.Database", lambda: fake_db)

        captured = StringIO()
        monkeypatch.setattr("sys.stdout", captured)
        rc = cli_main([
            "cite-import", "--dry-run", "--skip-missing",
            '[{"source": "2301.00001", "targets": ["2302.00001"]}]'
        ])
        out = captured.getvalue()
        assert rc == 0
        assert "[dry-run] skip (missing): 2301.00001" in out

    def test_dry_run_with_missing_target_shows_skip(self, tmp_path, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        fake_db = FakeDatabase({"2301.00001": True})  # source exists, target doesn't
        monkeypatch.setattr("cli.Database", lambda: fake_db)

        captured = StringIO()
        monkeypatch.setattr("sys.stdout", captured)
        rc = cli_main([
            "cite-import", "--dry-run", "--skip-missing",
            '[{"source": "2301.00001", "targets": ["2302.00001"]}]'
        ])
        out = captured.getvalue()
        assert rc == 0
        assert "[dry-run] skip (missing): 2302.00001" in out


class TestCiteImportActual:
    def _mock_db(self):
        return FakeDatabase({
            "2301.00001": True,
            "2302.00001": True,
            "2303.00003": True,
        })

    def test_actual_import_adds_citations(self, tmp_path, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        fake_db = self._mock_db()
        monkeypatch.setattr("cli.Database", lambda: fake_db)

        captured = StringIO()
        monkeypatch.setattr("sys.stdout", captured)
        rc = cli_main([
            "cite-import",
            '[{"source": "2301.00001", "targets": ["2302.00001", "2303.00003"]}]'
        ])
        out = captured.getvalue()
        assert rc == 0
        assert "new citations : 2" in out
        assert ("2301.00001", "2302.00001") in fake_db.citations_added
        assert ("2301.00001", "2303.00003") in fake_db.citations_added

    def test_skip_missing_suppresses_error(self, tmp_path, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        fake_db = FakeDatabase({"2302.00001": True})  # only target exists
        monkeypatch.setattr("cli.Database", lambda: fake_db)

        captured = StringIO()
        monkeypatch.setattr("sys.stdout", captured)
        rc = cli_main([
            "cite-import", "--skip-missing",
            '[{"source": "2301.00001", "targets": ["2302.00001"]}]'
        ])
        out = captured.getvalue()
        assert rc == 0
        assert "new citations : 0" in out
        assert "skipped (missing papers): 1" in out
        # No error lines since --skip-missing
        assert "warnings/errors" not in out

    def test_missing_source_without_skip_returns_1(self, tmp_path, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        fake_db = FakeDatabase({})
        monkeypatch.setattr("cli.Database", lambda: fake_db)

        captured = StringIO()
        monkeypatch.setattr("sys.stdout", captured)
        rc = cli_main([
            "cite-import",
            '[{"source": "2301.00001", "targets": ["2302.00001"]}]'
        ])
        assert rc == 1
        out = captured.getvalue()
        assert "2301.00001" in out
        assert "warnings/errors" in out


class TestCiteImportFileInput:
    def test_at_filepath_loads_file(self, tmp_path, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        json_file = tmp_path / "citations.json"
        json_file.write_text('[{"source": "2301.00001", "targets": ["2302.00001"]}]', encoding="utf-8")

        fake_db = FakeDatabase({"2301.00001": True, "2302.00001": True})
        monkeypatch.setattr("cli.Database", lambda: fake_db)

        captured = StringIO()
        monkeypatch.setattr("sys.stdout", captured)
        rc = cli_main(["cite-import", f"@{json_file}"])
        assert rc == 0
        assert "new citations : 1" in captured.getvalue()

    def test_at_filepath_not_found_returns_1(self, tmp_path, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        captured = StringIO()
        monkeypatch.setattr("sys.stderr", captured)
        rc = cli_main(["cite-import", "@nonexistent/file.json"])
        assert rc == 1
        assert "error reading" in captured.getvalue().lower()


class TestCiteImportFieldVariants:
    def test_source_id_and_target_ids_field_names(self, tmp_path, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        fake_db = FakeDatabase({"2301.00001": True, "2302.00001": True})
        monkeypatch.setattr("cli.Database", lambda: fake_db)

        captured = StringIO()
        monkeypatch.setattr("sys.stdout", captured)
        rc = cli_main([
            "cite-import",
            '[{"source_id": "2301.00001", "target_ids": ["2302.00001"]}]'
        ])
        assert rc == 0
        assert "new citations : 1" in captured.getvalue()
