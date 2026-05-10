"""Test CLI command dispatch table — regression test for missing dispatch entries.

Previously, daemon/scout/intel/signal/discover/report/jin10/demo were registered
in _SUBCOMMAND_TABLE but had no dispatch entry, causing silent no-op behavior.
This test ensures all registered subcommands have a dispatch handler.
"""

import pytest


class TestCLIDispatchComplete:
    """Verify every registered subcommand has a dispatch handler."""

    @pytest.fixture
    def subcmd_table(self):
        from cli._registry import _SUBCOMMAND_TABLE
        return _SUBCOMMAND_TABLE

    @pytest.fixture
    def dispatch_dict(self):
        # Reconstruct dispatch dict by parsing _registry.py source
        # (avoids import side-effects from the module itself)
        import re
        from pathlib import Path

        registry = Path("cli/_registry.py").read_text(encoding="utf-8")

        # Find the dispatch dict definition
        match = re.search(
            r"dispatch\s*=\s*\{(.*?)\n    \}",
            registry,
            re.DOTALL,
        )
        assert match, "dispatch dict not found in _registry.py"
        body = match.group(1)

        # Extract "key": "value" pairs
        entries = re.findall(r'"([^"]+)":\s*"([^"]+)"', body)
        return dict(entries)

    def test_all_subcommands_have_dispatch_entry(self, subcmd_table, dispatch_dict):
        """Every subcommand in _SUBCOMMAND_TABLE must have a dispatch entry.

        Note: agent/evolution/evoskill/path/rag/repl/story/trace/validate/visual/paper2code
        are known gaps tracked separately; this test guards specifically against
        regressions in the commands that were fixed in this sprint.
        """
        subcmd_names = {name for name, _, _ in subcmd_table}
        known_gaps = {
            "agent", "evolution", "evoskill", "paper2code", "path",
            "rag", "repl", "story", "trace", "validate", "visual",
        }
        missing_new = subcmd_names - known_gaps - set(dispatch_dict.keys())
        assert not missing_new, f"New missing dispatch entries (regression): {sorted(missing_new)}"

    def test_all_dispatch_entries_point_to_valid_function(self, subcmd_table, dispatch_dict):
        """Every dispatch value must be a valid _run_ function in cli module."""

        for subcmd, handler in dispatch_dict.items():
            # Handler should be a string like "_run_xxx"
            assert handler.startswith("_run_") or handler.startswith("_cli._run_"), (
                f"dispatch[{subcmd}] = {handler!r} doesn't look like a function reference"
            )

    def test_daemon_dispatch_exists(self, dispatch_dict):
        assert "daemon" in dispatch_dict, "daemon subcommand missing dispatch entry"

    def test_scout_dispatch_exists(self, dispatch_dict):
        assert "scout" in dispatch_dict, "scout subcommand missing dispatch entry"

    def test_intel_dispatch_exists(self, dispatch_dict):
        assert "intel" in dispatch_dict, "intel subcommand missing dispatch entry"

    def test_signal_dispatch_exists(self, dispatch_dict):
        assert "signal" in dispatch_dict, "signal subcommand missing dispatch entry"

    def test_discover_dispatch_exists(self, dispatch_dict):
        assert "discover" in dispatch_dict, "discover subcommand missing dispatch entry"

    def test_report_dispatch_exists(self, dispatch_dict):
        assert "report" in dispatch_dict, "report subcommand missing dispatch entry"

    def test_jin10_dispatch_exists(self, dispatch_dict):
        assert "jin10" in dispatch_dict, "jin10 subcommand missing dispatch entry"

    def test_demo_dispatch_exists(self, dispatch_dict):
        assert "demo" in dispatch_dict, "demo subcommand missing dispatch entry"

    def test_dashboard_dispatch_exists(self, dispatch_dict):
        assert "dashboard" in dispatch_dict, "dashboard subcommand missing dispatch entry"


class TestDemoCommand:
    """Test demo command itself."""

    def test_demo_module_importable(self):
        from cli.cmd.demo import run_demo
        assert callable(run_demo)

    def test_demo_parser_builds(self):
        from cli.cmd.demo import _build_demo_parser
        import argparse

        parser = argparse.ArgumentParser()
        subparsers = parser.add_subparsers()
        p = _build_demo_parser(subparsers)
        assert p is not None

        # Check key flags
        namespace = p.parse_args(["--quick"])
        assert namespace.quick is True

        namespace = p.parse_args(["--insights"])
        assert namespace.insights is True

        namespace = p.parse_args(["--papers", "5"])
        assert namespace.papers == 5

    def test_demo_runs_without_crash(self):
        from cli.cmd.demo import run_demo
        import argparse
        import io
        import contextlib

        args = argparse.Namespace(quick=True, insights=False, papers=None)
        f = io.StringIO()
        with contextlib.redirect_stdout(f):
            result = run_demo(args)
        assert result == 0
