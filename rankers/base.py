"""Base abstractions for ranking strategies."""
from __future__ import annotations

from abc import ABC, abstractmethod
from typing import TYPE_CHECKING, List, Tuple

if TYPE_CHECKING:
    from db.database import PaperRecord


RankedResult = Tuple["PaperRecord", float]
"""A paper record paired with its ranking score (higher = better)."""


class Ranker(ABC):
    """Abstract base for paper ranking strategies."""

    @abstractmethod
    def rank(
        self,
        paper_id: str,
        threshold: float = 0.0,
        limit: int = 20,
    ) -> List[RankedResult]:
        """
        Rank papers similar to paper_id.

        Args:
            paper_id: Query paper ID
            threshold: Minimum score to include (default 0.0 = no filter)
            limit: Maximum number of results to return

        Returns:
            List of (PaperRecord, score) sorted by score descending
        """
        ...
