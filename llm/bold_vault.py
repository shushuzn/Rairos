"""Bold Hypothesis Vault.

Separately tracked high-risk/high-reward Gene Pool capsules.
Bold = theoretical_gap OR negative polarity OR high novelty score.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional


CAPSULE_PATH = Path.home() / ".ai_research_os" / "gene_pool" / "capsules.json"
NOVELTY_THRESHOLD = 0.7


@dataclass
class BoldCapsule:
    capsule_id: str
    gap_title: str
    gap_type: str
    polarity: str
    outcome_score: float
    novelty_score: float
    trigger_keywords: List[str]
    reason: str  # why it's bold: "theoretical", "negative", "high-novelty"


def _load_capsules() -> List[Dict[str, Any]]:
    if not CAPSULE_PATH.exists():
        return []
    data = json.loads(CAPSULE_PATH.read_text(encoding="utf-8"))  # type: ignore[no-any-return]
    return data.get("capsules", [])  # type: ignore[no-any-return]


def jaccard(a: List[str], b: List[str]) -> float:
    s_a, s_b = set(a), set(b)
    if not s_a or not s_b:
        return 0.0
    return len(s_a & s_b) / len(s_a | s_b)

_jaccard = jaccard  # backward compatibility alias


_jaccard = jaccard  # backward compatibility alias


def get_bold_capsules() -> List[BoldCapsule]:
    """Return capsules flagged as bold hypothesis (high-risk/high-reward)."""
    capsules = _load_capsules()
    results: List[BoldCapsule] = []

    for cap in capsules:
        if cap.get("status") not in ("active", ""):
            continue

        gap_type = cap.get("action_gap_type", "") or cap.get("trigger_gap_type", "")
        polarity = cap.get("polarity", "positive")
        keywords = cap.get("trigger_keywords", [])

        # Compute novelty vs other capsules
        max_overlap = 0.0
        for other in capsules:
            if other.get("capsule_id") == cap.get("capsule_id"):
                continue
            ov = jaccard(keywords, other.get("trigger_keywords", []))
            if ov > max_overlap:
                max_overlap = ov
        novelty = 1.0 - max_overlap

        reasons: List[str] = []
        if gap_type == "theoretical_gap":
            reasons.append("theoretical")
        if polarity == "negative":
            reasons.append("negative")
        if novelty > NOVELTY_THRESHOLD:
            reasons.append(f"high-novelty({novelty:.0%})")

        if not reasons:
            continue

        results.append(
            BoldCapsule(
                capsule_id=cap.get("capsule_id", ""),
                gap_title=cap.get("action_gap_title", ""),
                gap_type=gap_type,
                polarity=polarity,
                outcome_score=cap.get("outcome_success_score", 0.0),
                novelty_score=round(novelty, 3),
                trigger_keywords=keywords,
                reason=", ".join(reasons),
            )
        )

    results.sort(key=lambda x: -x.novelty_score)
    return results


def render_html(capsules: Optional[List[BoldCapsule]] = None) -> str:
    if capsules is None:
        capsules = get_bold_capsules()

    if not capsules:
        return "<p>No bold hypotheses yet. Theoretical gaps and negative-polarity capsules will appear here.</p>"

    lines = ['<div class="bold-vault">']
    lines.append(
        "<h3>🔴 Bold Hypothesis Vault <small style='color:#888'>(high-risk / high-reward gaps)</small></h3>"
    )
    lines.append(
        f"<p style='font-size:13px;color:#A89E8C;margin-bottom:16px'>{len(capsules)} bold capsules tracked.</p>"
    )
    lines.append("<div class='bold-grid'>")

    for c in capsules:
        title_short = c.gap_title[:70] if c.gap_title else "(no title)"
        kw_str = ", ".join(c.trigger_keywords[:4])
        lines.append(
            f"<div class='bold-card'>"
            f"<div class='bold-reason'>{c.reason}</div>"
            f"<div class='bold-title' title='{c.gap_title}'>{title_short}</div>"
            f"<div class='bold-meta'>"
            f"<code>{c.gap_type}</code> · {c.polarity} · score={c.outcome_score:.2f} · novelty={c.novelty_score:.0%}"
            f"</div>"
            f"<div class='bold-kw'>{kw_str}</div>"
            f"</div>"
        )

    lines.append("</div>")
    lines.append("<style>")
    lines.append(".bold-vault { font-family: Georgia, serif; }")
    lines.append(
        ".bold-card { border: 2px solid #C4706A; border-radius: 6px; padding: 12px 14px; margin-bottom: 10px; background: rgba(196,112,106,0.06); }"
    )
    lines.append(
        ".bold-reason { font-size: 10px; text-transform: uppercase; letter-spacing: 0.5px; color: #C4706A; font-weight: 700; margin-bottom: 4px; }"
    )
    lines.append(
        ".bold-title { font-size: 14px; font-weight: 600; color: #2a2a2a; margin-bottom: 6px; line-height: 1.4; }"
    )
    lines.append(".bold-meta { font-size: 11px; color: #7a7570; margin-bottom: 4px; }")
    lines.append(".bold-kw { font-size: 11px; color: #A89E8C; font-style: italic; }")
    lines.append("</style>")
    lines.append("</div>")
    return "\n".join(lines)
