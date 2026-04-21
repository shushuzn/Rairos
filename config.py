"""Centralised configuration loaded from environment variables.

All hardcoded magic numbers in the codebase must be accessed via this module
rather than inlined.  Defaults match the historical values so the module is
safe to import even when no .env file is present.
"""
from __future__ import annotations

import os
from typing import Tuple

# ---------------------------------------------------------------------------
# Embedding
# ---------------------------------------------------------------------------
EMBEDDING_DIM: int = int(os.getenv("AIROS_EMBEDDING_DIM", "768"))
"""Embedding vector dimension (nomic-embed-text uses 768)."""

# ---------------------------------------------------------------------------
# HTTP cache
# ---------------------------------------------------------------------------
CACHE_TTL_SECONDS: int = int(os.getenv("AIROS_CACHE_TTL_SECONDS", str(24 * 3600)))
"""How long cached arXiv / Crossref API responses live, in seconds."""

# ---------------------------------------------------------------------------
# LLM cost table  (input_price_per_1M, output_price_per_1M)
# ---------------------------------------------------------------------------
def _load_model_prices() -> dict[str, Tuple[float, float]]:
    """Parse AIROS_MODEL_PRICES from the environment.

    Format: comma-separated ``model_prefix:input:output`` triples, e.g.
    ``gpt-4o:2.5:10.0,gpt-4o-mini:0.15:0.6``.

    If the env var is absent or malformed we fall back to the built-in table.
    """
    raw = os.getenv("AIROS_MODEL_PRICES", "").strip()
    if not raw:
        return {}
    prices: dict[str, Tuple[float, float]] = {}
    for entry in raw.split(","):
        parts = entry.strip().split(":")
        if len(parts) != 3:
            continue
        try:
            model, inp, out = parts[0].strip(), float(parts[1]), float(parts[2])
            prices[model] = (inp, out)
        except ValueError:
            pass
    return prices


# Built-in table – mirrors the original hardcoded values in llm/generate.py
_BUILTIN_MODEL_PRICES: dict[str, Tuple[float, float]] = {
    "gpt-4o": (2.5, 10.0),
    "gpt-4o-mini": (0.15, 0.6),
    "gpt-4-turbo": (10.0, 30.0),
    "gpt-3.5-turbo": (0.5, 1.5),
    "o1-preview": (15.0, 60.0),
    "o1-mini": (3.0, 12.0),
    "qwen3.5-plus": (0.1, 0.3),
    "qwen3.5": (0.1, 0.3),
    "qwen2.5": (0.1, 0.3),
    "deepseek-chat": (0.14, 0.28),
    "claude-3-5-sonnet": (3.0, 15.0),
    "claude-3-5-haiku": (0.8, 4.0),
    "default": (1.0, 4.0),
}

MODEL_PRICES: dict[str, Tuple[float, float]] = {
    **_BUILTIN_MODEL_PRICES,
    **_load_model_prices(),
}
"""Token price table, can be extended via AIROS_MODEL_PRICES env var."""

# ---------------------------------------------------------------------------
# Default models
# ---------------------------------------------------------------------------
DEFAULT_LLM_MODEL_CLI: str = os.getenv("AIROS_DEFAULT_MODEL_CLI", "qwen3.5-plus")
"""Default LLM model used by the CLI."""

DEFAULT_LLM_MODEL_RESEARCH: str = os.getenv("AIROS_DEFAULT_MODEL_RESEARCH", "gpt-4o-mini")
"""Default LLM model used by the research loop."""

# ---------------------------------------------------------------------------
# Miscellaneous
# ---------------------------------------------------------------------------
MAX_PARSE_AUTHORS_CACHE_SIZE: int = int(os.getenv("AIROS_PARSE_AUTHORS_CACHE_SIZE", "4096"))
"""Maxsize passed to the ``lru_cache`` wrapping the author JSON parser."""
