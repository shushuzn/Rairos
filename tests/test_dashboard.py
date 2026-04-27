"""Tests for research dashboard."""
import pytest
from unittest.mock import Mock, patch

from llm.dashboard import (
    Dashboard,
    DashboardData,
    QuestionSummary,
    ExperimentSummary,
    PaperStats,
)


class TestDashboard:
    """Test Dashboard."""

    @pytest.fixture
    def dashboard(self):
        """Create dashboard without DB."""
        return Dashboard(db=None)

    def test_collect_empty(self, dashboard):
        """Test collecting with no data."""
        # QuestionTracker and ExperimentTracker are imported into llm.dashboard
        # from their source modules, so patch at source.
        with patch("llm.question_tracker.QuestionTracker") as mock_qt, \
             patch("llm.experiment_tracker.ExperimentTracker") as mock_et:
            mock_qt_instance = mock_qt.return_value
            mock_qt_instance.list_questions.return_value = []
            mock_et_instance = mock_et.return_value
            mock_et_instance.list_experiments.return_value = []

            data = dashboard.collect(include_papers=False)

            assert data.generated_at != ""
            assert len(data.questions) == 0
            assert len(data.experiments) == 0

    def test_collect_empty_data(self, dashboard):
        """Test collecting with no tracker data."""
        # Just test that collect works without crashing
        data = dashboard.collect(include_papers=False)
        assert data is not None
        assert isinstance(data, DashboardData)

    def test_build_summary(self, dashboard):
        """Test summary building."""
        data = DashboardData()
        data.questions = [
            QuestionSummary(id="q1", question="Q1", status="open", priority="high",
                         hypotheses_count=1, roadmap_id=""),
            QuestionSummary(id="q2", question="Q2", status="resolved", priority="medium",
                         hypotheses_count=2, roadmap_id=""),
        ]
        data.experiments = [
            ExperimentSummary(id="e1", name="E1", status="running", milestone="m1", metrics_count=0),
            ExperimentSummary(id="e2", name="E2", status="completed", milestone="m1", metrics_count=1),
        ]

        summary = dashboard._build_summary(data)

        assert summary["total_questions"] == 2
        assert summary["questions_by_status"]["open"] == 1
        assert summary["questions_by_status"]["resolved"] == 1
        assert summary["total_experiments"] == 2
        assert summary["experiments_by_status"]["running"] == 1

    def test_render_text_empty(self, dashboard):
        """Test text rendering with no data."""
        data = DashboardData()
        output = dashboard.render_text(data)

        assert "Research Dashboard" in output
        assert "Questions: 0" in output
        assert "Experiments: 0" in output

    def test_render_text_with_data(self, dashboard):
        """Test text rendering with data."""
        data = DashboardData()
        data.questions = [
            QuestionSummary(id="q1", question="Test question?", status="open",
                         priority="high", hypotheses_count=2, roadmap_id=""),
        ]
        data.experiments = [
            ExperimentSummary(id="e1", name="Test exp", status="running",
                           milestone="m1", metrics_count=3),
        ]
        data.summary = dashboard._build_summary(data)

        output = dashboard.render_text(data)

        assert "Research Dashboard" in output
        assert "Questions: 1" in output
        assert "Experiments: 1" in output
        assert "Test question?" in output
        assert "Test exp" in output

    def test_render_json(self, dashboard):
        """Test JSON rendering."""
        import json

        data = DashboardData()
        data.questions = [
            QuestionSummary(id="q1", question="Q1?", status="open",
                         priority="high", hypotheses_count=1, roadmap_id=""),
        ]

        output = dashboard.render_json(data)
        parsed = json.loads(output)

        assert "generated_at" in parsed
        assert len(parsed["questions"]) == 1
        # summary may be empty if _build_summary not called


class TestQuestionSummary:
    """Test QuestionSummary dataclass."""

    def test_creation(self):
        """Test creating QuestionSummary."""
        qs = QuestionSummary(
            id="q1",
            question="Test?",
            status="open",
            priority="high",
            hypotheses_count=3,
            roadmap_id="r1",
        )

        assert qs.id == "q1"
        assert qs.status == "open"
        assert qs.hypotheses_count == 3


class TestExperimentSummary:
    """Test ExperimentSummary dataclass."""

    def test_creation(self):
        """Test creating ExperimentSummary."""
        es = ExperimentSummary(
            id="e1",
            name="Test",
            status="running",
            milestone="m1",
            metrics_count=5,
        )

        assert es.id == "e1"
        assert es.metrics_count == 5


class TestPaperStats:
    """Test PaperStats dataclass."""

    def test_creation(self):
        """Test creating PaperStats."""
        ps = PaperStats(
            total_papers=100,
            recent_papers=10,
            by_year={"2024": 50, "2023": 30},
            by_tag={"RAG": 20, "LLM": 15},
        )

        assert ps.total_papers == 100
        assert ps.by_year["2024"] == 50
