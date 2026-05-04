"""
Paper Impact Scorer — Composite influence scoring for research papers.

Combines:
- Raw citation count normalized by paper age
- PageRank-style influence propagation
- Citation velocity / momentum (recent citation rate)
- Author h-index aggregation
"""

from dataclasses import dataclass, field
from typing import Dict, List, Optional, Any
import math


@dataclass
class ImpactScore:
    """Composite impact score for a paper."""

    paper_id: str
    title: str
    year: int

    raw_citations: int = 0
    normalized_score: float = 0.0  # age-normalized citations per year
    pagerank_score: float = 0.0  # influence propagation score
    momentum_score: float = 0.0  # recent citation velocity
    author_h_index: float = 0.0  # author h-index aggregate
    composite_score: float = 0.0  # weighted composite

    percentile: float = 0.0  # rank among scored papers
    tier: str = ""  # S/A/B/C/D

    def to_dict(self) -> Dict[str, Any]:
        return {
            "paper_id": self.paper_id,
            "title": self.title,
            "year": self.year,
            "raw_citations": self.raw_citations,
            "normalized_score": round(self.normalized_score, 3),
            "pagerank_score": round(self.pagerank_score, 3),
            "momentum_score": round(self.momentum_score, 3),
            "author_h_index": round(self.author_h_index, 3),
            "composite_score": round(self.composite_score, 3),
            "percentile": round(self.percentile, 1),
            "tier": self.tier,
        }


class ImpactScorer:
    """Compute composite impact scores for papers."""

    # Weights for composite score
    WEIGHT_NORMALIZED = 0.30
    WEIGHT_PAGERANK = 0.30
    WEIGHT_MOMENTUM = 0.25
    WEIGHT_AUTHOR = 0.15

    # Tier thresholds (composite score)
    TIER_THRESHOLDS = {"S": 0.8, "A": 0.6, "B": 0.4, "C": 0.2}

    def __init__(self, db=None):
        self.db = db
        self._scores: Dict[str, ImpactScore] = {}
        self._pagerank_iterations = 4

    def score_paper(
        self,
        paper_id: str,
        title: str,
        year: int,
        raw_citations: int,
        citing_papers: Optional[List[Dict]] = None,
        author_h_index: float = 0.0,
    ) -> ImpactScore:
        """Score a single paper."""
        current_year = 2026

        # 1. Age-normalized citations per year
        age = max(current_year - year, 1)
        normalized = raw_citations / age

        # 2. PageRank-style propagation (simplified)
        pagerank = self._compute_pagerank(paper_id, citing_papers or [])

        # 3. Momentum: raw proxy via citations per year (no historical data needed)
        # Higher weight on recent years would need time-series; use age-normalized as proxy
        momentum = raw_citations / (age**0.7)  # slight de-emphasis of age

        # 4. Composite
        composite = (
            self.WEIGHT_NORMALIZED * self._normalize(normalized)
            + self.WEIGHT_PAGERANK * pagerank
            + self.WEIGHT_MOMENTUM * self._normalize(momentum)
            + self.WEIGHT_AUTHOR * min(author_h_index / 50.0, 1.0)
        )

        score = ImpactScore(
            paper_id=paper_id,
            title=title,
            year=year,
            raw_citations=raw_citations,
            normalized_score=normalized,
            pagerank_score=pagerank,
            momentum_score=momentum,
            author_h_index=author_h_index,
            composite_score=composite,
        )
        score.tier = self._tier(composite)
        self._scores[paper_id] = score
        return score

    def score_batch(
        self,
        papers: List[Dict],
        citing_map: Optional[Dict[str, List[str]]] = None,
    ) -> List[ImpactScore]:
        """Score a batch of papers and assign percentiles."""
        results = []
        for p in papers:
            citing = citing_map.get(p["paper_id"], []) if citing_map else []
            citing_refs = [{"paper_id": c} for c in citing]
            score = self.score_paper(
                paper_id=p["paper_id"],
                title=p.get("title", ""),
                year=p.get("year", 2020),
                raw_citations=p.get("citation_count", 0) or 0,
                citing_papers=citing_refs,
                author_h_index=p.get("author_h_index", 0.0),
            )
            results.append(score)

        # Assign percentiles
        self._assign_percentiles(results)
        return results

    def _compute_pagerank(self, paper_id: str, citing_papers: List[Dict]) -> float:
        """Simplified PageRank: citations from high-scoring papers count more."""
        if not citing_papers:
            return 0.1  # baseline

        # Iterative propagation
        scores = {p["paper_id"]: 1.0 for p in citing_papers}
        damping = 0.85

        for _ in range(self._pagerank_iterations):
            new_scores: Dict[str, float] = {}
            for pid, score in scores.items():
                if pid in self._scores:
                    # Transfer influence
                    inherited = score * damping
                    new_scores[pid] = new_scores.get(pid, 0.0) + inherited
            # Normalize
            total = sum(new_scores.values()) or 1.0
            scores = {k: v / total for k, v in new_scores.items()}

        return sum(scores.values())

    def _normalize(self, value: float, baseline: float = 10.0) -> float:
        """Sigmoid-ish normalization to [0, 1]."""
        return 1.0 - (1.0 / (1.0 + value / baseline))

    def _tier(self, composite: float) -> str:
        for tier, threshold in self.TIER_THRESHOLDS.items():
            if composite >= threshold:
                return tier
        return "D"

    def _assign_percentiles(self, scores: List[ImpactScore]):
        """Assign percentile rank among scored papers."""
        if not scores:
            return
        sorted_scores = sorted(scores, key=lambda s: s.composite_score, reverse=True)
        n = len(sorted_scores)
        for i, s in enumerate(sorted_scores):
            s.percentile = ((n - i) / n) * 100.0

    def get_top_papers(
        self,
        papers: List[Dict],
        limit: int = 20,
        min_score: float = 0.0,
        year_filter: Optional[int] = None,
    ) -> List[ImpactScore]:
        """Get top-scoring papers, optionally filtered."""
        scored = self.score_batch(papers)
        filtered = [s for s in scored if s.composite_score >= min_score]
        if year_filter:
            filtered = [s for s in filtered if s.year >= year_filter]
        return sorted(filtered, key=lambda s: s.composite_score, reverse=True)[:limit]

    def rank_papers(
        self,
        papers: List[Dict],
        top_k: int = 10,
    ) -> List[Dict[str, Any]]:
        """Rank papers and return detailed results."""
        scored = self.score_batch(papers)
        top = sorted(scored, key=lambda s: s.composite_score, reverse=True)[:top_k]

        return [
            {
                "rank": i + 1,
                **s.to_dict(),
                "why": self._explain_score(s),
            }
            for i, s in enumerate(top)
        ]

    def _explain_score(self, s: ImpactScore) -> str:
        """Human-readable explanation of score breakdown."""
        parts = []
        if s.normalized_score > 0.5:
            parts.append(f"高年化引用 ({s.normalized_score:.1f}/年)")
        if s.pagerank_score > 0.3:
            parts.append("被高影响力论文引用")
        if s.momentum_score > 0.4:
            parts.append("引用增长强劲")
        if s.author_h_index > 30:
            parts.append(f"作者H指数高 ({s.author_h_index:.0f})")
        return ", ".join(parts) if parts else "综合评分"

    # ── Rendering ──────────────────────────────────────────────

    def render_ranking(self, ranking: List[Dict]) -> str:
        """Render ranking as ASCII table."""
        if not ranking:
            return "No papers to rank."

        lines = ["=" * 70, "📊 Paper Impact Ranking", "=" * 70, ""]
        lines.append(f"{'Rank':<6}{'Tier':<6}{'Score':<8}{'Citations':<12}{'Year':<6} Title")
        lines.append("-" * 70)

        for entry in ranking:
            tier_emoji = {"S": "⭐", "A": "🅰️", "B": "🅱️", "C": "⚙️", "D": "📄"}
            emoji = tier_emoji.get(entry["tier"], "📄")
            title = entry["title"][:40]
            lines.append(
                f"{entry['rank']:<6}{emoji:<6}{entry['composite_score']:<8.3f}"
                f"{entry['raw_citations']:<12}{entry['year']:<6}{title}"
            )

        lines.append("=" * 70)
        return "\n".join(lines)
