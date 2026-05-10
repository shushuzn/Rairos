"""Warp CLI client — lazily initialized tool backend for any LLM model."""

from __future__ import annotations

import logging
from typing import Optional

logger = logging.getLogger(__name__)

_warp_cli_client = None


def is_warp_cli_available() -> bool:
    """Check if Warp CLI is available on the system."""
    try:
        import subprocess

        result = subprocess.run(
            ["warp", "--version"],
            capture_output=True,
            text=True,
            timeout=10,
        )
        return result.returncode == 0
    except Exception:
        return False


class WarpCLIClient:
    """Warp CLI wrapper for local LLM inference.

    Uses Warp's built-in LLM backend when available.
    Covers non-Anthropic models without requiring API keys.
    """

    def __init__(self):
        self._checked = False

    def chat(
        self,
        prompt: str,
        model: str = "gpt-4o-mini",
        system_prompt: Optional[str] = None,
        **kwargs,
    ) -> str:
        """Send a chat request via Warp CLI.

        Args:
            prompt: The user prompt
            model: Model name (any Warp-supported model)
            system_prompt: Optional system prompt
            **kwargs: Passed through to warp

        Returns:
            The model's response as a string.
        """
        import json
        import subprocess

        args = ["warp", "ai", "ask", "--model", model]
        if system_prompt:
            args.extend(["--system-prompt", system_prompt])

        try:
            result = subprocess.run(
                args,
                input=json.dumps({"prompt": prompt}),
                capture_output=True,
                text=True,
                timeout=120,
            )
            if result.returncode != 0:
                logger.warning("Warp CLI returned non-zero: %s", result.stderr)
            return result.stdout
        except subprocess.TimeoutExpired:
            logger.error("Warp CLI timed out after 120s")
            return ""


def get_warp_cli_client():
    """Get Warp CLI client, lazily initialized.

    Returns None if Warp CLI is not available.
    """
    global _warp_cli_client
    if _warp_cli_client is None:
        try:
            if is_warp_cli_available():
                _warp_cli_client = WarpCLIClient()
        except Exception:
            pass
    return _warp_cli_client
