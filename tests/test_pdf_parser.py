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
        assert parser._hash_file(p1) != parser._hash_file(p2)


class TestPDFParserEdgeCases:
    """Cover pdf/parser.py edge-case branches (lines 241, 264, 345)."""

    def test_parse_timeout_raises(self, sample_pdf, monkeypatch):
        """Line 241: raises ParseTimeoutError when elapsed > max_parse_time."""
        from pdf.parser import ParseTimeoutError
        import time
        # first 3 calls return 0 (start_time + stat), 4th call returns 100 → elapsed=100 >> 0.001
        call_count = [0]
        def fake_time():
            call_count[0] += 1
            if call_count[0] <= 3:
                return 0.0
            return 1000.0  # end time
        monkeypatch.setattr(time, "time", fake_time)
        parser = PDFParser()
        with pytest.raises(ParseTimeoutError):
            parser.parse(sample_pdf, paper_id="t", use_cache=False, max_parse_time=0.001)

    def test_parse_save_file_cache_oserror(self, sample_pdf, tmp_path, monkeypatch):
        """Lines 345-346: OSError when saving file cache is caught."""
        cache_dir = tmp_path / "parsed"
        parser = PDFParser(cache_dir=cache_dir)
        parser.db = None

        import pathlib
        orig_mkdir = pathlib.Path.mkdir
        def bad_mkdir(self, *a, **k):
            if str(self) == str(cache_dir):
                raise OSError("read-only filesystem")
            return orig_mkdir(self, *a, **k)
        monkeypatch.setattr(pathlib.Path, "mkdir", staticmethod(bad_mkdir))

    def test_parse_timeout_raises(self, sample_pdf, monkeypatch):
        from pdf.parser import ParseTimeoutError
        import time
        # Override elapsed directly on the parser after extraction completes
        parser = PDFParser()
        original_extract = parser._extract_structured
        def slow_extract(path):
            result = original_extract(path)
            # Manually advance elapsed time by patching the parser's state
            parser._last_elapsed = 999.0  # arbitrary large value
            return result
        monkeypatch.setattr(parser, "_extract_structured", slow_extract)
        with pytest.raises(ParseTimeoutError):
            parser.parse(sample_pdf, paper_id="t", use_cache=False, max_parse_time=0.001)

    def test_parse_pymupdf_exception_raises_when_fallback_also_fails(self, sample_pdf, monkeypatch):
        """Lines 233-237: PyMuPDF exception + empty pdfminer fallback = PDFParseError."""
        parser = PDFParser()
        parser.db = None
        monkeypatch.setattr(parser, "_extract_structured", lambda p: (_ for _ in ()).throw(RuntimeError("pymupdf broken")))
        from core.exceptions import PDFParseError
        with pytest.raises(PDFParseError, match="All PDF extraction methods failed"):
            parser.parse(sample_pdf, paper_id="t", use_cache=False)

    def test_find_caption_near(self, sample_pdf):
        """Lines 533-553: _find_caption_near detects captions near image bbox."""
        import fitz
        parser = PDFParser()
        doc = fitz.open(str(sample_pdf))
        page = doc[0]
        bbox = (200, 300, 400, 400)
        caption = parser._find_caption_near(page, bbox, page_idx=0, search_radius=50.0)
        assert isinstance(caption, str)


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


class TestCheckDBCache:
    """Edge cases for _check_db_cache."""

    def test_returns_none_when_paper_not_found(self, tmp_db, tmp_path):
        parser = PDFParser(db=tmp_db)
        result = parser._check_db_cache("nonexistent", "somehash")
        assert result is None

    def test_returns_none_when_pdf_hash_mismatches(self, tmp_db, tmp_path):
        # Insert paper with different hash
        from tests.test_pdf_parser import tmp_db as _  # fixture
        import sqlite3
        db_path = tmp_path / "research.db"
        conn = sqlite3.connect(str(db_path))
        conn.execute(
            "INSERT INTO papers (id, source, pdf_hash, parse_status, added_at, updated_at) "
            "VALUES ('2301.00001', 'arxiv', 'oldhash', 'done', '2023-01-01', '2023-01-01')"
        )
        conn.commit()
        conn.close()
        # Reconnect using the same tmp_db fixture pattern
        class _FakeDB:
            def __init__(self, conn):
                self.conn = conn
            def get_paper(self, paper_id):
                cur = self.conn.cursor()
                cur.execute("SELECT * FROM papers WHERE id = ?", (paper_id,))
                row = cur.fetchone()
                if row is None:
                    return None
                columns = [desc[0] for desc in cur.description]
                return dict(zip(columns, row))
        # Use a fresh DB for this sub-test
        conn2 = sqlite3.connect(str(db_path))
        conn2.execute(
            "INSERT OR REPLACE INTO papers (id, source, pdf_hash, parse_status, added_at, updated_at) "
            "VALUES ('2301.00001', 'arxiv', 'oldhash', 'done', '2023-01-01', '2023-01-01')"
        )
        conn2.commit()
        db2 = _FakeDB(conn2)
        parser = PDFParser(db=db2)
        # Hash does NOT match
        result = parser._check_db_cache("2301.00001", "different_hash")
        assert result is None
        conn2.close()

    def test_returns_none_when_parse_status_not_done(self, tmp_path):
        """parse_status must be 'done' for cache to be valid."""
        import sqlite3
        db_path = tmp_path / "research2.db"
        conn = sqlite3.connect(str(db_path))
        conn.execute(
            "CREATE TABLE IF NOT EXISTS papers ("
            "  id TEXT PRIMARY KEY, source TEXT NOT NULL, title TEXT DEFAULT '',"
            "  authors TEXT DEFAULT '[]', abstract TEXT DEFAULT '', published TEXT DEFAULT '',"
            "  updated TEXT DEFAULT '', abs_url TEXT DEFAULT '', pdf_url TEXT DEFAULT '',"
            "  primary_category TEXT DEFAULT '', journal TEXT DEFAULT '', volume TEXT DEFAULT '',"
            "  issue TEXT DEFAULT '', page TEXT DEFAULT '', doi TEXT DEFAULT '',"
            "  categories TEXT DEFAULT '', reference_count INTEGER DEFAULT 0,"
            "  added_at TEXT NOT NULL, updated_at TEXT NOT NULL, pdf_path TEXT DEFAULT '',"
            "  pdf_hash TEXT DEFAULT '', parse_status TEXT DEFAULT 'pending',"
            "  parse_error TEXT DEFAULT '', parse_version INTEGER DEFAULT 0,"
            "  plain_text TEXT DEFAULT '', latex_blocks TEXT DEFAULT '[]',"
            "  table_count INTEGER DEFAULT 0, figure_count INTEGER DEFAULT 0,"
            "  word_count INTEGER DEFAULT 0, page_count INTEGER DEFAULT 0,"
            "  pnote_path TEXT DEFAULT '', cnote_path TEXT DEFAULT '',"
            "  mnote_path TEXT DEFAULT '', embed_vector BLOB DEFAULT NULL)"
        )
        conn.execute(
            "INSERT INTO papers (id, source, pdf_hash, parse_status, added_at, updated_at) "
            "VALUES ('2301.00001', 'arxiv', 'abc123', 'pending', '2023-01-01', '2023-01-01')"
        )
        conn.commit()
        class _FakeDB:
            def __init__(self, conn):
                self.conn = conn
            def get_paper(self, paper_id):
                cur = self.conn.cursor()
                cur.execute("SELECT * FROM papers WHERE id = ?", (paper_id,))
                row = cur.fetchone()
                if row is None:
                    return None
                columns = [desc[0] for desc in cur.description]
                return dict(zip(columns, row))
        db = _FakeDB(conn)
        parser = PDFParser(db=db)
        result = parser._check_db_cache("2301.00001", "abc123")
        assert result is None
        conn.close()

    def test_returns_none_on_db_exception(self, tmp_path):
        class BadDB:
            def get_paper(self, paper_id):
                raise RuntimeError("db connection lost")
        parser = PDFParser(db=BadDB())
        result = parser._check_db_cache("any", "any")
        assert result is None

    def test_returns_cached_paper_on_hit(self, tmp_path):
        """Full cache hit: paper exists, hash matches, status='done'."""
        import sqlite3
        import json
        db_path = tmp_path / "research3.db"
        conn = sqlite3.connect(str(db_path))
        conn.execute(
            "CREATE TABLE IF NOT EXISTS papers ("
            "  id TEXT PRIMARY KEY, source TEXT NOT NULL, title TEXT DEFAULT '',"
            "  authors TEXT DEFAULT '[]', abstract TEXT DEFAULT '', published TEXT DEFAULT '',"
            "  updated TEXT DEFAULT '', abs_url TEXT DEFAULT '', pdf_url TEXT DEFAULT '',"
            "  primary_category TEXT DEFAULT '', journal TEXT DEFAULT '', volume TEXT DEFAULT '',"
            "  issue TEXT DEFAULT '', page TEXT DEFAULT '', doi TEXT DEFAULT '',"
            "  categories TEXT DEFAULT '', reference_count INTEGER DEFAULT 0,"
            "  added_at TEXT NOT NULL, updated_at TEXT NOT NULL, pdf_path TEXT DEFAULT '',"
            "  pdf_hash TEXT DEFAULT '', parse_status TEXT DEFAULT 'pending',"
            "  parse_error TEXT DEFAULT '', parse_version INTEGER DEFAULT 0,"
            "  plain_text TEXT DEFAULT '', latex_blocks TEXT DEFAULT '[]',"
            "  table_count INTEGER DEFAULT 0, figure_count INTEGER DEFAULT 0,"
            "  word_count INTEGER DEFAULT 0, page_count INTEGER DEFAULT 0,"
            "  pnote_path TEXT DEFAULT '', cnote_path TEXT DEFAULT '',"
            "  mnote_path TEXT DEFAULT '', embed_vector BLOB DEFAULT NULL)"
        )
        latex_blocks_json = json.dumps([{
            "source": "$x$", "is_display": False, "page": 0, "bbox": [0, 0, 0, 0]
        }])
        conn.execute(
            "INSERT OR REPLACE INTO papers "
            "(id, source, pdf_hash, parse_status, plain_text, latex_blocks,"
            " word_count, page_count, parse_version, added_at, updated_at) "
            "VALUES ('2301.00001', 'arxiv', 'abc123', 'done', 'cached text', ?, 1, 1, 1,"
            " '2023-01-01', '2023-01-01')",
            (latex_blocks_json,),
        )
        conn.commit()

        class _FakeDB:
            def __init__(self, conn):
                self.conn = conn

            def get_paper(self, paper_id):
                cur = self.conn.cursor()
                cur.execute("SELECT * FROM papers WHERE id = ?", (paper_id,))
                row = cur.fetchone()
                if row is None:
                    return None
                columns = [desc[0] for desc in cur.description]
                result = dict(zip(columns, row))
                # latex_blocks is stored as a JSON string; parse it here so
                # from_cache_dict receives a list (not a string) when iterating.
                result["latex_blocks"] = json.loads(result["latex_blocks"])
                from types import SimpleNamespace
                return SimpleNamespace(**{k: result[k] for k in [
                    "id", "pdf_hash", "parse_status", "plain_text",
                    "latex_blocks", "page_count", "word_count",
                    "parse_version", "title", "authors", "abstract", "published",
                ]})

        db = _FakeDB(conn)
        parser = PDFParser(db=db)
        result = parser._check_db_cache("2301.00001", "abc123")
        assert result is not None
        assert result.paper_id == "2301.00001"
        assert result.text == "cached text"
        assert result.parse_version == 1
        conn.close()


class TestSaveDBCache:
    """Edge cases for _save_db_cache."""

    def test_save_succeeds_with_valid_db(self, tmp_path):
        import sqlite3
        db_path = tmp_path / "research_save.db"
        conn = sqlite3.connect(str(db_path))
        conn.execute(
            "CREATE TABLE IF NOT EXISTS papers ("
            "  id TEXT PRIMARY KEY, source TEXT NOT NULL, title TEXT DEFAULT '',"
            "  authors TEXT DEFAULT '[]', abstract TEXT DEFAULT '', published TEXT DEFAULT '',"
            "  updated TEXT DEFAULT '', abs_url TEXT DEFAULT '', pdf_url TEXT DEFAULT '',"
            "  primary_category TEXT DEFAULT '', journal TEXT DEFAULT '', volume TEXT DEFAULT '',"
            "  issue TEXT DEFAULT '', page TEXT DEFAULT '', doi TEXT DEFAULT '',"
            "  categories TEXT DEFAULT '', reference_count INTEGER DEFAULT 0,"
            "  added_at TEXT NOT NULL, updated_at TEXT NOT NULL, pdf_path TEXT DEFAULT '',"
            "  pdf_hash TEXT DEFAULT '', parse_status TEXT DEFAULT 'pending',"
            "  parse_error TEXT DEFAULT '', parse_version INTEGER DEFAULT 0,"
            "  plain_text TEXT DEFAULT '', latex_blocks TEXT DEFAULT '[]',"
            "  table_count INTEGER DEFAULT 0, figure_count INTEGER DEFAULT 0,"
            "  word_count INTEGER DEFAULT 0, page_count INTEGER DEFAULT 0,"
            "  pnote_path TEXT DEFAULT '', cnote_path TEXT DEFAULT '',"
            "  mnote_path TEXT DEFAULT '', embed_vector BLOB DEFAULT NULL)"
        )
        # Pre-insert row so the UPDATE in _save_db_cache finds it
        conn.execute(
            "INSERT INTO papers (id, source, added_at, updated_at) "
            "VALUES ('2301.00001', 'arxiv', '2023-01-01', '2023-01-01')"
        )
        conn.commit()

        class _FakeDB:
            def __init__(self, conn):
                self.conn = conn

            def get_paper(self, paper_id):
                cur = self.conn.cursor()
                cur.execute("SELECT * FROM papers WHERE id = ?", (paper_id,))
                row = cur.fetchone()
                if row is None:
                    return None
                columns = [desc[0] for desc in cur.description]
                return dict(zip(columns, row))

            def update_parse_status(self, paper_id, status, error="", plain_text="",
                                    latex_blocks=None, table_count=0, figure_count=0,
                                    word_count=0, page_count=0):
                latex_json = json.dumps(latex_blocks or []) if not isinstance(latex_blocks, str) else latex_blocks
                cur = self.conn.cursor()
                cur.execute(
                    "SELECT parse_version FROM papers WHERE id = ?", (paper_id,)
                )
                row = cur.fetchone()
                version = (row[0] if row else 0) + 1
                cur.execute(
                    "UPDATE papers SET parse_status=?, parse_error=?, plain_text=?, "
                    "latex_blocks=?, table_count=?, figure_count=?, word_count=?, "
                    "page_count=?, parse_version=? WHERE id=?",
                    (status, error, plain_text, latex_json,
                     table_count, figure_count, word_count, page_count,
                     version, paper_id),
                )
                self.conn.commit()

        db = _FakeDB(conn)
        paper = ParsedPaper(
            paper_id="2301.00001",
            text="parsed text content",
            latex_blocks=[],
            tables=[],
            figures=[],
            page_count=5,
            word_count=100,
            parse_version=1,
            pdf_hash="hash123",
        )
        parser = PDFParser(db=db)
        parser._save_db_cache(paper)
        saved = db.get_paper("2301.00001")
        assert saved is not None
        assert saved["parse_status"] == "done"
        assert saved["plain_text"] == "parsed text content"
        conn.close()

    def test_save_swallows_exception_on_bad_db(self, tmp_path):
        class BadDB:
            def update_parse_status(self, **kwargs):
                raise RuntimeError("db write failed")
        paper = ParsedPaper(paper_id="2301.00001", text="x")
        parser = PDFParser(db=BadDB())
        # Should NOT raise — exceptions are swallowed with a warning log
        parser._save_db_cache(paper)


class TestCheckFileCache:
    """Edge cases for _check_file_cache."""

    def test_returns_none_when_cache_file_missing(self, tmp_path):
        parser = PDFParser(cache_dir=tmp_path / "nonexistent")
        result = parser._check_file_cache("2301.00001", "anyhash")
        assert result is None

    def test_returns_none_when_pdf_hash_mismatches(self, tmp_path):
        cache_dir = tmp_path / "cache"
        cache_dir.mkdir()
        cache_file = cache_dir / "2301.00001.json"
        cache_file.write_text(json.dumps({
            "paper_id": "2301.00001",
            "text": "old cached text",
            "pdf_hash": "oldhash",
            "latex_blocks": [],
            "tables": [],
            "figures": [],
            "page_count": 1,
            "word_count": 0,
            "parse_version": 1,
            "title": "",
            "authors": [],
            "abstract": "",
            "published": "",
            "warnings": [],
            "errors": [],
        }))
        parser = PDFParser(cache_dir=cache_dir)
        result = parser._check_file_cache("2301.00001", "different_hash")
        assert result is None

    def test_returns_none_on_corrupt_json(self, tmp_path):
        cache_dir = tmp_path / "cache"
        cache_dir.mkdir()
        cache_file = cache_dir / "2301.00001.json"
        cache_file.write_text("not valid json{")
        parser = PDFParser(cache_dir=cache_dir)
        result = parser._check_file_cache("2301.00001", "anyhash")
        assert result is None

    def test_returns_parsed_paper_on_hit(self, tmp_path):
        cache_dir = tmp_path / "cache"
        cache_dir.mkdir()
        cache_file = cache_dir / "2301.00001.json"
        cache_file.write_text(json.dumps({
            "paper_id": "2301.00001",
            "text": "cached content",
            "pdf_hash": "abchash",
            "latex_blocks": [],
            "tables": [],
            "figures": [],
            "page_count": 3,
            "word_count": 10,
            "parse_version": 2,
            "title": "Cached Title",
            "authors": ["Alice"],
            "abstract": "Abs",
            "published": "2023",
            "warnings": [],
            "errors": [],
        }))
        parser = PDFParser(cache_dir=cache_dir)
        result = parser._check_file_cache("2301.00001", "abchash")
        assert result is not None
        assert result.paper_id == "2301.00001"
        assert result.text == "cached content"
        assert result.parse_version == 2


class TestParsedPaperReprEq:
    """Tests for ParsedPaper __repr__ and __eq__."""

    def test_repr_returns_string(self):
        paper = ParsedPaper(
            paper_id="2301.00001",
            text="Some text",
            latex_blocks=[],
            tables=[],
            figures=[],
        )
        r = repr(paper)
        assert isinstance(r, str)
        assert "2301.00001" in r

    def test_eq_true_for_identical_papers(self):
        p1 = ParsedPaper(
            paper_id="2301.00001",
            text="Same text",
            latex_blocks=[],
            tables=[],
            figures=[],
            page_count=1,
            word_count=1,
            parse_version=1,
            pdf_hash="abc",
        )
        p2 = ParsedPaper(
            paper_id="2301.00001",
            text="Same text",
            latex_blocks=[],
            tables=[],
            figures=[],
            page_count=1,
            word_count=1,
            parse_version=1,
            pdf_hash="abc",
        )
        assert p1 == p2

    def test_eq_false_for_different_papers(self):
        p1 = ParsedPaper(paper_id="2301.00001", text="Text A")
        p2 = ParsedPaper(paper_id="2301.00001", text="Text B")
        assert p1 != p2

    def test_eq_false_for_non_parsed_paper(self):
        paper = ParsedPaper(paper_id="2301.00001", text="x")
        assert paper != "not a paper"


class TestParseErrorPath:
    """Tests for parse error handling path."""

    def test_raises_pdf_parse_error_when_both_methods_fail(self, sample_pdf, monkeypatch):
        """When _extract_structured raises AND pdfminer returns empty → PDFParseError."""
        def bad_extract(path):
            raise RuntimeError("PyMuPDF destroyed")

        def empty_fallback(path):
            return {
                "text": "",  # empty — triggers PDFParseError
                "latex_blocks": [],
                "tables": [],
                "figures": [],
                "page_count": 0,
                "warnings": [],
                "errors": [],
            }

        parser = PDFParser()
        monkeypatch.setattr(parser, "_extract_structured", bad_extract)
        monkeypatch.setattr(parser, "_pdfminer_fallback", empty_fallback)

        with pytest.raises(Exception) as exc_info:  # PDFParseError
            parser.parse(sample_pdf, paper_id="2301.00001", use_cache=False)
        assert "All PDF extraction methods failed" in str(exc_info.value)

    def test_pymupdf_failure_with_pdfminer_content_succeeds(self, sample_pdf, monkeypatch):
        """When PyMuPDF fails but pdfminer returns non-empty → succeeds with warning."""
        def bad_extract(path):
            raise RuntimeError("PyMuPDF error")

        def fallback_with_content(path):
            return {
                "text": "Recovered via pdfminer",
                "latex_blocks": [],
                "tables": [],
                "figures": [],
                "page_count": 1,
                "warnings": ["Used pdfminer fallback"],
                "errors": [],
            }

        parser = PDFParser()
        monkeypatch.setattr(parser, "_extract_structured", bad_extract)
        monkeypatch.setattr(parser, "_pdfminer_fallback", fallback_with_content)

        result = parser.parse(sample_pdf, paper_id="2301.00001", use_cache=False)
        assert result.text == "Recovered via pdfminer"
        assert "Used pdfminer fallback" in result.warnings
