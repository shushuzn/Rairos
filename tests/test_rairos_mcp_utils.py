"""Tests for rairos_mcp utility functions."""

from rairos_mcp import error_response, success_response


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
