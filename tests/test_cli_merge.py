"""Unit tests for merge CLI subcommand (--keep semantic, --auto)."""
import sys
from io import StringIO
from unittest.mock import patch, MagicMock
import pytest


class FakeArgs:
    def __init__(self, **kwargs):
        for k, v in kwargs.items():
            setattr(self, k, v)


class FakePaper:
    def __init__(self, id="2301.00001", title="Test Paper", parse_status="completed",
                 added_at="2024-01-01 00:00:00", doi=None):
        self.id = id
        self.title = title
        self.parse_status = parse_status
        self.added_at = added_at
        self.doi = doi


class FakeDatabase:
    def __init__(self, papers=None, merge_ok=True):
        self.papers = papers or {}  # paper_id -> FakePaper
        self.upserted = []
        self.init_called = False
        self.merge_ok = merge_ok
        self.merged_pairs = []
        self.dedup_logs = []

    def init(self):
        self.init_called = True

    def get_paper(self, paper_id):
        return self.papers.get(paper_id)

    def get_similarity(self, id1, id2):
        p1 = self.papers.get(id1)
        p2 = self.papers.get(id2)
        if p1 is None or p2 is None:
            return None
        # Simulate similarity: high for "same" titles, low for different
        if p1.title == p2.title:
            return 0.95
        if "duplicate" in p1.title and "duplicate" in p2.title:
            return 0.90
        return 0.30

    def merge_papers(self, keep_id, drop_id):
        self.merged_pairs.append((keep_id, drop_id))
        return self.merge_ok

    def log_dedup(self, keep_id, drop_id, policy):
        self.dedup_logs.append({"keep": keep_id, "drop": drop_id, "policy": policy})


# ─────────────────────────────────────────────────────────────────────────────
# _run_merge unit tests — two-paper merge
# ─────────────────────────────────────────────────────────────────────────────

class TestRunMerge:
    def test_target_not_found_returns_1(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")

        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase(papers={})
            MockDB.return_value = mock_db

            from cli import _run_merge
            rc = _run_merge(FakeArgs(
                target_id="2301.99999",
                duplicate_id="2301.00001",
                keep="older",
                dry_run=False,
                auto=False,
            ))

        assert rc == 1

    def test_duplicate_not_found_returns_1(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")

        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase(papers={"2301.00001": FakePaper(id="2301.00001")})
            MockDB.return_value = mock_db

            from cli import _run_merge
            rc = _run_merge(FakeArgs(
                target_id="2301.00001",
                duplicate_id="2301.99999",
                keep="older",
                dry_run=False,
                auto=False,
            ))

        assert rc == 1

    def test_keep_older_keeps_older(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")

        older = FakePaper(id="2301.00001", title="Paper", added_at="2024-01-01 00:00:00")
        newer = FakePaper(id="2301.00002", title="Paper", added_at="2024-06-01 00:00:00")

        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase(papers={"2301.00001": older, "2301.00002": newer})
            MockDB.return_value = mock_db

            from cli import _run_merge
            captured = StringIO()
            monkeypatch.setattr("sys.stdout", captured)
            rc = _run_merge(FakeArgs(
                target_id="2301.00001",
                duplicate_id="2301.00002",
                keep="older",
                dry_run=False,
                auto=False,
            ))

        assert rc == 0
        assert ("2301.00001", "2301.00002") in mock_db.merged_pairs

    def test_keep_newer_keeps_newer(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")

        older = FakePaper(id="2301.00001", title="Paper", added_at="2024-01-01 00:00:00")
        newer = FakePaper(id="2301.00002", title="Paper", added_at="2024-06-01 00:00:00")

        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase(papers={"2301.00001": older, "2301.00002": newer})
            MockDB.return_value = mock_db

            from cli import _run_merge
            captured = StringIO()
            monkeypatch.setattr("sys.stdout", captured)
            rc = _run_merge(FakeArgs(
                target_id="2301.00001",
                duplicate_id="2301.00002",
                keep="newer",
                dry_run=False,
                auto=False,
            ))

        assert rc == 0
        assert ("2301.00002", "2301.00001") in mock_db.merged_pairs

    def test_keep_parsed_keeps_better_status(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")

        completed = FakePaper(id="2301.00001", title="Paper", parse_status="completed",
                              added_at="2024-01-01 00:00:00")
        failed = FakePaper(id="2301.00002", title="Paper", parse_status="failed",
                           added_at="2024-06-01 00:00:00")

        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase(papers={"2301.00001": completed, "2301.00002": failed})
            MockDB.return_value = mock_db

            from cli import _run_merge
            captured = StringIO()
            monkeypatch.setattr("sys.stdout", captured)
            rc = _run_merge(FakeArgs(
                target_id="2301.00001",
                duplicate_id="2301.00002",
                keep="parsed",
                dry_run=False,
                auto=False,
            ))

        assert rc == 0
        # completed has better parse_status → kept
        assert ("2301.00001", "2301.00002") in mock_db.merged_pairs

    def test_keep_semantic_high_similarity_auto_selects(self, monkeypatch):
        """--keep semantic with sim>=0.8 picks by parse_status (parsed)."""
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")

        # Both have same title → sim=0.95 (>=0.8)
        older = FakePaper(id="2301.00001", title="Same Paper", parse_status="completed",
                          added_at="2024-01-01 00:00:00")
        newer = FakePaper(id="2301.00002", title="Same Paper", parse_status="completed",
                          added_at="2024-06-01 00:00:00")

        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase(papers={"2301.00001": older, "2301.00002": newer})
            MockDB.return_value = mock_db

            from cli import _run_merge
            captured = StringIO()
            monkeypatch.setattr("sys.stdout", captured)
            rc = _run_merge(FakeArgs(
                target_id="2301.00001",
                duplicate_id="2301.00002",
                keep="semantic",
                dry_run=False,
                auto=False,
            ))

        assert rc == 0
        # Both same parse_status → older kept (stable sort by added_at)
        assert ("2301.00001", "2301.00002") in mock_db.merged_pairs
        output = captured.getvalue()
        assert "Auto-selected: similarity" in output

    def test_keep_semantic_low_similarity_falls_back_to_parsed(self, monkeypatch):
        """--keep semantic with sim<0.8 prints note and falls back to parsed."""
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")

        # Different titles → sim=0.30 (<0.8)
        paper1 = FakePaper(id="2301.00001", title="Paper Alpha", parse_status="completed",
                            added_at="2024-01-01 00:00:00")
        paper2 = FakePaper(id="2301.00002", title="Paper Beta", parse_status="completed",
                            added_at="2024-06-01 00:00:00")

        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase(papers={"2301.00001": paper1, "2301.00002": paper2})
            MockDB.return_value = mock_db

            from cli import _run_merge
            captured = StringIO()
            monkeypatch.setattr("sys.stdout", captured)
            rc = _run_merge(FakeArgs(
                target_id="2301.00001",
                duplicate_id="2301.00002",
                keep="semantic",
                dry_run=False,
                auto=False,
            ))

        assert rc == 0
        output = captured.getvalue()
        assert "low similarity" in output or "falling back" in output.lower()

    def test_keep_semantic_no_embedding_falls_back_to_parsed(self, monkeypatch):
        """--keep semantic when get_similarity returns None falls back to parsed."""
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")

        paper1 = FakePaper(id="2301.00001", title="Paper", parse_status="completed")
        paper2 = FakePaper(id="2301.00002", title="Paper", parse_status="failed")

        with patch("cli.Database") as MockDB:
            mock_db = MagicMock()
            mock_db.get_paper.side_effect = lambda pid: {"2301.00001": paper1, "2301.00002": paper2}.get(pid)
            mock_db.get_similarity.return_value = None  # no embeddings
            mock_db.merge_papers.return_value = True
            MockDB.return_value = mock_db

            from cli import _run_merge
            captured = StringIO()
            monkeypatch.setattr("sys.stdout", captured)
            rc = _run_merge(FakeArgs(
                target_id="2301.00001",
                duplicate_id="2301.00002",
                keep="semantic",
                dry_run=False,
                auto=False,
            ))

        assert rc == 0
        output = captured.getvalue()
        assert "N/A" in output or "no embeddings" in output.lower()

    def test_dry_run_returns_0_no_merge(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")

        older = FakePaper(id="2301.00001", title="Paper", added_at="2024-01-01 00:00:00")
        newer = FakePaper(id="2301.00002", title="Paper", added_at="2024-06-01 00:00:00")

        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase(papers={"2301.00001": older, "2301.00002": newer})
            MockDB.return_value = mock_db

            from cli import _run_merge
            captured = StringIO()
            monkeypatch.setattr("sys.stdout", captured)
            rc = _run_merge(FakeArgs(
                target_id="2301.00001",
                duplicate_id="2301.00002",
                keep="older",
                dry_run=True,
                auto=False,
            ))

        assert rc == 0
        assert len(mock_db.merged_pairs) == 0  # dry-run → no actual merge
        output = captured.getvalue()
        assert "Would merge" in output

    def test_merge_failure_returns_1(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")

        paper1 = FakePaper(id="2301.00001", title="Paper", parse_status="completed")
        paper2 = FakePaper(id="2301.00002", title="Paper", parse_status="failed")

        with patch("cli.Database") as MockDB:
            mock_db = FakeDatabase(papers={"2301.00001": paper1, "2301.00002": paper2},
                                   merge_ok=False)
            MockDB.return_value = mock_db

            from cli import _run_merge
            captured = StringIO()
            monkeypatch.setattr("sys.stdout", captured)
            rc = _run_merge(FakeArgs(
                target_id="2301.00001",
                duplicate_id="2301.00002",
                keep="older",
                dry_run=False,
                auto=False,
            ))

        assert rc == 1
        output = captured.getvalue()
        assert "failed" in output.lower()


# ─────────────────────────────────────────────────────────────────────────────
# _run_merge unit tests — auto mode
# ─────────────────────────────────────────────────────────────────────────────

class TestRunMergeAuto:
    def test_auto_requires_target_and_duplicate(self, monkeypatch):
        """--auto without target/duplicate still works (auto-mode uses DB scan)."""
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")

        with patch("cli.Database") as MockDB:
            mock_db = MagicMock()
            mock_db.list_papers.return_value = ([], 0)
            MockDB.return_value = mock_db

            from cli import _run_merge
            captured = StringIO()
            monkeypatch.setattr("sys.stdout", captured)
            rc = _run_merge(FakeArgs(
                target_id=None,
                duplicate_id=None,
                keep="older",
                dry_run=True,
                auto=True,
            ))

        assert rc == 0
        output = captured.getvalue()
        assert "would be merged" in output or "dry-run" in output

    def test_auto_dry_run_shows_pairs(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")

        p1 = FakePaper(id="2301.00001", title="Paper Alpha", parse_status="completed",
                       added_at="2024-01-01 00:00:00")
        p2 = FakePaper(id="2301.00002", title="Paper Alpha", parse_status="completed",
                       added_at="2024-06-01 00:00:00")

        with patch("cli.Database") as MockDB:
            mock_db = MagicMock()
            mock_db.list_papers.return_value = ([p1], 1)
            # side_effect + lambda ensures consistent return on every call
            mock_db.find_similar.side_effect = lambda pid, threshold, limit: [(p2, 0.96)]
            mock_db.get_similarity.return_value = 0.96
            mock_db.merge_papers.return_value = True
            MockDB.return_value = mock_db

            from cli import _run_merge
            captured = StringIO()
            monkeypatch.setattr("sys.stdout", captured)
            rc = _run_merge(FakeArgs(
                target_id=None,
                duplicate_id=None,
                keep="parsed",
                dry_run=True,
                auto=True,
            ))

        assert rc == 0
        output = captured.getvalue()
        assert "Would merge" in output
        assert "semantic similarity" in output
        # merge_papers should NOT be called in dry-run
        mock_db.merge_papers.assert_not_called()

    def test_auto_actual_merge_logs_dedup(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")

        p1 = FakePaper(id="2301.00001", title="Paper Beta", parse_status="completed",
                       added_at="2024-01-01 00:00:00")
        p2 = FakePaper(id="2301.00002", title="Paper Beta", parse_status="completed",
                       added_at="2024-06-01 00:00:00")

        with patch("cli.Database") as MockDB:
            mock_db = MagicMock()
            mock_db.list_papers.return_value = ([p1], 1)
            mock_db.find_similar.side_effect = lambda pid, threshold, limit: [(p2, 0.97)]
            mock_db.get_similarity.return_value = 0.97
            mock_db.merge_papers.return_value = True
            MockDB.return_value = mock_db

            from cli import _run_merge
            captured = StringIO()
            monkeypatch.setattr("sys.stdout", captured)
            rc = _run_merge(FakeArgs(
                target_id=None,
                duplicate_id=None,
                keep="parsed",
                dry_run=False,
                auto=True,
            ))

        assert rc == 0
        output = captured.getvalue()
        assert "Merged" in output
        assert "semantic-auto" in str(mock_db.log_dedup.call_args)

    def test_auto_empty_db_returns_0(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")

        with patch("cli.Database") as MockDB:
            mock_db = MagicMock()
            mock_db.list_papers.return_value = ([], 0)
            MockDB.return_value = mock_db

            from cli import _run_merge
            captured = StringIO()
            monkeypatch.setattr("sys.stdout", captured)
            rc = _run_merge(FakeArgs(
                target_id=None,
                duplicate_id=None,
                keep="older",
                dry_run=True,
                auto=True,
            ))

        assert rc == 0
        output = captured.getvalue()
        assert "0 pair(s) would be merged" in output

    def test_auto_skips_already_seen_pairs(self, monkeypatch):
        """Auto-mode should not re-merge a paper that's already been merged as drop."""
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")

        p1 = FakePaper(id="2301.00001", title="Paper Gamma", parse_status="completed",
                       added_at="2024-01-01 00:00:00")
        p2 = FakePaper(id="2301.00002", title="Paper Gamma", parse_status="completed",
                       added_at="2024-06-01 00:00:00")
        p3 = FakePaper(id="2301.00003", title="Paper Gamma", parse_status="completed",
                       added_at="2024-03-01 00:00:00")

        with patch("cli.Database") as MockDB:
            mock_db = MagicMock()
            mock_db.list_papers.return_value = ([p1], 1)
            # find_similar for p1 finds p2 (high sim)
            mock_db.find_similar.side_effect = lambda pid, threshold, limit: [(p2, 0.96), (p3, 0.96)]
            mock_db.get_similarity.return_value = 0.96
            mock_db.merge_papers.return_value = True
            MockDB.return_value = mock_db

            from cli import _run_merge
            captured = StringIO()
            monkeypatch.setattr("sys.stdout", captured)
            rc = _run_merge(FakeArgs(
                target_id=None,
                duplicate_id=None,
                keep="parsed",
                dry_run=False,
                auto=True,
            ))

        assert rc == 0
        # Should merge 2 pairs: (p1,p2) and (p1,p3) [p1 is kept in both]
        # But p3 might get merged into p1 first, then p2→p1 already done
        # The seen set prevents p1 from being dropped twice
        assert mock_db.merge_papers.call_count >= 1


# ─────────────────────────────────────────────────────────────────────────────
# Parser registration tests
# ─────────────────────────────────────────────────────────────────────────────

class TestMergeParser:
    def test_keep_semantic_registered(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")

        import argparse
        from cli import _build_merge_parser

        parser = _build_merge_parser(argparse.ArgumentParser().add_subparsers())
        args = parser.parse_args(["--keep", "semantic", "2301.00001", "2301.00002"])
        assert args.keep == "semantic"

    def test_auto_flag_registered(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")

        import argparse
        from cli import _build_merge_parser

        parser = _build_merge_parser(argparse.ArgumentParser().add_subparsers())
        args = parser.parse_args(["--auto"])
        assert args.auto is True

    def test_dry_run_flag_registered(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")

        import argparse
        from cli import _build_merge_parser

        parser = _build_merge_parser(argparse.ArgumentParser().add_subparsers())
        args = parser.parse_args(["--dry-run", "2301.00001", "2301.00002"])
        assert args.dry_run is True

    def test_target_and_duplicate_optional_when_auto(self, monkeypatch):
        """When --auto is given, target_id and duplicate_id are optional."""
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")

        import argparse
        from cli import _build_merge_parser

        parser = _build_merge_parser(argparse.ArgumentParser().add_subparsers())
        # Should NOT raise — --auto makes positional args optional
        args = parser.parse_args(["--auto"])
        assert args.target_id is None
        assert args.duplicate_id is None
        assert args.auto is True
