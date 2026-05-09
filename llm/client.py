"""LLM API client — supports OpenAI-compatible, Anthropic API, and Claude CLI."""

from __future__ import annotations

import hashlib
import json
import logging
import os
import time
from pathlib import Path
from typing import Dict, Iterator, List, Optional, Tuple, cast, Any, Callable

import orjson
import requests

from core.retry import circuit_breaker
from core.rate_limiter import rate_limited
from llm.tools import claude_cli, ollama_client, warp_cli

logger = logging.getLogger(__name__)

# ── Persistent LLM Response Cache ────────────────────────────────────────────

_CACHE_DIR = Path("data/llm_cache")
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
    entry = {"response": response, "cached_at": time.time()}

    try:
        with open(path, "w", encoding="utf-8") as f:
            json.dump(entry, f)
    except OSError:
        pass  # Cache write failure is non-fatal


def _cache_stats() -> Dict[str, int]:
    """Get cache statistics."""
    hits = misses = expired = 0
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

    return {"hits": hits, "expired": expired, "entries": hits + expired}


def get_cache_stats() -> Dict[str, float]:
    """Get cache hit/miss statistics."""
    global _cache_hits, _cache_misses
    total = _cache_hits + _cache_misses
    hit_rate = round(_cache_hits / total * 100, 1) if total > 0 else 0.0
    return {"hits": _cache_hits, "misses": _cache_misses, "total": total, "hit_rate": hit_rate}


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
    """Get number of cached LLM entries."""
    if not _CACHE_DIR.exists():
        return 0
    return len(list(_CACHE_DIR.rglob("*.json")))


def _read_hermes_env() -> dict:
    """Read MiniMax credentials from hermes config."""
    result = {}
    hermes_home = Path.home() / ".hermes"
    env_file = hermes_home / ".env"
    if env_file.exists():
        text = env_file.read_text(encoding="utf-8").replace("\r", "")
        for line in text.splitlines():
            line = line.strip()
            if "=" in line and not line.startswith("#"):
                k, v = line.split("=", 1)
                if (
                    k
                    in (
                        "MINIMAX_CN_API_KEY",
                        "MINIMAX_CN_BASE_URL",
                        "MINIMAX_API_KEY",
                        "MINIMAX_BASE_URL",
                    )
                    and k not in result
                ):
                    result[k] = v.strip()
    return result


def _resolve_llm_credentials(base_url: str, api_key: str) -> tuple[str, str]:
    """Resolve (base_url, api_key) from all known sources, in priority order.

    Priority: explicit args > MINIMAX_CN_* > MINIMAX_* > OPENAI_* > hermes config
    """
    hermes = _read_hermes_env()

    resolved_key = api_key
    if not resolved_key:
        resolved_key = os.getenv("OPENAI_API_KEY") or ""
    if not resolved_key:
        resolved_key = os.getenv("MINIMAX_CN_API_KEY") or hermes.get("MINIMAX_CN_API_KEY", "") or ""
    if not resolved_key:
        resolved_key = os.getenv("MINIMAX_API_KEY") or ""

    resolved_url = base_url if base_url else ""
    if not resolved_url or resolved_url == "https://api.openai.com/v1":
        resolved_url = (
            os.getenv("MINIMAX_CN_BASE_URL") or hermes.get("MINIMAX_CN_BASE_URL", "") or ""
        )
    if not resolved_url:
        resolved_url = (
            os.getenv("MINIMAX_BASE_URL")
            or os.getenv("OPENAI_BASE_URL", "https://api.minimaxi.com/v1")
            or ""
            or ""
        )

    if "/anthropic" in resolved_url:
        resolved_url = resolved_url.replace("/anthropic", "/v1")

    return resolved_url, resolved_key


def warm_cache(
    queries: List[str],
    model: str = "gpt-4o-mini",
    base_url: Optional[str] = None,
    api_key: Optional[str] = None,
    system_prompt: Optional[str] = None,
) -> Dict[str, bool]:
    """Pre-warm the cache with LLM responses for common queries.

    Args:
        queries: List of query strings to pre-cache
        model: Model to use for generating responses
        base_url: API base URL
        api_key: API key (defaults to env OPENAI_API_KEY)
        system_prompt: Optional system prompt

    Returns:
        Dict mapping query to success status
    """
    results = {}
    for query in queries:
        try:
            call_llm_chat_completions(
                messages=[{"role": "user", "content": query}],
                model=model,
                base_url=base_url,
                api_key=api_key,
                system_prompt=system_prompt,
                use_cache=True,
            )
            results[query] = True
        except Exception as e:
            logger.warning("Failed to pre-warm cache for query %r: %s", query, e)
            results[query] = False
    return results


# ── Model detection helpers ────────────────────────────────────────────────────

_ANTHROPIC_MODELS = {
    "claude-3-5-sonnet-latest",
    "claude-3-5-sonnet-20241022",
    "claude-3-opus-latest",
    "claude-3-haiku-latest",
    "claude-4-*",
    "claude-sonnet-4-*",
    "claude-opus-4-*",
}


def _is_ollama_model(model: str) -> bool:
    """Check if model is an Ollama local model (prefixed with ollama/)."""
    if model.startswith("ollama/"):
        return True
    if not os.getenv("OPENAI_API_KEY") and not os.getenv("ANTHROPIC_API_KEY"):
        if model.startswith("llama") or model.startswith("qwen") or model.startswith("mistral"):
            return True
    return False


def _is_anthropic_model(model: str) -> bool:
    """Check if model is Anthropic-native (needs Anthropic API format)."""
    return model.startswith("claude") or "claude" in model.lower()


def _use_claude_cli_fallback(model: str) -> bool:
    """Check if we should use Claude CLI as the LLM backend.

    Use Claude CLI when: (1) Model is Anthropic-native (claude-*),
    (2) No ANTHROPIC_API_KEY is set, (3) Claude CLI is available.
    """
    if not _is_anthropic_model(model):
        return False
    if os.getenv("ANTHROPIC_API_KEY"):
        return False
    return claude_cli.get_claude_cli_client() is not None


def _use_warp_cli_fallback(model: str) -> bool:
    """Check if we should use Warp CLI as the LLM backend.

    Use Warp CLI when: (1) No OPENAI_API_KEY is set,
    (2) No ANTHROPIC_API_KEY is set, (3) Warp CLI is available.
    """
    if os.getenv("OPENAI_API_KEY"):
        return False
    if os.getenv("ANTHROPIC_API_KEY"):
        return False
    return warp_cli.get_warp_cli_client() is not None


# ── API endpoint constants ────────────────────────────────────────────────────

ANTHROPIC_API_URL = os.getenv("ANTHROPIC_API_URL", "https://api.anthropic.com/v1/messages")
MINIMAX_BASE_URL = os.getenv("MINIMAX_BASE_URL", "https://api.minimax.chat/v1")
ANTHROPIC_API_VERSION = "2023-06-01"

# Re-export for backwards compatibility
OLLAMA_BASE_URL = ollama_client.OLLAMA_BASE_URL

# ── HTTP session (connection pooling) ─────────────────────────────────────────

_http_session: Optional[requests.Session] = None


def _get_session() -> requests.Session:
    """Get a reusable session for connection pooling."""
    global _http_session
    if _http_session is None:
        _http_session = requests.Session()
        _http_session.headers.update({"Content-Type": "application/json"})
    return _http_session


# ── Cache key generation ──────────────────────────────────────────────────────

_llm_cache: Dict[str, str] = {}


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
    return hashlib.sha256(orjson.dumps(key_data)).hexdigest()


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
            continue
        delta = obj.get("choices", [{}])[0].get("delta", {})
        content = delta.get("content", "")
        if content:
            yield content


# ── Main LLM API ──────────────────────────────────────────────────────────────


@circuit_breaker(failure_threshold=5, recovery_timeout=60.0)
def call_llm_chat_completions(
    messages: List[Dict[str, str]],
    model: str,
    user_prompt: Optional[str] = None,
    base_url: Optional[str] = None,
    api_key: Optional[str] = None,
    timeout: int = 180,
    system_prompt: Optional[str] = None,
    stream: bool = False,
    use_cache: bool = True,
) -> str:
    """Call LLM chat completions API (OpenAI-compatible, Anthropic, Ollama, Claude CLI, Warp CLI).

    Auto-detects the best backend based on model name and available credentials.
    """
    # Auto-detect Ollama local model
    if _is_ollama_model(model):
        return ollama_client.call_ollama_chat(
            messages=messages,
            model=model,
            timeout=timeout,
            system_prompt=system_prompt,
            stream=stream,
            use_cache=use_cache,
        )

    anthropic_key = os.getenv("ANTHROPIC_API_KEY", "")

    # Auto-detect Anthropic API
    if _is_anthropic_model(model):
        if _use_claude_cli_fallback(model):
            cli = claude_cli.get_claude_cli_client()
            if cli:
                combined = ""
                for msg in messages:
                    role = msg.get("role", "user")
                    content = msg.get("content", "")
                    combined += f"{role.upper()}: {content}\n"
                if user_prompt:
                    combined += f"USER: {user_prompt}\n"
                return cast(
                    str, cli.chat(prompt=combined, model=model, system_prompt=system_prompt)
                )

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

    # Try Warp CLI as a zero-config fallback
    if _use_warp_cli_fallback(model):
        cli = warp_cli.get_warp_cli_client()
        if cli:
            combined = ""
            for msg in messages:
                role = msg.get("role", "user")
                content = msg.get("content", "")
                combined += f"{role.upper()}: {content}\n"
            if user_prompt:
                combined += f"USER: {user_prompt}\n"
            return cast(str, cli.chat(prompt=combined, model=model, system_prompt=system_prompt))

    resolved_url, resolved_key = _resolve_llm_credentials(base_url or "", api_key or "")
    api_key = resolved_key

    if not api_key:
        raise ValueError(
            "Missing API key. Set OPENAI_API_KEY, MINIMAX_CN_API_KEY, or MINIMAX_API_KEY.\n"
            "  For OpenAI-compatible: export OPENAI_API_KEY=***\n"
            "  For MiniMax China: export MINIMAX_CN_API_KEY=***\n"
            "  For MiniMax global: export MINIMAX_API_KEY=***\n"
            "  For Ollama (local, free): run ollama serve and use model prefix ollama/\n"
            "    e.g. use --model ollama/llama3.2 or set DEFAULT_LLM_MODEL=ollama/qwen2.5\n"
            "  Or install Claude Code CLI (npm i -g @anthropic-ai/claude-code)\n"
            "  Or install Warp (https://warp.dev)"
        )

    cache_key = None
    if use_cache and not stream:
        cache_key = _generate_cache_key(messages, model, user_prompt, system_prompt)
        cached_response, found = _cache_read(cache_key)
        if found and cached_response:
            return cached_response

    url = resolved_url.rstrip("/") + "/chat/completions"
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
        "extra_body": {"thinking": {"type": "disabled"}},
    }
    if user_prompt:
        payload["messages"] = msgs + [{"role": "user", "content": user_prompt}]

    try:
        with rate_limited("llm", config=None):
            r = session.post(url, headers=headers, json=payload, timeout=timeout)
        r.raise_for_status()

        if stream:
            result = _stream_to_string(r)
        else:
            data = r.json()
            result = data["choices"][0]["message"]["content"]
            if cache_key and use_cache:
                _cache_write(cache_key, result)

        return result

    except requests.RequestException as e:
        raise RuntimeError(f"LLM API request failed: {str(e)}") from e
    except (KeyError, ValueError) as e:
        raise RuntimeError(f"LLM API response parsing failed: {str(e)}") from e


def _stream_to_string(r: requests.Response) -> str:
    return "".join(_parse_sse_stream(r))


# ── Anthropic API ─────────────────────────────────────────────────────────────


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

    anthropic_messages: List[Dict[str, str]] = []
    if system_prompt:
        anthropic_messages.append({"role": "user", "content": system_prompt})

    for msg in messages:
        role = msg.get("role", "user")
        if role == "system":
            if anthropic_messages:
                anthropic_messages.insert(0, {"role": "user", "content": msg.get("content", "")})
            else:
                anthropic_messages.append({"role": "user", "content": msg.get("content", "")})
        elif role in ("user", "assistant"):
            anthropic_messages.append({"role": role, "content": msg.get("content", "")})

    payload = {
        "model": model,
        "messages": anthropic_messages,
        "max_tokens": 4096,
        "temperature": 0.2,
    }
    if stream:
        payload["stream"] = True

    try:
        r = session.post(
            ANTHROPIC_API_URL, headers=headers, json=payload, timeout=timeout, stream=stream
        )
        r.raise_for_status()

        if stream:
            return _stream_anthropic_to_string(r)

        data = r.json()
        result = ""
        for block in data.get("content", []):
            if block.get("type") == "text":
                result = block.get("text", "")
                break
        if not result:
            raise RuntimeError(f"No text content in Anthropic response: {data}")

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


# ── Streaming LLM API ─────────────────────────────────────────────────────────


def stream_llm_chat_completions(
    messages: List[Dict[str, str]],
    model: str,
    user_prompt: Optional[str] = None,
    base_url: Optional[str] = None,
    api_key: Optional[str] = None,
    timeout: int = 180,
    system_prompt: Optional[str] = None,
    use_cache: bool = True,
) -> Iterator[str]:
    """Stream LLM responses as an iterator of content deltas.

    Yields content deltas as they arrive from the SSE stream.
    """
    cache_key = None
    if use_cache:
        cache_key = _generate_cache_key(messages, model, user_prompt, system_prompt)
        cached_response, found = _cache_read(cache_key)
        if found and cached_response:
            yield cached_response
            return

    # Auto-detect Ollama local model
    if _is_ollama_model(model):
        result = ollama_client.call_ollama_chat(
            messages=messages,
            model=model,
            timeout=timeout,
            system_prompt=system_prompt,
            stream=True,
            use_cache=False,
        )
        if cache_key:
            _cache_write(cache_key, result)
        yield result
        return

    anthropic_key = os.getenv("ANTHROPIC_API_KEY", "")

    if _is_anthropic_model(model) and anthropic_key:
        result = _call_anthropic_api(
            messages=messages,
            model=model,
            api_key=anthropic_key,
            timeout=timeout,
            system_prompt=system_prompt,
            stream=True,
            use_cache=False,
        )
        if cache_key:
            _cache_write(cache_key, result)
        yield result
        return

    api_key = api_key or os.getenv("OPENAI_API_KEY", "")
    if not api_key:
        raise ValueError(
            "Missing API key. Set OPENAI_API_KEY or ANTHROPIC_API_KEY.\n"
            "  For OpenAI-compatible: export OPENAI_API_KEY=sk-...\n"
            "  For Anthropic Claude: export ANTHROPIC_API_KEY=sk-ant-..."
        )

    resolved_url, _ = _resolve_llm_credentials(base_url or "", api_key or "")
    url = resolved_url.rstrip("/") + "/chat/completions"

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

        full_response = ""
        for delta in _parse_sse_stream(r):
            yield delta
            full_response += delta

        if cache_key and full_response:
            _cache_write(cache_key, full_response)

    except requests.RequestException as e:
        raise RuntimeError(f"LLM API request failed: {str(e)}") from e


# ── Client wrapper for evolution.py ───────────────────────────────────────────


def get_client(
    model: str = "minimax-m2.7-highspeed",
    base_url: str = "",
    api_key: str = "",
) -> Any:
    """Get a callable LLM client wrapper that provides a .generate(prompt) interface.

    Reads credentials from (in priority order):
    1. Explicit api_key / base_url arguments
    2. MINIMAX_API_KEY / MINIMAX_BASE_URL env vars
    3. MINIMAX_CN_API_KEY env var (Hermes native MiniMax key)
    4. ~/.hermes/config.yaml + ~/.hermes/.env (auto-detected)
    """
    

    resolved_url, resolved_key = _resolve_llm_credentials(base_url, api_key)
    effective_key = resolved_key or api_key

    client_wrapper = type("LLMClient", (), {})()
    client_wrapper.model = model
    client_wrapper.base_url = resolved_url
    client_wrapper.api_key = effective_key

    def generate(
        prompt: str,
        system: str = "",
        **kwargs,
    ) -> str:
        messages = []
        if system:
            messages.append({"role": "system", "content": system})
        messages.append({"role": "user", "content": prompt})
        return cast(
            str,
            call_llm_chat_completions(
                messages=messages,
                model=model,
                base_url=resolved_url,
                api_key=effective_key,
                **kwargs,
            ),
        )

    client_wrapper.generate = generate

    return client_wrapper
