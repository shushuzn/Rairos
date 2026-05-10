"""Tests for llm/cross_referencer.py — contradiction/synergy detection."""

import pytest
from unittest.mock import patch, MagicMock

from llm.cross_referencer import (
    CrossReferenceItem,
    CrossReferenceResult,
    CrossReferencer,
    _CROSS_REF_SYSTEM_PROMPT,
    _CROSS_REF_USER_TEMPLATE,
)


class TestCrossReferenceItem:
    """Test CrossReferenceItem dataclass."""

    def test_creates_with_required_fields(self):
        """Should create with required fields."""
        item = CrossReferenceItem(
            relation="contradiction",
            target_paper_id="p1",
            target_title="Paper One",
            description="They disagree on X",
        )
        assert item.relation == "contradiction"
        assert item.target_paper_id == "p1"
        assert item.confidence == 0.5  # default
        assert item.evidence == ""  # default

    def test_defaults(self):
        """Should have default values."""
        item = CrossReferenceItem(
            relation="alignment",
            target_paper_id="p2",
            target_title="Paper Two",
            description="They agree",
        )
        assert item.confidence == 0.5
        assert item.evidence == ""


class TestCrossReferenceResult:
    """Test CrossReferenceResult dataclass."""

    def test_default_values(self):
        """Should have sensible defaults."""
        result = CrossReferenceResult(paper_id="p1")
        assert result.related_papers_found == 0
        assert result.items == []
        assert result.used_fallback is False
        assert result.error == ""

    def test_creates_with_values(self):
        """Should accept all fields."""
        item = CrossReferenceItem(
            relation="extension",
            target_paper_id="p2",
            target_title="P2",
            description="Extends P1",
        )
        result = CrossReferenceResult(
            paper_id="p1",
            related_papers_found=1,
            items=[item],
            used_fallback=True,
        )
        assert result.related_papers_found == 1
        assert len(result.items) == 1


class TestCrossReferencer:
    """Test CrossReferencer class."""

    def test_returns_error_when_no_db(self):
        """Should return error when no database."""
        referencer = CrossReferencer(db=None)
        result = referencer.analyze("p1", "Title", "Abstract", "Body text")
        assert result.error != ""
        assert result.used_fallback is True

    def test_returns_empty_when_no_candidates(self):
        """Should return empty result when no candidate papers."""
        mock_db = MagicMock()
        mock_db.get_papers_by_tag.return_value = []
        referencer = CrossReferencer(db=mock_db)
        result = referencer.analyze("p1", "Title", "Abstract", "Body text", tags=["AI"])
        assert result.related_papers_found == 0
        assert result.items == []

    def test_calls_llm_when_api_key_present(self):
        """Should use LLM path when api_key is provided."""
        mock_db = MagicMock()
        mock_rec = MagicMock()
        mock_rec.id = "p2"
        mock_rec.title = "Existing Paper"
        mock_rec.abstract = "Abstract"
        mock_rec.tags = ["AI"]
        mock_db.get_papers_by_tag.return_value = [mock_rec]

        referencer = CrossReferencer(db=mock_db, llm_config={"api_key": "secret"})
        with patch("llm.client.call_llm_chat_completions") as mock_call:
            mock_call.return_value = '[{"total_related": 1}]'
            _result = referencer.analyze("p1", "Title", "Abstract", "Body", tags=["AI"])
            mock_call.assert_called_once()

    def test_fallback_when_no_api_key(self):
        """Should use fallback when no api_key."""
        mock_db = MagicMock()
        mock_rec = MagicMock()
        mock_rec.id = "p2"
        mock_rec.title = "Existing Paper"
        mock_rec.abstract = "Abstract"
        mock_rec.tags = ["AI"]
        mock_db.get_papers_by_tag.return_value = [mock_rec]

        referencer = CrossReferencer(db=mock_db, llm_config={})
        result = referencer.analyze("p1", "Title", "Abstract", "Body", tags=["AI"])
        assert result.used_fallback is True

    def test_exception_when_api_call_fails(self):
        """Should raise when LLM call fails (no exception handling in current impl)."""
        mock_db = MagicMock()
        mock_rec = MagicMock()
        mock_rec.id = "p2"
        mock_rec.title = "Existing Paper"
        mock_rec.abstract = "Abstract"
        mock_rec.tags = ["AI"]
        mock_db.get_papers_by_tag.return_value = [mock_rec]

        referencer = CrossReferencer(db=mock_db, llm_config={"api_key": "secret"})
        with patch("llm.client.call_llm_chat_completions", side_effect=Exception("API error")):
            with pytest.raises(Exception, match="API error"):
                referencer.analyze("p1", "Title", "Abstract", "Body", tags=["AI"])


class TestFindCandidates:
    """Test _find_candidates()."""

    def test_finds_by_tag(self):
        """Should find papers sharing tags."""
        mock_db = MagicMock()
        mock_rec = MagicMock()
        mock_rec.id = "p2"
        mock_rec.title = "Related Paper"
        mock_rec.abstract = "Abstract"
        mock_rec.tags = ["AI", "ML"]
        mock_db.get_papers_by_tag.return_value = [mock_rec]

        referencer = CrossReferencer(db=mock_db)
        candidates = referencer._find_candidates("p1", ["AI"])
        assert len(candidates) == 1
        assert candidates[0]["id"] == "p2"

    def test_deduplicates_by_id(self):
        """Should not return duplicate papers."""
        mock_db = MagicMock()
        mock_rec1 = MagicMock()
        mock_rec1.id = "p2"
        mock_rec1.title = "Paper 2"
        mock_rec1.abstract = ""
        mock_rec1.tags = ["AI"]
        mock_rec2 = MagicMock()
        mock_rec2.id = "p2"  # Same ID via different tag
        mock_rec2.title = "Paper 2 Duplicate"
        mock_rec2.abstract = ""
        mock_rec2.tags = ["ML"]
        mock_db.get_papers_by_tag.side_effect = [[mock_rec1], [mock_rec2]]

        referencer = CrossReferencer(db=mock_db)
        candidates = referencer._find_candidates("p1", ["AI", "ML"])
        assert len(candidates) == 1  # deduplicated

    def test_excludes_target_paper(self):
        """Should exclude the target paper itself."""
        mock_db = MagicMock()
        mock_rec = MagicMock()
        mock_rec.id = "p1"  # same as target
        mock_rec.title = "Same Paper"
        mock_rec.abstract = ""
        mock_rec.tags = ["AI"]
        mock_db.get_papers_by_tag.return_value = [mock_rec]

        referencer = CrossReferencer(db=mock_db)
        candidates = referencer._find_candidates("p1", ["AI"])
        assert len(candidates) == 0

    def test_limits_max_candidates(self):
        """Should cap at max_candidates."""
        mock_db = MagicMock()
        mock_db.get_papers_by_tag.return_value = [
            MagicMock(id=f"p{i}", title=f"Paper {i}", abstract="", tags=["AI"]) for i in range(20)
        ]

        referencer = CrossReferencer(db=mock_db)
        candidates = referencer._find_candidates("p1", ["AI"], max_candidates=5)
        assert len(candidates) == 5


class TestParseResponse:
    """Test _parse_response()."""

    def test_parses_relation_blocks(self):
        """Should parse [paper_id] (relation) blocks."""
        referencer = CrossReferencer()
        raw = """
        [p2] (contradiction)
        description: The methods contradict each other
        evidence: Page 5 states X, but this paper states Y
        confidence: 高

        [p3] (alignment)
        description: Supports the findings
        """
        candidates = [
            {"id": "p2", "title": "Paper 2"},
            {"id": "p3", "title": "Paper 3"},
        ]
        items = referencer._parse_response(raw, candidates)
        assert len(items) == 2
        assert items[0].relation == "contradiction"
        assert items[1].relation == "alignment"

    def test_handles_plain_relation_format(self):
        """Should handle paper_id (relation) without brackets."""
        referencer = CrossReferencer()
        raw = "p2 (alignment)\np3 (extension)"
        candidates = [
            {"id": "p2", "title": "Paper 2"},
            {"id": "p3", "title": "Paper 3"},
        ]
        items = referencer._parse_response(raw, candidates)
        assert len(items) == 2

    def test_falls_back_for_unknown_relations(self):
        """Should create alignment items as fallback when relation is unknown."""
        referencer = CrossReferencer()
        raw = "[p2] (unknown_relation)"
        candidates = [{"id": "p2", "title": "Paper 2"}]
        items = referencer._parse_response(raw, candidates)
        # Unknown relation → skipped → fallback creates alignment item
        assert len(items) == 1
        assert items[0].relation == "alignment"

    def test_falls_back_to_generic_items(self):
        """Should create alignment items when no structured response."""
        referencer = CrossReferencer()
        raw = "Some free-form response without structure"
        candidates = [
            {"id": "p2", "title": "Paper 2"},
            {"id": "p3", "title": "Paper 3"},
        ]
        items = referencer._parse_response(raw, candidates)
        assert len(items) == 2
        assert all(item.relation == "alignment" for item in items)
        assert all(item.confidence == 0.3 for item in items)


class TestAnalyzeFallback:
    """Test _analyze_fallback()."""

    def test_returns_alignment_items(self):
        """Should return alignment items from fallback."""
        referencer = CrossReferencer()
        candidates = [
            {"id": "p2", "title": "Paper 2", "abstract": "Abstract"},
        ]
        result = referencer._analyze_fallback("p1", candidates)
        assert result.used_fallback is True
        assert len(result.items) == 1
        assert result.items[0].relation == "alignment"

    def test_limits_to_5_candidates(self):
        """Should limit fallback to 5 candidates."""
        referencer = CrossReferencer()
        candidates = [MagicMock(id=f"p{i}", title=f"P{i}", abstract="") for i in range(10)]
        result = referencer._analyze_fallback("p1", candidates)
        assert len(result.items) == 5


class TestPrompts:
    """Test prompt strings."""

    def test_system_prompt_is_chinese(self):
        """System prompt should be in Chinese."""
        assert "矛盾" in _CROSS_REF_SYSTEM_PROMPT
        assert "AI 研究助理" in _CROSS_REF_SYSTEM_PROMPT

    def test_user_template_has_placeholders(self):
        """User template should have all required placeholders."""
        placeholders = [
            "{target_title}",
            "{target_tags}",
            "{target_abstract}",
            "{reference_papers}",
        ]
        for ph in placeholders:
            assert ph in _CROSS_REF_USER_TEMPLATE, f"Missing: {ph}"

    def test_user_template_lists_relations(self):
        """Template should list all valid relations."""
        for rel in ["contradiction", "alignment", "extension", "unrelated"]:
            assert rel in _CROSS_REF_USER_TEMPLATE
