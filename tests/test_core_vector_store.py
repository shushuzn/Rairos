"""Tests for core/vector_store.py — SearchResult dataclass and helpers."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

from core.vector_store import SearchResult, is_zilliz_configured


class TestSearchResult:
    def test_creation(self):
        r = SearchResult(
            id="chunk-123",
            score=0.95,
            content="def foo(): pass",
            file="test.py",
            line=10,
        )
        assert r.id == "chunk-123"
        assert r.score == 0.95
        assert r.content == "def foo(): pass"
        assert r.file == "test.py"
        assert r.line == 10

    def test_fields_are_typed(self):
        r = SearchResult(id="x", score=0.5, content="y", file="f", line=1)
        assert isinstance(r.id, str)
        assert isinstance(r.score, float)
        assert isinstance(r.content, str)
        assert isinstance(r.file, str)
        assert isinstance(r.line, int)

    def test_score_range(self):
        r = SearchResult(id="x", score=1.0, content="y", file="f", line=1)
        assert r.score == 1.0
        r2 = SearchResult(id="x", score=0.0, content="y", file="f", line=1)
        assert r2.score == 0.0

    def test_line_number(self):
        r = SearchResult(id="x", score=0.5, content="y", file="f", line=42)
        assert r.line == 42


class TestIsZillizConfigured:
    def test_returns_bool(self):
        result = is_zilliz_configured()
        assert isinstance(result, bool)
