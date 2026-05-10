"""Tests for core/workflow.py — Workflow automation."""

from core.workflow import Workflow, register_workflow, get_workflow


class TestWorkflow:
    def test_init(self):
        w = Workflow("test")
        assert w.name == "test"
        assert w.steps == []

    def test_add_step(self):
        w = Workflow("test")
        calls = []

        def step1():
            calls.append(1)

        w.add_step(step1, "Step one")
        assert len(w.steps) == 1
        assert w.steps[0] == (step1, "Step one")

    def test_add_multiple_steps(self):
        w = Workflow("multi")
        calls = []

        def a():
            calls.append("a")

        def b():
            calls.append("b")

        def c():
            calls.append("c")

        w.add_step(a, "A")
        w.add_step(b, "B")
        w.add_step(c, "C")
        assert len(w.steps) == 3

    def test_run_executes_all_steps(self, capsys):
        w = Workflow("run_test")
        calls = []

        def s1():
            calls.append(1)

        def s2():
            calls.append(2)

        w.add_step(s1, "first")
        w.add_step(s2, "second")
        w.run()

        assert calls == [1, 2]

    def test_run_prints_descriptions(self, capsys):
        w = Workflow("print_test")
        w.add_step(lambda: None, "do thing")
        w.run()
        assert "do thing" in capsys.readouterr().out

    def test_run_empty_workflow(self, capsys):
        w = Workflow("empty")
        w.run()
        # No error, no output
        assert capsys.readouterr().out == ""


class TestRegisterWorkflow:
    def test_register_and_get(self):
        w = Workflow("regtest")
        register_workflow("my_workflow", w)
        result = get_workflow("my_workflow")
        assert result is w

    def test_get_nonexistent_returns_none(self):
        result = get_workflow("does_not_exist")
        assert result is None

    def test_register_overwrites(self):
        w1 = Workflow("first")
        w2 = Workflow("second")
        register_workflow("overwrite", w1)
        register_workflow("overwrite", w2)
        assert get_workflow("overwrite") is w2
