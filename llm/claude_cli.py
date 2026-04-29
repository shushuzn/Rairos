"""LLM provider using Claude Code CLI — zero API key required!

Uses the installed Claude Code CLI to make LLM calls, leveraging its
built-in Anthropic authentication. No API key configuration needed.

Usage:
    from llm.claude_cli import ClaudeCLIClient
    client = ClaudeCLIClient()
    response = client.chat("What is RAG?", model="claude-3-5-sonnet-latest")
"""
import json
import os
import subprocess
import sys
from typing import Optional


def _get_claude_executable() -> str:
    """Get the correct Claude CLI executable for the platform."""
    # Try 'claude' first (works on Unix/macOS)
    if sys.platform != "win32":
        return "claude"

    # On Windows, try different variants
    for name in ["claude.cmd", "claude.exe", "claude"]:
        try:
            result = subprocess.run(
                [name, "--version"],
                capture_output=True,
                text=True,
                timeout=10,
            )
            if result.returncode == 0:
                return name
        except Exception:
            continue

    # Fallback to claude.cmd on Windows
    return "claude.cmd"


class ClaudeCLIClient:
    """LLM client using Claude Code CLI as the backend.

    Advantages:
    - No API key needed — uses Claude Code's built-in authentication
    - Supports all Claude models
    - Works out of the box if Claude Code is installed

    Limitations:
    - Requires Claude Code CLI to be installed
    - Slower than direct API calls (subprocess overhead)
    """

    def __init__(self, cli_path: Optional[str] = None):
        """Initialize the Claude CLI client.

        Args:
            cli_path: Path to Claude CLI executable (auto-detected by default)
        """
        self.cli_path = cli_path or _get_claude_executable()

    def chat(
        self,
        prompt: str,
        model: str = "claude-3-5-sonnet-latest",
        system_prompt: Optional[str] = None,
        temperature: float = 0.2,
        max_tokens: int = 4096,
    ) -> str:
        """Send a chat message and get a response.

        Args:
            prompt: User message
            model: Model to use (claude-3-5-sonnet-latest, etc.)
            system_prompt: Optional system message
            temperature: Sampling temperature
            max_tokens: Maximum tokens in response

        Returns:
            Model's text response
        """
        full_prompt = prompt
        if system_prompt:
            full_prompt = f"[System: {system_prompt}]\n\n{prompt}"

        try:
            result = subprocess.run(
                [
                    self.cli_path,
                    "--print",
                    "--model", model,
                    "--output-format", "json",
                ],
                input=full_prompt,
                capture_output=True,
                text=True,
                timeout=120,
                encoding='utf-8',
            )

            if result.returncode != 0:
                stderr = result.stderr.strip()
                raise RuntimeError(f"Claude CLI error: {stderr}")

            # Parse JSON response to extract result
            output = result.stdout.strip()
            try:
                data = json.loads(output)
                # JSON format: {"result": "...", "subtype": "success", ...}
                return data.get("result", output)
            except json.JSONDecodeError:
                # Fallback: return raw output if not JSON
                return output

        except subprocess.TimeoutExpired:
            raise RuntimeError("Claude CLI timed out after 120 seconds")
        except FileNotFoundError:
            raise RuntimeError(
                f"Claude CLI not found at '{self.cli_path}'. "
                "Please install Claude Code: npm i -g @anthropic-ai/claude-code"
            )
        except Exception as e:
            raise RuntimeError(f"Claude CLI call failed: {e}")

    def is_available(self) -> bool:
        """Check if Claude CLI is available and working.

        Returns:
            True if Claude CLI can be invoked, False otherwise
        """
        try:
            result = subprocess.run(
                [self.cli_path, "--version"],
                capture_output=True,
                text=True,
                timeout=10,
            )
            return result.returncode == 0
        except Exception:
            return False

    def get_version(self) -> Optional[str]:
        """Get Claude CLI version if available.

        Returns:
            Version string or None if not available
        """
        try:
            result = subprocess.run(
                [self.cli_path, "--version"],
                capture_output=True,
                text=True,
                timeout=10,
                encoding='utf-8',
            )
            if result.returncode == 0:
                return result.stdout.strip()
            return None
        except Exception:
            return None


# Singleton instance for convenience
_default_client: Optional[ClaudeCLIClient] = None


def get_claude_cli_client() -> ClaudeCLIClient:
    """Get or create the default Claude CLI client instance."""
    global _default_client
    if _default_client is None:
        _default_client = ClaudeCLIClient()
    return _default_client


def is_claude_cli_available() -> bool:
    """Quick check if Claude CLI is available."""
    return get_claude_cli_client().is_available()
