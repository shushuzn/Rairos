"""LLM API client."""
import json
import os
from typing import Dict, Iterator, List, Optional

import requests

from core.retry import circuit_breaker

# Reusable session for connection pooling (avoids TCP+TLS handshake per request)
_http_session: requests.Session | None = None


def _get_session() -> requests.Session:
    global _http_session
    if _http_session is None:
        _http_session = requests.Session()
        _http_session.headers.update({"Content-Type": "application/json"})
    return _http_session


def _parse_sse_stream(r: requests.Response) -> Iterator[str]:
    """Yield content deltas from SSE stream."""
    for line in r.iter_lines(decode_unicode=True):
        if not line.startswith("data: "):
            continue
        payload = line[6:].strip()
        if payload == "[DONE]":
            break
        try:
            obj = json.loads(payload)
        except Exception:
            continue
        delta = obj.get("choices", [{}])[0].get("delta", {})
        content = delta.get("content", "")
        if content:
            yield content


@circuit_breaker(failure_threshold=5, recovery_timeout=60.0)
def call_llm_chat_completions(
    messages: List[Dict[str, str]],
    model: str,
    user_prompt: Optional[str] = None,
    base_url: str = "https://api.openai.com/v1",
    api_key: Optional[str] = None,
    timeout: int = 180,
    system_prompt: Optional[str] = None,
    stream: bool = False,
) -> str:
    api_key = api_key or os.getenv("OPENAI_API_KEY", "")
    if not api_key:
        raise ValueError("Missing API key. Provide --api-key or set OPENAI_API_KEY.")

    url = base_url.rstrip("/") + "/chat/completions"
    session = _get_session()
    headers = {"Authorization": f"Bearer {api_key}"}
    msgs = list(messages)
    if system_prompt:
        msgs = [{"role": "system", "content": system_prompt}] + msgs
    payload = {
        "model": model,
        "temperature": 0.2,
        "messages": msgs,
        "stream": stream,
    }
    if user_prompt:
        payload["messages"] = msgs + [{"role": "user", "content": user_prompt}]

    r = session.post(url, headers=headers, json=payload, timeout=timeout)
    r.raise_for_status()

    if stream:
        return _stream_to_string(r)
    data = r.json()
    return data["choices"][0]["message"]["content"]


def _stream_to_string(r: requests.Response) -> str:
    parts: List[str] = []
    for chunk in _parse_sse_stream(r):
        parts.append(chunk)
    return "".join(parts)
