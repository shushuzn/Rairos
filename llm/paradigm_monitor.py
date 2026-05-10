"""Paradigm Concentration Monitor.

Detects when >60% of citations in a domain cluster around ≤3 references.
Flags a generalization_gap risk alert.
"""

from __future__ import annotations

import sqlite3
from pathlib import Path
from typing import Any, Dict, List


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
    """
    Check for paradigm concentration in the Gene Pool.

    Detects when >60% of citations are concentrated in ≤3 papers,
    which signals a generalization_gap risk (the domain relies too heavily
    on a small number of foundational references).

    Returns {"categories": [...], "alerts": [...]}.
    """
    try:
        conn = _get_db()
        # Get all papers with non-zero citation_count
        if category == "all":
            rows = conn.execute(
                "SELECT id, title, primary_category, citation_count "
                "FROM papers WHERE citation_count > 0 ORDER BY citation_count DESC"
            ).fetchall()
        else:
            rows = conn.execute(
                "SELECT id, title, primary_category, citation_count "
                "FROM papers WHERE primary_category = ? AND citation_count > 0 "
                "ORDER BY citation_count DESC",
                (category,),
            ).fetchall()

        if not rows:
            return {"categories": [], "alerts": []}

        total_citations = sum(r[3] for r in rows if r[3])

        if total_citations == 0:
            return {"categories": [], "alerts": []}

        top_n = rows[:TOP_N]
        top_n_citations = sum(r[3] for r in top_n if r[3])
        concentration = top_n_citations / total_citations

        categories = [
            {
                "category": r[2] or "uncategorized",
                "paper_id": r[0],
                "title": (r[1] or "Unknown")[:80],
                "citation_count": r[3],
                "share_pct": round(100 * r[3] / total_citations, 1),
            }
            for r in top_n
        ]

        alerts: List[Dict[str, Any]] = []
        if concentration > ALERT_THRESHOLD:
            alerts.append({
                "type": "paradigm_concentration",
                "severity": "high",
                "message": (
                    f"{round(concentration * 100)}% of citations in "
                    f"{category!r} domain cluster around {TOP_N} papers "
                    f"(threshold: {round(ALERT_THRESHOLD * 100)}%). "
                    "Consider diversifying reading to reduce generalization gap risk."
                ),
                "top_papers": [r[0] for r in top_n],
            })

        return {"categories": categories, "alerts": alerts, "total_papers": len(rows), "total_citations": total_citations}
    except Exception:
        return {"categories": [], "alerts": [], "error": "db unavailable"}
    finally:
        try:
            conn.close()
        except Exception:
            pass


def render_html(data: Dict[str, Any]) -> str:
    """Render paradigm concentration results as HTML."""
    if not data or "error" in data:
        return "<p>Paradigm concentration monitor temporarily unavailable</p>"

    alerts = data.get("alerts", [])
    categories = data.get("categories", [])

    if not categories and not alerts:
        return "<p>No paradigm concentration detected.</p>"

    parts = []
    if alerts:
        for alert in alerts:
            parts.append(
                f'<div style="background:#fff3cd;border:1px solid #ffc107;'
                f'border-radius:6px;padding:12px;margin-bottom:12px;font-size:13px">'
                f'<strong>⚠️ Paradigm Concentration Alert</strong><br>'
                f'{alert.get("message", "")}'
                f'</div>'
            )

    if categories:
        rows = []
        for i, cat in enumerate(categories, 1):
            rows.append(
                f'<tr><td style="padding:6px 12px">{i}</td>'
                f'<td style="padding:6px 12px">{cat.get("title","")}</td>'
                f'<td style="padding:6px 12px;text-align:center">{cat.get("share_pct","")}%</td></tr>'
            )
        parts.append(
            '<table style="width:100%;border-collapse:collapse;font-size:13px">'
            '<tr style="background:#f5f5f5">'
            '<th style="padding:8px 12px">#</th>'
            '<th style="padding:8px 12px;text-align:left">Paper</th>'
            '<th style="padding:8px 12px;text-align:center">Citation Share</th>'
            '</tr>' + "".join(rows) + '</table>'
        )

    return "".join(parts)
