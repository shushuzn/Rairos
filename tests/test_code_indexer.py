"""Tests for core/code_indexer.py."""
import pytest
from core.code_indexer import CodeChunk, _get_zilliz_config


class TestCodeChunk:
    def test_fields(self):
        chunk = CodeChunk("c1", "test.py", 10, "def foo(): pass", [0.1, 0.2, 0.3])
        assert chunk.file == "test.py"
        assert chunk.line == 10

    def test_embedding_type(self):
        chunk = CodeChunk("c2", "a.py", 1, "x", [0.1, 0.2])
        assert isinstance(chunk.embedding, list)


class TestZillizConfig:
    def test_returns_tuple_of_nones(self):
        host, token = _get_zilliz_config()
        assert host is None or isinstance(host, str)
        assert token is None or isinstance(token, str)
