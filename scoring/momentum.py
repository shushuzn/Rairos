"""Research momentum / paper importance scoring.

Formula:
    score = citation_score * 0.3
          + tag_popularity * 0.25
          + recency_boost * 0.2
          + novelty_factor * 0.15
          + radar_heat * 0.1
All components are normalised to [0, 100].
"""

import math

import orjson
from datetime import datetime, timezone
from pathlib import Path

from kg.manager import KGManager
from typing import Optional


class ResearchMomentum:
    """Compute research momentum scores for papers and tags."""

    def __init__(self, kg: Optional[KGManager] = None):
        self.kg = kg or KGManager()
        self._scores_path = Path("data/momentum_scores.json")
        self._scores_path.parent.mkdir(parents=True, exist_ok=True)
        self._scores: dict[str, float] = self._load_scores()

    def _load_scores(self) -> dict:
        if self._scores_path.exists():
            try:
                return orjson.loads(self._scores_path.read_bytes())
            except Exception:
                pass
        return {}

    def save_scores(self):
        self._scores_path.write_bytes(orjson.dumps(self._scores))

    # ─── Core scoring ───────────────────────────────────────────────

    def score_paper(self, paper_uid: str) -> float:
        """Compute momentum score for a paper (cached)."""
        if paper_uid in self._scores:
            return self._scores[paper_uid]

        node = self.kg.get_node_by_entity("Paper", paper_uid)
        if node is None:
            return 0.0

        score = self._compute_score(node)
        self._scores[paper_uid] = score
        return score

    def _compute_score(self, node: dict) -> float:
        # _compute_score body starts below
        tags = []
        props = node.get("properties", {})
        year = props.get("year")

        # Get tags from edges
        node_id = node["id"]
        tag_edges = self.kg.get_edges_by_node(node_id, direction="out", rel_type="same_tag")
        for e in tag_edges:
            tgt = self.kg.get_node(e["target_id"])
            if tgt and tgt.get("type") == "Tag":
                tags.append(tgt.get("label", ""))

        # 1. Citation score (30%) — log scale
        in_cite = self.kg.get_edges_by_node(node_id, direction="in", rel_type="cite")
        out_cite = self.kg.get_edges_by_node(node_id, direction="out", rel_type="cite")
        total_cites = len(in_cite)  # forward citations
        citation_score = 0.0
        if total_cites > 0:
            citation_score = min(100.0, math.log1p(total_cites) / math.log(101) * 100)

        # 2. Tag popularity (25%) — percentile within same tag
        tag_popularity = 50.0  # default middle
        if tags:
            tag_pop_scores = []
            for t in tags:
                papers = self.kg.find_papers_by_tag(t)
                if len(papers) >= 2:
                    scores = [self._raw_cite_score(self.kg.get_node(p["id"])) for p in papers]
                    my_raw = self._raw_cite_score(node)
                    rank = sum(1 for s in scores if s < my_raw)
                    pct = rank / max(1, len(scores) - 1) * 100
                    tag_pop_scores.append(pct)
            if tag_pop_scores:
                tag_popularity = sum(tag_pop_scores) / len(tag_pop_scores)

        # 3. Recency boost (20%)
        recency_boost = 0.0
        if year:
            try:
                y = int(year)
                now = datetime.now(timezone.utc).year
                age = max(0, now - y)
                recency_boost = math.exp(-age / 5.0) * 20.0
            except Exception:
                pass

        # 4. Novelty factor (15%) — cited but rarely cites others (originator papers)
        novelty_factor = 0.0
        if out_cite and len(out_cite) > 0:
            novelty_factor = min(15.0, len(in_cite) / max(1, len(out_cite)) * 10.0)
        elif len(in_cite) > 0:
            novelty_factor = 15.0  # cited but cites no one — originator

        # 5. Radar heat (10%)
        radar_heat = 0.0
        radar_data = self._load_radar()
        for t in tags:
            heat = radar_data.get(t, {}).get("score", 0)
            radar_heat = max(radar_heat, heat)  # take highest tag heat

        total = (
            citation_score * 0.30 +
            tag_popularity * 0.25 +
            recency_boost * 0.20 +
            novelty_factor * 0.15 +
            radar_heat * 0.10
        )
        return round(min(100.0, total), 2)

    def _raw_cite_score(self, node: Optional[dict]) -> float:
        if node is None:
            return 0.0
        in_c = self.kg.get_edges_by_node(node["id"], direction="in", rel_type="cite")
        return len(in_c)

    def _load_radar(self) -> dict:
        candidates = [Path("data/radar.json"), Path("radar.json")]
        for p in candidates:
            if p.exists():
                try:
                    return orjson.loads(p.read_bytes())
                except Exception:
                    pass
        return {}

    # ─── High-level queries ────────────────────────────────────────

    def get_top_papers(self, tag: Optional[str] = None, top_n: int = 20) -> list[tuple[str, float]]:
        """Return top-N papers by momentum score, optionally filtered by tag."""
        if tag:
            paper_nodes = self.kg.find_papers_by_tag(tag)
            scored = [(p["entity_id"], self.score_paper(p["entity_id"])) for p in paper_nodes]
        else:
            all_papers = self.kg.get_all_nodes(node_type="Paper")
            scored = [(p["entity_id"], self.score_paper(p["entity_id"])) for p in all_papers]

        scored.sort(key=lambda x: x[1], reverse=True)
        return scored[:top_n]

    def score_tag(self, tag: str) -> dict:
        """Score a tag by aggregate paper momentum."""
        papers = self.kg.find_papers_by_tag(tag)
        if not papers:
            return {"raw_score": 0.0, "papers_count": 0, "avg_citation": 0.0,
                    "heat_trend": "unknown", "momentum_label": "niche"}

        papers_scores = [self.score_paper(p["entity_id"]) for p in papers]
        raw_score = sum(papers_scores) / len(papers_scores)
        citations = [len(self.kg.get_edges_by_node(p["id"], direction="in", rel_type="cite")) for p in papers]
        avg_cite = sum(citations) / max(1, len(citations))

        radar_data = self._load_radar()
        heat = radar_data.get(tag, {}).get("score", 0)
        if heat > 70:
            trend = "rising"
            label = "hot"
        elif heat > 40:
            trend = "stable"
            label = "established"
        elif heat > 0:
            trend = "declining"
            label = "maturing"
        else:
            trend = "unknown"
            label = "niche"

        return {
            "raw_score": round(raw_score, 2),
            "papers_count": len(papers),
            "avg_citation": round(avg_cite, 2),
            "heat_trend": trend,
            "momentum_label": label,
        }

    def get_tag_leaderboard(self) -> list[tuple[str, float]]:
        """All tags ranked by momentum score."""
        all_nodes = self.kg.get_all_nodes(node_type="Tag")
        scored = [(n["entity_id"], self.score_tag(n["entity_id"])["raw_score"]) for n in all_nodes]
        scored.sort(key=lambda x: x[1], reverse=True)
        return scored

    def refresh_all(self):
        """Recompute all scores from scratch."""
        self._scores.clear()
        all_papers = self.kg.get_all_nodes(node_type="Paper")
        for p in all_papers:
            self.score_paper(p["entity_id"])
        self.save_scores()
