"""Contradiction Heatmap.

Computes per-paper contradiction counts from Gene Pool capsules.
Papers are linked via archetype.source_paper_id.
"""

from __future__ import annotations

import json
from collections import defaultdict
from pathlib import Path
from typing import Any, Dict, List

from llm.gene_pool_io import load_capsules


def compute_paper_contradictions() -> Dict[str, Dict[str, Any]]:
    """Return {paper_id: {count, contradictions: [...]}} for all papers."""
    capsules = load_capsules()
    contrad = detect_contradictions(capsules)

    by_paper: Dict[str, Dict[str, Any]] = defaultdict(lambda: {"count": 0, "contradictions": []})

    for c in contrad:
        pos_cap = c.get("positive_capsule", {})
        neg_cap = c.get("negative_capsule", {})
        pos_id = pos_cap.get("archetype", {}).get("source_paper_id") or "?"
        neg_id = neg_cap.get("archetype", {}).get("source_paper_id") or "?"
        gap_type = c.get("gap_type", "")
        shared = c.get("shared_keywords", [])

        for pid, partner_id, polarity in [
            (pos_id, neg_id, "positive"),
            (neg_id, pos_id, "negative"),
        ]:
            if pid and pid != "?":
                by_paper[pid]["count"] += 1
                by_paper[pid]["contradictions"].append(
                    {
                        "gap_type": gap_type,
                        "partner_id": partner_id,
                        "polarity": polarity,
                        "shared_keywords": shared,
                    }
                )

    return dict(by_paper)


def _badge_color(count: int) -> str:
    if count == 0:
        return "#e8e4de"
    elif count == 1:
        return "#f5d76e"
    elif count == 2:
        return "#e67e22"
    else:
        return "#e74c3c"


def render_heatmap_html(
    papers: List[Dict[str, Any]], contrad_map: Dict[str, Dict[str, Any]]
) -> str:
    """Render a grid of paper cards with contradiction heat colors.

    papers: list of dicts with keys: id, title, primary_category, published
    contrad_map: {paper_id: {count, contradictions}}
    """
    if not papers:
        return "<p>No papers yet.</p>"

    lines = ['<div class="heatmap-grid">']
    for p in papers:
        pid = p.get("id", "")
        info = contrad_map.get(pid, {"count": 0, "contradictions": []})
        count = info["count"]
        bg = _badge_color(count)
        color = "#fff" if count >= 2 else "#555"

        # Build tooltip content
        tooltip_lines = []
        for c in info["contradictions"][:5]:
            tooltip_lines.append(
                f"• {c['polarity'][:3].upper()} {c['gap_type']} "
                f"(→ {c['partner_id'][:12]}...) kw={c['shared_keywords']}"
            )
        tooltip_text = " | ".join(tooltip_lines) if tooltip_lines else "No contradictions"
        title_short = (p.get("title") or "")[:60]

        lines.append(
            f'<div class="heatmap-card" '
            f'style="background:{bg};color:{color};border-color:{"#c0392b" if count >= 3 else "#bdc3c7"}" '
            f'title="{tooltip_text}">'
            f'<div class="heatmap-card-cat">{p.get("primary_category", "?")}</div>'
            f'<div class="heatmap-card-title">{title_short}</div>'
            f'<div class="heatmap-card-count">{count} 🔥</div>'
            f"</div>"
        )

    lines.append("</div>")
    lines.append("<style>")
    lines.append(
        ".heatmap-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 10px; }"
    )
    lines.append(
        ".heatmap-card { border-radius: 6px; padding: 12px; border: 1.5px solid #bdc3c7; cursor: help; transition: transform 0.1s; }"
    )
    lines.append(".heatmap-card:hover { transform: scale(1.02); z-index: 1; position: relative; }")
    lines.append(
        ".heatmap-card-cat { font-size: 10px; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 4px; opacity: 0.8; }"
    )
    lines.append(
        ".heatmap-card-title { font-size: 12px; font-weight: 600; line-height: 1.4; margin-bottom: 6px; }"
    )
    lines.append(".heatmap-card-count { font-size: 11px; font-weight: 700; text-align: right; }")
    lines.append("</style>")
    return "\n".join(lines)


def detect_contradictions(papers):
    """Proxy to the real implementation in contradiction_detector."""
    from llm.research.contradiction_detector import detect_contradictions as _real

    return _real(papers)
