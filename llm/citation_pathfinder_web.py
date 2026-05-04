"""Citation Pathfinder Web Graph — interactive SVG citation chain visualization.

Shows paper → cites → Gene Pool capsule chain, color-coded by gap_type.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, List, Optional

CAPSULES_PATH = Path.home() / ".ai_research_os" / "gene_pool" / "capsules.json"

GAP_COLORS = {
    "theoretical_gap": "#C4706A",
    "method_limitation": "#D4A055",
    "evaluation_gap": "#6B8FB5",
    "scalability_issue": "#7BAD7B",
    "dataset_gap": "#9B8EC4",
    "generalization_gap": "#C4946A",
    "contradiction": "#E07070",
    "unexplored_application": "#6BBF8A",
}


def _load_capsules() -> List[Dict[str, Any]]:
    if not CAPSULES_PATH.exists():
        return []
    return json.loads(CAPSULES_PATH.read_text(encoding="utf-8")).get("capsules", [])


def build_citation_graph(
    paper_id: str, paper_title: str, cited_paper_ids: List[str], cited_capsule_ids: List[str]
) -> Dict[str, Any]:
    capsules = _load_capsules()
    capsule_map = {c.get("capsule_id", ""): c for c in capsules}

    nodes = []
    edges = []

    # Source paper node
    nodes.append(
        {"id": paper_id, "label": paper_title[:50], "type": "source_paper", "x": 300, "y": 50}
    )

    # Cited paper nodes
    _n_cited = len(cited_paper_ids)
    for i, cpid in enumerate(cited_paper_ids[:8]):
        x = 120 + (i % 4) * 140
        y = 160 + (i // 4) * 80
        nodes.append(
            {"id": cpid, "label": f"Paper {cpid[:8]}", "type": "cited_paper", "x": x, "y": y}
        )
        edges.append({"from": paper_id, "to": cpid})

    # Capsule nodes
    for i, ccid in enumerate(cited_capsule_ids[:6]):
        cap = capsule_map.get(ccid, {})
        gap_type = cap.get("action_gap_type", "") or cap.get("trigger_gap_type", "unknown")
        color = GAP_COLORS.get(gap_type, "#A89E8C")
        x = 100 + (i % 3) * 220
        y = 360 + (i // 3) * 120
        nodes.append(
            {
                "id": ccid,
                "label": cap.get("action_gap_title", ccid)[:40],
                "type": "capsule",
                "gap_type": gap_type,
                "color": color,
                "x": x,
                "y": y,
            }
        )
        edges.append({"from": paper_id, "to": ccid, "style": "dashed"})

    return {"nodes": nodes, "edges": edges}


def render_citation_graph_svg(
    graph_data: Optional[Dict[str, Any]] = None,
    paper_id: str = "",
    paper_title: str = "",
    cited_paper_ids: Optional[List[str]] = None,
    cited_capsule_ids: Optional[List[str]] = None,
) -> str:
    if graph_data is None:
        cited_paper_ids = cited_paper_ids or []
        cited_capsule_ids = cited_capsule_ids or []
        graph_data = build_citation_graph(paper_id, paper_title, cited_paper_ids, cited_capsule_ids)

    nodes = graph_data.get("nodes", [])
    edges = graph_data.get("edges", [])
    node_map = {n["id"]: n for n in nodes}

    svg_nodes = []
    for n in nodes:
        nt = n["type"]
        if nt == "source_paper":
            svg_nodes.append(
                f"<g transform='translate({n['x']},{n['y']})'>"
                f"<circle r='28' fill='#2a4a6a' opacity='0.9'/>"
                f"<text text-anchor='middle' dy='.35em' fill='white' font-size='16'>📄</text>"
                f"<text text-anchor='middle' y='42' font-size='10' fill='#444'>{n['label']}</text>"
                f"</g>"
            )
        elif nt == "cited_paper":
            svg_nodes.append(
                f"<g transform='translate({n['x']},{n['y']})'>"
                f"<rect x='-40' y='-14' width='80' height='28' rx='4' fill='#e8e4dc' stroke='#ccc'/>"
                f"<text text-anchor='middle' dy='.35em' font-size='9' fill='#555'>{n['label']}</text>"
                f"</g>"
            )
        elif nt == "capsule":
            color = n.get("color", "#A89E8C")
            svg_nodes.append(
                f"<g transform='translate({n['x']},{n['y']})'>"
                f"<rect x='-60' y='-18' width='120' height='36' rx='5' fill='{color}' opacity='0.15' stroke='{color}' stroke-width='1.5'/>"
                f"<text text-anchor='middle' dy='-.3em' font-size='9' font-weight='600' fill='{color}'>{n['gap_type'].replace('_', ' ')}</text>"
                f"<text text-anchor='middle' dy='1em' font-size='8' fill='#444'>{n['label'][:25]}</text>"
                f"</g>"
            )

    svg_edges = []
    for e in edges:
        frm = node_map.get(e.get("from"))
        to = node_map.get(e.get("to"))
        if not frm or not to:
            continue
        style = "stroke-dasharray='4,3'" if e.get("style") == "dashed" else ""
        svg_edges.append(
            f"<line x1='{frm['x']}' y1='{frm['y']}' x2='{to['x']}' y2='{to['y']}' "
            f"stroke='#aaa' stroke-width='1.2' {style}/>"
        )

    all_nodes_svg = "\n    ".join(svg_nodes)
    all_edges_svg = "\n    ".join(svg_edges)

    legend_items = "".join(
        f"<rect x='0' y='{i * 18}' width='10' height='10' rx='2' fill='{c}'/>"
        f"<text x='14' y='{i * 18 + 9}' font-size='10' fill='#555'>{gt.replace('_', ' ')}</text>"
        for i, (gt, c) in enumerate(GAP_COLORS.items())
    )

    n_legend = len(GAP_COLORS)
    svg = (
        f"<svg width='700' height='550' xmlns='http://www.w3.org/2000/svg' style='font-family:Georgia,serif'>"
        f"<g transform='translate(10,10)'>"
        f"<text x='0' y='0' font-size='11' font-weight='700' fill='#333' dy='-2'>Legend</text>"
        f"{legend_items}"
        f"<rect x='0' y='{n_legend * 18 + 8}' width='10' height='10' fill='#2a4a6a' rx='1'/><text x='14' y='{n_legend * 18 + 17}' font-size='10' fill='#555'>Source Paper</text>"
        f"<rect x='0' y='{n_legend * 18 + 22}' width='10' height='10' fill='#e8e4dc' stroke='#ccc' rx='1'/><text x='14' y='{n_legend * 18 + 31}' font-size='10' fill='#555'>Cited Paper</text>"
        f"</g>"
        f"<g>{all_edges_svg}</g>"
        f"<g>{all_nodes_svg}</g>"
        f"</svg>"
    )
    return svg


def render_citation_chain_html(
    paper_id: str, paper_title: str, cited_paper_ids: List[str], cited_capsule_ids: List[str]
) -> str:
    capsules = _load_capsules()
    capsule_map = {c.get("capsule_id", ""): c for c in capsules}
    graph_svg = render_citation_graph_svg(
        paper_id=paper_id,
        paper_title=paper_title,
        cited_paper_ids=cited_paper_ids,
        cited_capsule_ids=cited_capsule_ids,
    )

    lines = ['<div class="citation-pathfinder">']
    lines.append("<h3>🔗 Citation Pathfinder</h3>")
    lines.append(
        "<p style='font-size:13px;color:#A89E8C;margin-bottom:14px'>"
        "Paper → cited references → Gene Pool capsules</p>"
    )
    lines.append(f"<div style='overflow:auto'>{graph_svg}</div>")

    if cited_capsule_ids:
        lines.append("<div style='margin-top:16px'>")
        lines.append(
            "<h4 style='font-size:13px;font-weight:700;color:#333;margin-bottom:8px'>Gene Pool Capsules Cited</h4>"
        )
        for ccid in cited_capsule_ids[:6]:
            cap = capsule_map.get(ccid, {})
            if cap:
                gt = cap.get("action_gap_type", "") or cap.get("trigger_gap_type", "unknown")
                color = GAP_COLORS.get(gt, "#A89E8C")
                lines.append(
                    f"<div style='border-left:3px solid {color};padding-left:10px;margin-bottom:8px'>"
                    f"<div style='font-size:12px;font-weight:600;color:#2a2a2a'>{cap.get('action_gap_title', ccid)[:70]}</div>"
                    f"<div style='font-size:11px;color:#A89E8C'>{gt} &middot; score={cap.get('outcome_success_score', 0):.2f}</div>"
                    f"</div>"
                )
        lines.append("</div>")

    lines.append("<style>.citation-pathfinder { font-family: Georgia, serif; }</style>")
    lines.append("</div>")
    return "\n".join(lines)
