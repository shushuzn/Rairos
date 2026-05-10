"""Claude CLI client — lazily initialized tool backend for claude-* models."""

from __future__ import annotations

import logging
from typing import Optional

logger = logging.getLogger(__name__)

_claude_cli_client = None


def is_claude_cli_available() -> bool:
    """Check if Claude CLI is available on the system."""
    try:
        import subprocess

        result = subprocess.run(
            ["claude", "--version"],
            capture_output=True,
            text=True,
            timeout=10,
        )
        return result.returncode == 0
    except Exception:
        return False


class ClaudeCLIClient:
    """Claude CLI wrapper for local LLM inference.

    Uses the Claude Code CLI's built-in LLM backend when available.
    Requires no API key — zero-config for claude-* models.
    """

    def __init__(self):
        self._checked = False

    def chat(
        self,
        prompt: str,
        model: str = "claude-3-5-sonnet-latest",
        system_prompt: Optional[str] = None,
        **kwargs,
    ) -> str:
        """Send a chat request via Claude CLI.

        Args:
            prompt: The user prompt (messages are concatenated into one prompt)
            model: Model name (claude-*)
            system_prompt: Optional system prompt
            **kwargs: Passed through to claude --print

        Returns:
            The model's response as a string.
        """
        import subprocess

        args = ["claude", "--print", "-m", model]
        if system_prompt:
            args.extend(["--system-prompt", system_prompt])
        # claude --print expects a prompt as stdin or trailing argument
        try:
            result = subprocess.run(
                args,
                input=prompt,
                capture_output=True,
                text=True,
                timeout=120,
            )
            if result.returncode != 0:
                logger.warning("Claude CLI returned non-zero: %s", result.stderr)
            return result.stdout
        except subprocess.TimeoutExpired:
            logger.error("Claude CLI timed out after 120s")
            return ""


def get_claude_cli_client():
    """Get Claude CLI client, lazily initialized.

    Returns None if Claude CLI is not available.
    """
    global _claude_cli_client
    if _claude_cli_client is None:
        try:
            if is_claude_cli_available():
                _claude_cli_client = ClaudeCLIClient()
        except Exception:
            pass
    return _claude_cli_client
