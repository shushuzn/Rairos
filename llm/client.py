"""LLM API client — supports OpenAI-compatible, Anthropic API, and Claude CLI."""

import hashlib


import json


import logging


import os


import time


from pathlib import Path


from typing import Dict, Iterator, List, Optional, Tuple, cast


import orjson


import requests


from core.retry import circuit_breaker


from core.rate_limiter import rate_limited


logger = logging.getLogger(__name__)


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


def get_cache_stats() -> Dict[str, float]:
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
    """Get number of cached LLM entries."""

    if not _CACHE_DIR.exists():
        return 0

    return len(list(_CACHE_DIR.rglob("*.json")))


def _read_hermes_env() -> dict:
    """Read MiniMax credentials from hermes config."""

    from pathlib import Path

    result = {}

    hermes_home = Path.home() / ".hermes"
    env_file = hermes_home / ".env"
    if env_file.exists():
        # Read and normalize: remove all \r to handle mixed line endings
        text = env_file.read_text(encoding="utf-8").replace("\r", "")
        for line in text.splitlines():
            line = line.strip()
            if "=" in line and not line.startswith("#"):
                k, v = line.split("=", 1)
                # Only set if not already present (take first occurrence)
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

    # Resolve api_key

    resolved_key = api_key

    if not resolved_key:
        resolved_key = os.getenv("OPENAI_API_KEY") or ""

    if not resolved_key:
        resolved_key = os.getenv("MINIMAX_CN_API_KEY") or hermes.get("MINIMAX_CN_API_KEY", "") or ""

    if not resolved_key:
        resolved_key = os.getenv("MINIMAX_API_KEY") or ""

    # Resolve base_url

    resolved_url = base_url

    if not resolved_url or resolved_url == "https://api.openai.com/v1":
        resolved_url = (
            os.getenv("MINIMAX_CN_BASE_URL") or hermes.get("MINIMAX_CN_BASE_URL", "") or ""
        )

    if not resolved_url:
        resolved_url = os.getenv("MINIMAX_BASE_URL") or os.getenv(
            "OPENAI_BASE_URL", "https://api.minimaxi.com/v1"
        )

    # Normalize hermes anthropic path

    if "/anthropic" in resolved_url:
        resolved_url = resolved_url.replace("/anthropic", "/v1")

    return resolved_url, resolved_key


def warm_cache(
    queries: List[str],
    model: str = "gpt-4o-mini",
    base_url: str = os.getenv("MINIMAX_BASE_URL")
    or os.getenv("OPENAI_BASE_URL", "https://api.openai.com/v1"),
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
                use_cache=True,  # Force cache write
            )

            results[query] = True

        except Exception as e:
            logger.warning("Failed to pre-warm cache for query %r: %s", query, e)

            results[query] = False

    return results


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


# Detect Warp CLI availability lazily


_warp_cli_client = None


def _get_warp_cli_client():
    """Get Warp CLI client, lazily initialized."""

    global _warp_cli_client

    if _warp_cli_client is None:
        try:
            from llm.warp_cli import WarpCLIClient, is_warp_cli_available

            if is_warp_cli_available():
                _warp_cli_client = WarpCLIClient()

        except ImportError:
            pass

    return _warp_cli_client


def _use_warp_cli_fallback(model: str) -> bool:
    """Check if we should use Warp CLI as the LLM backend.























    Use Warp CLI when:











    1. No OPENAI_API_KEY is set (Warp covers non-Anthropic models too)











    2. No ANTHROPIC_API_KEY is set











    3. Warp CLI is available











    """

    if os.getenv("OPENAI_API_KEY"):
        return False

    if os.getenv("ANTHROPIC_API_KEY"):
        return False

    return _get_warp_cli_client() is not None


# Anthropic API endpoint (override via ANTHROPIC_API_URL env for MiniMax etc.)


ANTHROPIC_API_URL = os.getenv("ANTHROPIC_API_URL", "https://api.anthropic.com/v1/messages")


# MiniMax API endpoint (override via MINIMAX_BASE_URL env)


MINIMAX_BASE_URL = os.getenv("MINIMAX_BASE_URL", "https://api.minimax.chat/v1")


ANTHROPIC_API_VERSION = "2023-06-01"


# Ollama local LLM endpoint (override via AIROS_OLLAMA_BASE_URL env)


OLLAMA_BASE_URL = os.getenv("AIROS_OLLAMA_BASE_URL", "http://localhost:11434")


# Detect if a model is Anthropic-native (claude-*)


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
    # Also auto-detect if no API key is set and Ollama is running
    if not os.getenv("OPENAI_API_KEY") and not os.getenv("ANTHROPIC_API_KEY"):
        if model.startswith("llama") or model.startswith("qwen") or model.startswith("mistral"):
            return True
    return False


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

    return hashlib.sha256(key_str).hexdigest()


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
    base_url: str = os.getenv("MINIMAX_BASE_URL")
    or os.getenv("OPENAI_BASE_URL", "https://api.openai.com/v1"),
    api_key: Optional[str] = None,
    timeout: int = 180,
    system_prompt: Optional[str] = None,
    stream: bool = False,
    use_cache: bool = True,
) -> str:

        # Auto-detect Ollama local model
    if _is_ollama_model(model):
        return _call_ollama_api(
            messages=messages,
            model=model,
            timeout=timeout,
            system_prompt=system_prompt,
            stream=stream,
            use_cache=use_cache,
        )

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

                return cast(
                    str,
                    cli.chat(
                        prompt=combined,
                        model=model,
                        system_prompt=system_prompt,
                    ),
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

    # Try Warp CLI as a zero-config fallback (covers any model type)

    if _use_warp_cli_fallback(model):
        cli = _get_warp_cli_client()

        if cli:
            combined = ""

            for msg in messages:
                role = msg.get("role", "user")

                content = msg.get("content", "")

                combined += f"{role.upper()}: {content}\n"

            if user_prompt:
                combined += f"USER: {user_prompt}\n"

            return cast(
                str,
                cli.chat(
                    prompt=combined,
                    model=model,
                    system_prompt=system_prompt,
                ),
            )

    resolved_url, resolved_key = _resolve_llm_credentials(base_url, api_key)

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

    # Generate cache key for non-streaming requests

    cache_key = None

    if use_cache and not stream:
        cache_key = _generate_cache_key(messages, model, user_prompt, system_prompt)

        # Check persistent cache first

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
        # MiniMax 思考模型需要禁用思考
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

            # Handle content blocks - skip "thinking" type, find "text" type (MiniMax compatibility)

            result = ""

            for block in data.get("content", []):
                if block.get("type") == "text":
                    result = block.get("text", "")

                    break

            if not result:
                raise RuntimeError(f"No text content in Anthropic response: {data}")

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


# ── Ollama Local LLM ─────────────────────────────────────────────────────


def _call_ollama_api(
    messages: List[Dict[str, str]],
    model: str,
    timeout: int = 180,
    system_prompt: Optional[str] = None,
    stream: bool = False,
    use_cache: bool = True,
) -> str:
    """Call Ollama's native /api/chat endpoint for local LLM inference.

    Strips the 'ollama/' prefix from model name before sending.
    """
    # Strip ollama/ prefix for the actual model name
    actual_model = model.replace("ollama/", "", 1)

    cache_key = None
    if use_cache and not stream:
        cache_key = _generate_cache_key(messages, actual_model, None, system_prompt)
        cached_response, found = _cache_read(cache_key)
        if found and cached_response:
            return cached_response

    session = _get_session()
    url = OLLAMA_BASE_URL.rstrip("/") + "/api/chat"

    ollama_messages = []
    if system_prompt:
        ollama_messages.append({"role": "system", "content": system_prompt})
    for msg in messages:
        role = msg.get("role", "user")
        if role in ("user", "assistant", "system"):
            ollama_messages.append({"role": role, "content": msg.get("content", "")})

    payload = {
        "model": actual_model,
        "messages": ollama_messages,
        "stream": stream,
        "options": {"temperature": 0.2},
    }

    try:
        headers = {"Content-Type": "application/json"}
        r = session.post(url, headers=headers, json=payload, timeout=timeout, stream=stream)
        r.raise_for_status()

        if stream:
            return _stream_ollama_to_string(r)
        else:
            data = r.json()
            result = data.get("message", {}).get("content", "")
            if not result:
                raise RuntimeError(f"No content in Ollama response: {data}")
            if cache_key and use_cache:
                _cache_write(cache_key, result)
            return result

    except requests.ConnectionError as e:
        raise RuntimeError(
            f"Ollama connection failed: {str(e)}. "
            f"Is Ollama running? Start it with: ollama serve\n"
            f"Then pull a model: ollama pull llama3.2"
        ) from e
    except requests.RequestException as e:
        msg = f"Ollama API request failed: {str(e)}"
        if "502" in str(e) or "connection" in str(e).lower():
            msg += (
                ". Is Ollama running? Start it with: ollama serve\n"
                "Then pull a model: ollama pull llama3.2"
            )
        raise RuntimeError(msg) from e
    except (KeyError, ValueError) as e:
        raise RuntimeError(f"Ollama API response parsing failed: {str(e)}") from e


def _stream_ollama_to_string(r: requests.Response) -> str:
    """Parse Ollama SSE stream into a string."""
    result = []
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


def stream_llm_chat_completions(
    messages: List[Dict[str, str]],
    model: str,
    user_prompt: Optional[str] = None,
    base_url: str = os.getenv("MINIMAX_BASE_URL")
    or os.getenv("OPENAI_BASE_URL", "https://api.openai.com/v1"),
    api_key: Optional[str] = None,
    timeout: int = 180,
    system_prompt: Optional[str] = None,
    use_cache: bool = True,
) -> Iterator[str]:
    """Stream LLM responses as an iterator of content deltas.























    Yields content deltas as they arrive from the SSE stream.











    Buffers full response and caches after streaming completes.























    Args:











        messages: Chat history











        model: Model name











        user_prompt: Additional user message











        base_url: API base URL











        api_key: API key











        timeout: Request timeout











        system_prompt: System prompt











        use_cache: Whether to use caching (default: True)























    Yields:











        Content deltas as strings











    """

    # Generate cache key for streaming requests

    cache_key = None

    if use_cache:
        cache_key = _generate_cache_key(messages, model, user_prompt, system_prompt)

        cached_response, found = _cache_read(cache_key)

        if found and cached_response:
            # Yield cached content as single delta for streaming interface

            yield cached_response

            return

        # Auto-detect Ollama local model
    if _is_ollama_model(model):
        result = _call_ollama_api(
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
            use_cache=False,  # Already handled above
        )

        # Cache the streamed result

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

        # Buffer full response for caching

        full_response = ""

        for delta in _parse_sse_stream(r):
            yield delta

            full_response += delta

        # Cache after streaming completes

        if cache_key and full_response:
            _cache_write(cache_key, full_response)

    except requests.RequestException as e:
        raise RuntimeError(f"LLM API request failed: {str(e)}") from e


def get_client(
    model: str = "minimax-m2.7-highspeed",
    base_url: str = "",
    api_key: str = "",
) -> callable:
    """Get a callable LLM client for evolution.py.























    Returns a wrapper around call_llm_chat_completions that provides a











    .generate(prompt) interface expected by InsightEvolution.























    Reads credentials from (in order of priority):











    1. Explicit api_key / base_url arguments











    2. MINIMAX_API_KEY / MINIMAX_BASE_URL env vars











    3. MINIMAX_CN_API_KEY env var (Hermes native MiniMax key)











    4. ~/.hermes/config.yaml + ~/.hermes/.env (auto-detected)























    Args:











        model: Model name (default: minimax-m2.7-highspeed)











        base_url: API base URL (auto-detected if empty)











        api_key: API key (auto-detected if empty)























    Returns:











        A callable with .generate(prompt) method that calls the LLM.











    """

    import os as _os

    from pathlib import Path as _Path

    # Auto-detect from hermes config if not provided

    if not api_key or not base_url:
        _hermes_home = _Path.home() / ".hermes"

        _env_file = _hermes_home / ".env"

        _config_file = _hermes_home / "config.yaml"

        # Read hermes .env

        if _env_file.exists():
            try:
                for _line in _env_file.read_text(encoding="utf-8").splitlines():
                    _line = _line.strip()

                    if "=" in _line and not _line.startswith("#"):
                        _k, _v = _line.split("=", 1)

                        if _k == "MINIMAX_CN_API_KEY" and not api_key:
                            api_key = _v.strip()

                        elif _k == "MINIMAX_CN_BASE_URL" and not base_url:
                            base_url = _v.strip()

            except Exception:
                pass

        # Read hermes config.yaml for base_url

        if not base_url:
            try:
                import yaml as _yaml

                if _config_file.exists():
                    _cfg = _yaml.safe_load(_config_file.read_text(encoding="utf-8"))

                    _model_cfg = _cfg.get("model", {}) or {}

                    _base = _model_cfg.get("base_url", "")

                    if _base:
                        base_url = _base

            except Exception:
                pass

        # Fallback env vars

        if not api_key:
            api_key = _os.getenv("MINIMAX_API_KEY", "")

        if not base_url:
            base_url = _os.getenv("MINIMAX_BASE_URL", "https://api.minimaxi.com/anthropic")

        # Normalize hermes minimax base_url: /anthropic → /v1

        # hermes config uses /anthropic but actual API uses /v1

        if "/anthropic" in base_url:
            base_url = base_url.replace("/anthropic", "/v1")

    def client_wrapper(messages: list) -> str:

        return call_llm_chat_completions(
            messages=messages,
            model=model,
            base_url=base_url,
            api_key=api_key,
        )

    def generate(prompt: str) -> str:
        """Direct LLM call for InsightEvolution — handles MiniMax thinking tags."""

        import re as _re

        _session = _get_session()

        url = resolved_url.rstrip("/") + "/chat/completions"

        payload = {
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "temperature": 0.2,
            "extra_body": {"thinking": {"type": "disabled"}},
        }

        headers = {"Authorization": f"Bearer {api_key}", "Content-Type": "application/json"}

        try:
            r = _session.post(url, headers=headers, json=payload, timeout=180)

            r.raise_for_status()

            data = r.json()

            raw = data["choices"][0]["message"]["content"]

            # Strip MiniMax thinking tags

            clean = _re.sub(r"<think>.*?</think>", "", raw, flags=_re.DOTALL).strip()

            # Strip markdown code block wrappers (```json ... ``` or just ``` ...)

            clean = _re.sub(r"^\s*json\s*", "", clean)  # "json\n{..." case

            clean = _re.sub(r"\`\`\`json\s*", "", _re.sub(r"\`\`\`\s*", "", clean)).strip()

            return clean

        except Exception:
            return ""

    client_wrapper.generate = generate

    return client_wrapper
