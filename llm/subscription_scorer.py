"""Subscription paper scorer: Smart relevance scoring for arXiv papers."""
from __future__ import annotations

import logging
import math
from datetime import datetime, timedelta
from typing import List, Optional, Dict, Any

logger = logging.getLogger(__name__)

# Scoring weights
_WEIGHT_GAP = 0.40       # Gap coverage
_WEIGHT_SEMANTIC = 0.30  # Semantic similarity
_WEIGHT_RECENCY = 0.20   # Freshness
_WEIGHT_COMPLEMENT = 0.10  # Complementary to existing papers


class SubscriptionScorer:
    """Smart paper scorer for subscription recommendations.

    Computes a composite score combining:
    - Gap coverage (40%): How well the paper addresses research gaps
    - Semantic similarity (30%): Relevance to user's reading history
    - Recency (20%): How recent/fresh the paper is
    - Complementarity (10%): How it complements existing paper collection
    """

    def __init__(self, db=None):
        self.db = db
        # Cache for user preference keywords
        self._user_keywords_cache: Optional[List[str]] = None

    def score_paper(self, paper: Dict[str, Any], subscription: Dict[str, Any]) -> Dict[str, float]:
        """Score a paper against a subscription.

        Args:
            paper: Paper dict with keys: title, abstract, published, arxiv_id
            subscription: Subscription dict with keys: topic, keywords, min_score

        Returns:
            Dict with composite score and individual components:
            {
                "score": 0.85,
                "gap_coverage": 0.90,
                "semantic_sim": 0.80,
                "recency": 0.95,
                "complement": 0.75,
            }
        """
        topic = subscription.get("topic", "")
        keywords = subscription.get("keywords", [])
        if isinstance(keywords, str):
            import json
            keywords = json.loads(keywords) if keywords else []

        abstract = paper.get("abstract", "") or ""
        title = paper.get("title", "")
        published = paper.get("published", "") or ""

        # Compute individual scores
        gap_score = self._compute_gap_coverage(title, abstract, topic, keywords)
        semantic_score = self._compute_semantic_similarity(title, abstract, topic)
        recency_score = self._compute_recency_score(published)
        complement_score = self._compute_complement_score(title, abstract)

        # Composite score
        composite = (
            gap_score * _WEIGHT_GAP +
            semantic_score * _WEIGHT_SEMANTIC +
            recency_score * _WEIGHT_RECENCY +
            complement_score * _WEIGHT_COMPLEMENT
        )

        return {
            "score": round(composite, 3),
            "gap_coverage": round(gap_score, 3),
            "semantic_sim": round(semantic_score, 3),
            "recency": round(recency_score, 3),
            "complement": round(complement_score, 3),
        }

    def _compute_gap_coverage(
        self,
        title: str,
        abstract: str,
        topic: str,
        keywords: List[str],
    ) -> float:
        """Compute gap coverage score based on keyword overlap with topic.

        Heuristic: Paper scores high if it mentions gap-related terms
        or addresses the topic directly.
        """
        text = f"{title} {abstract}".lower()
        topic_words = set(topic.lower().split())
        matched = 0

        for word in topic_words:
            if word in text:
                matched += 1

        topic_score = matched / max(len(topic_words), 1)

        # Keyword matching bonus
        kw_score = 0.0
        if keywords:
            kw_matches = sum(1 for kw in keywords if kw.lower() in text)
            kw_score = kw_matches / len(keywords)

        # Combine: 60% topic, 40% keywords
        return 0.6 * topic_score + 0.4 * kw_score

    def _compute_semantic_similarity(
        self,
        title: str,
        abstract: str,
        topic: str,
    ) -> float:
        """Compute semantic similarity to user's research interest.

        Uses lightweight TF-IDF style matching against user's
        reading history keywords if available.
        """
        text = f"{title} {abstract}".lower()
        topic_lower = topic.lower()

        # Direct topic match
        if topic_lower in text:
            return 0.9

        # Count overlapping words
        topic_words = set(topic_lower.split())
        text_words = set(text.split())
        overlap = len(topic_words & text_words)

        if overlap > 0:
            # More overlap = higher score
            return min(0.8, overlap * 0.25)

        # Penalize papers with opposite signals
        negative_terms = {"not ", "doesn't ", "doesn't ", "fail", "cannot "}
        neg_hits = sum(1 for t in negative_terms if t in text)
        if neg_hits > 2:
            return 0.3

        return 0.4  # Baseline for topic-related papers

    def _compute_recency_score(self, published: str) -> float:
        """Compute recency score based on publication date.

        Fresh papers get higher scores:
        - Within 7 days: 1.0
        - Within 30 days: 0.9
        - Within 90 days: 0.7
        - Within 180 days: 0.5
        - Within 365 days: 0.3
        - Older: 0.1
        """
        if not published:
            return 0.5  # Unknown date gets middle score

        try:
            pub_date = datetime.fromisoformat(published[:10])
        except (ValueError, TypeError):
            return 0.5

        days_old = (datetime.now() - pub_date).days
        if days_old < 0:
            days_old = 0  # Future dates treat as today

        if days_old <= 7:
            return 1.0
        elif days_old <= 30:
            return 0.9
        elif days_dir := days_old <= 90:
            return 0.7
        elif days_old <= 180:
            return 0.5
        elif days_old <= 365:
            return 0.3
        else:
            return 0.1

    def _compute_complement_score(self, title: str, abstract: str) -> float:
        """Compute complementarity to existing paper collection.

        Papers that introduce new methods, datasets, or frameworks
        score higher as they complement existing knowledge.
        """
        text = f"{title} {abstract}".lower()

        # Signals of novel contributions
        novel_signals = [
            "introduce", "propose", "new method", "novel",
            "benchmark", "dataset", "framework", "architecture",
            "improve", "state-of-the-art", "sota",
        ]

        novel_hits = sum(1 for sig in novel_signals if sig in text)
        if novel_hits >= 2:
            return 0.9
        elif novel_hits == 1:
            return 0.7

        # Survey/review papers complement individual studies
        survey_signals = ["survey", "review", "overview", "comprehensive"]
        if any(sig in text for sig in survey_signals):
            return 0.6

        return 0.5  # Baseline

    def batch_score(
        self,
        papers: List[Dict[str, Any]],
        subscription: Dict[str, Any],
    ) -> List[Dict[str, Any]]:
        """Score multiple papers and filter by minimum threshold.

        Args:
            papers: List of paper dicts
            subscription: Subscription dict with min_score

        Returns:
            Papers with scores, filtered and sorted by score descending
        """
        min_score = subscription.get("min_score", 0.5)
        scored = []

        for paper in papers:
            scores = self.score_paper(paper, subscription)
            if scores["score"] >= min_score:
                scored.append({
                    **paper,
                    **scores,
                })

        # Sort by score descending
        scored.sort(key=lambda x: x["score"], reverse=True)
        return scored
