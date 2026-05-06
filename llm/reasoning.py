"""Streaming reasoning/extended thinking wrapper for MiniMax/DeepSeek V3 API.

Wraps stream_llm_chat_completions to surface intermediate reasoning steps
(e.g. decomposition, search, synthesis) as separate ReasoningBlock deltas
before the final answer content.

Usage:
    reasoner = StreamingReasoner()
    for chunk in reasoner.stream("Why is the sky blue?", model="deepseek-chat"):
        print(chunk, end="", flush=True)

    # or with callbacks:
    reasoner = StreamingReasoner()
    for chunk in reasoner.stream(query, model="deepseek-chat",
                                  on_chunk=print_str,
                                  on_reasoning=print_reasoning_block):
        ...

    # convenience function:
    for chunk in stream_with_reasoning(messages, model, on_chunk, on_reasoning):
        ...
"""

from __future__ import annotations

import os
from dataclasses import dataclass, field
from typing import Any, Callable, Iterator, List, Optional

# Reuse SSE parsing from client; import the raw stream so we can reparse
# with thinking-field awareness.
from llm.client import stream_llm_chat_completions


# ---------------------------------------------------------------------------
# Data structures
# ---------------------------------------------------------------------------


@dataclass
class ReasoningBlock:
    """A single reasoning phase emitted during extended thinking.

    Attributes:
        phase:  Short name for the reasoning stage
                (e.g. "decomposition", "search", "synthesis", "reflection").
                Empty string "" means the default/unclassified phase.
        content: Raw text delta for this phase.
        done:    True when this is the last chunk for this phase
                (i.e. a new phase has started or the stream has ended).
    """
    phase: str = ""
    content: str = ""
    done: bool = False


# ---------------------------------------------------------------------------
# Internal SSE re-parser
# ---------------------------------------------------------------------------

# Known phase labels that reasoning models emit as part of their thinking
# content (without a formal phase field).  Used by _infer_phase.
_KNOWN_PHASES = frozenset([
    "decomposition", "analysis", "search", "retrieval",
    "reasoning", "planning", "synthesis", "reflection",
    "verification", "conclusion", "decomposition",
])


def _infer_phase(text: str) -> str:
    """Coarse phase label inferred from the start of a reasoning chunk.

    Looks for known keywords at the beginning of ``text`` (case-insensitive)
    and returns the normalised phase name, otherwise ``""``.
    """
    import re
    lowered = text.lower().strip()
    # Match "Phase: Label" or "[Label]" or just "Label:"
    m = re.match(r"^\[([^\]]+)\]|^([a-z]+):", lowered)
    if m:
        candidate = (m.group(1) or m.group(2) or "").strip()
        if candidate in _KNOWN_PHASES:
            return candidate
    return ""


def _parse_thinking_stream(r: Iterator[Any]) -> Iterator[tuple[Optional[ReasoningBlock], Optional[str]]]:
    """Parse an SSE stream that may contain thinking/reasoning events.

    This is a low-level helper that walks the raw SSE lines emitted by a
    MiniMax or DeepSeek V3 API call that has extended thinking enabled.

    It yields tuples of ``(reasoning_block, content_delta)``:
      * ``(ReasoningBlock(...), None)`` — a reasoning phase delta
      * ``(None, "...")``               — a final content delta
      * ``(None, None)``               — an empty or unrecognised line

    The caller is responsible for consuming the yielded ``ReasoningBlock``
    via ``on_reasoning`` before yielding the next content delta.

    SSE format variants handled::

        # DeepSeek V3 (thinking block as delta with content_type)
        data: {"choices":[{"delta":{"content_type":"reasoning","content":"..."}}]}

        # MiniMax plugins / extended thinking
        data: {"choices":[{"delta":{"type":"thinking","content":"..."}}]}

        # Standard completion delta (no thinking)
        data: {"choices":[{"delta":{"content":"hello"}}]}
    """
    current_phase = ""
    buf = ""

    for line in r:
        # Accept both raw SSE lines and already-parsed dicts.
        if isinstance(line, str):
            if not line.startswith("data: "):
                continue
            import orjson
            payload = line[6:].strip()
            if payload == "[DONE]":
                # Flush any pending reasoning.
                if buf:
                    yield ReasoningBlock(phase=current_phase, content=buf, done=True), None
                break
            try:
                obj = orjson.loads(payload)
            except Exception:
                continue
        elif isinstance(line, dict):
            obj = line
        else:
            continue

        choices = obj.get("choices", [{}])
        if not choices:
            continue
        delta = choices[0].get("delta", {})

        # 1) DeepSeek V3 / MiniMax extended thinking (content_type field).
        content_type = delta.get("content_type", "")
        if content_type in ("reasoning", "thinking"):
            text = delta.get("content", "")
            if not text:
                continue
            # Phase label may appear inline (e.g. "[decomposition] Some text").
            detected = _infer_phase(text)
            if detected and detected != current_phase:
                # Phase changed — close the old block.
                if buf:
                    yield ReasoningBlock(phase=current_phase, content=buf, done=True), None
                current_phase = detected
                buf = text
            else:
                buf += text
            # Emit reasoning block immediately (buffered).
            yield ReasoningBlock(phase=current_phase, content=text, done=False), None
            continue

        # 2) MiniMax "type=thinking" variant.
        type_ = delta.get("type", "")
        if type_ == "thinking":
            text = delta.get("content", "")
            if not text:
                continue
            detected = _infer_phase(text)
            if detected and detected != current_phase:
                if buf:
                    yield ReasoningBlock(phase=current_phase, content=buf, done=True), None
                current_phase = detected
                buf = text
            else:
                buf += text
            yield ReasoningBlock(phase=current_phase, content=text, done=False), None
            continue

        # 3) Standard content delta — flush any pending reasoning first.
        content = delta.get("content", "")
        if not content:
            continue
        if buf:
            yield ReasoningBlock(phase=current_phase, content=buf, done=True), None
            buf = ""
            current_phase = ""
        yield None, content

    # Final flush.
    if buf:
        yield ReasoningBlock(phase=current_phase, content=buf, done=True), None


# ---------------------------------------------------------------------------
# High-level wrapper
# ---------------------------------------------------------------------------


class StreamingReasoner:
    """Stream an LLM response with extended-thinking support.

    Parameters:
        model:            Model name (default ``"deepseek-chat"``).
        extended_thinking: Enable extended thinking for compatible models
                         (default ``True``).
    """

    def __init__(
        self,
        model: str = "deepseek-chat",
        extended_thinking: bool = True,
    ):
        self.model = model
        self.extended_thinking = extended_thinking

    # -----------------------------------------------------------------------
    # Public API
    # -----------------------------------------------------------------------

    def stream(
        self,
        query: str,
        on_chunk: Optional[Callable[[str], None]] = None,
        on_reasoning: Optional[Callable[[ReasoningBlock], None]] = None,
        **kwargs,
    ) -> Iterator[str]:
        """Stream a query with reasoning-phase callbacks.

        Args:
            query:         User query string.
            on_chunk:      Called for every final-content delta: ``fn(content: str)``.
            on_reasoning:  Called for every reasoning block: ``fn(block: ReasoningBlock)``.
            **kwargs:      Forwarded to ``stream_llm_chat_completions`` (e.g. ``base_url``, ``api_key``).

        Yields:
            Final content deltas (str).  Callbacks fire side-effectfully.

        Note:
            When the API does not emit thinking events this falls back to
            plain streaming — ``on_reasoning`` is simply never called.
        """
        messages: List[dict] = [{"role": "user", "content": query}]
        yield from self._stream_messages(
            messages=messages,
            on_chunk=on_chunk,
            on_reasoning=on_reasoning,
            **kwargs,
        )

    def stream_messages(
        self,
        messages: List[dict],
        on_chunk: Optional[Callable[[str], None]] = None,
        on_reasoning: Optional[Callable[[ReasoningBlock], None]] = None,
        **kwargs,
    ) -> Iterator[str]:
        """Stream an existing message list with reasoning-phase callbacks.

        Args:
            messages:      List of ``{"role": ..., "content": ...}`` dicts.
            on_chunk:      Called for every final-content delta.
            on_reasoning: Called for every reasoning block.
            **kwargs:      Forwarded to ``stream_llm_chat_completions``.

        Yields:
            Final content deltas (str).
        """
        yield from self._stream_messages(
            messages=messages,
            on_chunk=on_chunk,
            on_reasoning=on_reasoning,
            **kwargs,
        )

    # -----------------------------------------------------------------------
    # Private core
    # -----------------------------------------------------------------------

    def _stream_messages(
        self,
        messages: List[dict],
        on_chunk: Optional[Callable[[str], None]] = None,
        on_reasoning: Optional[Callable[[ReasoningBlock], None]] = None,
        **kwargs,
    ) -> Iterator[str]:
        """Send messages and parse the SSE for thinking events."""
        # Merge extended_thinking into extra_body.
        extra: dict = kwargs.pop("extra_body", {})
        if self.extended_thinking:
            extra = dict(extra)  # copy so we don't mutate caller's dict
            # DeepSeek V3 / MiniMax-compatible extended-thinking signal.
            extra.setdefault("thinking", {"type": "enabled"})
        kwargs["extra_body"] = extra

        # stream_llm_chat_completions is a sync generator.
        # We need to re-parse it as SSE lines, so we call it and walk
        # the raw response object from the session.
        import requests
        from llm.client import (
            _get_session,
            _generate_cache_key,
            _cache_read,
        )
        import orjson

        base_url = kwargs.pop("base_url", os.getenv("MINIMAX_BASE_URL") or os.getenv("OPENAI_BASE_URL", "https://api.openai.com/v1"))
        api_key = kwargs.pop("api_key", os.getenv("OPENAI_API_KEY", ""))
        model = kwargs.pop("model", self.model)
        timeout = kwargs.pop("timeout", 180)

        url = base_url.rstrip("/") + "/chat/completions"
        msgs = list(messages)

        payload = {
            "model": model,
            "temperature": 0.2,
            "messages": msgs,
            "stream": True,
            "extra_body": extra,
        }

        session = _get_session()
        headers = {"Authorization": f"Bearer {api_key}"} if api_key else {}

        try:
            r = session.post(url, headers=headers, json=payload, timeout=timeout, stream=True)
            r.raise_for_status()
        except requests.RequestException as e:
            # Graceful fallback: try plain streaming without extended thinking.
            if self.extended_thinking:
                extra["thinking"] = {"type": "disabled"}
                payload["extra_body"] = extra
                try:
                    r = session.post(url, headers=headers, json=payload, timeout=timeout, stream=True)
                    r.raise_for_status()
                except requests.RequestException:
                    raise RuntimeError(f"LLM API request failed (extended thinking disabled): {e}") from e
            else:
                raise RuntimeError(f"LLM API request failed: {e}") from e

        # Re-parse the SSE stream with thinking awareness.
        sse_lines = r.iter_lines(decode_unicode=True)

        for reasoning_block, content_delta in _parse_thinking_stream(sse_lines):
            if reasoning_block is not None:
                if on_reasoning:
                    on_reasoning(reasoning_block)
            if content_delta is not None:
                if on_chunk:
                    on_chunk(content_delta)
                yield content_delta


# ---------------------------------------------------------------------------
# Convenience function
# ---------------------------------------------------------------------------

def stream_with_reasoning(
    messages: List[dict],
    model: str = "deepseek-chat",
    on_chunk: Optional[Callable[[str], None]] = None,
    on_reasoning: Optional[Callable[[ReasoningBlock], None]] = None,
    **kwargs,
) -> Iterator[str]:
    """Stream chat completions with extended-thinking support.

    Shortcut for::

        reasoner = StreamingReasoner(model=model)
        yield from reasoner.stream_messages(messages, on_chunk=on_chunk,
                                             on_reasoning=on_reasoning, **kwargs)

    Args:
        messages:      List of ``{"role": ..., "content": ...}`` dicts.
        model:         Model name.
        on_chunk:      Called for every final-content delta.
        on_reasoning:  Called for every reasoning block.
        **kwargs:      Forwarded to ``stream_llm_chat_completions``.

    Yields:
        Final content deltas (str).
    """
    reasoner = StreamingReasoner(model=model)
    yield from reasoner.stream_messages(
        messages,
        on_chunk=on_chunk,
        on_reasoning=on_reasoning,
        **kwargs,
    )
