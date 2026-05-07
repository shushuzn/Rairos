"""Event processing pipeline: news → capsules → related papers → insights."""

from __future__ import annotations

import logging
import time
from datetime import datetime
from typing import Any, Dict, List, Optional

from llm.mcp_jin10 import Jin10Client
from llm.insight.tracker import EvolutionTracker
from parsers.arxiv_search import search_arxiv
from core import Paper

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
                "paper_id": getattr(p, "uid", getattr(p, "id", "?")),
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
        if isinstance(raw, dict):
            inner = raw.get("data", raw)
            if isinstance(inner, dict):
                items = inner.get("items", [])
            elif isinstance(inner, list):
                items = inner
        elif isinstance(raw, list):
            items = raw
    return items[:limit]


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
    """Find academic papers related to the event topic.

    Multi-backend search: tries arXiv first, then CrossRef, then Semantic Scholar.
    Each backend is isolated so a failure (including 429 rate-limit) won't
    block the whole pipeline.
    """
    papers = _try_search_arxiv(topic, limit)
    if papers:
        return papers

    papers = _try_search_crossref(topic, limit)
    if papers:
        return papers

    papers = _try_search_semantic_scholar(topic, limit)
    return papers


def _try_search_arxiv(topic: str, limit: int) -> List:
    """Search arXiv with exponential backoff on 429."""
    import time as _time

    for attempt in range(3):
        try:
            return search_arxiv(topic, max_results=limit)
        except Exception as e:
            err_str = str(e)
            is_429 = "429" in err_str
            if is_429 and attempt < 2:
                delay = (attempt + 1) * 5
                logger.warning(
                    f"arXiv 429 rate-limited (attempt {attempt+1}/3), "
                    f"retrying in {delay}s..."
                )
                _time.sleep(delay)
                continue
            logger.warning(f"ArXiv search failed: {e}")
            return []
    return []


def _try_search_crossref(topic: str, limit: int) -> List:
    """Search CrossRef API as a fallback for arXiv (covers all disciplines).

    Returns Paper objects compatible with the rest of the pipeline.
    """
    import json as _json
    import urllib.parse as _urlparse
    import urllib.request as _urlreq

    query = _urlparse.quote(topic)
    url = (
        f"https://api.crossref.org/works?query={query}&rows={limit}"
        f"&select=DOI,title,author,created,abstract,type"
    )

    try:
        req = _urlreq.Request(
            url,
            headers={
                "User-Agent": "Rairos/1.0 (mailto:rairos@example.com)",
                "Accept": "application/json",
            },
        )
        resp = _urlreq.urlopen(req, timeout=30)
        data = _json.loads(resp.read().decode())
        items = data.get("message", {}).get("items", [])
    except Exception as e:
        logger.warning(f"CrossRef search failed: {e}")
        return []

    papers = []
    for item in items:
        doi = item.get("DOI", "")
        title = (item.get("title") or [""])[0]
        if not title:
            continue

        authors = []
        for a in item.get("author", []):
            given = a.get("given", "")
            family = a.get("family", "")
            name = f"{given} {family}".strip()
            if name:
                authors.append(name)

        created = item.get("created", {}).get("date-parts", [[None]])[0]
        year = str(created[0]) if created and created[0] else ""
        abstract = item.get("abstract", "") or ""

        paper = Paper(
            source="doi",
            uid=doi,
            title=title.replace("\n", " ").strip(),
            authors=authors,
            abstract=abstract.replace("\n", " ").strip()[:500],
            published=year,
            updated="",
            abs_url=f"https://doi.org/{doi}" if doi else "",
            pdf_url="",
            doi=doi,
        )
        papers.append(paper)

    return papers[:limit]


def _try_search_semantic_scholar(topic: str, limit: int) -> List:
    """Search Semantic Scholar API as a second fallback.

    Covers computer science, biomed, and other S2-indexed disciplines.
    """
    import json as _json
    import urllib.parse as _urlparse
    import urllib.request as _urlreq

    query = _urlparse.quote(topic)
    url = (
        f"https://api.semanticscholar.org/graph/v1/paper/search"
        f"?query={query}&limit={limit}"
        f"&fields=title,year,authors,externalIds,abstract"
    )

    try:
        req = _urlreq.Request(
            url,
            headers={
                "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) "
                "AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0",
            },
        )
        resp = _urlreq.urlopen(req, timeout=30)
        data = _json.loads(resp.read().decode())
        items = data.get("data", [])
    except Exception as e:
        logger.warning(f"Semantic Scholar search failed: {e}")
        return []

    papers = []
    for item in items:
        title = item.get("title", "")
        if not title:
            continue

        year = str(item.get("year") or "")
        ext_ids = item.get("externalIds", {}) or {}

        authors = []
        for a in item.get("authors", []):
            name = a.get("name", "")
            if name:
                authors.append(name)

        abstract = item.get("abstract", "") or ""
        doi = ext_ids.get("DOI", "")
        arxiv_id = ext_ids.get("ArXiv", "")
        corpus_id = ext_ids.get("CorpusId", "")
        uid = doi or arxiv_id or f"s2-{corpus_id}" if corpus_id else title[:40]

        paper = Paper(
            source="doi" if doi else ("arxiv" if arxiv_id else "semantic-scholar"),
            uid=uid,
            title=title.replace("\n", " ").strip(),
            authors=authors,
            abstract=abstract.replace("\n", " ").strip()[:500],
            published=year,
            updated="",
            abs_url=f"https://doi.org/{doi}" if doi else (
                f"https://arxiv.org/abs/{arxiv_id}" if arxiv_id else ""
            ),
            pdf_url="",
            doi=doi,
        )
        papers.append(paper)

    return papers[:limit]


def render_event_report(result: Dict[str, Any]) -> str:
    """Render event processing result as human-readable text."""
    if "error" in result:
        return f"  Error: {result['error']}"

    lines = [
        "\n  ⚡ Event Processed",
        f"  ID: {result.get('event_id', '?')}",
        f"  Time: {result.get('timestamp', '?')[:19]}",
        f"  Keywords: {', '.join(result.get('keywords', []))}",
        "",
        "  Capsule encoded:",
        f"    {result.get('capsule_title', '')[:80]}",
        "",
        f"  Related academic papers ({len(result.get('related_papers', []))}):",
    ]
    for ref in result.get("related_papers", []):
        lines.append(f"    {ref.get('paper_id', '?')} relevance={ref.get('relevance', 0):.2f}")
        lines.append(f"    {ref.get('title', '')[:70]}")
    lines.append("")
    return "\n".join(lines)
