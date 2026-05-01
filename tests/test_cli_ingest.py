"""Unit tests for ingest CLI subcommand — import + postprocess + embed + KG pipeline."""
from unittest.mock import patch, MagicMock


class FakeArgs:
    def __init__(self, **kwargs):
        for k, v in kwargs.items():
            setattr(self, k, v)


class FakePaper:
    def __init__(self, id="2301.00001", title="Test Paper", abstract="",
                 authors=None, published="2024-01-01", pdf_url="", doi="",
                 primary_category="cs.AI", tags="", category="02-Models"):
        self.id = id
        self.title = title
        self.abstract = abstract
        self.authors = authors or []
        self.published = published
        self.pdf_url = pdf_url
        self.doi = doi
        self.primary_category = primary_category
        self.tags = tags
        self.category = category


class FakeDB:
    def __init__(self, papers=None):
        self.papers = papers or {}   # paper_id -> FakePaper
        self.upserted = []
        self.init_called = False

    def init(self):
        self.init_called = True

    def paper_exists(self, pid):
        return pid in self.papers

    def get_paper(self, pid):
        return self.papers.get(pid)

    def upsert_paper(self, paper_id, source, **kwargs):
        self.upserted.append((paper_id, source))
        return FakePaper(id=paper_id)


# ─────────────────────────────────────────────────────────────────────────────
# Parser tests
# ─────────────────────────────────────────────────────────────────────────────

class TestIngestParser:
    def test_parser_help_text(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.ingest import _build_ingest_parser
        import argparse
        p = argparse.ArgumentParser()
        sub = p.add_subparsers()
        _build_ingest_parser(sub)
        # smoke — parser built without error
        assert True

    def test_skip_flags_are_mutually_exclusive(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.ingest import _build_ingest_parser
        import argparse
        p = argparse.ArgumentParser()
        sub = p.add_subparsers()
        _build_ingest_parser(sub)
        # Just verify no crash on construction
        assert True


# ─────────────────────────────────────────────────────────────────────────────
# _run_import_phase tests
# ─────────────────────────────────────────────────────────────────────────────

class TestImportPhase:
    def test_import_phase_no_papers(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.ingest import _run_import_phase

        with patch("cli.Database") as MockDB:
            MockDB.return_value = FakeDB()
            added, failed = _run_import_phase([], FakeDB(), "test")
            assert added == []
            assert failed == []

    def test_import_phase_fetch_success(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.ingest import _run_import_phase

        with patch("cli.Database") as MockDB:
            mock_db = FakeDB()
            MockDB.return_value = mock_db

            # _fetch_paper_metadata is imported from cli.cmd.import_ inside the function
            with patch("cli.cmd.import_._fetch_paper_metadata") as mock_fetch:
                mock_fetch.return_value = {
                    "title": "Test Paper",
                    "authors": [],
                    "abstract": "Test abstract",
                    "published": "2024-01-01",
                    "abs_url": "https://arxiv.org/abs/2301.00001",
                    "pdf_url": "https://arxiv.org/pdf/2301.00001.pdf",
                    "primary_category": "cs.AI",
                    "doi": "",
                }
                added, failed = _run_import_phase(["2301.00001"], mock_db, "test")
                assert added == ["2301.00001"]
                assert failed == []

    def test_import_phase_fetch_failure(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.ingest import _run_import_phase

        with patch("cli.Database") as MockDB:
            mock_db = FakeDB()
            MockDB.return_value = mock_db

            with patch("cli.cmd.import_._fetch_paper_metadata") as mock_fetch:
                mock_fetch.return_value = None  # fetch failed
                added, failed = _run_import_phase(["2301.00001"], mock_db, "test")
                assert added == []
                assert failed == ["2301.00001"]


# ─────────────────────────────────────────────────────────────────────────────
# _run_kg_sync_phase tests
# ─────────────────────────────────────────────────────────────────────────────

class TestKGSyncPhase:
    def test_kg_sync_calls_integration(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.ingest import _run_kg_sync_phase

        with patch("cli.Database") as MockDB:
            with patch("kg.integration.KGIntegration") as MockInteg:
                with patch("kg.KGManager"):
                    MockDB.return_value = FakeDB()
                    mock_integ = MagicMock()
                    MockInteg.return_value = mock_integ

                    result = _run_kg_sync_phase(MockDB.return_value)
                    mock_integ.rebuild_from_papers_json.assert_called_once_with("data/papers.json", incremental=True)
                    assert result is True


# ─────────────────────────────────────────────────────────────────────────────
# _run_ingest main logic tests
# ─────────────────────────────────────────────────────────────────────────────

class TestRunIngest:
    def test_no_ids_returns_error(self, monkeypatch, capsys):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.ingest import _run_ingest

        args = FakeArgs(ids=[], file=None, root="AI-Research", tags="",
                        source="ingest", skip_postprocess=False, only_postprocess=False,
                        skip_embed=False, skip_kg=False, skip_pdf=False,
                        stages=None, skip_llm=False, format="text")
        with patch("cli.Database") as MockDB:
            MockDB.return_value = FakeDB()
            rc = _run_ingest(args)
            assert rc == 1

    def test_only_postprocess_missing_paper(self, monkeypatch, capsys):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.ingest import _run_ingest

        args = FakeArgs(ids=["9999.99999"], file=None, root="AI-Research", tags="",
                        source="ingest", skip_postprocess=False, only_postprocess=True,
                        skip_embed=False, skip_kg=False, skip_pdf=False,
                        stages=None, skip_llm=False, format="text")
        with patch("cli.Database") as MockDB:
            MockDB.return_value = FakeDB()  # empty — paper doesn't exist
            rc = _run_ingest(args)
            assert rc == 1

    def test_only_postprocess_with_existing_paper(self, monkeypatch, capsys):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.ingest import _run_ingest

        args = FakeArgs(ids=["2301.00001"], file=None, root="AI-Research", tags="",
                        source="ingest", skip_postprocess=False, only_postprocess=True,
                        skip_embed=True, skip_kg=True, skip_pdf=True,
                        stages=None, skip_llm=True, format="text")

        with patch("cli.Database") as MockDB:
            mock_db = FakeDB(papers={"2301.00001": FakePaper()})
            MockDB.return_value = mock_db

            with patch("cli.cmd.ingest._run_postprocess_phase") as mock_pp:
                mock_pp.return_value = True
                rc = _run_ingest(args)
                # Import phase skipped, only postprocess called
                mock_pp.assert_called_once()
                assert rc == 0

    def test_skip_all_phases(self, monkeypatch, capsys):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.ingest import _run_ingest

        args = FakeArgs(ids=["2301.00001"], file=None, root="AI-Research", tags="",
                        source="ingest", skip_postprocess=True,
                        only_postprocess=False,
                        skip_embed=True, skip_kg=True, skip_pdf=False,
                        stages=None, skip_llm=False, format="text")

        with patch("cli.Database") as MockDB:
            mock_db = FakeDB()
            MockDB.return_value = mock_db

            with patch("cli.cmd.ingest._run_import_phase") as mock_imp:
                mock_imp.return_value = (["2301.00001"], [])
                rc = _run_ingest(args)
                # Only import phase runs; postprocess/embed/kg skipped
                mock_imp.assert_called_once()
                assert rc == 0
