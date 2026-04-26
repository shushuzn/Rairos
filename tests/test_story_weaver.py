"""Tests for research story weaver."""
import pytest

from llm.story_weaver import (
    StoryWeaver,
    NarrativeRole,
    RelationshipType,
    PaperNarrative,
    Chapter,
    Relationship,
    StoryResult,
)


class TestStoryWeaver:
    """Test StoryWeaver."""

    def test_empty_result_when_no_db(self):
        """Test empty result when no DB available."""
        weaver = StoryWeaver(db=None)
        result = weaver.weave("nonexistent_topic_xyz")

        assert isinstance(result, StoryResult)
        assert result.topic == "nonexistent_topic_xyz"
        assert len(result.chapters) == 0

    def test_narrative_role_enum(self):
        """Test NarrativeRole enum values."""
        assert NarrativeRole.PROTAGONIST.value == "protagonist"
        assert NarrativeRole.ANTAGONIST.value == "antagonist"
        assert NarrativeRole.TURNING_POINT.value == "turning_point"
        assert NarrativeRole.DIVERGENCE.value == "divergence"
        assert NarrativeRole.SYNTHESIS.value == "synthesis"

    def test_relationship_type_enum(self):
        """Test RelationshipType enum values."""
        assert RelationshipType.INHERITS.value == "inherits"
        assert RelationshipType.EXTENDS.value == "extends"
        assert RelationshipType.CONTRASTS.value == "contrasts"
        assert RelationshipType.CONTRADICTS.value == "contradicts"
        assert RelationshipType.SYNTHESIZES.value == "synthesizes"

    def test_paper_narrative_creation(self):
        """Test PaperNarrative dataclass."""
        narrative = PaperNarrative(
            paper_id="1234.5678",
            title="Attention Is All You Need",
            year=2017,
            role=NarrativeRole.TURNING_POINT,
            core_contribution="Transformer architecture",
            key_insight="Attention mechanism is sufficient",
            turning_point_type="Paradigm shift",
        )

        assert narrative.paper_id == "1234.5678"
        assert narrative.year == 2017
        assert narrative.role == NarrativeRole.TURNING_POINT
        assert "Transformer" in narrative.core_contribution

    def test_chapter_creation(self):
        """Test Chapter dataclass."""
        chapter = Chapter(
            title="The Transformer Era",
            time_range=(2017, 2020),
            papers=[],
            summary="Revolution in NLP",
        )

        assert chapter.title == "The Transformer Era"
        assert chapter.time_range == (2017, 2020)

    def test_relationship_creation(self):
        """Test Relationship dataclass."""
        rel = Relationship(
            from_paper="paper1",
            to_paper="paper2",
            relationship=RelationshipType.EXTENDS,
            description="Extends the original work",
        )

        assert rel.from_paper == "paper1"
        assert rel.relationship == RelationshipType.EXTENDS

    def test_determine_role_turning_point(self):
        """Test role detection for turning points."""
        weaver = StoryWeaver()
        text = "This is a breakthrough that revolutionizes the field"

        role = weaver._determine_role(text, 2020)
        assert role == NarrativeRole.TURNING_POINT

    def test_determine_role_divergence(self):
        """Test role detection for divergences."""
        weaver = StoryWeaver()
        text = "We propose an alternative approach instead of the standard method"

        role = weaver._determine_role(text, 2021)
        assert role == NarrativeRole.DIVERGENCE

    def test_determine_role_protagonsit(self):
        """Test role detection for protagonists."""
        weaver = StoryWeaver()
        text = "We present a new method for classification"

        role = weaver._determine_role(text, 2020)
        assert role == NarrativeRole.PROTAGONIST

    def test_extract_contribution(self):
        """Test contribution extraction."""
        weaver = StoryWeaver()
        paper = {
            "title": "BERT: Pre-training of Deep Bidirectional Transformers",
            "abstract": "We propose a new method for language understanding.",
        }

        contribution = weaver._extract_contribution(paper)
        assert contribution  # Should extract something

    def test_extract_insight(self):
        """Test insight extraction."""
        weaver = StoryWeaver()
        text = "we find that bidirectional attention significantly improves performance"

        insight = weaver._extract_insight(text)
        assert "find" in insight.lower() or "bidirectional" in insight.lower()

    def test_detect_turning_point_breakthrough(self):
        """Test turning point detection for breakthroughs."""
        weaver = StoryWeaver()
        text = "This is a breakthrough that revolutionizes the field"

        turning_type = weaver._detect_turning_point(text)
        assert turning_type in ["颠覆性突破", "范式转变"]

    def test_detect_turning_point_sota(self):
        """Test turning point detection for SOTA claims."""
        weaver = StoryWeaver()
        text = "Our method achieves state-of-the-art results"

        turning_type = weaver._detect_turning_point(text)
        assert "性能突破" in turning_type or "SOTA" in turning_type

    def test_detect_turning_point_none(self):
        """Test turning point detection with no match."""
        weaver = StoryWeaver()
        text = "This paper presents an interesting approach"

        turning_type = weaver._detect_turning_point(text)
        assert turning_type == ""

    def test_infer_relationship_extends(self):
        """Test relationship inference for extensions."""
        weaver = StoryWeaver()
        a = PaperNarrative(
            paper_id="1", title="Original Work", year=2017,
            role=NarrativeRole.PROTAGONIST, core_contribution="",
            key_insight=""
        )
        b = PaperNarrative(
            paper_id="2", title="Building on Original Work", year=2019,
            role=NarrativeRole.PROTAGONIST, core_contribution="",
            key_insight=""
        )

        rel_type, desc = weaver._infer_relationship(a, b)
        # Should infer some relationship
        assert rel_type is not None or "original" in desc.lower()

    def test_organize_chapters(self):
        """Test chapter organization by time period."""
        weaver = StoryWeaver()
        narratives = [
            PaperNarrative(paper_id="1", title="Early Work", year=2015,
                          role=NarrativeRole.PROTAGONIST, core_contribution="", key_insight=""),
            PaperNarrative(paper_id="2", title="Transformer", year=2017,
                          role=NarrativeRole.TURNING_POINT, core_contribution="", key_insight=""),
            PaperNarrative(paper_id="3", title="BERT", year=2018,
                          role=NarrativeRole.PROTAGONIST, core_contribution="", key_insight=""),
        ]

        chapters = weaver._organize_chapters(narratives)

        assert len(chapters) >= 1
        # Should have chapter for 2015-2017 or similar
        chapter_years = [c.time_range for c in chapters]
        assert any(2015 <= yr <= 2017 for rng in chapter_years for yr in rng)

    def test_find_contradictions(self):
        """Test contradiction detection."""
        weaver = StoryWeaver()
        narratives = [
            PaperNarrative(paper_id="1", title="Efficient Small Model", year=2023,
                          role=NarrativeRole.PROTAGONIST, core_contribution="", key_insight=""),
            PaperNarrative(paper_id="2", title="Large Scale Model", year=2024,
                          role=NarrativeRole.PROTAGONIST, core_contribution="", key_insight=""),
        ]

        contradictions = weaver._find_contradictions(narratives)

        # Should find efficiency vs scale contradiction
        assert len(contradictions) > 0

    def test_identify_themes(self):
        """Test theme identification."""
        weaver = StoryWeaver()
        narratives = [
            PaperNarrative(paper_id="1", title="Attention Mechanism Survey", year=2022,
                          role=NarrativeRole.PROTAGONIST, core_contribution="", key_insight=""),
            PaperNarrative(paper_id="2", title="Self-Attention Analysis", year=2023,
                          role=NarrativeRole.PROTAGONIST, core_contribution="", key_insight=""),
        ]

        themes = weaver._identify_themes(narratives)

        assert "Attention 机制" in themes

    def test_render_result(self):
        """Test story rendering."""
        weaver = StoryWeaver()
        result = StoryResult(
            topic="Transformer",
            chapters=[
                Chapter(
                    title="The Attention Era",
                    time_range=(2017, 2020),
                    papers=[
                        PaperNarrative(
                            paper_id="1",
                            title="Attention Is All You Need",
                            year=2017,
                            role=NarrativeRole.TURNING_POINT,
                            core_contribution="Transformer",
                            key_insight="Attention is all you need",
                            turning_point_type="Paradigm shift",
                        )
                    ],
                )
            ],
            themes=["Attention 机制", "规模化"],
            contradictions=[("Efficient", "Large Scale")],
            summary="Revolution in NLP",
        )

        output = weaver.render_result(result)

        assert "Transformer" in output
        assert "Attention Era" in output
        assert "2017" in output
        assert "Attention 机制" in output

    def test_render_mermaid(self):
        """Test Mermaid flowchart rendering."""
        weaver = StoryWeaver()
        result = StoryResult(
            topic="LLM",
            chapters=[
                Chapter(
                    title="Language Models",
                    time_range=(2018, 2024),
                    papers=[
                        PaperNarrative(
                            paper_id="1",
                            title="BERT",
                            year=2018,
                            role=NarrativeRole.PROTAGONIST,
                            core_contribution="",
                            key_insight="",
                        )
                    ],
                )
            ],
        )

        output = weaver.render_mermaid(result)

        assert "mermaid" in output
        assert "LLM" in output
        assert "flowchart" in output

    def test_empty_result(self):
        """Test empty result handling."""
        weaver = StoryWeaver()
        result = weaver._empty_result("Test Topic")

        assert result.topic == "Test Topic"
        assert len(result.chapters) == 0
        assert len(result.relationships) == 0

    def test_compare(self):
        """Test story comparison."""
        weaver = StoryWeaver(db=None)
        story_a = StoryResult(
            topic="BERT",
            chapters=[Chapter(title="Pre-training Era", time_range=(2018, 2020), papers=[])],
            themes=["Pre-training"],
        )
        story_b = StoryResult(
            topic="GPT",
            chapters=[Chapter(title="Autoregressive Era", time_range=(2018, 2020), papers=[])],
            themes=["Autoregressive"],
        )

        # We need to mock the weave method to avoid DB access
        weaver.weave = lambda topic, **kw: story_a if topic == "BERT" else story_b

        comparison = weaver.compare("BERT", "GPT", use_llm=False)

        assert "BERT" in comparison
        assert "GPT" in comparison
        assert "故事线对比" in comparison
