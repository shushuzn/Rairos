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

# Cache for search results
_SEARCH_CACHE: Dict[str, Tuple[float, List[Dict]]] = {}
_CACHE_TTL = 3600  # 1 hour

# News RSS feeds — free, no API key needed
NEWS_FEEDS = [
    ("Reuters World", "https://www.reuters.com/world/rss"),
    ("Reuters Business", "https://www.reuters.com/business/rss"),
    ("BBC World", "https://feeds.bbci.co.uk/news/world/rss.xml"),
    ("BBC Technology", "https://feeds.bbci.co.uk/news/technology/rss.xml"),
    ("Google News Top", "https://news.google.com/rss?hl=en-US&gl=US&ceid=US:en"),
    (
        "Google News Science",
        "https://news.google.com/rss/topics/CAAqJggKIiBDQkFTRWdvSUwyMHZNRFp0Y1RjU0FtVnVHZ0pWVXlnQVAB",
    ),
    ("Hacker News", "https://hnrss.org/frontpage"),
]


@dataclass
class ScoutResult:
    """A paper found by the scout, scored against Gene Pool."""

    arxiv_id: str
    title: str
    authors: List[str]
    abstract: str
    categories: List[str]
    published: str
    source: str = "arxiv"  # "arxiv" or news source name

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
                results.append(
                    {
                        "arxiv_id": pid,
                        "title": title,
                        "authors": authors,
                        "abstract": abstract,
                        "categories": [cat] if cat else [],
                        "published": published,
                    }
                )
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
            topics.add(f"{top_kws[i]} {top_kws[i + 1]}")

    # Sort by estimated specificity (shortest first for broader search)
    return sorted(topics, key=len)[:10]


def _fetch_rss(feed_url: str, feed_name: str, max_items: int = 10) -> List[Dict]:
    """Fetch articles from an RSS feed."""
    import feedparser

    try:
        feed = feedparser.parse(feed_url)
        articles = []
        for entry in feed.entries[:max_items]:
            title = entry.get("title", "")
            link = entry.get("link", "")
            summary = entry.get("summary", "") or entry.get("description", "") or ""
            published = entry.get("published", "") or entry.get("updated", "") or ""
            authors = []
            if hasattr(entry, "authors") and entry.authors:
                authors = [a.get("name", "") for a in entry.authors]
            articles.append(
                {
                    "arxiv_id": f"news_{feed_name}_{hash(link) % 10**8}",
                    "title": title,
                    "authors": authors,
                    "abstract": summary[:500],
                    "categories": [feed_name],
                    "published": published[:10],
                    "source": feed_name,
                    "url": link,
                }
            )
        return articles
    except Exception as e:
        logger.warning(f"RSS feed '{feed_name}' failed: {e}")
        return []


def _score_article(p: Dict, active: List, topic: str) -> Optional[ScoutResult]:
    """Score a single article/paper against all active Gene Pool capsules."""
    pid = p["arxiv_id"]
    title = p["title"]
    abstract = p["abstract"]
    text = (title + " " + abstract).lower()

    best_score = 0.0
    best_capsule = None
    best_reason = ""

    for c in active:
        match = c.trigger_match(topic, c.trigger_gap_type, c.trigger_keywords)
        kw_overlap = sum(1 for kw in c.trigger_keywords if kw.lower() in text)
        if kw_overlap > 0:
            match = max(match, min(0.3 + kw_overlap * 0.15, 0.8))
        cred = getattr(c, "credibility_score", 0.5)
        weighted = match * (0.5 + 0.5 * cred)

        if weighted > best_score:
            best_score = weighted
            best_capsule = c
            rp = []
            if match > 0:
                rp.append(f"trigger_match={match:.2f}")
            if kw_overlap > 0:
                rp.append(f"keyword overlap={kw_overlap}")
            if cred > 0:
                rp.append(f"capsule credibility={cred:.2f}")
            best_reason = "; ".join(rp) if rp else "topic match"

    if best_score > 0 and best_capsule:
        return ScoutResult(
            arxiv_id=pid,
            title=title[:200],
            authors=p.get("authors", [])[:5],
            abstract=abstract[:500],
            categories=p.get("categories", []),
            published=p.get("published", "")[:10],
            source=p.get("source", "arxiv"),
            match_score=round(best_score, 3),
            matched_capsule_id=best_capsule.capsule_id,
            matched_gap_title=best_capsule.action_gap_title[:100],
            matched_gap_type=best_capsule.action_gap_type,
            credibility_of_match=round(getattr(best_capsule, "credibility_score", 0.5), 3),
            reason=best_reason,
        )
    return None


def scout(
    topic: str = "",
    sources: str = "arxiv",  # "arxiv", "news", or "all"
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

    seen: set = set()
    all_papers: List[ScoutResult] = []

    if sources in ("news", "all"):
        for feed_name, feed_url in NEWS_FEEDS:
            articles = _fetch_rss(feed_url, feed_name, max_items=5)
            for p in articles:
                pid = p["arxiv_id"]
                if pid in seen:
                    continue
                seen.add(pid)
                all_papers.append(_score_article(p, active, topic))

    # Search ArXiv for each topic
    if sources in ("arxiv", "all"):
        for t in topics[:5]:
            papers = _search_arxiv(t, max_results=max_papers_per_query)
            for p in papers:
                pid = p["arxiv_id"]
                if pid in seen:
                    continue
                seen.add(pid)
                sr = _score_article(p, active, topic if topic else t)
                if sr and sr.match_score >= min_match_score:
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

    lines = [f"\n  Found {len(results)} items matching Gene Pool interests:\n"]
    for r in results:
        sev = r.match_score
        icon = "🟢" if sev >= 0.5 else "🟡" if sev >= 0.3 else "⚪"
        authors_str = ", ".join(r.authors[:2])
        source_tag = f"[{r.source}]" if r.source != "arxiv" else ""
        lines.append(f"  {icon} {source_tag} [#{r.rank}] {r.title[:70]}")
        lines.append(f"       {r.published} · {authors_str}")
        lines.append(f"       Match: {r.match_score:.2f} ← {r.matched_gap_type}")
        lines.append(f"       Capsule: {r.matched_gap_title[:50]}")
        if r.reason:
            lines.append(f"       Why: {r.reason}")
        lines.append("")

    return "\n".join(lines)
