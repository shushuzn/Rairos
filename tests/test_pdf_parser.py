"""Tests for pdf/parser.py — PDFParser, ParsedPaper, cache."""
from __future__ import annotations

import json
import sqlite3
from pathlib import Path

import pytest

from pdf.parser import (
    LaTeXBlock,
    ParsedPaper,
    PDFParser,
    TableData,
    FigureData,
)


@pytest.fixture
def tmp_db(tmp_path):
    """Create an in-memory database for parser cache tests."""
    db_path = tmp_path / "research.db"
    conn = sqlite3.connect(str(db_path))
    conn.execute(
        "CREATE TABLE IF NOT EXISTS papers ("
        "  id TEXT PRIMARY KEY,"
        "  source TEXT NOT NULL,"
        "  title TEXT DEFAULT '',"
        "  authors TEXT DEFAULT '[]',"
        "  abstract TEXT DEFAULT '',"
        "  published TEXT DEFAULT '',"
        "  updated TEXT DEFAULT '',"
        "  abs_url TEXT DEFAULT '',"
        "  pdf_url TEXT DEFAULT '',"
        "  primary_category TEXT DEFAULT '',"
        "  journal TEXT DEFAULT '',"
        "  volume TEXT DEFAULT '',"
        "  issue TEXT DEFAULT '',"
        "  page TEXT DEFAULT '',"
        "  doi TEXT DEFAULT '',"
        "  categories TEXT DEFAULT '',"
        "  reference_count INTEGER DEFAULT 0,"
        "  added_at TEXT NOT NULL,"
        "  updated_at TEXT NOT NULL,"
        "  pdf_path TEXT DEFAULT '',"
        "  pdf_hash TEXT DEFAULT '',"
        "  parse_status TEXT DEFAULT 'pending',"
        "  parse_error TEXT DEFAULT '',"
        "  parse_version INTEGER DEFAULT 0,"
        "  plain_text TEXT DEFAULT '',"
        "  latex_blocks TEXT DEFAULT '[]',"
        "  table_count INTEGER DEFAULT 0,"
        "  figure_count INTEGER DEFAULT 0,"
        "  word_count INTEGER DEFAULT 0,"
        "  page_count INTEGER DEFAULT 0,"
        "  pnote_path TEXT DEFAULT '',"
        "  cnote_path TEXT DEFAULT '',"
        "  mnote_path TEXT DEFAULT '',"
        "  embed_vector BLOB DEFAULT NULL"
        ")"
    )
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute("PRAGMA foreign_keys=ON")
    conn.commit()

    # Wrap in a minimal Database-like object
    class _FakeDB:
        def get_paper(self, paper_id):
            cur = conn.cursor()
            cur.execute("SELECT * FROM papers WHERE id = ?", (paper_id,))
            row = cur.fetchone()
            if row is None:
                return None
            columns = [desc[0] for desc in cur.description]
            return dict(zip(columns, row))

        def update_parse_status(
            self,
            paper_id,
            status,
            error="",
            plain_text="",
            latex_blocks=None,
            table_count=0,
            figure_count=0,
            word_count=0,
            page_count=0,
        ):
            latex_json = json.dumps(latex_blocks or []) if not isinstance(latex_blocks, str) else latex_blocks
            cur = conn.cursor()
            cur.execute(
                "SELECT parse_version FROM papers WHERE id = ?",
                (paper_id,),
            )
            row = cur.fetchone()
            version = (row["parse_version"] if row else 0) + 1
            cur.execute(
                "UPDATE papers SET parse_status=?, parse_error=?, plain_text=?, "
                "latex_blocks=?, table_count=?, figure_count=?, word_count=?, "
                "page_count=?, parse_version=? WHERE id=?",
                (
                    status, error, plain_text, latex_json,
                    table_count, figure_count, word_count, page_count,
                    version, paper_id,
                ),
            )
            conn.commit()

    return _FakeDB()


@pytest.fixture
def sample_pdf(tmp_path):
    """Create a minimal valid PDF for testing."""
    pdf_path = tmp_path / "sample.pdf"
    # Minimal PDF structure
    pdf_path.write_bytes(
        b"%PDF-1.4\n"
        b"1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n"
        b"2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n"
        b"3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]"
        b"/Contents 4 0 R/Resources<</Font<</F1 5 0 R>>>>>>endobj\n"
        b"4 0 obj<</Length 44>>stream\n"
        b"BT /F1 12 Tf 100 700 Td (Hello World) Tj ET\n"
        b"endstream endobj\n"
        b"5 0 obj<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>endobj\n"
        b"xref\n0 6\n0000000000 65535 f\n0000000009 00000 n\n0000000058 00000 n\n0000000115 00000 n\n0000000270 00000 n\n0000000350 00000 n\ntrailer<</Size 6/Root 1 0 R>>\nstartxref\n427\n%%EOF"
    )
    return pdf_path


class TestParsedPaperCache:
    """Test ParsedPaper.to_cache_dict / from_cache_dict round-trip."""

    def test_roundtrip(self):
        paper = ParsedPaper(
            paper_id="2301.00001",
            text="Some extracted text",
            latex_blocks=[
                LaTeXBlock(source=r"$\alpha + \beta$", is_display=False, page=0),
                LaTeXBlock(source=r"$$\int_0^1 x^2 dx$$", is_display=True, page=1),
            ],
            tables=[
                TableData(
                    headers=["A", "B"],
                    rows=[["1", "2"], ["3", "4"]],
                    page=2,
                    bbox=(1, 2, 3, 4),
                    caption="Table 1",
                ),
            ],
            figures=[
                FigureData(caption="Figure 1", page=3, bbox=(0, 0, 100, 100), alt_text="img1"),
            ],
            page_count=5,
            word_count=42,
            parse_version=1,
            pdf_hash="abc123",
            title="Test Paper",
            authors=["Alice", "Bob"],
            abstract="Abstract text",
            published="2023-01-01",
            warnings=["warning 1"],
            errors=[],
        )

        cached = paper.to_cache_dict()
        restored = ParsedPaper.from_cache_dict(cached)

        assert restored.paper_id == "2301.00001"
        assert restored.text == "Some extracted text"
        assert len(restored.latex_blocks) == 2
        assert restored.latex_blocks[0].source == r"$\alpha + \beta$"
        assert restored.latex_blocks[0].is_display is False
        assert restored.latex_blocks[1].is_display is True
        assert len(restored.tables) == 1
        assert restored.tables[0].headers == ["A", "B"]
        assert len(restored.figures) == 1
        assert restored.figures[0].caption == "Figure 1"
        assert restored.page_count == 5
        assert restored.word_count == 42
        assert restored.parse_version == 1
        assert restored.pdf_hash == "abc123"
        assert restored.authors == ["Alice", "Bob"]
        assert restored.warnings == ["warning 1"]

    def test_from_cache_dict_handles_missing_fields(self):
        minimal = {"paper_id": "2301.00001", "text": "x"}
        paper = ParsedPaper.from_cache_dict(minimal)
        assert paper.paper_id == "2301.00001"
        assert paper.text == "x"
        assert paper.latex_blocks == []
        assert paper.tables == []
        assert paper.figures == []
        assert paper.page_count == 0
        assert paper.word_count == 0
        assert paper.parse_version == 0
        assert paper.warnings == []
        assert paper.errors == []


class TestPDFParser:
    """Test PDFParser.parse() with real PDF."""

    def test_parse_returns_parsed_paper(self, sample_pdf):
        parser = PDFParser()
        result = parser.parse(sample_pdf, paper_id="2301.00001", use_cache=False)

        assert isinstance(result, ParsedPaper)
        assert result.paper_id == "2301.00001"
        assert result.pdf_hash != ""
        assert result.parse_version == 1
        assert result.page_count > 0

    def test_parse_file_not_found(self, tmp_path):
        parser = PDFParser()
        with pytest.raises(Exception):  # PDFParseError
            parser.parse(tmp_path / "nonexistent.pdf", paper_id="2301.00001")

    def test_hash_file(self, sample_pdf):
        parser = PDFParser()
        h1 = parser._hash_file(sample_pdf)
        h2 = parser._hash_file(sample_pdf)
        assert h1 == h2
        assert len(h1) == 64  # SHA256 hex

    def test_hash_changes_on_content(self, tmp_path):
        parser = PDFParser()
        p1 = tmp_path / "a.pdf"
        p1.write_bytes(b"content A")
        p2 = tmp_path / "b.pdf"
        p2.write_bytes(b"content B")
        assert parser._hash_file(p1) != parser._hash_file(p2)


class TestPDFParserCache:
    """Test PDFParser file-based cache (no DB)."""

    def test_cache_hit_returns_same_result(self, sample_pdf, tmp_path, monkeypatch):
        cache_dir = tmp_path / "parsed"
        parser = PDFParser(cache_dir=cache_dir)

        # Patch db to None to force file cache
        parser.db = None

        result1 = parser.parse(sample_pdf, paper_id="2301.00001", use_cache=True)
        assert result1.parse_version == 1

        # Second call should hit cache
        result2 = parser.parse(sample_pdf, paper_id="2301.00001", use_cache=True)
        # Cache is file-based; result should be equivalent
        assert result2.paper_id == result1.paper_id
        assert result2.pdf_hash == result1.pdf_hash

    def test_cache_miss_when_pdf_changes(self, sample_pdf, tmp_path):
        cache_dir = tmp_path / "parsed"
        parser = PDFParser(cache_dir=cache_dir)
        parser.db = None

        r1 = parser.parse(sample_pdf, paper_id="2301.00001", use_cache=True)
        hash1 = r1.pdf_hash

        # Modify PDF content (changes hash)
        pdf_new = tmp_path / "modified.pdf"
        pdf_new.write_bytes(sample_pdf.read_bytes() + b"\n% modified")

        r2 = parser.parse(pdf_new, paper_id="2301.00001", use_cache=True)
        assert r2.pdf_hash != hash1
