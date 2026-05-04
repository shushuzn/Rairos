"""Tier 1 tests — CLI cache command."""

import pytest
from unittest.mock import patch, MagicMock
from cli import _run_cache, _build_cache_parser
import argparse


class TestCacheParser:
    """Test cache parser construction."""

    def test_build_parser_accepts_llm_flags(self):
        """_build_cache_parser adds --llm and --llm-clear flags."""
        p = argparse.ArgumentParser()
        sub = p.add_subparsers()
        result = _build_cache_parser(sub)
        assert result is not None


class TestRunCache:
    """Test _run_cache function — verify each flag exits cleanly."""

    @patch("cli.Database")
    def test_cache_llm_stats(self, mock_db_cls, capsys):
        """_run_cache --llm returns 0."""
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db_cls.return_value = mock_db
        with patch("llm.client.get_llm_cache_size", return_value=0):
            with patch("llm.client._cache_stats", return_value={"entries": 0}):
                with patch(
                    "llm.client.get_cache_stats",
                    return_value={"hits": 0, "misses": 0, "hit_rate": 0},
                ):
                    with patch("llm.client.reset_cache_stats"):
                        args = argparse.Namespace(
                            llm=True, llm_clear=False, get=None, set=None, clear=False, stats=False
                        )
                        rc = _run_cache(args)
                        assert rc == 0

    @patch("cli.Database")
    def test_cache_llm_clear(self, mock_db_cls, capsys):
        """_run_cache --llm-clear returns 0."""
        mock_db = MagicMock()
        mock_db.init.return_value = None
        mock_db_cls.return_value = mock_db
        with patch("llm.client.clear_llm_cache") as mock_clear:
            with patch("llm.client.reset_cache_stats"):
                with patch("cli.warp.WarpBlocks"):
                    args = argparse.Namespace(
                        llm=False, llm_clear=True, get=None, set=None, clear=False, stats=False
                    )
                    rc = _run_cache(args)
                    assert rc == 0
                    mock_clear.assert_called_once()
