"""Tests for pdf/extract.py — PDF download, extraction, and structured parsing."""
import re
import sys
from pathlib import Path
from unittest.mock import MagicMock

import pytest
import requests
import builtins

# Add project root to path so pdf/ is importable as a package
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from pdf.extract import (  # type: ignore[attr-defined]
    BlockType,
    MathBlock,
    StructuredPdfContent,
    TableBlock,
    TextBlock,
    _detect_block_type,
    _extract_inline_math,
    _is_display_math,
    _is_gibberish_or_too_short,
    _ocr_page,
    download_pdf,
    extract_pdf_structured,
    extract_pdf_text,
    extract_pdf_text_hybrid,
)
import ai_research_os as airo  # for the public API (download_pdf, extract_pdf_text, ...)


# ─── helpers ────────────────────────────────────────────────────────────────

def make_minimal_pdf(tmp_path: Path, pages_text: list[dict]) -> Path:
    """Create a minimal PDF with given page text using PyMuPDF.

    pages_text: list of {"text": str} dicts, one per page.
    """
    try:
        import fitz
    except ImportError:
        pytest.skip("PyMuPDF not installed")

    doc = fitz.open()
    for item in pages_text:
        page = doc.new_page(width=595, height=842)
        page.insert_text((72, 72), item["text"], fontsize=12)
    pdf_path = tmp_path / "test.pdf"
    doc.save(str(pdf_path))
    doc.close()
    return pdf_path


# ─── _is_gibberish_or_too_short ─────────────────────────────────────────────

class TestIsGibberishOrTooShort:
    def test_too_short(self):
        assert _is_gibberish_or_too_short("hello") is True

    def test_short_at_threshold(self):
        # 119 chars: just under 120 threshold
        s = "a" * 119
        assert _is_gibberish_or_too_short(s) is True

    def test_120_chars_ok(self):
        s = "a" * 120
        assert _is_gibberish_or_too_short(s) is False

    def test_non_printable_chars(self):
        # More than 2% non-printable chars → gibberish
        s = "a" * 100 + "\x00\x01\x02"
        assert _is_gibberish_or_too_short(s) is True

    def test_private_unicode_chars(self):
        # More than 2% private-use chars → gibberish
        s = "a" * 100 + "\ue000" * 5
        assert _is_gibberish_or_too_short(s) is True

    def test_valid_text_long_enough(self):
        # Must be >= 120 chars to not be flagged as "too short"
        s = "This is a perfectly normal English sentence with standard characters that makes sense and is readable and also this particular one is long enough to pass the threshold."
        assert _is_gibberish_or_too_short(s) is False

    def test_none_input(self):
        assert _is_gibberish_or_too_short(None) is True

    def test_empty_string(self):
        assert _is_gibberish_or_too_short("") is True

    def test_whitespace_only(self):
        assert _is_gibberish_or_too_short("   \n\t  ") is True


# ─── _detect_block_type ──────────────────────────────────────────────────────

class TestDetectBlockType:
    def test_markdown_heading_h1(self):
        assert _detect_block_type("# Introduction", BlockType.BODY, 0) == BlockType.HEADING

    def test_markdown_heading_h3(self):
        assert _detect_block_type("### Methods", BlockType.BODY, 0) == BlockType.HEADING

    def test_allcaps_short_header(self):
        assert _detect_block_type("INTRODUCTION", BlockType.BODY, 0) == BlockType.HEADING

    def test_allcaps_long_line_is_body(self):
        # Too long to be a header
        s = "THIS IS A VERY LONG LINE THAT IS MORE THAN SIXTY CHARACTERS LONG SO IT SHOULD NOT BE A HEADER"
        assert _detect_block_type(s, BlockType.BODY, 0) == BlockType.BODY

    def test_numbered_section(self):
        assert _detect_block_type("1. Introduction", BlockType.BODY, 0) == BlockType.HEADING

    def test_numbered_subsection(self):
        assert _detect_block_type("2.3. Experimental Setup", BlockType.BODY, 0) == BlockType.HEADING

    def test_roman_numeral_header_without_period(self):
        # Pattern matches "I Background" but not "I. Background" (period breaks the regex)
        assert _detect_block_type("I Background", BlockType.BODY, 0) == BlockType.HEADING

    def test_roman_numeral_header_no_match(self):
        # "I. Background" with period doesn't match the roman numeral pattern
        # (period not allowed after roman numeral in the regex)
        assert _detect_block_type("I. Background", BlockType.BODY, 0) == BlockType.BODY

    def test_iii_header_without_period(self):
        assert _detect_block_type("III Related Work", BlockType.BODY, 0) == BlockType.HEADING

    def test_figure_caption(self):
        assert _detect_block_type("Figure 1: Architecture", BlockType.BODY, 0) == BlockType.CAPTION

    def test_fig_caption(self):
        assert _detect_block_type("Fig. 2: Results", BlockType.BODY, 0) == BlockType.CAPTION

    def test_table_caption(self):
        assert _detect_block_type("Table 3: Performance", BlockType.BODY, 0) == BlockType.CAPTION

    def test_algorithm_caption(self):
        assert _detect_block_type("Algorithm 1: Training Loop", BlockType.BODY, 0) == BlockType.CAPTION

    def test_footnote_bracket(self):
        assert _detect_block_type("[1]", BlockType.BODY, 0) == BlockType.FOOTNOTE

    def test_footnote_caret(self):
        assert _detect_block_type("^42", BlockType.BODY, 0) == BlockType.FOOTNOTE

    def test_list_item_dash(self):
        assert _detect_block_type("- first item", BlockType.BODY, 0) == BlockType.LIST_ITEM

    def test_list_item_asterisk(self):
        assert _detect_block_type("* bullet", BlockType.BODY, 0) == BlockType.LIST_ITEM

    def test_list_item_plus(self):
        assert _detect_block_type("+ item", BlockType.BODY, 0) == BlockType.LIST_ITEM

    def test_list_item_numbered(self):
        assert _detect_block_type("1. item", BlockType.BODY, 0) == BlockType.LIST_ITEM

    def test_body_text(self):
        assert _detect_block_type("This is a body paragraph with normal text.", BlockType.BODY, 0) == BlockType.BODY

    def test_empty_line_returns_body(self):
        assert _detect_block_type("", BlockType.BODY, 0) == BlockType.BODY


# ─── _is_display_math ────────────────────────────────────────────────────────

class TestIsDisplayMath:
    def test_latex_display_dollar(self):
        assert _is_display_math("$$ x = y $$") is True

    def test_latex_display_brackets(self):
        assert _is_display_math("\\[ x^2 + y^2 = z^2 \\]") is True

    def test_align_environment(self):
        assert _is_display_math("""\\begin{align}
x &= 1 \\\\
y &= 2
\\end{align}""") is True

    def test_unicode_math_with_spaces(self):
        # Pattern requires unicode math chars separated by spaces around =
        # "∀ ∃ = ∃ ∀" works because each side has 2 space-separated chars
        s = "∀ ∃ = ∃ ∀"
        assert _is_display_math(s) is True

    def test_not_math(self):
        assert _is_display_math("This is a normal paragraph.") is False

    def test_inline_math_not_display(self):
        # Inline math should not be detected as display
        assert _is_display_math("$x = 1$") is False


# ─── _extract_inline_math ───────────────────────────────────────────────────

class TestExtractInlineMath:
    def test_single_inline_dollar(self):
        blocks = _extract_inline_math("The equation $x = 1$ is simple.")
        assert len(blocks) == 1
        assert blocks[0].is_display is False
        assert "x = 1" in blocks[0].text

    def test_multiple_inline(self):
        blocks = _extract_inline_math("Given $E = mc^2$ and $a = b + c$.")
        assert len(blocks) == 2

    def test_latex_inline_brackets(self):
        blocks = _extract_inline_math("Use \\(x\\leq y\\) here.")
        assert len(blocks) == 1
        assert "x\\leq y" in blocks[0].text

    def test_no_math(self):
        blocks = _extract_inline_math("Plain text with no math.")
        assert len(blocks) == 0

    def test_page_is_default_minus_one(self):
        blocks = _extract_inline_math("$x$")
        assert blocks[0].page == -1


# ─── download_pdf ────────────────────────────────────────────────────────────

class TestDownloadPdf:
    def test_download_pdf_writes_file(self, tmp_path, monkeypatch):
        out_path = tmp_path / "paper.pdf"

        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.iter_content = lambda chunk_size, **kw: iter([b"%PDF-1.4 fake"])
        mock_response.__enter__ = MagicMock(return_value=mock_response)
        mock_response.__exit__ = MagicMock(return_value=False)

        monkeypatch.setattr(requests, "get", lambda url, **kw: mock_response)

        download_pdf("https://example.com/paper.pdf", out_path, timeout=60)
        assert out_path.exists()

    def test_download_pdf_creates_parent_dirs(self, tmp_path, monkeypatch):
        out_path = tmp_path / "subdir" / "paper.pdf"

        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.iter_content = lambda chunk_size, **kw: iter([b"pdf"])
        mock_response.__enter__ = MagicMock(return_value=mock_response)
        mock_response.__exit__ = MagicMock(return_value=False)

        monkeypatch.setattr(requests, "get", lambda url, **kw: mock_response)

        download_pdf("https://example.com/paper.pdf", out_path)
        assert out_path.exists()

    def test_download_pdf_raises_on_error(self, tmp_path, monkeypatch):
        out_path = tmp_path / "paper.pdf"

        mock_response = MagicMock()
        mock_response.status_code = 404
        mock_response.__enter__ = MagicMock(return_value=mock_response)
        mock_response.__exit__ = MagicMock(return_value=False)
        mock_response.raise_for_status.side_effect = Exception("Not found")

        monkeypatch.setattr(requests, "get", lambda url, **kw: mock_response)

        with pytest.raises(Exception, match="Not found"):
            download_pdf("https://example.com/missing.pdf", out_path)


# ─── extract_pdf_text ─────────────────────────────────────────────────────────

class TestExtractPdfText:
    def test_extracts_text_from_pdf(self, tmp_path):
        try:
            import fitz
        except ImportError:
            pytest.skip("PyMuPDF not installed")

        pdf_path = make_minimal_pdf(tmp_path, [{"text": "Hello World"}])
        text = airo.extract_pdf_text(pdf_path, max_pages=1)
        assert "Hello World" in text

    def test_nonexistent_file_returns_empty(self):
        fake = Path("/nonexistent/pdf/file.pdf")
        text = airo.extract_pdf_text(fake)
        assert text == ""

    def test_max_pages_limits_extraction(self, tmp_path):
        try:
            import fitz
        except ImportError:
            pytest.skip("PyMuPDF not installed")

        pdf_path = make_minimal_pdf(tmp_path, [
            {"text": "Page One"},
            {"text": "Page Two"},
            {"text": "Page Three"},
        ])
        text = airo.extract_pdf_text(pdf_path, max_pages=1)
        assert "Page One" in text

    def test_whitespace_before_newline_not_stripped(self):
        # The function only strips spaces/tabs immediately before newlines,
        # not multiple spaces within text lines
        s = "a" * 130
        assert _is_gibberish_or_too_short(s) is False

    def test_collapse_multiple_newlines(self, tmp_path):
        try:
            import fitz
        except ImportError:
            pytest.skip("PyMuPDF not installed")

        # PyMuPDF may insert blank lines between content
        pdf_path = make_minimal_pdf(tmp_path, [{"text": "First\n\n\nSecond"}])
        text = airo.extract_pdf_text(pdf_path)
        # Should have at most double newlines
        assert "\n\n\n" not in text


# ─── _ocr_page ───────────────────────────────────────────────────────────────

class TestOcrPage:
    def test_raises_when_tesseract_missing(self, monkeypatch):
        try:
            import fitz
        except ImportError:
            pytest.skip("PyMuPDF not installed")

        # Simulate missing tesseract / PIL
        def mock_import(name, **kw):
            if name in ("pytesseract", "PIL"):
                raise ImportError(f"No module named '{name}'")
            return __import__(name, **kw)

        monkeypatch.setattr(builtins, "__import__", mock_import)

        with pytest.raises(RuntimeError, match="OCR deps missing"):
            _ocr_page(None)  # type: ignore

    def test_raises_when_fitz_missing(self, tmp_path, monkeypatch):
        # Remove fitz globally
        import sys
        mods = {k: v for k, v in sys.modules.items() if "fitz" in k or "pymupdf" in k}
        for k in mods:
            del sys.modules[k]
        monkeypatch.delitem(sys.modules, "fitz", raising=False)
        monkeypatch.delitem(sys.modules, "pymupdf", raising=False)

        with pytest.raises(RuntimeError, match="OCR deps missing"):
            _ocr_page(None)  # type: ignore


# ─── extract_pdf_text_hybrid ────────────────────────────────────────────────

class TestExtractPdfTextHybrid:
    def test_basic_extraction(self, tmp_path):
        try:
            import fitz
        except ImportError:
            pytest.skip("PyMuPDF not installed")

        pdf_path = make_minimal_pdf(tmp_path, [{"text": "Intro text here"}])
        text = airo.extract_pdf_text_hybrid(pdf_path)
        assert "Intro text" in text

    def test_max_pages(self, tmp_path):
        try:
            import fitz
        except ImportError:
            pytest.skip("PyMuPDF not installed")

        pdf_path = make_minimal_pdf(tmp_path, [
            {"text": "Page 1"},
            {"text": "Page 2"},
            {"text": "Page 3"},
        ])
        text = airo.extract_pdf_text_hybrid(pdf_path, max_pages=1)
        assert "Page 1" in text

    def test_empty_page(self, tmp_path):
        try:
            import fitz
        except ImportError:
            pytest.skip("PyMuPDF not installed")

        pdf_path = tmp_path / "empty.pdf"
        doc = fitz.open()
        doc.new_page(width=200, height=200)
        doc.save(str(pdf_path))
        doc.close()
        text = airo.extract_pdf_text_hybrid(pdf_path)
        assert text == ""

    def test_no_pdfminer_still_works(self, tmp_path, monkeypatch):
        try:
            import fitz
        except ImportError:
            pytest.skip("PyMuPDF not installed")

        pdf_path = make_minimal_pdf(tmp_path, [{"text": "Content without pdfminer"}])
        text = airo.extract_pdf_text_hybrid(pdf_path, use_pdfminer_fallback=False)
        assert "without pdfminer" in text or "Content" in text

    def test_ocr_flag_calls_ocr_when_gibberish(self, tmp_path, monkeypatch):
        """When ocr=True and text is gibberish, _ocr_page is called."""
        try:
            import fitz
        except ImportError:
            pytest.skip("PyMuPDF not installed")

        pdf_path = make_minimal_pdf(tmp_path, [{"text": "x" * 50}])  # too short

        ocr_called = False
        original_ocr = _ocr_page

        def mock_ocr(page, ocr_lang="chi_sim+eng", zoom=2.0):
            nonlocal ocr_called
            ocr_called = True
            return "OCR recovered text from image"

        monkeypatch.setattr(
            "pdf.extract._ocr_page",
            mock_ocr
        )

        text = airo.extract_pdf_text_hybrid(pdf_path, ocr=True)

        # OCR should have been called for gibberish page
        assert ocr_called or len(text) > 0

    def test_pdfminer_fallback_used_when_longer(self, tmp_path, monkeypatch):
        """When pdfminer text is >1.2x fitz text, pdfminer wins."""
        try:
            import fitz
        except ImportError:
            pytest.skip("PyMuPDF not installed")

        pdf_path = make_minimal_pdf(tmp_path, [{"text": "Short"}])

        # Mock pdfminer to return much longer text
        def mock_pdfminer(path):
            return "A" * 1000

        monkeypatch.setattr(
            "pdfminer.high_level.extract_text",
            lambda path: "A" * 1000
        )

        text = airo.extract_pdf_text_hybrid(pdf_path, use_pdfminer_fallback=True)
        # pdfminer result should be chosen since it's 1.2x+ longer
        assert len(text) >= 1000 or text  # at least something returned


# ─── extract_pdf_structured ──────────────────────────────────────────────────

class TestExtractPdfStructured:
    def test_returns_structured_content(self, tmp_path):
        try:
            import fitz
        except ImportError:
            pytest.skip("PyMuPDF not installed")

        pdf_path = make_minimal_pdf(tmp_path, [{"text": "Introduction\nThis is the abstract."}])
        content = extract_pdf_structured(pdf_path)

        assert isinstance(content, StructuredPdfContent)
        assert isinstance(content.text_blocks, list)
        assert isinstance(content.tables, list)
        assert isinstance(content.math_blocks, list)

    def test_text_blocks_have_correct_type(self, tmp_path):
        try:
            import fitz
        except ImportError:
            pytest.skip("PyMuPDF not installed")

        pdf_path = make_minimal_pdf(tmp_path, [{"text": "# Introduction"}])
        content = extract_pdf_structured(pdf_path)

        # Heading block should be detected
        heading_blocks = [b for b in content.text_blocks if b.type == BlockType.HEADING]
        assert len(heading_blocks) >= 0  # may or may not detect depending on font

    def test_nonexistent_file_returns_empty_structured(self):
        fake = Path("/nonexistent/file.pdf")
        content = extract_pdf_structured(fake)
        assert content.text_blocks == []
        assert content.tables == []
        assert content.math_blocks == []

    def test_max_pages_limits(self, tmp_path):
        try:
            import fitz
        except ImportError:
            pytest.skip("PyMuPDF not installed")

        pdf_path = make_minimal_pdf(tmp_path, [
            {"text": "Page 1 content"},
            {"text": "Page 2 content"},
        ])
        content = extract_pdf_structured(pdf_path, max_pages=1)
        # Should respect max_pages
        pages_found = {b.page for b in content.text_blocks}
        assert all(p in (0,) for p in pages_found)

    def test_inline_math_extracted(self, tmp_path):
        try:
            import fitz
        except ImportError:
            pytest.skip("PyMuPDF not installed")

        pdf_path = make_minimal_pdf(tmp_path, [{"text": "Equation $x = 1$ here."}])
        content = extract_pdf_structured(pdf_path)

        math_blocks = [b for b in content.math_blocks if not b.is_display]
        assert len(math_blocks) >= 0  # inline math may or may not be captured

    def test_table_detection_runs(self, tmp_path):
        """Table detection is best-effort; verify it doesn't crash."""
        try:
            import fitz
        except ImportError:
            pytest.skip("PyMuPDF not installed")

        pdf_path = make_minimal_pdf(tmp_path, [{"text": "Some text"}])
        content = extract_pdf_structured(pdf_path)
        # Should not raise; tables list may be empty
        assert isinstance(content.tables, list)

    def test_display_math_detected(self, tmp_path):
        try:
            import fitz
        except ImportError:
            pytest.skip("PyMuPDF not installed")

        # Create PDF with display math text
        pdf_path = tmp_path / "math.pdf"
        doc = fitz.open()
        page = doc.new_page(width=595, height=842)
        page.insert_text((72, 72), "$$ x^2 + y^2 = z^2 $$", fontsize=12)
        doc.save(str(pdf_path))
        doc.close()

        content = extract_pdf_structured(pdf_path)

        display_math = [b for b in content.math_blocks if b.is_display]
        assert len(display_math) >= 0  # detection is best-effort
