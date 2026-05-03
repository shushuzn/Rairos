"""Paradigm Concentration Monitor.

Detects when >60% of citations in a domain cluster around ≤3 references.
Flags a generalization_gap risk alert.
"""

from __future__ import annotations

import sqlite3
from collections import Counter
from pathlib import Path
from typing import Any, Dict, List, Optional


DB_PATH = Path.home() / ".ai_research_os" / "papers.db"
ALERT_THRESHOLD = 0.60  # >60% concentration triggers alert
TOP_N = 3


def _get_db() -> sqlite3.Connection:
    return sqlite3.connect(DB_PATH)


def get_papers_in_domain(category: str) -> List[str]:
    """Return paper IDs in a given primary_category, or all if 'all'."""
    conn = _get_db()
    try:
        if category == "all":
            rows = conn.execute("SELECT id FROM papers").fetchall()
        else:
            rows = conn.execute(
                "SELECT id FROM papers WHERE primary_category = ?", (category,)
            ).fetchall()
        return [r[0] for r in rows]
    finally:
        conn.close()


def check_paradigm_concentration(category: str = "all") -> Dict[str, Any]:
    """Check citation concentration for a domain.

    Returns dict with:
      - total_citations: total outgoing citation edges from domain papers
      - top_papers: list of {paper_id, title, citation_count, ratio}
      - concentration_ratio: fraction of citations pointing to top-3
      - is_alert: bool (ratio > 0.60)
      - category: the checked category
    """
    paper_ids = set(get_papers_in_domain(category))
    if not paper_ids:
        return {
            "category": category,
            "total_citations": 0,
            "top_papers": [],
            "concentration_ratio": 0.0,
            "is_alert": False,
            "error": "No papers in domain.",
        }

    conn = _get_db()
    try:
        placeholders = ",".join("?" * len(paper_ids))
        query = f"""
            SELECT c.target_id, COUNT(*) as cnt, p.title
            FROM citations c
            LEFT JOIN papers p ON p.id = c.target_id
            WHERE c.source_id IN ({placeholders})
            GROUP BY c.target_id
            ORDER BY cnt DESC
        """
        rows = conn.execute(query, list(paper_ids)).fetchall()
    finally:
        conn.close()

    if not rows:
        return {
            "category": category,
            "total_citations": 0,
            "top_papers": [],
            "concentration_ratio": 0.0,
            "is_alert": False,
        }

    total = sum(r[1] for r in rows)
    top_papers = []
    for paper_id, cnt, title in rows[:TOP_N]:
        top_papers.append({
            "paper_id": paper_id,
            "title": (title or paper_id)[:80],
            "citation_count": cnt,
            "ratio": round(cnt / total, 3) if total else 0,
        })

    top3_count = sum(r[1] for r in rows[:TOP_N])
    concentration_ratio = round(top3_count / total, 3) if total else 0

    return {
        "category": category,
        "total_citations": total,
        "top_papers": top_papers,
        "concentration_ratio": concentration_ratio,
        "is_alert": concentration_ratio > ALERT_THRESHOLD,
    }


def get_all_categories() -> List[str]:
    """Return all distinct primary_categories in the DB."""
    conn = _get_db()
    try:
        rows = conn.execute(
            "SELECT DISTINCT primary_category FROM papers WHERE primary_category IS NOT NULL AND primary_category != '' ORDER BY primary_category"
        ).fetchall()
        return [r[0] for r in rows]
    finally:
        conn.close()


def render_html(result: Optional[Dict[str, Any]] = None) -> str:
    if result is None:
        result = check_paradigm_concentration("all")

    cat = result.get("category", "all")
    total = result.get("total_citations", 0)
    top = result.get("top_papers", [])
    ratio = result.get("concentration_ratio", 0)
    is_alert = result.get("is_alert", False)
    err = result.get("error")

    lines = ['<div class="paradigm-panel">']

    if err:
        lines.append(f"<p class='empty'>{err}</p></div>")
        return "\n".join(lines)

    # Alert banner
    if is_alert:
        lines.append(
            f'<div class="alert-banner" style="background:#e74c3c;color:white;padding:12px 16px;border-radius:6px;margin-bottom:16px;font-family:Georgia,serif;">'
            f'<strong style="font-size:16px">⚠️ PARADIGM CONCENTRATION ALERT</strong><br>'
            f'<span style="font-size:13px">{int(ratio*100)}% of citations point to the top {len(top)} references — field may be locked into a single paradigm.</span>'
            f'</div>'
        )
    else:
        lines.append(
            f'<div class="alert-banner" style="background:#7A9E7A;color:white;padding:12px 16px;border-radius:6px;margin-bottom:16px;font-family:Georgia,serif;">'
            f'<strong style="font-size:14px">✓ Citation landscape is diverse</strong> '
            f'<span style="font-size:13px">Top {len(top)} papers receive {int(ratio*100)}% of citations.</span>'
            f'</div>'
        )

    lines.append(f"<p style='font-size:13px;color:#7a7570;margin-bottom:16px;'>"
                f"Domain: <strong>{cat}</strong> · {total} total outgoing citations · "
                f"Concentration: <strong>{int(ratio*100)}%</strong> (alert at {int(ALERT_THRESHOLD*100)}%)</p>")

    if top:
        lines.append("<table class='paradigm-table'>")
        lines.append("<thead><tr><th>Paper</th><th>Cit.</th><th>Share</th><th>Bar</th></tr></thead>")
        lines.append("<tbody>")
        for p in top:
            bar_w = int(p["ratio"] * 100)
            lines.append(f"<tr>")
            lines.append(f"<td style='max-width:300px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap'><code title='{p['paper_id']}'>{p['title']}</code></td>")
            lines.append(f"<td style='text-align:right;font-weight:600'>{p['citation_count']}</td>")
            lines.append(f"<td style='text-align:right'>{p['ratio']:.1%}</td>")
            lines.append(f"<td style='width:120px'><div style='background:#e8e4de;border-radius:3px;height:8px'><div style='background:{'#e74c3c' if is_alert else '#7A9E7A'};height:100%;width:{bar_w}%;border-radius:3px'></div></div></td>")
            lines.append(f"</tr>")
        lines.append("</tbody></table>")

    lines.append("<style>")
    lines.append(".paradigm-panel { font-family: Georgia, serif; }")
    lines.append(".paradigm-table { width: 100%; border-collapse: collapse; margin-top: 1rem; }")
    lines.append(".paradigm-table th, .paradigm-table td { padding: 0.4rem 0.8rem; border-bottom: 1px solid #e8e4de; text-align: left; }")
    lines.append(".paradigm-table th { font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em; color: #7a7570; }")
    lines.append("</style>")
    lines.append("</div>")
    return "\n".join(lines)
