"""Policy Impact Tracer — map new regulation → affected domains → updated Gene Pool priority weights.

Tracks AI policy/regulation developments and maps their impact to research domains.
"""

from __future__ import annotations

import json
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional

POLICY_FILE = Path.home() / ".ai_research_os" / "policy_impact.json"

# Known regulations and their affected domains
REGULATIONS = {
    "EU_AI_Act": {
        "name": "EU AI Act",
        "jurisdiction": "European Union",
        "effective_date": "2025-08",
        "affected_domains": ["cs.AI", "cs.LG", "cs.CY", "cs.CV"],
        "affected_gap_types": ["evaluation_gap", "scalability_issue", "method_limitation"],
        "keywords": ["eu ai act", "high-risk", " conformity", "prohibited AI"],
        "priority_boost": {"evaluation_gap": 0.3, "generalization_gap": 0.1},
    },
    "US_AI_Executive_Order": {
        "name": "US AI Executive Order",
        "jurisdiction": "United States",
        "effective_date": "2024-01",
        "affected_domains": ["cs.AI", "cs.LG"],
        "affected_gap_types": ["safety", "alignment", "evaluation"],
        "keywords": ["executive order ai", "safety", "us government ai"],
        "priority_boost": {"theoretical_gap": 0.2, "evaluation_gap": 0.2},
    },
    "GDPR_AI": {
        "name": "GDPR for AI Systems",
        "jurisdiction": "European Union",
        "effective_date": "2024-05",
        "affected_domains": ["cs.AI", "cs.LG", "cs.CY"],
        "affected_gap_types": ["evaluation_gap", "dataset_gap"],
        "keywords": ["gdpr", "data protection", "privacy ai", "personal data ai"],
        "priority_boost": {"dataset_gap": 0.3, "evaluation_gap": 0.1},
    },
    "China_AI_Regulation": {
        "name": "China AI Regulations",
        "jurisdiction": "China",
        "effective_date": "2024-01",
        "affected_domains": ["cs.AI", "cs.LG", "cs.CL"],
        "affected_gap_types": ["method_limitation", "scalability_issue"],
        "keywords": ["china ai regulation", "generative ai china", "china ml policy"],
        "priority_boost": {"scalability_issue": 0.2, "method_limitation": 0.1},
    },
}


def _load_policy_data() -> Dict[str, Any]:
    if not POLICY_FILE.exists():
        return {"regulations": REGULATIONS, "watched_papers": [], "last_scan": ""}
    return json.loads(POLICY_FILE.read_text(encoding="utf-8"))


def _save_policy_data(data: Dict[str, Any]) -> None:
    POLICY_FILE.parent.mkdir(parents=True, exist_ok=True)
    POLICY_FILE.write_text(json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")


def check_policy_impact(paper: Dict[str, Any]) -> List[Dict[str, Any]]:
    """Check which regulations a paper relates to."""
    text = (paper.get("title", "") + " " + paper.get("abstract", "")).lower()
    cats = set(paper.get("categories", []) or [])
    results: List[Dict[str, Any]] = []

    for _rid, reg in REGULATIONS.items():
        # Keyword match
        kw_match = any(kw.lower() in text for kw in reg["keywords"])
        # Category match
        cat_match = any(c in cats for c in reg["affected_domains"])
        if kw_match or cat_match:
            results.append({
                "regulation_id": rid,
                "regulation_name": reg["name"],
                "jurisdiction": reg["jurisdiction"],
                "effective_date": reg["effective_date"],
                "affected_domains": reg["affected_domains"],
                "priority_boost": reg["priority_boost"],
                "match_reason": "keyword" if kw_match else "category",
            })
    return results


def get_impacted_capsules() -> List[Dict[str, Any]]:
    """Return Gene Pool capsules whose gap types are affected by current regulations."""
    from llm.bold_vault import _load_capsules as _load
    capsules = _load()
    impacted: List[Dict[str, Any]] = []

    for cap in capsules:
        if cap.get("status") not in ("", "active"):
            continue
        gap_type = cap.get("action_gap_type", "") or cap.get("trigger_gap_type", "")
        for reg in REGULATIONS.values():
            if gap_type in reg["affected_gap_types"]:
                impacted.append({
                    "capsule_id": cap.get("capsule_id", ""),
                    "gap_title": cap.get("action_gap_title", ""),
                    "gap_type": gap_type,
                    "regulation": reg["name"],
                    "priority_boost": reg["priority_boost"].get(gap_type, 0),
                })
                break
    impacted.sort(key=lambda x: -x["priority_boost"])
    return impacted


def render_policy_tracer_html() -> str:
    impacted = get_impacted_capsules()
    _ = _load_policy_data()

    lines = ['<div class="policy-tracer">']
    lines.append("<h3>🏛️ Policy Impact Tracer</h3>")
    lines.append("<p style='font-size:13px;color:#A89E8C;margin-bottom:16px'>"
                "Maps AI regulations to affected Gene Pool gaps. "
                "Priority weights increase for gap types targeted by new policies.</p>")

    # Regulation list
    for _rid, reg in REGULATIONS.items():
        lines.append(f"""
<div style='border:1px solid #e0dbd4;border-radius:6px;padding:12px;margin-bottom:10px;border-left:4px solid #D4A055'>
  <div style='display:flex;justify-content:space-between'>
    <div style='font-weight:700;font-size:13px'>{reg['name']}</div>
    <div style='font-size:11px;color:#A89E8C'>{reg['jurisdiction']} · effective {reg['effective_date']}</div>
  </div>
  <div style='font-size:12px;color:#7a7570;margin-top:4px'>Affected: {', '.join(reg['affected_domains'])}</div>
</div>""")

    # Impacted capsules
    lines.append("<h4 style='font-size:13px;font-weight:700;color:#333;margin-top:20px;margin-bottom:10px'>"
                f"Policy-Impacted Capsules ({len(impacted)})</h4>")

    if not impacted:
        lines.append("<p style='color:#A89E8C;font-size:13px'>No capsules directly affected by current regulations.</p>")
    else:
        for cap in impacted[:10]:
            boost_pct = int(cap["priority_boost"] * 100)
            lines.append(f"""
<div style='display:flex;justify-content:space-between;align-items:center;padding:8px 12px;background:#f8f4ef;border-radius:4px;margin-bottom:6px'>
  <div>
    <div style='font-size:12px;font-weight:600;color:#2a2a2a'>{cap['gap_title'][:55]}</div>
    <div style='font-size:11px;color:#A89E8C'>{cap['gap_type']} · {cap['regulation']}</div>
  </div>
  <div style='color:#6BBF8A;font-size:12px;font-weight:700'>+{boost_pct}% priority</div>
</div>""")

    lines.append("<style>.policy-tracer { font-family: Georgia, serif; }</style>")
    lines.append("</div>")
    return "\n".join(lines)
