"""
rankers — Pluggable ranking strategies for paper similarity search.

Modules:
  base     — RankedResult type and Ranker abstract base
  cosine   — CosineSimilarityRanker
  score    — CompositeScorer (semantic + recency + parse_quality)

Usage:
  from rankers import CosineSimilarityRanker
  results = CosineSimilarityRanker(db).rank(paper_id, threshold=0.85, limit=20)
"""
from rankers.base import RankedResult, Ranker
from rankers.cosine import CosineSimilarityRanker
from rankers.score import CompositeScorer

__all__ = [
    "RankedResult",
    "Ranker",
    "CosineSimilarityRanker",
    "CompositeScorer",
]
