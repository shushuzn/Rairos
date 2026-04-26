"""Tests for research journal."""
import pytest
import tempfile
from pathlib import Path

from llm.journal import Journal, JournalEntry


class TestJournal:
    """Test Journal."""

    @pytest.fixture
    def journal(self, tmp_path):
        return Journal(data_dir=str(tmp_path))

    def test_add_entry(self, journal):
        """Test adding an entry."""
        entry = journal.add("Test journal entry", tags=["test"])
        assert entry.content == "Test journal entry"
        assert "test" in entry.tags
        assert len(entry.id) == 8

    def test_get_entry(self, journal):
        """Test getting an entry."""
        created = journal.add("Test")
        retrieved = journal.get(created.id)
        assert retrieved.id == created.id
        assert retrieved.content == "Test"

    def test_get_nonexistent(self, journal):
        """Test getting non-existent entry."""
        assert journal.get("nonexistent") is None

    def test_update_entry(self, journal):
        """Test updating an entry."""
        entry = journal.add("Original content")
        updated = journal.update(entry.id, content="Updated content")
        assert updated.content == "Updated content"
        assert updated.updated_at != ""

    def test_delete_entry(self, journal):
        """Test deleting an entry."""
        entry = journal.add("To delete")
        assert journal.delete(entry.id) is True
        assert journal.get(entry.id) is None

    def test_list_entries(self, journal):
        """Test listing entries."""
        journal.add("Entry 1")
        journal.add("Entry 2")
        entries = journal.list_entries()
        assert len(entries) == 2

    def test_list_with_tag(self, journal):
        """Test filtering by tag."""
        journal.add("Entry 1", tags=["important"])
        journal.add("Entry 2", tags=["normal"])
        entries = journal.list_entries(tag="important")
        assert len(entries) == 1
        assert entries[0].content == "Entry 1"

    def test_list_today(self, journal):
        """Test today's entries."""
        entry = journal.add("Today's entry")
        entries = journal.list_entries(today=True)
        assert len(entries) == 1

    def test_list_by_days(self, journal):
        """Test entries by days."""
        journal.add("Recent entry")
        entries = journal.list_entries(days=7)
        assert len(entries) == 1

    def test_list_by_question(self, journal):
        """Test filtering by question."""
        journal.add("Linked to question", question_id="q123")
        journal.add("Not linked")
        entries = journal.list_entries(question_id="q123")
        assert len(entries) == 1

    def test_list_by_experiment(self, journal):
        """Test filtering by experiment."""
        journal.add("Linked to exp", experiment_id="e456")
        entries = journal.list_entries(experiment_id="e456")
        assert len(entries) == 1

    def test_search(self, journal):
        """Test searching entries."""
        journal.add("RAG is interesting")
        journal.add("Transformer architecture")
        results = journal.search("RAG")
        assert len(results) == 1
        assert "RAG" in results[0].content

    def test_stats(self, journal):
        """Test statistics."""
        journal.add("Entry 1", tags=["test", "ai"])
        journal.add("Entry 2", tags=["test"])
        stats = journal.stats()
        assert stats["total"] == 2
        assert stats["this_week"] == 2
        assert stats["top_tags"][0] == ("test", 2)

    def test_render_list(self, journal):
        """Test rendering entries."""
        journal.add("Test entry", mood="productive", tags=["test"])
        entries = journal.list_entries()
        output = journal.render_list(entries)
        assert "Test entry" in output
        assert "⚡" in output

    def test_render_list_verbose(self, journal):
        """Test verbose rendering."""
        journal.add("Verbose entry", tags=["important"], question_id="q1")
        entries = journal.list_entries()
        output = journal.render_list(entries, verbose=True)
        assert "important" in output
        assert "q1" in output


class TestJournalEntry:
    """Test JournalEntry."""

    def test_creation(self):
        """Test creating an entry."""
        entry = JournalEntry(id="test123", content="Test")
        assert entry.id == "test123"
        assert entry.created_at != ""

    def test_to_dict(self):
        """Test converting to dict."""
        entry = JournalEntry(id="test", content="Content")
        d = entry.to_dict()
        assert d["id"] == "test"
        assert d["content"] == "Content"

    def test_from_dict(self):
        """Test creating from dict."""
        data = {"id": "e1", "content": "Test", "created_at": "2024-01-01T00:00:00",
                "updated_at": "", "tags": [], "question_id": "", "experiment_id": "",
                "paper_id": "", "mood": "", "highlights": []}
        entry = JournalEntry.from_dict(data)
        assert entry.id == "e1"
