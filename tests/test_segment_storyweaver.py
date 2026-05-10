"""Tests for sections/segment.py and llm/story_weaver.py."""

from sections.segment import looks_like_heading, _section_priority
from llm.story_weaver import (
    NarrativeRole,
    RelationshipType,
    PaperNarrative,
    Chapter,
    Relationship,
    StoryResult,
)


# ── sections/segment ─────────────────────────────────────────────────────────


class TestLooksLikeHeading:
    def test_numbered(self):
        assert looks_like_heading("1. Introduction") is True
        assert looks_like_heading("3.2.1 Subsection") is True

    def test_roman(self):
        assert looks_like_heading("IV. Experiments") is True

    def test_keywords(self):
        assert looks_like_heading("Introduction") is True
        assert looks_like_heading("ABSTRACT") is True
        assert looks_like_heading("Conclusion") is True

    def test_not_heading(self):
        assert looks_like_heading("Normal sentence here.") is False
        assert looks_like_heading("ab") is False


class TestSectionPriority:
    def test_abstract(self):
        assert _section_priority("Abstract") == 10

    def test_introduction(self):
        assert _section_priority("Introduction") == 9

    def test_conclusion(self):
        assert _section_priority("Conclusion") == 4

    def test_unknown(self):
        assert _section_priority("Fuzzy Wuzzy Section") == 2


# ── llm/story_weaver ─────────────────────────────────────────────────────────


class TestNarrativeRole:
    def test_values(self):
        assert NarrativeRole.PROTAGONIST is not None
        assert NarrativeRole.ANTAGONIST is not None


class TestRelationshipType:
    def test_values(self):
        assert RelationshipType.CITES is not None


class TestPaperNarrative:
    def test_fields(self):
        pn = PaperNarrative(
            paper_id="1234",
            title="Test",
            year=2024,
            role=NarrativeRole.PROTAGONIST,
            core_contribution="method",
            key_insight="finding",
        )
        assert pn.paper_id == "1234"
        assert pn.role == NarrativeRole.PROTAGONIST


class TestChapter:
    def test_fields(self):
        chapter = Chapter(title="Beginning", time_range="2020-2024", papers=["p1", "p2"])
        assert chapter.title == "Beginning"
        assert len(chapter.papers) == 2


class TestRelationship:
    def test_fields(self):
        rel = Relationship(from_paper="p1", to_paper="p2", relationship=RelationshipType.CITES)
        assert rel.from_paper == "p1"
        assert rel.relationship == RelationshipType.CITES


class TestStoryResult:
    def test_fields(self):
        chapter = Chapter("C1", "2024", ["p1"])
        result = StoryResult(
            topic="RL Evolution",
            chapters=[chapter],
            relationships=[],
            protagonist_arc="learning",
        )
        assert result.topic == "RL Evolution"
        assert len(result.chapters) == 1
