"""Gap extraction from papers — lightweight single-paper gap analysis."""

from __future__ import annotations

import json
from typing import List, Optional


def _extract_keywords(text: str) -> List[str]:
    """Simple keyword extraction from text."""
    words = text.lower().split()
    stop = {
        "the",
        "a",
        "an",
        "of",
        "in",
        "to",
        "for",
        "and",
        "or",
        "with",
        "is",
        "are",
        "that",
        "this",
        "we",
        "our",
        "by",
        "on",
        "as",
        "at",
        "from",
    }
    keywords = [w.strip(".,;:!?()[]{}") for w in words if len(w) > 4 and w not in stop]
    return list(dict.fromkeys(keywords))[:6]


def extract_gap_from_paper(
    paper_id: str,
    title: str,
    abstract: str,
    authors: Optional[List[str]] = None,
    api_key: Optional[str] = None,
    base_url: Optional[str] = None,
    model: Optional[str] = None,
) -> dict:
    """Extract research gap from a paper.

    Returns a dict with:
        gap_type: one of the GapType enum values
        gap_title: short title for the gap
        keywords: list of extracted keywords
        summary: brief explanation of what gap this paper addresses
    """
    authors_str = ", ".join(authors[:3]) if authors else "Unknown"

    prompt = f"""You are a research gap detector. Given a paper's title and abstract, extract ONE research gap that this paper addresses.

PAPER:
Title: {title}
Authors: {authors_str}
Abstract: {abstract[:800]}

TASK:
Identify what research gap this paper addresses. Choose ONE gap type from:
- unexplored_application: using known methods in new domains
- method_limitation: current methods have specific drawbacks
- contradiction: paper challenges or refutes existing findings
- evaluation_gap: lack of proper benchmarks or evaluation
- scalability_issue: methods don't scale to real-world settings
- theoretical_gap: lack of theoretical foundations
- dataset_gap: no suitable dataset for the problem
- generalization_gap: methods fail on out-of-distribution data

Respond ONLY with a JSON object (no markdown, no code fences):
{{
  "gap_type": "the_gap_type_here",
  "gap_title": "Short descriptive title (max 80 chars)",
  "keywords": ["keyword1", "keyword2", "keyword3"],
  "summary": "One sentence explaining what gap this paper addresses",
  "polarity": "positive or negative"
}}
"""

    try:
        from llm.client import call_llm_chat_completions
    except ImportError:
        return {
            "gap_type": "method_limitation",
            "gap_title": title[:80],
            "keywords": _extract_keywords(abstract),
            "summary": "Gap extraction requires llm.client.",
            "error": "llm.client not available",
        }

    try:
        content = call_llm_chat_completions(
            messages=[{"role": "user", "content": prompt}],
            model="claude-3-5-sonnet-latest",
        )
        return json.loads(content.strip())  # type: ignore[no-any-return]
    except Exception as e:
        return {
            "gap_type": "method_limitation",
            "gap_title": title[:80],
            "keywords": _extract_keywords(abstract),
            "summary": f"LLM call failed: {e}",
            "error": str(e),
        }
