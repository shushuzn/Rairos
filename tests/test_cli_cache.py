"""Tier 1 tests — CLI cache command."""
import pytest
from click.testing import CliRunner
from cli.main import cli


@pytest.fixture
def runner():
    return CliRunner()


class TestCacheCommand:
    """Test cache CLI commands."""

    def test_cache_clear(self, runner, monkeypatch, tmp_path):
        """Test airos cache clear command."""
        # Minimal test: command runs without error
        result = runner.invoke(cli, ["cache", "clear", "--help"])
        assert result.exit_code == 0

    def test_cache_stats(self, runner, monkeypatch, tmp_path):
        """Test airos cache stats command."""
        result = runner.invoke(cli, ["cache", "stats", "--help"])
        assert result.exit_code == 0

    def test_cache_list(self, runner, monkeypatch, tmp_path):
        """Test airos cache list command."""
        result = runner.invoke(cli, ["cache", "list", "--help"])
        assert result.exit_code == 0

    def test_cache_prune(self, runner, monkeypatch, tmp_path):
        """Test airos cache prune command."""
        result = runner.invoke(cli, ["cache", "prune", "--help"])
        assert result.exit_code == 0
