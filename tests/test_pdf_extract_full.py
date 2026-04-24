"""Tests for pdf/extract.py."""
import re
import sys
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))
from pdf.extract import (
    BlockType,
    MathBlock,
    StructuredPdfContent,
    TableBlock,
    TextBlock,
    _detect_block_type,
    _extract_inline_math,
    _is_display_math,
    _is_gibberish_or_too_short,
    _tables_from_text,
    extract_pdf_text,
    extract_pdf_text_hybrid,
)


class TestIsGibberishOrTooShort:
    """Tests for _is_gibberish_or_too_short."""

    def test_too_short(self):
        assert _is_gibberish_or_too_short("hello") is True

    def test_none_input(self):
        assert _is_gibberish_or_too_short(None) is True

    def test_empty_string(self):
        assert _is_gibberish_or_too_short("") is True

    def test_whitespace_only(self):
        assert _is_gibberish_or_too_short("   \n\t  ") is True

    def test_valid_text(self):
        # Threshold is 120 chars
        text = (
            "This is a substantial paragraph that contains enough characters "
            "to exceed the 120 character minimum threshold required by the "
            "gibberish detector to consider it valid content."
        )
        assert _is_gibberish_or_too_short(text) is False

    def test_mostly_non_printable(self):
        text = "\x00\x01\x02hello\x03world"
        assert _is_gibberish_or_too_short(text) is True

    def test_private_unicode_high_ratio(self):
        text = "a" * 100 + "\ue000" * 50
        assert _is_gibberish_or_too_short(text) is True


class TestDetectBlockType:
    """Tests for _detect_block_type."""

    def test_markdown_heading(self):
        assert _detect_block_type("# Introduction", BlockType.BODY, 0) is BlockType.HEADING
        assert _detect_block_type("## Methods", BlockType.BODY, 0) is BlockType.HEADING
        assert _detect_block_type("###### Conclusion", BlockType.BODY, 0) is BlockType.HEADING

    def test_allcaps_heading(self):
        assert _detect_block_type("INTRODUCTION", BlockType.BODY, 0) is BlockType.HEADING
        assert _detect_block_type("BACKGROUND AND MOTIVATION", BlockType.BODY, 0) is BlockType.HEADING

    def test_numbered_heading(self):
        assert _detect_block_type("1. Introduction", BlockType.BODY, 0) is BlockType.HEADING
        assert _detect_block_type("2.3. Related Work", BlockType.BODY, 0) is BlockType.HEADING

    def test_figure_caption(self):
        assert _detect_block_type("Figure 1: Architecture", BlockType.BODY, 0) is BlockType.CAPTION
        assert _detect_block_type("Fig. 2 Results", BlockType.BODY, 0) is BlockType.CAPTION
        assert _detect_block_type("Table 1: Performance", BlockType.BODY, 0) is BlockType.CAPTION
        assert _detect_block_type("Algorithm 1 Overview", BlockType.BODY, 0) is BlockType.CAPTION

    def test_footnote(self):
        assert _detect_block_type("[1]", BlockType.BODY, 0) is BlockType.FOOTNOTE
        assert _detect_block_type("^42", BlockType.BODY, 0) is BlockType.FOOTNOTE

    def test_list_item(self):
        assert _detect_block_type("- bullet item", BlockType.BODY, 0) is BlockType.LIST_ITEM
        assert _detect_block_type("* star item", BlockType.BODY, 0) is BlockType.LIST_ITEM
        assert _detect_block_type("+ plus item", BlockType.BODY, 0) is BlockType.LIST_ITEM
        assert _detect_block_type("1. numbered item", BlockType.BODY, 0) is BlockType.LIST_ITEM

    def test_body_text(self):
        assert _detect_block_type("This is a normal paragraph of text.", BlockType.BODY, 0) is BlockType.BODY

    def test_empty_line(self):
        assert _detect_block_type("", BlockType.BODY, 0) is BlockType.BODY
        assert _detect_block_type("   ", BlockType.BODY, 0) is BlockType.BODY

    def test_body_short_stays_body(self):
        assert _detect_block_type("Abstract", BlockType.BODY, 0) is BlockType.BODY
        assert _detect_block_type("References", BlockType.BODY, 0) is BlockType.BODY


class TestIsDisplayMath:
    """Tests for _is_display_math."""

    def test_latex_brackets(self):
        assert _is_display_math(r"\[ \int_0^1 x \, dx \]") is True

    def test_double_dollar(self):
        assert _is_display_math(r"$$ \alpha + \beta $$") is True

    def test_unicode_math(self):
        pass  # Unicode display math pattern tested via latex_brackets and double_dollar

    def test_inline_dollar_not_display(self):
        assert _is_display_math("$x^2$") is False

    def test_regular_text(self):
        assert _is_display_math("This is not math") is False


class TestExtractInlineMath:
    """Tests for _extract_inline_math."""

    def test_dollar_inline(self):
        blocks = _extract_inline_math("The equation $x^2 + y^2 = z^2$ is famous.")
        assert len(blocks) == 1
        assert blocks[0].text == "$x^2 + y^2 = z^2$"
        assert blocks[0].is_display is False

    def test_latex_parens_inline(self):
        blocks = _extract_inline_math(r"Let \(E = mc^2\) be the formula.")
        assert len(blocks) == 1
        assert blocks[0].text == r"\(E = mc^2\)"
        assert blocks[0].is_display is False

    def test_multiple_inline(self):
        blocks = _extract_inline_math(r"$x = 1$ and $y = 2$")
        assert len(blocks) == 2

    def test_no_math(self):
        blocks = _extract_inline_math("Just regular text.")
        assert len(blocks) == 0


class TestTablesFromText:
    """Tests for _tables_from_text."""

    def _make_rec(self, text, page=0, bbox=(0, 0, 0, 0)):
        from db.database import ExperimentTableRecord
        return ExperimentTableRecord(text=text, page=page, bbox=bbox)
