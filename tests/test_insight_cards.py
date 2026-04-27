"""Tests for insight cards."""
import pytest
from pathlib import Path
from llm.insight_cards import (
    InsightManager,
    InsightCard,
    InsightCollection,
)


class TestInsightManager:
    """Test InsightManager."""

    @pytest.fixture
    def manager(self, tmp_path):
        return InsightManager(data_dir=tmp_path)

    def test_add_card(self, manager):
        """Test adding a card."""
        card = manager.add_card(
            paper_id="p1",
            paper_title="Test Paper",
            content="This is a key finding.",
        )
        assert card.card_id == "i0001"
        assert card.paper_id == "p1"
        assert card.content == "This is a key finding."

    def test_get_card(self, manager):
        """Test getting a card."""
        created = manager.add_card("p1", "Test", "Finding 1")
        fetched = manager.get_card(created.card_id)
        assert fetched is not None
        assert fetched.card_id == created.card_id

    def test_update_card(self, manager):
        """Test updating a card."""
        card = manager.add_card("p1", "Test", "Finding 1")
        result = manager.update_card(card.card_id, tags=["tag1", "tag2"])
        assert result is True

        updated = manager.get_card(card.card_id)
        assert "tag1" in updated.tags

    def test_search_by_query(self, manager):
        """Test searching by query."""
        manager.add_card("p1", "Paper 1", "BERT improves accuracy")
        manager.add_card("p2", "Paper 2", "GPT generates text")

        results = manager.search_cards(query="BERT")
        assert len(results) == 1
        assert "BERT" in results[0].content

    def test_search_by_tags(self, manager):
        """Test searching by tags."""
        manager.add_card("p1", "Paper 1", "Finding 1", tags=["nlp"])
        manager.add_card("p2", "Paper 2", "Finding 2", tags=["cv"])

        results = manager.search_cards(tags=["nlp"])
        assert len(results) == 1

    def test_search_by_type(self, manager):
        """Test searching by type."""
        manager.add_card("p1", "Paper 1", "Finding 1", insight_type="finding")
        manager.add_card("p2", "Paper 2", "Finding 2", insight_type="method")

        results = manager.search_cards(insight_type="method")
        assert len(results) == 1

    def test_search_case_insensitive(self, manager):
        """Query matching should be case-insensitive."""
        manager.add_card("p1", "Paper 1", "Deep Learning works well")
        results = manager.search_cards(query="deep")
        assert len(results) == 1
        results = manager.search_cards(query="DEEP LEARNING")
        assert len(results) == 1

    def test_search_query_matches_paper_title(self, manager):
        """Query should also search in paper_title."""
        manager.add_card("p1", "Attention Is All You Need", "Transformer model")
        results = manager.search_cards(query="attention")
        assert len(results) == 1

    def test_search_no_query_returns_all(self, manager):
        """No query returns all cards regardless of content."""
        manager.add_card("p1", "Paper 1", "Content A")
        manager.add_card("p2", "Paper 2", "Content B")
        results = manager.search_cards()
        assert len(results) == 2

    def test_search_no_results(self, manager):
        """No matching cards returns empty list."""
        manager.add_card("p1", "Paper 1", "BERT and NLP content")
        results = manager.search_cards(query="quantum computing")
        assert len(results) == 0
        assert results == []

    def test_search_multi_tag_or_logic(self, manager):
        """Multiple tags use OR logic: card matches if it has ANY of the tags."""
        manager.add_card("p1", "Paper 1", "Finding 1", tags=["nlp", "transformer"])
        manager.add_card("p2", "Paper 2", "Finding 2", tags=["nlp"])
        manager.add_card("p3", "Paper 3", "Finding 3", tags=["transformer"])
        manager.add_card("p4", "Paper 4", "Finding 4", tags=["cv"])
        results = manager.search_cards(tags=["nlp", "transformer"])
        # OR logic: all three cards match (each has at least one of the tags)
        assert len(results) == 3
        paper_ids = {r.paper_id for r in results}
        assert paper_ids == {"p1", "p2", "p3"}
        # Card with neither tag is excluded
        assert "p4" not in paper_ids

    def test_search_combined_criteria(self, manager):
        """Multiple filters apply together (AND between filter types)."""
        manager.add_card("p1", "Paper 1", "BERT finding", tags=["nlp"], insight_type="finding")
        manager.add_card("p2", "Paper 2", "BERT method", tags=["nlp"], insight_type="method")
        manager.add_card("p3", "Paper 3", "GPT finding", tags=["nlp"], insight_type="finding")
        results = manager.search_cards(query="BERT", tags=["nlp"], insight_type="method")
        assert len(results) == 1
        assert results[0].paper_id == "p2"

    def test_search_by_paper_id(self, manager):
        """Filter cards by paper_id."""
        manager.add_card("p1", "Paper 1", "Finding from paper 1")
        manager.add_card("p2", "Paper 2", "Finding from paper 2")
        results = manager.search_cards(paper_id="p1")
        assert len(results) == 1
        assert results[0].paper_id == "p1"

    def test_search_tags_empty_list(self, manager):
        """Empty tags list returns all cards (no filter)."""
        manager.add_card("p1", "Paper 1", "Finding 1", tags=["nlp"])
        manager.add_card("p2", "Paper 2", "Finding 2", tags=["cv"])
        results = manager.search_cards(tags=[])
        assert len(results) == 2

    def test_search_tags_nonexistent(self, manager):
        """Tag not in any card returns empty."""
        manager.add_card("p1", "Paper 1", "Finding 1", tags=["nlp"])
        results = manager.search_cards(tags=["nonexistent_tag"])
        assert len(results) == 0

    def test_add_reference(self, manager):
        """Test adding a reference between cards."""
        card1 = manager.add_card("p1", "Paper 1", "Finding 1")
        card2 = manager.add_card("p2", "Paper 2", "Finding 2")

        result = manager.add_reference(card1.card_id, card2.card_id)
        assert result is True

        updated = manager.get_card(card1.card_id)
        assert card2.card_id in updated.references

    def test_get_tag_cloud(self, manager):
        """Test getting tag cloud."""
        manager.add_card("p1", "Paper 1", "Finding 1", tags=["nlp", "transformer"])
        manager.add_card("p2", "Paper 2", "Finding 2", tags=["nlp", "bert"])

        cloud = manager.get_tag_cloud()
        assert cloud["nlp"] == 2
        assert cloud["transformer"] == 1

    def test_create_collection(self, manager):
        """Test creating a collection."""
        collection = manager.create_collection(
            title="RAG Insights",
            description="All RAG-related findings",
            tags=["rag"],
        )
        assert collection.collection_id == "c0001"
        assert collection.title == "RAG Insights"

    def test_render_text(self, manager):
        """Test text rendering."""
        manager.add_card("p1", "Test Paper", "Key finding here")

        cards = manager.search_cards()
        output = manager.render_text(cards)
        assert "Insight Cards" in output
        assert "Test Paper" in output

    def test_render_markdown(self, manager):
        """Test markdown rendering."""
        manager.add_card("p1", "Test Paper", "Key finding here")

        cards = manager.search_cards()
        output = manager.render_markdown(cards)
        assert "# Key Insight Cards" in output

    def test_export_for_note(self, manager):
        """Test exporting for notes."""
        manager.add_card("p1", "Test Paper", "Key finding", tags=["nlp"])

        cards = manager.search_cards()
        output = manager.export_for_note(cards)
        assert "[[p1]]" in output
        assert "#nlp" in output


class TestInsightCard:
    """Test InsightCard."""

    def test_creation(self):
        """Test creating a card."""
        card = InsightCard(
            card_id="i1",
            paper_id="p1",
            paper_title="Test",
            content="Finding",
        )
        assert card.card_id == "i1"
        assert card.insight_type == "finding"
        assert len(card.references) == 0


class TestInsightCollection:
    """Test InsightCollection."""

    def test_creation(self):
        """Test creating a collection."""
        collection = InsightCollection(
            collection_id="c1",
            title="Test Collection",
        )
        assert collection.collection_id == "c1"
        assert len(collection.card_ids) == 0
