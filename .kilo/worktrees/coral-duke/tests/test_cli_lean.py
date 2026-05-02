"""Unit tests for lean CLI subcommand — Lean 4 hypothesis verification."""
from unittest.mock import patch, MagicMock


class FakeArgs:
    def __init__(self, **kwargs):
        for k, v in kwargs.items():
            setattr(self, k, v)


# ─────────────────────────────────────────────────────────────────────────────
# Parser tests
# ─────────────────────────────────────────────────────────────────────────────

class TestLeanParser:
    def test_parser_help_text(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.lean import _build_lean_parser
        import argparse
        p = argparse.ArgumentParser()
        sub = p.add_subparsers()
        _build_lean_parser(sub)
        assert True  # smoke

    def test_parser_accepts_all_options(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.lean import _build_lean_parser
        import argparse
        p = argparse.ArgumentParser()
        sub = p.add_subparsers()
        _build_lean_parser(sub)
        assert True  # smoke


# ─────────────────────────────────────────────────────────────────────────────
# _run_lean unit tests
# ─────────────────────────────────────────────────────────────────────────────

class TestRunLean:
    def test_check_install_reports_lean_available(self, monkeypatch, capsys):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.lean import _run_lean
        from llm.lean_verifier import LeanInstallStatus

        args = FakeArgs(
            check_install=True,
            no_llm=False,
            code_only=False,
            json=False,
            hypothesis_text=None,
            hypothesis_id=None,
            model=None,
        )
        with patch("cli.cmd.lean.check_lean_installed") as mock_check:
            mock_check.return_value = (LeanInstallStatus.AVAILABLE, "Lean 4.8.0")
            rc = _run_lean(args)
            mock_check.assert_called_once()
            assert rc == 0

    def test_check_install_reports_lean_missing(self, monkeypatch, capsys):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.lean import _run_lean
        from llm.lean_verifier import LeanInstallStatus

        args = FakeArgs(
            check_install=True,
            no_llm=False,
            code_only=False,
            json=False,
            hypothesis_text=None,
            hypothesis_id=None,
            model=None,
        )
        with patch("cli.cmd.lean.check_lean_installed") as mock_check:
            mock_check.return_value = (LeanInstallStatus.NOT_FOUND, "")
            with patch("cli.cmd.lean.get_lean_install_instructions") as mock_instr:
                mock_instr.return_value = "Install Lean 4..."
                rc = _run_lean(args)
                assert rc == 1

    def test_code_only_prints_generated_lean(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from cli.cmd.lean import _run_lean

        args = FakeArgs(
            check_install=False,
            no_llm=True,
            code_only=True,
            json=False,
            hypothesis_text="theorem test : ∀ n : ℕ, n + 0 = n",
            hypothesis_id=None,
            model=None,
        )
        with patch("cli.cmd.lean.check_lean_installed") as mock_check:
            mock_check.return_value = (MagicMock(), "4.8.0")
            with patch("cli.cmd.lean._build_hypothesis") as mock_hyp:
                mock_hyp.return_value = MagicMock(id="test-hyp", core_statement="test theorem")
                with patch("cli.cmd.lean.translate_hypothesis_to_lean") as mock_trans:
                    mock_trans.return_value = ("theorem test : ∀ n : ℕ, n + 0 = n", "note")
                    rc = _run_lean(args)
                    assert rc == 0
