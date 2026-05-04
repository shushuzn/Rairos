"""Capsule Review Queue — new capsules pending first feedback.

Capsules enter the queue when:
  - status is empty/active AND
  - has never received a verdict (no feedback entry in feedback store)
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, List, Optional

CAPSULES_PATH = Path.home() / ".ai_research_os" / "gene_pool" / "capsules.json"
FEEDBACK_DIR = Path.home() / ".ai_research_os" / "insights"


@dataclass
class QueuedCapsule:
    capsule_id: str
    gap_title: str
    gap_type: str
    polarity: str
    trigger_keywords: List[str]
    outcome_score: float
    source_paper_id: Optional[str]
    created_days_ago: int


def _load_capsules() -> List[Dict[str, Any]]:
    if not CAPSULES_PATH.exists():
        return []
    return json.loads(CAPSULES_PATH.read_text(encoding="utf-8")).get("capsules", [])


def _load_feedback() -> Dict[str, Any]:
    path = FEEDBACK_DIR / "feedback.json"
    if not path.exists():
        return {}
    return json.loads(path.read_text(encoding="utf-8"))


def _days_ago(ts: str) -> int:
    from datetime import datetime

    try:
        dt = datetime.fromisoformat(ts.replace("Z", "+00:00"))
        return (datetime.now() - dt.replace(tzinfo=None)).days
    except Exception:
        return 0


def get_review_queue() -> List[QueuedCapsule]:
    capsules = _load_capsules()
    feedback = _load_feedback()

    # Build set of capsules with feedback
    verdicted = set()
    for entry in feedback.values():
        if isinstance(entry, dict) and entry.get("verdict"):
            cid = entry.get("capsule_id", "")
            if cid:
                verdicted.add(cid[:12] if len(cid) >= 12 else cid)

    results: List[QueuedCapsule] = []
    for cap in capsules:
        status = cap.get("status", "")
        if status not in ("", "active"):
            continue

        cid = cap.get("capsule_id", "")
        # Check if this capsule has received feedback
        cid_short = cid[:12] if len(cid) >= 12 else cid
        if cid_short in verdicted:
            continue

        created = cap.get("created_at", "")
        results.append(
            QueuedCapsule(
                capsule_id=cid,
                gap_title=cap.get("action_gap_title", ""),
                gap_type=cap.get("action_gap_type", ""),
                polarity=cap.get("polarity", "positive"),
                trigger_keywords=cap.get("trigger_keywords", [])[:5],
                outcome_score=cap.get("outcome_success_score", 0.0),
                source_paper_id=cap.get("source_paper_id"),
                created_days_ago=_days_ago(created),
            )
        )

    results.sort(key=lambda x: x.created_days_ago, reverse=True)
    return results


def render_review_queue_html(queue: Optional[List[QueuedCapsule]] = None) -> str:
    if queue is None:
        queue = get_review_queue()

    lines = ['<div class="review-queue">']
    lines.append("<h3>📋 Capsule Review Queue</h3>")

    if not queue:
        lines.append(
            "<p style='font-size:14px;color:#A89E8C'>All capsules reviewed! 🎉 Check back after extracting gaps from new papers.</p>"
        )
    else:
        lines.append(
            f"<p style='font-size:13px;color:#A89E8C;margin-bottom:16px'>{len(queue)} capsules pending review</p>"
        )
        for c in queue:
            age_str = f"{c.created_days_ago}d ago" if c.created_days_ago > 0 else "today"
            kw_str = ", ".join(f"<code>{kw}</code>" for kw in c.trigger_keywords[:4])
            lines.append(f"""
<div style="border: 1px solid #e0dbd4; border-radius: 6px; padding: 14px; margin-bottom: 12px; background: rgba(107,143,181,0.04);">
  <div style="display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 6px;">
    <div>
      <span style="font-size: 10px; background: var(--pen-blue); color: white; padding: 1px 6px; border-radius: 2px; margin-right: 6px;">{c.gap_type}</span>
      <span style="font-size: 10px; color: #A89E8C; margin-left: 4px;">{c.polarity}</span>
    </div>
    <span style="font-size: 11px; color: #A89E8C;">{age_str}</span>
  </div>
  <div style="font-size: 14px; font-weight: 600; color: #2a2a2a; margin-bottom: 6px; line-height: 1.4;">{c.gap_title[:80]}</div>
  <div style="font-size: 11px; color: #7a7570; margin-bottom: 8px;">{kw_str}</div>
  <div style="display: flex; gap: 8px; align-items: center;">
    <button onclick="submitVerdict('{c.capsule_id}', 'match')"
      style="background: #6B8FB5; color: white; border: none; border-radius: 4px; padding: 5px 14px; cursor: pointer; font-size: 12px;">
      ✅ Match
    </button>
    <button onclick="submitVerdict('{c.capsule_id}', 'partial')"
      style="background: transparent; color: #D4A055; border: 1px solid #D4A055; border-radius: 4px; padding: 5px 14px; cursor: pointer; font-size: 12px;">
      ⚠️ Partial
    </button>
    <button onclick="submitVerdict('{c.capsule_id}', 'not_relevant')"
      style="background: transparent; color: #A89E8C; border: 1px solid #ccc; border-radius: 4px; padding: 5px 14px; cursor: pointer; font-size: 12px;">
      ❌ Not Relevant
    </button>
  </div>
</div>""")

    lines.append("""
<script>
function submitVerdict(capsuleId, verdict) {
    fetch('/insights/queue/verdict', {
        method: 'POST',
        headers: {'Content-Type': 'application/json'},
        body: JSON.stringify({capsule_id: capsuleId, verdict: verdict})
    }).then(r => r.json()).then(d => {
        if (d.success) location.reload();
        else alert('Error: ' + (d.error || 'unknown'));
    });
}
</script>""")

    lines.append("<style>")
    lines.append(".review-queue { font-family: Georgia, serif; }")
    lines.append("</style>")
    lines.append("</div>")
    return "\n".join(lines)
