"""HTML rendering helpers for Rairos web UI — pure functions, no FastAPI deps."""

from __future__ import annotations

import json
from typing import Any, Dict, List


def render_gene_pool_graph_html() -> str:
    """D3.js force-directed graph of all Gene Pool capsules.

    Returns a complete HTML page with embedded D3.js visualization.
    """
    from llm.gene_pool_io import load_capsules

    capsules = load_capsules()

    GAP_TYPE_COLORS = {
        "embodied_planning": "#7A9E7A",
        "rl_efficiency": "#6B8FB5",
        "method_limitation": "#B57A7A",
        "unexplored_application": "#9B7AB5",
        "evaluation_gap": "#B5A57A",
        "theoretical_gap": "#7AB5B5",
        "dataset_gap": "#B57A9B",
        "generalization_gap": "#7A8FB5",
        "scalability_issue": "#B58B7A",
        "contradiction": "#D9534F",
        "other": "#AAAAAA",
    }

    nodes = []
    for c in capsules:
        gap_type = c.get("action_gap_type", "other")
        color = GAP_TYPE_COLORS.get(gap_type, GAP_TYPE_COLORS["other"])
        source_paper_id = c.get("archetype", {}).get("source_paper_id", "")
        label = (c.get("action_gap_title") or c.get("trigger_topic") or "?")[:60]
        if source_paper_id:
            label = f"[{source_paper_id}] {label}"
        nodes.append({
            "id": c.get("capsule_id", ""),
            "label": label,
            "gap_type": gap_type,
            "color": color,
            "score": c.get("outcome_success_score", 0.0),
            "source": c.get("trigger_topic", "")[:40],
            "source_paper_id": source_paper_id,
        })

    links = []
    for i, a in enumerate(capsules):
        for j, b in enumerate(capsules):
            if i >= j:
                continue
            if a.get("action_gap_type") != b.get("action_gap_type"):
                continue
            same_title = a.get("action_gap_title") == b.get("action_gap_title")
            if not same_title:
                links.append({
                    "source": a.get("capsule_id", ""),
                    "target": b.get("capsule_id", ""),
                    "type": "contradiction",
                    "stroke": "#D9534F",
                    "strokeWidth": 2.5,
                    "strokeDasharray": None,
                })
            else:
                links.append({
                    "source": a.get("capsule_id", ""),
                    "target": b.get("capsule_id", ""),
                    "type": "same_gap",
                    "stroke": "#999999",
                    "strokeWidth": 1.0,
                    "strokeDasharray": "4,4",
                })

    nodes_json = json.dumps(nodes, ensure_ascii=False)
    links_json = json.dumps(links, ensure_ascii=False)

    return f"""<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>Gene Pool — Research Map</title>
<script src="https://d3js.org/d3.v7.min.js"></script>
<style>
:root {{ --font-display: 'Courier New', monospace; }}
body {{ margin:0; font-family: var(--font-display); background:#fafaf7; overflow:hidden; }}
svg {{ width:100vw; height:100vh; }}
.node circle {{ cursor:pointer; stroke-width:2px; }}
.node text {{ font-size:10px; fill:#333; pointer-events:none; }}
.link {{ fill:none; stroke-opacity:0.7; }}
h1 {{ position:fixed; top:14px; left:20px; margin:0; font-size:16px; color:#1a1a2e; z-index:10; background:#fafaf7; padding:0 8px 4px 0; }}
.legend {{ position:fixed; bottom:20px; left:20px; background:#fff; border:1px solid #ddd; border-radius:8px; padding:10px 14px; font-size:11px; z-index:10; box-shadow:0 2px 6px rgba(0,0,0,0.08); }}
.legend-item {{ display:flex; align-items:center; gap:7px; margin-bottom:5px; }}
.legend-dot {{ width:12px; height:12px; border-radius:50%; flex-shrink:0; }}
.legend-line {{ width:24px; height:3px; flex-shrink:0; border-radius:2px; }}
.legend-label {{ color:#555; }}
</style>
</head>
<body>
<h1>Gene Pool — Research Map</h1>
<div class="legend">
<div class="legend-item"><div class="legend-dot" style="background:#7A9E7A"></div><span class="legend-label">embodied_planning</span></div>
<div class="legend-item"><div class="legend-dot" style="background:#6B8FB5"></div><span class="legend-label">rl_efficiency</span></div>
<div class="legend-item"><div class="legend-dot" style="background:#B57A7A"></div><span class="legend-label">method_limitation</span></div>
<div class="legend-item"><div class="legend-dot" style="background:#9B7AB5"></div><span class="legend-label">unexplored_application</span></div>
<div class="legend-item"><div class="legend-dot" style="background:#B5A57A"></div><span class="legend-label">evaluation_gap</span></div>
<div class="legend-item"><div class="legend-dot" style="background:#7AB5B5"></div><span class="legend-label">theoretical_gap</span></div>
<div class="legend-item"><div class="legend-dot" style="background:#AAAAAA"></div><span class="legend-label">other</span></div>
<div class="legend-item"><div class="legend-line" style="background:#D9534F"></div><span class="legend-label">contradiction (diff conclusion)</span></div>
<div class="legend-item"><div class="legend-line" style="background:#999; border-top:2px dashed #999"></div><span class="legend-label">same gap (same conclusion)</span></div>
</div>
<script>
const nodes = {nodes_json};
const links = {links_json};

const width = window.innerWidth;
const height = window.innerHeight;

const svg = d3.select("body").append("svg")
    .attr("width", width).attr("height", height);

const g = svg.append("g");
svg.call(d3.zoom().scaleExtent([0.1, 4]).on("zoom", (event) => {{
    g.attr("transform", event.transform);
}}));

const simulation = d3.forceSimulation(nodes)
    .force("link", d3.forceLink(links).id(d => d.id).distance(120).strength(0.4))
    .force("charge", d3.forceManyBody().strength(-300))
    .force("center", d3.forceCenter(width / 2, height / 2))
    .force("collision", d3.forceCollide().radius(40));

const link = g.append("g")
    .selectAll("line")
    .data(links)
    .join("line")
    .attr("class", "link")
    .attr("stroke", d => d.stroke)
    .attr("stroke-width", d => d.strokeWidth)
    .attr("stroke-dasharray", d => d.strokeDasharray || null);

const node = g.append("g")
    .selectAll("g")
    .data(nodes)
    .join("g")
    .attr("class", "node")
    .call(d3.drag()
        .on("start", (event, d) => {{
            if (!event.active) simulation.alphaTarget(0.3).restart();
            d.fx = d.x; d.fy = d.y;
        }})
        .on("drag", (event, d) => {{
            d.fx = event.x; d.fy = event.y;
        }})
        .on("end", (event, d) => {{
            if (!event.active) simulation.alphaTarget(0);
            d.fx = null; d.fy = null;
        }}));

node.append("circle")
    .attr("r", d => 6 + (d.score || 0) * 10)
    .attr("fill", d => d.color)
    .attr("stroke", d => d.source_paper_id ? "#333" : "#fff")
    .attr("stroke-width", d => d.source_paper_id ? 2 : 1.5);

node.append("text")
    .attr("dx", 14).attr("dy", 4)
    .text(d => d.label);

node.on("mouseover", function(event, d) {{
    d3.select(this).select("text").style("font-weight", "bold");
    if (d.source_paper_id) {{
        d3.select(this).select("circle").style("cursor", "pointer");
    }}
}}).on("mouseout", function(event, d) {{
    d3.select(this).select("text").style("font-weight", "normal");
}}).on("click", function(event, d) {{
    if (d.source_paper_id) {{
        window.open('/paper/' + d.source_paper_id, '_blank');
    }}
}});

simulation.on("tick", () => {{
    link
        .attr("x1", d => d.source.x)
        .attr("y1", d => d.source.y)
        .attr("x2", d => d.target.x)
        .attr("y2", d => d.target.y);
    node.attr("transform", d => "translate(" + d.x + "," + d.y + ")");
}});
</script>
</body>
</html>"""
