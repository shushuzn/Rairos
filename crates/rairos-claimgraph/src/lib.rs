//! Claim Graph — cross-paper numerical claim tracking and contradiction detection.
//!
//! Python original: `research_loop/claim_graph.py` (847 lines)

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

// ─── Enums ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClaimType {
    Accuracy,
    Speedup,
    Reduction,
    ParamSize,
    Memory,
    Other,
}

impl ClaimType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ClaimType::Accuracy => "accuracy",
            ClaimType::Speedup => "speedup",
            ClaimType::Reduction => "reduction",
            ClaimType::ParamSize => "param_size",
            ClaimType::Memory => "memory",
            ClaimType::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComparisonOp {
    Gte,
    Lte,
    Eq,
}

impl ComparisonOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            ComparisonOp::Gte => ">=",
            ComparisonOp::Lte => "<=",
            ComparisonOp::Eq => "==",
        }
    }
}

// ─── Data structs ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimNode {
    pub claim_id: String,
    pub paper_id: String,
    pub claim_type: ClaimType,
    pub value: f64,
    pub comparison_op: ComparisonOp,
    #[serde(rename = "source_text")]
    pub source_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_ref: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub char_start: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub char_end: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimEdge {
    #[serde(rename = "from_paper")]
    pub from_paper: String,
    #[serde(rename = "to_paper")]
    pub to_paper: String,
    #[serde(rename = "claim_type")]
    pub claim_type: ClaimType,
    #[serde(rename = "improvement_ratio")]
    pub improvement_ratio: f64,
    #[serde(rename = "source_text")]
    pub source_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contradiction {
    #[serde(rename = "claim_a")]
    pub claim_a: ClaimNode,
    #[serde(rename = "claim_b")]
    pub claim_b: ClaimNode,
    pub metric: String,
    pub description: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BidirectionalContradiction {
    #[serde(rename = "paper_a")]
    pub paper_a: String,
    #[serde(rename = "paper_b")]
    pub paper_b: String,
    #[serde(rename = "edge_ab")]
    pub edge_ab: ClaimEdge,
    #[serde(rename = "edge_ba")]
    pub edge_ba: ClaimEdge,
    pub severity: String,
    pub description: String,
}

// ─── ClaimGraph ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ClaimGraph {
    nodes: HashMap<String, ClaimNode>,
    edges: Vec<ClaimEdge>,
    _next_id: usize,
    _paper_claims: HashMap<String, Vec<String>>,
}

impl Default for ClaimGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl ClaimGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            _next_id: 0,
            _paper_claims: HashMap::new(),
        }
    }

    pub fn add_claim(
        &mut self,
        paper_id: &str,
        claim_type: ClaimType,
        value: f64,
        comparison_op: ComparisonOp,
        source_text: &str,
        page_ref: Option<i32>,
        char_start: Option<i32>,
        char_end: Option<i32>,
    ) -> String {
        let claim_id = format!("n{}", self._next_id);
        self._next_id += 1;
        let node = ClaimNode {
            claim_id: claim_id.clone(),
            paper_id: paper_id.to_string(),
            claim_type,
            value,
            comparison_op,
            source_text: source_text.chars().take(200).collect(),
            page_ref,
            char_start,
            char_end,
        };
        self.nodes.insert(claim_id.clone(), node);
        self._paper_claims
            .entry(paper_id.to_string())
            .or_default()
            .push(claim_id.clone());
        claim_id
    }

    pub fn add_edge(
        &mut self,
        from_paper: &str,
        to_paper: &str,
        claim_type: ClaimType,
        improvement_ratio: f64,
        source_text: &str,
    ) {
        self.edges.push(ClaimEdge {
            from_paper: from_paper.to_string(),
            to_paper: to_paper.to_string(),
            claim_type,
            improvement_ratio,
            source_text: source_text.to_string(),
        });
    }

    pub fn find_contradictions(&self) -> Vec<Contradiction> {
        let mut contradictions = Vec::new();
        let mut by_type: HashMap<ClaimType, Vec<&ClaimNode>> = HashMap::new();
        for node in self.nodes.values() {
            by_type.entry(node.claim_type).or_default().push(node);
        }

        for (claim_type, claims) in by_type {
            if !matches!(
                claim_type,
                ClaimType::Accuracy | ClaimType::Speedup | ClaimType::Reduction
            ) {
                continue;
            }

            for i in 0..claims.len() {
                for j in (i + 1)..claims.len() {
                    let ca = claims[i];
                    let cb = claims[j];
                    if ca.paper_id == cb.paper_id {
                        continue;
                    }
                    if ca.comparison_op != cb.comparison_op {
                        let diff = (ca.value - cb.value).abs();
                        let avg = (ca.value + cb.value) / 2.0;
                        let severity = if avg > 0.0 && diff / avg > 0.05 {
                            "high"
                        } else {
                            "medium"
                        };
                        contradictions.push(Contradiction {
                            claim_a: ca.clone(),
                            claim_b: cb.clone(),
                            metric: claim_type.as_str().to_string(),
                            description: format!(
                                "Paper {} claims {} {} but paper {} claims {} {} on {} — {:.1}% gap",
                                ca.paper_id,
                                ca.comparison_op.as_str(),
                                ca.value,
                                cb.paper_id,
                                cb.comparison_op.as_str(),
                                cb.value,
                                claim_type.as_str(),
                                diff / avg * 100.0
                            ),
                            severity: severity.to_string(),
                        });
                    }
                }
            }
        }
        contradictions
    }

    pub fn find_bidirectional_contradictions(&self) -> Vec<BidirectionalContradiction> {
        let mut contradictions = Vec::new();
        let mut edge_map: HashMap<(String, String), &ClaimEdge> = HashMap::new();
        for edge in &self.edges {
            edge_map.insert(
                (edge.from_paper.clone(), edge.to_paper.clone()),
                edge,
            );
        }

        let mut seen: HashSet<String> = HashSet::new();
        for ((from_a, to_b), edge_ab) in &edge_map {
            let reverse_key = (to_b.clone(), from_a.clone());
            if let Some(edge_ba) = edge_map.get(&reverse_key) {
                let pair_key = if from_a < to_b {
                    format!("{}:{}", from_a, to_b)
                } else {
                    format!("{}:{}", to_b, from_a)
                };
                if seen.contains(&pair_key) {
                    continue;
                }
                seen.insert(pair_key);

                if edge_ab.claim_type != edge_ba.claim_type {
                    continue;
                }

                let metric = edge_ab.claim_type.as_str();
                let diff_ratio =
                    (edge_ab.improvement_ratio - edge_ba.improvement_ratio).abs()
                        / edge_ab
                            .improvement_ratio
                            .max(edge_ba.improvement_ratio)
                            .max(1.0);

                let avg_ratio =
                    (edge_ab.improvement_ratio + edge_ba.improvement_ratio) / 2.0;
                let severity = if avg_ratio > 1.2 && diff_ratio > 0.15 {
                    "critical"
                } else if avg_ratio > 1.1 {
                    "high"
                } else {
                    "medium"
                };

                contradictions.push(BidirectionalContradiction {
                    paper_a: if from_a < to_b {
                        from_a.clone()
                    } else {
                        to_b.clone()
                    },
                    paper_b: if from_a < to_b {
                        to_b.clone()
                    } else {
                        from_a.clone()
                    },
                    edge_ab: (*edge_ab).clone(),
                    edge_ba: (*edge_ba).clone(),
                    severity: severity.to_string(),
                    description: format!(
                        "{} claims {:.2}x {} improvement over {}, but {} claims {:.2}x {} improvement over {} — {} bidirectional contradiction",
                        from_a,
                        edge_ab.improvement_ratio,
                        metric,
                        to_b,
                        to_b,
                        edge_ba.improvement_ratio,
                        metric,
                        from_a,
                        severity
                    ),
                });
            }
        }

        contradictions.sort_by_key(|c| match c.severity.as_str() {
            "critical" => 0,
            "high" => 1,
            _ => 2,
        });
        contradictions
    }

    pub fn get_paper_claims(&self, paper_id: &str) -> Vec<ClaimNode> {
        self._paper_claims
            .get(paper_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.nodes.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_all_claims_by_type(&self, claim_type: ClaimType) -> Vec<ClaimNode> {
        self.nodes
            .values()
            .filter(|n| n.claim_type == claim_type)
            .cloned()
            .collect()
    }

    pub fn get_inbound_improvement_claims(&self, paper_id: &str) -> Vec<ClaimNode> {
        let paper_nodes: HashMap<&str, Vec<&ClaimNode>> =
            self.nodes.values().fold(HashMap::new(), |mut acc, n| {
                acc.entry(n.paper_id.as_str()).or_default().push(n);
                acc
            });

        let mut inbound = Vec::new();
        let mut seen_papers: HashSet<String> = HashSet::new();
        for edge in &self.edges {
            if edge.to_paper == paper_id && !seen_papers.contains(&edge.from_paper) {
                if let Some(nodes) = paper_nodes.get(edge.from_paper.as_str()) {
                    for node in nodes {
                        inbound.push((*node).clone());
                    }
                }
                seen_papers.insert(edge.from_paper.clone());
            }
        }
        inbound
    }

    pub fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "nodes": &self.nodes,
            "edges": &self.edges,
            "_next_id": self._next_id,
            "_paper_claims": &self._paper_claims,
            "exported_at": gp_now_iso(),
        })
    }

    pub fn from_json_value(data: &serde_json::Value) -> Self {
        let mut g = Self::new();
        if let Some(v) = data.get("_next_id") {
            g._next_id = v.as_u64().unwrap_or(0) as usize;
        }
        if let Some(v) = data.get("_paper_claims") {
            if let Some(obj) = v.as_object() {
                for (k, arr) in obj {
                    if let Some(list) = arr.as_array() {
                        let ids: Vec<String> =
                            list.iter().filter_map(|v| v.as_str().map(String::from)).collect();
                        g._paper_claims.insert(k.clone(), ids);
                    }
                }
            }
        }
        if let Some(nodes) = data.get("nodes").and_then(|v| v.as_object()) {
            for (cid, ndict) in nodes {
                if let Some(n) = parse_claim_node(ndict) {
                    g.nodes.insert(cid.clone(), n);
                }
            }
        }
        if let Some(edges) = data.get("edges").and_then(|v| v.as_array()) {
            for edict in edges {
                if let Some(e) = parse_claim_edge(edict) {
                    g.edges.push(e);
                }
            }
        }
        g
    }

    pub fn save(&self, path: Option<&PathBuf>) -> PathBuf {
        let default_path = gp_dir().join("claim_graph.json");
        let path = path.unwrap_or(&default_path);
        let _ = fs::create_dir_all(path.parent().unwrap_or(&PathBuf::from(".")));
        let json = serde_json::to_string_pretty(&self.to_json_value()).unwrap_or_default();
        let _ = fs::write(path, json);
        path.clone()
    }

    pub fn load(path: Option<&PathBuf>) -> Self {
        let default_path = gp_dir().join("claim_graph.json");
        let path = path.unwrap_or(&default_path);
        if !path.exists() {
            return Self::new();
        }
        let text = fs::read_to_string(path).ok();
        match text.and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()) {
            Some(data) => Self::from_json_value(&data),
            None => Self::new(),
        }
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

fn parse_claim_node(data: &serde_json::Value) -> Option<ClaimNode> {
    let obj = data.as_object()?;
    Some(ClaimNode {
        claim_id: obj.get("claim_id")?.as_str()?.to_string(),
        paper_id: obj.get("paper_id")?.as_str()?.to_string(),
        claim_type: match obj.get("claim_type")?.as_str()? {
            "accuracy" => ClaimType::Accuracy,
            "speedup" => ClaimType::Speedup,
            "reduction" => ClaimType::Reduction,
            "param_size" => ClaimType::ParamSize,
            "memory" => ClaimType::Memory,
            _ => ClaimType::Other,
        },
        value: obj.get("value")?.as_f64()?,
        comparison_op: match obj.get("comparison_op")?.as_str()? {
            ">=" => ComparisonOp::Gte,
            "<=" => ComparisonOp::Lte,
            "==" => ComparisonOp::Eq,
            _ => ComparisonOp::Gte,
        },
        source_text: obj.get("source_text")?.as_str()?.to_string(),
        page_ref: obj.get("page_ref").and_then(|v| v.as_i64()).map(|n| n as i32),
        char_start: obj.get("char_start").and_then(|v| v.as_i64()).map(|n| n as i32),
        char_end: obj.get("char_end").and_then(|v| v.as_i64()).map(|n| n as i32),
    })
}

fn parse_claim_edge(data: &serde_json::Value) -> Option<ClaimEdge> {
    let obj = data.as_object()?;
    Some(ClaimEdge {
        from_paper: obj.get("from_paper")?.as_str()?.to_string(),
        to_paper: obj.get("to_paper")?.as_str()?.to_string(),
        claim_type: match obj.get("claim_type")?.as_str()? {
            "accuracy" => ClaimType::Accuracy,
            "speedup" => ClaimType::Speedup,
            "reduction" => ClaimType::Reduction,
            "param_size" => ClaimType::ParamSize,
            "memory" => ClaimType::Memory,
            _ => ClaimType::Other,
        },
        improvement_ratio: obj.get("improvement_ratio")?.as_f64()?,
        source_text: obj.get("source_text")?.as_str()?.to_string(),
    })
}

// ─── HTML Visualization ─────────────────────────────────────────────────────────

pub fn render_claim_graph_html(graph: Option<&ClaimGraph>) -> String {
    let graph = graph.map(|g| g.clone()).unwrap_or_else(|| ClaimGraph::load(None));
    let contradictions = graph.find_contradictions();

    let mut paper_stats: HashMap<String, (usize, HashMap<String, usize>, String)> =
        HashMap::new();
    for node in graph.nodes.values() {
        let entry = paper_stats
            .entry(node.paper_id.clone())
            .or_insert_with(|| (0, HashMap::new(), String::new()));
        entry.0 += 1;
        *entry.1.entry(node.claim_type.as_str().to_string()).or_insert(0) += 1;
        if entry.2.is_empty() {
            entry.2 = node.source_text.chars().take(80).collect();
        }
    }

    let edge_list: Vec<serde_json::Value> = graph
        .edges
        .iter()
        .map(|e| {
            serde_json::json!({
                "from": e.from_paper,
                "to": e.to_paper,
                "type": e.claim_type.as_str(),
                "ratio": e.improvement_ratio,
                "label": format!("{:.2}x", e.improvement_ratio),
            })
        })
        .collect();

    let node_list: Vec<serde_json::Value> = paper_stats
        .iter()
        .map(|(pid, (count, by_type, top_claim))| {
            serde_json::json!({
                "id": pid,
                "count": count,
                "by_type": by_type,
                "top_claim": top_claim,
            })
        })
        .collect();

    let contradiction_list: Vec<serde_json::Value> = contradictions
        .iter()
        .map(|c| {
            serde_json::json!({
                "paper_a": c.claim_a.paper_id,
                "paper_b": c.claim_b.paper_id,
                "metric": c.metric,
                "description": c.description,
                "severity": c.severity,
            })
        })
        .collect();

    let graph_data = serde_json::json!({
        "nodes": node_list,
        "edges": edge_list,
        "contradictions": contradiction_list,
    });

    let data_json = serde_json::to_string(&graph_data).unwrap_or_default();
    let contra_count = contradictions.len();

    // Build static HTML shell, inject data_json as a JS variable
    format!(
        r##"<!DOCTYPE html>
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

contraHeader.textContent = "\u26a0 {contra_count} Contradiction(s)";

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
  const ell = "\u2026";
  div.innerHTML = "<small style=\"color:#8b949e\">" + ell + "and " + (data.nodes.length - 20) + " more</small>";
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
  .text(function(d) {{ return d.id.length > 8 ? d.id.substring(0, 8) + "\u2026" : d.id; }});

node.append("title").text(function(d) {{
  return "arXiv: " + d.id + "\n" + d.count + " claim(s)\n" + d.top_claim;
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
</html>"##
    )
}

// ─── MCP tool dispatcher ────────────────────────────────────────────────────────

pub fn claim_graph_action(
    action: &str,
    paper_id: Option<&str>,
    claim_type: Option<&str>,
    value: Option<f64>,
    source_text: Option<&str>,
    from_paper: Option<&str>,
    to_paper: Option<&str>,
    improvement_ratio: Option<f64>,
) -> serde_json::Value {
    let mut graph = ClaimGraph::load(None);

    match action {
        "status" => {
            let contradictions = graph.find_contradictions();
            let mut by_type: HashMap<String, usize> = HashMap::new();
            for node in graph.nodes.values() {
                *by_type.entry(node.claim_type.as_str().to_string()).or_insert(0) += 1;
            }
            serde_json::json!({
                "total_claims": graph.node_count(),
                "total_edges": graph.edge_count(),
                "total_papers": graph.nodes.values().map(|n| &n.paper_id).collect::<HashSet<_>>().len(),
                "by_type": by_type,
                "contradictions_count": contradictions.len(),
                "saved_at": gp_dir().join("claim_graph.json").to_string_lossy(),
            })
        }

        "add_claim" => {
            let (Some(pid), Some(val), Some(st)) = (paper_id, value, source_text) else {
                return serde_json::json!({ "error": "paper_id, value, and source_text are required for add_claim" });
            };
            let ct = parse_ctype(claim_type);
            let op = if ct == ClaimType::Accuracy {
                ComparisonOp::Gte
            } else {
                ComparisonOp::Lte
            };
            let claim_id = graph.add_claim(pid, ct, val, op, st, None, None, None);
            let path = graph.save(None);
            serde_json::json!({ "added": claim_id, "paper_id": pid, "saved_to": path.to_string_lossy() })
        }

        "add_edge" => {
            let (Some(fp), Some(tp), Some(ratio)) = (from_paper, to_paper, improvement_ratio) else {
                return serde_json::json!({ "error": "from_paper, to_paper, and improvement_ratio are required" });
            };
            let ct = parse_ctype(claim_type);
            graph.add_edge(fp, tp, ct, ratio, source_text.unwrap_or(""));
            let path = graph.save(None);
            serde_json::json!({
                "added_edge": format!("{} -> {}", fp, tp),
                "ratio": ratio,
                "saved_to": path.to_string_lossy(),
            })
        }

        "contradictions" => {
            let contradictions = graph.find_contradictions();
            serde_json::json!({
                "contradictions": contradictions.iter().map(|c| {
                    serde_json::json!({
                        "paper_a": c.claim_a.paper_id,
                        "paper_b": c.claim_b.paper_id,
                        "metric": c.metric,
                        "description": c.description,
                        "severity": c.severity,
                        "value_a": c.claim_a.value,
                        "value_b": c.claim_b.value,
                    })
                }).collect::<Vec<_>>(),
                "total": contradictions.len(),
            })
        }

        "bidirectional_contradictions" => {
            let bi = graph.find_bidirectional_contradictions();
            serde_json::json!({
                "contradictions": bi.iter().map(|c| {
                    serde_json::json!({
                        "paper_a": c.paper_a,
                        "paper_b": c.paper_b,
                        "metric": c.edge_ab.claim_type.as_str(),
                        "edge_ab_ratio": c.edge_ab.improvement_ratio,
                        "edge_ba_ratio": c.edge_ba.improvement_ratio,
                        "severity": c.severity,
                        "description": c.description,
                    })
                }).collect::<Vec<_>>(),
                "total": bi.len(),
                "critical_count": bi.iter().filter(|c| c.severity == "critical").count(),
            })
        }

        "render" => {
            let html = render_claim_graph_html(Some(&graph));
            serde_json::json!({ "html": html, "size_kb": html.len() as f64 / 1024.0 })
        }

        "export" => {
            let path = graph.save(None);
            serde_json::json!({
                "saved_to": path.to_string_lossy(),
                "nodes": graph.node_count(),
                "edges": graph.edge_count(),
            })
        }

        _ => serde_json::json!({ "error": format!("Unknown action: {}", action) }),
    }
}

// ─── Utilities ─────────────────────────────────────────────────────────────────

fn gp_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ai_research_os")
        .join("evolution")
}

fn gp_now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

fn parse_ctype(s: Option<&str>) -> ClaimType {
    match s.map(|x| x.to_lowercase()).as_deref() {
        Some("accuracy") => ClaimType::Accuracy,
        Some("speedup") => ClaimType::Speedup,
        Some("reduction") => ClaimType::Reduction,
        Some("param_size") => ClaimType::ParamSize,
        Some("memory") => ClaimType::Memory,
        _ => ClaimType::Other,
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_claim() {
        let mut graph = ClaimGraph::new();
        let id1 = graph.add_claim(
            "paperA", ClaimType::Accuracy, 0.95, ComparisonOp::Gte,
            "achieves 95% accuracy", None, None, None,
        );
        let id2 = graph.add_claim(
            "paperB", ClaimType::Accuracy, 0.88, ComparisonOp::Gte,
            "achieves 88% accuracy", None, None, None,
        );
        assert_eq!(graph.node_count(), 2);
        assert_eq!(id1, "n0");
        assert_eq!(id2, "n1");
    }

    #[test]
    fn test_contradiction_detection() {
        let mut graph = ClaimGraph::new();
        graph.add_claim("pA", ClaimType::Accuracy, 0.95, ComparisonOp::Gte, "test", None, None, None);
        graph.add_claim("pB", ClaimType::Accuracy, 0.90, ComparisonOp::Lte, "test", None, None, None);
        let contradictions = graph.find_contradictions();
        assert_eq!(contradictions.len(), 1);
        assert_eq!(contradictions[0].severity, "high");
    }

    #[test]
    fn test_bidirectional_contradiction() {
        let mut graph = ClaimGraph::new();
        graph.add_edge("A", "B", ClaimType::Accuracy, 1.2, "A claims 1.2x improvement");
        graph.add_edge("B", "A", ClaimType::Accuracy, 1.3, "B claims 1.3x improvement");
        let bi = graph.find_bidirectional_contradictions();
        assert_eq!(bi.len(), 1);
        assert_eq!(bi[0].paper_a, "A");
        assert_eq!(bi[0].paper_b, "B");
    }

    #[test]
    fn test_paper_claims() {
        let mut graph = ClaimGraph::new();
        graph.add_claim("p1", ClaimType::Accuracy, 0.95, ComparisonOp::Gte, "test", None, None, None);
        graph.add_claim("p1", ClaimType::Speedup, 2.0, ComparisonOp::Gte, "test", None, None, None);
        graph.add_claim("p2", ClaimType::Accuracy, 0.90, ComparisonOp::Gte, "test", None, None, None);
        let p1 = graph.get_paper_claims("p1");
        assert_eq!(p1.len(), 2);
        assert_eq!(graph.get_paper_claims("p2").len(), 1);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut graph = ClaimGraph::new();
        graph.add_claim("p1", ClaimType::Accuracy, 0.95, ComparisonOp::Gte, "test claim", None, None, None);
        graph.add_edge("p2", "p1", ClaimType::Speedup, 1.5, "p2 faster");
        let val = graph.to_json_value();
        let loaded = ClaimGraph::from_json_value(&val);
        assert_eq!(loaded.node_count(), 1);
        assert_eq!(loaded.edge_count(), 1);
    }
}
