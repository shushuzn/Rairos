"""Paper scout — proactively finds papers matching Gene Pool interests."""

from __future__ import annotations

import logging
import sys
import time
from dataclasses import dataclass, field
from datetime import datetime, timedelta
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

logger = logging.getLogger(__name__)

# Cache for ArXiv search results
_SEARCH_CACHE: Dict[str, Tuple[float, List[Dict]]] = {}
_CACHE_TTL = 3600  # 1 hour


@dataclass
class ScoutResult:
    """A paper found by the scout, scored against Gene Pool."""

    arxiv_id: str
    title: str
    authors: List[str]
    abstract: str
    categories: List[str]
    published: str

    # Gene Pool matching
    match_score: float = 0.0  # 0.0–1.0, best capsule trigger_match × credibility
    matched_capsule_id: str = ""
    matched_gap_title: str = ""
    matched_gap_type: str = ""
    credibility_of_match: float = 0.0

    # Ranking
    rank: int = 0  # overall rank in this scout run
    reason: str = ""  # human-readable reason for recommendation


def _search_arxiv(query: str, max_results: int = 10) -> List[Dict]:
    """Search ArXiv, with simple cache."""
    cache_key = f"{query}:{max_results}"
    now = time.time()
    if cache_key in _SEARCH_CACHE:
        ts, results = _SEARCH_CACHE[cache_key]
        if now - ts < _CACHE_TTL:
            return results

    try:
        from parsers.arxiv_search import search_arxiv

        raw = search_arxiv(query, max_results=max_results)
        results = []
        for p in raw:
            pid = getattr(p, "uid", getattr(p, "id", ""))
            title = getattr(p, "title", "")
            authors = getattr(p, "authors", [])
            if isinstance(authors, str):
                authors = [a.strip() for a in authors.split(",") if a.strip()]
            abstract = getattr(p, "abstract", "")
            cat = getattr(p, "primary_category", "") or ""
            published = getattr(p, "published", "") or ""
            if pid:
                results.append({
                    "arxiv_id": pid,
                    "title": title,
                    "authors": authors,
                    "abstract": abstract,
                    "categories": [cat] if cat else [],
                    "published": published,
                })
        _SEARCH_CACHE[cache_key] = (now, results)
        return results
    except Exception as e:
        logger.warning(f"ArXiv search failed for '{query}': {e}")
        return []


def _get_topics_from_pool(capsules: List) -> List[str]:
    """Extract short search queries from Gene Pool capsules."""
    topics = set()
    # Collect all individual keywords across all capsules
    all_keywords = []
    for c in capsules:
        kws = c.trigger_keywords if hasattr(c, "trigger_keywords") else []
        all_keywords.extend(kws)

    # Count keyword frequency to find important terms
    from collections import Counter
    kw_counts = Counter(k.lower() for k in all_keywords if len(k) > 2)

    # Build short queries from top keywords
    top_kws = [kw for kw, _ in kw_counts.most_common(10)]

    # Also use short trigger_topic phrases (max 3 words)
    for c in capsules:
        t = c.trigger_topic if hasattr(c, "trigger_topic") else ""
        if t:
            words = t.split()
            # Use first 3-4 words as query
            short = " ".join(words[:4])
            if len(short) > 10:
                topics.add(short)

    # Add top keyword pairs
    for i in range(0, len(top_kws) - 1, 2):
        if i + 1 < len(top_kws):
            topics.add(f"{top_kws[i]} {top_kws[i+1]}")

    # Sort by estimated specificity (shortest first for broader search)
    return sorted(topics, key=len)[:10]


def scout(
    topic: str = "",
    max_papers_per_query: int = 10,
    max_results: int = 20,
    min_match_score: float = 0.15,
) -> List[ScoutResult]:
    """Scan ArXiv for papers matching Gene Pool interests.

    Args:
        topic: Optional specific topic. If empty, derived from Gene Pool.
        max_papers_per_query: Papers to fetch per ArXiv query.
        max_results: Max papers to return.
        min_match_score: Minimum match score to include.

    Returns:
        Ranked list of ScoutResult.
    """
    from llm.insight.tracker import EvolutionTracker

    tracker = EvolutionTracker()
    capsules = tracker._load_capsules()
    active = [c for c in capsules if c.status == "active"]

    if not active:
        return []

    # Determine topics to search
    topics = [topic] if topic else _get_topics_from_pool(active)
    if not topics:
        topics = ["machine learning"]  # fallback

    # Search ArXiv for each topic
    seen: set = set()
    all_papers: List[ScoutResult] = []

    for t in topics[:5]:  # max 5 topics
        papers = _search_arxiv(t, max_results=max_papers_per_query)
        for p in papers:
            pid = p["arxiv_id"]
            if pid in seen:
                continue
            seen.add(pid)

            # Score against all active capsules
            best_score = 0.0
            best_capsule = None
            best_reason = ""

            title = p["title"]
            abstract = p["abstract"]
            text = (title + " " + abstract).lower()

            for c in active:
                # Use trigger_match
                match = c.trigger_match(t, c.trigger_gap_type, c.trigger_keywords)

                # Also check keyword overlap in title/abstract
                kw_overlap = sum(1 for kw in c.trigger_keywords if kw.lower() in text)
                if kw_overlap > 0:
                    match = max(match, min(0.3 + kw_overlap * 0.15, 0.8))

                # Weight by credibility
                cred = getattr(c, "credibility_score", 0.5)
                weighted = match * (0.5 + 0.5 * cred)

                if weighted > best_score:
                    best_score = weighted
                    best_capsule = c
                    reason_parts = []
                    if match > 0:
                        reason_parts.append(f"trigger_match={match:.2f}")
                    if kw_overlap > 0:
                        reason_parts.append(f"keyword overlap={kw_overlap}")
                    if cred > 0:
                        reason_parts.append(f"capsule credibility={cred:.2f}")
                    best_reason = "; ".join(reason_parts) if reason_parts else "topic match"

            if best_score >= min_match_score and best_capsule:
                sr = ScoutResult(
                    arxiv_id=pid,
                    title=p["title"][:200],
                    authors=p["authors"][:5],
                    abstract=p["abstract"][:500],
                    categories=p["categories"],
                    published=p["published"][:10],
                    match_score=round(best_score, 3),
                    matched_capsule_id=best_capsule.capsule_id,
                    matched_gap_title=best_capsule.action_gap_title[:100],
                    matched_gap_type=best_capsule.action_gap_type,
                    credibility_of_match=round(
                        getattr(best_capsule, "credibility_score", 0.5), 3
                    ),
                    reason=best_reason,
                )
                all_papers.append(sr)

    # Sort by match_score descending
    all_papers.sort(key=lambda x: x.match_score, reverse=True)

    # Assign ranks
    for i, sr in enumerate(all_papers[:max_results]):
        sr.rank = i + 1

    return all_papers[:max_results]


def render_scout_results(results: List[ScoutResult]) -> str:
    """Render scout results as human-readable text."""
    if not results:
        return "  No matching papers found."

    lines = [f"\n  Found {len(results)} papers matching Gene Pool interests:\n"]
    for r in results:
        sev = r.match_score
        icon = "🟢" if sev >= 0.5 else "🟡" if sev >= 0.3 else "⚪"
        authors_str = ", ".join(r.authors[:2])
        lines.append(
            f"  {icon} [#{r.rank}] {r.title[:70]}"
        )
        lines.append(f"       {r.arxiv_id} · {r.published} · {authors_str}")
        lines.append(f"       Match: {r.match_score:.2f} ← {r.matched_gap_type}")
        lines.append(f"       Capsule: {r.matched_gap_title[:50]}")
        if r.reason:
            lines.append(f"       Why: {r.reason}")
        lines.append("")

    return "\n".join(lines)
