"""Labor Displacement Tracker — dedicated Gene Pool filter for AI vs. human labor gaps.

Tracks papers about AI's impact on employment across cs.cyber-ph, cs.soc, and related categories.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, List, Optional

PAPERS_DB = Path.home() / ".ai_research_os" / "papers.json"

LABOR_KEYWORDS = [
    "labor displacement",
    "job displacement",
    "automation",
    "unemployment",
    "employment",
    "workforce",
    "labor market",
    "skill gap",
    "income inequality",
    "AI and jobs",
    "AI impact on employment",
    "future of work",
    "automation risk",
    "robots",
    "replacement",
    "outsourcing",
    "gig economy",
    "platform work",
    "social protection",
    "universal basic income",
    "reskilling",
    "cs.cyber-ph",
    "cs.soc",
    "econ.GN",
    "econ.GR",
]

LABOR_CATS = ["cs.cyber-ph", "cs.soc", "cs.HC", "econ.GN"]


def _load_papers() -> List[Dict[str, Any]]:
    if not PAPERS_DB.exists():
        return []
    data = json.loads(PAPERS_DB.read_text(encoding="utf-8"))
    return data.get("papers", [])


def is_labor_related(paper: Dict[str, Any]) -> bool:
    text = (paper.get("title", "") + " " + paper.get("abstract", "")).lower()
    cats = set(paper.get("categories", []) or [])
    if any(c in ["cs.cyber-ph", "cs.soc", "cs.HC", "econ.GN"] for c in cats):
        return True
    return any(kw.lower() in text for kw in LABOR_KEYWORDS)


def get_labor_papers() -> List[Dict[str, Any]]:
    return [p for p in _load_papers() if is_labor_related(p)]


def render_labor_tracker_html() -> str:
    papers = get_labor_papers()

    lines = ['<div class="labor-tracker">']
    lines.append("<h3>👷 Labor Displacement Tracker</h3>")
    lines.append(
        "<p style='font-size:13px;color:#A89E8C;margin-bottom:16px'>"
        "Papers about AI's impact on employment, workforce, and labor markets. "
        "ArXiv: cs.cyber-ph, cs.soc, cs.HC</p>"
    )
    lines.append(
        f"<p style='font-size:13px;color:#6B8FB5;margin-bottom:14px'>"
        f"<b>{len(papers)}</b> labor-related papers in your library.</p>"
    )

    if not papers:
        lines.append(
            "<p style='color:#A89E8C;font-size:13px'>No labor-related papers yet. "
            "Papers from cs.cyber-ph and cs.soc categories will appear here.</p>"
        )
    else:
        for p in papers[:20]:
            cats = ", ".join((p.get("categories", []) or [])[:2])
            title = p.get("title", "")[:70]
            published = p.get("published", "")[:4]
            kw_matches = [
                kw
                for kw in LABOR_KEYWORDS
                if kw.lower() in (p.get("title", "") + " " + p.get("abstract", "")).lower()
                and len(kw) > 4
            ]
            kw_display = ", ".join(f"<code>{k}</code>" for k in kw_matches[:3])

            lines.append(f"""
<div style='border:1px solid #e0dbd4;border-radius:6px;padding:12px;margin-bottom:10px'>
  <div style='font-size:13px;font-weight:600;color:#2a2a2a;margin-bottom:3px'>{title}</div>
  <div style='font-size:11px;color:#A89E8C;margin-bottom:4px'>{cats} · {published}</div>
  {f"<div style='font-size:11px;color:#7a7570'>{kw_display}</div>" if kw_display else ""}
</div>""")

    lines.append("<style>.labor-tracker { font-family: Georgia, serif; }</style>")
    lines.append("</div>")
    return "\n".join(lines)
