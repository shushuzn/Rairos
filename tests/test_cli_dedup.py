"""Unit tests for dedup and dedup-semantic CLI subcommands."""
from io import StringIO
from unittest.mock import patch, MagicMock


class FakeArgs:
    def __init__(self, **kwargs):
        for k, v in kwargs.items():
            setattr(self, k, v)


class FakePaper:
    def __init__(self, id="2301.00001", title="Test Paper", doi=None,
                 parse_status="completed", added_at="2024-01-01",
                 abstract=None):
        self.id = id
        self.title = title
        self.doi = doi
        self.parse_status = parse_status
        self.added_at = added_at
        self.abstract = abstract


class FakeDatabase:
    def __init__(self, pairs=None, log_entries=None):
        self.pairs = pairs or []
        self.log_entries = log_entries or []
        self.merged = []
        self.init_called = False
        self.dedup_log_called = False

    def init(self):
        self.init_called = True

    def find_duplicates(self, since=None):
        return self.pairs

    def get_dedup_log(self):
        self.dedup_log_called = True
        return self.log_entries

    def merge_papers(self, keep_id, dup_id):
        self.merged.append((keep_id, dup_id))
        return True

    def log_dedup(self, keep_id, dup_id, policy):
        pass


# ─────────────────────────────────────────────────────────────────────────────
# dedup --report
# ─────────────────────────────────────────────────────────────────────────────

class TestDedupReport:
    def test_report_empty(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_dedup
        with patch("cli.Database") as MockDB:
            MockDB.return_value = FakeDatabase(log_entries=[])
            rc = _run_dedup(FakeArgs(report=True, dry_run=False, auto=False,
                                      batch=False, keep="older", since=""))
            assert rc == 0

    def test_report_shows_records(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_dedup
        log = [{
            "id": 1, "logged_at": "2024-01-01 12:00",
            "keep_policy": "older", "target_id": "a", "duplicate_id": "b",
            "target_title": "Paper A with a long title here",
            "duplicate_title": "Paper B with a long title here",
        }]
        with patch("cli.Database") as MockDB:
            MockDB.return_value = FakeDatabase(log_entries=log)
            rc = _run_dedup(FakeArgs(report=True, dry_run=False, auto=False,
                                      batch=False, keep="older", since=""))
        assert rc == 0


# ─────────────────────────────────────────────────────────────────────────────
# dedup --dry-run
# ─────────────────────────────────────────────────────────────────────────────

class TestDedupDryRun:
    def test_no_duplicates(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_dedup
        with patch("cli.Database") as MockDB:
            MockDB.return_value = FakeDatabase(pairs=[])
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_dedup(FakeArgs(report=False, dry_run=True, auto=False,
                                          batch=False, keep="older", since=""))
            assert "No duplicates found" in captured.getvalue()
            assert rc == 0

    def test_dry_run_shows_pairs(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_dedup
        older = FakePaper(id="P1", title="Paper One", doi="10.1234/test",
                         parse_status="completed", added_at="2024-01-01")
        newer = FakePaper(id="P2", title="Paper One", doi="10.1234/test",
                         parse_status="pending", added_at="2024-01-02")
        with patch("cli.Database") as MockDB:
            MockDB.return_value = FakeDatabase(pairs=[(older, newer)])
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_dedup(FakeArgs(report=False, dry_run=True, auto=False,
                                          batch=False, keep="older", since=""))
            out = captured.getvalue()
            assert "Duplicate pair: P1 / P2" in out
            assert "dry-run" in out
            assert rc == 0

    def test_dry_run_keep_newer(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_dedup
        older = FakePaper(id="P1", title="Paper One", doi="10.1234/test",
                         parse_status="completed", added_at="2024-01-01")
        newer = FakePaper(id="P2", title="Paper One", doi="10.1234/test",
                         parse_status="pending", added_at="2024-01-02")
        with patch("cli.Database") as MockDB:
            MockDB.return_value = FakeDatabase(pairs=[(older, newer)])
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_dedup(FakeArgs(report=False, dry_run=True, auto=False,
                                          batch=False, keep="newer", since=""))
            out = captured.getvalue()
            assert "would keep [P2]" in out
            assert rc == 0


# ─────────────────────────────────────────────────────────────────────────────
# dedup --auto
# ─────────────────────────────────────────────────────────────────────────────

class TestDedupAuto:
    def test_auto_merges_all_pairs(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_dedup
        older = FakePaper(id="P1", title="Paper One", doi="10.1234/test",
                         parse_status="completed", added_at="2024-01-01")
        newer = FakePaper(id="P2", title="Paper One", doi="10.1234/test",
                         parse_status="pending", added_at="2024-01-02")
        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase(pairs=[(older, newer)])
            MockDB.return_value = mock_db
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_dedup(FakeArgs(report=False, dry_run=False, auto=True,
                                          batch=False, keep="older", since=""))
            out = captured.getvalue()
            assert "Auto-merged" in out
            assert mock_db.merged == [("P1", "P2")]
            assert rc == 0


# ─────────────────────────────────────────────────────────────────────────────
# dedup --batch
# ─────────────────────────────────────────────────────────────────────────────

class TestDedupBatch:
    def test_batch_merges_same_doi(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_dedup
        older = FakePaper(id="P1", title="Paper One", doi="10.1234/test",
                         parse_status="completed", added_at="2024-01-01")
        newer = FakePaper(id="P2", title="Paper One", doi="10.1234/test",
                         parse_status="pending", added_at="2024-01-02")
        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase(pairs=[(older, newer)])
            MockDB.return_value = mock_db
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_dedup(FakeArgs(report=False, dry_run=False, auto=False,
                                          batch=True, keep="older", since=""))
            out = captured.getvalue()
            assert "[batch] Merged" in out
            assert mock_db.merged == [("P1", "P2")]
            assert rc == 0

    def test_batch_skips_no_doi(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_dedup
        older = FakePaper(id="P1", title="Paper One", doi=None,
                         parse_status="completed", added_at="2024-01-01")
        newer = FakePaper(id="P2", title="Paper One", doi=None,
                         parse_status="pending", added_at="2024-01-02")
        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase(pairs=[(older, newer)])
            MockDB.return_value = mock_db
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_dedup(FakeArgs(report=False, dry_run=False, auto=False,
                                          batch=True, keep="older", since=""))
            out = captured.getvalue()
            assert "Skipped" in out
            assert mock_db.merged == []
            assert rc == 0


# ─────────────────────────────────────────────────────────────────────────────
# dedup-semantic --stats
# ─────────────────────────────────────────────────────────────────────────────

class TestDedupSemanticStats:
    def test_stats_empty(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_dedup_semantic
        with patch("cli.Database") as MockDB:
            mock_db = MagicMock()
            mock_db.get_embedding_stats.return_value = {
                "with_embedding": 0, "total_with_text": 0}
            MockDB.return_value = mock_db
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_dedup_semantic(FakeArgs(
                    stats=True, generate=False, paper=None, dry_run=False,
                    threshold=0.85, limit=20, format="text"))
            out = captured.getvalue()
            assert "Papers with embedding" in out
            assert rc == 0

    def test_stats_shows_coverage(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_dedup_semantic
        with patch("cli.Database") as MockDB:
            mock_db = MagicMock()
            mock_db.get_embedding_stats.return_value = {
                "with_embedding": 80, "total_with_text": 100}
            MockDB.return_value = mock_db
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_dedup_semantic(FakeArgs(
                    stats=True, generate=False, paper=None, dry_run=False,
                    threshold=0.85, limit=20, format="text"))
            out = captured.getvalue()
            assert "80.0%" in out
            assert rc == 0


# ─────────────────────────────────────────────────────────────────────────────
# dedup-semantic --paper PAPER_ID
# ─────────────────────────────────────────────────────────────────────────────

class TestDedupSemanticPaper:
    def test_paper_not_found(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_dedup_semantic
        with patch("cli.Database") as MockDB:
            mock_db = MagicMock()
            mock_db.paper_exists.return_value = False
            MockDB.return_value = mock_db
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_dedup_semantic(FakeArgs(
                    stats=False, generate=False, paper="notexist",
                    dry_run=False, threshold=0.85, limit=20, format="text"))
            assert rc == 1

    def test_paper_no_similar(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_dedup_semantic
        with patch("cli.Database") as MockDB:
            mock_db = MagicMock()
            mock_db.paper_exists.return_value = True
            mock_db.get_paper.return_value = FakePaper(id="P1", title="Test")
            mock_db.find_similar.return_value = []
            MockDB.return_value = mock_db
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_dedup_semantic(FakeArgs(
                    stats=False, generate=False, paper="P1",
                    dry_run=False, threshold=0.85, limit=20, format="text"))
            out = captured.getvalue()
            assert "No similar papers found" in out
            assert rc == 0

    def test_paper_shows_similar(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_dedup_semantic
        with patch("cli.Database") as MockDB:
            mock_db = MagicMock()
            mock_db.paper_exists.return_value = True
            mock_db.get_paper.return_value = FakePaper(id="P1", title="Test Paper")
            sim_paper = FakePaper(id="P2", title="Similar Paper")
            mock_db.find_similar.return_value = [(sim_paper, 0.9234)]
            MockDB.return_value = mock_db
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_dedup_semantic(FakeArgs(
                    stats=False, generate=False, paper="P1",
                    dry_run=False, threshold=0.85, limit=20, format="text"))
            out = captured.getvalue()
            assert "0.9234" in out
            assert "P2" in out
            assert rc == 0

    def test_threshold_and_limit_passed_to_find_similar(self, monkeypatch):
        """--threshold and --limit values are forwarded to db.find_similar."""
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_dedup_semantic
        with patch("cli.Database") as MockDB:
            mock_db = MagicMock()
            mock_db.paper_exists.return_value = True
            mock_db.get_paper.return_value = FakePaper(id="P1", title="Test Paper")
            mock_db.find_similar.return_value = []
            MockDB.return_value = mock_db
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_dedup_semantic(FakeArgs(
                    stats=False, generate=False, paper="P1",
                    dry_run=False, threshold=0.92, limit=5, format="text"))
            mock_db.find_similar.assert_called_once_with("P1", threshold=0.92, limit=5)
            assert rc == 0

    def test_custom_threshold_shows_only_above_threshold(self, monkeypatch):
        """find_similar returns only results >= threshold; CLI displays all it receives."""
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_dedup_semantic
        with patch("cli.Database") as MockDB:
            mock_db = MagicMock()
            mock_db.paper_exists.return_value = True
            mock_db.get_paper.return_value = FakePaper(id="P1", title="Test Paper")
            # Only P2 (0.95) passes threshold=0.90; P3 (0.87) is filtered by db
            p2 = FakePaper(id="P2", title="Very Similar")
            mock_db.find_similar.return_value = [(p2, 0.95)]
            MockDB.return_value = mock_db
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_dedup_semantic(FakeArgs(
                    stats=False, generate=False, paper="P1",
                    dry_run=False, threshold=0.90, limit=20, format="text"))
            out = captured.getvalue()
            assert "P2" in out
            assert "P3" not in out
            assert rc == 0

    def test_paper_csv_format(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_dedup_semantic
        with patch("cli.Database") as MockDB:
            mock_db = MagicMock()
            mock_db.paper_exists.return_value = True
            mock_db.get_paper.return_value = FakePaper(id="P1", title="Test Paper")
            sim_paper = FakePaper(id="P2", title="Similar Paper")
            mock_db.find_similar.return_value = [(sim_paper, 0.9234)]
            MockDB.return_value = mock_db
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_dedup_semantic(FakeArgs(
                    stats=False, generate=False, paper="P1",
                    dry_run=False, threshold=0.85, limit=20, format="csv"))
            out = captured.getvalue()
            assert out.startswith("paper_a,paper_b,similarity,title_a,title_b")
            assert "P1,P2,0.9234" in out
            assert rc == 0


# ─────────────────────────────────────────────────────────────────────────────
# dedup-semantic --generate
# ─────────────────────────────────────────────────────────────────────────────

class TestDedupSemanticGenerate:
    def test_generate_calls_ollama(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_dedup_semantic
        with patch("cli.Database") as MockDB:
            mock_db = MagicMock()
            paper = FakePaper(id="P1", title="Test Paper\n\nAbstract text")
            mock_db.get_papers_without_embeddings.return_value = [paper]
            MockDB.return_value = mock_db
            with patch("cli._get_ollama_embedding", return_value=[0.1]*768):
                captured = StringIO()
                with patch("sys.stdout", captured):
                    rc = _run_dedup_semantic(FakeArgs(
                        stats=False, generate=True, paper=None,
                        dry_run=False, threshold=0.85, limit=20, format="text"))
            out = captured.getvalue()
            assert "Generated: 1" in out
            mock_db.set_embedding.assert_called_once()
            assert rc == 0


# ─────────────────────────────────────────────────────────────────────────────
# dedup-semantic global (all papers, no --paper flag)
# ─────────────────────────────────────────────────────────────────────────────

class TestDedupSemanticGlobal:
    def test_global_no_papers_with_text(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_dedup_semantic
        with patch("cli.Database") as MockDB:
            mock_db = MagicMock()
            mock_db.list_papers.return_value = ([], 0)
            MockDB.return_value = mock_db
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_dedup_semantic(FakeArgs(
                    stats=False, generate=False, paper=None,
                    dry_run=False, threshold=0.85, limit=20, format="text"))
            assert rc == 0


# ─────────────────────────────────────────────────────────────────────────────
# dedup-semantic CSV output (global mode)
# ─────────────────────────────────────────────────────────────────────────────

class TestDedupSemanticCsvGlobal:
    def test_global_csv_header(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_dedup_semantic
        with patch("cli.Database") as MockDB:
            mock_db = MagicMock()
            mock_db.list_papers.return_value = ([], 0)
            MockDB.return_value = mock_db
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_dedup_semantic(FakeArgs(
                    stats=False, generate=False, paper=None,
                    dry_run=False, threshold=0.85, limit=20, format="csv"))
            out = captured.getvalue()
            assert out.startswith("paper_a,paper_b,similarity,title_a,title_b")
            assert rc == 0
