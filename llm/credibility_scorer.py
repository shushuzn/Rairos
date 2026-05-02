"""Gene Pool Credibility Scorer.

Computes per-capsule novelty scores based on keyword overlap (Jaccard).
Flags capsules with high keyword redundancy as "trendslop".
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, List, Optional


CAPSULE_PATH = Path.home() / ".ai_research_os" / "gene_pool" / "capsules.json"
TRENDSLOP_THRESHOLD = 0.7  # Jaccard overlap above this = "trendslop"


def jaccard(a: List[str], b: List[str]) -> float:
    s_a, s_b = set(a), set(b)
    if not s_a or not s_b:
        return 0.0
    return len(s_a & s_b) / len(s_a | s_b)


@dataclass
class CapsuleCredibility:
    capsule_id: str
    gap_title: str
    gap_type: str
    outcome_score: float
    novelty_score: float  # 1 - max_overlap (high = original)
    max_overlap: float  # Jaccard with most-similar capsule
    is_trendslop: bool
    trigger_keywords: List[str]

    def to_dict(self) -> Dict[str, Any]:
        return {
            "capsule_id": self.capsule_id,
            "gap_title": self.gap_title,
            "gap_type": self.gap_type,
            "outcome_score": self.outcome_score,
            "novelty_score": self.novelty_score,
            "max_overlap": self.max_overlap,
            "is_trendslop": self.is_trendslop,
            "trigger_keywords": self.trigger_keywords,
        }


from dataclasses import dataclass


class CredibilityScorer:
    """Compute per-capsule novelty scores from Gene Pool capsule history."""

    def __init__(self):
        self._credibility: Optional[List[CapsuleCredibility]] = None

    # ── Core computation ────────────────────────────────────────────────

    def compute_credibility(self, force: bool = False) -> List[CapsuleCredibility]:
        """Compute novelty + trendslop flag for all capsules.

        A capsule's novelty_score = 1 - max_jaccard, where max_jaccard is
        the highest Jaccard similarity against any other capsule in the pool.
        """
        if self._credibility is not None and not force:
            return self._credibility

        if not CAPSULE_PATH.exists():
            self._credibility = []
            return self._credibility

        data = json.loads(CAPSULE_PATH.read_text(encoding="utf-8"))
        capsules = data.get("capsules", [])

        results: List[CapsuleCredibility] = []
        n = len(capsules)

        for i, cap in enumerate(capsules):
            kw = cap.get("trigger_keywords", [])
            cap_id = cap.get("capsule_id", f"cap-{i}")

            # Compute max Jaccard against all other capsules
            max_overlap = 0.0
            for j, other in enumerate(capsules):
                if i == j:
                    continue
                ov = jaccard(kw, other.get("trigger_keywords", []))
                if ov > max_overlap:
                    max_overlap = ov

            novelty = 1.0 - max_overlap
            results.append(CapsuleCredibility(
                capsule_id=cap_id,
                gap_title=cap.get("action_gap_title", ""),
                gap_type=cap.get("action_gap_type", ""),
                outcome_score=cap.get("outcome_success_score", 0.0),
                novelty_score=round(novelty, 3),
                max_overlap=round(max_overlap, 3),
                is_trendslop=max_overlap > TRENDSLOP_THRESHOLD,
                trigger_keywords=kw,
            ))

        # Sort: most original first
        results.sort(key=lambda x: -x.novelty_score)
        self._credibility = results
        return results

    def get_trendslop_capsules(self) -> List[CapsuleCredibility]:
        """Return only capsules flagged as trendslop."""
        return [c for c in self.compute_credibility() if c.is_trendslop]

    def get_all_credibility(self) -> List[CapsuleCredibility]:
        """Return all capsules sorted by novelty desc."""
        return self.compute_credibility()

    # ── HTML rendering ─────────────────────────────────────────────────

    def render_html(self) -> str:
        """Render credibility scores as an HTML fragment for the web UI."""
        capsules = self.get_all_credibility()
        if not capsules:
            return "<p>No capsules yet. Create some capsules first.</p>"

        trendslop_count = len(self.get_trendslop_capsules())

        lines = ['<div class="credibility-panel">']
        lines.append(f"<h3>Gap Credibility Scores <small style='color:#888'>({len(capsules)} capsules, {trendslop_count} trendslop)</small></h3>")
        lines.append('<table class="credibility-table">')
        lines.append("<thead><tr>"
                     "<th>Gap Title</th>"
                     "<th>Type</th>"
                     "<th>Outcome</th>"
                     "<th>Novelty</th>"
                     "<th>Max Overlap</th>"
                     "<th>Status</th>"
                     "</tr></thead>")
        lines.append("<tbody>")

        for c in capsules:
            novelty_pct = int(c.novelty_score * 100)
            color = "#C4706A" if c.is_trendslop else "#7A9E7A"
            badge = '<span style="background:#C4706A;color:white;padding:2px 8px;border-radius:10px;font-size:11px">⚠️ TRENDSLOP</span>' if c.is_trendslop else '<span style="background:#7A9E7A;color:white;padding:2px 8px;border-radius:10px;font-size:11px">✓ Original</span>'
            lines.append(f"<tr>")
            lines.append(f"<td style='max-width:260px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap'><code title='{c.gap_title}'>{c.gap_title[:40]}</code></td>")
            lines.append(f"<td><code>{c.gap_type}</code></td>")
            lines.append(f"<td>{c.outcome_score:.2f}</td>")
            lines.append(f"<td>{novelty_pct}%</td>")
            lines.append(f"<td>{int(c.max_overlap * 100)}%</td>")
            lines.append(f"<td>{badge}</td>")
            lines.append(f"</tr>")

        lines.append("</tbody></table>")
        lines.append("<style>")
        lines.append(".credibility-panel { font-family: Georgia, serif; }")
        lines.append(".credibility-table { width: 100%; border-collapse: collapse; margin-top: 1rem; }")
        lines.append(".credibility-table th, .credibility-table td { padding: 0.4rem 0.8rem; border-bottom: 1px solid #e8e4de; text-align: left; }")
        lines.append(".credibility-table th { font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em; color: #7a7570; }")
        lines.append("</style>")
        lines.append("</div>")
        return "\n".join(lines)
