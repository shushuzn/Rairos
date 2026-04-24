"""LLM API client."""
import json
import os
import hashlib
from typing import Dict, Iterator, List, Optional

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
    key_str = json.dumps(key_data, sort_keys=True, ensure_ascii=False)
    return hashlib.md5(key_str.encode()).hexdigest()


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


def clear_llm_cache() -> None:
    """Clear the LLM response cache."""
    _llm_cache.clear()


def get_llm_cache_size() -> int:
    """Get the current size of the LLM response cache."""
    return len(_llm_cache)
