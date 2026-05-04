"""Credibility scoring and trendslop detection for Gene Pool capsules."""

from __future__ import annotations

import json
import math
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional, Set, Tuple

from llm.insight.gene import CapsuleGene


# ─── Constants ──────────────────────────────────────────────────────────

TRENDSLOP_KEYWORD_OVERLAP_THRESHOLD = 0.70
CREDIBILITY_HIGH_THRESHOLD = 0.70
CREDIBILITY_LOW_THRESHOLD = 0.35

EVIDENCE_WEIGHT = 0.35
NOVELTY_WEIGHT = 0.30
SOURCE_TRUST_WEIGHT = 0.20
CONSISTENCY_WEIGHT = 0.15


@dataclass
class CredibilityScore:
    """Multi-dimensional credibility assessment for a capsule."""

    capsule_id: str
    overall: float  # 0.0–1.0 composite

    # Dimensions
    novelty_v2: float  # 0.0–1.0, low = trendslop risk
    evidence_strength: float  # 0.0–1.0, based on feedback × outcome
    source_trust: float  # 0.0–1.0, from SourceTrustTracker
    consistency: float  # 0.0–1.0, cross-type coherence

    # Flags
    trendslop: bool  # True if keyword overlap > threshold
    trendslop_reason: str = ""  # e.g. "83% overlap with 4 capsules on 'transformer'"

    # Badge
    badge: str = "medium"  # "high" | "medium" | "low"

    def to_dict(self) -> Dict[str, Any]:
        return {
            "capsule_id": self.capsule_id,
            "overall": round(self.overall, 3),
            "novelty_v2": round(self.novelty_v2, 3),
            "evidence_strength": round(self.evidence_strength, 3),
            "source_trust": round(self.source_trust, 3),
            "consistency": round(self.consistency, 3),
            "trendslop": self.trendslop,
            "trendslop_reason": self.trendslop_reason,
            "badge": self.badge,
        }


class CredibilityScorer:
    """Compute credibility scores for capsules with trendslop detection.

    Two pass design:
      Pass 1: compute pairwise keyword overlaps for ALL active capsules
              to get the novelty_v2 baseline.
      Pass 2: per-capsule composite score using novelty_v2, evidence,
              source trust, and consistency.
    """

    def __init__(
        self,
        source_trust: Optional[Dict[str, float]] = None,
    ):
        self.source_trust = source_trust or {}

    # ─── Pass 1: Novelty V2 (keyword overlap) ─────────────────────────

    def compute_novelty_scores(
        self, capsules: List[CapsuleGene]
    ) -> Dict[str, CredibilityScore]:
        """Run full credibility assessment on a list of capsules.

        Returns dict[capsule_id] → CredibilityScore.
        """
        active = [c for c in capsules if c.status == "active"]
        if not active:
            return {}

        # Keyword index: keyword → list of capsule_ids
        kw_index: Dict[str, List[str]] = {}
        for c in active:
            seen: Set[str] = set()
            for kw in c.trigger_keywords:
                norm = kw.lower().strip()
                if norm and norm not in seen:
                    seen.add(norm)
                    kw_index.setdefault(norm, []).append(c.capsule_id)

        # Per capsule: compute max overlap with any other capsule
        overlaps: Dict[str, Tuple[float, str]] = {}
        for c in active:
            kws = set(kw.lower().strip() for kw in c.trigger_keywords if kw.strip())
            if not kws:
                overlaps[c.capsule_id] = (0.0, "")
                continue

            max_overlap = 0.0
            worst_match = ""
            for other in active:
                if other.capsule_id == c.capsule_id:
                    continue
                other_kws = set(
                    kw.lower().strip() for kw in other.trigger_keywords if kw.strip()
                )
                if not other_kws:
                    continue
                intersection = len(kws & other_kws)
                union = len(kws | other_kws)
                jaccard = intersection / union if union > 0 else 0.0
                if jaccard > max_overlap:
                    max_overlap = jaccard
                    worst_match = other.capsule_id

            overlaps[c.capsule_id] = (max_overlap, worst_match)

        # Also find top-N most similar capsules for each
        similar_counts: Dict[str, int] = {}
        for cid, (_, _) in overlaps.items():
            similar_counts[cid] = sum(
                1 for o_cid, (o_ov, _) in overlaps.items() if o_ov >= TRENDSLOP_KEYWORD_OVERLAP_THRESHOLD and o_cid != cid
            )

        # Build results
        results = {}
        for c in active:
            overlap_ratio, worst_id = overlaps.get(c.capsule_id, (0.0, ""))
            trendslop = overlap_ratio >= TRENDSLOP_KEYWORD_OVERLAP_THRESHOLD

            # Novelty v2 = inverse of max overlap, floored at 0
            novelty_v2 = max(0.0, 1.0 - overlap_ratio)

            # Evidence strength
            evidence = c.outcome_success_score * math.log(c.feedback_count + 2) / math.log(12)

            # Source trust (default 0.5)
            source = 0.5
            src = c.archetype.get("source_arxiv_category", "")
            if src and src in self.source_trust:
                source = self.source_trust[src]

            # Consistency: if trendslop, reduce
            consistency = 0.5 if trendslop else 0.7
            if c.outcome_success_score > 0.7 and c.feedback_count > 2:
                consistency = 0.9
            elif c.outcome_success_score < 0.3:
                consistency = 0.3

            overall = (
                EVIDENCE_WEIGHT * evidence
                + NOVELTY_WEIGHT * novelty_v2
                + SOURCE_TRUST_WEIGHT * source
                + CONSISTENCY_WEIGHT * consistency
            )

            badge = (
                "high" if overall >= CREDIBILITY_HIGH_THRESHOLD
                else "low" if overall < CREDIBILITY_LOW_THRESHOLD
                else "medium"
            )

            reason_parts = []
            if trendslop:
                count = similar_counts.get(c.capsule_id, 0)
                reason_parts.append(
                    f"{overlap_ratio:.0%} keyword overlap with {count} other capsule(s)"
                )
            if evidence < 0.3:
                reason_parts.append("low evidence (few feedbacks)")
            if c.feedback_count == 0:
                reason_parts.append("unvalidated (no feedback yet)")

            score = CredibilityScore(
                capsule_id=c.capsule_id,
                overall=overall,
                novelty_v2=novelty_v2,
                evidence_strength=evidence,
                source_trust=source,
                consistency=consistency,
                trendslop=trendslop,
                trendslop_reason="; ".join(reason_parts) if reason_parts else "",
                badge=badge,
            )
            results[c.capsule_id] = score

        return results

    def is_trendslop(
        self,
        capsule: CapsuleGene,
        all_capsules: List[CapsuleGene],
    ) -> Tuple[bool, float, str]:
        """Quick check: is a single capsule trendslop against the pool?

        Returns (is_trendslop, max_overlap_ratio, reason).
        """
        kws = set(kw.lower().strip() for kw in capsule.trigger_keywords if kw.strip())
        if not kws:
            return False, 0.0, ""

        max_overlap = 0.0
        similar_count = 0
        for other in all_capsules:
            if other.capsule_id == capsule.capsule_id or other.status == "archived":
                continue
            other_kws = set(
                kw.lower().strip() for kw in other.trigger_keywords if kw.strip()
            )
            if not other_kws:
                continue
            intersection = len(kws & other_kws)
            union = len(kws | other_kws)
            jaccard = intersection / union if union > 0 else 0.0
            if jaccard >= TRENDSLOP_KEYWORD_OVERLAP_THRESHOLD:
                similar_count += 1
                if jaccard > max_overlap:
                    max_overlap = jaccard

        trendslop = max_overlap >= TRENDSLOP_KEYWORD_OVERLAP_THRESHOLD
        reason = (
            f"{max_overlap:.0%} keyword overlap with {similar_count} other capsule(s)"
            if trendslop else ""
        )
        return trendslop, max_overlap, reason

    # ─── HTML rendering (for Web UI) ─────────────────────────────────

    def render_html(self) -> str:
        """Render credibility scores as HTML fragment for the web UI."""
        from llm.insight.storage import CapsuleStorageMixin

        # Get capsules from EvolutionTracker's storage
        try:
            from llm.insight.tracker import EvolutionTracker
            tracker = EvolutionTracker()
            gene_file = tracker.data_dir / "gene_pool.jsonl"
            if not gene_file.exists():
                return "<p>No capsules in Gene Pool yet.</p>"
            capsules = []
            import json
            with open(gene_file, encoding="utf-8") as f:
                for line in f:
                    line = line.strip()
                    if line:
                        try:
                            from llm.insight.gene import CapsuleGene
                            capsules.append(CapsuleGene.from_dict(json.loads(line)))
                        except Exception:
                            continue
        except Exception:
            return "<p>Could not load Gene Pool data.</p>"

        if not capsules:
            return "<p>No capsules in Gene Pool yet.</p>"

        scores = self.compute_novelty_scores(capsules)
        if not scores:
            return "<p>No active capsules to assess.</p>"

        capsule_map = {c.capsule_id: c for c in capsules}
        score_list = list(scores.values())
        trendslop_count = sum(1 for s in score_list if s.trendslop)

        lines = ['<div class="credibility-panel">']
        lines.append(
            f"<h3>Gap Credibility Scores <small style='color:#888'>({len(capsules)} capsules, {trendslop_count} trendslop)</small></h3>"
        )
        lines.append(
            "<p style='color:#666;font-size:13px;'>Novelty = inverse of keyword overlap. "
            "Capsules with >70% Jaccard similarity are flagged as trendslop.</p>"
        )

        # Summary badges
        high_count = sum(1 for s in score_list if s.badge == "high")
        low_count = sum(1 for s in score_list if s.badge == "low")
        lines.append(
            f"<div style='display:flex;gap:16px;margin-bottom:16px;'>"
            f"<span style='background:#7A9E7A;color:white;padding:4px 12px;border-radius:12px;font-size:12px'>High: {high_count}</span>"
            f"<span style='background:#D4A059;color:white;padding:4px 12px;border-radius:12px;font-size:12px'>Medium: {len(score_list) - high_count - low_count}</span>"
            f"<span style='background:#C4706A;color:white;padding:4px 12px;border-radius:12px;font-size:12px'>Low: {low_count}</span>"
            f"<span style='background:#D9534F;color:white;padding:4px 12px;border-radius:12px;font-size:12px'>Trendslop: {trendslop_count}</span>"
            f"</div>"
        )

        lines.append('<table class="credibility-table">')
        lines.append(
            "<thead><tr>"
            "<th>Gap Title</th>"
            "<th>Type</th>"
            "<th>Outcome</th>"
            "<th>Novelty V2</th>"
            "<th>Evidence</th>"
            "<th>Badge</th>"
            "<th>Status</th>"
            "</tr></thead>"
        )
        lines.append("<tbody>")

        for s in sorted(score_list, key=lambda x: x.overall, reverse=True):
            c = capsule_map.get(s.capsule_id)
            title = c.action_gap_title if c else s.capsule_id[:12]
            gap_type = c.action_gap_type if c else "?"

            novelty_pct = int(s.novelty_v2 * 100)
            evidence_pct = int(s.evidence_strength * 100)

            color_map = {"high": "#7A9E7A", "medium": "#D4A059", "low": "#C4706A"}
            badge_color = color_map.get(s.badge, "#888")
            badge_html = (
                f'<span style="background:{badge_color};color:white;padding:2px 8px;border-radius:10px;font-size:11px">{s.badge.upper()}</span>'
            )
            status_html = (
                '<span style="background:#D9534F;color:white;padding:2px 8px;border-radius:10px;font-size:11px">⚠️ TRENDSLOP</span>'
                if s.trendslop
                else '<span style="background:#7A9E7A;color:white;padding:2px 8px;border-radius:10px;font-size:11px">✓ Original</span>'
            )

            lines.append("<tr>")
            lines.append(
                f"<td style='max-width:260px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap'><code title='{title}'>{str(title)[:40]}</code></td>"
            )
            lines.append(f"<td><code>{gap_type}</code></td>")
            lines.append(f"<td>{c.outcome_success_score:.2f}" if c else "<td>?</td>")
            lines.append(f"<td>{novelty_pct}%</td>")
            lines.append(f"<td>{evidence_pct}%</td>")
            lines.append(f"<td>{badge_html}</td>")
            lines.append(f"<td>{status_html}</td>")
            lines.append("</tr>")

        lines.append("</tbody></table>")
        lines.append("<style>")
        lines.append(".credibility-panel { font-family: Georgia, serif; }")
        lines.append(
            ".credibility-table { width: 100%; border-collapse: collapse; margin-top: 1rem; }"
        )
        lines.append(
            ".credibility-table th, .credibility-table td { padding: 0.4rem 0.8rem; border-bottom: 1px solid #e8e4de; text-align: left; font-size: 13px; }"
        )
        lines.append(
            ".credibility-table th { font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em; color: #7a7570; }"
        )
        lines.append("</style>")
        lines.append("</div>")
        return "\n".join(lines)

    # ─── Batch rendering ──────────────────────────────────────────────

    def render_credibility_report(
        self,
        scores: Dict[str, CredibilityScore],
        capsules: List[CapsuleGene],
        top_n: int = 20,
    ) -> str:
        """Render a human-readable credibility report."""
        lines = ["=== Gene Pool Credibility Report ===", ""]

        capsule_map = {c.capsule_id: c for c in capsules}

        # Summary stats
        all_scores = list(scores.values())
        if not all_scores:
            return "No capsules to assess."

        avg = sum(s.overall for s in all_scores) / len(all_scores)
        trendslop_count = sum(1 for s in all_scores if s.trendslop)
        high_count = sum(1 for s in all_scores if s.badge == "high")
        low_count = sum(1 for s in all_scores if s.badge == "low")

        lines.append(f"  Total capsules:     {len(all_scores)}")
        lines.append(f"  Average credibility: {avg:.3f}")
        lines.append(f"  High credibility:    {high_count}")
        lines.append(f"  Low credibility:     {low_count}")
        lines.append(f"  Trendslop flagged:   {trendslop_count}")
        lines.append("")

        # Trendslop capsules
        trendslop_list = [s for s in all_scores if s.trendslop]
        if trendslop_list:
            lines.append("── Trendslop Capsules ──")
            for s in sorted(trendslop_list, key=lambda x: x.overall)[:top_n]:
                c = capsule_map.get(s.capsule_id)
                title = c.action_gap_title[:50] if c else s.capsule_id[:12]
                lines.append(
                    f"  [LOW]  {title:<50}  novelty={s.novelty_v2:.2f}  "
                    f"evidence={s.evidence_strength:.2f}  {s.trendslop_reason}"
                )
            lines.append("")

        # Top credible
        high_list = [s for s in all_scores if s.badge == "high"]
        if high_list:
            lines.append("── Top Credible Capsules ──")
            for s in sorted(high_list, key=lambda x: x.overall, reverse=True)[:10]:
                c = capsule_map.get(s.capsule_id)
                title = c.action_gap_title[:50] if c else s.capsule_id[:12]
                lines.append(
                    f"  [HIGH] {title:<50}  overall={s.overall:.3f}  "
                    f"novelty={s.novelty_v2:.2f}  evidence={s.evidence_strength:.2f}"
                )
            lines.append("")

        return "\n".join(lines)
