"""Tests for core/code_indexer.py — CodeChunk class."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

from core.code_indexer import CodeChunk


class TestCodeChunk:
    def test_creation_without_embedding(self):
        chunk = CodeChunk(
            chunk_id="chunk-1",
            file="test.py",
            line=10,
            content="def foo(): pass",
        )
        assert chunk.id == "chunk-1"
        assert chunk.file == "test.py"
        assert chunk.line == 10
        assert chunk.content == "def foo(): pass"
        assert chunk.embedding is None

    def test_creation_with_embedding(self):
        emb = [0.1] * 768
        chunk = CodeChunk(
            chunk_id="chunk-2",
            file="main.py",
            line=42,
            content="x = 1",
            embedding=emb,
        )
        assert chunk.embedding == emb
        assert len(chunk.embedding) == 768

    def test_fields_are_correct_types(self):
        chunk = CodeChunk("id", "f.py", 1, "code")
        assert isinstance(chunk.id, str)
        assert isinstance(chunk.file, str)
        assert isinstance(chunk.line, int)
        assert isinstance(chunk.content, str)
        # embedding is None or List[float]
        assert chunk.embedding is None or isinstance(chunk.embedding, list)
