"""Tests for core/workflow.py."""
import sys
from pathlib import Path
from unittest.mock import MagicMock
import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))
from core.workflow import Workflow, register_workflow, get_workflow


class TestWorkflow:
    """Tests for Workflow class."""

    def test_init(self):
        wf = Workflow("test_workflow")
        assert wf.name == "test_workflow"
        assert wf.steps == []

    def test_add_step(self):
        wf = Workflow("test")
        mock_fn = MagicMock()
        wf.add_step(mock_fn, "step 1")
        assert len(wf.steps) == 1
        assert wf.steps[0][0] is mock_fn
        assert wf.steps[0][1] == "step 1"

    def test_add_multiple_steps(self):
        wf = Workflow("test")
        f1, f2, f3 = MagicMock(), MagicMock(), MagicMock()
        wf.add_step(f1, "first")
        wf.add_step(f2, "second")
        wf.add_step(f3, "third")
        assert len(wf.steps) == 3

    def test_run_calls_all_steps(self, capsys):
        wf = Workflow("test")
        f1, f2 = MagicMock(), MagicMock()
        wf.add_step(f1, "step one")
        wf.add_step(f2, "step two")
        wf.run()
        f1.assert_called_once()
        f2.assert_called_once()
        captured = capsys.readouterr()
        assert "step one" in captured.out
        assert "step two" in captured.out

    def test_run_sequential(self, capsys):
        wf = Workflow("test")
        order = []

        def make_func(name):
            def fn():
                order.append(name)
            return fn

        wf.add_step(make_func("A"), "A desc")
        wf.add_step(make_func("B"), "B desc")
        wf.run()
        assert order == ["A", "B"]

    def test_run_empty(self, capsys):
        wf = Workflow("empty")
        wf.run()
        # No error, no output
        captured = capsys.readouterr()
        assert captured.out == ""


class TestRegistry:
    """Tests for workflow registry functions."""

    def test_register_and_get(self):
        wf = Workflow("my_workflow")
        register_workflow("my_workflow", wf)
        retrieved = get_workflow("my_workflow")
        assert retrieved is wf

    def test_get_missing_returns_none(self):
        result = get_workflow("nonexistent_workflow_xyz")
        assert result is None

    def test_register_overwrites(self):
        wf1 = Workflow("w1")
        wf2 = Workflow("w2")
        register_workflow("shared_name", wf1)
        register_workflow("shared_name", wf2)
        assert get_workflow("shared_name") is wf2

    def test_multiple_workflows(self):
        wfs = {f"wf_{i}": Workflow(f"workflow_{i}") for i in range(5)}
        for name, wf in wfs.items():
            register_workflow(name, wf)
        for name, wf in wfs.items():
            assert get_workflow(name) is wf
