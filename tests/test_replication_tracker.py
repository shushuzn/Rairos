"""Tests for replication tracker."""
import pytest
import tempfile
import os
from pathlib import Path
from llm.replication_tracker import (
    ReplicationTracker,
    ReplicationAttempt,
    ReplicationReport,
)


class TestReplicationTracker:
    """Test ReplicationTracker."""

    @pytest.fixture
    def tracker(self, tmp_path):
        return ReplicationTracker(data_dir=tmp_path)

    def test_create_attempt(self, tracker):
        """Test creating a replication attempt."""
        attempt = tracker.create_attempt("p1", "Test Paper")
        assert attempt.paper_id == "p1"
        assert attempt.paper_title == "Test Paper"
        assert attempt.status == "in_progress"

    def test_update_attempt(self, tracker):
        """Test updating an attempt."""
        attempt = tracker.create_attempt("p1", "Test Paper")
        result = tracker.update_attempt(
            attempt.attempt_id,
            status="success",
            notes="All tests passed",
        )
        assert result is True

        updated = tracker.get_attempt(attempt.attempt_id)
        assert updated.status == "success"
        assert updated.notes == "All tests passed"

    def test_get_attempt(self, tracker):
        """Test getting an attempt by ID."""
        created = tracker.create_attempt("p1", "Test Paper")
        fetched = tracker.get_attempt(created.attempt_id)
        assert fetched is not None
        assert fetched.attempt_id == created.attempt_id

    def test_get_paper_attempts(self, tracker):
        """Test getting all attempts for a paper."""
        tracker.create_attempt("p1", "Paper One")
        tracker.create_attempt("p1", "Paper One")
        tracker.create_attempt("p2", "Paper Two")

        p1_attempts = tracker.get_paper_attempts("p1")
        assert len(p1_attempts) == 2

    def test_get_all_attempts(self, tracker):
        """Test getting all attempts."""
        tracker.create_attempt("p1", "Paper One")
        tracker.create_attempt("p2", "Paper Two")

        all_attempts = tracker.get_all_attempts()
        assert len(all_attempts) == 2

    def test_get_all_attempts_filtered(self, tracker):
        """Test filtering attempts by status."""
        a1 = tracker.create_attempt("p1", "Paper One")
        tracker.create_attempt("p2", "Paper Two")
        tracker.update_attempt(a1.attempt_id, status="success")

        success = tracker.get_all_attempts(status="success")
        assert len(success) == 1

    def test_get_statistics(self, tracker):
        """Test getting statistics."""
        a1 = tracker.create_attempt("p1", "Paper One")
        tracker.create_attempt("p2", "Paper Two")
        tracker.update_attempt(a1.attempt_id, status="success")

        stats = tracker.get_statistics()
        assert stats["total"] == 2
        assert stats["success"] == 1
        assert stats["failed"] == 0

    def test_generate_report(self, tracker):
        """Test generating a report."""
        attempt = tracker.create_attempt("p1", "Test Paper")
        tracker.update_attempt(
            attempt.attempt_id,
            status="success",
            notes="All good",
        )

        report = tracker.generate_report(attempt.attempt_id)
        assert report is not None
        assert "Successfully" in report.summary

    def test_render_text(self, tracker):
        """Test text rendering."""
        tracker.create_attempt("p1", "Paper One")

        output = tracker.render_text(tracker.get_all_attempts())
        assert "Replication Tracker" in output
        assert "Paper One" in output

    def test_render_markdown(self, tracker):
        """Test Markdown rendering."""
        tracker.create_attempt("p1", "Paper One")

        output = tracker.render_markdown(tracker.get_all_attempts())
        assert "# Replication Tracker" in output
        assert "Total" in output


class TestReplicationAttempt:
    """Test ReplicationAttempt."""

    def test_creation(self):
        """Test creating an attempt."""
        attempt = ReplicationAttempt(
            attempt_id="test1",
            paper_id="p1",
            paper_title="Test Paper",
        )
        assert attempt.attempt_id == "test1"
        assert attempt.status == "in_progress"
        assert len(attempt.differences) == 0


class TestReplicationReport:
    """Test ReplicationReport."""

    def test_creation(self):
        """Test creating a report."""
        attempt = ReplicationAttempt(
            attempt_id="test1",
            paper_id="p1",
            paper_title="Test",
        )
        report = ReplicationReport(attempt=attempt)
        assert report.attempt.attempt_id == "test1"
        assert len(report.findings) == 0
