"""Shared text processing utilities for research analysis."""
from __future__ import annotations

import re
from typing import List

# Stopwords excluded from keyword extraction
_KEYWORD_STOPWORDS: frozenset = frozenset({
    "the", "and", "for", "are", "but", "not", "you", "all",
    "can", "had", "her", "was", "one", "our", "out", "has",
    "have", "been", "with", "they", "this", "that", "from",
    "will", "would", "there", "their", "what", "about", "which",
    "when", "make", "just", "over", "such", "into", "than",
    "null", "none", "also", "how", "may", "does",
    "method", "approach", "gap", "issue", "problem", "limitation",
    "study", "work", "paper", "research", "based", "using",
})


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
