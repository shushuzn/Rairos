"""Shared text processing utilities for research analysis."""

from __future__ import annotations

import math
import re
from typing import List

# Stopwords excluded from keyword extraction
_KEYWORD_STOPWORDS: frozenset = frozenset(
    {
        "the",
        "and",
        "for",
        "are",
        "but",
        "not",
        "you",
        "all",
        "can",
        "had",
        "her",
        "was",
        "one",
        "our",
        "out",
        "has",
        "have",
        "been",
        "with",
        "they",
        "this",
        "that",
        "from",
        "will",
        "would",
        "there",
        "their",
        "what",
        "about",
        "which",
        "when",
        "make",
        "just",
        "over",
        "such",
        "into",
        "than",
        "null",
        "none",
        "also",
        "how",
        "may",
        "does",
        "method",
        "approach",
        "gap",
        "issue",
        "problem",
        "limitation",
        "study",
        "work",
        "paper",
        "research",
        "based",
        "using",
    }
)


def extract_keywords(text: str, min_len: int = 3) -> List[str]:
    """Extract research-relevant keywords from text.

    Args:
        text: Input text to extract keywords from.
        min_len: Minimum keyword length (default 3).

    Returns:
        Lowercase keywords of min_len+ characters, excluding common stopwords.
    """
    words = re.findall(r"[A-Za-z0-9]+", text.lower())
    return [w for w in words if len(w) >= min_len and w not in _KEYWORD_STOPWORDS]


def cosine_sim(a: List[float], b: List[float]) -> float:
    """Cosine similarity between two vectors (0-1). Returns 0 if either norm is 0."""
    dot = sum(x * y for x, y in zip(a, b, strict=True))
    norm_a = math.sqrt(sum(x * x for x in a))
    norm_b = math.sqrt(sum(y * y for y in b))
    if norm_a == 0 or norm_b == 0:
        return 0.0
    return dot / (norm_a * norm_b)


def jaccard(a: List[str], b: List[str]) -> float:
    """Jaccard similarity between two sets (0-1)."""
    if not a or not b:
        return 0.0
    set_a, set_b = set(a), set(b)
    intersection = len(set_a & set_b)
    union = len(set_a | set_b)
    return intersection / union if union > 0 else 0.0
