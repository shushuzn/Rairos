"""Topic Discovery — intelligently suggest new arXiv subscription topics from gaps and papers.

Given recent research output (gaps, papers), this module identifies research areas
that are active but not yet subscribed to, so the system can proactively expand
its monitoring boundary.

Discovery strategies
────────────────────
1. Gap-cluster based:  hot clusters with many high-novelty gaps → suggest subscription
2. Gap-type trending:  rising gap types (METHOD_LIMITATION growing) → suggest subscription
3. Paper keyword extraction:  frequent untracked keywords in recent papers
4. Gap→subscription mapping:  gap_type → topic keyword suggestions

Integration
───────────
    discoverer = TopicDiscoverer(db=db)
    suggestions = discoverer.suggest_new_topics(
        recent_gaps=[...], recent_papers=[...], max_suggestions=5
    )
    for s in suggestions:
        print(f"  [{s.source}] {s.topic} (confidence={s.confidence:.2f})")
        print(f"    reason: {s.reason}")
"""

from __future__ import annotations

import re
import uuid
from collections import Counter
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Set, cast

import logging

logger = logging.getLogger(__name__)


# ─── Dataclasses ─────────────────────────────────────────────────────────────


@dataclass
class TopicSuggestion:
    """A suggested new subscription topic."""

    topic: str               # e.g. "scaling laws for reasoning"
    source: str             # 'gap_cluster' | 'gap_type_trend' | 'paper_keyword' | 'gap_subscription_map'
    confidence: float       # 0.0–1.0
    reason: str             # human-readable explanation
    gap_type: str = ""      # associated gap type if applicable
    keywords: List[str] = field(default_factory=list)
    cluster_id: str = ""     # if from gap cluster
    novelty_score: float = 0.0  # average novelty of source gaps


# ─── Keyword extraction ───────────────────────────────────────────────────────


def _extract_keywords_from_text(texts: List[str], top_n: int = 10) -> List[tuple[str, int]]:
    """Extract frequent meaningful phrases from a list of text strings.

    Returns [(keyword, frequency), ...] sorted by frequency desc.
    Filters out generic academic terms.
    """
    generic = {
        "paper", "work", "method", "approach", "result", "experiment",
        "performance", "show", "propose", "state-of-the-art", "sota",
        "baseline", "existing", "current", "recent", "new", "novel",
        "task", "problem", "model", "data", "dataset", "training",
        "evaluation", "benchmark", "learning", "system", "framework",
        "the", "and", "for", "with", "from", "that", "this", "are",
    }

    # Split into tokens
    token_counter: Counter = Counter()
    for text in texts:
        tokens = re.findall(r"[a-z][a-z0-9-]*[a-z]", text.lower())
        for token in tokens:
            if len(token) >= 4 and token not in generic:
                token_counter[token] += 1

    return token_counter.most_common(top_n * 2)


def _phrase_suggestion_from_keywords(keywords: List[tuple[str, int]]) -> str:
    """Turn a list of (keyword, freq) into a coherent topic phrase."""
    if not keywords:
        return ""
    top = [k for k, _ in keywords[:4]]
    return " + ".join(top)


# ─── Gap-cluster based discovery ──────────────────────────────────────────────


def _from_gap_clusters(
    clusters: List[Any], all_gaps: List[Any], thresholdNovelty: float = 0.4
) -> List[TopicSuggestion]:
    """Suggest topics from hot gap clusters.

    A hot cluster (3+ gaps, high avg novelty) → suggests a subscription topic.
    """
    suggestions: List[TopicSuggestion] = []

    for cluster in clusters:
        gaps = getattr(cluster, "gaps", []) or []
        if len(gaps) < 2:
            continue

        avg_novelty = sum(
            getattr(g, "novelty_score", 0.0) or 0.0 for g in gaps
        ) / len(gaps)

        if avg_novelty < thresholdNovelty:
            continue

        gap_type = getattr(cluster, "gap_type", "unknown")
        cluster_id = getattr(cluster, "cluster_id", "")

        # Build topic from cluster keywords
        titles = [getattr(g, "title", "") or getattr(g, "gap_title", "") or "" for g in gaps]
        keywords = _extract_keywords_from_text(titles, top_n=5)
        topic = _phrase_suggestion_from_keywords(keywords)

        if not topic:
            topic = f"{gap_type}: {titles[0][:60]}"

        # Extract keywords for subscription
        subscription_keywords = [k for k, _ in keywords[:5]]

        suggestions.append(TopicSuggestion(
            topic=topic,
            source="gap_cluster",
            confidence=min(1.0, avg_novelty * 1.2),
            reason=(
                f"Hot cluster: {len(gaps)} gaps (avg novelty={avg_novelty:.2f}). "
                f"Top: {titles[0][:60]}"
            ),
            gap_type=gap_type,
            keywords=subscription_keywords,
            cluster_id=cluster_id,
            novelty_score=avg_novelty,
        ))

    return suggestions


# ─── Gap-type trend based discovery ──────────────────────────────────────────


def _from_gap_type_trends(
    trends: Dict[str, str], gaps: List[Any], threshold_count: int = 3
) -> List[TopicSuggestion]:
    """Suggest topics from rising/trending gap types.

    If a gap type is 'rising', suggest a subscription around that type's keywords.
    """
    suggestions: List[TopicSuggestion] = []

    rising_types = [t for t, trend in trends.items() if trend == "rising"]
    if not rising_types:
        return suggestions

    for gap_type in rising_types:
        type_gaps = [g for g in gaps if (
            getattr(g, "gap_type", "") == gap_type or
            (hasattr(g.gap_type, "value") and g.gap_type.value == gap_type)
        )]

        if len(type_gaps) < threshold_count:
            continue

        titles = [getattr(g, "title", "") or "" for g in type_gaps]
        keywords = _extract_keywords_from_text(titles, top_n=5)
        topic = _phrase_suggestion_from_keywords(keywords) or f"rising {gap_type} research"

        suggestions.append(TopicSuggestion(
            topic=topic,
            source="gap_type_trend",
            confidence=0.6,
            reason=f"Gap type '{gap_type}' is trending ({len(type_gaps)} recent gaps)",
            gap_type=gap_type,
            keywords=[k for k, _ in keywords[:4]],
            novelty_score=sum(getattr(g, "novelty_score", 0.0) or 0.0 for g in type_gaps) / len(type_gaps),
        ))

    return suggestions


# ─── Paper keyword based discovery ───────────────────────────────────────────


def _from_paper_keywords(
    papers: List[Dict[str, Any]], existing_topics: Set[str], threshold_freq: int = 3
) -> List[TopicSuggestion]:
    """Suggest topics from keywords that appear frequently in recent papers but aren't subscribed.

    Looks at paper titles + abstracts for frequent terms that match no existing topic.
    """
    suggestions: List[TopicSuggestion] = []

    texts = []
    for p in papers:
        title = p.get("title", "") or ""
        abstract = p.get("abstract", "") or ""
        texts.append(title + " " + abstract[:300])

    keywords = _extract_keywords_from_text(texts, top_n=20)

    # Find the most frequent keyword phrases that aren't covered by existing topics
    # existing_lower = {t.lower() for t in existing_topics}  # reserved
    for keyword, freq in keywords:
        if freq < threshold_freq:
            break  # sorted by freq desc

        # Check if existing topics cover this keyword
        covered = any(keyword in t.lower() for t in existing_topics)
        if covered:
            continue

        suggestions.append(TopicSuggestion(
            topic=f"{keyword} research",
            source="paper_keyword",
            confidence=min(0.9, freq / 10.0 + 0.3),
            reason=f"'{keyword}' appears in {freq} recent papers but has no subscription",
            keywords=[keyword],
        ))

    return suggestions[:5]


# ─── Gap→subscription mapping ────────────────────────────────────────────────


# Mapping from gap types to suggested subscription topic keywords
_GAP_TYPE_TOPIC_MAP: Dict[str, List[str]] = {
    "method_limitation": ["method limitation", "inefficiency", "scalability"],
    "unexplored_application": ["application", "domain", "new setting"],
    "evaluation_gap": ["benchmark", "evaluation", "measurement"],
    "scalability_issue": ["scaling", "large-scale", "efficiency"],
    "theoretical_gap": ["theory", "analysis", "foundation"],
    "dataset_gap": ["dataset", "data", "corpus"],
    "generalization_gap": ["generalization", "out-of-distribution", "robustness"],
    "contradiction": ["contradiction", "rebuttal", "counter-example"],
    "capability_missing": ["capability", "ability", "missing"],
    "unknown": ["open problem", "underexplored"],
}

_GAP_TYPE_TOPIC_MAP["capability"] = _GAP_TYPE_TOPIC_MAP["capability_missing"]
_GAP_TYPE_TOPIC_MAP["quality"] = _GAP_TYPE_TOPIC_MAP["method_limitation"]
_GAP_TYPE_TOPIC_MAP["missing"] = _GAP_TYPE_TOPIC_MAP["unexplored_application"]


def _from_gap_subscription_map(
    gaps: List[Any], existing_topics: Set[str], min_gaps_per_type: int = 2
) -> List[TopicSuggestion]:
    """Use gap_type→topic keyword mapping to suggest new subscriptions.

    For each gap type that has N+ gaps but no covering subscription, suggest a topic.
    """
    suggestions: List[TopicSuggestion] = []

    # Count gaps per gap type
    type_counter: Counter = Counter()
    type_gaps: Dict[str, List[Any]] = {}
    for g in gaps:
        gt = getattr(g, "gap_type", "unknown")
        if hasattr(gt, "value"):
            gt = gt.value
        gt = str(gt).lower()
        type_counter[gt] += 1
        type_gaps.setdefault(gt, []).append(g)

    for gap_type, count in type_counter.items():
        if count < min_gaps_per_type:
            continue

        mapped_keywords = _GAP_TYPE_TOPIC_MAP.get(gap_type, [])
        if not mapped_keywords:
            continue

        topic = f"{gap_type.replace('_', ' ')}: {mapped_keywords[0]}"

        # Check if this topic is already covered
        covered = any(
            gap_type.replace("_", " ") in t.lower() or mapped_keywords[0] in t.lower()
            for t in existing_topics
        )
        if covered:
            continue

        avg_novelty = sum(
            getattr(g, "novelty_score", 0.0) or 0.0 for g in type_gaps[gap_type]
        ) / len(type_gaps[gap_type])

        suggestions.append(TopicSuggestion(
            topic=topic,
            source="gap_subscription_map",
            confidence=min(0.8, avg_novelty + 0.2),
            reason=(
                f"Gap type '{gap_type}' has {count} gaps but no subscription. "
                f"Mapped from known gap-type patterns."
            ),
            gap_type=gap_type,
            keywords=mapped_keywords[:3],
            novelty_score=avg_novelty,
        ))

    return suggestions


# ─── Main TopicDiscoverer ────────────────────────────────────────────────────


class TopicDiscoverer:
    """Suggest new arXiv subscription topics from recent gaps and papers.

    Usage:
        discoverer = TopicDiscoverer(db=db)
        suggestions = discoverer.suggest_new_topics(recent_gaps=gaps, recent_papers=papers)
    """

    def __init__(self, db: Optional[Any] = None):
        self.db = db

    def suggest_new_topics(
        self,
        recent_gaps: Optional[List[Any]] = None,
        recent_papers: Optional[List[Dict[str, Any]]] = None,
        gap_clusters: Optional[List[Any]] = None,
        gap_trends: Optional[Dict[str, str]] = None,
        max_suggestions: int = 5,
    ) -> List[TopicSuggestion]:
        """Return topic suggestions ranked by confidence.

        Args:
            recent_gaps: List of ResearchGapV2 or similar gap objects
            recent_papers: List of paper dicts with 'title' and 'abstract'
            gap_clusters: List of GapCluster namedtuples from GapClusterer
            gap_trends: Dict[gap_type -> 'rising'|'stable'|'declining']
            max_suggestions: max suggestions to return
        """
        recent_gaps = recent_gaps or []
        recent_papers = recent_papers or []
        gap_clusters = gap_clusters or []
        gap_trends = gap_trends or {}

        # Get existing subscriptions to avoid duplicates
        existing_topics: Set[str] = set()
        if self.db:
            try:
                subs = self.db.list_arxiv_subscriptions()
                existing_topics = {str(s.get("topic", "")) for s in subs}
            except Exception as e:
                logger.warning(f"Could not load existing subscriptions: {e}")

        all_suggestions: List[TopicSuggestion] = []

        # Strategy 1: gap clusters (highest priority)
        if gap_clusters:
            all_suggestions.extend(_from_gap_clusters(gap_clusters, recent_gaps))

        # Strategy 2: gap-type trends
        if gap_trends:
            all_suggestions.extend(_from_gap_type_trends(gap_trends, recent_gaps))

        # Strategy 3: gap→subscription mapping
        all_suggestions.extend(_from_gap_subscription_map(recent_gaps, existing_topics))

        # Strategy 4: paper keywords
        if recent_papers:
            all_suggestions.extend(_from_paper_keywords(recent_papers, existing_topics))

        # Deduplicate by topic (keep highest confidence)
        seen_topics: Dict[str, TopicSuggestion] = {}
        for s in all_suggestions:
            key = s.topic.lower()
            if key not in seen_topics or s.confidence > seen_topics[key].confidence:
                seen_topics[key] = s

        # Sort by confidence desc
        sorted_suggestions = sorted(
            seen_topics.values(), key=lambda x: x.confidence, reverse=True
        )

        return sorted_suggestions[:max_suggestions]

    def apply_suggestion(self, suggestion: TopicSuggestion) -> Optional[int]:
        """Create a new arXiv subscription from a TopicSuggestion.

        Returns the new subscription ID, or None if db is not available.
        """
        if not self.db:
            logger.warning("No db provided — cannot apply suggestion")
            return None

        try:
            keywords_str = ", ".join(suggestion.keywords[:5])
            sub_id = cast(int, self.db.add_arxiv_subscription(
                topic=suggestion.topic,
                categories="",
                keywords=keywords_str,
            ))
            logger.info(f"Created subscription [{sub_id}]: {suggestion.topic}")
            return sub_id
        except Exception as e:
            logger.error(f"Failed to create subscription: {e}")
            return None
