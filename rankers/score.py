"""Composite scoring strategy combining semantic similarity + paper metadata."""
from __future__ import annotations
from typing import TYPE_CHECKING, List, Optional

if TYPE_CHECKING:
    from db.database import Database

from rankers.base import RankedResult, Ranker


class CompositeScorer(Ranker):
    """
    Rank papers by a weighted combination of:

    - cosine similarity of embeddings (semantic overlap)
    - recency bonus (newer papers rank higher)
    - parse quality bonus (papers with better parse_status rank higher)

    Weights are configurable; defaults aim to boost relevant + high-quality papers.
    """

    name = "composite"

    def __init__(
        self,
        db: "Database",
        *,
        sim_weight: float = 0.7,
        recency_weight: float = 0.2,
        parse_weight: float = 0.1,
        year_boost_range: int = 5,
    ) -> None:
        self._db = db
        self.sim_weight = sim_weight
        self.recency_weight = recency_weight
        self.parse_weight = parse_weight
        self.year_boost_range = year_boost_range

    def rank(
        self,
        paper_id: str,
        threshold: float = 0.0,
        limit: int = 20,
    ) -> List[RankedResult]:
        """
        Rank papers by composite score.

        Args:
            paper_id: Query paper ID
            threshold: Minimum composite score to include (default 0.0)
            limit: Maximum number of results (default 20)

        Returns:
            List of (PaperRecord, composite_score) sorted by score descending
        """
        from datetime import date

        from rankers.cosine import CosineSimilarityRanker

        sim_ranker = CosineSimilarityRanker(self._db)
        sim_results = sim_ranker.rank(paper_id, threshold=0.0, limit=100)

        if not sim_results:
            return []

        # Determine reference year (most recent paper in results, or current year)
        ref_year: Optional[int] = None
        for prow, _ in sim_results:
            if prow.published is not None:
                yr = prow.published.year if hasattr(prow.published, "year") else int(str(prow.published)[:4])
                if ref_year is None or yr > ref_year:
                    ref_year = yr
        if ref_year is None:
            ref_year = date.today().year

        scored: List[RankedResult] = []
        for prow, sim_score in sim_results:
            # Normalize similarity to [0, 1]
            sim_norm = min(sim_score, 1.0)

            # Recency: 0-1 based on distance from ref_year
            recency_norm = 0.0
            if prow.published is not None:
                yr = prow.published.year if hasattr(prow.published, "year") else int(str(prow.published)[:4])
                year_dist = max(0, min(ref_year - yr, self.year_boost_range))
                recency_norm = 1.0 - (year_dist / self.year_boost_range)

            # Parse quality: map parse_status to 0-1
            parse_norm = self._parse_quality_score(prow.parse_status)

            composite = (
                self.sim_weight * sim_norm
                + self.recency_weight * recency_norm
                + self.parse_weight * parse_norm
            )

            if composite >= threshold:
                scored.append((prow, composite))

        scored.sort(key=lambda x: x[1], reverse=True)
        return scored[:limit]

    @staticmethod
    def _parse_quality_score(parse_status: Optional[str]) -> float:
        """Map parse_status string to 0-1 quality score."""
        if parse_status is None:
            return 0.0
        mapping = {
            "full": 1.0,
            "sections": 0.8,
            "partial": 0.5,
            "failed": 0.1,
        }
        return mapping.get(parse_status.lower(), 0.0)
