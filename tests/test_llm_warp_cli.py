"""Unit tests for llm/warp_cli.py — Warp CLI LLM provider."""

from unittest.mock import patch, MagicMock


class FakeArgs:
    def __init__(self, **kwargs):
        for k, v in kwargs.items():
            setattr(self, k, v)


# ─────────────────────────────────────────────────────────────────────────────
# _get_warp_executable tests
# ─────────────────────────────────────────────────────────────────────────────


class TestGetWarpExecutable:
    def test_returns_warp_on_unix(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        monkeypatch.setattr("sys.platform", "darwin")
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(returncode=0, stdout="Warp 1.0")
            from llm.warp_cli import _get_warp_executable

            result = _get_warp_executable()
            assert result == "warp"

    def test_returns_warp_cmd_on_windows(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        monkeypatch.setattr("sys.platform", "win32")
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(returncode=0, stdout="Warp 1.0")
            from llm.warp_cli import _get_warp_executable

            result = _get_warp_executable()
            assert result == "warp.cmd"


# ─────────────────────────────────────────────────────────────────────────────
# WarpCLIClient tests
# ─────────────────────────────────────────────────────────────────────────────


class TestWarpCLIClientAvailable:
    def test_is_available_returns_true(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from llm.warp_cli import WarpCLIClient

        client = WarpCLIClient(cli_path="warp")
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(returncode=0, stdout="Warp 1.0")
            assert client.is_available() is True

    def test_is_available_returns_false_on_error(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from llm.warp_cli import WarpCLIClient

        client = WarpCLIClient(cli_path="nonexistent_warp")
        with patch("subprocess.run") as mock_run:
            mock_run.side_effect = FileNotFoundError
            assert client.is_available() is False

    def test_get_version_returns_string(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from llm.warp_cli import WarpCLIClient

        client = WarpCLIClient(cli_path="warp")
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(returncode=0, stdout="Warp 2.0.0\n")
            assert client.get_version() == "Warp 2.0.0"

    def test_get_version_returns_none_on_error(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from llm.warp_cli import WarpCLIClient

        client = WarpCLIClient(cli_path="warp")
        with patch("subprocess.run") as mock_run:
            mock_run.side_effect = Exception("fail")
            assert client.get_version() is None


class TestWarpCLIClientChat:
    def test_chat_parses_json_response(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from llm.warp_cli import WarpCLIClient

        client = WarpCLIClient(cli_path="warp")
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(
                returncode=0,
                stdout='{"result": "Hello from Warp!", "subtype": "success"}',
                stderr="",
            )
            result = client.chat("Hello", model="claude-3-5-sonnet-latest")
            assert result == "Hello from Warp!"

    def test_chat_with_system_prompt(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from llm.warp_cli import WarpCLIClient

        client = WarpCLIClient(cli_path="warp")
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(
                returncode=0,
                stdout='{"result": "response text"}',
                stderr="",
            )
            client.chat("Hello", system_prompt="You are helpful.")
            mock_run.assert_called_once()
            # prompt is passed via --prompt flag in the cmd list
            call_args = mock_run.call_args
            cmd_list = call_args.args[0]
            prompt_flag_idx = cmd_list.index("--prompt")
            prompt_value = cmd_list[prompt_flag_idx + 1]
            assert "[System: You are helpful.]" in prompt_value

    def test_chat_returns_non_json_fallback(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from llm.warp_cli import WarpCLIClient

        client = WarpCLIClient(cli_path="warp")
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(
                returncode=0,
                stdout="plain text response",
                stderr="",
            )
            result = client.chat("Hello")
            assert result == "plain text response"

    def test_chat_raises_on_nonzero_returncode(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from llm.warp_cli import WarpCLIClient

        client = WarpCLIClient(cli_path="warp")
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(
                returncode=1,
                stdout="",
                stderr="warp: command not found",
            )
            try:
                client.chat("Hello")
                raise AssertionError("Should have raised RuntimeError")
            except RuntimeError as e:
                assert "warp: command not found" in str(e)

    def test_chat_raises_on_file_not_found(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from llm.warp_cli import WarpCLIClient

        client = WarpCLIClient(cli_path="nonexistent_warp")
        with patch("subprocess.run") as mock_run:
            mock_run.side_effect = FileNotFoundError
            try:
                client.chat("Hello")
                raise AssertionError("Should have raised RuntimeError")
            except RuntimeError as e:
                assert "not found" in str(e)

    def test_chat_extracts_response_field(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from llm.warp_cli import WarpCLIClient

        client = WarpCLIClient(cli_path="warp")
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(
                returncode=0,
                stdout='{"response": "Answer from warp response field", "model": "claude"}',
                stderr="",
            )
            result = client.chat("Hello")
            assert result == "Answer from warp response field"

    def test_chat_extracts_text_field(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        from llm.warp_cli import WarpCLIClient

        client = WarpCLIClient(cli_path="warp")
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(
                returncode=0,
                stdout='{"text": "Answer from warp text field"}',
                stderr="",
            )
            result = client.chat("Hello")
            assert result == "Answer from warp text field"


# ─────────────────────────────────────────────────────────────────────────────
# is_warp_cli_available singleton test
# ─────────────────────────────────────────────────────────────────────────────


class TestIsWarpCliAvailable:
    def test_is_warp_cli_available_calls_get_client(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")
        with patch("llm.warp_cli.get_warp_cli_client") as mock_get:
            mock_client = MagicMock()
            mock_client.is_available.return_value = True
            mock_get.return_value = mock_client
            from llm.warp_cli import is_warp_cli_available

            result = is_warp_cli_available()
            assert result is True
            mock_get.assert_called_once()
