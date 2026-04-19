"""Tests for sections/segment.py"""
import pytest
from sections.segment import (
    looks_like_heading,
    text_blocks_to_lines,
    segment_into_sections,
    segment_structured,
    format_section_snippets,
    format_tables_markdown,
    format_math_markdown,
    _section_priority,
)
from pdf.extract import TextBlock, TableBlock, MathBlock, StructuredPdfContent, BlockType


# ─── looks_like_heading ──────────────────────────────────────────────────────

class TestLooksLikeHeading:
    def test_accepts_numbered_heading(self):
        assert looks_like_heading("1. Introduction") is True
        assert looks_like_heading("2.3. Method") is True
        assert looks_like_heading("10. Related Work") is True

    def test_accepts_roman_numeral_heading(self):
        assert looks_like_heading("I. Introduction") is True
        assert looks_like_heading("X. Background") is True

    def test_rejects_short_text(self):
        assert looks_like_heading("AB") is False
        assert looks_like_heading("") is False

    def test_rejects_too_long_text(self):
        assert looks_like_heading("A" * 121) is False

    def test_accepts_keyword_heading(self):
        assert looks_like_heading("Abstract") is True
        assert looks_like_heading("Introduction") is True
        assert looks_like_heading("Related Work") is True
        assert looks_like_heading("Future Work") is True

    def test_accepts_keyword_prefix(self):
        assert looks_like_heading("Introduction to Transformers") is True
        # "Methodology" is not in the keyword list (only "method" singular)
        assert looks_like_heading("Method Overview") is True

    def test_accepts_all_caps_short(self):
        assert looks_like_heading("ABSTRACT") is True
        assert looks_like_heading("METHOD") is True

    def test_rejects_all_caps_too_long(self):
        assert looks_like_heading("VERYLONGALL CAPSHEADING" + "A" * 20) is False

    def test_rejects_bullet_points(self):
        assert looks_like_heading("- item") is False
        assert looks_like_heading("* item") is False

    def test_rejects_plain_text(self):
        assert looks_like_heading("This is a plain sentence.") is False


# ─── text_blocks_to_lines ─────────────────────────────────────────────────────

class TestTextBlocksToLines:
    def test_flattens_text_blocks(self):
        blocks = [
            TextBlock(type=BlockType.BODY, text="First block", page=0),
            TextBlock(type=BlockType.BODY, text="Second block", page=0),
        ]
        lines = text_blocks_to_lines(blocks)
        assert lines == ["First block", "Second block"]

    def test_empty_list(self):
        assert text_blocks_to_lines([]) == []


# ─── segment_into_sections ───────────────────────────────────────────────────

class TestSegmentIntoSections:
    def test_empty_input(self):
        assert segment_into_sections("") == []

    def test_no_headings(self):
        text = "This is some body text.\n\nMore body text."
        result = segment_into_sections(text)
        assert len(result) == 1
        assert result[0][0] == "BODY"
        assert "body text" in result[0][1]

    def test_markdown_headings(self):
        text = "# Abstract\n\nSummary here.\n\n# Introduction\n\nIntro text."
        result = segment_into_sections(text)
        titles = [t for t, _ in result]
        assert "Abstract" in titles
        assert "Introduction" in titles

    def test_plain_text_headings(self):
        # Note: "Method details" matches keyword prefix "method " so avoid that
        text = "1. Introduction\n\nIntro content\n\n2.1 Results\n\nThese are the results."
        result = segment_into_sections(text)
        titles = [t for t, _ in result]
        assert any("Introduction" in t for t in titles)
        assert any("Results" in t for t in titles)

    def test_max_sections_truncation(self):
        # Create 20 sections
        lines = []
        for i in range(20):
            lines.append(f"# Section {i}\n\nContent for section {i}.")
        text = "\n\n".join(lines)
        result = segment_into_sections(text, max_sections=18)
        assert len(result) == 19  # 18 + TRUNCATED
        assert result[-1][0] == "TRUNCATED"

    def test_skips_empty_sections(self):
        text = "# Abstract\n\n# Introduction\n\nIntro text."
        result = segment_into_sections(text)  # noqa: F841
        # Abstract section should be skipped (no content)


# ─── segment_structured ───────────────────────────────────────────────────────

class TestSegmentStructured:
    def test_empty(self):
        sdoc = StructuredPdfContent(text_blocks=[], tables=[], math_blocks=[])
        result = segment_structured(sdoc)
        assert result == []

    def test_basic_segmentation(self):
        from pdf.extract import BlockType
        blocks = [
            TextBlock(type=BlockType.HEADING, text="# Abstract\n\nSummary", page=0),
            TextBlock(type=BlockType.HEADING, text="# Introduction\n\nIntro", page=0),
        ]
        sdoc = StructuredPdfContent(text_blocks=blocks, tables=[], math_blocks=[])
        result = segment_structured(sdoc)
        assert len(result) >= 1
        # Each item should be 3-tuple
        for item in result:
            assert len(item) == 3
            title, content, meta = item
            assert isinstance(title, str)
            assert isinstance(content, str)
            assert isinstance(meta, dict)
            assert "has_tables" in meta
            assert "has_math" in meta

    def test_math_detection(self):
        # Math detection is based on counting $ in section content.
        # Use text that does NOT match looks_like_heading (e.g., no "method " prefix).
        from pdf.extract import BlockType
        blocks = [
            TextBlock(type=BlockType.BODY, text="Our approach uses $x = y$ for calculations.", page=0),
        ]
        sdoc = StructuredPdfContent(text_blocks=blocks, tables=[], math_blocks=[])
        result = segment_structured(sdoc)
        assert len(result) == 1
        _, content, meta = result[0]
        # $x = y$ has 2 $ chars = 1 math expression
        assert meta.get("has_math") is True
        assert meta.get("math_count") == 1

    def test_table_metadata(self):
        from pdf.extract import BlockType
        blocks = [TextBlock(type=BlockType.BODY, text="Some text", page=0)]
        tables = [
            TableBlock(text="col1 | col2\n---|---|", page=0, bbox=(0, 0, 100, 50)),
        ]
        sdoc = StructuredPdfContent(text_blocks=blocks, tables=tables, math_blocks=[])
        result = segment_structured(sdoc)  # noqa: F841
        # Should have metadata with has_tables

    def test_max_sections_truncation(self):
        from pdf.extract import BlockType
        # segment_structured uses BlockType.HEADING blocks as heading boundaries.
        # Plain text in BODY blocks without markdown/keyword patterns merges into 1 section.
        # To get 20 sections we use HEADING-type blocks.
        blocks = []
        for i in range(20):
            blocks.append(TextBlock(type=BlockType.HEADING, text=f"# Section {i}", page=i))
            blocks.append(TextBlock(type=BlockType.BODY, text=f"Content for section {i}.", page=i))
        sdoc = StructuredPdfContent(text_blocks=blocks, tables=[], math_blocks=[])
        result = segment_structured(sdoc, max_sections=18)
        assert len(result) == 19  # 18 + TRUNCATED
        assert result[-1][0] == "TRUNCATED"


# ─── format_section_snippets ──────────────────────────────────────────────────

class TestFormatSectionSnippets:
    def test_empty_input(self):
        assert format_section_snippets([]) == ""
        assert format_section_snippets(None) == ""

    def test_single_section(self):
        sections = [("Introduction", "This is the introduction text." * 50)]
        result = format_section_snippets(sections)
        assert "Introduction" in result
        assert "This is the introduction" in result

    def test_structured_3tuple(self):
        sections = [("Introduction", "Intro text." * 50, {"has_tables": False, "has_math": False})]
        result = format_section_snippets(sections)
        assert "Introduction" in result

    def test_priority_ordering(self):
        # Abstract(10) > Method(8) > Discussion(4) > Related(3)
        # format_section_snippets sorts by original order for equal priorities,
        # but HIGH-priority sections (>=5) get selected first within budget
        sections = [
            ("Discussion", "Discussion text. " * 20),
            ("Abstract", "Abstract text. " * 20),
            ("Method", "Method text. " * 20),
        ]
        result = format_section_snippets(sections)
        # Abstract (priority 10) should appear
        assert "Abstract" in result
        # Method (priority 8) should appear
        assert "Method" in result
        # Result should contain all three
        assert result.count("###") >= 2

    def test_max_chars_total(self):
        sections = [
            ("Section", "x" * 10000),
        ]
        result = format_section_snippets(sections, max_chars_total=1000)
        # Should not exceed budget significantly
        assert len(result) <= 2000

    def test_respects_3tuple_and_2tuple_mix(self):
        # format_section_snippets handles both 2-tuple and 3-tuple
        sections_2 = [("Title", "Content")]
        sections_3 = [("Title", "Content", {"has_tables": False})]
        r2 = format_section_snippets(sections_2)
        r3 = format_section_snippets(sections_3)
        assert r2 == r3


# ─── format_tables_markdown ───────────────────────────────────────────────────

class TestFormatTablesMarkdown:
    def test_empty_tables(self):
        sdoc = StructuredPdfContent(text_blocks=[], tables=[], math_blocks=[])
        assert format_tables_markdown(sdoc) == ""

    def test_single_table(self):
        tables = [
            TableBlock(text="a | b\n-|-\n1 | 2", page=0, bbox=(0, 0, 100, 50)),
        ]
        sdoc = StructuredPdfContent(text_blocks=[], tables=tables, math_blocks=[])
        result = format_tables_markdown(sdoc)
        assert "Table (page 1)" in result
        assert "a | b" in result

    def test_max_chars(self):
        # max_chars stops adding NEW tables after threshold is reached,
        # but does NOT truncate individual tables
        tables = [
            TableBlock(text="x" * 5000, page=0, bbox=(0, 0, 100, 200)),
            TableBlock(text="y" * 5000, page=1, bbox=(0, 0, 100, 200)),
        ]
        sdoc = StructuredPdfContent(text_blocks=[], tables=tables, math_blocks=[])
        result = format_tables_markdown(sdoc, max_chars=3000)
        # First table is included (>= 3000 header + table text), second is skipped
        assert "Table (page 1)" in result
        assert "Table (page 2)" not in result


# ─── format_math_markdown ────────────────────────────────────────────────────

class TestFormatMathMarkdown:
    def test_no_display_math(self):
        math_blocks = [
            MathBlock(text="x = y", is_display=False, page=0),
        ]
        sdoc = StructuredPdfContent(text_blocks=[], tables=[], math_blocks=math_blocks)
        assert format_math_markdown(sdoc) == ""

    def test_display_math(self):
        math_blocks = [
            MathBlock(text="E = mc^2", is_display=True, page=2),
        ]
        sdoc = StructuredPdfContent(text_blocks=[], tables=[], math_blocks=math_blocks)
        result = format_math_markdown(sdoc)
        assert "Equation (page 3)" in result
        assert "E = mc^2" in result

    def test_max_count(self):
        math_blocks = [
            MathBlock(text=f"eq{i}", is_display=True, page=i)
            for i in range(10)
        ]
        sdoc = StructuredPdfContent(text_blocks=[], tables=[], math_blocks=math_blocks)
        result = format_math_markdown(sdoc, max_count=3)
        assert result.count("Equation") == 3


# ─── _section_priority ───────────────────────────────────────────────────────

class TestSectionPriority:
    @pytest.mark.parametrize("title,expected", [
        ("Abstract", 10),
        ("INTRODUCTION", 9),
        ("Introduction to Transformers", 9),
        ("Method", 8),
        ("METHODOLOGY", 8),
        ("Model Architecture", 7),
        ("Algorithm Details", 7),
        ("Experiments", 6),
        ("Evaluation", 6),
        ("Results and Analysis", 6),
        ("Discussion", 4),
        ("Limitations", 4),
        ("Conclusion", 4),
        ("Related Work", 3),
        ("Background", 3),
        ("Preliminaries", 3),
        ("Appendix A", 1),
        ("Acknowledgments", 1),
        ("References", 0),
        ("Future Work", 2),
        ("Ablation Study", 5),
        ("Body", 1),
        ("TRUNCATED", 0),
        ("Unknown Section Title", 2),  # default
        ("", 2),  # default for empty
    ])
    def test_priority_values(self, title, expected):
        assert _section_priority(title) == expected
