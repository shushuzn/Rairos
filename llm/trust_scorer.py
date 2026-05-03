"""Gene Pool Source Trust Scorer.

Computes per-arXiv-category trust scores based on capsule quality history.
Trust map is used to filter paper imports and rate capsule credibility.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, List, Optional
from dataclasses import dataclass


TRUST_MAP_PATH = Path.home() / ".ai_research_os" / "trust_map.json"
TRUST_THRESHOLD = 0.5  # outcome_success_score above this = "trusted" capsule


@dataclass
class CategoryTrust:
    category: str
    total_capsules: int
    trusted_capsules: int
    avg_score: float
    trust_ratio: float  # trusted / total

    def to_dict(self) -> Dict[str, Any]:
        return {
            "category": self.category,
            "total_capsules": self.total_capsules,
            "trusted_capsules": self.trusted_capsules,
            "avg_score": round(self.avg_score, 3),
            "trust_ratio": round(self.trust_ratio, 3),
        }


class TrustScorer:
    """Compute and query per-arXiv-category trust scores from Gene Pool history."""

    def __init__(self, db=None):
        self.db = db
        self._trust_map: Optional[Dict[str, CategoryTrust]] = None

    # ── Core computation ────────────────────────────────────────────────

    def compute_trust_map(self, force: bool = False) -> Dict[str, CategoryTrust]:
        """Compute trust scores for all arXiv categories from capsule history.

        A category's trust_ratio = fraction of its capsules with
        outcome_success_score >= 0.5.
        """
        if self._trust_map is not None and not force:
            return self._trust_map

        capsule_path = Path.home() / ".ai_research_os" / "gene_pool" / "capsules.json"
        if not capsule_path.exists():
            self._trust_map = {}
            return self._trust_map

        data = json.loads(capsule_path.read_text(encoding="utf-8"))
        capsules = data.get("capsules", [])

        # Group capsules by source paper category
        by_category: Dict[str, List[Dict]] = {}

        for cap in capsules:
            if cap.get("status") == "archived":
                continue
            src = cap.get("archetype", {}).get("source_paper_id", "")
            if not src:
                continue

            cat = self._get_paper_category(src)
            if not cat:
                continue

            by_category.setdefault(cat, []).append(cap)

        # Compute trust per category
        trust_map: Dict[str, CategoryTrust] = {}
        for cat, caps in by_category.items():
            scores = [c.get("outcome_success_score", 0) for c in caps]
            trusted = sum(1 for s in scores if s >= TRUST_THRESHOLD)
            avg = sum(scores) / len(scores) if scores else 0
            trust_map[cat] = CategoryTrust(
                category=cat,
                total_capsules=len(caps),
                trusted_capsules=trusted,
                avg_score=avg,
                trust_ratio=trusted / len(caps) if caps else 0,
            )

        self._trust_map = trust_map
        return trust_map

    def _get_paper_category(self, paper_id: str) -> Optional[str]:
        """Look up a paper's primary_category from DB."""
        if not self.db:
            try:
                from db.database import Database
                self.db = Database()
                self.db.init()
            except Exception:
                return None

        try:
            paper = self.db.get_paper(paper_id)
            if paper:
                cat = getattr(paper, "primary_category", "") or ""
                if not cat:
                    # Fall back to parsed categories field
                    cats = getattr(paper, "categories", "") or ""
                    if cats:
                        cat = cats.split(",")[0].strip()
                return cat or None
        except Exception:
            pass
        return None

    def get_category_trust(self, category: str) -> Optional[CategoryTrust]:
        """Get trust for a specific category (cached after first compute)."""
        if self._trust_map is None:
            self.compute_trust_map()
        return self._trust_map.get(category)

    def get_all_trust(self) -> List[CategoryTrust]:
        """Get all category trust scores, sorted by trust_ratio desc."""
        if self._trust_map is None:
            self.compute_trust_map()
        return sorted(self._trust_map.values(), key=lambda x: -x.trust_ratio)

    def is_trusted_category(self, category: str, threshold: float = 0.5) -> bool:
        """Check if a category has trust_ratio above threshold."""
        trust = self.get_category_trust(category)
        if not trust:
            return False  # Unknown category = untrusted by default
        return trust.trust_ratio >= threshold

    # ── Persistence ─────────────────────────────────────────────────

    def save_trust_map(self) -> bool:
        """Save computed trust map to disk for fast loading."""
        if self._trust_map is None:
            return False
        TRUST_MAP_PATH.parent.mkdir(parents=True, exist_ok=True)
        data = {cat: t.to_dict() for cat, t in self._trust_map.items()}
        TRUST_MAP_PATH.write_text(
            json.dumps({"version": 1, "computed_at": str(Path(__file__).stat().st_mtime), "map": data}),
            encoding="utf-8",
        )
        return True

    def load_trust_map(self) -> bool:
        """Load pre-computed trust map from disk."""
        if not TRUST_MAP_PATH.exists():
            return False
        try:
            data = json.loads(TRUST_MAP_PATH.read_text(encoding="utf-8"))
            self._trust_map = {
                cat: CategoryTrust(**v) for cat, v in data.get("map", {}).items()
            }
            return True
        except Exception:
            return False

    # ── HTML rendering ────────────────────────────────────────────────

    def render_html(self) -> str:
        """Render trust scores as an HTML fragment for the web UI."""
        trusts = self.get_all_trust()
        if not trusts:
            return "<p>No trust data yet. Import papers and create capsules first.</p>"

        lines = ['<div class="trust-panel">']
        lines.append("<h3>Source Trust Scores</h3>")
        lines.append('<table class="trust-table">')
        lines.append("<thead><tr><th>Category</th><th>Capsules</th><th>Trusted</th><th>Avg Score</th><th>Trust Ratio</th></tr></thead>")
        lines.append("<tbody>")

        for t in trusts:
            bar_width = int(t.trust_ratio * 100)
            color = "#7A9E7A" if t.trust_ratio >= 0.5 else "#C4706A"
            lines.append("<tr>")
            lines.append(f"<td><code>{t.category}</code></td>")
            lines.append(f"<td>{t.total_capsules}</td>")
            lines.append(f"<td>{t.trusted_capsules}</td>")
            lines.append(f"<td>{t.avg_score:.2f}</td>")
            lines.append(f'<td><div class="trust-bar" style="width:{bar_width}%;background:{color}">{t.trust_ratio:.0%}</div></td>')
            lines.append("</tr>")

        lines.append("</tbody></table>")
        lines.append("<style>")
        lines.append(".trust-panel { font-family: Georgia, serif; }")
        lines.append(".trust-table { width: 100%; border-collapse: collapse; margin-top: 1rem; }")
        lines.append(".trust-table th, .trust-table td { padding: 0.4rem 0.8rem; border-bottom: 1px solid #e8e4de; text-align: left; }")
        lines.append(".trust-table th { font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em; color: #7a7570; }")
        lines.append(".trust-bar { height: 1.2em; border-radius: 4px; font-size: 0.75rem; color: white; padding: 0 0.4em; min-width: 3em; display: inline-block; }")
        lines.append("</style>")
        lines.append("</div>")
        return "\n".join(lines)
