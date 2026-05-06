"""Subscription monitor: Checks arXiv for new papers matching subscriptions."""

from __future__ import annotations

import json
import logging
from typing import Dict, List, Any, Optional

from llm.subscription_scorer import SubscriptionScorer

logger = logging.getLogger(__name__)


class SubscriptionMonitor:
    """Monitors arXiv for papers matching subscriptions.

    Uses the arXiv API to search for papers since the last check
    and scores them against the subscription criteria.
    """

    def __init__(self, db, scorer: Optional[SubscriptionScorer] = None):
        self.db = db
        self.scorer = scorer or SubscriptionScorer(db)

    def check_subscription(self, sub_id: str) -> List[Dict[str, Any]]:
        """Check a single subscription for new papers.

        Args:
            sub_id: Subscription ID to check

        Returns:
            List of paper dicts that scored above the threshold
        """
        sub = self.db.get_arxiv_subscription(sub_id)
        if not sub:
            logger.warning(f"Subscription {sub_id} not found")
            return []

        if not sub.get("enabled", True):
            logger.info(f"Subscription {sub_id} is disabled, skipping")
            return []

        # Build search query from topic and keywords
        topic = sub.get("topic", "")
        keywords = sub.get("keywords", [])
        if isinstance(keywords, str):
            keywords = json.loads(keywords) if keywords else []
        keywords = keywords or []

        query = self._build_query(topic, keywords)
        max_results = sub.get("max_results", 10)

        # Search arXiv
        papers = self._search_arxiv(query, max_results)

        if not papers:
            return []

        # Score papers
        scored = self.scorer.batch_score(papers, sub)

        # Record results and update last check
        new_papers = []
        last_check_id = ""

        for paper in scored:
            arxiv_id = paper.get("arxiv_id", "")
            last_check_id = arxiv_id

            # Check if already recorded
            existing = self.db.get_subscription_papers(sub_id, limit=1)
            already_recorded = any(p.get("arxiv_id") == arxiv_id for p in existing)

            if not already_recorded:
                self.db.record_subscription_paper(
                    sub_id=sub_id,
                    arxiv_id=arxiv_id,
                    title=paper.get("title", ""),
                    score=paper.get("score", 0),
                    gap_coverage=paper.get("gap_coverage", 0),
                    semantic_sim=paper.get("semantic_sim", 0),
                    published=paper.get("published", ""),
                )
                new_papers.append(paper)

        # Update last check position
        if last_check_id:
            self.db.update_subscription_last_check(sub_id, last_check_id)

        logger.info(
            f"Checked subscription [{sub_id}] '{topic}': "
            f"{len(papers)} found, {len(new_papers)} new above threshold"
        )

        return new_papers

    def check_all(self) -> Dict[str, List[Dict[str, Any]]]:
        """Check all enabled subscriptions.

        Returns:
            Dict mapping subscription ID to list of new papers
        """
        subs = self.db.list_arxiv_subscriptions()
        results = {}

        for sub in subs:
            if sub.get("enabled", True):
                sub_id = sub.get("id", "")
                try:
                    papers = self.check_subscription(sub_id)
                    results[sub_id] = papers
                except Exception as e:
                    logger.error(f"Error checking subscription {sub_id}: {e}")
                    results[sub_id] = []

        return results

    def _build_query(self, topic: str, keywords: List[str]) -> str:
        """Build arXiv search query from topic and keywords."""
        parts = [topic]

        # Add keywords as additional search terms
        for kw in keywords:
            if kw not in topic.lower():
                parts.append(kw)

        # Join with AND for specificity
        query = " AND ".join(f'"{p}"' if " " in p else p for p in parts)
        return query

    def _search_arxiv(self, query: str, max_results: int) -> List[Dict[str, Any]]:
        papers = search_arxiv(query, max_results)
        return [
            {
                "arxiv_id": p.uid,
                "title": p.title,
                "abstract": p.abstract,
                "published": p.published,
            }
            for p in papers
        ]


def search_arxiv(query: str, max_results: int = 10):
    """Search arXiv API and return papers (delegates to canonical parser)."""
    from parsers.arxiv_search import search_arxiv as canonical_search
    return canonical_search(query, max_results)
