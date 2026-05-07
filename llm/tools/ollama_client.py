"""Ollama client for local LLM inference."""

from __future__ import annotations

import json
import logging
import os
from typing import Any, Dict, Iterator, List, Optional

import requests

logger = logging.getLogger(__name__)

OLLAMA_BASE_URL = os.getenv("AIROS_OLLAMA_BASE_URL", "http://localhost:11434")


def is_ollama_available() -> bool:
    """Check if Ollama is running and accessible."""
    try:
        session = requests.Session()
        session.headers.update({"Content-Type": "application/json"})
        resp = session.get(f"{OLLAMA_BASE_URL}/api/tags", timeout=5)
        return resp.status_code == 200
    except Exception:
        return False


def _get_ollama_session() -> requests.Session:
    """Get a requests session with Ollama base URL configured."""
    session = requests.Session()
    session.headers.update({"Content-Type": "application/json"})
    return session


def call_ollama_chat(
    messages: List[Dict[str, str]],
    model: str,
    timeout: int = 180,
    system_prompt: Optional[str] = None,
    stream: bool = False,
    use_cache: bool = True,
) -> str:
    """Call Ollama's /api/chat endpoint for local LLM inference.

    Strips the 'ollama/' prefix from model name before sending.

    Args:
        messages: Chat history list of {role, content}
        model: Model name (with optional 'ollama/' prefix)
        timeout: Request timeout in seconds
        system_prompt: Optional system prompt
        stream: Whether to stream the response
        use_cache: Whether to use response caching

    Returns:
        The model's response as a string.
    """
    actual_model = model.replace("ollama/", "", 1)

    session = _get_ollama_session()
    url = f"{OLLAMA_BASE_URL}/api/chat"

    ollama_messages: List[Dict[str, str]] = []
    if system_prompt:
        ollama_messages.append({"role": "system", "content": system_prompt})
    for msg in messages:
        role = msg.get("role", "user")
        if role in ("user", "assistant", "system"):
            ollama_messages.append({"role": role, "content": msg.get("content", "")})

    payload: Dict[str, Any] = {
        "model": actual_model,
        "messages": ollama_messages,
        "stream": stream,
        "options": {"temperature": 0.2},
    }

    try:
        r = session.post(url, json=payload, timeout=timeout, stream=stream)
        r.raise_for_status()

        if stream:
            return _stream_ollama_to_string(r)
        else:
            data = r.json()
            result = data.get("message", {}).get("content", "")
            if not result:
                raise RuntimeError(f"No content in Ollama response: {data}")
            return result

    except requests.ConnectionError as e:
        raise RuntimeError(
            f"Ollama connection failed: {e}. Is Ollama running? Start with: ollama serve"
        ) from e
    except requests.RequestException as e:
        raise RuntimeError(f"Ollama API request failed: {e}") from e


def _stream_ollama_to_string(r: requests.Response) -> str:
    """Parse Ollama SSE stream into a string."""
    result: List[str] = []
    for line in r.iter_lines(decode_unicode=True):
        if not line.strip():
            continue
        try:
            data = json.loads(line)
            content = data.get("message", {}).get("content", "")
            if content:
                result.append(content)
            if data.get("done", False):
                break
        except (json.JSONDecodeError, KeyError):
            continue
    return "".join(result)


def stream_ollama_chat(
    messages: List[Dict[str, str]],
    model: str,
    timeout: int = 180,
    system_prompt: Optional[str] = None,
) -> Iterator[str]:
    """Stream LLM responses from Ollama as an iterator of content deltas.

    Args:
        messages: Chat history
        model: Model name
        timeout: Request timeout
        system_prompt: Optional system prompt

    Yields:
        Content deltas as strings.
    """
    actual_model = model.replace("ollama/", "", 1)
    session = _get_ollama_session()
    url = f"{OLLAMA_BASE_URL}/api/chat"

    ollama_messages: List[Dict[str, str]] = []
    if system_prompt:
        ollama_messages.append({"role": "system", "content": system_prompt})
    for msg in messages:
        role = msg.get("role", "user")
        if role in ("user", "assistant", "system"):
            ollama_messages.append({"role": role, "content": msg.get("content", "")})

    payload: Dict[str, Any] = {
        "model": actual_model,
        "messages": ollama_messages,
        "stream": True,
        "options": {"temperature": 0.2},
    }

    try:
        r = session.post(url, json=payload, timeout=timeout, stream=True)
        r.raise_for_status()
        for line in r.iter_lines(decode_unicode=True):
            if not line.strip():
                continue
            try:
                data = json.loads(line)
                content = data.get("message", {}).get("content", "")
                if content:
                    yield content
                if data.get("done", False):
                    break
            except (json.JSONDecodeError, KeyError):
                continue
    except requests.RequestException as e:
        raise RuntimeError(f"Ollama streaming request failed: {e}") from e
