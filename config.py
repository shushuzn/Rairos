"""Centralised configuration loaded from environment variables.

All hardcoded magic numbers in the codebase must be accessed via this module
rather than inlined.  Defaults match the historical values so the module is
safe to import even when no .env file is present.

Environment variables follow the pattern AIROS_<CONFIG_NAME> for consistency.
"""
from __future__ import annotations

import os
from pathlib import Path
from typing import Tuple, Dict, Any

# Load .env file if present
_env_file = Path(__file__).parent / ".env"
if _env_file.exists():
    with open(_env_file, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line and not line.startswith("#") and "=" in line:
                key, _, value = line.partition("=")
                os.environ.setdefault(key.strip(), value.strip())

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

CACHE_DIR: str = os.getenv("AIROS_CACHE_DIR", str(Path(__file__).parent / "data"))
"""Directory for disk cache storage."""

MAX_CACHE_FILES: int = int(os.getenv("AIROS_MAX_CACHE_FILES", "2000"))
"""Maximum number of cache files per directory."""

MEMORY_CACHE_MAX_SIZE: int = int(os.getenv("AIROS_MEMORY_CACHE_MAX_SIZE", "1000"))
"""Maximum number of items in memory cache."""

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
    "minimax-m2.7-highspeed": (0.1, 0.1),  # MiniMax M2.7 高速版
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
# LLM API configuration
# ---------------------------------------------------------------------------
DEFAULT_OPENAI_BASE_URL: str = os.getenv("AIROS_DEFAULT_OPENAI_BASE_URL", "https://api.openai.com/v1")
"""Default OpenAI-compatible API base URL."""

DEFAULT_LLM_TIMEOUT: int = int(os.getenv("AIROS_LLM_TIMEOUT", "180"))
"""Default timeout for LLM API calls in seconds."""

# ---------------------------------------------------------------------------
# PDF processing
# ---------------------------------------------------------------------------
PDF_MAX_PAGES: int = int(os.getenv("AIROS_PDF_MAX_PAGES", "100"))
"""Maximum number of pages to process from a PDF."""

PDF_OCR_ZOOM: float = float(os.getenv("AIROS_PDF_OCR_ZOOM", "2.0"))
"""Zoom factor for OCR processing."""

PDF_OCR_LANG: str = os.getenv("AIROS_PDF_OCR_LANG", "chi_sim+eng")
"""Default OCR language(s)."""

# ---------------------------------------------------------------------------
# Tagging
# ---------------------------------------------------------------------------
MAX_TAGS: int = int(os.getenv("AIROS_MAX_TAGS", "5"))
"""Maximum number of tags to infer for a paper."""

# ---------------------------------------------------------------------------
# Research loop
# ---------------------------------------------------------------------------
RESEARCH_LOOP_DEFAULT_LIMIT: int = int(os.getenv("AIROS_RESEARCH_LOOP_DEFAULT_LIMIT", "5"))
"""Default number of papers to process in research loop."""

RESEARCH_LOOP_DEFAULT_OUTPUT_DIR: str = os.getenv("AIROS_RESEARCH_LOOP_DEFAULT_OUTPUT_DIR", "")
"""Default output directory for research loop."""

# ---------------------------------------------------------------------------
# Miscellaneous
# ---------------------------------------------------------------------------
MAX_PARSE_AUTHORS_CACHE_SIZE: int = int(os.getenv("AIROS_PARSE_AUTHORS_CACHE_SIZE", "4096"))
"""Maxsize passed to the ``lru_cache`` wrapping the author JSON parser."""

CONCURRENT_WORKERS: int = int(os.getenv("AIROS_CONCURRENT_WORKERS", "8"))
"""Number of concurrent workers for parallel operations."""

# ---------------------------------------------------------------------------
# Helper functions
# ---------------------------------------------------------------------------
def get_config() -> Dict[str, Any]:
    """Get all configuration values as a dictionary."""
    return {
        "EMBEDDING_DIM": EMBEDDING_DIM,
        "CACHE_TTL_SECONDS": CACHE_TTL_SECONDS,
        "CACHE_DIR": CACHE_DIR,
        "MAX_CACHE_FILES": MAX_CACHE_FILES,
        "MEMORY_CACHE_MAX_SIZE": MEMORY_CACHE_MAX_SIZE,
        "MODEL_PRICES": MODEL_PRICES,
        "DEFAULT_LLM_MODEL_CLI": DEFAULT_LLM_MODEL_CLI,
        "DEFAULT_LLM_MODEL_RESEARCH": DEFAULT_LLM_MODEL_RESEARCH,
        "DEFAULT_OPENAI_BASE_URL": DEFAULT_OPENAI_BASE_URL,
        "DEFAULT_LLM_TIMEOUT": DEFAULT_LLM_TIMEOUT,
        "PDF_MAX_PAGES": PDF_MAX_PAGES,
        "PDF_OCR_ZOOM": PDF_OCR_ZOOM,
        "PDF_OCR_LANG": PDF_OCR_LANG,
        "MAX_TAGS": MAX_TAGS,
        "RESEARCH_LOOP_DEFAULT_LIMIT": RESEARCH_LOOP_DEFAULT_LIMIT,
        "RESEARCH_LOOP_DEFAULT_OUTPUT_DIR": RESEARCH_LOOP_DEFAULT_OUTPUT_DIR,
        "MAX_PARSE_AUTHORS_CACHE_SIZE": MAX_PARSE_AUTHORS_CACHE_SIZE,
        "CONCURRENT_WORKERS": CONCURRENT_WORKERS,
    }

def validate_config() -> bool:
    """Validate configuration values."""
    # Add validation logic here if needed
    return True
