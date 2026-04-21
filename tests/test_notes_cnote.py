"""
Tests for notes/cnote.py — C-note creation, link management, and AI auto-fill.
Covers: ensure_cnote, upsert_link_under_heading, update_cnote_links,
        _section_is_empty, _fill_cnote_section, _parse_cnote_sections,
        auto_fill_cnotes_with_ai
"""
import re
from pathlib import Path
from unittest.mock import patch

import sys
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from notes.cnote import (
    ensure_cnote,
    upsert_link_under_heading,
    _section_is_empty,
    _fill_cnote_section,
    _parse_cnote_sections,
    auto_fill_cnotes_with_ai,
)


# ============================================================================
# Test ensure_cnote
# ============================================================================

class TestEnsureCnote:
    def test_creates_cnote_file(self, tmp_path):
        path = ensure_cnote(tmp_path, "TestConcept")
        assert path.name == "C - TestConcept.md"
        assert path.exists()

    def test_creates_correct_content(self, tmp_path):
        path = ensure_cnote(tmp_path, "TestConcept")
        content = path.read_text(encoding="utf-8")
        assert "# TestConcept" in content
        assert "## 核心定义" in content
        assert "## 产生背景" in content
        assert "## 技术本质" in content

    def test_does_not_overwrite_existing(self, tmp_path):
        existing = tmp_path / "C - Existing.md"
        existing.write_text("existing content", encoding="utf-8")
        path = ensure_cnote(tmp_path, "Existing")
        assert path.read_text(encoding="utf-8") == "existing content"

    def test_returns_path_even_if_exists(self, tmp_path):
        path = ensure_cnote(tmp_path, "TestConcept")
        path2 = ensure_cnote(tmp_path, "TestConcept")
        assert path == path2

    def test_concept_with_spaces(self, tmp_path):
        path = ensure_cnote(tmp_path, "Test Concept")
        assert path.name == "C - Test Concept.md"

    def test_renders_includes_status_frontmatter(self, tmp_path):
        path = ensure_cnote(tmp_path, "TestConcept")
        content = path.read_text(encoding="utf-8")
        assert "status:" in content


# ============================================================================
# Test upsert_link_under_heading
# ============================================================================

class TestUpsertLinkUnderHeading:
    def test_heading_not_exists_append_section(self):
        md = "# TestDoc\n\nSome content."
        result = upsert_link_under_heading(md, "## NewSection", "- [[Link]]")
        assert "## NewSection" in result
        assert "[[Link]]" in result

    def test_heading_not_exists_strips_hash_prefix(self):
        md = "# TestDoc"
        result = upsert_link_under_heading(md, "## NewSection", "- [[Link]]")
        assert "## NewSection" in result

    def test_heading_exists_appends_to_section(self):
        md = "# TestDoc\n\n## Target\n\n- [[OldLink]]\n\nMore content."
        result = upsert_link_under_heading(md, "## Target", "- [[NewLink]]")
        # New link should come before old content (old link removed)
        assert "[[NewLink]]" in result
        assert "[[OldLink]]" not in result

    def test_existing_wikilink_removed_before_insert(self):
        md = "## Section\n\n- [[OldLink]] some text"
        result = upsert_link_under_heading(md, "## Section", "- [[NewLink]]")
        assert "[[OldLink]]" not in result
        assert "[[NewLink]]" in result

    def test_section_with_no_existing_link(self):
        md = "## Section\n\nPlain content here."
        result = upsert_link_under_heading(md, "## Section", "- [[NewLink]]")
        assert "[[NewLink]]" in result
        assert "Plain content here" in result

    def test_section_at_end_of_file(self):
        md = "# Doc\n\n## LastSection\n\nSome text."
        result = upsert_link_under_heading(md, "## LastSection", "- [[Link]]")
        assert "[[Link]]" in result

    def test_preserves_content_after_section(self):
        md = "## Section\n\nContent.\n\n## NextSection\n\nMore."
        result = upsert_link_under_heading(md, "## Section", "- [[Link]]")
        assert "## NextSection" in result
        assert "[[Link]]" in result

    def test_multiple_wikilinks_removed(self):
        md = "## Section\n\n- [[Link1]]\n- [[Link2]]\n- Regular line"
        result = upsert_link_under_heading(md, "## Section", "- [[NewLink]]")
        assert "[[Link1]]" not in result
        assert "[[Link2]]" not in result
        assert "[[NewLink]]" in result
        assert "Regular line" in result

    def test_no_hash_prefix_in_heading_arg(self):
        md = "# Doc"
        result = upsert_link_under_heading(md, "MyHeading", "- [[Link]]")
        assert "## MyHeading" in result
        assert "[[Link]]" in result


# ============================================================================
# Test _section_is_empty
# ============================================================================

class TestSectionIsEmpty:
    def test_missing_section_returns_true(self):
        md = "# Doc\n\n## Other\n\nContent."
        assert _section_is_empty(md, "MissingSection") is True

    def test_empty_section_returns_true(self):
        md = "## Section\n\n"
        assert _section_is_empty(md, "Section") is True

    def test_only_dashes_returns_true(self):
        md = "## Section\n\n---"
        assert _section_is_empty(md, "Section") is True

    def test_only_em_dashes_returns_true(self):
        md = "## Section\n\n——"
        assert _section_is_empty(md, "Section") is True

    def test_only_whitespace_returns_true(self):
        md = "## Section\n\n   \n\t"
        assert _section_is_empty(md, "Section") is True

    def test_short_placeholder_no_punctuation_returns_true(self):
        md = "## Section\n\nTo be filled"  # no punctuation, short
        assert _section_is_empty(md, "Section") is True

    def test_short_with_punctuation_returns_false(self):
        md = "## Section\n\nTo be filled. Yes!"
        assert _section_is_empty(md, "Section") is False

    def test_normal_content_returns_false(self):
        md = "## Section\n\nThis is meaningful content with a sentence."
        assert _section_is_empty(md, "Section") is False

    def test_section_not_at_end_of_file(self):
        md = "## Section\n\nReal content.\n\n## Next\n\nMore."
        assert _section_is_empty(md, "Section") is False


# ============================================================================
# Test _fill_cnote_section
# ============================================================================

class TestFillCnoteSection:
    def test_replaces_existing_section_content(self):
        md = "## Section\n\nOld content here."
        result = _fill_cnote_section(md, "Section", "New content.")
        assert "New content." in result
        assert "Old content" not in result

    def test_appends_section_if_not_found(self):
        md = "# Doc\n\n## Other\n\nContent."
        result = _fill_cnote_section(md, "NewSection", "New content.")
        assert "## NewSection" in result
        assert "New content." in result

    def test_preserves_other_sections(self):
        md = "## Section1\n\nContent1.\n\n## Section2\n\nContent2."
        result = _fill_cnote_section(md, "Section1", "Updated1.")
        assert "Updated1." in result
        assert "Content2." in result

    def test_empty_new_content(self):
        md = "## Section\n\nOld content."
        result = _fill_cnote_section(md, "Section", "")
        assert "Old content" not in result

    def test_preserves_heading_format(self):
        md = "## Section\n\nOld."
        result = _fill_cnote_section(md, "Section", "New.")
        assert re.search(r"##\s+Section", result) is not None


# ============================================================================
# Test _parse_cnote_sections
# ============================================================================

class TestParseCnoteSections:
    def test_single_section(self):
        draft = "## Section\n\nContent here."
        result = _parse_cnote_sections(draft)
        assert "Section" in result
        assert result["Section"] == "Content here."

    def test_multiple_sections(self):
        draft = "## Section1\n\nContent1.\n\n## Section2\n\nContent2."
        result = _parse_cnote_sections(draft)
        assert result["Section1"] == "Content1."
        assert result["Section2"] == "Content2."

    def test_multiline_content(self):
        draft = "## Section\n\nLine1.\nLine2.\nLine3."
        result = _parse_cnote_sections(draft)
        assert "Line1." in result["Section"]
        assert "Line3." in result["Section"]

    def test_empty_draft_returns_empty_dict(self):
        result = _parse_cnote_sections("")
        assert result == {}

    def test_no_headings(self):
        draft = "Just some text without headings."
        result = _parse_cnote_sections(draft)
        assert result == {}

    def test_section_with_blank_lines(self):
        draft = "## Section\n\nLine1.\n\n\nLine2."
        result = _parse_cnote_sections(draft)
        assert result["Section"] == "Line1.\n\n\nLine2."

    def test_heading_with_no_content(self):
        draft = "## Section1\n\n## Section2\n\nContent."
        result = _parse_cnote_sections(draft)
        assert result["Section1"] == ""
        assert result["Section2"] == "Content."


# ============================================================================
# Test auto_fill_cnotes_with_ai
# ============================================================================

class TestAutoFillCnotesWithAi:
    def test_no_papers_skipped(self, tmp_path):
        # Create concept dir with no P-notes
        concept_dir = tmp_path / "01-Foundations"
        concept_dir.mkdir(parents=True)
        # Patch at notes.pnotes where it is imported from
        with patch("notes.pnotes.pnotes_by_tag", return_value={}):
            results = auto_fill_cnotes_with_ai(
                root=tmp_path,
                api_key="fake-key",
                base_url="https://api.example.com",
                model="test-model",
                min_papers=1,
            )
        assert results == []

    def test_below_min_papers_skipped(self, tmp_path):
        concept_dir = tmp_path / "01-Foundations"
        concept_dir.mkdir(parents=True)
        pnote_dir = tmp_path / "02-Papers"
        pnote_dir.mkdir(parents=True)
        # Create one P-note
        pnote = pnote_dir / "P - Test.md"
        pnote.write_text("tags: [test]\n\n# Test\n\nContent.", encoding="utf-8")
        with patch("notes.pnotes.pnotes_by_tag", return_value={"test": [("2024-01-01", pnote)]}):
            results = auto_fill_cnotes_with_ai(
                root=tmp_path,
                api_key="fake-key",
                base_url="https://api.example.com",
                model="test-model",
                min_papers=2,
            )
        assert ("test", "skipped") in results

    def test_all_sections_filled_skipped(self, tmp_path):
        concept_dir = tmp_path / "01-Foundations"
        concept_dir.mkdir(parents=True)
        pnote_dir = tmp_path / "02-Papers"
        pnote_dir.mkdir(parents=True)
        # C-note with all sections filled
        cnote = concept_dir / "C - Filled.md"
        cnote.write_text(
            "## 核心定义\nMeaningful content.\n\n"
            "## 产生背景\nBackground info.\n\n"
            "## 技术本质\nTechnical details.",
            encoding="utf-8",
        )
        pnote = pnote_dir / "P - Filled.md"
        pnote.write_text("tags: [filled]\n\n# Filled\n\nContent.", encoding="utf-8")
        with patch("notes.pnotes.pnotes_by_tag", return_value={"filled": [("2024-01-01", pnote)]}):
            with patch("notes.pnotes.read_pnote_metadata", return_value={"title": "Filled"}):
                results = auto_fill_cnotes_with_ai(
                    root=tmp_path,
                    api_key="fake-key",
                    base_url="https://api.example.com",
                    model="test-model",
                    min_papers=1,
                )
        assert ("filled", "skipped") in results

    def test_ai_generates_and_fills(self, tmp_path):
        concept_dir = tmp_path / "01-Foundations"
        concept_dir.mkdir(parents=True)
        pnote_dir = tmp_path / "02-Papers"
        pnote_dir.mkdir(parents=True)
        pnote = pnote_dir / "P - Concept.md"
        pnote.write_text("tags: [concept]\n\n# Concept\n\nContent.", encoding="utf-8")

        generated_draft = (
            "## 核心定义\nAI generated core definition.\n\n"
            "## 产生背景\nAI generated background.\n\n"
            "## 技术本质\nAI generated technical content."
        )

        with patch("notes.pnotes.pnotes_by_tag", return_value={"concept": [("2024-01-01", pnote)]}):
            with patch("notes.pnotes.read_pnote_metadata", return_value={"title": "Concept"}):
                with patch("llm.generate.ai_generate_cnote_draft", return_value=generated_draft) as mock_gen:
                    results = auto_fill_cnotes_with_ai(
                        root=tmp_path,
                        api_key="fake-key",
                        base_url="https://api.example.com",
                        model="test-model",
                        min_papers=1,
                    )

        assert ("concept", "filled") in results
        mock_gen.assert_called_once()

        # Verify C-note was updated
        cnote_path = concept_dir / "C - concept.md"
        content = cnote_path.read_text(encoding="utf-8")
        assert "AI generated core definition" in content

    def test_ai_failure_appends_failed(self, tmp_path):
        concept_dir = tmp_path / "01-Foundations"
        concept_dir.mkdir(parents=True)
        pnote_dir = tmp_path / "02-Papers"
        pnote_dir.mkdir(parents=True)
        pnote = pnote_dir / "P - Fail.md"
        pnote.write_text("tags: [fail]\n\n# Fail\n\nContent.", encoding="utf-8")

        with patch("notes.pnotes.pnotes_by_tag", return_value={"fail": [("2024-01-01", pnote)]}):
            with patch("notes.pnotes.read_pnote_metadata", return_value={"title": "Fail"}):
                with patch(
                    "llm.generate.ai_generate_cnote_draft",
                    side_effect=RuntimeError("API error"),
                ):
                    results = auto_fill_cnotes_with_ai(
                        root=tmp_path,
                        api_key="fake-key",
                        base_url="https://api.example.com",
                        model="test-model",
                        min_papers=1,
                    )
        assert ("fail", "failed") in results

    def test_existing_cnote_read_before_ai_call(self, tmp_path):
        concept_dir = tmp_path / "01-Foundations"
        concept_dir.mkdir(parents=True)
        pnote_dir = tmp_path / "02-Papers"
        pnote_dir.mkdir(parents=True)
        pnote = pnote_dir / "P - Existing.md"
        pnote.write_text("tags: [existing]\n\n# Existing\n\nContent.", encoding="utf-8")

        # All sections short and without sentence-ending punctuation -> considered empty
        # -> AI draft should be triggered and C-note updated
        existing_content = "## 核心定义\nExisting."  # short, no ending punctuation
        cnote = concept_dir / "C - Existing.md"
        cnote.write_text(existing_content, encoding="utf-8")

        with patch("notes.pnotes.pnotes_by_tag", return_value={"existing": [("2024-01-01", pnote)]}):
            with patch("notes.pnotes.read_pnote_metadata", return_value={"title": "Existing"}):
                with patch(
                    "llm.generate.ai_generate_cnote_draft",
                    return_value="## 核心定义\nNew core.\n\n## 产生背景\nBg.\n\n## 技术本质\nTech.",
                ):
                    results = auto_fill_cnotes_with_ai(
                        root=tmp_path,
                        api_key="fake-key",
                        base_url="https://api.example.com",
                        model="test-model",
                        min_papers=1,
                    )

        assert ("existing", "filled") in results

        # Verify C-note was updated with AI-generated content
        content = cnote.read_text(encoding="utf-8")
        assert "New core." in content

    def test_no_concept_dir_creates_it(self, tmp_path):
        pnote_dir = tmp_path / "02-Papers"
        pnote_dir.mkdir(parents=True)
        pnote = pnote_dir / "P - New.md"
        pnote.write_text("tags: [new]\n\n# New\n\nContent.", encoding="utf-8")

        with patch("notes.pnotes.pnotes_by_tag", return_value={"new": [("2024-01-01", pnote)]}):
            with patch("notes.pnotes.read_pnote_metadata", return_value={"title": "New"}):
                with patch(
                    "llm.generate.ai_generate_cnote_draft",
                    return_value="## 核心定义\nCore.\n\n## 产生背景\nBg.\n\n## 技术本质\nTech.",
                ):
                    results = auto_fill_cnotes_with_ai(
                        root=tmp_path,
                        api_key="fake-key",
                        base_url="https://api.example.com",
                        model="test-model",
                        min_papers=1,
                    )

        # Should succeed even though 01-Foundations didn't exist
        assert ("new", "filled") in results
        cnote_path = tmp_path / "01-Foundations" / "C - new.md"
        assert cnote_path.exists()
