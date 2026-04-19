"""Unit tests for merge CLI subcommand."""
from io import StringIO
from unittest.mock import patch


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
    def __init__(self, papers=None):
        self.papers = papers or {}  # id -> FakePaper
        self.merged = []
        self.logged = []
        self.init_called = False

    def init(self):
        self.init_called = True

    def get_paper(self, paper_id):
        return self.papers.get(paper_id)

    def merge_papers(self, keep_id, dup_id):
        self.merged.append((keep_id, dup_id))
        return True

    def log_dedup(self, keep_id, dup_id, policy):
        self.logged.append((keep_id, dup_id, policy))

    def list_papers(self, limit=100):
        return list(self.papers.values()), len(self.papers)

    def find_similar(self, paper_id, threshold=0.85, limit=20):
        return []

    def get_similarity(self, id1, id2):
        return None


# ─────────────────────────────────────────────────────────────────────────────
# merge: missing arguments
# ─────────────────────────────────────────────────────────────────────────────

class TestMergeArgs:
    def test_missing_both_ids(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_merge
        with patch("cli.Database") as MockDB:
            MockDB.return_value = FakeDatabase()
            rc = _run_merge(FakeArgs(target_id=None, duplicate_id=None,
                                      keep="older", dry_run=False, auto=False))
            assert rc == 1

    def test_missing_duplicate_id(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_merge
        with patch("cli.Database") as MockDB:
            MockDB.return_value = FakeDatabase()
            rc = _run_merge(FakeArgs(target_id="P1", duplicate_id=None,
                                      keep="older", dry_run=False, auto=False))
            assert rc == 1


# ─────────────────────────────────────────────────────────────────────────────
# merge: paper not found
# ─────────────────────────────────────────────────────────────────────────────

class TestMergeNotFound:
    def test_target_not_found(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_merge
        with patch("cli.Database") as MockDB:
            MockDB.return_value = FakeDatabase()
            rc = _run_merge(FakeArgs(target_id="notexist", duplicate_id="P2",
                                      keep="older", dry_run=False, auto=False))
            assert rc == 1

    def test_duplicate_not_found(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_merge
        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase()
            mock_db.papers["P1"] = FakePaper(id="P1")
            MockDB.return_value = mock_db
            rc = _run_merge(FakeArgs(target_id="P1", duplicate_id="notexist",
                                      keep="older", dry_run=False, auto=False))
            assert rc == 1


# ─────────────────────────────────────────────────────────────────────────────
# merge --dry-run
# ─────────────────────────────────────────────────────────────────────────────

class TestMergeDryRun:
    def test_dry_run_shows_would_merge(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_merge
        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase()
            mock_db.papers["P1"] = FakePaper(id="P1", title="Paper One",
                                              added_at="2024-01-01")
            mock_db.papers["P2"] = FakePaper(id="P2", title="Paper One dup",
                                              added_at="2024-01-02")
            MockDB.return_value = mock_db
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_merge(FakeArgs(target_id="P1", duplicate_id="P2",
                                          keep="older", dry_run=True, auto=False))
            out = captured.getvalue()
            assert "Would merge P2 into P1" in out
            assert "--keep=older" in out
            assert mock_db.merged == []
            assert rc == 0


# ─────────────────────────────────────────────────────────────────────────────
# merge: keep strategies
# ─────────────────────────────────────────────────────────────────────────────

class TestMergeKeepStrategies:
    def test_keep_older(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_merge
        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase()
            mock_db.papers["P1"] = FakePaper(id="P1", title="Older Paper",
                                              added_at="2024-01-01")
            mock_db.papers["P2"] = FakePaper(id="P2", title="Newer Paper",
                                              added_at="2024-01-02")
            MockDB.return_value = mock_db
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_merge(FakeArgs(target_id="P1", duplicate_id="P2",
                                          keep="older", dry_run=False, auto=False))
            assert mock_db.merged == [("P1", "P2")]
            assert rc == 0

    def test_keep_newer(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_merge
        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase()
            mock_db.papers["P1"] = FakePaper(id="P1", title="Older Paper",
                                              added_at="2024-01-01")
            mock_db.papers["P2"] = FakePaper(id="P2", title="Newer Paper",
                                              added_at="2024-01-02")
            MockDB.return_value = mock_db
            rc = _run_merge(FakeArgs(target_id="P1", duplicate_id="P2",
                                      keep="newer", dry_run=False, auto=False))
            assert mock_db.merged == [("P2", "P1")]
            assert rc == 0

    def test_keep_parsed(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_merge
        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase()
            # target (P1) is older but pending; P2 is newer but completed
            mock_db.papers["P1"] = FakePaper(id="P1", title="Older Paper",
                                              parse_status="pending",
                                              added_at="2024-01-01")
            mock_db.papers["P2"] = FakePaper(id="P2", title="Newer Paper",
                                              parse_status="completed",
                                              added_at="2024-01-02")
            MockDB.return_value = mock_db
            rc = _run_merge(FakeArgs(target_id="P1", duplicate_id="P2",
                                      keep="parsed", dry_run=False, auto=False))
            # "parsed" keeps better parse_status regardless of added_at
            assert mock_db.merged == [("P2", "P1")]
            assert rc == 0


# ─────────────────────────────────────────────────────────────────────────────
# merge --keep semantic
# ─────────────────────────────────────────────────────────────────────────────

class TestMergeKeepSemantic:
    def test_semantic_high_similarity(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_merge
        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase()
            mock_db.papers["P1"] = FakePaper(id="P1", title="Paper One",
                                              parse_status="pending",
                                              added_at="2024-01-01")
            mock_db.papers["P2"] = FakePaper(id="P2", title="Paper One dup",
                                              parse_status="pending",
                                              added_at="2024-01-02")
            mock_db.get_similarity = lambda id1, id2: 0.95
            MockDB.return_value = mock_db
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_merge(FakeArgs(target_id="P1", duplicate_id="P2",
                                          keep="semantic", dry_run=False, auto=False))
            out = captured.getvalue()
            assert "Auto-selected: similarity 0.950" in out
            assert mock_db.merged == [("P1", "P2")]
            assert mock_db.logged[0][2] == "semantic"
            assert rc == 0

    def test_semantic_low_similarity_falls_back_to_parsed(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_merge
        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase()
            mock_db.papers["P1"] = FakePaper(id="P1", title="Paper One",
                                              parse_status="completed",
                                              added_at="2024-01-01")
            mock_db.papers["P2"] = FakePaper(id="P2", title="Different Paper",
                                              parse_status="pending",
                                              added_at="2024-01-02")
            mock_db.get_similarity = lambda id1, id2: 0.50
            MockDB.return_value = mock_db
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_merge(FakeArgs(target_id="P1", duplicate_id="P2",
                                          keep="semantic", dry_run=False, auto=False))
            out = captured.getvalue()
            assert "low similarity" in out
            # Falls back to "parsed" which keeps P1 (completed > pending)
            assert mock_db.merged == [("P1", "P2")]
            assert rc == 0

    def test_semantic_no_embedding(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_merge
        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase()
            mock_db.papers["P1"] = FakePaper(id="P1", title="Paper One",
                                              parse_status="completed",
                                              added_at="2024-01-01")
            mock_db.papers["P2"] = FakePaper(id="P2", title="Paper Two",
                                              parse_status="pending",
                                              added_at="2024-01-02")
            mock_db.get_similarity = lambda id1, id2: None
            MockDB.return_value = mock_db
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_merge(FakeArgs(target_id="P1", duplicate_id="P2",
                                          keep="semantic", dry_run=False, auto=False))
            out = captured.getvalue()
            assert "low similarity" in out
            assert mock_db.merged == [("P1", "P2")]
            assert rc == 0


# ─────────────────────────────────────────────────────────────────────────────
# merge: dry-run shows semantic similarity
# ─────────────────────────────────────────────────────────────────────────────

class TestMergeDryRunSimilarity:
    def test_dry_run_shows_similarity(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_merge
        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase()
            mock_db.papers["P1"] = FakePaper(id="P1", title="Paper One",
                                              added_at="2024-01-01")
            mock_db.papers["P2"] = FakePaper(id="P2", title="Paper One dup",
                                              added_at="2024-01-02")
            mock_db.get_similarity = lambda id1, id2: 0.923
            MockDB.return_value = mock_db
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_merge(FakeArgs(target_id="P1", duplicate_id="P2",
                                          keep="older", dry_run=True, auto=False))
            out = captured.getvalue()
            assert "semantic similarity: 0.923" in out
            assert mock_db.merged == []
            assert rc == 0

    def test_dry_run_no_embedding(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_merge
        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase()
            mock_db.papers["P1"] = FakePaper(id="P1", title="Paper One",
                                              added_at="2024-01-01")
            mock_db.papers["P2"] = FakePaper(id="P2", title="Paper One dup",
                                              added_at="2024-01-02")
            mock_db.get_similarity = lambda id1, id2: None
            MockDB.return_value = mock_db
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_merge(FakeArgs(target_id="P1", duplicate_id="P2",
                                          keep="older", dry_run=True, auto=False))
            out = captured.getvalue()
            assert "no embeddings available" in out
            assert rc == 0


# ─────────────────────────────────────────────────────────────────────────────
# merge --auto
# ─────────────────────────────────────────────────────────────────────────────

class TestMergeAuto:
    def test_auto_finds_and_merges_similar(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_merge
        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase()
            p1 = FakePaper(id="P1", title="Paper One", added_at="2024-01-01")
            p2 = FakePaper(id="P2", title="Paper One Copy", added_at="2024-01-02")
            mock_db.papers = {"P1": p1, "P2": p2}
            mock_db.find_similar = lambda pid, threshold, limit: [(p2, 0.97)] if pid == "P1" else []
            mock_db.get_similarity = lambda id1, id2: 0.97
            MockDB.return_value = mock_db
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_merge(FakeArgs(target_id=None, duplicate_id=None,
                                          keep="semantic", dry_run=False, auto=True))
            out = captured.getvalue()
            assert "Merged P2 into P1" in out
            assert mock_db.merged == [("P1", "P2")]
            assert mock_db.logged[0][2] == "semantic-auto"
            assert rc == 0

    def test_auto_dry_run_shows_pairs(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_merge
        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase()
            p1 = FakePaper(id="P1", title="Paper One", added_at="2024-01-01")
            p2 = FakePaper(id="P2", title="Paper One Copy", added_at="2024-01-02")
            mock_db.papers = {"P1": p1, "P2": p2}
            mock_db.find_similar = lambda pid, threshold, limit: [(p2, 0.97)] if pid == "P1" else []
            mock_db.get_similarity = lambda id1, id2: 0.97
            MockDB.return_value = mock_db
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_merge(FakeArgs(target_id=None, duplicate_id=None,
                                          keep="semantic", dry_run=True, auto=True))
            out = captured.getvalue()
            assert "Would merge P2 into P1" in out
            assert "similarity: 0.970" in out
            assert mock_db.merged == []
            assert rc == 0

    def test_auto_empty_db(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_merge
        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase()
            MockDB.return_value = mock_db
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_merge(FakeArgs(target_id=None, duplicate_id=None,
                                          keep="semantic", dry_run=False, auto=True))
            out = captured.getvalue()
            assert "Auto-merge complete: 0" in out
            assert rc == 0

    def test_auto_skips_seen_pairs(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _run_merge
        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase()
            p1 = FakePaper(id="P1", title="Paper One", added_at="2024-01-01")
            p2 = FakePaper(id="P2", title="Paper One Copy", added_at="2024-01-02")
            p3 = FakePaper(id="P3", title="Paper Three", added_at="2024-01-03")
            mock_db.papers = {"P1": p1, "P2": p2, "P3": p3}
            # P2 found similar to P1, P1 found similar to P2 (but should only merge once)
            def find_similar(pid, threshold, limit):
                if pid == "P1":
                    return [(p2, 0.97)]
                if pid == "P2":
                    return [(p1, 0.97)]
                return []
            mock_db.find_similar = find_similar
            mock_db.get_similarity = lambda id1, id2: 0.97
            MockDB.return_value = mock_db
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_merge(FakeArgs(target_id=None, duplicate_id=None,
                                          keep="semantic", dry_run=False, auto=True))
            # Should only merge once (P1-P2 pair)
            assert len(mock_db.merged) == 1
            assert rc == 0


# ─────────────────────────────────────────────────────────────────────────────
# _pick_keep
# ─────────────────────────────────────────────────────────────────────────────

class TestPickKeep:
    def test_pick_keep_older(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _pick_keep
        older = FakePaper(id="P1", added_at="2024-01-01")
        newer = FakePaper(id="P2", added_at="2024-01-02")
        keep, drop = _pick_keep(older, newer, "older")
        assert keep.id == "P1"
        assert drop.id == "P2"

    def test_pick_keep_newer(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _pick_keep
        older = FakePaper(id="P1", added_at="2024-01-01")
        newer = FakePaper(id="P2", added_at="2024-01-02")
        keep, drop = _pick_keep(older, newer, "newer")
        assert keep.id == "P2"
        assert drop.id == "P1"

    def test_pick_keep_parsed(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli import _pick_keep
        older = FakePaper(id="P1", parse_status="pending", added_at="2024-01-01")
        newer = FakePaper(id="P2", parse_status="completed", added_at="2024-01-02")
        keep, drop = _pick_keep(older, newer, "parsed")
        # "parsed" ignores added_at, keeps better parse_status
        assert keep.id == "P2"
        assert drop.id == "P1"
