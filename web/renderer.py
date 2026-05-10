"""Web renderers — HTML generation for various visualizations."""
from __future__ import annotations

import json
from collections import defaultdict
from typing import Any, Dict, List, Tuple


# ─── Gene Pool Force-Directed Graph ─────────────────────────────────────────────


def render_gene_pool_graph_html() -> str:
    """D3.js force-directed graph of all Gene Pool capsules."""
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
        nodes.append(
            {
                "id": c.get("capsule_id", ""),
                "label": label,
                "gap_type": gap_type,
                "color": color,
                "score": c.get("outcome_success_score", 0.0),
                "source": c.get("trigger_topic", "")[:40],
                "source_paper_id": source_paper_id,
            }
        )

    links = []
    for i, a in enumerate(capsules):
        for j, b in enumerate(capsules):
            if i >= j:
                continue
            if a.get("action_gap_type") != b.get("action_gap_type"):
                continue
            same_title = a.get("action_gap_title") == b.get("action_gap_title")
            links.append(
                {
                    "source": a.get("capsule_id", ""),
                    "target": b.get("capsule_id", ""),
                    "type": "same_gap" if same_title else "contradiction",
                    "stroke": "#999999" if same_title else "#D9534F",
                    "strokeWidth": 1.0 if same_title else 2.5,
                    "strokeDasharray": "4,4" if same_title else None,
                }
            )

    nodes_json = json.dumps(nodes, ensure_ascii=False)
    links_json = json.dumps(links, ensure_ascii=False)

    return (
        '<!DOCTYPE html>\n<html>\n<head>\n<meta charset="utf-8">\n<title>Gene Pool — Force Graph</title>\n'
        '<script src="https://d3js.org/d3.v7.min.js"></script>\n<style>\n'
        "body { margin: 0; background: #0d1117; }\n"
        "h2 { color: #c9d1d9; padding: 12px 16px; font-family: monospace; font-size: 14px; }\n"
        "svg { width: 100vw; height: 100vh; }\n"
        ".node text { fill: #c9d1d9; font-size: 11px; font-family: monospace; }\n"
        ".link { stroke-opacity: 0.6; }\n"
        "</style>\n</head>\n<body>\n"
        f'<h2>Gene Pool — {len(capsules)} capsules · force-directed by gap type</h2>\n'
        "<svg></svg>\n<script>\n"
        f"const nodes = {nodes_json};\n"
        f"const links = {links_json};\n"
        "const simulation = d3.forceSimulation(nodes)\n"
        '  .force("link", d3.forceLink(links).id(d => d.id).distance(80))\n'
        '  .force("charge", d3.forceManyBody().strength(-200))\n'
        '  .force("center", d3.forceCenter(window.innerWidth / 2, window.innerHeight / 2));\n'
        "const svg = d3.select('svg');\n"
        "const link = svg.append('g').selectAll('line').data(links).join('line')\n"
        '  .attr("class", "link")\n'
        "  .attr('stroke', d => d.stroke)\n"
        "  .attr('stroke-width', d => d.strokeWidth)\n"
        "  .attr('stroke-dasharray', d => d.strokeDasharray || null);\n"
        "const node = svg.append('g').selectAll('g').data(nodes).join('g')\n"
        "  .attr('class', 'node')\n"
        "  .call(d3.drag().on('start', dragstarted).on('drag', dragged).on('end', dragended));\n"
        "node.append('rect').attr('width', 130).attr('height', 36).attr('rx', 6)\n"
        "  .attr('fill', d => d.color).attr('opacity', 0.9);\n"
        "node.append('text').attr('x', 8).attr('y', 14)\n"
        "  .text(d => d.label.slice(0, 18)).attr('fill', 'white');\n"
        "node.append('text').attr('x', 8).attr('y', 27)\n"
        "  .text(d => d.gap_type.slice(0, 14)).attr('fill', '#aaa');\n"
        "node.append('text').attr('x', 8).attr('y', 40)\n"
        "  .text(d => 'score:' + d.score.toFixed(2))\n"
        "  .attr('fill', d => d.score >= 0.7 ? '#3fb950' : '#da3633');\n"
        "simulation.on('tick', () => {\n"
        "  link.attr('x1', d => d.source.x + 65).attr('y1', d => d.source.y + 18)\n"
        "      .attr('x2', d => d.target.x + 65).attr('y2', d => d.target.y + 18);\n"
        "  node.attr('transform', d => 'translate(' + d.x + ',' + d.y + ')');\n"
        "});\n"
        "function dragstarted(event) { if (!event.active) simulation.alphaTarget(0.3).restart(); event.subject.fx = event.subject.x; event.subject.fy = event.subject.y; }\n"
        "function dragged(event) { event.subject.fx = event.x; event.subject.fy = event.y; }\n"
        "function dragended(event) { if (!event.active) simulation.alphaTarget(0); event.subject.fx = null; event.subject.fy = null; }\n"
        "</script>\n</body>\n</html>"
    )


# ─── Gene Pool Family Tree ─────────────────────────────────────────────────────


def render_gene_pool_family_tree_html() -> str:
    """Phylogenetic tree (dendrogram) of Gene Pool capsules.

    Constructs implicit parent→child relationships from capsules sharing the same
    (trigger_topic, action_gap_type) cluster, ordered by evolved_generation.
    Root = earliest generation, leaves = latest evolved capsules.
    Layout: horizontal dendrogram, left=oldest generation, right=newest.
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

    # ── Cluster by (trigger_topic, action_gap_type) ──────────────────────
    clusters: Dict[tuple, List[Dict]] = defaultdict(list)
    for c in capsules:
        key = (c.get("trigger_topic", "") or "", c.get("action_gap_type", "other") or "other")
        clusters[key].append(c)

    for key in clusters:
        clusters[key].sort(key=lambda x: x.get("evolved_generation", 0))

    # ── Build nodes with parent links ─────────────────────────────────────
    BOX_W = 160
    BOX_H = 54
    GAP_X = 80
    GAP_Y = 20
    PAD_TOP = 80
    PAD_LEFT = 30

    nodes: List[Dict[str, Any]] = []
    id_to_node = {}

    for (topic, gap_type), members in clusters.items():
        if not members:
            continue
        parent_id = None
        for capsule in members:
            gen = capsule.get("evolved_generation", 0)
            capsule_id = capsule.get("capsule_id", f"c{len(nodes)}")
            label = (capsule.get("action_gap_title") or topic or "?")[:40]
            color = GAP_TYPE_COLORS.get(gap_type, GAP_TYPE_COLORS["other"])
            score = capsule.get("outcome_success_score", 0.0)
            status = capsule.get("status", "active")
            badge = capsule.get("credibility_badge", "medium")

            if status == "archived":
                color = "#555555"
            opacity = 1.0 if status == "active" else 0.4

            n = {
                "id": capsule_id,
                "label": label,
                "color": color,
                "generation": gen,
                "parent_id": parent_id,
                "cluster_key": f"{topic[:20]}|{gap_type}",
                "x": 0,
                "y": 0,
                "score": score,
                "status": status,
                "badge": badge,
                "opacity": opacity,
            }
            nodes.append(n)
            id_to_node[capsule_id] = n
            parent_id = capsule_id

    # ── Layout: assign x,y ────────────────────────────────────────────────
    by_gen: Dict[int, List[Dict]] = defaultdict(list)
    for n in nodes:
        by_gen[n["generation"]].append(n)

    max_gen = max(by_gen.keys()) if by_gen else 0

    for gen, gen_nodes in by_gen.items():
        x = PAD_LEFT + gen * (BOX_W + GAP_X)
        for i, n in enumerate(gen_nodes):
            n["x"] = x
            n["y"] = PAD_TOP + i * (BOX_H + GAP_Y)

    # ── Build SVG links ──────────────────────────────────────────────────
    links = []
    for n in nodes:
        if n["parent_id"] and n["parent_id"] in id_to_node:
            parent = id_to_node[n["parent_id"]]
            links.append({
                "x1": parent["x"] + BOX_W,
                "y1": parent["y"] + BOX_H // 2,
                "x2": n["x"],
                "y2": n["y"] + BOX_H // 2,
                "color": n["color"],
                "opacity": min(n["opacity"], parent["opacity"]),
            })

    # ── SVG dimensions ────────────────────────────────────────────────────
    all_y = [n["y"] for n in nodes]
    max_y = (max(all_y) + BOX_H + PAD_TOP) if all_y else 800
    total_width = PAD_LEFT + (max_gen + 1) * (BOX_W + GAP_X) + PAD_LEFT
    svg_height = max(max_y, 400)

    nodes_json = json.dumps(nodes, ensure_ascii=False)
    links_json = json.dumps(links, ensure_ascii=False)
    gen_labels_json = json.dumps([
        {"x": PAD_LEFT + g * (BOX_W + GAP_X) - 8, "y": 20, "label": f"Gen {g}"}
        for g in range(max_gen + 1)
    ], ensure_ascii=False)

    # ── Build HTML without f-string JS conflicts ──────────────────────────
    # Use regular string concatenation for the script block
    head = (
        '<!DOCTYPE html>\n<html>\n<head>\n<meta charset="utf-8">\n'
        '<title>Gene Pool Family Tree</title>\n<style>\n'
        'body { margin: 0; background: #0d1117; font-family: monospace; }\n'
        'h2 { color: #c9d1d9; margin: 16px 0 4px 20px; font-size: 14px; }\n'
        '.subtitle { color: #484f58; margin: 0 0 12px 20px; font-size: 11px; }\n'
        '.legend { display: flex; flex-wrap: wrap; gap: 10px; padding: 0 20px 12px; }\n'
        '.legend-item { display: flex; align-items: center; gap: 5px; font-size: 10px; color: #8b949e; }\n'
        '.legend-dot { width: 9px; height: 9px; border-radius: 2px; }\n'
        '.gen-label { font-size: 10px; fill: #484f58; }\n'
        '</style>\n</head>\n<body>\n'
        f'<h2>Gene Pool Family Tree</h2>\n'
        f'<div class="subtitle">{len(nodes)} capsules across {max_gen + 1} generations · horizontal = time (left=oldest)</div>\n'
        '<div class="legend">\n'
        '  <div class="legend-item"><div class="legend-dot" style="background:#7A9E7A"></div>embodied_planning</div>\n'
        '  <div class="legend-item"><div class="legend-dot" style="background:#6B8FB5"></div>rl_efficiency</div>\n'
        '  <div class="legend-item"><div class="legend-dot" style="background:#B57A7A"></div>method_limitation</div>\n'
        '  <div class="legend-item"><div class="legend-dot" style="background:#9B7AB5"></div>unexplored_application</div>\n'
        '  <div class="legend-item"><div class="legend-dot" style="background:#7AB5B5"></div>theoretical_gap</div>\n'
        '  <div class="legend-item"><div class="legend-dot" style="background:#B57A9B"></div>dataset_gap</div>\n'
        '  <div class="legend-item"><div class="legend-dot" style="background:#555555"></div>archived</div>\n'
        '  <div class="legend-item" style="margin-left:20px">| box color = gap type |</div>\n'
        '  <div class="legend-item"><div style="width:20px;height:10px;background:#238636;border-radius:2px"></div>high credibility</div>\n'
        '  <div class="legend-item"><div style="width:20px;height:10px;background:#9e6a03;border-radius:2px"></div>medium</div>\n'
        '  <div class="legend-item"><div style="width:20px;height:10px;background:#da3633;border-radius:2px"></div>low</div>\n'
        '</div>\n'
        f'<svg width="{total_width}" height="{svg_height}" xmlns="http://www.w3.org/2000/svg">\n'
        '  <g id="links"></g>\n'
        '  <g id="nodes"></g>\n'
        '  <g id="gen-labels"></g>\n'
        '</svg>\n'
    )

    script = (
        '<script>\n'
        'const nodes = ' + nodes_json + ';\n'
        'const links = ' + links_json + ';\n'
        'const genLabels = ' + gen_labels_json + ';\n'
        'const BOX_W = ' + str(BOX_W) + ', BOX_H = ' + str(BOX_H) + ';\n'
        # Render links
        "d3.select('#links').selectAll('line').data(links).join('line')\n"
        "  .attr('x1', function(d) { return d.x1; })\n"
        "  .attr('y1', function(d) { return d.y1; })\n"
        "  .attr('x2', function(d) { return d.x2; })\n"
        "  .attr('y2', function(d) { return d.y2; })\n"
        "  .attr('stroke', function(d) { return d.color; })\n"
        "  .attr('stroke-width', 1.5)\n"
        "  .attr('opacity', function(d) { return d.opacity * 0.6; });\n"
        # Render generation labels
        "d3.select('#gen-labels').selectAll('text').data(genLabels).join('text')\n"
        "  .attr('x', function(d) { return d.x; })\n"
        "  .attr('y', function(d) { return d.y; })\n"
        "  .attr('class', 'gen-label')\n"
        "  .text(function(d) { return d.label; });\n"
        # Render nodes
        "const nodeGroups = d3.select('#nodes').selectAll('g').data(nodes).join('g')\n"
        "  .attr('transform', function(d) { return 'translate(' + d.x + ',' + d.y + ')'; })\n"
        "  .attr('opacity', function(d) { return d.opacity; });\n"
        "nodeGroups.append('rect')\n"
        "  .attr('width', BOX_W).attr('height', BOX_H).attr('rx', 6)\n"
        "  .attr('fill', function(d) { return d.color; })\n"
        "  .attr('opacity', 0.9);\n"
        "nodeGroups.append('rect')\n"
        "  .attr('x', BOX_W - 18).attr('y', 5).attr('width', 14).attr('height', 9).attr('rx', 2)\n"
        "  .attr('fill', function(d) {\n"
        "    return d.badge === 'high' ? '#238636' : d.badge === 'medium' ? '#9e6a03' : '#da3633'; })\n"
        "  .attr('opacity', 0.9);\n"
        "nodeGroups.append('text').attr('x', 6).attr('y', 14).attr('fill', 'white').attr('font-size', 10)\n"
        "  .text(function(d) {\n"
        "    var parts = d.cluster_key.split('|');\n"
        "    return 'g' + d.generation + ' ' + parts[1].slice(0, 12); });\n"
        "nodeGroups.append('text').attr('x', 6).attr('y', 30).attr('fill', 'white').attr('font-size', 10)\n"
        "  .text(function(d) { return d.label.slice(0, 20); });\n"
        "nodeGroups.append('text').attr('x', 6).attr('y', 45).attr('font-size', 9)\n"
        "  .attr('fill', function(d) { return d.score >= 0.7 ? '#3fb950' : d.score >= 0.4 ? '#d29922' : '#da3633'; })\n"
        "  .text(function(d) { return 'score:' + d.score.toFixed(2) + ' ' + d.status; });\n"
        "</script>\n"
    )

    return head + script + '</body>\n</html>\n'


# ─── Contradiction Heatmap ─────────────────────────────────────────────────────


def render_contradiction_heatmap_html() -> str:
    """Simple heatmap of papers colored by contradiction count."""
    return """<!DOCTYPE html>
<html><head><meta charset="utf-8"><title>Contradiction Heatmap</title>
<style>body{margin:0;background:#0d1117;font-family:monospace}h2{color:#c9d1d9;padding:12px}table{width:100%;border-collapse:collapse}th,td{padding:6px 10px;border-bottom:1px solid #21262d;text-align:left;font-size:12px}th{color:#8b949e;background:#161b22}td{color:#c9d1d9}</style></head>
<body><h2>Contradiction Heatmap — no data yet</h2>
<p style="color:#484f58;padding:0 16px">Run `rairos analyze --contradictions` first to populate.</p>
</body></html>"""
