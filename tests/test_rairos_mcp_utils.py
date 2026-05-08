"""Tests for rairos_mcp utility functions."""

import pytest
from rairos_mcp import error_response, success_response, _ensure_data_dir


class TestRairosMcpUtils:
    def test_error_response(self):
        r = error_response("TEST_ERR", "test message")
        assert r["error"]["code"] == "TEST_ERR"
        assert r["error"]["message"] == "test message"

    def test_success_response(self):
        r = success_response({"key": "value"})
        assert r["result"]["key"] == "value"

    def test_success_response_none(self):
        r = success_response(None)
        assert r["result"] is None

    def test_ensure_data_dir(self):
        # Should not raise
        _ensure_data_dir()
