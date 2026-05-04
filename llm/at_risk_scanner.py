"""At-Risk Capsule Scanner.

Shows capsules approaching auto-archive threshold (low_score_streak >= 2).
Supports keep-active (reset streak) and pin-to-TTL operations.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional


CAPSULE_PATH = Path.home() / ".ai_research_os" / "gene_pool" / "capsules.json"
STREAK_THRESHOLD = 2  # at-risk: low_score_streak >= 2


@dataclass
class AtRiskCapsule:
    capsule_id: str
    gap_title: str
    gap_type: str
    outcome_score: float
    low_score_streak: int
    status: str
    pinned_ttl: int = 0
    trigger_keywords: List[str] = field(default_factory=list)


def _load_capsules() -> List[Dict[str, Any]]:
    if not CAPSULE_PATH.exists():
        return []
    data = json.loads(CAPSULE_PATH.read_text(encoding="utf-8"))
    return data.get("capsules", [])


def _save_capsules(capsules: List[Dict[str, Any]]) -> None:
    CAPSULE_PATH.parent.mkdir(parents=True, exist_ok=True)
    CAPSULE_PATH.write_text(
        json.dumps({"capsules": capsules}, indent=2, ensure_ascii=False),
        encoding="utf-8",
    )


def get_at_risk_capsules(threshold: int = STREAK_THRESHOLD) -> List[AtRiskCapsule]:
    """Return active capsules with low_score_streak >= threshold."""
    all_caps = _load_capsules()
    results = []
    for cap in all_caps:
        if cap.get("status") not in ("active", ""):
            continue
        streak = cap.get("low_score_streak", 0)
        if streak < threshold:
            continue
        results.append(AtRiskCapsule(
            capsule_id=cap.get("capsule_id", ""),
            gap_title=cap.get("action_gap_title", ""),
            gap_type=cap.get("action_gap_type", ""),
            outcome_score=cap.get("outcome_success_score", 0.0),
            low_score_streak=streak,
            status=cap.get("status", "active"),
            pinned_ttl=cap.get("pinned_ttl", 0),
            trigger_keywords=cap.get("trigger_keywords", []),
        ))
    results.sort(key=lambda x: -x.low_score_streak)
    return results


def keep_active(capsule_id: str) -> bool:
    """Reset low_score_streak to 0 for a capsule."""
    capsules = _load_capsules()
    found = False
    for cap in capsules:
        if cap.get("capsule_id") == capsule_id:
            cap["low_score_streak"] = 0
            cap["pinned_ttl"] = 0
            found = True
            break
    if not found:
        return False
    _save_capsules(capsules)
    return True


def pin_to_ttl(capsule_id: str, ttl: int = 3) -> bool:
    """Pin a capsule to TTL cycles (resets streak, sets pinned_ttl)."""
    capsules = _load_capsules()
    found = False
    for cap in capsules:
        if cap.get("capsule_id") == capsule_id:
            cap["pinned_ttl"] = ttl
            cap["low_score_streak"] = 0
            found = True
            break
    if not found:
        return False
    _save_capsules(capsules)
    return True


def render_html(capsules: Optional[List[AtRiskCapsule]] = None) -> str:
    if capsules is None:
        capsules = get_at_risk_capsules()

    if not capsules:
        return "<p>No at-risk capsules. All capsules are healthy.</p>"

    lines = ['<div class="at-risk-panel">']
    lines.append(f"<h3>🚨 At-Risk Capsules <small style='color:#888'>({len(capsules)} need attention)</small></h3>")
    lines.append('<table class="at-risk-table">')
    lines.append("<thead><tr>"
                 "<th>Gap Title</th>"
                 "<th>Type</th>"
                 "<th>Score</th>"
                 "<th>Streak</th>"
                 "<th>Pinned</th>"
                 "<th>Action</th>"
                 "</tr></thead>")
    lines.append("<tbody>")

    for cap in capsules:
        streak_bar = "🔴" * cap.low_score_streak
        pinned = f"TTL {cap.pinned_ttl}" if cap.pinned_ttl > 0 else "—"
        lines.append("<tr>")
        lines.append(f"<td style='max-width:220px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap'><code title='{cap.gap_title}'>{cap.gap_title[:35]}</code></td>")
        lines.append(f"<td><code>{cap.gap_type}</code></td>")
        lines.append(f"<td>{cap.outcome_score:.2f}</td>")
        lines.append(f"<td>{streak_bar} <small>{cap.low_score_streak}</small></td>")
        lines.append(f"<td>{pinned}</td>")
        lines.append("<td>")
        lines.append(f'<button class="btn btn-small btn-keep" onclick="keepActive(\'{cap.capsule_id}\')">✓ Keep Active</button>')
        lines.append(f'<button class="btn btn-small btn-pin" onclick="pinToTTL(\'{cap.capsule_id}\')">📌 Pin TTL</button>')
        lines.append("</td>")
        lines.append("</tr>")

    lines.append("</tbody></table>")
    lines.append("<style>")
    lines.append(".at-risk-panel { font-family: Georgia, serif; }")
    lines.append(".at-risk-table { width: 100%; border-collapse: collapse; margin-top: 1rem; }")
    lines.append(".at-risk-table th, .at-risk-table td { padding: 0.4rem 0.8rem; border-bottom: 1px solid #e8e4de; text-align: left; }")
    lines.append(".at-risk-table th { font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em; color: #7a7570; }")
    lines.append(".btn-small { padding: 3px 10px; font-size: 12px; border-radius: 4px; cursor: pointer; }")
    lines.append(".btn-keep { background: #7A9E7A; color: white; border: none; }")
    lines.append(".btn-pin { background: #6B8FB5; color: white; border: none; margin-left: 4px; }")
    lines.append("</style>")
    lines.append("</div>")
    return "\n".join(lines)
