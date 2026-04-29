"""LLM provider using Warp CLI — zero API key required!

Uses the installed Warp terminal to make LLM calls via its built-in
agentic AI features. No API key configuration needed.

Warp supports various AI models and provides a clean terminal interface
for AI-assisted development.

Usage:
    from llm.warp_cli import WarpCLIClient
    client = WarpCLIClient()
    response = client.chat("What is RAG?", model="claude-3-5-sonnet-latest")
"""
import json
import os
import subprocess
import sys
from typing import Optional


def _get_warp_executable() -> str:
    """Get the correct Warp executable for the platform."""
    if sys.platform != "win32":
        for name in ["warp", "Warp"]:
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
        return "warp"

    # On Windows, try different variants
    for name in ["warp.cmd", "warp.exe", "warp"]:
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

    return "warp"


class WarpCLIClient:
    """LLM client using Warp CLI as the backend.

    Advantages:
    - No API key needed — uses Warp's built-in authentication
    - Supports multiple AI models
    - Works out of the box if Warp is installed

    Limitations:
    - Requires Warp CLI to be installed
    - Slower than direct API calls (subprocess overhead)
    - Windows support may be limited
    """

    def __init__(self, cli_path: Optional[str] = None):
        """Initialize the Warp CLI client.

        Args:
            cli_path: Path to Warp executable (auto-detected by default)
        """
        self.cli_path = cli_path or _get_warp_executable()

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
            model: Model to use (claude-3-5-sonnet-latest, gpt-4o, etc.)
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
            # Warp CLI usage (based on warp.dev agent platform docs):
            # warp ai --model <model> [--prompt <text>] [--output json]
            result = subprocess.run(
                [
                    self.cli_path,
                    "ai",
                    "--model", model,
                    "--prompt", full_prompt,
                    "--output-format", "json",
                ],
                capture_output=True,
                text=True,
                timeout=120,
                encoding='utf-8',
            )

            if result.returncode != 0:
                stderr = result.stderr.strip()
                raise RuntimeError(f"Warp CLI error: {stderr}")

            # Parse JSON response to extract result
            output = result.stdout.strip()
            try:
                data = json.loads(output)
                # Try common JSON response formats
                if isinstance(data, dict):
                    # Warp might return {"response": "...", ...} or {"result": "...", ...}
                    return data.get("response") or data.get("result") or data.get("text") or str(data)
                return str(data)
            except json.JSONDecodeError:
                # Fallback: return raw output if not JSON
                return output

        except subprocess.TimeoutExpired:
            raise RuntimeError("Warp CLI timed out after 120 seconds")
        except FileNotFoundError:
            raise RuntimeError(
                f"Warp not found at '{self.cli_path}'. "
                "Please install Warp: https://warp.dev"
            )
        except Exception as e:
            raise RuntimeError(f"Warp CLI call failed: {e}")

    def is_available(self) -> bool:
        """Check if Warp CLI is available and working.

        Returns:
            True if Warp can be invoked, False otherwise
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
        """Get Warp version if available.

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
_default_client: Optional[WarpCLIClient] = None


def get_warp_cli_client() -> WarpCLIClient:
    """Get or create the default Warp CLI client instance."""
    global _default_client
    if _default_client is None:
        _default_client = WarpCLIClient()
    return _default_client


def is_warp_cli_available() -> bool:
    """Quick check if Warp CLI is available."""
    return get_warp_cli_client().is_available()
