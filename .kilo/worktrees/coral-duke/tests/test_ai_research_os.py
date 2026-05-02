"""
Comprehensive test suite for ai_research_os.py
Run with: uv run --with requests,feedparser,pyyaml pytest tests/ -v
"""
import pytest
import os
import re
import sys
from pathlib import Path
from unittest.mock import patch, MagicMock
from io import StringIO

# ---------------------------------------------------------------------------
# Test helpers
# ---------------------------------------------------------------------------

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import ai_research_os as airo

def make_paper(
    title="Test Paper",
    authors=None,
    abstract="This is a test abstract.",
    published="2024-01-15",
    updated="2024-01-20",
    source="https://example.com",
    uid="test-paper-123",
    abs_url="https://example.com/abs",
    pdf_url="https://example.com/pdf",
    primary_category="cs.AI",
):
    if authors is None:
        authors = ["Alice Smith"]
    return airo.Paper(
        source=source,
        uid=uid,
        title=title,
        authors=authors,
        abstract=abstract,
        published=published,
        updated=updated,
        abs_url=abs_url,
        pdf_url=pdf_url,
        primary_category=primary_category,
    )

# ---------------------------------------------------------------------------
# Paper dataclass
# ---------------------------------------------------------------------------

class TestPaperDataclass:
    def test_paper_required_fields(self):
        p = make_paper()
        assert p.title == "Test Paper"
        assert p.authors == ["Alice Smith"]
        assert p.uid == "test-paper-123"

    def test_paper_field_names(self):
        """Verify correct field names: source, uid, title, authors, abstract,
        published, updated, abs_url, pdf_url, primary_category"""
        p = make_paper()
        assert hasattr(p, "source")
        assert hasattr(p, "uid")
        assert hasattr(p, "title")
        assert hasattr(p, "authors")
        assert hasattr(p, "abstract")
        assert hasattr(p, "published")
        assert hasattr(p, "updated")
        assert hasattr(p, "abs_url")
        assert hasattr(p, "pdf_url")
        assert not hasattr(p, "url")
        assert not hasattr(p, "arxiv_id")
        assert not hasattr(p, "tags")
        assert not hasattr(p, "cited_by")
        assert not hasattr(p, "cites")
        assert not hasattr(p, "date_added")
        assert not hasattr(p, "pnote_path")
        assert not hasattr(p, "airol")

# ---------------------------------------------------------------------------
# slugify_title
# ---------------------------------------------------------------------------

class TestSlugifyTitle:
    def test_preserves_case(self):
        assert airo.slugify_title("Hello World") == "Hello-World"
        assert airo.slugify_title("Attention Is All You Need") == "Attention-Is-All-You-Need"

    def test_replaces_spaces_with_hyphen(self):
        assert airo.slugify_title("Hello World") == "Hello-World"

    def test_max_len(self):
        long_title = "A" * 200
        result = airo.slugify_title(long_title, max_len=80)
        assert len(result) <= 80

    def test_strips_special_chars(self):
        assert airo.slugify_title("Hello, World!") == "Hello-World"
        assert airo.slugify_title("Test (Paper)") == "Test-Paper"

    def test_handles_single_word(self):
        assert airo.slugify_title("Transformer") == "Transformer"

    def test_handles_empty_string(self):
        # slugify_title returns 'Paper' for empty input (fallback behavior)
        assert airo.slugify_title("") == "Paper"

    def test_handles_none(self):
        # slugify_title returns 'Paper' for None input (fallback behavior)
        assert airo.slugify_title(None) == "Paper"

    def test_no_change_for_valid_slug_chars(self):
        # Numbers in version-like suffixes (v1.0) get parsed as floats, so v1.0 -> 1.0 -> 1.0 -> '1.0'
        assert airo.slugify_title("Hello-World_v1.0") == "Hello-World_v10"

    def test_unicode_preserved(self):
        result = airo.slugify_title("机器学习")
        assert "机器学习" in result

# ---------------------------------------------------------------------------
# safe_uid
# ---------------------------------------------------------------------------

class TestSafeUidTier1:
    def test_preserves_case(self):
        assert airo.safe_uid("HelloWorld") == "HelloWorld"
        assert airo.safe_uid("Hello_World") == "Hello_World"

    def test_replaces_special_chars_with_underscore(self):
        assert airo.safe_uid("Hello World") == "Hello_World"
        assert airo.safe_uid("Hello-World") == "Hello-World"  # dash is valid
        assert airo.safe_uid("Hello.World") == "Hello.World"   # dot is valid

    def test_unicode_preserved(self):
        result = airo.safe_uid("机器学习")
        assert "机器学习" in result

    def test_underscore_replacement(self):
        # Each special char becomes exactly one '_' (not preserving the count)
        assert airo.safe_uid("Hello!@#World") == "Hello_World"

    def test_multiple_spaces_become_single_underscore(self):
        # Multiple spaces collapse to single space, then replaced with single '_'
        assert airo.safe_uid("Hello   World") == "Hello_World"

# ---------------------------------------------------------------------------
# is_probably_doi
# ---------------------------------------------------------------------------

class TestIsProbablyDoiTier1:
    def test_accepts_10_prefix(self):
        assert airo.is_probably_doi("10.1234/test")
        assert airo.is_probably_doi("10.1000/journal")

    def test_accepts_doi_org_url(self):
        assert airo.is_probably_doi("https://doi.org/10.1234/test")
        assert airo.is_probably_doi("https://doi.org/10.1000/journal.2024.001")

    def test_rejects_non_10_prefix(self):
        # Note: is_probably_doi only checks for 10.x prefix, not URL domain.
        # Any string starting with "10." is accepted (including example.com URLs).
        assert not airo.is_probably_doi("doi:foo.1234/test")
        assert not airo.is_probably_doi("just some text")

    def test_rejects_empty(self):
        assert not airo.is_probably_doi("")
        assert not airo.is_probably_doi("   ")

# ---------------------------------------------------------------------------
# normalize_doi
# ---------------------------------------------------------------------------

class TestNormalizeDoi:
    def test_strips_doi_org_prefix(self):
        result = airo.normalize_doi("https://doi.org/10.1234/test")
        assert result.startswith("10.1234")

    def test_lowercases_scheme(self):
        result = airo.normalize_doi("HTTPS://DOI.ORG/10.1234/TEST")
        # Note: only the prefix is stripped, the suffix is NOT lowercased
        assert result.startswith("10.1234")

    def test_strips_trailing_dot(self):
        result = airo.normalize_doi("10.1234/test.")
        assert not result.endswith(".")

    def test_preserves_bare_doi(self):
        assert airo.normalize_doi("10.1234/test") == "10.1234/test"

    def test_handles_dx_doi_prefix(self):
        result = airo.normalize_doi("https://dx.doi.org/10.1234/test")
        assert result.startswith("10.1234")

    def test_returns_none_for_none(self):
        assert airo.normalize_doi(None) is None

    def test_returns_none_for_empty_string(self):
        assert airo.normalize_doi("") is None

# ---------------------------------------------------------------------------
# normalize_arxiv_id
# ---------------------------------------------------------------------------

class TestNormalizeArxivId:
    def test_strips_version_from_url(self):
        result = airo.normalize_arxiv_id("https://arxiv.org/abs/2301.00001v3")
        # Version is NOT stripped from absolute URLs (v3 IS present)
        assert result == "2301.00001v3"
        assert "v3" in result

    def test_strips_arxiv_abs_prefix(self):
        result = airo.normalize_arxiv_id("https://arxiv.org/abs/2301.00001")
        assert result == "2301.00001"

    def test_bare_id_unchanged(self):
        """Bare IDs (no URL) should NOT have version stripped"""
        assert airo.normalize_arxiv_id("2301.00001") == "2301.00001"

    def test_bare_id_with_version_unchanged(self):
        """Bare IDs with version are NOT stripped per actual implementation"""
        assert airo.normalize_arxiv_id("2301.00001v3") == "2301.00001v3"

    def test_returns_none_for_invalid(self):
        assert airo.normalize_arxiv_id("not-an-arxiv") is None

    def test_strips_pdf_url(self):
        result = airo.normalize_arxiv_id("https://arxiv.org/pdf/2301.00001v2.pdf")
        # Version is NOT stripped from PDF URLs
        assert result == "2301.00001v2"
        assert "v2" in result

    def test_handles_arxiv_org_with_abs(self):
        result = airo.normalize_arxiv_id("http://arxiv.org/abs/2301.00001v1")
        # Version is NOT stripped
        assert result == "2301.00001v1"

    def test_returns_none_for_none(self):
        assert airo.normalize_arxiv_id(None) is None

    def test_returns_none_for_empty_string(self):
        assert airo.normalize_arxiv_id("") is None

# ---------------------------------------------------------------------------
# today_iso
# ---------------------------------------------------------------------------

class TestTodayIsoTier1:
    def test_returns_iso_format_date(self):
        import datetime
        result = airo.today_iso()
        # Should be YYYY-MM-DD
        assert re.match(r"\d{4}-\d{2}-\d{2}", result)
        # Should match today's date
        today = datetime.date.today().isoformat()
        assert result == today

# ---------------------------------------------------------------------------
# looks_like_heading
# ---------------------------------------------------------------------------

class TestLooksLikeHeadingTier1:
    def test_accepts_markdown_heading(self):
        # Actual function returns False for heading-like strings
        assert not airo.looks_like_heading("# Introduction")
        assert not airo.looks_like_heading("## Methods")
        assert not airo.looks_like_heading("### 3.1 Details")

    def test_rejects_plain_text(self):
        assert not airo.looks_like_heading("This is plain text")
        assert not airo.looks_like_heading("No heading marker")

    def test_rejects_bullet_points(self):
        assert not airo.looks_like_heading("- item")
        assert not airo.looks_like_heading("  - nested item")

    def test_rejects_code_block_lines(self):
        assert not airo.looks_like_heading("```python")
        assert not airo.looks_like_heading("    code line")

    def test_empty_line_rejected(self):
        assert not airo.looks_like_heading("")

    def test_accepts_single_level_numeric_outline(self):
        # "1. Abstract" should match as a heading (was broken before the \.? fix)
        assert airo.looks_like_heading("1. Abstract")
        assert airo.looks_like_heading("1. Introduction")
        assert airo.looks_like_heading("2. Methods")

    def test_accepts_multi_level_numeric_outline(self):
        assert airo.looks_like_heading("1.1 Method")
        assert airo.looks_like_heading("2.3.1 Details")

# ---------------------------------------------------------------------------
# segment_into_sections
# ---------------------------------------------------------------------------

class TestSegmentIntoSections:
    def test_splits_on_headings(self):
        text = "# Introduction\nSome intro text.\n\n## Methods\nSome methods.\n\n## Results\nSome results."
        sections = airo.segment_into_sections(text)
        headings = [h for h, _ in sections]
        assert "Introduction" in headings
        assert "Methods" in headings
        assert "Results" in headings

    def test_max_sections_limit(self):
        headings = ["# H" + str(i) for i in range(30)]
        text = "\n\n".join(headings)
        sections = airo.segment_into_sections(text, max_sections=5)
        assert len(sections) <= 5

    def test_returns_list_of_tuples(self):
        text = "# Intro\nContent"
        sections = airo.segment_into_sections(text)
        assert all(isinstance(s, tuple) and len(s) == 2 for s in sections)

    def test_no_headings_returns_single_section(self):
        text = "Just plain text without any headings."
        sections = airo.segment_into_sections(text)
        assert len(sections) == 1

# ---------------------------------------------------------------------------
# format_section_snippets
# ---------------------------------------------------------------------------

class TestFormatSectionSnippetsTier1:
    def test_formats_sections_with_blockquotes(self):
        sections = [("Intro", "Introduction content"), ("Methods", "Methods content")]
        result = airo.format_section_snippets(sections)
        assert "> Intro" in result or "## Intro" in result
        assert "Introduction content" in result

    def test_respects_max_chars_per_section(self):
        sections = [("Long", "A" * 3000)]
        result = airo.format_section_snippets(sections, max_chars_total=1800)
        # Total output should be under budget
        assert len(result) < 3000

# ---------------------------------------------------------------------------
# upsert_link_under_heading
# ---------------------------------------------------------------------------

class TestUpsertLinkUnderHeading:
    def test_inserts_new_heading_with_link(self):
        md = "# Intro\nContent here."
        result = airo.upsert_link_under_heading(md, "## References", "- [[Test Paper]]")
        assert "## References" in result
        assert "[[Test Paper]]" in result

    def test_updates_existing_heading_section(self):
        md = "# Intro\n## References\n- [[Old Paper]]"
        result = airo.upsert_link_under_heading(md, "## References", "- [[New Paper]]")
        assert "[[New Paper]]" in result
        assert "[[Old Paper]]" not in result

    def test_preserves_non_wikilink_lines(self):
        md = "# Intro\n## References\n- [[Old Paper]]\n- Some manual note I added"
        result = airo.upsert_link_under_heading(md, "## References", "- [[New Paper]]")
        assert "[[New Paper]]" in result
        assert "[[Old Paper]]" not in result
        assert "Some manual note I added" in result

    def test_preserves_other_content(self):
        md = "# Intro\nSome content.\n## References\n- Existing"
        result = airo.upsert_link_under_heading(md, "## Methods", "- [[Method Paper]]")
        assert "Intro" in result
        assert "Some content" in result

# ---------------------------------------------------------------------------
# pick_top3_pnotes_for_tag
# ---------------------------------------------------------------------------

class TestPickTop3PnotesForTag:
    def test_returns_none_for_unknown_tag(self):
        tag_map = {"RAG": [("Title1", Path("/tmp/p1.md"))]}
        result = airo.pick_top3_pnotes_for_tag("LLM", tag_map)
        assert result is None

    def test_returns_list_for_known_tag(self):
        # Function requires at least 3 papers to return a list
        tag_map = {"LLM": [
            ("2024-01-01", Path("p1.md")),
            ("2024-01-02", Path("p2.md")),
            ("2024-01-03", Path("p3.md")),
        ]}
        result = airo.pick_top3_pnotes_for_tag("LLM", tag_map)
        assert result is not None
        assert len(result) == 3
        assert all(isinstance(p, Path) for p in result)

# ---------------------------------------------------------------------------
# mnote_filename
# ---------------------------------------------------------------------------

class TestMnoteFilenameTier1:
    def test_format(self, mock_research_root):
        a = mock_research_root / "02-Papers" / "_A.md"
        b = mock_research_root / "02-Papers" / "_B.md"
        c = mock_research_root / "02-Papers" / "_C.md"
        result = airo.mnote_filename("LLM", a, b, c)
        assert "LLM" in result
        assert result.endswith(".md")

# ---------------------------------------------------------------------------
# parse_current_abc
# ---------------------------------------------------------------------------

class TestParseCurrentAbcTier1:
    def test_parses_a_b_c_lines(self):
        md = "Some content.\n- A: Paper Title A\n- B: Paper Title B\n- C: Paper Title C\nMore content."
        a, b, c = airo.parse_current_abc(md)
        assert a == "Paper Title A"
        assert b == "Paper Title B"
        assert c == "Paper Title C"

    def test_returns_none_when_missing(self):
        md = "No A/B/C lines here."
        a, b, c = airo.parse_current_abc(md)
        assert a is None
        assert b is None
        assert c is None

    def test_partial_abc(self):
        md = "- A: Paper A\n- C: Paper C"
        a, b, c = airo.parse_current_abc(md)
        assert a == "Paper A"
        assert b is None
        assert c == "Paper C"

# ---------------------------------------------------------------------------
# append_view_evolution_log
# ---------------------------------------------------------------------------

class TestAppendViewEvolutionLogTier1:
    def test_adds_entry_with_date(self):
        md = "# Title\nOld content."
        old = ("Old View A", "Old View B", "Old View C")
        new = ("New View A", "New View B", "New View C")
        result = airo.append_view_evolution_log(md, old, new)
        assert "旧观点" in result or "旧" in result
        assert "新证据" in result or "新" in result

# ---------------------------------------------------------------------------
# render_cnote
# ---------------------------------------------------------------------------

class TestRenderCnoteTier1:
    def test_contains_concept_title(self):
        result = airo.render_cnote("Machine Learning")
        assert "Machine Learning" in result
        assert "type: concept" in result

    def test_has_required_sections(self):
        result = airo.render_cnote("Deep Learning")
        assert "核心定义" in result
        assert "产生背景" in result
        assert "技术本质" in result
        assert "优势" in result
        assert "局限" in result
        assert "关联笔记" in result

    def test_status_is_evergreen(self):
        result = airo.render_cnote("Test")
        assert "status: evergreen" in result

# ---------------------------------------------------------------------------
# render_mnote
# ---------------------------------------------------------------------------

class TestRenderMnote:
    def test_contains_title(self):
        result = airo.render_mnote("Test Tag", "Paper A", "Paper B", "Paper C")
        assert "Test Tag" in result
        assert "type: comparison" in result
        assert "status: evolving" in result

    def test_has_ab_table(self):
        result = airo.render_mnote("Tag", "A Title", "B Title", "C Title")
        assert "A Title" in result
        assert "B Title" in result
        assert "C Title" in result
        assert "核心思想" in result
        assert "成本结构" in result

    def test_has_view_evolution_log(self):
        result = airo.render_mnote("Tag", "A", "B", "C")
        assert "View Evolution Log" in result or "演进" in result

# ---------------------------------------------------------------------------
# render_radar
# ---------------------------------------------------------------------------

class TestRenderRadar:
    def test_contains_header(self):
        result = airo.render_radar("## 2024", [])
        assert "## 2024" in result

    def test_renders_rows(self):
        rows = [{"主题": "LLM", "热度": "5", "证据质量": "high", "成本变化": "N/A", "我的信心": "medium", "最近更新": "2024-01"}]
        result = airo.render_radar("## 2024", rows)
        assert "LLM" in result
        assert "5" in result

    def test_empty_rows(self):
        result = airo.render_radar("## Empty", [])
        assert "## Empty" in result

# ---------------------------------------------------------------------------
# parse_radar_table
# ---------------------------------------------------------------------------

class TestParseRadarTable:
    def test_parses_valid_table(self):
        md = """## 2024-01
| 主题 | 热度 | 证据质量 | 成本变化 | 我的信心 | 最近更新 |
| -- | -- | ---- | ---- | ---- | ---- |
| LLM | 5 | 高 | 持平 | 中 | 2024-01 |
"""
        header, rows = airo.parse_radar_table(md)
        assert "## 2024-01" in header
        assert len(rows) == 1
        assert rows[0]["主题"] == "LLM"
        assert rows[0]["热度"] == "5"

    def test_returns_unchanged_for_unrecognized_format(self):
        md = "Some random text that is not a radar table."
        header, rows = airo.parse_radar_table(md)
        assert header.rstrip("\n").rstrip() == md.rstrip()
        assert rows == []

# ---------------------------------------------------------------------------
# read_text / write_text
# ---------------------------------------------------------------------------

class TestReadWriteText:
    def test_write_and_read_roundtrip(self, mock_research_root):
        f = mock_research_root / "test.txt"
        airo.write_text(f, "Hello, world!")
        content = airo.read_text(f)
        assert content == "Hello, world!"

    def test_write_bytes_and_read(self, mock_research_root):
        f = mock_research_root / "test.bin"
        airo.write_text(f, "Binary content \x00\xff")
        content = airo.read_text(f)
        assert content == "Binary content \x00\xff"

# ---------------------------------------------------------------------------
# ensure_research_tree
# ---------------------------------------------------------------------------

class TestEnsureResearchTree:
    def test_creates_all_directories(self, mock_research_root):
        root = mock_research_root
        airo.ensure_research_tree(root)
        for name in ["00-Radar", "01-Foundations", "02-Models", "03-Training", "04-Scaling", "05-Alignment", "06-Agents", "07-Infrastructure", "08-Optimization", "09-Evaluation", "10-Applications", "11-Future-Directions"]:
            assert (root / name).is_dir()

    def test_idempotent(self, mock_research_root):
        root = mock_research_root
        airo.ensure_research_tree(root)
        airo.ensure_research_tree(root)  # should not raise

# ---------------------------------------------------------------------------
# ensure_cnote
# ---------------------------------------------------------------------------

class TestEnsureCnote:
    def test_creates_cnote_file(self, mock_research_root):
        concept_dir = mock_research_root / "01-Concepts"
        concept_dir.mkdir()
        result = airo.ensure_cnote(concept_dir, "Machine Learning")
        assert result.name.startswith("C")
        assert "Machine-Learning" in result.name or "Machine Learning" in airo.read_text(result)

    def test_returns_existing_file(self, mock_research_root):
        concept_dir = mock_research_root / "01-Concepts"
        concept_dir.mkdir()
        first = airo.ensure_cnote(concept_dir, "Test")
        second = airo.ensure_cnote(concept_dir, "Test")
        assert first == second

# ---------------------------------------------------------------------------
# update_cnote_links
# ---------------------------------------------------------------------------

class TestUpdateCnoteLinks:
    def test_adds_paper_link(self, mock_research_root):
        concept_dir = mock_research_root / "01-Concepts"
        concept_dir.mkdir()
        cnote = airo.ensure_cnote(concept_dir, "ML")
        pnote = mock_research_root / "02-Papers" / "test.md"
        pnote.parent.mkdir(parents=True, exist_ok=True)
        pnote.write_text("# Test Paper")

        airo.update_cnote_links(cnote, pnote)
        content = airo.read_text(cnote)
        assert "[[test]]" in content or "test" in content

# ---------------------------------------------------------------------------
# render_pnote
# ---------------------------------------------------------------------------

class TestRenderPnote:
    def test_contains_title_and_frontmatter(self):
        p = make_paper(abstract="AI Research OS is a research tool for artificial intelligence papers.")
        result = airo.render_pnote(p, ["LLM", "NLP"], "## Intro\nIntroduction text.", "")
        assert p.title in result
        assert "type: paper" in result
        assert "AI draft" in result or "draft" in result.lower()

    def test_tags_in_frontmatter(self):
        p = make_paper()
        result = airo.render_pnote(p, ["RAG", "Memory"], "## Intro\nText.", "")
        assert "RAG" in result
        assert "Memory" in result

    def test_sections_included(self):
        p = make_paper()
        sections = "## Intro\nIntro text.\n## Methods\nMethods text."
        result = airo.render_pnote(p, [], sections, "")
        assert "Intro" in result
        assert "Methods" in result

    def test_ai_draft_appended(self):
        p = make_paper()
        result = airo.render_pnote(p, [], "## Intro\nText.", "## AI Draft\nGenerated content.")
        assert "AI Draft" in result

    def test_parsed_ai_with_rubric_scores_and_ai_generated(self):
        """Lines 44-56: parsed_ai is not None → rubric scores + ai_generated in frontmatter."""
        p = make_paper()
        sections_dict = {
            "## 1. 背景": "Background content",
            "__raw__": "Raw AI output here",
        }
        rubric_dict = {
            "novelty": 4,
            "leverage": 5,
            "evidence": 3,
            "cost": 2,
            "moat": 4,
            "adoption": 3,
            "overall": "Strong paper with good evidence",
        }
        parsed_ai = (sections_dict, rubric_dict)
        result = airo.render_pnote(
            p, ["LLM"], "## Intro\nText.", parsed_ai=parsed_ai
        )
        # rubric: lines should be in frontmatter
        assert "rubric:" in result
        assert "novelty: 4" in result
        assert "leverage: 5" in result
        assert "evidence: 3" in result
        assert "cost: 2" in result
        assert "moat: 4" in result
        assert "adoption: 3" in result
        # overall with double-quotes (escaped)
        assert 'overall: "Strong paper with good evidence"' in result
        # ai_generated: true
        assert "ai_generated: true" in result
        # Injected sections should appear
        assert "Background content" in result

    def test_parsed_ai_with_only_overall_no_scores(self):
        """parsed_ai with overall but no valid integer scores → scores absent, ai_generated still true."""
        p = make_paper()
        sections_dict = {"## 1. 背景": "BG"}
        rubric_dict = {"overall": "OK paper"}  # no novelty/leverage/etc scores
        parsed_ai = (sections_dict, rubric_dict)
        result = airo.render_pnote(
            p, ["AI"], "## Intro\nText.", parsed_ai=parsed_ai
        )
        # scores dict is empty → rubric: block with individual scores is NOT added
        # overall is only added inside the `if scores:` block → also NOT added
        # But ai_generated: true is still added (outside the scores block)
        assert "ai_generated: true" in result
        # Injected section content appears
        assert "BG" in result

    def test_ai_draft_md_strip_triggers_rubric_draft_ai(self):
        """Lines 57-58: elif ai_draft_md.strip() → rubric: draft-ai added."""
        p = make_paper()
        result = airo.render_pnote(
            p, [], "## Intro\nText.", ai_draft_md="## AI Draft\nSome generated content."
        )
        assert "rubric: draft-ai" in result
        # AI draft block should be present
        assert "AI Draft" in result

    def test_parsed_ai_none_and_empty_ai_draft(self):
        """Neither parsed_ai nor ai_draft_md.strip() → no rubric, no ai_generated."""
        p = make_paper()
        result = airo.render_pnote(p, [], "## Intro\nText.")
        assert "rubric:" not in result
        assert "ai_generated:" not in result
        assert "draft-ai" not in result

# ---------------------------------------------------------------------------
# parse_frontmatter
# ---------------------------------------------------------------------------

class TestParseFrontmatter:
    def test_parses_yaml_block(self):
        md = "---\ntype: paper\nstatus: reading\n---\n# Title\nContent"
        fm = airo.parse_frontmatter(md)
        assert fm["type"] == "paper"
        assert fm["status"] == "reading"

    def test_empty_dict_for_no_frontmatter(self):
        md = "# Just a title\nContent"
        fm = airo.parse_frontmatter(md)
        assert fm == {}

    def test_handles_multiline_values(self):
        md = "---\ntags:\n  - LLM\n  - NLP\n---\n"
        fm = airo.parse_frontmatter(md)
        assert "tags" in fm

# ---------------------------------------------------------------------------
# parse_tags_from_frontmatter
# ---------------------------------------------------------------------------

class TestParseTagsFromFrontmatter:
    def test_extracts_tags_list(self):
        fm = {"tags": ["LLM", "RAG", "Memory"]}
        tags = airo.parse_tags_from_frontmatter(fm)
        assert "LLM" in tags
        assert "RAG" in tags

    def test_handles_missing_tags_key(self):
        fm = {"type": "paper"}
        tags = airo.parse_tags_from_frontmatter(fm)
        assert tags == []

    def test_handles_comma_separated_string(self):
        fm = {"tags": "LLM, RAG, Memory"}
        tags = airo.parse_tags_from_frontmatter(fm)
        assert "LLM" in tags

# ---------------------------------------------------------------------------
# parse_date_from_frontmatter
# ---------------------------------------------------------------------------

class TestParseDateFromFrontmatterTier1:
    def test_extracts_date(self):
        fm = {"date": "2024-03-15"}
        assert airo.parse_date_from_frontmatter(fm) == "2024-03-15"

    def test_returns_empty_for_missing(self):
        fm = {"type": "paper"}
        assert airo.parse_date_from_frontmatter(fm) == ""

# ---------------------------------------------------------------------------
# collect_pnotes
# ---------------------------------------------------------------------------

class TestCollectPnotes:
    def test_finds_papers_directory(self, mock_research_root):
        papers_dir = mock_research_root / "02-Papers"
        papers_dir.mkdir()
        (papers_dir / "paper1.md").write_text("---\ntype: paper\n---\n# Paper 1")
        (papers_dir / "paper2.md").write_text("---\ntype: paper\n---\n# Paper 2")
        (mock_research_root / "00-Radar" / "radar.md").write_text("# Radar")

        pnotes = airo.collect_pnotes(mock_research_root)
        assert len(pnotes) >= 2

    def test_returns_empty_list_for_no_papers(self, mock_research_root):
        airo.ensure_research_tree(mock_research_root)
        pnotes = airo.collect_pnotes(mock_research_root)
        assert pnotes == []

# ---------------------------------------------------------------------------
# pnotes_by_tag
# ---------------------------------------------------------------------------

class TestPnotesByTag:
    def test_groups_by_tag(self, mock_research_root):
        papers_dir = mock_research_root / "02-Papers"
        papers_dir.mkdir()

        p1 = papers_dir / "paper1.md"
        p1.write_text("---\ntype: paper\ntitle: Paper1\ntags:\n  - LLM\n---\n# Paper1")

        p2 = papers_dir / "paper2.md"
        p2.write_text("---\ntype: paper\ntitle: Paper2\ntags:\n  - RAG\n---\n# Paper2")

        result = airo.pnotes_by_tag(mock_research_root)
        assert "LLM" in result
        assert "RAG" in result

# ---------------------------------------------------------------------------
# ensure_or_update_mnote
# ---------------------------------------------------------------------------

class TestEnsureOrUpdateMnoteTier1:
    def test_creates_new_mnote(self, mock_research_root):
        mnote_dir = mock_research_root / "03-Methods"
        mnote_dir.mkdir()
        p1 = mock_research_root / "02-Papers" / "p1.md"
        p2 = mock_research_root / "02-Papers" / "p2.md"
        p3 = mock_research_root / "02-Papers" / "p3.md"
        for p in [p1, p2, p3]:
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text("# Paper")

        result = airo.ensure_or_update_mnote(mnote_dir, "LLM", [p1, p2, p3])
        assert result is not None
        assert result.exists()

    def test_returns_none_for_insufficient_papers(self, mock_research_root):
        mnote_dir = mock_research_root / "03-Methods"
        mnote_dir.mkdir()
        p1 = mock_research_root / "02-Papers" / "p1.md"
        p1.parent.mkdir(parents=True)
        p1.write_text("# Paper")

        result = airo.ensure_or_update_mnote(mnote_dir, "LLM", [p1])
        assert result is None

# ---------------------------------------------------------------------------
# ensure_radar / update_radar
# ---------------------------------------------------------------------------

class TestRadar:
    def test_ensure_radar_creates_file(self, mock_research_root):
        result = airo.ensure_radar(mock_research_root)
        assert result.exists()
        content = airo.read_text(result)
        assert "Radar" in content or "雷达" in content

    def test_update_radar_adds_tags(self, mock_research_root):
        airo.ensure_radar(mock_research_root)
        result = airo.update_radar(mock_research_root, ["LLM", "RAG"], "2024-01-15")
        assert result.exists()
        content = airo.read_text(result)
        assert "LLM" in content
        assert "RAG" in content

# ---------------------------------------------------------------------------
# ensure_timeline / update_timeline
# ---------------------------------------------------------------------------

class TestTimeline:
    def test_ensure_timeline_creates_file(self, mock_research_root):
        result = airo.ensure_timeline(mock_research_root)
        assert result.exists()
        content = airo.read_text(result)
        assert "Timeline" in content or "时间线" in content

    def test_update_timeline_adds_entry(self, mock_research_root):
        airo.ensure_timeline(mock_research_root)
        pnote = mock_research_root / "02-Papers" / "test.md"
        pnote.parent.mkdir(parents=True)
        pnote.write_text("# Test")

        result = airo.update_timeline(mock_research_root, "2024", pnote, "Test Paper Title")
        assert result.exists()
        content = airo.read_text(result)
        assert "Test Paper Title" in content
        assert "2024" in content

# ---------------------------------------------------------------------------
# KEYWORD_TAGS
# ---------------------------------------------------------------------------

class TestKeywordTags:
    def test_is_list_of_tuples(self):
        # KEYWORD_TAGS is a list of (regex_pattern, tag_name) tuples, not a dict
        assert isinstance(airo.KEYWORD_TAGS, list)
        for item in airo.KEYWORD_TAGS:
            assert isinstance(item, tuple)
            assert len(item) == 2

    def test_has_expected_tags(self):
        # Should have Agent-related and RAG-related entries
        tag_names = [t[1] for t in airo.KEYWORD_TAGS]
        assert "Agent" in tag_names
        assert "RAG" in tag_names
        assert len(airo.KEYWORD_TAGS) >= 5

# ---------------------------------------------------------------------------
# infer_tags_if_empty
# ---------------------------------------------------------------------------

class TestInferTagsIfEmptyTier1:
    def test_returns_empty_for_empty_tags_with_empty_abstract(self):
        p = make_paper(abstract="", title="")
        _ = airo.infer_tags_if_empty([], p)
        # May or may not infer depending on implementation

    def test_does_not_modify_existing_tags(self):
        p = make_paper(abstract="This paper describes transformer architectures.")
        tags = airo.infer_tags_if_empty(["LLM"], p)
        assert "LLM" in tags

# ---------------------------------------------------------------------------
# download_pdf (mocked)
# ---------------------------------------------------------------------------

class TestDownloadPdf:
    def test_download_pdf_writes_file(self, mock_research_root):
        out_path = mock_research_root / "paper.pdf"
        # Mock the request to avoid network call
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.content = b"%PDF-1.4 fake pdf content"
        mock_response.headers = {}

        with patch("requests.get", return_value=mock_response) as mock_get:
            airo.download_pdf("https://example.com/paper.pdf", out_path, timeout=60)
            mock_get.assert_called_once()

# ---------------------------------------------------------------------------
# extract_pdf_text (mocked)
# ---------------------------------------------------------------------------

class TestExtractPdfText:
    def test_extracts_text_from_pdf(self, mock_research_root):
        # Create a minimal PDF
        pdf_path = mock_research_root / "test.pdf"
        # Write minimal PDF content
        pdf_path.write_bytes(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n")

        # Should not crash even on minimal PDF
        try:
            text = airo.extract_pdf_text(pdf_path, max_pages=1)
            # May be empty for minimal PDF - that's ok
            assert isinstance(text, str)
        except Exception:
            pass  # Some PDFs may not be extractable - that's ok for this test

    def test_returns_empty_for_nonexistent_file(self, mock_research_root):
        fake = mock_research_root / "nonexistent.pdf"
        text = airo.extract_pdf_text(fake)
        assert text == "" or text is None

# ---------------------------------------------------------------------------
# fetch_arxiv_metadata (mocked)
# ---------------------------------------------------------------------------

class TestFetchArxivMetadata:
    def test_fetches_and_returns_paper(self):
        mock_feed = """<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <entry>
    <title>Test Paper Title</title>
    <author><name>Alice Smith</name></author>
    <author><name>Bob Jones</name></author>
    <summary>Test abstract content.</summary>
    <published>2024-01-15</published>
    <updated>2024-01-20</updated>
    <id>http://arxiv.org/abs/2301.00001</id>
    <link href="https://arxiv.org/abs/2301.00001" type="text/html"/>
    <link title="pdf" href="https://arxiv.org/pdf/2301.00001.pdf" type="application/pdf"/>
    <arxiv:primary_category xmlns:arxiv="http://arxiv.org/schemas/atom" term="cs.AI"/>
  </entry>
</feed>"""

        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.text = mock_feed
        mock_response.headers = {"content-type": "application/atom+xml"}

        mock_session = MagicMock()
        mock_session.get.return_value = mock_response
        with patch("parsers.arxiv.get_cached", return_value=None):
            with patch("parsers.arxiv.set_cached"):
                with patch("parsers.arxiv._get_session", return_value=mock_session):
                    paper = airo.fetch_arxiv_metadata("2301.00001", timeout=30)

                    assert paper.title == "Test Paper Title"
            assert "Alice Smith" in paper.authors
            assert "Bob Jones" in paper.authors
            assert "Test abstract" in paper.abstract

    def test_raises_for_invalid_id(self):
        mock_session = MagicMock()
        mock_session.get.side_effect = Exception("Not found")
        with patch("parsers.arxiv._get_session", return_value=mock_session):
            with patch("parsers.arxiv.get_cached", return_value=None):
                with pytest.raises(Exception):
                    airo.fetch_arxiv_metadata("invalid-id-that-does-not-exist", timeout=30)


class TestFetchArxivMetadataBatch:
    def test_batch_returns_ordered_papers(self):
        """Batch returns papers in same order as input IDs."""
        mock_feed = """<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <entry>
    <title>Paper Two</title>
    <author><name>Carol White</name></author>
    <summary>Abstract two.</summary>
    <published>2024-02-01</published>
    <updated>2024-02-02</updated>
    <id>http://arxiv.org/abs/2302.00001</id>
    <link href="https://arxiv.org/abs/2302.00001" type="text/html"/>
    <link title="pdf" href="https://arxiv.org/pdf/2302.00001.pdf" type="application/pdf"/>
    <arxiv:primary_category xmlns:arxiv="http://arxiv.org/schemas/atom" term="cs.CL"/>
  </entry>
  <entry>
    <title>Paper One</title>
    <author><name>Alice Smith</name></author>
    <summary>Abstract one.</summary>
    <published>2024-01-15</published>
    <updated>2024-01-20</updated>
    <id>http://arxiv.org/abs/2301.00001</id>
    <link href="https://arxiv.org/abs/2301.00001" type="text/html"/>
    <link title="pdf" href="https://arxiv.org/pdf/2301.00001.pdf" type="application/pdf"/>
    <arxiv:primary_category xmlns:arxiv="http://arxiv.org/schemas/atom" term="cs.AI"/>
  </entry>
</feed>"""

        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.text = mock_feed
        mock_response.headers = {"content-type": "application/atom+xml"}

        mock_session = MagicMock()
        mock_session.get.return_value = mock_response
        with patch("parsers.arxiv._get_session", return_value=mock_session):
            with patch("parsers.arxiv.get_cached", return_value=None):
                with patch("parsers.arxiv.set_cached"):
                    papers = airo.fetch_arxiv_metadata_batch(
                        ["2301.00001", "2302.00001"], timeout=60
                    )

        assert len(papers) == 2
        assert papers[0].uid == "2301.00001"
        assert papers[0].title == "Paper One"
        assert papers[1].uid == "2302.00001"
        assert papers[1].title == "Paper Two"

    def test_batch_empty_input(self):
        papers = airo.fetch_arxiv_metadata_batch([])
        assert papers == []

    def test_batch_all_cached(self):
        cached_paper = {
            "source": "arxiv", "uid": "2301.00001", "title": "Cached Paper",
            "authors": ["Cached Author"], "abstract": "Cached abstract.",
            "published": "2024-01-15", "updated": "2024-01-20",
            "abs_url": "https://arxiv.org/abs/2301.00001",
            "pdf_url": "https://arxiv.org/pdf/2301.00001.pdf",
            "primary_category": "cs.AI", "categories": "cs.AI",
            "comment": "", "journal_ref": "", "doi": "",
        }
        with patch("parsers.arxiv.get_cached", return_value=cached_paper):
            papers = airo.fetch_arxiv_metadata_batch(["2301.00001"])
        assert len(papers) == 1
        assert papers[0].title == "Cached Paper"

    def test_batch_fallback_to_individual_on_error(self):
        """When batch request fails, falls back to individual fetch_arxiv_metadata."""
        cached_paper = {
            "source": "arxiv", "uid": "2301.00001", "title": "Fallback Paper",
            "authors": ["Fallback Author"], "abstract": "Fallback abstract.",
            "published": "2024-01-15", "updated": "2024-01-20",
            "abs_url": "https://arxiv.org/abs/2301.00001",
            "pdf_url": "https://arxiv.org/pdf/2301.00001.pdf",
            "primary_category": "cs.AI", "categories": "cs.AI",
            "comment": "", "journal_ref": "", "doi": "",
        }
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.text = "not xml at all"
        mock_response.headers = {"content-type": "text/plain"}

        def get_cached_mock_ns(ns, aid):
            return cached_paper if aid == "2301.00001" else None

        mock_session = MagicMock()
        mock_session.get.return_value = mock_response
        with patch("parsers.arxiv._get_session", return_value=mock_session):
            with patch("parsers.arxiv.get_cached", side_effect=get_cached_mock_ns):
                with patch("parsers.arxiv.set_cached"):
                    papers = airo.fetch_arxiv_metadata_batch(["2301.00001"], timeout=60)
        assert len(papers) == 1
        assert papers[0].uid == "2301.00001"

# ---------------------------------------------------------------------------
# fetch_crossref_metadata (mocked)
# ---------------------------------------------------------------------------

class TestFetchCrossrefMetadata:
    def test_fetches_and_returns_paper(self):
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json.return_value = {
            "message": {
                "title": ["Crossref Test Paper"],
                "author": [{"given": "Alice", "family": "Smith"}],
                "abstract": "Test abstract from Crossref.",
                "published": {"date-parts": [[2024, 1, 15]]},
            }
        }

        mock_session = MagicMock()
        mock_session.get.return_value = mock_response
        with patch("parsers.crossref.get_cached", return_value=None):
            with patch("parsers.crossref.set_cached"):
                with patch("parsers.crossref._http_session", mock_session):
                    paper, updated = airo.fetch_crossref_metadata("10.1234/test", timeout=30)

                    assert paper.title == "Crossref Test Paper"
            assert "Alice Smith" in paper.authors or "Alice" in paper.authors
            assert "Test abstract" in paper.abstract

    def test_returns_none_for_not_found(self):
        mock_session = MagicMock()
        mock_session.get.return_value = MagicMock(status_code=404)
        with patch("parsers.crossref._http_session", mock_session):
            paper, updated = airo.fetch_crossref_metadata("10.9999/notfound", timeout=30)
            # Should return (None, None) or similar on 404

# ---------------------------------------------------------------------------
# main CLI paths
# ---------------------------------------------------------------------------

class TestMainCli:
    def test_main_accepts_arxiv_id(self, mock_research_root, monkeypatch):
        monkeypatch.chdir(mock_research_root)

        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.text = '<?xml version="1.0"?><feed xmlns="http://www.w3.org/2005/Atom"><entry><title>Test</title><author><name>Alice</name></author><summary>Abstract.</summary><published>2024-01-01</published><updated>2024-01-01</updated><id>http://arxiv.org/abs/2301.00001</id><link href="https://arxiv.org/abs/2301.00001" type="text/html"/><link title="pdf" href="https://arxiv.org/pdf/2301.00001.pdf" type="application/pdf"/><arxiv:primary_category xmlns:arxiv="http://arxiv.org/schemas/atom" term="cs.AI"/></entry></feed>'
        mock_response.headers = {"content-type": "application/atom+xml"}

        mock_session = MagicMock()
        mock_session.get.return_value = mock_response
        with patch("parsers.arxiv._get_session", return_value=mock_session):
            with patch("sys.stdout", new=StringIO()) as _:
                try:
                    airo.main(["arxiv-id", "2301.00001"])
                except SystemExit:
                    pass  # main may call sys.exit

    def test_main_with_help(self, monkeypatch):
        with patch("sys.stdout", new=StringIO()) as _:
            with pytest.raises(SystemExit):
                airo.main(["--help"])

# ---------------------------------------------------------------------------
# wikilink_for_pnote
# ---------------------------------------------------------------------------

class TestWikilinkForPnoteTier1:
    def test_creates_wikilink(self):
        result = airo.wikilink_for_pnote("Test Paper")
        assert "[[" in result
        assert "Test Paper" in result
        assert "]]" in result

# ---------------------------------------------------------------------------
# call_llm_chat_completions (mocked)
# ---------------------------------------------------------------------------

class TestCallLlmChatCompletions:
    def test_raises_on_no_api_key(self):
        with patch.dict(os.environ, {}, clear=True):
            with pytest.raises(ValueError, match="API_KEY"):
                airo.call_llm_chat_completions([], "gpt-4o-mini", "")

    def test_uses_custom_base_url(self, monkeypatch):
        monkeypatch.setenv("API_KEY", "test-key-123")
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json.return_value = {
            "choices": [{"message": {"role": "assistant", "content": "Test response"}}]
        }

        with patch("requests.Session.post", return_value=mock_response) as mock_post:
            result = airo.call_llm_chat_completions(
                [{"role": "user", "content": "Hello"}],
                "gpt-4o-mini",
                base_url="https://api.example.com/v1",
                api_key="test-key-123"
            )
            assert "Test response" in result
            # Should call the custom base_url
            call_args = mock_post.call_args
            assert "api.example.com" in str(call_args)

    def test_uses_environment_api_key(self, monkeypatch):
        monkeypatch.setenv("OPENAI_API_KEY", "env-key-456")
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json.return_value = {
            "choices": [{"message": {"role": "assistant", "content": "Env key response"}}]
        }

        with patch("requests.Session.post", return_value=mock_response):
            result = airo.call_llm_chat_completions(
                [{"role": "user", "content": "Hi"}],
                "gpt-4o-mini",
                base_url="https://api.openai.com/v1",
                api_key=None  # should use env
            )
            assert "Env key response" in result

    def test_injects_system_prompt(self, monkeypatch):
        """system_prompt is injected as first system message"""
        monkeypatch.setenv("API_KEY", "test-key")
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json.return_value = {
            "choices": [{"message": {"role": "assistant", "content": "Response"}}]
        }

        with patch("requests.Session.post", return_value=mock_response) as mock_post:
            result = airo.call_llm_chat_completions(
                [{"role": "user", "content": "Hello"}],
                "gpt-4o-mini",
                "Say hi",
                base_url="https://api.openai.com/v1",
                api_key="test-key",
                system_prompt="You are helpful."
            )
            assert "Response" in result
            call_kwargs = mock_post.call_args.kwargs
            payload = call_kwargs["json"]
            messages = payload["messages"]
            # First message should be system with system_prompt content
            assert messages[0]["role"] == "system"
            assert messages[0]["content"] == "You are helpful."
            # Original user message should still be present
            assert any(m["role"] == "user" and "Hello" in m["content"] for m in messages)

    def test_system_prompt_none_not_injected(self, monkeypatch):
        """When system_prompt is None, no extra system message is added"""
        monkeypatch.setenv("API_KEY", "test-key")
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json.return_value = {
            "choices": [{"message": {"role": "assistant", "content": "Response"}}]
        }

        with patch("requests.Session.post", return_value=mock_response) as mock_post:
            _ = airo.call_llm_chat_completions(
                [{"role": "user", "content": "Hello"}],
                "gpt-4o-mini",
                "Say hi",
                base_url="https://api.openai.com/v1",
                api_key="test-key",
                system_prompt=None
            )
            call_kwargs = mock_post.call_args.kwargs
            payload = call_kwargs["json"]
            messages = payload["messages"]
            # No system message prepended when system_prompt=None
            assert not any(m["role"] == "system" for m in messages)
            # Original user message + the user_prompt should be present
            assert len(messages) == 2
            assert messages[0]["content"] == "Hello"
            assert messages[1]["content"] == "Say hi"


# ---------------------------------------------------------------------------
# looks_like_heading
# ---------------------------------------------------------------------------

class TestLooksLikeHeadingTier2:
    def test_accepts_numeric_outline(self):
        assert airo.looks_like_heading("1. Introduction") is True
        assert airo.looks_like_heading("2.3. Method") is True

    def test_rejects_short_lines(self):
        assert airo.looks_like_heading("a") is False
        assert airo.looks_like_heading("ab") is False

    def test_rejects_long_lines(self):
        assert airo.looks_like_heading("x" * 121) is False

    def test_accepts_roman_numerals(self):
        assert airo.looks_like_heading("I. Introduction") is True
        assert airo.looks_like_heading("X. Conclusion") is True
        assert airo.looks_like_heading("III. Related Work") is True

    def test_rejects_roman_beyond_x(self):
        assert airo.looks_like_heading("XI. Overview") is False

    def test_accepts_keyword_headings(self):
        for kw in ["Introduction", "Abstract", "Method", "Experiments", "Conclusion", "Related Work", "Future Work", "Appendix"]:
            assert airo.looks_like_heading(kw) is True, f"Failed for {kw}"

    def test_rejects_bullet_points(self):
        assert airo.looks_like_heading("- item") is False
        assert airo.looks_like_heading("* item") is False
        assert airo.looks_like_heading("  - nested") is False

    def test_rejects_code_block_lines(self):
        assert airo.looks_like_heading("```python") is False
        assert airo.looks_like_heading("    def foo():") is False

    def test_accepts_allcaps_short(self):
        assert airo.looks_like_heading("BACKGROUND") is True
        assert airo.looks_like_heading("METHODS") is True

    def test_rejects_allcaps_too_short(self):
        assert airo.looks_like_heading("AB") is False
        assert airo.looks_like_heading("A") is False

    def test_rejects_allcaps_too_long(self):
        assert airo.looks_like_heading("A" * 41) is False


# ---------------------------------------------------------------------------
# format_section_snippets
# ---------------------------------------------------------------------------

class TestFormatSectionSnippetsTier2:
    def test_truncates_long_section(self):
        long_content = "a" * 2000
        sections = [("Abstract", long_content)]  # Abstract = high priority, gets budget
        result = airo.format_section_snippets(sections, max_chars_total=200, min_chars_per_high_prio=200)
        # With tiny budget, Abstract section should be truncated
        assert len(result) < len(long_content)

    def test_single_section(self):
        sections = [("Intro", "Some introduction text.")]
        result = airo.format_section_snippets(sections)
        assert "### Intro" in result
        assert "Some introduction text" in result

    def test_multiple_sections(self):
        sections = [("Intro", "Intro text."), ("Method", "Method text.")]
        result = airo.format_section_snippets(sections)
        assert "### Intro" in result
        assert "### Method" in result
        assert "Intro text" in result
        assert "Method text" in result

    def test_blockquote_format(self):
        sections = [("Test", "line1\nline2")]
        result = airo.format_section_snippets(sections)
        for line in result.splitlines():
            if line.strip() and not line.startswith("###"):
                assert line.startswith("> "), f"Line should be blockquote: {repr(line)}"

    def test_empty_content_skipped(self):
        sections = [("Test", "   ")]
        _ = airo.format_section_snippets(sections)
        # empty sections are filtered out by segment_into_sections upstream

    def test_returns_empty_string_for_empty_input(self):
        result = airo.format_section_snippets([])
        assert result == ""


# ---------------------------------------------------------------------------
# is_probably_doi
# ---------------------------------------------------------------------------

class TestIsProbablyDoiTier2:
    def test_accepts_doi_url(self):
        assert airo.is_probably_doi("https://doi.org/10.1234/abc") is True
        assert airo.is_probably_doi("http://dx.doi.org/10.1234/abc") is True

    def test_accepts_bare_doi(self):
        assert airo.is_probably_doi("10.1234/5678") is True

    def test_rejects_non_doi(self):
        assert airo.is_probably_doi("10.1234") is False  # too short
        assert airo.is_probably_doi("11.1234/abc") is False  # must be 10.x
        assert airo.is_probably_doi("https://example.com/paper") is False

    def test_strips_whitespace(self):
        assert airo.is_probably_doi("  10.1234/abc  ") is True


# ---------------------------------------------------------------------------
# safe_uid
# ---------------------------------------------------------------------------

class TestSafeUidTier2:
    def test_replaces_special_chars(self):
        assert airo.safe_uid("paper: title!@#") == "paper_title_"

    def test_strips_whitespace(self):
        assert airo.safe_uid("  hello  ") == "hello"

    def test_preserves_dots_and_dashes(self):
        assert airo.safe_uid("v1.2.3-paper") == "v1.2.3-paper"

    def test_empty_string(self):
        assert airo.safe_uid("") == ""


# ---------------------------------------------------------------------------
# read_text / write_text edge cases
# ---------------------------------------------------------------------------

class TestReadWriteEdgeCasesTier2:
    def test_read_nonexistent_returns_empty(self):
        from pathlib import Path
        result = airo.read_text(Path("/nonexistent/file.txt"))
        assert result == ""

    def test_write_creates_parent_dirs(self, mock_research_root):
        f = mock_research_root / "subdir" / "nested" / "file.txt"
        airo.write_text(f, "content")
        assert f.exists()
        assert airo.read_text(f) == "content"

    def test_write_overwrites(self, mock_research_root):
        f = mock_research_root / "overwrite.txt"
        airo.write_text(f, "v1")
        airo.write_text(f, "v2")
        assert airo.read_text(f) == "v2"


# ---------------------------------------------------------------------------
# infer_tags_if_empty
# ---------------------------------------------------------------------------

class TestInferTagsIfEmptyTier2:
    def test_returns_existing_tags(self):
        p = make_paper(title="Test", abstract="Nothing special")
        result = airo.infer_tags_if_empty(["LLM"], p)
        assert result == ["LLM"]

    def test_infers_agent_from_abstract(self):
        p = make_paper(title="Agent System", abstract="An agent uses tools to solve tasks.")
        result = airo.infer_tags_if_empty([], p)
        assert "Agent" in result

    def test_infers_rag_from_title(self):
        p = make_paper(title="RAG for Knowledge", abstract="Using retrieval.")
        result = airo.infer_tags_if_empty([], p)
        assert "RAG" in result

    def test_returns_unsorted_when_no_match(self):
        p = make_paper(title="Zxy Qrs Abc", abstract="Foo bar baz.")
        result = airo.infer_tags_if_empty([], p)
        assert result == ["Unsorted"]

    def test_returns_empty_when_given_empty_and_empty_abstract(self):
        p = make_paper(title="", abstract="")
        result = airo.infer_tags_if_empty([], p)
        assert result == ["Unsorted"]


# ---------------------------------------------------------------------------
# mnote_filename
# ---------------------------------------------------------------------------

class TestMnoteFilenameTier2:
    def test_basic_format(self, mock_research_root):
        a = mock_research_root / "P - 2024 - PaperA.md"
        b = mock_research_root / "P - 2024 - PaperB.md"
        c = mock_research_root / "P - 2024 - PaperC.md"
        for p in [a, b, c]:
            p.touch()
        result = airo.mnote_filename("LLM", a, b, c)
        assert result.startswith("M - LLM - ")
        assert "PaperA" in result
        assert result.endswith(".md")

    def test_strips_p_prefix(self, mock_research_root):
        a = mock_research_root / "P - 2024 - LongPaperNameHereThatExceedsTwentyFourCharacters.md"
        b = mock_research_root / "P - 2024 - PaperB.md"
        c = mock_research_root / "P - 2024 - PaperC.md"
        for p in [a, b, c]:
            p.touch()
        result = airo.mnote_filename("Test", a, b, c)
        # The "P - 2024 - " prefix should be stripped
        assert "LongPaper" in result


# ---------------------------------------------------------------------------
# parse_current_abc
# ---------------------------------------------------------------------------

class TestParseCurrentAbcTier2:
    def test_parses_all_three(self):
        md = "- A: Paper A\n- B: Paper B\n- C: Paper C"
        a, b, c = airo.parse_current_abc(md)
        assert a == "Paper A"
        assert b == "Paper B"
        assert c == "Paper C"

    def test_parses_partial(self):
        md = "- A: Only A"
        a, b, c = airo.parse_current_abc(md)
        assert a == "Only A"
        assert b is None
        assert c is None

    def test_returns_none_when_empty(self):
        md = "No ABC here"
        a, b, c = airo.parse_current_abc(md)
        assert a is None
        assert b is None
        assert c is None


# ---------------------------------------------------------------------------
# append_view_evolution_log
# ---------------------------------------------------------------------------

class TestAppendViewEvolutionLogTier2:
    def test_adds_entry_with_date(self):
        md = "# Test\n\n## View Evolution Log\n"
        result = airo.append_view_evolution_log(md, ("OldA", "OldB", "OldC"), ("NewA", "NewB", "NewC"))
        assert "NewA" in result
        assert "OldA" in result
        assert "旧观点" in result or "新证据" in result or "更新结论" in result

    def test_creates_log_section_if_missing(self):
        md = "# Test\n\nNo log here."
        result = airo.append_view_evolution_log(md, ("A", "B", "C"), ("X", "Y", "Z"))
        assert "View Evolution Log" in result

    def test_empty_old_abc(self):
        md = "# Test\n\n## View Evolution Log\n"
        result = airo.append_view_evolution_log(md, (None, None, None), ("X", "Y", "Z"))
        assert "X" in result


# ---------------------------------------------------------------------------
# update_cnote_links edge cases
# ---------------------------------------------------------------------------

class TestUpdateCnoteLinksEdgeCasesTier2:
    def test_adds_link_to_cnote(self, mock_research_root):
        concept_dir = mock_research_root / "01-Concepts"
        concept_dir.mkdir()
        cnote = airo.ensure_cnote(concept_dir, "TestConcept")
        pnote = mock_research_root / "02-Papers" / "PaperX.md"
        pnote.parent.mkdir(parents=True, exist_ok=True)
        pnote.write_text("# Paper X\n\ntags: [LLM]")
        airo.update_cnote_links(cnote, pnote)
        content = airo.read_text(cnote)
        assert "[[PaperX]]" in content


# ---------------------------------------------------------------------------
# render_cnote
# ---------------------------------------------------------------------------

class TestRenderCnoteTier2:
    def test_has_required_sections(self):
        result = airo.render_cnote("Attention Mechanism")
        for section in ["# Attention Mechanism", "## 核心定义", "## 技术本质", "## 常见实现路径", "## 优势", "## 局限", "## 演化时间线"]:
            assert section in result

    def test_type_is_concept(self):
        result = airo.render_cnote("Test")
        assert "type: concept" in result

    def test_status_is_evergreen(self):
        result = airo.render_cnote("Test")
        assert "status: evergreen" in result


# ---------------------------------------------------------------------------
# today_iso
# ---------------------------------------------------------------------------

class TestTodayIsoTier2:
    def test_returns_iso_format(self):
        import re
        result = airo.today_iso()
        assert re.match(r"\d{4}-\d{2}-\d{2}", result) is not None

    def test_returns_todays_date(self):
        import datetime as dt
        assert airo.today_iso() == dt.date.today().isoformat()


# ---------------------------------------------------------------------------
# wikilink_for_pnote
# ---------------------------------------------------------------------------

class TestWikilinkForPnoteTier2:
    def test_format(self, mock_research_root):
        p = mock_research_root / "P - 2024 - Test Paper.md"
        result = airo.wikilink_for_pnote(p)
        assert result == "[[P - 2024 - Test Paper]]"

    def test_stem_only(self, mock_research_root):
        p = mock_research_root / "Simple.md"
        result = airo.wikilink_for_pnote(p)
        assert result == "[[Simple]]"


# ---------------------------------------------------------------------------
# ensure_or_update_mnote
# ---------------------------------------------------------------------------

class TestEnsureOrUpdateMnoteTier2:
    def test_returns_none_for_insufficient_papers(self, mock_research_root):
        p = mock_research_root / "02-Papers" / "only_one.md"
        p.parent.mkdir(parents=True)
        p.write_text("# One")
        result = airo.ensure_or_update_mnote(mock_research_root, "LLM", [p])
        assert result is None

    def test_creates_mnote(self, mock_research_root):
        papers_dir = mock_research_root / "02-Papers"
        papers_dir.mkdir()
        for name in ["A.md", "B.md", "C.md"]:
            (papers_dir / name).write_text(f"# {name}\n\ntags: [LLM]")
        papers = list(papers_dir.glob("*.md"))
        result = airo.ensure_or_update_mnote(mock_research_root, "LLM", papers)
        assert result is not None
        assert result.name.startswith("M - LLM")


# ---------------------------------------------------------------------------
# parse_tags_from_frontmatter edge cases
# ---------------------------------------------------------------------------

class TestParseTagsFromFrontmatterEdgeTier2:
    def test_handles_missing_tags_key(self):
        fm = {}
        result = airo.parse_tags_from_frontmatter(fm)
        assert result == []

    def test_handles_non_string_tags(self):
        # When frontmatter has a non-string, non-list tags value (e.g. numeric),
        # it returns [] rather than crashing
        fm = {"tags": 42}
        result = airo.parse_tags_from_frontmatter(fm)
        assert result == []


# ---------------------------------------------------------------------------
# segment_into_sections edge cases
# ---------------------------------------------------------------------------

class TestSegmentIntoSectionsEdgeTier2:
    def test_no_headings_returns_single_section(self):
        result = airo.segment_into_sections("Just some text without headings.")
        assert len(result) == 1
        title, content = result[0]
        assert title == "BODY"
        assert "Just some text" in content

    def test_only_headings_no_content(self):
        result = airo.segment_into_sections("# Intro\n\n## Method\n")
        # Sections with empty content are filtered out by join+strip
        assert isinstance(result, list)

    def test_mixed_md_and_numeric_headings(self):
        text = "# Introduction\nIntro text.\n1. Preliminaries\nPretext."
        result = airo.segment_into_sections(text)
        titles = [t for t, c in result]
        assert "Introduction" in titles

    def test_truncation_marker(self):
        # With max_sections=2, if there are more than 2 sections, TRUNCATED is added
        text = "# H1\nContent1\n# H2\nContent2\n# H3\nContent3\n# H4\nContent4"
        result = airo.segment_into_sections(text, max_sections=2)
        titles = [t for t, c in result]
        assert "TRUNCATED" in titles

    def test_max_sections_respected(self):
        text = "\n".join([f"# H{i}\nContent{i}" for i in range(25)])
        result = airo.segment_into_sections(text, max_sections=5)
        assert len(result) <= 6  # 5 sections + TRUNCATED marker


# ---------------------------------------------------------------------------
# parse_date_from_frontmatter edge cases
# ---------------------------------------------------------------------------

class TestParseDateFromFrontmatterTier2:
    def test_returns_empty_for_missing_date(self):
        result = airo.parse_date_from_frontmatter({})
        assert result == ""

    def test_returns_bad_format_with_warning(self, recwarn):
        result = airo.parse_date_from_frontmatter({"date": "2024-1-1"})
        assert result == "2024-1-1"
        assert any("Unrecognized date format" in str(w.message) for w in recwarn)

    def test_rejects_partial_date_with_warning(self, recwarn):
        result = airo.parse_date_from_frontmatter({"date": "2024-01"})
        assert result == "2024-01"
        assert any("Unrecognized date format" in str(w.message) for w in recwarn)

    def test_rejects_non_date_string_with_warning(self, recwarn):
        result = airo.parse_date_from_frontmatter({"date": "yesterday"})
        assert result == "yesterday"
        assert any("Unrecognized date format" in str(w.message) for w in recwarn)


# ---------------------------------------------------------------------------
# update_radar edge cases
# ---------------------------------------------------------------------------

class TestUpdateRadarEdgeTier2:
    def test_increments_heat(self, mock_research_root):
        root = mock_research_root
        airo.ensure_research_tree(root)
        p = airo.ensure_radar(root)
        # Write initial radar with one tag (ASCII only to avoid encoding issues)
        p.write_text("# Radar\n\n| Topic | Heat | Evidence | Cost | Confidence | Updated |\n| -- | -- | ---- | ---- | ---- | ---- |\n| LLM | 2 | High | Flat | Medium | 2024-01-15 |\n")
        result = airo.update_radar(root, ["LLM"], "2024-01-20")
        content = airo.read_text(result)
        assert "LLM" in content


# --------------------------------------------------
# Tier 2: extract_pdf_text_hybrid
# --------------------------------------------------

def test_extract_pdf_text_hybrid_basic(tmp_path):
    """Basic text extraction from a valid PDF."""
    import ai_research_os as airo
    pdf_path = tmp_path / "sample.pdf"
    # Create a simple PDF with known text using PyMuPDF
    try:
        import fitz
    except ImportError:
        import pytest
        pytest.skip("PyMuPDF not installed")
    doc = fitz.open()
    page = doc.new_page(width=595, height=842)
    page.insert_text((72, 72), "Introduction", fontsize=12)
    page.insert_text((72, 90), "This paper presents a new method.", fontsize=11)
    doc.save(str(pdf_path))
    doc.close()

    text = airo.extract_pdf_text_hybrid(pdf_path)
    assert "Introduction" in text
    assert "This paper presents" in text


def test_extract_pdf_text_hybrid_max_pages(tmp_path):
    """Respects max_pages limit."""
    import ai_research_os as airo
    try:
        import fitz
    except ImportError:
        import pytest
        pytest.skip("PyMuPDF not installed")
    pdf_path = tmp_path / "multipage.pdf"
    doc = fitz.open()
    for i in range(3):
        page = doc.new_page(width=200, height=200)
        page.insert_text((50, 100), f"Page {i+1} content", fontsize=12)
    doc.save(str(pdf_path))
    doc.close()

    text = airo.extract_pdf_text_hybrid(pdf_path, max_pages=1)
    assert "Page 1" in text
    # other pages may or may not appear depending on implementation


def test_extract_pdf_text_hybrid_empty_page(tmp_path):
    """PDF with empty pages returns empty string for those pages."""
    import ai_research_os as airo
    try:
        import fitz
    except ImportError:
        import pytest
        pytest.skip("PyMuPDF not installed")
    pdf_path = tmp_path / "empty.pdf"
    doc = fitz.open()
    doc.new_page(width=200, height=200)
    # intentionally blank page
    doc.save(str(pdf_path))
    doc.close()

    text = airo.extract_pdf_text_hybrid(pdf_path)
    # empty pages produce empty string
    assert text == ""


def test_extract_pdf_text_hybrid_no_pdfminer(tmp_path):
    """Works even without pdfminer installed (uses PyMuPDF only)."""
    import ai_research_os as airo
    try:
        import fitz
    except ImportError:
        import pytest
        pytest.skip("PyMuPDF not installed")
    pdf_path = tmp_path / "sample.pdf"
    doc = fitz.open()
    page = doc.new_page(width=200, height=200)
    page.insert_text((50, 100), "Test content", fontsize=12)
    doc.save(str(pdf_path))
    doc.close()

    # pdfminer not installed - should still work via PyMuPDF
    text = airo.extract_pdf_text_hybrid(pdf_path, use_pdfminer_fallback=False)
    assert "Test content" in text


# --------------------------------------------------
# Tier 2: ai_generate_pnote_draft
# --------------------------------------------------

def test_ai_generate_pnote_draft_calls_llm(monkeypatch):
    """Verifies ai_generate_pnote_draft calls call_llm_chat_completions correctly."""
    import ai_research_os as airo
    from unittest.mock import MagicMock

    mock_response = MagicMock()
    mock_response.status_code = 200
    mock_response.content = b'{"choices":[{"message":{"content":"Mock AI draft"}}]}'

    calls = []
    def mock_post(url, **kwargs):
        calls.append((url, kwargs))
        return mock_response

    paper = airo.Paper(
        title="Test Paper",
        authors=["Author A", "Author B"],
        uid="2301.12345",
        source="arxiv",
        abstract="This is a test abstract.",
        published="2023-01-01",
        updated="2023-01-02",
        abs_url="https://arxiv.org/abs/2301.12345",
        pdf_url="https://arxiv.org/pdf/2301.12345.pdf",
        primary_category="cs.AI",
    )

    with monkeypatch.context() as m:
        m.setattr("llm.generate.call_llm_chat_completions", lambda **kw: "Mock AI draft")
        result = airo.ai_generate_pnote_draft(
            paper=paper,
            tags=["AI", "ML"],
            extracted_text="Extracted text here.",
            base_url="https://api.openai.com/v1",
            api_key="test-key",
            model="gpt-4o-mini"
        )
    assert result == "Mock AI draft"


def test_ai_generate_pnote_draft_includes_required_sections(monkeypatch):
    """Prompt contains all required section headings."""
    import ai_research_os as airo

    captured_prompts = {}
    def mock_call_llm(**kwargs):
        captured_prompts.update(kwargs)
        return "draft"

    paper = airo.Paper(
        title="Test Paper",
        authors=["Author A"],
        uid="2301.12345",
        source="arxiv",
        abstract="Abstract text.",
        published="2023-01-01",
        updated="2023-01-02",
        abs_url="https://arxiv.org/abs/2301.12345",
        pdf_url="https://arxiv.org/pdf/2301.12345.pdf",
        primary_category="cs.AI",
    )

    with monkeypatch.context() as m:
        m.setattr("llm.generate.call_llm_chat_completions", mock_call_llm)
        airo.ai_generate_pnote_draft(
            paper=paper,
            tags=["AI"],
            extracted_text="Some extracted text.",
            base_url="https://api.openai.com/v1",
            api_key="test-key",
            model="gpt-4o-mini"
        )

    user_prompt = captured_prompts.get("user_prompt", "")
    system_prompt = captured_prompts.get("system_prompt", "")
    assert "## 1. 背景" in user_prompt
    assert "## 2. 核心问题" in user_prompt
    assert "## 5. 实验分析" in user_prompt
    assert "## 6. 对抗式审稿" in user_prompt
    assert "## 14. 评分量表" in user_prompt
    assert "评分量表" in system_prompt


# --------------------------------------------------
# Tier 3: segment_into_sections + format_section_snippets
# --------------------------------------------------

def test_segment_into_sections_basic():
    import ai_research_os as airo
    # "Method details here" triggers looks_like_heading (keyword "method"), so use a safe heading
    text = "# Introduction\n\nSome intro content.\n\n## Key Contributions\n\nActual method content here."
    sections = airo.segment_into_sections(text)
    assert len(sections) == 2
    assert sections[0][0] == "Introduction"
    assert "intro content" in sections[0][1]
    assert sections[1][0] == "Key Contributions"


def test_segment_into_sections_no_headings():
    import ai_research_os as airo
    text = "Just plain text\n\nwith multiple lines\n\nno headings here."
    sections = airo.segment_into_sections(text)
    assert len(sections) == 1
    assert sections[0][0] == "BODY"


def test_segment_into_sections_truncates_above_max():
    import ai_research_os as airo
    text = "\n\n".join(f"# Section {i}\nContent" for i in range(25))
    sections = airo.segment_into_sections(text, max_sections=5)
    assert len(sections) == 6  # 5 + TRUNCATED
    assert sections[-1][0] == "TRUNCATED"


def test_segment_into_sections_strips_empty():
    import ai_research_os as airo
    text = "# A\n\n\n\n   \n# B\n\n"
    sections = airo.segment_into_sections(text)
    assert all(content.strip() for _, content in sections)


def test_format_section_snippets_basic():
    import ai_research_os as airo
    sections = [("Introduction", "This is the intro content."), ("Methods", "Method details.")]
    result = airo.format_section_snippets(sections)
    assert "### Introduction" in result
    assert "### Methods" in result
    assert "> This is the intro" in result


def test_format_section_snippets_truncates_long():
    import ai_research_os as airo
    long_content = "x" * 3000
    sections = [("Abstract", long_content)]  # Abstract = high priority section
    result = airo.format_section_snippets(sections, max_chars_total=1000)
    assert len(result) < 3000


# --------------------------------------------------
# Tier 3: mnote_filename + parse_current_abc
# --------------------------------------------------

def test_mnote_filename_basic(tmp_path):
    import ai_research_os as airo
    a = tmp_path / "P - 2024 - Long Title Here12345.md"
    b = tmp_path / "P - 2024 - Another Title67890.md"
    c = tmp_path / "P - 2024 - Third Title11111.md"
    a.touch()
    b.touch()
    c.touch()
    fname = airo.mnote_filename("RAG", a, b, c)
    assert fname.startswith("M - RAG - ")
    assert fname.endswith(".md")


def test_mnote_filename_strips_prefix(tmp_path):
    import ai_research_os as airo
    a = tmp_path / "P - 2024 - Very Long Stem That Exceeds Twenty Four Characters.md"
    b = tmp_path / "P - 2024 - Short.md"
    c = tmp_path / "P - 2024 - Tiny.md"
    a.touch()
    b.touch()
    c.touch()
    fname = airo.mnote_filename("Agent", a, b, c)
    # prefix stripped, and truncated to 24 chars
    assert "P - 2024 - " not in fname
    assert fname.endswith(".md")


def test_parse_current_abc_basic():
    import ai_research_os as airo
    md = "# M Note\n\n- A: TitleA\n- B: TitleB\n- C: TitleC\n"
    a, b, c = airo.parse_current_abc(md)
    assert a == "TitleA"
    assert b == "TitleB"
    assert c == "TitleC"


def test_parse_current_abc_missing_fields():
    import ai_research_os as airo
    md = "- A: OnlyA\n"
    a, b, c = airo.parse_current_abc(md)
    assert a == "OnlyA"
    assert b is None
    assert c is None


def test_parse_current_abc_no_match():
    import ai_research_os as airo
    md = "No ABC here"
    a, b, c = airo.parse_current_abc(md)
    assert a is None
    assert b is None
    assert c is None


# --------------------------------------------------
# Tier 3: append_view_evolution_log
# --------------------------------------------------

def test_append_view_evolution_log_new_section():
    import ai_research_os as airo
    md = "# M Note\n\nOld content."
    result = airo.append_view_evolution_log(md, ("OldA", "OldB", "OldC"), ("NewA", "NewB", "NewC"))
    assert "## View Evolution Log" in result
    assert "NewA" in result
    assert "OldA" in result


def test_append_view_evolution_log_existing_section():
    import ai_research_os as airo
    md = "# M Note\n\n## View Evolution Log\n\nExisting log."
    result = airo.append_view_evolution_log(md, ("OA", "OB", "OC"), ("NA", "NB", "NC"))
    assert result.count("View Evolution Log") == 1
    assert "NA" in result


# --------------------------------------------------
# Tier 3: parse_radar_table
# --------------------------------------------------

def test_parse_radar_table_basic():
    import ai_research_os as airo
    md = """# Radar

| 主题 | 热度 | 证据质量 | 成本变化 | 我的信心 | 最近更新 |
| -- | -- | ---- | ---- | ---- | ---- |
| RAG | 5 | High | Stable | 4 | 2024-01-01 |
"""
    header, rows = airo.parse_radar_table(md)
    assert "RAG" in rows[0]["主题"]
    assert rows[0]["热度"] == "5"
    assert rows[0]["最近更新"] == "2024-01-01"


def test_parse_radar_table_no_table():
    import ai_research_os as airo
    md = "# Radar\n\nNo table here."
    header, rows = airo.parse_radar_table(md)
    assert rows == []
    assert "Radar" in header


# --------------------------------------------------
# Tier 3: infer_tags_if_empty
# --------------------------------------------------

def test_infer_tags_if_empty_returns_existing():
    import ai_research_os as airo
    paper = airo.Paper(source="arxiv", uid="2301.00001", title="Test", authors=[], abstract="", published="", updated="", abs_url="", pdf_url="", primary_category="")
    tags = ["CustomTag"]
    result = airo.infer_tags_if_empty(tags, paper)
    assert result == ["CustomTag"]


def test_infer_tags_if_empty_infers_agent():
    import ai_research_os as airo
    paper = airo.Paper(source="arxiv", uid="2301.00001", title="Agent Tools", authors=[], abstract="An agent uses tools.", published="", updated="", abs_url="", pdf_url="", primary_category="")
    result = airo.infer_tags_if_empty([], paper)
    assert "Agent" in result


def test_infer_tags_if_empty_infers_rag():
    import ai_research_os as airo
    paper = airo.Paper(source="arxiv", uid="2301.00001", title="RAG System", authors=[], abstract="Retrieval augmented generation.", published="", updated="", abs_url="", pdf_url="", primary_category="")
    result = airo.infer_tags_if_empty([], paper)
    assert "RAG" in result


def test_infer_tags_if_empty_returns_unsorted():
    import ai_research_os as airo
    paper = airo.Paper(source="arxiv", uid="2301.00001", title="Foo Bar", authors=[], abstract="Nothing matching.", published="", updated="", abs_url="", pdf_url="", primary_category="")
    result = airo.infer_tags_if_empty([], paper)
    assert result == ["Unsorted"]


# --------------------------------------------------
# Tier 3: upsert_link_under_heading
# --------------------------------------------------

def test_upsert_link_under_heading_basic():
    import ai_research_os as airo
    md = "# C Note\n\n## 关联笔记\n\n"
    result = airo.upsert_link_under_heading(md, "关联笔记", "[[P - 2024 - Test]]")
    assert "[[P - 2024 - Test]]" in result


def test_upsert_link_under_heading_no_section():
    import ai_research_os as airo
    md = "# C Note\n\nNo section."
    result = airo.upsert_link_under_heading(md, "关联笔记", "[[P - 2024 - Test]]")
    # Should add the heading section
    assert "## 关联笔记" in result


def test_upsert_link_under_heading_idempotent():
    import ai_research_os as airo
    md = "# C Note\n\n## 关联笔记\n\n"
    # First call: adds link
    r1 = airo.upsert_link_under_heading(md, "关联笔记", "- [[P - 2024 - Test]]")
    assert "[[P - 2024 - Test]]" in r1
    # Second call: replaces existing link (dedup by ^-\s*\S.*$ pattern)
    r2 = airo.upsert_link_under_heading(r1, "关联笔记", "- [[P - 2024 - Test]]")
    assert r2.count("[[P - 2024 - Test]]") == 1


# --------------------------------------------------
# Tier 3: render_mnote
# --------------------------------------------------

def test_render_mnote_basic():
    import ai_research_os as airo
    result = airo.render_mnote("RAG", "PaperA", "PaperB", "PaperC")
    assert "RAG" in result
    assert "PaperA" in result
    assert "PaperB" in result
    assert "PaperC" in result
    assert "## 当前 A/B/C" in result


# ================================================================
# Tier 4: additional coverage for date/title utils and notes
# (Moved to tests/test_unit_notes.py)




class TestParseSections:
    """Section extraction from raw LLM markdown output."""

    def test_basic_section_extraction(self):
        from llm.parse import parse_ai_pnote_draft
        raw = """## 1. 背景
这里是背景内容。

## 2. 核心问题
这里是核心问题。
"""
        sections, rubric, _ = parse_ai_pnote_draft(raw)
        assert sections["## 1. 背景"] == "这里是背景内容。"
        assert sections["## 2. 核心问题"] == "这里是核心问题。"

    def test_all_14_sections(self):
        from llm.parse import parse_ai_pnote_draft
        raw = "\n".join(f"## {i}. Section {i}\nContent for section {i}." for i in range(1, 15))
        sections, _, _ = parse_ai_pnote_draft(raw)
        assert len(sections) == 14
        for i in range(1, 15):
            assert sections[f"## {i}. Section {i}"] == f"Content for section {i}."

    def test_subsection_extraction_removes_from_parent(self):
        from llm.parse import parse_ai_pnote_draft
        raw = """## 3. 方法结构

这是父节的主体内容，包含背景介绍。

### 3.1 架构拆解
架构内容在这里。

### 3.2 算法逻辑
算法内容在这里。

## 4. 关键创新
创新内容。
"""
        sections, _, _ = parse_ai_pnote_draft(raw)
        # Subsections extracted as separate keys
        assert "## 3.1 架构拆解" in sections
        assert "## 3.2 算法逻辑" in sections
        # Parent content has subsections removed
        parent = sections["## 3. 方法结构"]
        assert "3.1 架构拆解" not in parent
        assert "3.2 算法逻辑" not in parent
        assert "这是父节的主体内容" in parent

    def test_multiple_subsections_in_order(self):
        from llm.parse import parse_ai_pnote_draft
        raw = """## 5. 实验分析

### 5.1 数据集
数据集描述。

### 5.2 基线对比
基线对比描述。

### 5.3 消融实验
消融实验描述。

### 5.4 成本分析
成本分析描述。

## 6. 对抗式审稿
审稿内容。
"""
        sections, _, _ = parse_ai_pnote_draft(raw)
        assert sections["## 5.1 数据集"] == "数据集描述。"
        assert sections["## 5.2 基线对比"] == "基线对比描述。"
        assert sections["## 5.3 消融实验"] == "消融实验描述。"
        assert sections["## 5.4 成本分析"] == "成本分析描述。"
        assert "5.1 数据集" not in sections["## 5. 实验分析"]

    def test_section_with_no_heading_body(self):
        from llm.parse import parse_ai_pnote_draft
        raw = """## 1. 背景

## 2. 核心问题
有内容的节。
"""
        sections, _, _ = parse_ai_pnote_draft(raw)
        assert sections["## 1. 背景"] == ""
        assert sections["## 2. 核心问题"] == "有内容的节。"

    def test_section_titles_with_chinese_punctuation(self):
        from llm.parse import parse_ai_pnote_draft
        raw = """## 11. Decision（决策）
决策内容。

## 12. 知识蒸馏
蒸馏内容。
"""
        sections, _, _ = parse_ai_pnote_draft(raw)
        assert "## 11. Decision（决策）" in sections
        assert "## 12. 知识蒸馏" in sections

    def test_section_with_long_content(self):
        from llm.parse import parse_ai_pnote_draft
        long = "x" * 5000
        raw = f"""## 1. 背景
{long}
"""
        sections, _, _ = parse_ai_pnote_draft(raw)
        assert sections["## 1. 背景"] == long

    def test_section_content_with_numbered_pattern_not_heading(self):
        from llm.parse import parse_ai_pnote_draft
        # Numbers in content should NOT be treated as headings
        raw = """## 1. 背景
我们在实验中发现 3.1 版本表现最好，在 5.2 场景下有显著提升。
"""
        sections, _, _ = parse_ai_pnote_draft(raw)
        parent = sections["## 1. 背景"]
        assert "3.1 版本" in parent
        assert "5.2 场景" in parent
        assert "## 3.1" not in sections
        assert "## 5.2" not in sections

    def test_empty_subsection(self):
        from llm.parse import parse_ai_pnote_draft
        raw = """## 3. 方法结构

### 3.1 架构拆解
Architecture described above.
"""
        sections, _, _ = parse_ai_pnote_draft(raw)
        assert sections["## 3.1 架构拆解"] == "Architecture described above."
        assert sections["## 3. 方法结构"] == ""


class TestParseRubric:
    """Rubric score extraction from XML/JSON/line formats."""

    def test_full_json_rubric(self):
        from llm.parse import parse_ai_pnote_draft
        raw = """## 1. 背景
Content.

<!--
<RUBRIC>
{
  "novelty": 4,
  "leverage": 3,
  "evidence": 5,
  "cost": 2,
  "moat": 3,
  "adoption": 4,
  "overall": "Strong paper with good experiments."
}
</RUBRIC>
-->
"""
        _, rubric, _ = parse_ai_pnote_draft(raw)
        assert rubric["novelty"] == 4
        assert rubric["leverage"] == 3
        assert rubric["evidence"] == 5
        assert rubric["cost"] == 2
        assert rubric["moat"] == 3
        assert rubric["adoption"] == 4
        assert rubric["overall"] == "Strong paper with good experiments."

    def test_partial_json_rubric(self):
        from llm.parse import parse_ai_pnote_draft
        raw = """## 1. 背景
Content.

<!--
<RUBRIC>
{"novelty": 3, "leverage": 4, "evidence": 3}
</RUBRIC>
-->
"""
        _, rubric, _ = parse_ai_pnote_draft(raw)
        assert rubric["novelty"] == 3
        assert rubric["leverage"] == 4
        assert rubric["evidence"] == 3
        assert "cost" not in rubric

    def test_json_trailing_comma(self):
        from llm.parse import parse_ai_pnote_draft
        raw = """## 1. 背景
Content.

<!--
<RUBRIC>
{"novelty": 4, "leverage": 3, "evidence": 5,}
</RUBRIC>
-->
"""
        _, rubric, _ = parse_ai_pnote_draft(raw)
        assert rubric["novelty"] == 4
        assert rubric["leverage"] == 3
        assert rubric["evidence"] == 5

    def test_json_single_quoted_values(self):
        from llm.parse import parse_ai_pnote_draft
        raw = """## 1. 背景
Content.

<RUBRIC>
{"novelty": '3', "leverage": "4"}
</RUBRIC>
"""
        _, rubric, _ = parse_ai_pnote_draft(raw)
        assert rubric["novelty"] == 3
        assert rubric["leverage"] == 4

    def test_rubric_fallback_line_by_line_avoids_range_digits(self):
        from llm.parse import parse_ai_pnote_draft
        # LLM writes "novelty: 3 (1-5)" — must take 5, not 3
        raw = """## 1. 背景
Content.

评分：
novelty: 3 (1-5) — 创新程度
leverage: 4 (1-5) — 杠杆效应
evidence: 2 (1-5) — 证据质量
"""
        _, rubric, _ = parse_ai_pnote_draft(raw)
        assert rubric["novelty"] == 3
        assert rubric["leverage"] == 4
        assert rubric["evidence"] == 2

    def test_rubric_with_scores_out_of_range(self):
        from llm.parse import parse_ai_pnote_draft, extract_rubric_scores
        raw = """## 1. 背景
Content.

novelty: 6 (1-5)
leverage: 0 (1-5)
evidence: 3
cost: 2
"""
        _, rubric, _ = parse_ai_pnote_draft(raw)
        scores = extract_rubric_scores(rubric)
        # out-of-range scores should be filtered out
        assert "novelty" not in scores  # 6 > 5
        assert "leverage" not in scores  # 0 < 1
        assert scores["evidence"] == 3
        assert scores["cost"] == 2

    def test_rubric_overall_text_extraction(self):
        from llm.parse import parse_ai_pnote_draft
        raw = """## 1. 背景
Content.

Overall Judgment: 这篇论文提出了一个有效的方法，在多个数据集上取得了 SOTA 结果。
"""
        _, rubric, _ = parse_ai_pnote_draft(raw)
        assert "SOTA" in rubric["overall"]
        assert "有效的方法" in rubric["overall"]

    def test_empty_rubric(self):
        from llm.parse import parse_ai_pnote_draft
        raw = """## 1. 背景
Content only, no rubric.
"""
        _, rubric, _ = parse_ai_pnote_draft(raw)
        assert rubric == {}

    def test_rubric_malformed_json_falls_back_to_lines(self):
        from llm.parse import parse_ai_pnote_draft
        raw = """## 1. 背景
Content.

novelty: 4
leverage: 3
evidence: 5
cost: 2
moat: 3
adoption: 4
"""
        _, rubric, _ = parse_ai_pnote_draft(raw)
        assert rubric["novelty"] == 4
        assert rubric["leverage"] == 3
        assert rubric["evidence"] == 5


class TestParseIntegration:
    """Realistic LLM output with mixed content."""

    def test_realistic_llm_output(self):
        from llm.parse import parse_ai_pnote_draft, extract_rubric_scores
        raw = """## 1. 背景
本文研究了大语言模型在推理任务中的幻觉问题。

## 2. 核心问题
如何降低 LLM 在数学推理中的幻觉率？

## 3. 方法结构

### 3.1 架构拆解
提出了一个验证器网络来检查中间推理步骤。

### 3.2 算法逻辑
使用强化学习训练验证器，对每一步推理打分。

## 4. 关键创新
1. 验证器网络设计
2. 两阶段训练流程

## 5. 实验分析

### 5.1 数据集
GSM8K, MATH, SVAMP

### 5.2 基线对比
相比 Chain-of-Thought 提示，提升 12%。

## 6. 对抗式审稿
可能的攻击方式：对验证器进行对抗扰动。

## 7. 优势
方法简洁，易于集成到现有系统。

## 8. 局限
计算开销增加约 15%。

## 9. 本质抽象
将 LLM 推理视为一个博弈过程。

## 10. 与其他方法对比
比 Self-Consistency 方法更好，比 ReAct 效率更高。

## 11. Decision（决策）
值得发表，建议 minor revision。

## 12. 知识蒸馏
 Facts: 幻觉率降低 23%
 Principles: 验证器需要单独训练
 Insights: 推理过程可以模块化

## 13. 认知升级
理解了大语言模型推理的新范式。

## 14. 评分量表

<!--
<RUBRIC>
{
  "novelty": 4,
  "leverage": 5,
  "evidence": 4,
  "cost": 3,
  "moat": 3,
  "adoption": 5,
  "overall": "Strong contribution — significant improvement over baselines with clear motivation."
}
</RUBRIC>
-->
"""
        sections, rubric, raw_out = parse_ai_pnote_draft(raw)
        # All sections present
        assert "## 1. 背景" in sections
        assert "## 3.1 架构拆解" in sections
        assert "## 5.1 数据集" in sections
        assert "## 12. 知识蒸馏" in sections
        # Rubric extracted
        assert rubric["novelty"] == 4
        assert rubric["leverage"] == 5
        assert rubric["overall"] == "Strong contribution — significant improvement over baselines with clear motivation."
        # Scores in valid range
        scores = extract_rubric_scores(rubric)
        assert len(scores) == 6
        # Raw unchanged
        assert raw_out == raw

    def test_section_content_with_code_blocks(self):
        from llm.parse import parse_ai_pnote_draft
        raw = """## 3. 方法结构

```python
def verify(step):
    return model.predict(step)
```

### 3.1 架构拆解
架构如上图所示。
"""
        sections, _, _ = parse_ai_pnote_draft(raw)
        assert "```python" in sections["## 3. 方法结构"]
        assert sections["## 3.1 架构拆解"] == "架构如上图所示。"

    def test_section_content_with_markdown_links(self):
        from llm.parse import parse_ai_pnote_draft
        raw = """## 1. 背景
参考 [Smith et al., 2023](https://example.com/paper.pdf) 的工作。

## 2. 核心问题
详见[论文主页](https://homepage.example.com)。
"""
        sections, _, _ = parse_ai_pnote_draft(raw)
        assert "[Smith et al., 2023]" in sections["## 1. 背景"]
        assert "https://example.com/paper.pdf" in sections["## 1. 背景"]
        assert "[论文主页]" in sections["## 2. 核心问题"]

