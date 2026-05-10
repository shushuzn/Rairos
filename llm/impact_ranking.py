"""Impact ranking — compute and render paper impact scores from the database."""

from __future__ import annotations

from typing import Any, Dict, List


def compute_impact(db: Any) -> List[Dict[str, Any]]:
    """
    Compute impact ranking for all papers in the database.

    Impact score = citation_count (from CrossRef/OpenAlex) + reference_count.

    Returns a list of dicts sorted by impact_score descending.
    """
    if db is None:
        return []

    try:
        cur = db.conn.cursor()
        cur.execute(
            """
            SELECT id, title, citation_count, reference_count, published, abs_url
            FROM papers
            WHERE title IS NOT NULL AND title != ''
            ORDER BY (COALESCE(citation_count, 0) + COALESCE(reference_count, 0)) DESC
            LIMIT 100
            """,
        )
        results = []
        for row in cur.fetchall():
            citations = row[2] if row[2] is not None else 0
            references = row[3] if row[3] is not None else 0
            results.append(
                {
                    "paper_id": row[0],
                    "title": row[1],
                    "citation_count": citations,
                    "reference_count": references,
                    "impact_score": citations + references,
                    "published": row[4] or "",
                    "abs_url": row[5] or "",
                }
            )
        return results
    except Exception:
        return []


def render_impact_html(data: List[Dict[str, Any]]) -> str:
    """Render impact ranking as an HTML table."""
    if not data:
        return "<p>No impact data available.</p>"

    rows = []
    for i, item in enumerate(data[:20], 1):
        score = item.get("impact_score", 0)
        title = item.get("title", "Unknown")[:70]
        pub = item.get("published", "")[:4]
        rows.append(
            f'<tr>'
            f'<td style="text-align:center">{i}</td>'
            f'<td><a href="{item.get("abs_url", "#")}">{title}</a></td>'
            f'<td style="text-align:center">{pub}</td>'
            f'<td style="text-align:right;font-weight:600">{score}</td>'
            f'</tr>'
        )

    return (
        '<table style="width:100%;border-collapse:collapse;font-size:14px">'
        '<thead><tr style="background:#f5f5f5">'
        '<th style="padding:8px 12px;text-align:center">#</th>'
        '<th style="padding:8px 12px;text-align:left">Title</th>'
        '<th style="padding:8px 12px;text-align:center">Year</th>'
        '<th style="padding:8px 12px;text-align:right">Impact</th>'
        '</tr></thead>'
        '<tbody>' + "".join(rows) + '</tbody>'
        '</table>'
    )
