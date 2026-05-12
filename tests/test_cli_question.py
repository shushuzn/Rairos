"""Unit tests for question CLI subcommand — research question tracking."""

from unittest.mock import patch


class FakeArgs:
    def __init__(self, **kwargs):
        for k, v in kwargs.items():
            setattr(self, k, v)


# ─────────────────────────────────────────────────────────────────────────────
# Parser tests
# ─────────────────────────────────────────────────────────────────────────────


class TestQuestionParser:
    def test_parser_creates_question_subparser(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.question import _build_question_parser
        import argparse

        p = argparse.ArgumentParser()
        sub = p.add_subparsers()
        result = _build_question_parser(sub)
        assert result is not None
        assert "research questions" in result.description.lower()

    def test_parser_has_all_subcommand_actions(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.question import _build_question_parser
        import argparse

        p = argparse.ArgumentParser()
        sub = p.add_subparsers()
        result = _build_question_parser(sub)
        # The question parser has a nested subparsers for actions (dest="action")
        # Find the action subparser
        action_sub = None
        for a in result._actions:
            if hasattr(a, "choices") and a.choices and "list" in a.choices:
                action_sub = a.choices
                break
        assert action_sub is not None, "No action subparser found"
        # Verify all 9 expected actions exist
        expected_actions = {
            "list",
            "add",
            "get",
            "update",
            "link",
            "unlink",
            "delete",
            "sync",
            "stats",
        }
        assert expected_actions == set(action_sub.keys()), (
            f"Missing actions: {expected_actions - set(action_sub.keys())}"
        )
        # Verify list subparser has expected arguments
        list_parser = action_sub["list"]
        list_actions = {a.dest: a for a in list_parser._actions}
        assert "status" in list_actions
        assert "topic" in list_actions
        # Verify add subparser has expected arguments
        add_parser = action_sub["add"]
        add_actions = {a.dest: a for a in add_parser._actions}
        assert "question" in add_actions  # positional


# ─────────────────────────────────────────────────────────────────────────────
# _run_question unit tests
# ─────────────────────────────────────────────────────────────────────────────


class TestRunQuestion:
    def test_list_action_calls_tracker(self, monkeypatch, capsys):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.question import _run_question

        class FakeQuestionTracker:
            def list_questions(self, status=None, topic=None, source=None):
                return []

            def render_list(self, questions, verbose=False):
                return "No questions."

        args = FakeArgs(
            action="list",
            status=None,
            topic=None,
            source=None,
            verbose=False,
            question=None,
            priority=5,
            notes=None,
            id=None,
            paper_id=None,
        )
        with patch("cli.cmd.question.QuestionTracker") as MockTracker:
            MockTracker.return_value = FakeQuestionTracker()
            rc = _run_question(args)
            assert rc == 0

    def test_stats_action_calls_tracker(self, monkeypatch, capsys):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.question import _run_question

        class FakeQuestionTracker:
            def get_stats(self):
                return {
                    "total": 0,
                    "answered": 0,
                    "unanswered": 0,
                    "by_status": {},
                    "by_source": {},
                    "by_topic": {},
                }

        args = FakeArgs(
            action="stats",
            status=None,
            topic=None,
            source=None,
            verbose=False,
            question=None,
            priority=5,
            notes=None,
            id=None,
            paper_id=None,
        )
        with patch("cli.cmd.question.QuestionTracker") as MockTracker:
            MockTracker.return_value = FakeQuestionTracker()
            rc = _run_question(args)
            assert rc == 0

    def test_add_action_calls_tracker(self, monkeypatch, capsys):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.question import _run_question

        class FakeQuestionTracker:
            def add(self, question, source, topic, priority, notes):
                # Capture params before entering class body (which creates a local scope)
                _q, _src, _pri = question, source, priority

                class FakeQ:
                    id = "q-1"
                    question = _q
                    source = _src
                    priority = _pri

                return FakeQ()

        args = FakeArgs(
            action="add",
            status=None,
            topic=None,
            source=None,
            verbose=False,
            question="What is attention?",
            priority=5,
            notes="",
            id=None,
            paper_id=None,
        )
        with patch("cli.cmd.question.QuestionTracker") as MockTracker:
            MockTracker.return_value = FakeQuestionTracker()
            rc = _run_question(args)
            assert rc == 0

    def test_sync_action_calls_tracker(self, monkeypatch, capsys):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.question import _run_question

        class FakeQuestionTracker:
            def sync_from_gaps(self, gaps=None, topic=None, priority=7):
                return []  # no new questions

        args = FakeArgs(
            action="sync",
            status=None,
            topic=None,
            source=None,
            verbose=False,
            question=None,
            priority=7,
            notes=None,
            id=None,
            paper_id=None,
        )
        with patch("cli.cmd.question.QuestionTracker") as MockTracker:
            MockTracker.return_value = FakeQuestionTracker()
            rc = _run_question(args)
            assert rc == 0
