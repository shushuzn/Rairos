"""LLM API client — supports OpenAI-compatible, Anthropic API, and Claude CLI."""
import hashlib
import json
import os
import re
import time
from pathlib import Path
from typing import Dict, Iterator, List, Optional, Tuple

import orjson
import requests

from core.retry import circuit_breaker

# ── Persistent LLM Response Cache ────────────────────────────────────────────
_CACHE_DIR = Path("data/llm_cache")

# Cache hit/miss statistics
_cache_hits = 0
_cache_misses = 0


def _get_cache_ttl() -> int:
    """Get cache TTL from environment variable, default 7 days."""
    env_ttl = os.getenv("AIROS_CACHE_TTL_SECONDS", "")
    if env_ttl:
        try:
            return int(env_ttl)
        except ValueError:
            pass
    return 7 * 24 * 3600  # 7 days default


_CACHE_TTL_SECONDS = _get_cache_ttl()


def _get_cache_path(key: str) -> Path:
    """Get file path for a cache key (use subdirs to avoid too many files in one dir)."""
    _CACHE_DIR.mkdir(parents=True, exist_ok=True)
    return _CACHE_DIR / f"{key[:2]}" / f"{key}.json"


def _cache_read(key: str) -> Tuple[Optional[str], bool]:
    """Read from persistent cache. Returns (value, found).

    Checks TTL before returning. Records hit/miss statistics.
    """
    global _cache_hits, _cache_misses
    path = _get_cache_path(key)
    if not path.exists():
        _cache_misses += 1
        return None, False

    try:
        with open(path, "r", encoding="utf-8") as f:
            entry = json.load(f)

        # Check expiry
        if time.time() - entry.get("cached_at", 0) > _CACHE_TTL_SECONDS:
            path.unlink(missing_ok=True)
            _cache_misses += 1
            return None, False

        _cache_hits += 1
        return entry.get("response"), True
    except (json.JSONDecodeError, OSError):
        _cache_misses += 1
        return None, False


def _cache_write(key: str, response: str) -> None:
    """Write to persistent cache."""
    path = _get_cache_path(key)
    path.parent.mkdir(parents=True, exist_ok=True)

    entry = {
        "response": response,
        "cached_at": time.time(),
    }

    try:
        with open(path, "w", encoding="utf-8") as f:
            json.dump(entry, f)
    except OSError:
        pass  # Cache write failure is non-fatal


def _cache_stats() -> Dict[str, int]:
    """Get cache statistics."""
    hits = 0
    misses = 0
    expired = 0

    if not _CACHE_DIR.exists():
        return {"hits": 0, "misses": 0, "expired": 0, "entries": 0}

    now = time.time()
    for path in _CACHE_DIR.rglob("*.json"):
        try:
            with open(path, "r", encoding="utf-8") as f:
                entry = json.load(f)
            if now - entry.get("cached_at", 0) > _CACHE_TTL_SECONDS:
                expired += 1
            else:
                hits += 1
        except (json.JSONDecodeError, OSError):
            misses += 1

    return {
        "hits": hits,
        "expired": expired,
        "entries": hits + expired,
    }


def get_cache_stats() -> Dict[str, int]:
    """Get cache hit/miss statistics."""
    total = _cache_hits + _cache_misses
    hit_rate = round(_cache_hits / total * 100, 1) if total > 0 else 0.0
    return {
        "hits": _cache_hits,
        "misses": _cache_misses,
        "total": total,
        "hit_rate": hit_rate,
    }


def reset_cache_stats() -> None:
    """Reset cache hit/miss counters."""
    global _cache_hits, _cache_misses
    _cache_hits = 0
    _cache_misses = 0


def clear_llm_cache() -> None:
    """Clear all cached LLM responses."""
    if _CACHE_DIR.exists():
        for path in _CACHE_DIR.rglob("*.json"):
            path.unlink(missing_ok=True)


def get_llm_cache_size() -> int:
    """Get number of cached entries."""
    if not _CACHE_DIR.exists():
        return 0
    return len(list(_CACHE_DIR.rglob("*.json")))


# Detect Claude CLI availability lazily
_claude_cli_client = None


def _get_claude_cli_client():
    """Get Claude CLI client, lazily initialized."""
    global _claude_cli_client
    if _claude_cli_client is None:
        try:
            from llm.claude_cli import ClaudeCLIClient, is_claude_cli_available
            if is_claude_cli_available():
                _claude_cli_client = ClaudeCLIClient()
        except ImportError:
            pass
    return _claude_cli_client


def _use_claude_cli_fallback(model: str) -> bool:
    """Check if we should use Claude CLI as the LLM backend.

    Use Claude CLI when:
    1. Model is Anthropic-native (claude-*)
    2. No ANTHROPIC_API_KEY is set
    3. Claude CLI is available
    """
    if not _is_anthropic_model(model):
        return False
    if os.getenv("ANTHROPIC_API_KEY"):
        return False  # Use direct API if key is available
    return _get_claude_cli_client() is not None


# Anthropic API endpoint
ANTHROPIC_API_URL = "https://api.anthropic.com/v1/messages"
ANTHROPIC_API_VERSION = "2023-06-01"

# Detect if a model is Anthropic-native (claude-*)
_ANTHROPIC_MODELS = {"claude-3-5-sonnet-latest", "claude-3-5-sonnet-20241022", "claude-3-opus-latest", "claude-3-haiku-latest"}


def _is_anthropic_model(model: str) -> bool:
    """Check if model is Anthropic-native (needs Anthropic API format)."""
    if model.startswith("claude"):
        return True
    if "claude" in model.lower():
        return True
    return False

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
    use_cache: bool = True,
) -> str:
    # Auto-detect Anthropic API if model is Anthropic-native
    anthropic_key = os.getenv("ANTHROPIC_API_KEY", "")
    if _is_anthropic_model(model):
        # Try Claude CLI first (zero config if available)
        if _use_claude_cli_fallback(model):
            cli = _get_claude_cli_client()
            if cli:
                # Combine messages into a single prompt
                combined = ""
                for msg in messages:
                    role = msg.get("role", "user")
                    content = msg.get("content", "")
                    combined += f"{role.upper()}: {content}\n"
                if user_prompt:
                    combined += f"USER: {user_prompt}\n"
                return cli.chat(
                    prompt=combined,
                    model=model,
                    system_prompt=system_prompt,
                )
        # Fall back to direct API if key is available
        if anthropic_key:
            return _call_anthropic_api(
                messages=messages,
                model=model,
                api_key=anthropic_key,
                timeout=timeout,
                system_prompt=system_prompt,
                stream=stream,
                use_cache=use_cache,
            )

    api_key = api_key or os.getenv("OPENAI_API_KEY", "")
    if not api_key:
        raise ValueError(
            "Missing API key. Set OPENAI_API_KEY or ANTHROPIC_API_KEY.\n"
            "  For OpenAI-compatible: export OPENAI_API_KEY=sk-...\n"
            "  For Anthropic Claude: export ANTHROPIC_API_KEY=sk-ant-..."
        )

    # Generate cache key for non-streaming requests
    cache_key = None
    if use_cache and not stream:
        cache_key = _generate_cache_key(messages, model, user_prompt, system_prompt)
        # Check persistent cache first
        cached_response, found = _cache_read(cache_key)
        if found and cached_response:
            return cached_response

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
            # Cache the result for future requests (persistent cache)
            if cache_key and use_cache:
                _cache_write(cache_key, result)
        return result
    except requests.RequestException as e:
        raise RuntimeError(f"LLM API request failed: {str(e)}") from e
    except (KeyError, ValueError) as e:
        raise RuntimeError(f"LLM API response parsing failed: {str(e)}") from e


def _stream_to_string(r: requests.Response) -> str:
    return "".join(_parse_sse_stream(r))


def _call_anthropic_api(
    messages: List[Dict[str, str]],
    model: str,
    api_key: str,
    timeout: int = 180,
    system_prompt: Optional[str] = None,
    stream: bool = False,
    use_cache: bool = True,
) -> str:
    """Call Anthropic Messages API directly.

    Anthropic uses a different format from OpenAI:
    - Endpoint: /v1/messages (not /chat/completions)
    - Auth: x-api-key header (not Bearer)
    - System: first message in messages array
    - Requires: max_tokens
    """
    # Generate cache key for non-streaming requests
    cache_key = None
    if use_cache and not stream:
        cache_key = _generate_cache_key(messages, model, None, system_prompt)
        cached_response, found = _cache_read(cache_key)
        if found and cached_response:
            return cached_response

    session = _get_session()
    headers = {
        "x-api-key": api_key,
        "anthropic-version": ANTHROPIC_API_VERSION,
        "Content-Type": "application/json",
    }

    # Build messages array - system message goes first for Anthropic
    anthropic_messages = []
    if system_prompt:
        anthropic_messages.append({"role": "user", "content": system_prompt})

    # Convert messages - handle both 'user' and 'assistant' roles
    for msg in messages:
        role = msg.get("role", "user")
        # Anthropic only supports 'user' and 'assistant'
        if role == "system":
            # Prepend to anthropic_messages if exists, else add as first user message
            if anthropic_messages:
                anthropic_messages.insert(0, {"role": "user", "content": msg.get("content", "")})
            else:
                anthropic_messages.append({"role": "user", "content": msg.get("content", "")})
        elif role in ("user", "assistant"):
            anthropic_messages.append({"role": role, "content": msg.get("content", "")})
        # Skip unsupported roles like 'function'

    # Build payload
    payload = {
        "model": model,
        "messages": anthropic_messages,
        "max_tokens": 4096,  # Required by Anthropic
        "temperature": 0.2,
    }

    if stream:
        payload["stream"] = True

    try:
        r = session.post(
            ANTHROPIC_API_URL,
            headers=headers,
            json=payload,
            timeout=timeout,
            stream=stream,
        )
        r.raise_for_status()

        if stream:
            # Anthropic SSE format: "data: {"type": "content_block_delta", ...}"
            return _stream_anthropic_to_string(r)
        else:
            data = r.json()
            result = data["content"][0]["text"]
            # Cache the result for future requests
            if cache_key and use_cache:
                _cache_write(cache_key, result)
            return result

    except requests.RequestException as e:
        raise RuntimeError(f"Anthropic API request failed: {str(e)}") from e
    except (KeyError, ValueError) as e:
        raise RuntimeError(f"Anthropic API response parsing failed: {str(e)}") from e


def _stream_anthropic_to_string(r: requests.Response) -> str:
    """Parse Anthropic SSE stream into a string."""
    result = []
    for line in r.iter_lines(decode_unicode=True):
        if not line.startswith("data: "):
            continue
        payload = line[6:].strip()
        if payload == "[DONE]":
            break
        try:
            obj = orjson.loads(payload)
            if obj.get("type") == "content_block_delta":
                delta = obj.get("delta", {})
                if delta.get("type") == "text_delta":
                    result.append(delta.get("text", ""))
        except orjson.JSONDecodeError:
            continue
    return "".join(result)


def stream_llm_chat_completions(
    messages: List[Dict[str, str]],
    model: str,
    user_prompt: Optional[str] = None,
    base_url: str = "https://api.openai.com/v1",
    api_key: Optional[str] = None,
    timeout: int = 180,
    system_prompt: Optional[str] = None,
    use_cache: bool = True,
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
    # Auto-detect Anthropic API if model is Anthropic-native
    anthropic_key = os.getenv("ANTHROPIC_API_KEY", "")
    if _is_anthropic_model(model) and anthropic_key:
        # For streaming, we call and iterate
        result = _call_anthropic_api(
            messages=messages,
            model=model,
            api_key=anthropic_key,
            timeout=timeout,
            system_prompt=system_prompt,
            stream=True,
            use_cache=use_cache,
        )
        yield result
        return

    api_key = api_key or os.getenv("OPENAI_API_KEY", "")
    if not api_key:
        raise ValueError(
            "Missing API key. Set OPENAI_API_KEY or ANTHROPIC_API_KEY.\n"
            "  For OpenAI-compatible: export OPENAI_API_KEY=sk-...\n"
            "  For Anthropic Claude: export ANTHROPIC_API_KEY=sk-ant-..."
        )

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
