"""Async LLM API client using aiohttp."""
import json
import os
from typing import Any, Callable, Dict, List, Optional

import aiohttp

from core.retry import circuit_breaker

ProgressCallback = Optional[Callable[[str], None]]

# Module-level session for connection pooling — created lazily on first use
_connector = aiohttp.TCPConnector(limit=10, keepalive_timeout=30)
_timeout_cfg = aiohttp.ClientTimeout(total=180)
_session: Optional[aiohttp.ClientSession] = None


async def _get_session() -> aiohttp.ClientSession:
    global _session
    if _session is None or _session.closed:
        _session = aiohttp.ClientSession(connector=_connector, timeout=_timeout_cfg)
    return _session


@circuit_breaker(failure_threshold=5, recovery_timeout=60.0)
async def call_llm_chat_completions_async(
    messages: List[Dict[str, str]],
    model: str,
    user_prompt: Optional[str] = None,
    base_url: str = "https://api.openai.com/v1",
    api_key: Optional[str] = None,
    timeout: int = 180,
    system_prompt: Optional[str] = None,
    stream: bool = False,
    progress_callback: ProgressCallback = None,
) -> str:
    api_key = api_key or os.getenv("OPENAI_API_KEY", "")
    if not api_key:
        raise ValueError("Missing API key. Provide --api-key or set OPENAI_API_KEY.")

    url = base_url.rstrip("/") + "/chat/completions"
    headers = {
        "Authorization": f"Bearer {api_key}",
        "Content-Type": "application/json",
    }
    msgs = list(messages)
    if system_prompt:
        msgs = [{"role": "system", "content": system_prompt}] + msgs
    payload: Dict[str, Any] = {
        "model": model,
        "temperature": 0.2,
        "messages": msgs,
        "stream": stream,
    }
    if user_prompt:
        payload["messages"] = msgs + [{"role": "user", "content": user_prompt}]

    session = await _get_session()
    async with session.post(url, json=payload) as r:
        r.raise_for_status()
        if stream:
            if progress_callback:
                return await _stream_to_string_with_callback_async(session, r, progress_callback)
            return await _stream_to_string_async(session, r)
        data = await r.json()
        return data["choices"][0]["message"]["content"]


async def _stream_to_string_async(session: aiohttp.ClientSession, response: aiohttp.ClientResponse) -> str:
    """Yield content deltas from SSE stream asynchronously."""
    parts: List[str] = []
    async for line in response.content:
        decoded = line.decode("utf-8", errors="replace")
        if not decoded.startswith("data: "):
            continue
        payload = decoded[6:].strip()
        if payload == "[DONE]":
            break
        try:
            obj = json.loads(payload)
        except Exception:
            continue
        delta = obj.get("choices", [{}])[0].get("delta", {})
        content = delta.get("content", "")
        if content:
            parts.append(content)
    return "".join(parts)


async def _stream_to_string_with_callback_async(
    session: aiohttp.ClientSession,
    response: aiohttp.ClientResponse,
    callback: Callable[[str], None],
) -> str:
    """Stream with per-chunk progress callback. Returns full assembled content."""
    parts: List[str] = []
    async for line in response.content:
        decoded = line.decode("utf-8", errors="replace")
        if not decoded.startswith("data: "):
            continue
        payload = decoded[6:].strip()
        if payload == "[DONE]":
            break
        try:
            obj = json.loads(payload)
        except Exception:
            continue
        delta = obj.get("choices", [{}])[0].get("delta", {})
        content = delta.get("content", "")
        if content:
            parts.append(content)
            callback(content)
    return "".join(parts)


async def close_session() -> None:
    """Close the module-level aiohttp session. Call on app shutdown."""
    global _session
    if _session is not None and not _session.closed:
        await _session.close()
        _session = None
