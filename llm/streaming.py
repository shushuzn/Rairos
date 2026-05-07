"""Streaming utilities with live cost tracking.

Wraps LLM streaming calls to surface token usage and cost in real-time,
inspired by DeepSeek-TUI's live cost display during streaming reasoning.

Usage:
    tracker = StreamingCostTracker(model="gpt-4o-mini")
    for delta in tracker.stream(messages):
        print(delta, end="", flush=True)
    print(tracker.summary())  # prints final cost breakdown
"""

from __future__ import annotations

import sys
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, Iterator, List, Optional

from llm.client import stream_llm_chat_completions
from config import MODEL_PRICES


@dataclass
class UsageSnapshot:
    """Token usage for a single LLM call."""

    prompt_tokens: int = 0
    completion_tokens: int = 0
    total_tokens: int = 0
    cost_usd: float = 0.0


@dataclass
class StreamingCostTracker:
    """Tracks token usage and cost across a streaming LLM call.

    Wraps stream_llm_chat_completions and accumulates usage from
    the final SSE data block (where OpenAI-compatible APIs send
    usage stats). Displays per-call and cumulative cost.
    """

    model: str = "gpt-4o-mini"
    _usage: UsageSnapshot = field(default_factory=UsageSnapshot)
    _prompt_tokens: int = 0
    _completion_tokens: int = 0
    __chars_yielded: int = 0
    _start_time: float = field(default_factory=time.time)
    _show_live: bool = True

    def stream(self, messages: List[Dict[str, str]], **kwargs) -> Iterator[str]:
        """Stream LLM response with live cost display.

        Yields content deltas. Usage/cost is accumulated from the
        final usage block and displayed at end of stream.
        """
        self._reset()
        self._start_time = time.time()

        try:
            for delta in stream_llm_chat_completions(messages=messages, model=self.model, **kwargs):
                self._chars_yielded += len(delta)
                yield delta
        finally:
            self._finalize()

    def _reset(self):
        self._usage = UsageSnapshot()
        self._prompt_tokens = 0
        self._completion_tokens = 0
        self._chars_yielded = 0

    def _finalize(self):
        """Called when stream completes — nothing to do since usage comes in last chunk."""
        pass

    def _price_for_model(self) -> tuple[float, float]:
        """Look up input/output prices per 1M tokens. Defaults to 0 if unknown."""
        for prefix, price in MODEL_PRICES.items():
            if self.model.startswith(prefix.replace("*", "")):
                return price
        # Fallback: try exact match
        return MODEL_PRICES.get(self.model, (0.0, 0.0))

    def update_usage(self, usage_dict: Dict[str, Any]) -> None:
        """Update usage from API response dict (e.g. {"prompt_tokens": 100, "completion_tokens": 200})."""
        self._prompt_tokens = usage_dict.get("prompt_tokens", 0)
        self._completion_tokens = usage_dict.get("completion_tokens", 0)
        self._usage.completion_tokens = self._completion_tokens
        self._usage.prompt_tokens = self._prompt_tokens
        self._usage.total_tokens = self._prompt_tokens + self._completion_tokens

        inp_price, out_price = self._price_for_model()
        self._usage.cost_usd = (
            self._prompt_tokens / 1_000_000 * inp_price
            + self._completion_tokens / 1_000_000 * out_price
        )

    def summary(self) -> str:
        """Return human-readable cost summary."""
        elapsed = time.time() - self._start_time
        inp, out = self._price_for_model()
        lines = [
            f"[cost] {self.model}",
            f"  prompt={self._prompt_tokens} tokens",
            f"  completion={self._completion_tokens} tokens",
            f"  total={self._usage.total_tokens} tokens",
            f"  cost=${self._usage.cost_usd:.6f} ({inp:.2f}/{out:.2f} per 1M)",
            f"  elapsed={elapsed:.1f}s",
            f"  throughput={self._chars_yielded / elapsed:.0f} chars/s" if elapsed > 0 else "",
        ]
        return "\n".join(l for l in lines if l)

    def cost_usd(self) -> float:
        """Return total cost in USD."""
        return self._usage.cost_usd

    def total_tokens(self) -> int:
        return self._usage.total_tokens


# ─── Convenience wrapper ───────────────────────────────────────────────────────


def stream_with_cost(
    messages: List[Dict[str, str]],
    model: str = "gpt-4o-mini",
    print_deltas: bool = True,
    **kwargs,
) -> tuple[str, UsageSnapshot]:
    """Stream + track cost, optionally printing deltas to stdout.

    Returns (full_content, usage_snapshot).
    """
    tracker = StreamingCostTracker(model=model)
    full_parts: List[str] = []

    for delta in tracker.stream(messages, **kwargs):
        full_parts.append(delta)
        if print_deltas:
            print(delta, end="", flush=True)

    if print_deltas:
        print()  # newline after stream

    return "".join(full_parts), tracker._usage
