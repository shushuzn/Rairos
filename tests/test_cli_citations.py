"""Unit tests for CLI citations, cite-stats, cite-fetch, cite-import, export, and stats subcommands."""
import argparse
import json
from unittest.mock import MagicMock, patch

import pytest

from cli import _run_stats, _run_import, _run_export, _run_citations, _run_cite_fetch, _run_cite_import, _run_cite_stats, main


# ─────────────────────────────────────────────────────────────────────────────
# Helpers
# ─────────────────────────────────────────────────────────────────────────────

def make_args(**kwargs):
    defaults = dict(
        subcmd="", json=False, limit=20, offset=0, format="text",
        source="import", skip_existing=False, out=None,
        citation_from=None, citation_to=None,
        paper_id=None, direction="both", dry_run=False, skip_external=False,
        delay=0.11, max_per_paper=0,
        json_input=None, skip_missing=False, dedup=False,
        extract=False, extract_paper=None,
        stats_paper=None, top=None,
        ids=None, file=None,
    )
    defaults.update(kwargs)
    ns = argparse.Namespace()
    for k, v in defaults.items():
        setattr(ns, k, v)
    return ns


# ─────────────────────────────────────────────────────────────────────────────
# _run_stats tests
# ─────────────────────────────────────────────────────────────────────────────

class TestRunStats:
    """Test _run_stats."""

    @patch("cli.Database")
    def test_stats_text_mode(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_stats.return_value = {
            "total_papers": 42,
            "by_source": {"arxiv": 30, "import": 12},
            "by_status": {"done": 20, "pending": 22},
            "queue_queued": 5,
            "queue_running": 1,
            "cache_entries": 100,
            "dedup_records": 200,
        }
        mock_db_cls.return_value = mock_db

        args = make_args(subcmd="stats", json=False)
        result = _run_stats(args)

        captured = capsys.readouterr().out
        assert "42" in captured
        assert "total" in captured.lower()
        assert result == 0

    @patch("cli.Database")
    def test_stats_json_mode(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_stats.return_value = {
            "total_papers": 42,
            "by_source": {"arxiv": 42},
            "by_status": {"done": 42},
            "queue_queued": 0,
            "queue_running": 0,
            "cache_entries": 0,
            "dedup_records": 0,
        }
        mock_db_cls.return_value = mock_db

        args = make_args(subcmd="stats", json=True)
        result = _run_stats(args)

        captured = capsys.readouterr().out
        # _run_stats always prints text stats then JSON; find the JSON boundary
        brace_start = captured.find("{")
        brace_end = captured.rfind("}")
        if brace_start >= 0 and brace_end > brace_start:
            json_text = captured[brace_start : brace_end + 1]
            data = json.loads(json_text)
            assert data["total_papers"] == 42
        assert result == 0

    @patch("cli.Database")
    def test_stats_empty_db(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_stats.return_value = {
            "total_papers": 0,
            "by_source": {},
            "by_status": {},
            "queue_queued": 0,
            "queue_running": 0,
            "cache_entries": 0,
            "dedup_records": 0,
        }
        mock_db_cls.return_value = mock_db

        args = make_args(subcmd="stats", json=False)
        result = _run_stats(args)

        captured = capsys.readouterr().out
        assert "0" in captured
        assert result == 0


# ─────────────────────────────────────────────────────────────────────────────
# _run_citations tests
# ─────────────────────────────────────────────────────────────────────────────

class TestRunCitations:
    """Test _run_citations."""

    @patch("cli.Database")
    def test_citations_from_shows_backward(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_paper_title.return_value = "Attention Is All You Need"
        mock_db.get_citations.return_value = [
            MagicMock(source_id="2301.00001", target_id="2306.00001"),
            MagicMock(source_id="2301.00001", target_id="2305.00001"),
        ]
        mock_db_cls.return_value = mock_db

        args = make_args(subcmd="citations", citation_from="2301.00001", citation_to=None, format="text")
        result = _run_citations(args)

        captured = capsys.readouterr().out
        assert "BACKWARD CITATIONS" in captured
        assert "2306.00001" in captured
        assert result == 0

    @patch("cli.Database")
    def test_citations_to_shows_forward(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_paper_title.return_value = "Attention Is All You Need"
        mock_db.get_citations.return_value = [
            MagicMock(source_id="2307.00001", target_id="2301.00001"),
        ]
        mock_db_cls.return_value = mock_db

        args = make_args(subcmd="citations", citation_from=None, citation_to="2301.00001", format="text")
        result = _run_citations(args)

        captured = capsys.readouterr().out
        assert "FORWARD CITATIONS" in captured
        assert "2307.00001" in captured
        assert result == 0

    @patch("cli.Database")
    def test_citations_csv_format(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_paper_title.return_value = "Paper A"
        mock_db.get_citations.return_value = [
            MagicMock(source_id="2301.00001", target_id="2306.00001"),
        ]
        mock_db_cls.return_value = mock_db

        args = make_args(subcmd="citations", citation_from="2301.00001", citation_to=None, format="csv")
        result = _run_citations(args)

        captured = capsys.readouterr().out.replace("\r", "")
        lines = captured.strip().split("\n")
        assert "direction" in lines[0].lower()
        assert result == 0

    @patch("cli.Database")
    def test_citations_paper_not_found(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_paper_title.return_value = None
        mock_db_cls.return_value = mock_db

        args = make_args(subcmd="citations", citation_from="nonexistent", citation_to=None, format="text")
        result = _run_citations(args)

        captured = capsys.readouterr().out
        assert "not found" in captured.lower()
        assert result == 1

    @patch("cli.Database")
    def test_citations_no_from_or_to(self, mock_db_cls, capfd):
        mock_db = MagicMock()
        mock_db_cls.return_value = mock_db

        args = make_args(subcmd="citations", citation_from=None, citation_to=None, format="text")
        result = _run_citations(args)

        captured = capfd.readouterr().err
        assert "--from" in captured or "--to" in captured
        assert result == 1

    @patch("cli.Database")
    def test_citations_from_empty_result(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.get_paper_title.return_value = "Paper A"
        mock_db.get_citations.return_value = []
        mock_db_cls.return_value = mock_db

        args = make_args(subcmd="citations", citation_from="2301.00001", citation_to=None, format="text")
        result = _run_citations(args)

        captured = capsys.readouterr().out
        assert "No citations" in captured
        assert result == 0


# ─────────────────────────────────────────────────────────────────────────────
# _run_cite_stats tests
# ─────────────────────────────────────────────────────────────────────────────

class TestRunCiteStats:
    """Test _run_cite_stats."""

    @patch("cli.Database")
    def test_cite_stats_global(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_cursor = MagicMock()
        mock_cursor.fetchone.side_effect = [(100,), (20,), (15,), (200,), (25,)]
        mock_db.conn.cursor.return_value = mock_cursor
        mock_db_cls.return_value = mock_db

        args = make_args(subcmd="cite-stats", stats_paper=None, format="text")
        result = _run_cite_stats(args)

        captured = capsys.readouterr().out
        assert "Citation Statistics" in captured
        assert result == 0

    @patch("cli.Database")
    def test_cite_stats_paper_specific(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.paper_exists.return_value = True
        mock_db.get_paper_title.return_value = "Attention Is All You Need"
        mock_db.get_citation_count.return_value = {"backward": 150, "forward": 50000}
        mock_db_cls.return_value = mock_db

        args = make_args(subcmd="cite-stats", stats_paper="2301.00001", format="text")
        result = _run_cite_stats(args)

        captured = capsys.readouterr().out
        assert "Attention" in captured
        assert "50000" in captured or "50,000" in captured
        assert result == 0

    @patch("cli.Database")
    def test_cite_stats_paper_not_found(self, mock_db_cls, capfd):
        mock_db = MagicMock()
        mock_db.paper_exists.return_value = False
        mock_db_cls.return_value = mock_db

        args = make_args(subcmd="cite-stats", stats_paper="nonexistent", format="text")
        result = _run_cite_stats(args)

        captured = capfd.readouterr().err
        assert "not found" in captured.lower()
        assert result == 1

    @patch("cli.Database")
    def test_cite_stats_csv_format(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_cursor = MagicMock()
        mock_cursor.fetchone.side_effect = [(100,), (20,), (15,), (200,), (25,)]
        mock_db.conn.cursor.return_value = mock_cursor
        mock_db_cls.return_value = mock_db

        args = make_args(subcmd="cite-stats", stats_paper=None, format="csv")
        result = _run_cite_stats(args)

        captured = capsys.readouterr().out
        assert "metric,value" in captured
        assert "total_citations" in captured
        assert result == 0

    @patch("cli.Database")
    def test_cite_stats_no_args(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_cursor = MagicMock()
        mock_cursor.fetchone.side_effect = [(0,), (0,), (0,), (1,), (0,)]
        mock_db.conn.cursor.return_value = mock_cursor
        mock_db_cls.return_value = mock_db

        args = make_args(subcmd="cite-stats", stats_paper=None, format="text")
        result = _run_cite_stats(args)

        captured = capsys.readouterr().out
        assert result == 0


# ─────────────────────────────────────────────────────────────────────────────
# _run_cite_fetch tests
# ─────────────────────────────────────────────────────────────────────────────

class TestRunCiteFetch:
    """Test _run_cite_fetch."""

    @patch("cli._arxiv_doi_to_openalex")
    @patch("cli.Database")
    def test_cite_fetch_paper_not_found(self, mock_db_cls, mock_arxiv_to_oa, capfd):
        mock_db = MagicMock()
        mock_db.paper_exists.return_value = False
        mock_db_cls.return_value = mock_db

        args = make_args(subcmd="cite-fetch", paper_id="nonexistent")
        result = _run_cite_fetch(args)

        captured = capfd.readouterr().err
        assert "not found" in captured.lower()
        assert result == 1

    @patch("cli.Database")
    @patch("cli._arxiv_doi_to_openalex")
    def test_cite_fetch_no_papers_in_db(self, mock_arxiv_to_oa, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.paper_exists.return_value = False
        mock_db.list_papers.return_value = ([], 0)
        mock_db_cls.return_value = mock_db

        args = make_args(subcmd="cite-fetch", paper_id=None)
        result = _run_cite_fetch(args)

        captured = capsys.readouterr().out
        assert "No papers" in captured
        assert result == 0


# ─────────────────────────────────────────────────────────────────────────────
# _run_cite_import tests
# ─────────────────────────────────────────────────────────────────────────────

class TestRunCiteImport:
    """Test _run_cite_import."""

    @patch("cli.Database")
    def test_cite_import_no_json_input(self, mock_db_cls, capfd):
        mock_db = MagicMock()
        mock_db_cls.return_value = mock_db

        args = make_args(subcmd="cite-import", json_input=None)
        result = _run_cite_import(args)

        captured = capfd.readouterr().err
        assert "required" in captured.lower() or "json_input" in captured.lower()
        assert result == 1

    @patch("cli.Database")
    def test_cite_import_invalid_json(self, mock_db_cls, capfd):
        mock_db = MagicMock()
        mock_db_cls.return_value = mock_db

        args = make_args(subcmd="cite-import", json_input="not valid json{{{")
        result = _run_cite_import(args)

        captured = capfd.readouterr().err
        assert "invalid JSON" in captured.lower() or "Error" in captured
        assert result == 1

    @patch("cli.Database")
    def test_cite_import_valid_single_object(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.paper_exists.return_value = True
        mock_db.add_citations_batch.return_value = 1
        mock_db_cls.return_value = mock_db

        args = make_args(subcmd="cite-import", json_input='{"source": "2301.00001", "targets": ["2306.00001"]}')
        result = _run_cite_import(args)

        mock_db.add_citations_batch.assert_called_once()
        assert result == 0

    @patch("cli.Database")
    def test_cite_import_valid_list(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.paper_exists.return_value = True
        mock_db.add_citations_batch.return_value = 2
        mock_db_cls.return_value = mock_db

        args = make_args(subcmd="cite-import",
                         json_input='[{"source": "2301.00001", "targets": ["2306.00001", "2305.00001"]}]')
        result = _run_cite_import(args)

        # All targets share same source → one batch call
        assert mock_db.add_citations_batch.call_count >= 1
        assert result == 0

    @patch("cli.Database")
    def test_cite_import_empty_targets(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db_cls.return_value = mock_db

        args = make_args(subcmd="cite-import", json_input='{"source": "2301.00001", "targets": []}')
        result = _run_cite_import(args)

        captured = capsys.readouterr().out
        assert result == 1

    @patch("cli.Database")
    def test_cite_import_skip_missing_dry_run(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.paper_exists.side_effect = lambda pid: pid == "2301.00001"
        mock_db_cls.return_value = mock_db

        args = make_args(subcmd="cite-import",
                         json_input='{"source": "2301.00001", "targets": ["nonexistent"]}',
                         skip_missing=True, dry_run=True)
        result = _run_cite_import(args)

        captured = capsys.readouterr().out
        assert "dry" in captured.lower() or "skip" in captured.lower()
        assert result == 0

    @patch("cli.Database")
    def test_cite_import_dedup_flag_extract_mode(self, mock_db_cls, capsys):
        """--dedup in --extract mode calls upsert_citations and reports duplicate count."""
        mock_db = MagicMock()
        mock_paper = MagicMock()
        mock_paper.plain_text = "References\n1. Attention Is All You Need. arXiv: 1706.03762\n"
        mock_db.get_paper.return_value = mock_paper
        mock_db.paper_exists.return_value = True
        mock_db.upsert_citations.return_value = (0, 1)  # 0 new, 1 duplicate
        mock_db_cls.return_value = mock_db

        args = make_args(subcmd="cite-import", extract=True, extract_paper="2301.00001", dedup=True)
        result = _run_cite_import(args)

        mock_db.upsert_citations.assert_called_once()
        captured = capsys.readouterr().out
        assert "0 new edge" in captured
        assert "1 duplicate" in captured
        assert result == 0

    @patch("cli.Database")
    def test_cite_import_extract_mode_with_pmid_isbn(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_paper = MagicMock()
        mock_paper.plain_text = (
            "References\n"
            "1. PMID: 12345678\n"
            "2. ISBN: 978-0-13-468599-1\n"
            "3. Attention Is All You Need. arXiv: 1706.03762\n"
        )
        mock_db.get_paper.return_value = mock_paper
        mock_db.paper_exists.side_effect = lambda pid: pid == "arXiv:1706.03762"
        mock_db.add_citations_batch.return_value = 1
        mock_db_cls.return_value = mock_db

        args = make_args(subcmd="cite-import", extract=True, extract_paper="2301.00001")
        result = _run_cite_import(args)

        captured = capsys.readouterr().out
        assert "PMID" in captured
        assert "ISBN" in captured
        assert "12345678" in captured
        assert result == 0


# ─────────────────────────────────────────────────────────────────────────────
# _run_export tests
# ─────────────────────────────────────────────────────────────────────────────

class TestRunExport:
    """Test _run_export."""

    @patch("cli.Database")
    def test_export_writes_jsonl(self, mock_db_cls, capsys, tmp_path):
        mock_db = MagicMock()
        mock_db.export_papers.return_value = (
            ["id", "title", "authors"],
            [{"id": "2301.00001", "title": "Paper One", "authors": "Vaswani et al."}],
        )
        mock_db_cls.return_value = mock_db

        out_file = tmp_path / "export.jsonl"
        args = make_args(subcmd="export", out=str(out_file), format="json")
        result = _run_export(args)

        assert out_file.exists()
        content = out_file.read_text(encoding="utf-8")
        assert "2301.00001" in content
        assert "Paper One" in content
        assert result == 0

    @patch("cli.Database")
    def test_export_writes_to_stdout(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.export_papers.return_value = (
            ["id", "title"],
            [{"id": "2301.00001", "title": "Paper One"}],
        )
        mock_db_cls.return_value = mock_db

        args = make_args(subcmd="export", out=None, format="json")
        result = _run_export(args)

        captured = capsys.readouterr().out
        assert "2301.00001" in captured
        assert result == 0

    @patch("cli.Database")
    def test_export_csv_format(self, mock_db_cls, capsys, tmp_path):
        mock_db = MagicMock()
        mock_db.export_papers.return_value = (
            ["id", "title"],
            [{"id": "2301.00001", "title": "Paper One"}],
        )
        mock_db_cls.return_value = mock_db

        out_file = tmp_path / "export.csv"
        args = make_args(subcmd="export", out=str(out_file), format="csv")
        result = _run_export(args)

        assert out_file.exists()
        content = out_file.read_text(encoding="utf-8")
        assert "id" in content
        assert "2301.00001" in content
        assert result == 0

    @patch("cli.Database")
    def test_export_empty_db(self, mock_db_cls, capsys, tmp_path):
        mock_db = MagicMock()
        mock_db.export_papers.return_value = (["id", "title"], [])
        mock_db_cls.return_value = mock_db

        out_file = tmp_path / "empty.jsonl"
        args = make_args(subcmd="export", out=str(out_file), format="json")
        result = _run_export(args)

        assert out_file.exists()
        assert result == 0


# ─────────────────────────────────────────────────────────────────────────────
# _run_import tests
# ─────────────────────────────────────────────────────────────────────────────

class TestRunImport:
    """Test _run_import."""

    @patch("cli._main_legacy")
    @patch("cli.Database")
    def test_import_no_ids_no_file(self, mock_db_cls, mock_legacy, capfd):
        mock_db = MagicMock()
        mock_db_cls.return_value = mock_db

        args = make_args(subcmd="import", ids=[], file=None)
        result = _run_import(args)

        captured = capfd.readouterr().err
        assert "paper ID" in captured.lower() or "--file" in captured
        assert result == 1

    @patch("cli.Database")
    def test_import_json_flow_with_ids(self, mock_db_cls, capfd):
        """When source='import', _run_import uses JSON import path with ids."""
        mock_db = MagicMock()
        mock_db.get_paper.return_value = None  # not in DB → will add
        mock_db.upsert_paper.return_value = None
        mock_db_cls.return_value = mock_db

        args = make_args(subcmd="import", ids=["2301.00001"], source="import", file=None)
        result = _run_import(args)

        assert result == 0
        assert mock_db.upsert_paper.call_count == 1
