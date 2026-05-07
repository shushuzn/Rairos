"""Source trust tracking per arXiv category for Gene Pool capsules."""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional

from llm.insight.gene import CapsuleGene


TRUST_FILE = "source_trust.json"
DEFAULT_TRUST = 0.5
TRUST_LEARN_RATE = 0.1  # how fast trust adjusts per new capsule


@dataclass
class SourceTrustEntry:
    """Trust score for a single arXiv category."""

    category: str
    trust_score: float  # 0.0–1.0
    capsule_count: int
    avg_success_score: float
    avg_feedback_count: float
    acceptance_rate: float  # accepts / (accepts + rejects)
    last_updated: str

    def to_dict(self) -> Dict[str, Any]:
        return {
            "category": self.category,
            "trust_score": round(self.trust_score, 3),
            "capsule_count": self.capsule_count,
            "avg_success_score": round(self.avg_success_score, 3),
            "avg_feedback_count": round(self.avg_feedback_count, 1),
            "acceptance_rate": round(self.acceptance_rate, 3),
            "last_updated": self.last_updated,
        }

    @classmethod
    def from_dict(cls, d: Dict[str, Any]) -> SourceTrustEntry:
        return cls(
            category=d.get("category", ""),
            trust_score=d.get("trust_score", DEFAULT_TRUST),
            capsule_count=d.get("capsule_count", 0),
            avg_success_score=d.get("avg_success_score", 0.0),
            avg_feedback_count=d.get("avg_feedback_count", 0.0),
            acceptance_rate=d.get("acceptance_rate", 0.0),
            last_updated=d.get("last_updated", ""),
        )


class SourceTrustTracker:
    """Track trust scores per arXiv category based on capsule quality.

    Data stored at: {data_dir}/source_trust.json

    Trust formula per category:
      base = avg_success_score * 0.4 + min(capsule_count / 20, 1.0) * 0.3
             + acceptance_rate * 0.3
      trust = decay * base + (1 - decay) * DEFAULT_TRUST

    Where decay = min(capsule_count / 10, 1.0) — stabilizes with more data.
    """

    def __init__(self, data_dir: Optional[Path] = None):
        self.data_dir = data_dir or Path.home() / ".ai_research_os" / "evolution"
        self.data_dir.mkdir(parents=True, exist_ok=True)
        self._trust_file = self.data_dir / TRUST_FILE
        self._trust_data: Dict[str, SourceTrustEntry] = {}
        self._load()

    def _load(self) -> None:
        if self._trust_file.exists():
            try:
                with open(self._trust_file, "r", encoding="utf-8") as f:
                    data = json.load(f)
                for cat, entry in data.items():
                    self._trust_data[cat] = SourceTrustEntry.from_dict(entry)
            except Exception:
                self._trust_data = {}

    def _save(self) -> None:
        data = {cat: entry.to_dict() for cat, entry in self._trust_data.items()}
        with open(self._trust_file, "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)

    def get_trust(self, category: str) -> float:
        """Get trust score for a category (defaults to 0.5)."""
        entry = self._trust_data.get(category)
        return entry.trust_score if entry else DEFAULT_TRUST

    def get_all_trusts(self) -> Dict[str, float]:
        """Get {category → trust_score} for all tracked categories."""
        return {cat: entry.trust_score for cat, entry in self._trust_data.items()}

    def get_all_entries(self) -> Dict[str, SourceTrustEntry]:
        """Get all detailed entries."""
        return dict(self._trust_data)

    def update_from_capsule(
        self,
        capsule: CapsuleGene,
        events_data: Optional[Dict[str, Any]] = None,
    ) -> None:
        """Update trust data based on a single capsule."""
        category = capsule.archetype.get("source_arxiv_category", "")
        if not category:
            return

        entry = self._trust_data.get(category)
        if entry is None:
            entry = SourceTrustEntry(
                category=category,
                trust_score=DEFAULT_TRUST,
                capsule_count=0,
                avg_success_score=0.0,
                avg_feedback_count=0.0,
                acceptance_rate=0.0,
                last_updated=datetime.now().isoformat(),
            )

        # Rolling averages
        n = entry.capsule_count
        entry.avg_success_score = (entry.avg_success_score * n + capsule.outcome_success_score) / (
            n + 1
        )
        entry.avg_feedback_count = (entry.avg_feedback_count * n + capsule.feedback_count) / (n + 1)
        entry.capsule_count = n + 1
        entry.last_updated = datetime.now().isoformat()

        # Acceptance rate from events data if available
        if events_data:
            accepts = events_data.get("accepts", 0)
            rejects = events_data.get("rejects", 0)
            total = accepts + rejects
            entry.acceptance_rate = accepts / total if total > 0 else entry.acceptance_rate

        # Recompute trust score
        decay = min(entry.capsule_count / 10.0, 1.0)
        base = (
            entry.avg_success_score * 0.4
            + min(entry.capsule_count / 20.0, 1.0) * 0.3
            + entry.acceptance_rate * 0.3
        )
        entry.trust_score = decay * base + (1 - decay) * DEFAULT_TRUST
        entry.trust_score = max(0.0, min(1.0, entry.trust_score))

        self._trust_data[category] = entry
        self._save()

    def batch_update(self, capsules: List[CapsuleGene]) -> None:
        """Recalculate trust from all capsules."""
        categories: Dict[str, List[CapsuleGene]] = {}
        for c in capsules:
            cat = c.archetype.get("source_arxiv_category", "")
            if cat:
                categories.setdefault(cat, []).append(c)

        for cat, cat_capsules in categories.items():
            n = len(cat_capsules)
            avg_score = sum(c.outcome_success_score for c in cat_capsules) / n
            avg_fb = sum(c.feedback_count for c in cat_capsules) / n

            decay = min(n / 10.0, 1.0)
            base = avg_score * 0.4 + min(n / 20.0, 1.0) * 0.3 + 0.5 * 0.3
            trust = decay * base + (1 - decay) * DEFAULT_TRUST

            self._trust_data[cat] = SourceTrustEntry(
                category=cat,
                trust_score=max(0.0, min(1.0, trust)),
                capsule_count=n,
                avg_success_score=round(avg_score, 3),
                avg_feedback_count=round(avg_fb, 1),
                acceptance_rate=0.5,
                last_updated=datetime.now().isoformat(),
            )

        self._save()

    def render_html(self) -> str:
        """Render trust scores as HTML fragment for the web UI."""
        entries = [e for e in self._trust_data.values() if e.capsule_count >= 1]
        if not entries:
            return "<p>No trust data yet. Import papers and create capsules first.</p>"

        entries.sort(key=lambda e: e.trust_score, reverse=True)

        lines = ['<div class="trust-panel">']
        lines.append("<h3>Source Trust Scores</h3>")
        lines.append(
            "<p style='color:#666;font-size:13px;'>"
            "Per-arXiv-category trust ratings based on capsule quality history. "
            "Categories with more high-quality capsules earn higher trust.</p>"
        )

        lines.append('<table class="trust-table">')
        lines.append(
            "<thead><tr>"
            "<th>Category</th>"
            "<th>Trust Score</th>"
            "<th>Capsules</th>"
            "<th>Avg Score</th>"
            "<th>Avg Feedback</th>"
            "</tr></thead>"
        )
        lines.append("<tbody>")

        for e in entries:
            bar_width = int(e.trust_score * 100)
            color = "#7A9E7A" if e.trust_score >= 0.5 else "#C4706A"
            lines.append("<tr>")
            lines.append(f"<td><code>{e.category}</code></td>")
            lines.append(
                f'<td><div class="trust-bar" style="width:{bar_width}%;background:{color};padding:2px 6px;min-width:40px">{e.trust_score:.2f}</div></td>'
            )
            lines.append(f"<td>{e.capsule_count}</td>")
            lines.append(f"<td>{e.avg_success_score:.2f}</td>")
            lines.append(f"<td>{e.avg_feedback_count:.1f}</td>")
            lines.append("</tr>")

        lines.append("</tbody></table>")
        lines.append("<style>")
        lines.append(".trust-panel { font-family: Georgia, serif; }")
        lines.append(".trust-table { width: 100%; border-collapse: collapse; margin-top: 1rem; }")
        lines.append(
            ".trust-table th, .trust-table td { padding: 0.4rem 0.8rem; border-bottom: 1px solid #e8e4de; text-align: left; font-size: 13px; }"
        )
        lines.append(
            ".trust-table th { font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em; color: #7a7570; }"
        )
        lines.append(
            ".trust-bar { height: 1.4em; border-radius: 4px; font-size: 0.8rem; color: white; display: inline-block; text-align: center; }"
        )
        lines.append("</style>")
        lines.append("</div>")
        return "\n".join(lines)

    def render_trust_table(self, min_capsules: int = 1) -> str:
        """Render a human-readable trust table."""
        entries = [e for e in self._trust_data.values() if e.capsule_count >= min_capsules]
        if not entries:
            return "No source trust data yet. Import papers with arXiv categories to build trust scores."

        entries.sort(key=lambda e: e.trust_score, reverse=True)

        lines = ["=== Source Trust Scores ===", ""]
        lines.append(
            f"  {'Category':<25} {'Trust':<8} {'Count':<6} {'Avg Score':<10} {'Accept Rate':<12}"
        )
        lines.append(f"  {'─' * 25} {'─' * 8} {'─' * 6} {'─' * 10} {'─' * 12}")

        for e in entries:
            bar_len = int(e.trust_score * 10)
            bar = "█" * bar_len + "░" * (10 - bar_len)
            lines.append(
                f"  {e.category:<25} {bar} {e.trust_score:.2f}  "
                f"{e.capsule_count:<4}  {e.avg_success_score:.2f}      "
                f"{e.acceptance_rate:.2f}"
            )

        lines.append("")
        return "\n".join(lines)
