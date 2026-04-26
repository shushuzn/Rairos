"""Tests for research question tracker."""
import pytest
import tempfile
import os
from pathlib import Path

from llm.question_tracker import (
    QuestionTracker,
    ResearchQuestion,
    QuestionStatus,
    QuestionSource,
)


class TestResearchQuestion:
    """Test ResearchQuestion dataclass."""

    def test_question_creation(self):
        """Test question creation with defaults."""
        q = ResearchQuestion(
            id="test1",
            question="如何提升RAG的召回率？",
            source=QuestionSource.MANUAL.value,
            status=QuestionStatus.OPEN.value,
        )

        assert q.id == "test1"
        assert q.question == "如何提升RAG的召回率？"
        assert q.status == "open"
        assert q.related_papers == []
        assert q.priority == 5

    def test_question_with_fields(self):
        """Test question with all fields."""
        q = ResearchQuestion(
            id="test2",
            question="Test question",
            source=QuestionSource.GAP_DETECTION.value,
            status=QuestionStatus.IN_PROGRESS.value,
            topic="RAG",
            priority=8,
            notes="Test notes",
        )

        assert q.source == "gap_detection"
        assert q.topic == "RAG"
        assert q.priority == 8
        assert q.notes == "Test notes"

    def test_to_dict(self):
        """Test serialization to dict."""
        q = ResearchQuestion(
            id="test3",
            question="Test",
            source="manual",
            status="open",
        )

        d = q.to_dict()
        assert d["id"] == "test3"
        assert d["question"] == "Test"

    def test_from_dict(self):
        """Test deserialization from dict."""
        data = {
            "id": "test4",
            "question": "Test",
            "source": "manual",
            "status": "open",
            "related_papers": [],
            "created_at": "",
            "updated_at": "",
            "notes": "",
            "priority": 5,
            "topic": "",
        }

        q = ResearchQuestion.from_dict(data)
        assert q.id == "test4"


class TestQuestionTracker:
    """Test QuestionTracker."""

    def setup_method(self):
        """Create temp directory for each test."""
        self.temp_dir = tempfile.mkdtemp()
        self.tracker = QuestionTracker(data_dir=self.temp_dir)

    def teardown_method(self):
        """Clean up temp files."""
        import shutil
        shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_add_question(self):
        """Test adding a question."""
        q = self.tracker.add(
            question="如何提升检索质量？",
            source=QuestionSource.MANUAL.value,
            topic="RAG",
            priority=7,
        )

        assert q.id is not None
        assert q.question == "如何提升检索质量？"
        assert q.status == "open"

    def test_list_questions(self):
        """Test listing questions."""
        self.tracker.add("Question 1")
        self.tracker.add("Question 2")

        questions = self.tracker.list_questions()
        assert len(questions) == 2

    def test_list_questions_filter_status(self):
        """Test filtering by status."""
        q1 = self.tracker.add("Open question")
        self.tracker.add("Resolved question")

        self.tracker.update(q1.id, status=QuestionStatus.RESOLVED.value)

        open_qs = self.tracker.list_questions(status="open")
        assert len(open_qs) == 1
        assert "Resolved" in open_qs[0].question or "Resolved question" in open_qs[0].question

        resolved_qs = self.tracker.list_questions(status="resolved")
        assert len(resolved_qs) == 1

    def test_list_questions_filter_topic(self):
        """Test filtering by topic."""
        self.tracker.add("RAG question", topic="RAG")
        self.tracker.add("LLM question", topic="LLM")

        rag_qs = self.tracker.list_questions(topic="RAG")
        assert len(rag_qs) == 1

    def test_get_question(self):
        """Test getting a question by ID."""
        added = self.tracker.add("Test question")
        retrieved = self.tracker.get(added.id)

        assert retrieved is not None
        assert retrieved.id == added.id
        assert retrieved.question == "Test question"

    def test_get_nonexistent(self):
        """Test getting nonexistent question."""
        result = self.tracker.get("nonexistent")
        assert result is None

    def test_update_status(self):
        """Test updating question status."""
        q = self.tracker.add("Test")
        updated = self.tracker.update(q.id, status=QuestionStatus.RESOLVED.value)

        assert updated is not None
        assert updated.status == "resolved"

    def test_update_priority(self):
        """Test updating priority."""
        q = self.tracker.add("Test")
        updated = self.tracker.update(q.id, priority=9)

        assert updated is not None
        assert updated.priority == 9

    def test_update_priority_bounds(self):
        """Test priority is bounded 1-10."""
        q = self.tracker.add("Test")
        updated = self.tracker.update(q.id, priority=15)
        assert updated.priority == 10

        updated = self.tracker.update(q.id, priority=0)
        assert updated.priority == 1

    def test_link_paper(self):
        """Test linking a paper."""
        q = self.tracker.add("Test")
        updated = self.tracker.link_paper(q.id, "arxiv:1234.5678")

        assert updated is not None
        assert "arxiv:1234.5678" in updated.related_papers

    def test_link_paper_idempotent(self):
        """Test linking same paper twice is idempotent."""
        q = self.tracker.add("Test")
        self.tracker.link_paper(q.id, "arxiv:1234.5678")
        self.tracker.link_paper(q.id, "arxiv:1234.5678")

        updated = self.tracker.get(q.id)
        assert len(updated.related_papers) == 1

    def test_unlink_paper(self):
        """Test unlinking a paper."""
        q = self.tracker.add("Test")
        self.tracker.link_paper(q.id, "arxiv:1234.5678")
        self.tracker.unlink_paper(q.id, "arxiv:1234.5678")

        updated = self.tracker.get(q.id)
        assert len(updated.related_papers) == 0

    def test_delete_question(self):
        """Test deleting a question."""
        q = self.tracker.add("Test")
        deleted = self.tracker.delete(q.id)

        assert deleted is True
        assert self.tracker.get(q.id) is None

    def test_delete_nonexistent(self):
        """Test deleting nonexistent question."""
        result = self.tracker.delete("nonexistent")
        assert result is False

    def test_sync_from_gaps(self):
        """Test syncing questions from gap detection."""
        gaps = [
            "长文档检索效率问题",
            "知识一致性问题",
        ]

        new_qs = self.tracker.sync_from_gaps(gaps, topic="RAG", priority=8)

        assert len(new_qs) == 2
        assert all(q.source == "gap_detection" for q in new_qs)

    def test_sync_duplicate_prevention(self):
        """Test sync doesn't create duplicates."""
        gaps = ["Test gap question"]

        self.tracker.add("Test gap question?")  # Similar exists

        new_qs = self.tracker.sync_from_gaps(gaps)
        assert len(new_qs) == 0  # Prevented by similarity check

    def test_get_stats(self):
        """Test statistics calculation."""
        self.tracker.add("Q1", source=QuestionSource.MANUAL.value)
        self.tracker.add("Q2", source=QuestionSource.GAP_DETECTION.value)

        stats = self.tracker.get_stats()

        assert stats["total"] == 2
        assert stats["by_source"]["manual"] == 1
        assert stats["by_source"]["gap_detection"] == 1

    def test_render_list(self):
        """Test rendering questions as text."""
        self.tracker.add("Test question 1")
        self.tracker.add("Test question 2")

        questions = self.tracker.list_questions()
        output = self.tracker.render_list(questions)

        assert "Test question 1" in output
        assert "Test question 2" in output

    def test_render_empty(self):
        """Test rendering empty list."""
        output = self.tracker.render_list([])
        assert "没有找到" in output

    def test_render_verbose(self):
        """Test verbose rendering."""
        q = self.tracker.add("Test with notes", notes="These are detailed notes")
        self.tracker.link_paper(q.id, "arxiv:1234")

        questions = self.tracker.list_questions()
        output = self.tracker.render_list(questions, verbose=True)

        assert "notes" in output.lower()
