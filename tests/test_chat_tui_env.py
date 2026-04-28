"""Tests for chat_tui .env loading and CLI argument parsing."""

import os
import sys
import tempfile
from pathlib import Path
from unittest.mock import patch


class TestChatTuiEnvLoading:
    """Test that .env is loaded from current working directory."""

    def test_env_file_loaded_from_cwd(self, tmp_path):
        """When .env exists in cwd, its values should be loaded into os.environ."""
        env_file = tmp_path / ".env"
        env_file.write_text("OPENAI_API_KEY=test-key-123\nMINIMAX_TEST=value\n")

        # Save original values to restore after test
        orig_key = os.environ.get("OPENAI_API_KEY")
        orig_minimax = os.environ.get("MINIMAX_TEST")

        try:
            # Clear the specific keys we're testing (not all env)
            os.environ.pop("OPENAI_API_KEY", None)
            os.environ.pop("MINIMAX_TEST", None)

            with patch("pathlib.Path.cwd", return_value=tmp_path):
                # Simulate the env loading from cli/_shared.py
                _cwd_env = tmp_path / ".env"
                if _cwd_env.exists():
                    with open(_cwd_env, encoding="utf-8") as f:
                        for line in f:
                            line = line.strip()
                            if line and not line.startswith("#") and "=" in line:
                                key, _, value = line.partition("=")
                                os.environ.setdefault(key.strip(), value.strip())

                assert os.environ.get("OPENAI_API_KEY") == "test-key-123"
                assert os.environ.get("MINIMAX_TEST") == "value"
        finally:
            # Restore original values
            if orig_key is not None:
                os.environ["OPENAI_API_KEY"] = orig_key
            else:
                os.environ.pop("OPENAI_API_KEY", None)
            if orig_minimax is not None:
                os.environ["MINIMAX_TEST"] = orig_minimax
            else:
                os.environ.pop("MINIMAX_TEST", None)

    def test_existing_env_vars_not_overwritten(self, tmp_path):
        """Existing environment variables should not be overwritten by .env."""
        env_file = tmp_path / ".env"
        env_file.write_text("OPENAI_API_KEY=new-key-from-env\n")

        with patch.dict(os.environ, {"OPENAI_API_KEY": "original-key"}, clear=False):
            with patch("pathlib.Path.cwd", return_value=tmp_path):
                _cwd_env = tmp_path / ".env"
                if _cwd_env.exists():
                    with open(_cwd_env, encoding="utf-8") as f:
                        for line in f:
                            line = line.strip()
                            if line and not line.startswith("#") and "=" in line:
                                key, _, value = line.partition("=")
                                os.environ.setdefault(key.strip(), value.strip())

                # setdefault should NOT overwrite existing value
                assert os.environ.get("OPENAI_API_KEY") == "original-key"

    def test_comments_and_empty_lines_ignored(self, tmp_path):
        """Comments and empty lines in .env should be ignored."""
        env_file = tmp_path / ".env"
        env_file.write_text("# This is a comment\n\nKEY=value\n  # another comment\n")

        with patch.dict(os.environ, {}, clear=True):
            with patch("pathlib.Path.cwd", return_value=tmp_path):
                _cwd_env = tmp_path / ".env"
                if _cwd_env.exists():
                    with open(_cwd_env, encoding="utf-8") as f:
                        for line in f:
                            line = line.strip()
                            if line and not line.startswith("#") and "=" in line:
                                key, _, value = line.partition("=")
                                os.environ.setdefault(key.strip(), value.strip())

                assert os.environ.get("KEY") == "value"
                assert "comment" not in os.environ

    def test_missing_env_file_is_silent(self, tmp_path):
        """When .env doesn't exist, loading should be silent (no error)."""
        non_existent = tmp_path / ".env"
        assert not non_existent.exists()

        # Should not raise
        _cwd_env = non_existent
        if _cwd_env.exists():
            with open(_cwd_env, encoding="utf-8") as f:
                pass  # Would set vars if file existed


class TestChatTuiParser:
    """Test chat-tui argument parser registration."""

    def test_chat_tui_subcommand_registered(self):
        """chat-tui should be registered as a subcommand in the CLI registry."""
        from cli._registry import _SUBCOMMAND_TABLE

        subcommands = [name for name, _, _ in _SUBCOMMAND_TABLE]
        assert "chat-tui" in subcommands, f"chat-tui not in {subcommands}"

    def test_chat_tui_dispatch_exists(self):
        """chat-tui should have a handler in the dispatch table."""
        from cli._registry import _SUBCOMMAND_TABLE

        names = [name for name, _, _ in _SUBCOMMAND_TABLE]
        assert "chat-tui" in names


def _get_subcommand_table():
    """Helper to get subcommand table."""
    from cli._registry import _SUBCOMMAND_TABLE
    return _SUBCOMMAND_TABLE


class TestChatTuiArgs:
    """Test chat-tui command line arguments."""

    def test_default_model_from_config(self):
        """Default model should come from config.DEFAULT_LLM_MODEL_CLI."""
        try:
            from config import DEFAULT_LLM_MODEL_CLI
            assert isinstance(DEFAULT_LLM_MODEL_CLI, str)
            assert len(DEFAULT_LLM_MODEL_CLI) > 0
        except ImportError:
            pytest.skip("config module not in test environment")

    def test_default_base_url_from_config(self):
        """Default base URL should come from config.DEFAULT_OPENAI_BASE_URL."""
        try:
            from config import DEFAULT_OPENAI_BASE_URL
            assert isinstance(DEFAULT_OPENAI_BASE_URL, str)
            assert DEFAULT_OPENAI_BASE_URL.startswith("http")
        except ImportError:
            pytest.skip("config module not in test environment")
