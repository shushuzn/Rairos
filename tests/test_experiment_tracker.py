"""Tests for experiment tracker."""
import pytest
import tempfile
import os
from pathlib import Path

from llm.experiment_tracker import (
    ExperimentTracker,
    Experiment,
    ExperimentStatus,
    Metric,
)


class TestExperimentTracker:
    """Test ExperimentTracker."""

    @pytest.fixture
    def tracker(self, tmp_path):
        """Create tracker with temp directory."""
        return ExperimentTracker(data_dir=str(tmp_path))

    def test_run_experiment(self, tracker):
        """Test creating a new experiment."""
        e = tracker.run(
            name="Test Experiment",
            description="A test experiment",
            roadmap_milestone="m1",
            tags=["test", "unit"],
        )

        assert e.name == "Test Experiment"
        assert e.description == "A test experiment"
        assert e.roadmap_milestone == "m1"
        assert e.status == "running"
        assert len(e.id) == 8
        assert "test" in e.tags

    def test_get_experiment(self, tracker):
        """Test retrieving an experiment."""
        created = tracker.run(name="Get Test")
        retrieved = tracker.get(created.id)

        assert retrieved is not None
        assert retrieved.id == created.id
        assert retrieved.name == "Get Test"

    def test_get_nonexistent(self, tracker):
        """Test getting non-existent experiment."""
        result = tracker.get("nonexistent")
        assert result is None

    def test_list_experiments(self, tracker):
        """Test listing experiments."""
        tracker.run(name="Exp 1")
        tracker.run(name="Exp 2")
        tracker.run(name="Exp 3")

        all_exps = tracker.list_experiments()
        assert len(all_exps) == 3

    def test_list_by_status(self, tracker):
        """Test filtering by status."""
        e1 = tracker.run(name="Running Exp")
        tracker.run(name="Completed Exp")

        tracker.complete(e1.id)

        running = tracker.list_experiments(status="running")
        completed = tracker.list_experiments(status="completed")

        assert len(running) == 1
        assert len(completed) == 1
        assert running[0].name == "Completed Exp"
        assert completed[0].name == "Running Exp"

    def test_list_by_milestone(self, tracker):
        """Test filtering by milestone."""
        tracker.run(name="Exp 1", roadmap_milestone="m1")
        tracker.run(name="Exp 2", roadmap_milestone="m2")
        tracker.run(name="Exp 3", roadmap_milestone="m1")

        m1_exps = tracker.list_experiments(milestone="m1")
        assert len(m1_exps) == 2

    def test_list_by_tag(self, tracker):
        """Test filtering by tag."""
        tracker.run(name="Exp 1", tags=["alpha"])
        tracker.run(name="Exp 2", tags=["beta"])
        tracker.run(name="Exp 3", tags=["alpha", "beta"])

        alpha_exps = tracker.list_experiments(tag="alpha")
        assert len(alpha_exps) == 2

    def test_complete_experiment(self, tracker):
        """Test completing an experiment."""
        e = tracker.run(name="Complete Test")
        completed = tracker.complete(e.id, results={"accuracy": 0.95})

        assert completed.status == "completed"
        assert completed.results["accuracy"] == 0.95
        assert completed.completed_at != ""

    def test_fail_experiment(self, tracker):
        """Test failing an experiment."""
        e = tracker.run(name="Fail Test")
        failed = tracker.fail(e.id, error="Out of memory")

        assert failed.status == "failed"
        assert failed.results["error"] == "Out of memory"

    def test_add_metric(self, tracker):
        """Test adding metrics."""
        e = tracker.run(name="Metric Test")
        tracker.add_metric(e.id, "accuracy", 0.92, unit="%")
        tracker.add_metric(e.id, "latency", 150, unit="ms")

        e = tracker.get(e.id)
        assert len(e.metrics) == 2
        assert e.metrics[0].name == "accuracy"
        assert e.metrics[0].value == 0.92

    def test_compare_experiments(self, tracker):
        """Test comparing experiments."""
        e1 = tracker.run(name="Exp A")
        e2 = tracker.run(name="Exp B")

        tracker.add_metric(e1.id, "accuracy", 0.90)
        tracker.add_metric(e1.id, "f1", 0.85)
        tracker.add_metric(e2.id, "accuracy", 0.92)
        tracker.add_metric(e2.id, "f1", 0.88)

        comp = tracker.compare([e1.id, e2.id])

        assert "metrics" in comp
        assert "experiments" in comp
        assert len(comp["experiments"]) == 2

    def test_compare_specific_metrics(self, tracker):
        """Test comparing specific metrics only."""
        e1 = tracker.run(name="Exp A")
        e2 = tracker.run(name="Exp B")

        tracker.add_metric(e1.id, "accuracy", 0.90)
        tracker.add_metric(e1.id, "f1", 0.85)
        tracker.add_metric(e2.id, "accuracy", 0.92)
        tracker.add_metric(e2.id, "f1", 0.88)

        comp = tracker.compare([e1.id, e2.id], metric_names=["accuracy"])

        assert comp["metrics"] == ["accuracy"]
        assert len(comp["experiments"]) == 2
        assert comp["experiments"][0]["accuracy"] == 0.90
        assert comp["experiments"][1]["accuracy"] == 0.92

    def test_delete_experiment(self, tracker):
        """Test deleting an experiment."""
        e = tracker.run(name="Delete Test")
        deleted = tracker.delete(e.id)

        assert deleted is True
        assert tracker.get(e.id) is None

    def test_delete_nonexistent(self, tracker):
        """Test deleting non-existent experiment."""
        result = tracker.delete("nonexistent")
        assert result is False

    def test_render_list(self, tracker):
        """Test rendering experiment list."""
        tracker.run(name="List Test 1")
        tracker.run(name="List Test 2")

        exps = tracker.list_experiments()
        output = tracker.render_list(exps)

        assert "List Test 1" in output
        assert "List Test 2" in output

    def test_render_compare(self, tracker):
        """Test rendering comparison table."""
        e1 = tracker.run(name="Exp A")
        e2 = tracker.run(name="Exp B")

        tracker.add_metric(e1.id, "accuracy", 0.90)
        tracker.add_metric(e2.id, "accuracy", 0.92)

        comp = tracker.compare([e1.id, e2.id])
        output = tracker.render_compare(comp)

        assert "## Experiment Comparison" in output
        assert "accuracy" in output

    def test_render_compare_error(self, tracker):
        """Test rendering comparison with error."""
        comp = {"error": "No experiments found"}
        output = tracker.render_compare(comp)

        assert "Error" in output


class TestMetric:
    """Test Metric dataclass."""

    def test_metric_creation(self):
        """Test creating a metric."""
        m = Metric(name="accuracy", value=0.95, unit="%")

        assert m.name == "accuracy"
        assert m.value == 0.95
        assert m.unit == "%"


class TestExperiment:
    """Test Experiment dataclass."""

    def test_experiment_creation(self):
        """Test creating an experiment."""
        e = Experiment(
            id="test123",
            name="Test Experiment",
            description="A test",
        )

        assert e.id == "test123"
        assert e.name == "Test Experiment"
        assert e.status == "running"
        assert e.created_at != ""

    def test_experiment_to_dict(self):
        """Test converting to dictionary."""
        e = Experiment(id="test456", name="Dict Test")
        d = e.to_dict()

        assert d["id"] == "test456"
        assert d["name"] == "Dict Test"

    def test_experiment_from_dict(self):
        """Test creating from dictionary."""
        data = {
            "id": "test789",
            "name": "From Dict",
            "status": "completed",
            "config": {},
            "results": {},
            "metrics": [],
            "artifacts": [],
            "tags": [],
            "created_at": "2024-01-01T00:00:00",
            "completed_at": "",
            "description": "",
            "roadmap_milestone": "",
        }
        e = Experiment.from_dict(data)

        assert e.id == "test789"
        assert e.name == "From Dict"

    def test_experiment_from_dict_with_metrics(self):
        """Test creating from dict with metrics."""
        data = {
            "id": "test-metric",
            "name": "Metric Test",
            "status": "running",
            "config": {},
            "results": {},
            "metrics": [
                {"name": "accuracy", "value": 0.92, "unit": "%"}
            ],
            "artifacts": [],
            "tags": [],
            "created_at": "",
            "completed_at": "",
            "description": "",
            "roadmap_milestone": "",
        }
        e = Experiment.from_dict(data)

        assert len(e.metrics) == 1
        assert e.metrics[0].name == "accuracy"


class TestExperimentStatus:
    """Test ExperimentStatus enum."""

    def test_status_values(self):
        """Test status enum values."""
        assert ExperimentStatus.RUNNING.value == "running"
        assert ExperimentStatus.COMPLETED.value == "completed"
        assert ExperimentStatus.FAILED.value == "failed"
        assert ExperimentStatus.CANCELLED.value == "cancelled"
