"""LLM API client."""
import hashlib
import os
from typing import Dict, Iterator, List, Optional

import orjson
import requests

from core.retry import circuit_breaker

# Reusable session for connection pooling (avoids TCP+TLS handshake per request)
_http_session: Optional[requests.Session] = None

# Simple in-memory cache for LLM responses
_llm_cache: Dict[str, str] = {}


def _get_session() -> requests.Session:
    global _http_session
    if _http_session is None:
        _http_session = requests.Session()
        _http_session.headers.update({"Content-Type": "application/json"})
    return _http_session


def _generate_cache_key(
    messages: List[Dict[str, str]],
    model: str,
    user_prompt: Optional[str] = None,
    system_prompt: Optional[str] = None,
) -> str:
    """Generate a cache key based on the request parameters."""
    key_data = {
        "messages": messages,
        "model": model,
        "user_prompt": user_prompt,
        "system_prompt": system_prompt,
    }
    key_str = orjson.dumps(key_data)
    return hashlib.md5(key_str).hexdigest()


def _parse_sse_stream(r: requests.Response) -> Iterator[str]:
    """Yield content deltas from SSE stream."""
    for line in r.iter_lines(decode_unicode=True):
        if not line.startswith("data: "):
            continue
        payload = line[6:].strip()
        if payload == "[DONE]":
            break
        try:
            obj = orjson.loads(payload)
        except orjson.JSONDecodeError:
            # Malformed SSE data line — skip without crashing, continue streaming.
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

    # Generate cache key for non-streaming requests
    cache_key = None
    if not stream:
        cache_key = _generate_cache_key(messages, model, user_prompt, system_prompt)
        if cache_key in _llm_cache:
            return _llm_cache[cache_key]

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
        # MiniMax 思考模型需要禁用思考
        "extra_body": {"thinking": {"type": "disabled"}},
    }
    if user_prompt:
        payload["messages"] = msgs + [{"role": "user", "content": user_prompt}]

    try:
        r = session.post(url, headers=headers, json=payload, timeout=timeout)
        r.raise_for_status()

        if stream:
            result = _stream_to_string(r)
        else:
            data = r.json()
            result = data["choices"][0]["message"]["content"]
            # Cache the result for future requests
            if cache_key:
                _llm_cache[cache_key] = result
        return result
    except requests.RequestException as e:
        raise RuntimeError(f"LLM API request failed: {str(e)}") from e
    except (KeyError, ValueError) as e:
        raise RuntimeError(f"LLM API response parsing failed: {str(e)}") from e


def _stream_to_string(r: requests.Response) -> str:
    return "".join(_parse_sse_stream(r))


def stream_llm_chat_completions(
    messages: List[Dict[str, str]],
    model: str,
    user_prompt: Optional[str] = None,
    base_url: str = "https://api.openai.com/v1",
    api_key: Optional[str] = None,
    timeout: int = 180,
    system_prompt: Optional[str] = None,
) -> Iterator[str]:
    """Stream LLM responses as an iterator of content deltas.

    Yields content deltas as they arrive from the SSE stream.

    Args:
        messages: Chat history
        model: Model name
        user_prompt: Additional user message
        base_url: API base URL
        api_key: API key
        timeout: Request timeout
        system_prompt: System prompt

    Yields:
        Content deltas as strings
    """
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
        "stream": True,
        "extra_body": {"thinking": {"type": "disabled"}},
    }
    if user_prompt:
        payload["messages"] = msgs + [{"role": "user", "content": user_prompt}]

    try:
        r = session.post(url, headers=headers, json=payload, timeout=timeout, stream=True)
        r.raise_for_status()
        yield from _parse_sse_stream(r)
    except requests.RequestException as e:
        raise RuntimeError(f"LLM API request failed: {str(e)}") from e


def clear_llm_cache() -> None:
    """Clear the LLM response cache."""
    _llm_cache.clear()


def get_llm_cache_size() -> int:
    """Get the current size of the LLM response cache."""
    return len(_llm_cache)
