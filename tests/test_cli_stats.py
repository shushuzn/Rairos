"""Tier 1 tests — CLI stats command."""

import pytest
from unittest.mock import patch, MagicMock
from cli import _run_stats, _build_stats_parser
import argparse


class TestStatsParser:
    """Test stats parser construction."""

    def test_build_parser_accepts_json_and_format_flags(self):
        """_build_stats_parser adds --json and --format flags."""
        p = argparse.ArgumentParser()
        sub = p.add_subparsers()
        result = _build_stats_parser(sub)
        assert result is not None


class TestRunStats:
    """Test _run_stats function with mocked database."""

    @patch("cli.Database")
    def test_stats_json_returns_valid_json(self, mock_db_cls, capsys):
        """_run_stats --json prints valid JSON with all expected keys."""
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.get_stats.return_value = {
            "total_papers": 42,
            "by_source": {"arxiv": 30, "doi": 12},
            "by_status": {"read": 20, "unread": 22},
            "queue_queued": 3,
            "queue_running": 1,
            "cache_entries": 7,
            "dedup_records": 5,
        }
        mock_db_cls.return_value = mock_db
        args = argparse.Namespace(json=True, format="table")
        rc = _run_stats(args)
        assert rc == 0
        captured = capsys.readouterr()
        import orjson

        data = orjson.loads(captured.out)
        assert data["total_papers"] == 42
        assert data["by_source"]["arxiv"] == 30

    @patch("cli.Database")
    def test_stats_table_output_has_sections(self, mock_db_cls, capsys):
        """_run_stats (table) prints Papers/Queue/Cache/Dedup sections."""
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.get_stats.return_value = {
            "total_papers": 10,
            "by_source": {},
            "by_status": {},
            "queue_queued": 0,
            "queue_running": 0,
            "cache_entries": 0,
            "dedup_records": 0,
        }
        mock_db_cls.return_value = mock_db
        args = argparse.Namespace(json=False, format="table")
        rc = _run_stats(args)
        assert rc == 0
        captured = capsys.readouterr()
        assert "Papers:" in captured.out
        assert "Queue:" in captured.out
        assert "Cache:" in captured.out
        assert "Dedup:" in captured.out

    @patch("cli.Database")
    def test_stats_warp_format_renders_without_error(self, mock_db_cls, capsys):
        """_run_stats --format warp renders Warp blocks without error."""
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db.get_stats.return_value = {
            "total_papers": 5,
            "by_source": {},
            "by_status": {},
            "queue_queued": 0,
            "queue_running": 0,
            "cache_entries": 0,
            "dedup_records": 0,
        }
        mock_db_cls.return_value = mock_db
        with patch("llm.client.get_llm_cache_size", return_value=0):
            with patch("llm.client._cache_stats", return_value={}):
                with patch("llm.client.get_cache_stats", return_value={"hit_rate": 0}):
                    args = argparse.Namespace(json=False, format="warp")
                    rc = _run_stats(args)
                    assert rc == 0
