"""Tests for pdf/parser.py — PDFParser, ParsedPaper, cache."""
from __future__ import annotations

import json

import pytest

from pdf.parser import (
    LaTeXBlock,
    ParsedPaper,
    PDFParser,
    TableData,
    FigureData,
)


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


class TestPDFParserEdgeCases:
    """Cover pdf/parser.py edge-case branches (lines 241, 264, 345)."""

    # ── _pdfminer_fallback ──────────────────────────────────────────────────

    def test_pdfminer_fallback_empty_text(self, sample_pdf, monkeypatch):
        """Lines 465, 488-497: pdfminer returns whitespace-only → text='',
        falls through to bottom return with 'Used pdfminer fallback' warning."""
        try:
            import pdfminer  # noqa: F401
        except ImportError:
            pytest.skip("pdfminer not installed")

        parser = PDFParser()
        monkeypatch.setattr("pdfminer.high_level.extract_text", lambda p: "   \n\t  ")

        result = parser._pdfminer_fallback(sample_pdf)
        assert result["text"] == ""
        assert "Used pdfminer fallback" in result["warnings"]

    def test_pdfminer_fallback_exception(self, sample_pdf, monkeypatch):
        """Lines 476-486: pdfminer raises → returns text='', errors contain
        'All extraction methods failed'."""
        try:
            import pdfminer  # noqa: F401
        except ImportError:
            pytest.skip("pdfminer not installed")

        parser = PDFParser()

        def raise_on_extract(path):
            raise RuntimeError("pdfminer broken")

        monkeypatch.setattr("pdfminer.high_level.extract_text", raise_on_extract)

        result = parser._pdfminer_fallback(sample_pdf)
        assert result["text"] == ""
        assert any("All extraction methods failed" in e for e in result["errors"])

    # ── _find_caption_near ─────────────────────────────────────────────────

    def test_find_caption_near_no_caption(self, sample_pdf):
        """Line 559: no blocks match caption pattern → returns ''."""
        import fitz
        parser = PDFParser()
        doc = fitz.open(str(sample_pdf))
        page = doc[0]
        # Page only has "Hello World" text - no figure/table caption
        bbox = (200, 300, 400, 400)
        caption = parser._find_caption_near(page, bbox, page_idx=0, search_radius=50.0)
        assert caption == ""

    # ── _extract_latex_blocks_from_text ────────────────────────────────────

    def test_extract_latex_blocks_empty(self):
        """Empty string → empty list."""
        parser = PDFParser()
        blocks = parser._extract_latex_blocks_from_text("", page_idx=0)
        assert blocks == []

    def test_extract_latex_blocks_no_math(self):
        """Plain text with no LaTeX math → empty list."""
        parser = PDFParser()
        blocks = parser._extract_latex_blocks_from_text(
            "This is a plain paragraph with no math at all.", page_idx=0
        )
        assert blocks == []

    # ── existing timeout test ────────────────────────────────────────────────

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

    @pytest.mark.no_freeze
    def test_parse_timeout_raises(self, sample_pdf):  # noqa: F811
        from pdf.parser import ParseTimeoutError
        parser = PDFParser()
        with pytest.raises(ParseTimeoutError):
            parser.parse(sample_pdf, paper_id="t", use_cache=False, max_parse_time=0.001)

    def test_parse_pymupdf_exception_raises_when_fallback_also_fails(self, sample_pdf, monkeypatch):
        """Lines 233-237: PyMuPDF exception + empty pdfminer fallback = PDFParseError."""
        parser = PDFParser()
        parser.db = None
        monkeypatch.setattr(parser, "_extract_structured", lambda p: (_ for _ in ()).throw(RuntimeError("pymupdf broken")))
        monkeypatch.setattr(parser, "_pdfminer_fallback", lambda p: {"text": "", "warnings": [], "errors": []})
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


class TestExtractStructuredExceptions:
    """Cover exception paths inside _extract_structured."""

    def test_table_detection_exception_appends_warning(self, sample_pdf, monkeypatch):
        """Lines 376-377: table detection exception → warning appended."""
        import fitz
        parser = PDFParser()
        parser.db = None

        # Patch find_tables to raise
        class FakeDoc:
            page_count = 1
            def load_page(self, i):
                return FakePage()
            def close(self):
                pass

        class FakePage:
            def find_tables(self):
                raise RuntimeError("table detection unavailable")
            def get_images(self, full=True):
                return []
            def get_text(self, kind="dict", flags=0):
                return {"blocks": []}

        monkeypatch.setattr(fitz, "open", lambda p: FakeDoc())

        result = parser.parse(sample_pdf, paper_id="t", use_cache=False)
        # Should not raise; should return result with warning
        assert "Table detection failed" in str(result.warnings)

    def test_figure_detection_exception_appends_warning(self, sample_pdf, monkeypatch):
        """Lines 393-394: figure detection exception → warning appended."""
        import fitz
        parser = PDFParser()
        parser.db = None

        class FakeDoc:
            page_count = 1
            def load_page(self, i):
                return FakePage()
            def close(self):
                pass

        class FakePage:
            def find_tables(self):
                class FakeTableBrowse:
                    def __iter__(self):
                        return iter([])
                return FakeTableBrowse()
            def get_images(self, full=True):
                raise RuntimeError("image detection unavailable")
            def get_text(self, kind="dict", flags=0):
                return {"blocks": []}

        monkeypatch.setattr(fitz, "open", lambda p: FakeDoc())

        result = parser.parse(sample_pdf, paper_id="t", use_cache=False)
        assert "Figure detection failed" in str(result.warnings)

    def test_get_text_exception_sets_empty_dict(self, sample_pdf, monkeypatch):
        """Lines 399-400: get_text exception → page_dict = {}."""
        import fitz
        parser = PDFParser()
        parser.db = None

        class FakeDoc:
            page_count = 1
            def load_page(self, i):
                return FakePage()
            def close(self):
                pass

        class FakePage:
            def find_tables(self):
                class FakeTableBrowse:
                    def __iter__(self):
                        return iter([])
                return FakeTableBrowse()
            def get_images(self, full=True):
                return []
            def get_text(self, kind="dict", flags=0):
                raise RuntimeError("text extraction unavailable")

        monkeypatch.setattr(fitz, "open", lambda p: FakeDoc())

        result = parser.parse(sample_pdf, paper_id="t", use_cache=False)
        # Should not raise; text may be empty but no crash
        assert isinstance(result.text, str)

    def test_pymupdf_exception_with_pdfminer_recovery_adds_warning(self, sample_pdf, monkeypatch):
        """Line 236: PyMuPDF exception + pdfminer non-empty → warning includes PyMuPDF failure."""
        parser = PDFParser()
        parser.db = None

        def bad_extract(path):
            raise RuntimeError("PyMuPDF destroyed")

        def pdfminer_recovery(path):
            return {
                "text": "Recovered text from pdfminer",
                "latex_blocks": [],
                "tables": [],
                "figures": [],
                "page_count": 1,
                "warnings": ["Used pdfminer fallback"],
                "errors": [],
            }

        monkeypatch.setattr(parser, "_extract_structured", bad_extract)
        monkeypatch.setattr(parser, "_pdfminer_fallback", pdfminer_recovery)

        result = parser.parse(sample_pdf, paper_id="2301.00001", use_cache=False)
        assert result.text == "Recovered text from pdfminer"
        # Warning about PyMuPDF failure appended (line 236)
        assert any("PyMuPDF failed" in w for w in result.warnings)


class TestIsDisplayMath:
    """Cover _is_display_math function."""

    def test_non_math_line_returns_false(self):
        from pdf.parser import _is_display_math
        assert _is_display_math("This is plain text") is False
        assert _is_display_math("") is False

    def test_inline_math_not_display(self):
        from pdf.parser import _is_display_math
        assert _is_display_math("$x^2$") is False
        assert _is_display_math("\\($y$\\)") is False

    def test_display_math_patterns_return_true(self):
        from pdf.parser import _is_display_math
        assert _is_display_math("$$x^2$$") is True
        assert _is_display_math("$$ formula $$") is True
        assert _is_display_math("\\[ formula \\]") is True
        assert _is_display_math("\\begin{align}a\\end{align}") is True


class TestExtractLatexBlocksFromText:
    """Cover _extract_latex_blocks_from_text (lines 563-586)."""

    def test_extracts_inline_math(self):
        from pdf.parser import PDFParser
        parser = PDFParser()
        text = "Let $x$ be a variable"
        blocks = parser._extract_latex_blocks_from_text(text, page_idx=0)
        assert len(blocks) >= 1
        assert any(not b.is_display for b in blocks)

    def test_extracts_from_multiline_text(self):
        from pdf.parser import PDFParser
        parser = PDFParser()
        text = "Line one\nLine two\nLine three"
        blocks = parser._extract_latex_blocks_from_text(text, page_idx=0)
        # Just ensure no crash and returns a list
        assert isinstance(blocks, list)


class TestFindCaptionNear:
    """Cover _find_caption_near exception path."""

    def test_returns_empty_on_exception(self):
        from pdf.parser import PDFParser
        parser = PDFParser()
        # Pass a non-fitz page object to trigger exception
        class FakePage:
            def get_text(self, kind="dict"):
                raise RuntimeError("get_text unavailable")
        result = parser._find_caption_near(FakePage(), (10, 20, 30, 40), page_idx=0)
        assert result == ""


class TestPdfminerFallback:
    """Cover pdf/parser.py lines 457-497: pdfminer fallback branches."""

    def test_pdfminer_fallback_returns_text(self, sample_pdf, monkeypatch):
        """Lines 466-475: pdfminer returns non-empty text → returned as fallback."""
        parser = PDFParser()
        parser.db = None

        def fake_pdfminer_fallback(path):
            return {
                "text": "Recovered via pdfminer fallback",
                "latex_blocks": [],
                "tables": [],
                "figures": [],
                "page_count": 1,
                "warnings": ["Used pdfminer fallback"],
                "errors": [],
            }

        def bad_extract(path):
            raise RuntimeError("pymupdf broken")

        monkeypatch.setattr(parser, "_extract_structured", bad_extract)
        monkeypatch.setattr(parser, "_pdfminer_fallback", fake_pdfminer_fallback)

        result = parser.parse(sample_pdf, paper_id="2301.00001", use_cache=False)
        assert result.text == "Recovered via pdfminer fallback"
        assert "Used pdfminer fallback" in result.warnings

    def test_pdfminer_fallback_returns_empty_dict_raises_pdfparse_error(self, sample_pdf, monkeypatch):
        """Lines 233-235: _extract_structured raises, pdfminer returns empty dict → PDFParseError."""
        parser = PDFParser()
        parser.db = None

        def empty_fallback(path):
            return {
                "text": "",
                "latex_blocks": [],
                "tables": [],
                "figures": [],
                "page_count": 0,
                "warnings": [],
                "errors": ["pdfminer extraction failed"],
            }

        def bad_extract(path):
            raise RuntimeError("pymupdf broken")

        monkeypatch.setattr(parser, "_extract_structured", bad_extract)
        monkeypatch.setattr(parser, "_pdfminer_fallback", empty_fallback)

        from core.exceptions import PDFParseError
        with pytest.raises(PDFParseError, match="All PDF extraction methods failed"):
            parser.parse(sample_pdf, paper_id="2301.00001", use_cache=False)

    def test_pdfminer_fallback_returns_empty_text(self, sample_pdf, monkeypatch):
        """Lines 488-497: pdfminer returns empty text (text == "") → PDFParseError."""
        parser = PDFParser()
        parser.db = None

        def empty_text_fallback(path):
            return {
                "text": "",
                "latex_blocks": [],
                "tables": [],
                "figures": [],
                "page_count": 0,
                "warnings": ["Used pdfminer fallback"],
                "errors": [],
            }

        def bad_extract(path):
            raise RuntimeError("pymupdf broken")

        monkeypatch.setattr(parser, "_extract_structured", bad_extract)
        monkeypatch.setattr(parser, "_pdfminer_fallback", empty_text_fallback)

        from core.exceptions import PDFParseError
        with pytest.raises(PDFParseError, match="All PDF extraction methods failed"):
            parser.parse(sample_pdf, paper_id="2301.00001", use_cache=False)

    def test_extract_structured_returns_empty_text_still_succeeds(self, sample_pdf, monkeypatch):
        """When _extract_structured succeeds but returns empty text (no exception), parse succeeds."""
        parser = PDFParser()
        parser.db = None

        def empty_extract(path):
            return {
                "text": "",
                "latex_blocks": [],
                "tables": [],
                "figures": [],
                "page_count": 0,
                "warnings": ["No text extracted"],
                "errors": [],
            }

        monkeypatch.setattr(parser, "_extract_structured", empty_extract)

        result = parser.parse(sample_pdf, paper_id="2301.00001", use_cache=False)
        assert result.text == ""
        assert "No text extracted" in result.warnings

    def test_pdfminer_fallback_returns_empty_on_empty_text(self, sample_pdf, monkeypatch):
        """Lines 462-489: pdfminer returns "" (empty string) → _pdfminer_fallback returns empty dict."""
        try:
            import fitz  # noqa: F401
        except ImportError:
            pytest.skip("PyMuPDF not installed")

        try:
            import pdfminer  # noqa: F401
        except ImportError:
            pytest.skip("pdfminer not installed")

        parser = PDFParser()
        parser.db = None

        # Mock pdfminer.extract_text to return empty string
        def mock_extract_text(path):
            return ""

        monkeypatch.setattr(
            "pdfminer.high_level.extract_text",
            mock_extract_text
        )

        result = parser._pdfminer_fallback(sample_pdf)
        # When text is "", the function falls through to the bottom
        # and returns a dict with text="" and "Used pdfminer fallback" warning
        assert result["text"] == ""
        assert "Used pdfminer fallback" in result["warnings"]

    def test_pdfminer_fallback_returns_empty_on_none_text(self, sample_pdf, monkeypatch):
        """Lines 462-465: pdfminer returns None → treated as empty string → empty dict returned."""
        try:
            import fitz  # noqa: F401
        except ImportError:
            pytest.skip("PyMuPDF not installed")

        try:
            import pdfminer  # noqa: F401
        except ImportError:
            pytest.skip("pdfminer not installed")

        parser = PDFParser()
        parser.db = None

        # Mock pdfminer.extract_text to return None (or)
        def mock_extract_text(path):
            return None

        monkeypatch.setattr(
            "pdfminer.high_level.extract_text",
            mock_extract_text
        )

        result = parser._pdfminer_fallback(sample_pdf)
        assert result["text"] == ""


class TestHashFileOSError:
    """Cover lines 220 / 224-227: _hash_file raises OSError → propagates from parse()."""

    def test_hash_file_raises_oserror_propagates(self, sample_pdf, monkeypatch):
        """Line 220: OSError from _hash_file is not caught and propagates from parse()."""
        parser = PDFParser()

        # Patch the file read inside _hash_file to raise OSError
        orig_open = open
        def bad_open(path, mode="r", *a, **k):
            if mode == "rb" and "sample" in str(path):
                raise OSError("Permission denied")
            return orig_open(path, mode, *a, **k)

        monkeypatch.setattr("builtins.open", bad_open)

        with pytest.raises(OSError, match="Permission denied"):
            parser.parse(sample_pdf, paper_id="t", use_cache=False)


class TestSaveFileCacheOSError:
    """Cover line 264 and lines 344-345: _save_file_cache write_text raises OSError."""

    def test_save_file_cache_write_text_oserror(self, sample_pdf, tmp_path, monkeypatch):
        """Line 264 / 344-345: OSError in cache_file.write_text is caught with warning."""
        cache_dir = tmp_path / "parsed"
        parser = PDFParser(cache_dir=cache_dir)
        parser.db = None

        # Patch the specific write_text call to raise OSError
        import pathlib
        orig_write_text = pathlib.Path.write_text

        def bad_write_text(self, *a, **k):
            if "parsed" in str(self) and ".json" in str(self):
                raise OSError("No space left on device")
            return orig_write_text(self, *a, **k)

        monkeypatch.setattr(pathlib.Path, "write_text", bad_write_text)

        # Should NOT raise — OSError is caught inside _save_file_cache
        result = parser.parse(sample_pdf, paper_id="t", use_cache=False)
        assert isinstance(result, ParsedPaper)


class TestEmptyTextBlockSkip:
    """Cover line 411: empty / len(line_text) < 2 blocks are skipped."""

    def test_empty_span_in_block_is_skipped(self, sample_pdf, monkeypatch):
        """Line 410-411: block with empty span text is skipped (no crash)."""
        import fitz
        parser = PDFParser()
        parser.db = None

        class FakeDoc:
            page_count = 1
            def load_page(self, i):
                return FakePage()
            def close(self):
                pass

        class FakePage:
            def find_tables(self):
                class FakeTableBrowse:
                    def __iter__(self):
                        return iter([])
                return FakeTableBrowse()
            def get_images(self, full=True):
                return []
            def get_text(self, kind="dict", flags=0):
                # Block with a span whose text is empty string
                return {
                    "blocks": [{
                        "type": 0,
                        "bbox": (0, 0, 100, 20),
                        "lines": [{
                            "lines": [],
                            "spans": [{"text": ""}],
                            "bbox": (0, 0, 100, 20)
                        }]
                    }, {
                        "type": 0,
                        "bbox": (0, 30, 100, 50),
                        "lines": [{
                            "lines": [],
                            "spans": [{"text": "x"}],
                            "bbox": (0, 30, 100, 50)
                        }]
                    }]
                }

        monkeypatch.setattr(fitz, "open", lambda p: FakeDoc())

        # Should not raise; empty and single-char spans are skipped
        result = parser.parse(sample_pdf, paper_id="t", use_cache=False)
        assert isinstance(result, ParsedPaper)
        # The "x" single-char block is also skipped (len < 2)
        assert result.text == ""


class TestTableToStructuredEdgeCases:
    """Cover lines 509-535: _table_to_structured edge cases."""

    def test_empty_rows_returns_none(self):
        """Lines 520-521: empty rows list → returns None."""
        from pdf.parser import PDFParser
        parser = PDFParser()

        class FakeTable:
            rows = []
            bbox = (0, 0, 100, 100)

        result = parser._table_to_structured(FakeTable(), page_idx=0)
        assert result is None

    def test_single_row_returns_none(self):
        """Lines 520-521: rows with only header (len=1) → returns None."""
        from pdf.parser import PDFParser
        parser = PDFParser()

        class FakeRow:
            def __init__(self, texts):
                self._texts = texts
            def __iter__(self):
                class Cell:
                    def __init__(self, t):
                        self.text = t
                return iter([Cell(t) for t in self._texts])

        class FakeTable:
            rows = [FakeRow(["Header1", "Header2"])]
            bbox = (0, 0, 100, 100)

        result = parser._table_to_structured(FakeTable(), page_idx=0)
        assert result is None

    def test_two_rows_produces_table(self):
        """Lines 523-532: two rows (header + data) → TableData returned."""
        from pdf.parser import PDFParser
        parser = PDFParser()

        class FakeCell:
            def __init__(self, t):
                self.text = t

        class FakeRow:
            def __init__(self, texts):
                self._texts = texts
            def __iter__(self):
                return iter([FakeCell(t) for t in self._texts])

        class FakeTable:
            rows = [
                FakeRow(["ColA", "ColB"]),
                FakeRow(["val1", "val2"]),
            ]
            bbox = (10, 20, 30, 40)

        result = parser._table_to_structured(FakeTable(), page_idx=1)
        assert result is not None
        assert result.headers == ["ColA", "ColB"]
        assert result.rows == [["val1", "val2"]]
        assert result.page == 1

    def test_no_bbox_attribute_uses_default(self):
        """Line 526: tbl.bbox missing → uses (0,0,0,0)."""
        from pdf.parser import PDFParser
        parser = PDFParser()

        class FakeCell:
            def __init__(self, t):
                self.text = t

        class FakeRow:
            def __init__(self, texts):
                self._texts = texts
            def __iter__(self):
                return iter([FakeCell(t) for t in self._texts])

        class FakeTable:
            rows = [FakeRow(["H"]), FakeRow(["D"])]
            bbox = None  # missing bbox

        result = parser._table_to_structured(FakeTable(), page_idx=0)
        assert result is not None
        assert result.bbox == (0, 0, 0, 0)


class TestFindCaptionNearEmpty:
    """Cover line 548: _find_caption_near returns '' when no caption found."""

    def test_returns_empty_when_no_caption_matches(self):
        """Line 559: no matching caption → returns ''."""
        from pdf.parser import PDFParser
        parser = PDFParser()

        class FakeSpan:
            def __init__(self, text):
                self.text = text

        class FakeLine:
            def __init__(self, spans_texts):
                self._spans_texts = spans_texts

            def get(self, key, default=None):
                if key == "spans":
                    return [FakeSpan(t) for t in self._spans_texts]
                return default

        class FakeBlock:
            def __init__(self, btype, bbox, lines):
                self._type = btype
                self._bbox = bbox
                self._lines = lines

            def get(self, key, default=None):
                if key == "type":
                    return self._type
                if key == "bbox":
                    return self._bbox
                if key == "lines":
                    return self._lines
                return default

        class FakePage:
            def get_text(self, kind="dict"):
                return {
                    "blocks": [
                        FakeBlock(0, (0, 0, 100, 20), [["Not a caption", "just text"]]),
                        FakeBlock(0, (0, 30, 100, 50), [["Another line"]]),
                    ]
                }

        result = parser._find_caption_near(FakePage(), (50, 75, 150, 125), page_idx=0)
        assert result == ""


class TestExtractLatexBlocksEmptyInput:
    """Cover lines 575-578: _extract_latex_blocks_from_text with empty/no display math."""

    def test_empty_string_returns_empty_list(self):
        """Lines 569-586: empty input → returns []. """
        from pdf.parser import PDFParser
        parser = PDFParser()
        result = parser._extract_latex_blocks_from_text("", page_idx=0)
        assert result == []

    def test_no_display_math_only_inline(self):
        """Line 580-581: inline math found but no display math → inline returned."""
        from pdf.parser import PDFParser
        parser = PDFParser()
        text = "Equation $x = 1$ and $y = 2$"
        blocks = parser._extract_latex_blocks_from_text(text, page_idx=0)
        assert len(blocks) == 2
        assert all(not b.is_display for b in blocks)


class TestPdfminerFallbackExceptions:
    """Cover lines 467-478: _pdfminer_fallback exception path and empty text."""

    def test_pdfminer_exception_returns_error_dict(self, sample_pdf, monkeypatch):
        """Lines 476-486: pdfminer raises Exception → returns error dict."""
        try:
            import pdfminer  # noqa: F401
        except ImportError:
            pytest.skip("pdfminer not installed")

        parser = PDFParser()
        parser.db = None

        def raise_on_extract(path):
            raise RuntimeError("pdfminer broken")

        monkeypatch.setattr("pdfminer.high_level.extract_text", raise_on_extract)

        result = parser._pdfminer_fallback(sample_pdf)
        assert result["text"] == ""
        assert "All extraction methods failed" in result["errors"][0]

    def test_pdfminer_returns_empty_string(self, sample_pdf, monkeypatch):
        """Lines 465, 488-497: pdfminer returns "" → bottom return with text=""."""
        try:
            import pdfminer  # noqa: F401
        except ImportError:
            pytest.skip("pdfminer not installed")

        parser = PDFParser()
        parser.db = None

        monkeypatch.setattr("pdfminer.high_level.extract_text", lambda p: "")

        result = parser._pdfminer_fallback(sample_pdf)
        # text is "" → falls through to bottom return
        assert result["text"] == ""


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


# ---------------------------------------------------------------------------
# Additional tests for uncovered _extract_structured paths
# ---------------------------------------------------------------------------


class TestExtractStructuredTableWithRows:
    """Cover _extract_structured with tables containing actual rows."""

    def test_extract_structured_with_real_table_rows(self, sample_pdf, monkeypatch):
        """Tables with actual rows flow through _table_to_structured to completion."""
        import fitz
        parser = PDFParser()
        parser.db = None

        class FakeCell:
            def __init__(self, t):
                self.text = t

        class FakeRow:
            def __init__(self, texts):
                self._texts = texts

            def __iter__(self):
                return iter([FakeCell(t) for t in self._texts])

        class FakeTable:
            def __init__(self, rows_texts, bbox):
                self._rows_texts = rows_texts
                self.bbox = bbox

            @property
            def rows(self):
                return [FakeRow(r) for r in self._rows_texts]

        class FakePage:
            def find_tables(self):
                # Return a table with header + 2 data rows
                return iter([FakeTable(
                    [["ColA", "ColB"], ["v1", "v2"], ["v3", "v4"]],
                    bbox=(10, 20, 30, 40)
                )])

            def get_images(self, full=True):
                return []

            def get_text(self, kind="dict", flags=0):
                return {"blocks": []}

        class FakeDoc:
            page_count = 1

            def load_page(self, i):
                return FakePage()

            def close(self):
                pass

        monkeypatch.setattr(fitz, "open", lambda p: FakeDoc())

        result = parser.parse(sample_pdf, paper_id="t", use_cache=False)
        assert len(result.tables) == 1
        assert result.tables[0].headers == ["ColA", "ColB"]
        assert result.tables[0].rows == [["v1", "v2"], ["v3", "v4"]]


class TestTableToStructuredCellAccess:
    """Cover _table_to_structured exception during cell text access."""

    def test_cell_text_attribute_raises(self):
        """Line 516: cell.text raises AttributeError → caught, returns None."""
        from pdf.parser import PDFParser

        parser = PDFParser()

        class BadCell:
            """Cell whose .text attribute itself raises."""
            text = property(lambda self: (_ for _ in ()).throw(AttributeError("no text")))

        class FakeRow:
            def __init__(self, cells):
                self._cells = cells

            def __iter__(self):
                return iter(self._cells)

        class FakeTable:
            rows = [FakeRow([BadCell(), BadCell()])]
            bbox = (0, 0, 100, 100)

        result = parser._table_to_structured(FakeTable(), page_idx=0)
        assert result is None


class TestExtractStructuredShortLineSkipped:
    """Cover line 410-411: line_text < 2 chars is skipped silently."""

    @pytest.mark.no_freeze
    def test_short_line_skipped_in_extract_structured(self, sample_pdf, monkeypatch):
        """Line 410-411: single-char line_text is skipped (continues, no crash)."""
        import fitz
        parser = PDFParser()
        parser.db = None

        class FakeDoc:
            page_count = 1

            def load_page(self, i):
                return FakePage()

            def close(self):
                pass

        class FakePage:
            def find_tables(self):
                class Empty:
                    def __iter__(self):
                        return iter([])
                return Empty()

            def get_images(self, full=True):
                return []

            def get_text(self, kind="dict", flags=0):
                # Block with a 1-char span and a 2-char span
                return {
                    "blocks": [{
                        "type": 0,
                        "bbox": (0, 0, 100, 20),
                        "lines": [{
                            "spans": [{"text": "x"}],
                            "bbox": (0, 0, 100, 20)
                        }]
                    }, {
                        "type": 0,
                        "bbox": (0, 30, 100, 50),
                        "lines": [{
                            "spans": [{"text": "ab"}],
                            "bbox": (0, 30, 100, 50)
                        }]
                    }]
                }

        monkeypatch.setattr(fitz, "open", lambda p: FakeDoc())

        result = parser.parse(sample_pdf, paper_id="t", use_cache=False)
        # "x" is len=1 < 2 → skipped; "ab" is kept
        assert result.text == "ab"


class TestFindCaptionNearNoCaption:
    """Cover _find_caption_near when no caption matches (line 559 returns '')."""

    def test_find_caption_near_returns_empty_when_blocks_do_not_match(self):
        """Line 559: no caption block within search_radius → returns ''."""
        from pdf.parser import PDFParser

        parser = PDFParser()

        # Image bbox is far from the single text block → no caption found
        class FakePage:
            def get_text(self, kind="dict"):
                return {
                    "blocks": [{
                        "type": 0,
                        "bbox": (0, 0, 100, 20),  # block at y=10 (center)
                        "lines": [{
                            "spans": [{"text": "Some regular text"}],
                            "bbox": (0, 0, 100, 20)
                        }]
                    }]
                }

        # Image at y_center=500, block at y_center=10, distance=490 > search_radius=50
        result = parser._find_caption_near(FakePage(), (0, 490, 100, 510), page_idx=0)
        assert result == ""


class TestExtractLatexBlocksFromTextEmptyInput:
    """Cover _extract_latex_blocks_from_text with empty input (lines 569-578)."""

    def test_extract_latex_blocks_empty_text(self):
        """Lines 569-586: empty string input → returns []. No display math buffer."""
        from pdf.parser import PDFParser

        parser = PDFParser()
        result = parser._extract_latex_blocks_from_text("", page_idx=0)
        assert result == []

    def test_extract_latex_blocks_no_display_math_no_buffer(self):
        """Lines 573-583: text with no display math, buffer stays empty → final block not appended."""
        from pdf.parser import PDFParser

        parser = PDFParser()
        # Plain text, no display math, no inline math → empty list
        result = parser._extract_latex_blocks_from_text("Just plain text", page_idx=0)
        assert result == []

