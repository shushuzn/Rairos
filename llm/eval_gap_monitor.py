"""Evaluation Gap Monitor — flag deployment timelines outpacing benchmark research.

Trigger when: benchmark_paper_count / deployment_year_gap < threshold
Heuristic: if papers mention "deployment" + "2024/2025/2026" but few papers
with matching keywords exist in the library → flag as at-risk evaluation gap.
"""

from __future__ import annotations

import json
import re
from collections import defaultdict
from pathlib import Path
from typing import Any, Dict, List, Optional

PAPERS_DB = Path.home() / ".ai_research_os" / "papers.json"
GAP_THRESHOLD = 0.1  # papers per month of deployment headroom
DEPLOYMENT_KEYWORDS = [
    "deployment",
    "deployed",
    "in production",
    "in deployment",
    "real-world",
    "field trial",
    "pilot program",
    "operational",
]
YEAR_PATTERN = re.compile(r"\b(202[4-9]|203[0-5])\b")


def _load_papers() -> List[Dict[str, Any]]:
    if not PAPERS_DB.exists():
        return []
    data = json.loads(PAPERS_DB.read_text(encoding="utf-8"))  # type: ignore[no-any-return]
    return data.get("papers", [])  # type: ignore[no-any-return]


def detect_deployment_claims(title: str, abstract: str = "") -> Optional[str]:
    """Return the deployment year claimed in title/abstract, or None."""
    text = f"{title} {abstract}".lower()
    if not any(kw in text for kw in DEPLOYMENT_KEYWORDS):
        return None
    match = YEAR_PATTERN.search(f"{title} {abstract}")
    return match.group(1) if match else None


def check_eval_gaps() -> Dict[str, Any]:
    """Scan paper library for domains where deployment outpaces benchmark research."""
    papers = _load_papers()

    # Group by category
    by_category: Dict[str, List[Dict]] = defaultdict(list)
    for p in papers:
        cats = p.get("categories", []) or []
        for c in cats:
            by_category[c].append(p)

    alerts: List[Dict[str, Any]] = []

    for cat, cat_papers in by_category.items():
        if len(cat_papers) < 3:
            continue

        # Count deployment claims in this category
        deploying = []
        for p in cat_papers:
            yr = detect_deployment_claims(p.get("title", ""), p.get("abstract", ""))
            if yr:
                deploying.append(
                    {"title": p.get("title", ""), "year": yr, "paper_id": p.get("id", "")}
                )

        if not deploying:
            continue

        # Deployment year vs. research paper count ratio
        deploying_years = [int(d["year"]) for d in deploying]
        nearest_deploy = min(deploying_years)
        from datetime import datetime

        current_year = datetime.now().year
        headroom = max(0, nearest_deploy - current_year)
        paper_count = len(cat_papers)

        # Flag if headroom > 0 but paper count is low relative to headroom
        ratio = paper_count / max(headroom, 1)
        if ratio < GAP_THRESHOLD and headroom >= 1:
            alerts.append(
                {
                    "category": cat,
                    "paper_count": paper_count,
                    "nearest_deployment_year": nearest_deploy,
                    "headroom_years": headroom,
                    "ratio": round(ratio, 3),
                    "deploying_papers": deploying[:3],
                    "severity": "high" if headroom >= 3 else "medium",
                }
            )

    alerts.sort(key=lambda x: -x["headroom_years"])
    return {
        "alerts": alerts,
        "total_domains_checked": len(by_category),
        "alert_count": len(alerts),
    }


def render_eval_gap_html(data: Optional[Dict[str, Any]] = None) -> str:
    if data is None:
        data = check_eval_gaps()

    alerts = data.get("alerts", [])

    lines = ['<div class="eval-gap">']
    lines.append("<h3>⚠️ Evaluation Gap Monitor</h3>")
    lines.append(
        f"<p style='font-size:13px;color:#A89E8C;margin-bottom:16px'>"
        f"{data.get('alert_count', 0)} deployment-timeframe gaps detected across {data.get('total_domains_checked', 0)} domains. "
        f"<span style='color:#C4706A'>Red</span> = ≥3yr headroom · <span style='color:#D4A055'>Orange</span> = 1-2yr</p>"
    )

    if not alerts:
        lines.append(
            "<p>No evaluation gaps detected. Deployment timelines appear adequately covered by benchmark research.</p>"
        )
    else:
        for alert in alerts:
            color = "#C4706A" if alert["severity"] == "high" else "#D4A055"
            lines.append(
                f"<div style='border-left: 4px solid {color}; padding: 10px 14px; margin-bottom: 14px; background: rgba(0,0,0,0.02);'>"
            )
            lines.append(
                f"<div style='font-weight:700;font-size:14px;color:#2a2a2a'>{alert['category']}</div>"
            )
            lines.append(
                f"<div style='font-size:12px;color:#7a7570;margin:4px 0'>"
                f"<b>{alert['paper_count']}</b> papers · deployment in <b>{alert['nearest_deployment_year']}</b> "
                f"({alert['headroom_years']}yr headroom) · ratio={alert['ratio']:.2f}</div>"
            )
            for dep in alert.get("deploying_papers", [])[:2]:
                lines.append(
                    f"<div style='font-size:11px;color:#A89E8C;margin-left:8px'>• {dep['title'][:70]}</div>"
                )
            lines.append("</div>")

    lines.append("<style>")
    lines.append(".eval-gap { font-family: Georgia, serif; }")
    lines.append("</style>")
    lines.append("</div>")
    return "\n".join(lines)
