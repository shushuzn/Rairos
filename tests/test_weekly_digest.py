"""Tests for weekly digest."""
import pytest
from unittest.mock import Mock, patch
from datetime import datetime

from llm.weekly_digest import WeeklyDigest, WeekData


class TestWeeklyDigest:
    """Test WeeklyDigest."""

    @pytest.fixture
    def digest(self):
        return WeeklyDigest()

    def test_collect_empty(self, digest):
        """Test collecting with no data."""
        data = digest.collect_week_data(days=7)
        # May have real data, just check structure
        assert hasattr(data, 'journal_entries')
        assert hasattr(data, 'experiments_started')

    def test_collect_with_data(self, digest):
        """Test collecting with data."""
        data = digest.collect_week_data(days=7)
        assert data.journal_entries >= 0
        assert isinstance(data.top_tags, list)

    def test_calculate_score(self, digest):
        """Test productivity score calculation."""
        data = WeekData(start_date="2024-01-01", end_date="2024-01-07")
        data.journal_entries = 4
        data.experiments_completed = 1
        data.questions_resolved = 1

        score = digest._calculate_productivity_score(data)
        # 4*5=20, 1*20=20, 1*15=15 = 55
        assert score == 55

    def test_generate_summary(self, digest):
        """Test summary generation."""
        data = WeekData(start_date="2024-01-01", end_date="2024-01-07")
        data.journal_entries = 5
        data.experiments_started = 2
        data.experiments_completed = 1
        data.questions_new = 3
        data.questions_resolved = 1
        data.mood_breakdown = {"productive": 3, "excited": 2}
        data.top_tags = [("ai", 5), ("ml", 3)]

        output = digest.generate_summary(data)

        assert "Weekly Research Digest" in output
        assert "Journal entries: 5" in output
        assert "Productivity Score" in output

    def test_render_markdown(self, digest):
        """Test Markdown rendering."""
        data = WeekData(start_date="2024-01-01", end_date="2024-01-07")
        data.journal_entries = 3

        output = digest.render_markdown(data)

        assert "# Weekly Research Digest" in output
        assert "Journal entries" in output


class TestWeekData:
    """Test WeekData."""

    def test_creation(self):
        """Test creating WeekData."""
        data = WeekData(start_date="2024-01-01", end_date="2024-01-07")
        assert data.start_date == "2024-01-01"
        assert data.end_date == "2024-01-07"
        assert data.journal_entries == 0
