"""Tests for parsers/arxiv.py — arXiv API metadata fetching."""

from __future__ import annotations

import pytest
from unittest.mock import MagicMock, patch


class TestLazyImports:
    """Test that lazy import pattern works correctly."""

    def test_lazy_feedparser_loads_once(self):
        import parsers.arxiv as arxiv_parser

        arxiv_parser._feedparser = None
        mock_fp = MagicMock()
        with patch.dict("sys.modules", {"feedparser": mock_fp}):
            fp = arxiv_parser._lazy_feedparser()
            assert fp is mock_fp
            assert arxiv_parser._lazy_feedparser() is mock_fp

    def test_lazy_requests_loads_once(self):
        import parsers.arxiv as arxiv_parser

        arxiv_parser._requests = None
        mock_req = MagicMock()
        with patch.dict("sys.modules", {"requests": mock_req}):
            req = arxiv_parser._lazy_requests()
            assert req is mock_req


class TestSession:
    """Test HTTP session singleton behavior."""

    def test_session_returns_singleton(self):
        import parsers.arxiv as arxiv_parser

        arxiv_parser._http_session = None
        with patch.object(arxiv_parser, "_lazy_requests") as mock_lr:
            mock_session = MagicMock()
            mock_lr.return_value.Session.return_value = mock_session

            s1 = arxiv_parser._get_session()
            s2 = arxiv_parser._get_session()

            assert s1 is s2
            assert mock_lr.return_value.Session.call_count == 1


class TestRateLimiting:
    """Test rate limiting behavior."""

    def test_rate_limiter_allows_after_3_seconds(self):
        import parsers.arxiv as arxiv_parser

        arxiv_parser._last_arxiv_request_time = 0.0
        with patch("time.monotonic", return_value=10.0):
            with patch("time.sleep") as mock_sleep:
                arxiv_parser._rate_limit()
                mock_sleep.assert_not_called()

    def test_rate_limiter_sleeps_when_too_soon(self):
        import parsers.arxiv as arxiv_parser

        arxiv_parser._last_arxiv_request_time = 10.0
        with patch("time.monotonic", return_value=10.5):
            with patch("time.sleep") as mock_sleep:
                arxiv_parser._rate_limit()
                mock_sleep.assert_called_once()


class TestArxivIdFromInput:
    """Test arxiv_id_from_input normalization."""

    def test_strips_abs_url_version(self):
        import parsers.arxiv as arxiv_parser

        assert arxiv_parser.arxiv_id_from_input("http://arxiv.org/abs/2301.00001v3") == "2301.00001"

    def test_strips_pdf_url_version(self):
        import parsers.arxiv as arxiv_parser

        assert arxiv_parser.arxiv_id_from_input("http://arxiv.org/pdf/2301.00001v2") == "2301.00001"

    def test_raw_id_with_version_strips_suffix(self):
        import parsers.arxiv as arxiv_parser

        assert arxiv_parser.arxiv_id_from_input("2301.00001v2") == "2301.00001"

    def test_raw_id_without_version_passthrough(self):
        import parsers.arxiv as arxiv_parser

        assert arxiv_parser.arxiv_id_from_input("2301.00001") == "2301.00001"

    def test_doi_arxiv_form(self):
        import parsers.arxiv as arxiv_parser

        assert (
            arxiv_parser.arxiv_id_from_input("http://doi.org/10.48550/arXiv.2601.00155")
            == "2601.00155"
        )

    def test_arxiv_capital_prefix(self):
        import parsers.arxiv as arxiv_parser

        assert arxiv_parser.arxiv_id_from_input("arXiv.2601.00155") == "2601.00155"

    def test_empty_string_returns_empty(self):
        import parsers.arxiv as arxiv_parser

        assert arxiv_parser.arxiv_id_from_input("") == ""

    def test_id_without_v_prefix(self):
        import parsers.arxiv as arxiv_parser

        assert arxiv_parser.arxiv_id_from_input("2601.00155") == "2601.00155"
