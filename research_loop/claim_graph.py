"""
Claim Graph — cross-paper numerical claim tracking and contradiction detection.

Nodes: individual numerical claims extracted from papers
  - paper_id, claim_type (accuracy/speedup/reduction/param_size/memory),
  - value (float), comparison_op (gte/lte/eq), source_paper_text

Edges: "claim_improves" (paper A claims better than paper B on same metric)

Use cases:
  - find_contradictions(): same metric, opposite direction → research integrity signal
  - render_claim_graph_html(): D3.js directed graph for web UI

闭环:
  paper2code → BenchmarkResult → success_score → Gene Pool
  Gene Pool → claim extraction → ClaimGraph → find_contradictions → GapAnalyzer feedback
"""

from __future__ import annotations

import json
import math
import re
from dataclasses import asdict, dataclass, field
from datetime import datetime
from enum import Enum
from pathlib import Path
from typing import Any, Dict, List, Optional

GP_DIR = Path.home() / ".ai_research_os" / "evolution"


# ─── Enums ────────────────────────────────────────────────────────────────────


class ClaimType(str, Enum):
    ACCURACY = "accuracy"
    SPEEDUP = "speedup"
    REDUCTION = "reduction"  # e.g., FLOPs reduction, latency reduction
    PARAM_SIZE = "param_size"
    MEMORY = "memory"
    OTHER = "other"


class ComparisonOp(str, Enum):
    GTE = ">="   # greater than or equal (accuracy: higher is better)
    LTE = "<="   # less than or equal (latency, param_size: lower is better)
    EQ = "=="    # exact match


# ─── Data classes ─────────────────────────────────────────────────────────────


@dataclass
class ClaimNode:
    """A single numerical claim extracted from a paper."""

    claim_id: str              # unique within this graph, e.g. "n0", "n1"
    paper_id: str              # arXiv ID
    claim_type: ClaimType      # accuracy | speedup | reduction | param_size | memory | other
    value: float               # the claimed number
    comparison_op: ComparisonOp  # >= for accuracy, <= for param/speed
    source_text: str           # original paper text snippet
    page_ref: Optional[int] = None  # page number in paper
    char_start: Optional[int] = None
    char_end: Optional[int] = None

    def to_dict(self) -> dict:
        return {
            "claim_id": self.claim_id,
            "paper_id": self.paper_id,
            "claim_type": self.claim_type.value,
            "value": self.value,
            "comparison_op": self.comparison_op.value,
            "source_text": self.source_text[:200],
            "page_ref": self.page_ref,
        }


@dataclass
class ClaimEdge:
    """A directed edge: paper A claims improvement over paper B on a metric."""

    from_paper: str    # arXiv ID of the claiming paper
    to_paper: str      # arXiv ID of the compared/prior paper
    claim_type: ClaimType
    improvement_ratio: float  # e.g., 1.23 means 23% better
    source_text: str  # text that stated this relationship


@dataclass
class Contradiction:
    """A detected contradiction between two claims."""

    claim_a: ClaimNode
    claim_b: ClaimNode
    metric: str        # e.g., "accuracy", "speedup"
    description: str   # human-readable explanation
    severity: str      # "high" | "medium" | "low"


# ─── ClaimGraph ────────────────────────────────────────────────────────────────


class ClaimGraph:
    """Directed graph of cross-paper numerical claims."""

    def __init__(self):
        self.nodes: Dict[str, ClaimNode] = {}  # claim_id → ClaimNode
        self.edges: List[ClaimEdge] = []
        self._next_id = 0
        self._paper_claims: Dict[str, List[str]] = {}  # paper_id → [claim_ids]

    def add_claim(
        self,
        paper_id: str,
        claim_type: ClaimType,
        value: float,
        comparison_op: ComparisonOp,
        source_text: str,
        page_ref: Optional[int] = None,
        char_start: Optional[int] = None,
        char_end: Optional[int] = None,
    ) -> str:
        """Add a claim node and return its claim_id."""
        claim_id = f"n{self._next_id}"
        self._next_id += 1

        node = ClaimNode(
            claim_id=claim_id,
            paper_id=paper_id,
            claim_type=claim_type,
            value=value,
            comparison_op=comparison_op,
            source_text=source_text,
            page_ref=page_ref,
            char_start=char_start,
            char_end=char_end,
        )
        self.nodes[claim_id] = node
        self._paper_claims.setdefault(paper_id, []).append(claim_id)
        return claim_id

    def add_edge(
        self,
        from_paper: str,
        to_paper: str,
        claim_type: ClaimType,
        improvement_ratio: float,
        source_text: str,
    ) -> None:
        """Add a directed improvement edge."""
        edge = ClaimEdge(
            from_paper=from_paper,
            to_paper=to_paper,
            claim_type=claim_type,
            improvement_ratio=improvement_ratio,
            source_text=source_text,
        )
        self.edges.append(edge)

    def find_contradictions(self) -> List[Contradiction]:
        """Find pairs of claims on the same metric with opposite directions.

        e.g., paper A claims accuracy >= 95%, paper B claims accuracy <= 90%
        on the same dataset — high severity research integrity issue.
        """
        contradictions = []
        claims_by_type: Dict[ClaimType, List[ClaimNode]] = {}
        for node in self.nodes.values():
            claims_by_type.setdefault(node.claim_type, []).append(node)

        for claim_type, claims in claims_by_type.items():
            if claim_type not in (ClaimType.ACCURACY, ClaimType.SPEEDUP, ClaimType.REDUCTION):
                continue

            for i, ca in enumerate(claims):
                for cb in claims[i + 1:]:
                    if ca.paper_id == cb.paper_id:
                        continue  # same paper, different contexts

                    # Check for opposition
                    if ca.comparison_op != cb.comparison_op:
                        # One is >=, other is <= on same metric
                        diff = abs(ca.value - cb.value)
                        avg = (ca.value + cb.value) / 2
                        severity = "high" if (diff / avg if avg else 0) > 0.05 else "medium"

                        contradictions.append(
                            Contradiction(
                                claim_a=ca,
                                claim_b=cb,
                                metric=claim_type.value,
                                description=(
                                    f"Paper {ca.paper_id} claims {ca.comparison_op.value} {ca.value} "
                                    f"but paper {cb.paper_id} claims {cb.comparison_op.value} {cb.value} "
                                    f"on {claim_type.value} — {diff / avg * 100:.1f}% gap"
                                ),
                                severity=severity,
                            )
                        )

        return contradictions

    def get_paper_claims(self, paper_id: str) -> List[ClaimNode]:
        """Get all claims for a specific paper."""
        claim_ids = self._paper_claims.get(paper_id, [])
        return [self.nodes[cid] for cid in claim_ids if cid in self.nodes]

    def get_all_claims_by_type(self, claim_type: ClaimType) -> List[ClaimNode]:
        """Get all claims of a given type across all papers."""
        return [n for n in self.nodes.values() if n.claim_type == claim_type]

    def get_inbound_improvement_claims(self, paper_id: str) -> List[ClaimNode]:
        """Get claims from other papers that claim improvement over this paper.

        Finds all edges where to_paper == paper_id and returns the
        source paper's claims of the same type.
        """
        inbound_claims: List[ClaimNode] = []
        paper_ids_seen: set = set()
        for edge in self.edges:
            if edge.to_paper == paper_id:
                # Get the source paper's accuracy claims (they claim to be "better than" this paper)
                for node in self.nodes.values():
                    if node.paper_id == edge.from_paper:
                        inbound_claims.append(node)
                paper_ids_seen.add(edge.from_paper)
        return inbound_claims

    # ─── Serialization ────────────────────────────────────────────────────────

    def to_dict(self) -> dict:
        return {
            "nodes": {cid: asdict(n) for cid, n in self.nodes.items()},
            "edges": [asdict(e) for e in self.edges],
            "_next_id": self._next_id,
            "_paper_claims": self._paper_claims,
            "exported_at": _now_iso(),
        }

    @classmethod
    def from_dict(cls, data: dict) -> "ClaimGraph":
        g = cls()
        g._next_id = data.get("_next_id", 0)
        g._paper_claims = data.get("_paper_claims", {})
        for cid, ndict in data.get("nodes", {}).items():
            ndict = dict(ndict)
            ndict["claim_type"] = ClaimType(ndict["claim_type"])
            ndict["comparison_op"] = ComparisonOp(ndict["comparison_op"])
            g.nodes[cid] = ClaimNode(**ndict)
        for edict in data.get("edges", []):
            edict = dict(edict)
            edict["claim_type"] = ClaimType(edict["claim_type"])
            g.edges.append(ClaimEdge(**edict))
        return g

    def save(self, path: Optional[Path] = None) -> Path:
        """Persist graph to JSON file."""
        path = path or (GP_DIR / "claim_graph.json")
        GP_DIR.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(self.to_dict(), indent=2, ensure_ascii=False), encoding="utf-8")
        return path

    @classmethod
    def load(cls, path: Optional[Path] = None) -> "ClaimGraph":
        """Load graph from JSON file, or return empty graph if not found."""
        path = path or (GP_DIR / "claim_graph.json")
        if not path.exists():
            return cls()
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
            return cls.from_dict(data)
        except Exception:
            return cls()


# ─── Claim extraction from Gene Pool ──────────────────────────────────────────


def extract_claims_from_gene_pool(
    tracker=None,
) -> ClaimGraph:
    """Extract all numerical claims encoded in Gene Pool capsules.

    Reads CapsuleGene entries from tracker and converts numerical
    claim data into ClaimGraph nodes.
    """
    graph = ClaimGraph()

    try:
        if tracker is None:
            from llm.insight.tracker import EvolutionTracker
            tracker = EvolutionTracker(data_dir=GP_DIR)
    except Exception:
        return graph

    try:
        capsules = tracker.list_capsules(limit=1000)
        for cap in capsules:
            _add_capsule_claims_to_graph(graph, cap)
    except Exception:
        pass

    return graph


def _add_capsule_claims_to_graph(graph: ClaimGraph, capsule: Any) -> None:
    """Extract numerical claims from a single CapsuleGene and add to graph."""
    try:
        archetype = getattr(capsule, "archetype", {}) or {}
        if isinstance(archetype, str):
            try:
                archetype = json.loads(archetype)
            except Exception:
                archetype = {}

        keywords = getattr(capsule, "trigger_keywords", []) or []
        _extract_from_keywords(graph, str(getattr(capsule, "trigger_topic", "")), keywords)

        section_refs = archetype.get("paper_section_refs", [])
        for ref in section_refs:
            if isinstance(ref, dict) and "claim" in ref:
                _parse_claim_from_dict(graph, str(getattr(capsule, "trigger_topic", "")), ref)

    except Exception:
        pass


def _extract_from_keywords(graph: ClaimGraph, paper_id: str, keywords: List[str]) -> None:
    """Extract numerical claims from trigger keywords list."""
    claim_patterns = [
        (r"(\d+(?:\.\d+)?)\s*(?:x|times)\s*faster", ClaimType.SPEEDUP, ComparisonOp.GTE),
        (r"(\d+(?:\.\d+)?)\s*(?:%%|%)\s*(?:accuracy|acc)", ClaimType.ACCURACY, ComparisonOp.GTE),
        (r"(\d+(?:\.\d+)?)\s*(?:%%|%)\s*(?:reduction|reduce)", ClaimType.REDUCTION, ComparisonOp.LTE),
        (r"(\d+(?:\.\d+)?)\s*(?:M|B|K)\s*params?", ClaimType.PARAM_SIZE, ComparisonOp.LTE),
    ]
    for kw in keywords:
        for pat, ctype, op in claim_patterns:
            m = re.search(pat, kw.lower())
            if m:
                val = float(m.group(1))
                if ctype == ClaimType.SPEEDUP:
                    graph.add_claim(paper_id, ctype, val, ComparisonOp.GTE, kw)
                elif ctype == ClaimType.ACCURACY:
                    graph.add_claim(paper_id, ctype, val / 100.0, ComparisonOp.GTE, kw)
                elif ctype == ClaimType.REDUCTION:
                    graph.add_claim(paper_id, ctype, val, ComparisonOp.LTE, kw)


def _parse_claim_from_dict(graph: ClaimGraph, paper_id: str, ref: dict) -> None:
    """Parse a claim from a paper_section_ref dict."""
    claim = ref.get("claim", "")
    if not claim:
        return
    m = re.search(r"([\d.]+)\s*(?:%%|%|$)", claim)
    if m:
        val = float(m.group(1))
        if val > 1:
            val = val / 100.0
        graph.add_claim(paper_id, ClaimType.ACCURACY, val, ComparisonOp.GTE, claim)


# ─── HTML visualization ────────────────────────────────────────────────────────


def render_claim_graph_html(
    graph: Optional[ClaimGraph] = None,
    contradictions: Optional[List[Contradiction]] = None,
) -> str:
    """Render ClaimGraph as a D3.js directed graph HTML.

    Nodes = papers (aggregated claim counts)
    Edges = improvement claims between papers
    Contradictions highlighted in red
    """
    if graph is None:
        graph = ClaimGraph.load()
    if contradictions is None:
        contradictions = graph.find_contradictions()

    # Aggregate: one node per paper
    paper_ids = sorted(set(n.paper_id for n in graph.nodes.values()))
    paper_stats: Dict[str, dict] = {}
    for pid in paper_ids:
        claims = graph.get_paper_claims(pid)
        by_type: Dict[str, int] = {}
        for c in claims:
            by_type.setdefault(c.claim_type.value, 0)
            by_type[c.claim_type.value] += 1
        paper_stats[pid] = {
            "count": len(claims),
            "by_type": by_type,
            "top_claim": claims[0].source_text[:80] if claims else "",
        }

    edge_list = []
    for e in graph.edges:
        edge_list.append({
            "from": e.from_paper,
            "to": e.to_paper,
            "type": e.claim_type.value,
            "ratio": e.improvement_ratio,
            "label": f"{e.improvement_ratio:.2f}x",
        })

    nodes = [
        {"id": pid, "count": stats["count"], "by_type": stats["by_type"], "top_claim": stats["top_claim"]}
        for pid, stats in paper_stats.items()
    ]

    graph_data = {
        "nodes": nodes,
        "edges": edge_list,
        "contradictions": [
            {
                "paper_a": c.claim_a.paper_id,
                "paper_b": c.claim_b.paper_id,
                "metric": c.metric,
                "description": c.description,
                "severity": c.severity,
            }
            for c in contradictions
        ],
    }

    data_json = json.dumps(graph_data, ensure_ascii=False)

    # Build contradiction HTML safely via DOM API
    contra_items_html = _build_contradiction_items(contradictions)
    node_items_html = _build_node_items(nodes)

    html = f"""<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>Paper Claim Graph</title>
<script src="https://d3js.org/d3.v7.min.js"></script>
<style>
  body {{ margin: 0; font-family: sans-serif; background: #0d1117; color: #e6edf3; }}
  #graph {{ width: 100vw; height: 100vh; }}
  .node circle {{ stroke: #58a6ff; stroke-width: 2px; fill: #1f6feb; cursor: pointer; }}
  .node text {{ fill: #e6edf3; font-size: 11px; pointer-events: none; }}
  .edge line {{ stroke: #30363d; stroke-width: 1.5px; }}
  .edge.highlighted line {{ stroke: #f85149; stroke-width: 2px; }}
  #sidebar {{ position: fixed; top: 10px; right: 10px; width: 300px; max-height: 90vh;
    overflow-y: auto; background: #161b22; border: 1px solid #30363d; border-radius: 6px;
    padding: 12px; font-size: 13px; }}
  #sidebar h3 {{ margin: 0 0 8px; color: #f85149; }}
  .claim-item {{ background: #21262d; border-radius: 4px; padding: 6px 8px; margin-bottom: 6px; }}
  .metric-tag {{ display: inline-block; background: #1f6feb; border-radius: 3px; padding: 1px 5px;
    font-size: 10px; margin-right: 4px; }}
  .node-item {{ background: #21262d; border-radius: 4px; padding: 6px 8px; margin-bottom: 6px; }}
  .no-issues {{ color: #3fb950; }}
</style>
</head>
<body>
<div id="graph"></div>
<div id="sidebar">
  <h3 id="contra-header"></h3>
  <div id="contradiction-list"></div>
  <h3 style="color:#58a6ff;margin-top:16px">&#128202; Paper Nodes</h3>
  <div id="node-list"></div>
</div>
<script>
const data = {data_json};
const contraHeader = document.getElementById("contra-header");
const contraList = document.getElementById("contradiction-list");
const nodeList = document.getElementById("node-list");

// Set contradiction header
contraHeader.textContent = "⚠️ " + data.contradictions.length + " Contradiction(s)";

// Build contradiction list via DOM (safe, no innerHTML with user data)
if (data.contradictions.length === 0) {{
  const p = document.createElement("p");
  p.className = "no-issues";
  p.textContent = "No contradictions found";
  contraList.appendChild(p);
}} else {{
  data.contradictions.forEach(function(c) {{
    const div = document.createElement("div");
    div.className = "claim-item";

    const tag = document.createElement("span");
    tag.className = "metric-tag";
    tag.textContent = c.metric;

    const boldA = document.createElement("b");
    boldA.textContent = c.paper_a;

    const boldB = document.createElement("b");
    boldB.textContent = c.paper_b;

    const br = document.createElement("br");

    const small = document.createElement("small");
    small.style.color = "#8b949e";
    small.textContent = c.description;

    div.appendChild(tag);
    div.appendChild(boldA);
    div.appendChild(document.createTextNode(" vs "));
    div.appendChild(boldB);
    div.appendChild(br);
    div.appendChild(small);
    contraList.appendChild(div);
  }});
}}

// Build node list
data.nodes.slice(0, 20).forEach(function(n) {{
  const div = document.createElement("div");
  div.className = "node-item";
  const b = document.createElement("b");
  b.textContent = n.id;
  div.appendChild(b);
  div.appendChild(document.createTextNode(" (" + n.count + " claims)"));
  nodeList.appendChild(div);
}});
if (data.nodes.length > 20) {{
  const div = document.createElement("div");
  div.innerHTML = "<small style=\"color:#8b949e\">…and " + (data.nodes.length - 20) + " more</small>";
  nodeList.appendChild(div);
}}

const width = window.innerWidth - 340;
const height = window.innerHeight;

const svg = d3.select("#graph").append("svg")
  .attr("width", width).attr("height", height);

const simulation = d3.forceSimulation(data.nodes)
  .force("link", d3.forceLink(data.edges).id(function(d) {{ return d.id; }}).distance(120))
  .force("charge", d3.forceManyBody().strength(-300))
  .force("center", d3.forceCenter(width / 2, height / 2))
  .force("collision", d3.forceCollide().radius(40));

const edge = svg.append("g").selectAll("line")
  .data(data.edges).join("line")
  .attr("stroke", function(d) {{
    const isContra = data.contradictions.some(function(c) {{
      return (c.paper_a === d.from && c.paper_b === d.to) ||
             (c.paper_a === d.to && c.paper_b === d.from);
    }});
    return isContra ? "#f85149" : "#30363d";
  }})
  .attr("stroke-width", 1.5);

const edgeLabel = svg.append("g").selectAll("text")
  .data(data.edges).join("text")
  .attr("font-size", "10px").attr("fill", "#8b949e")
  .text(function(d) {{ return d.label || ""; }});

const node = svg.append("g").selectAll("g")
  .data(data.nodes).join("g")
  .attr("class", "node")
  .call(d3.drag()
    .on("start", function(event, d) {{
      if (!event.active) simulation.alphaTarget(0.3).restart();
      d.fx = d.x; d.fy = d.y;
    }})
    .on("drag", function(event, d) {{
      d.fx = event.x; d.fy = event.y;
    }})
    .on("end", function(event, d) {{
      if (!event.active) simulation.alphaTarget(0);
      d.fx = null; d.fy = null;
    }}));

node.append("circle").attr("r", function(d) {{ return 20 + d.count * 3; }});
node.append("text").attr("text-anchor", "middle").attr("dy", "0.35em")
  .text(function(d) {{ return d.id.length > 8 ? d.id.substring(0, 8) + "…" : d.id; }});

node.append("title").text(function(d) {{
  return "arXiv: " + d.id + "\\n" + d.count + " claim(s)\\n" + d.top_claim;
}});

simulation.on("tick", function() {{
  edge
    .attr("x1", function(d) {{ return d.source.x; }})
    .attr("y1", function(d) {{ return d.source.y; }})
    .attr("x2", function(d) {{ return d.target.x; }})
    .attr("y2", function(d) {{ return d.target.y; }});
  edgeLabel.attr("transform", function(d) {{
    return "translate(" + ((d.source.x + d.target.x) / 2) + "," + ((d.source.y + d.target.y) / 2) + ")";
  }});
  node.attr("transform", function(d) {{ return "translate(" + d.x + "," + d.y + ")"; }});
}});
</script>
</body>
</html>"""
    return html


def _build_contradiction_items(contradictions: List[Contradiction]) -> str:
    """Build HTML string for contradiction list (server-side fallback)."""
    if not contradictions:
        return '<p class="no-issues">No contradictions found</p>'
    parts = []
    for c in contradictions:
        parts.append(
            f'<div class="claim-item">'
            f'<span class="metric-tag">{c.metric}</span>'
            f'<b>{c.claim_a.paper_id}</b> vs <b>{c.claim_b.paper_id}</b>'
            f'<br><small style="color:#8b949e">{c.description}</small>'
            f'</div>'
        )
    return "".join(parts)


def _build_node_items(nodes: List[dict]) -> str:
    """Build HTML string for node list."""
    visible = nodes[:20]
    parts = []
    for n in visible:
        parts.append(f'<div class="node-item"><b>{n["id"]}</b> ({n["count"]} claims)</div>')
    if len(nodes) > 20:
        parts.append(f'<small style="color:#8b949e">…and {len(nodes) - 20} more</small>')
    return "".join(parts)


# ─── Utility ───────────────────────────────────────────────────────────────────


def _now_iso() -> str:
    return datetime.utcnow().isoformat()


# ─── MCP tool actions ─────────────────────────────────────────────────────────


def claim_graph_action(
    action: str = "status",
    paper_id: Optional[str] = None,
    claim_type: Optional[str] = None,
    value: Optional[float] = None,
    source_text: Optional[str] = None,
    from_paper: Optional[str] = None,
    to_paper: Optional[str] = None,
    improvement_ratio: Optional[float] = None,
) -> dict:
    """MCP tool action dispatcher for claim_graph.

    Actions:
      status     — load graph, return summary stats
      add_claim  — add a single claim node
      add_edge   — add a cross-paper improvement edge
      contradictions — find and return all contradictions
      render     — return full HTML for D3 visualization
      export     — save graph to disk, return path
      import_capsules — re-extract claims from Gene Pool capsules
    """
    graph = ClaimGraph.load()

    if action == "status":
        contradictions = graph.find_contradictions()
        return {
            "total_claims": len(graph.nodes),
            "total_edges": len(graph.edges),
            "total_papers": len(set(n.paper_id for n in graph.nodes.values())),
            "by_type": {
                str(ct.value): len([n for n in graph.nodes.values() if n.claim_type == ct])
                for ct in ClaimType
            },
            "contradictions_count": len(contradictions),
            "saved_at": str(GP_DIR / "claim_graph.json"),
        }

    elif action == "add_claim":
        if not paper_id or value is None or not source_text:
            return {"error": "paper_id, value, and source_text are required for add_claim"}
        ct = ClaimType(claim_type) if claim_type else ClaimType.OTHER
        op = ComparisonOp.GTE if ct == ClaimType.ACCURACY else ComparisonOp.LTE
        claim_id = graph.add_claim(paper_id, ct, float(value), op, source_text)
        path = graph.save()
        return {"added": claim_id, "paper_id": paper_id, "saved_to": str(path)}

    elif action == "add_edge":
        if not from_paper or not to_paper or improvement_ratio is None:
            return {"error": "from_paper, to_paper, and improvement_ratio are required"}
        ct = ClaimType(claim_type) if claim_type else ClaimType.OTHER
        graph.add_edge(from_paper, to_paper, ct, float(improvement_ratio), source_text or "")
        path = graph.save()
        return {"added_edge": f"{from_paper} → {to_paper}", "ratio": improvement_ratio, "saved_to": str(path)}

    elif action == "contradictions":
        contradictions = graph.find_contradictions()
        return {
            "contradictions": [
                {
                    "paper_a": c.claim_a.paper_id,
                    "paper_b": c.claim_b.paper_id,
                    "metric": c.metric,
                    "description": c.description,
                    "severity": c.severity,
                    "value_a": c.claim_a.value,
                    "value_b": c.claim_b.value,
                }
                for c in contradictions
            ],
            "total": len(contradictions),
        }

    elif action == "render":
        html = render_claim_graph_html(graph)
        return {"html": html, "size_kb": round(len(html) / 1024, 1)}

    elif action == "export":
        path = graph.save()
        return {"saved_to": str(path), "nodes": len(graph.nodes), "edges": len(graph.edges)}

    elif action == "import_capsules":
        extracted_graph = extract_claims_from_gene_pool()
        for cid, node in extracted_graph.nodes.items():
            if cid not in graph.nodes:
                graph.nodes[cid] = node
                graph._paper_claims.setdefault(node.paper_id, []).append(cid)
        graph._next_id = max(graph._next_id, extracted_graph._next_id)
        for edge in extracted_graph.edges:
            if edge not in graph.edges:
                graph.edges.append(edge)
        path = graph.save()
        return {
            "imported": len(extracted_graph.nodes),
            "total_nodes": len(graph.nodes),
            "saved_to": str(path),
        }

    else:
        return {"error": f"Unknown action: {action}"}
