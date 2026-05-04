"""Event processing pipeline: news → capsules → related papers → insights."""

from __future__ import annotations

import logging
import time
from datetime import datetime
from typing import Any, Dict, List, Optional

from llm.mcp_jin10 import Jin10Client
from llm.insight.tracker import EvolutionTracker
from parsers.arxiv_search import search_arxiv

logger = logging.getLogger(__name__)

# Keywords that trigger auto-capsule creation
HIGH_IMPACT_KEYWORDS = [
    "导弹", "袭击", "无人机", "石油", "霍尔木兹", "制裁",
    "missile", "drone", "oil", "sanctions", "Strait of Hormuz",
    "利率", "通胀", "非农", "美联储", "加息",
    "rate", "inflation", "Fed", "加息",
]


def process_event(keyword: str = "", max_news: int = 5, max_papers: int = 3) -> Dict[str, Any]:
    """Process a news event: fetch news → score → encode as capsule → find related papers.

    Args:
        keyword: Event keyword (e.g. "伊朗", "富查伊拉")
        max_news: Max news items to process
        max_papers: Max related papers to find

    Returns:
        Dict with event_id, capsule_id, related_papers, summary
    """
    client = Jin10Client()
    client.ensure_init()
    tracker = EvolutionTracker()

    # 1. Fetch relevant news
    news_items = _fetch_event_news(client, keyword, max_news)
    if not news_items:
        return {"error": f"No news found for '{keyword}'"}

    # 2. Create event summary
    summary = _build_summary(news_items, keyword)

    # 3. Extract keywords and encode as capsule
    gap_type = _infer_gap_type(summary)
    topic = keyword or summary["primary_keyword"]
    title = summary["capsule_title"]

    capsule = tracker.encode_capsule(
        topic=topic,
        gap_type=gap_type,
        gap_title=title,
        success_score=0.75,
        source_arxiv_category="cs.GL",
    )

    # 4. Find related academic papers
    related_papers = _find_related_papers(topic, max_papers)

    # 5. Cross-reference: score papers against the news capsule
    cross_refs = []
    for p in related_papers:
        match = capsule.trigger_match(
            topic, "evaluation_gap",
            [kw.lower() for kw in summary["keywords"]]
        )
        if match > 0.1:
            cross_refs.append({
                "paper_id": p.get("uid", p.get("id", "?")),
                "title": getattr(p, "title", str(p))[:100],
                "relevance": round(match, 3),
            })

    return {
        "event_id": capsule.capsule_id,
        "capsule_id": capsule.capsule_id,
        "capsule_title": capsule.action_gap_title,
        "timestamp": summary["timestamp"],
        "keywords": summary["keywords"],
        "news_count": len(news_items),
        "related_papers": cross_refs,
        "summary": summary["brief"],
    }


def _fetch_event_news(client: Jin10Client, keyword: str, limit: int) -> List[Dict]:
    """Fetch news items related to an event keyword."""
    items = []
    if keyword:
        raw = client.search_flash(keyword)
        data = raw.get("data", raw) if isinstance(raw, dict) else {"items": raw if isinstance(raw, list) else []}
        items = data.get("items", [])[:limit]
    return items


def _build_summary(news_items: List[Dict], keyword: str) -> Dict:
    """Extract key info from news items and build a summary."""
    all_text = " ".join([
        item.get("content", "") if isinstance(item, dict) else str(item)
        for item in news_items
    ])

    # Extract keywords (simple frequency-based)
    words = all_text.replace("\n", " ").split()
    from collections import Counter
    word_counts = Counter(w for w in words if len(w) > 1)
    top_kws = [w for w, _ in word_counts.most_common(10)]

    # First few items for capsule title
    first = news_items[0] if news_items else {}
    first_text = first.get("content", "") if isinstance(first, dict) else str(first)

    return {
        "primary_keyword": keyword or top_kws[0] if top_kws else "event",
        "keywords": top_kws[:8] if top_kws else [keyword],
        "capsule_title": first_text[:120] if first_text else f"Event: {keyword}",
        "brief": all_text[:300],
        "timestamp": datetime.now().isoformat(),
    }


def _infer_gap_type(summary: Dict) -> str:
    """Infer the gap type from event content."""
    text = summary["brief"].lower()
    if any(w in text for w in ["导弹", "袭击", "drone", "missile", "军事"]):
        return "scalability_issue"
    if any(w in text for w in ["石油", "油", "oil", "能源"]):
        return "evaluation_gap"
    if any(w in text for w in ["利率", "通胀", "加息", "rate"]):
        return "method_limitation"
    return "unexplored_application"


def _find_related_papers(topic: str, limit: int) -> List:
    """Find academic papers related to the event topic."""
    try:
        return search_arxiv(topic, max_results=limit)
    except Exception as e:
        logger.warning(f"ArXiv search failed: {e}")
        return []


def render_event_report(result: Dict[str, Any]) -> str:
    """Render event processing result as human-readable text."""
    if "error" in result:
        return f"  Error: {result['error']}"

    lines = [
        f"\n  ⚡ Event Processed",
        f"  ID: {result.get('event_id', '?')}",
        f"  Time: {result.get('timestamp', '?')[:19]}",
        f"  Keywords: {', '.join(result.get('keywords', []))}",
        f"",
        f"  Capsule encoded:",
        f"    {result.get('capsule_title', '')[:80]}",
        f"",
        f"  Related academic papers ({len(result.get('related_papers', []))}):",
    ]
    for ref in result.get("related_papers", []):
        lines.append(f"    {ref.get('paper_id', '?')} relevance={ref.get('relevance', 0):.2f}")
        lines.append(f"    {ref.get('title', '')[:70]}")
    lines.append("")
    return "\n".join(lines)
