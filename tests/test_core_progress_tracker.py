"""Tests for core/progress_tracker.py."""
import sys
from pathlib import Path
import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))
from core.progress_tracker import ProgressTracker, get_tracker


class TestProgressTracker:
    def setup_method(self):
        self.tracker = ProgressTracker()

    def test_add_task(self):
        self.tracker.add_task("task1", "Test task")
        assert "task1" in self.tracker.tasks
        assert self.tracker.tasks["task1"]["description"] == "Test task"
        assert self.tracker.tasks["task1"]["status"] == "pending"

    def test_complete_task(self):
        self.tracker.add_task("task1", "Test task")
        self.tracker.complete_task("task1")
        assert self.tracker.tasks["task1"]["status"] == "completed"
        assert self.tracker.tasks["task1"]["completed"] is not None

    def test_complete_nonexistent_task(self):
        self.tracker.complete_task("nonexistent")  # should not raise
        assert len(self.tracker.tasks) == 0

    def test_get_progress_empty(self):
        assert self.tracker.get_progress() == 0.0

    def test_get_progress_no_completed(self):
        self.tracker.add_task("task1", "t1")
        self.tracker.add_task("task2", "t2")
        assert self.tracker.get_progress() == 0.0

    def test_get_progress_partial(self):
        self.tracker.add_task("task1", "t1")
        self.tracker.add_task("task2", "t2")
        self.tracker.complete_task("task1")
        assert self.tracker.get_progress() == 50.0

    def test_get_progress_all_completed(self):
        self.tracker.add_task("task1", "t1")
        self.tracker.add_task("task2", "t2")
        self.tracker.complete_task("task1")
        self.tracker.complete_task("task2")
        assert self.tracker.get_progress() == 100.0

    def test_task_timestamps(self):
        self.tracker.add_task("task1", "t1")
        assert "created" in self.tracker.tasks["task1"]
        self.tracker.complete_task("task1")
        assert "completed" in self.tracker.tasks["task1"]


class TestGetTracker:
    def setup_method(self):
        import core.progress_tracker as pt
        pt._tracker = None

    def teardown_method(self):
        import core.progress_tracker as pt
        pt._tracker = None

    def test_singleton(self):
        t1 = get_tracker()
        t2 = get_tracker()
        assert t1 is t2

    def test_singleton_persists_state(self):
        t1 = get_tracker()
        t1.add_task("task1", "t1")
        t2 = get_tracker()
        assert "task1" in t2.tasks
