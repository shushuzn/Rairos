use chrono::Utc;

#![allow(
    clippy::too_many_arguments,
)]
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimNode {
    pub claim_id: String,
    pub paper_id: String,
    pub claim_type: ClaimType,
    pub value: f64,
    pub comparison_op: ComparisonOp,
    pub source_text: String,
    pub page_ref: Option<u32>,
    pub char_start: Option<u32>,
    pub char_end: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimEdge {
    pub from_paper: String,
    pub to_paper: String,
    pub claim_type: ClaimType,
    pub improvement_ratio: f64,
    pub source_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contradiction {
    pub paper_a: String,
    pub paper_b: String,
    pub metric: String,
    pub description: String,
    pub severity: String,
    pub value_a: f64,
    pub value_b: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BidirectionalContradiction {
    pub paper_a: String,
    pub paper_b: String,
    pub metric: String,
    pub edge_ab_ratio: f64,
    pub edge_ba_ratio: f64,
    pub severity: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphData {
    pub nodes: Vec<serde_json::Value>,
    pub edges: Vec<serde_json::Value>,
    pub contradictions: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClaimGraphSerde {
    nodes: HashMap<String, ClaimNode>,
    edges: Vec<ClaimEdge>,
    _next_id: u32,
    _paper_claims: HashMap<String, Vec<String>>,
    exported_at: String,
}

#[derive(Debug, Clone)]
pub struct ClaimGraph {
    nodes: HashMap<String, ClaimNode>,
    edges: Vec<ClaimEdge>,
    next_id: u32,
    paper_claims: HashMap<String, Vec<String>>,
    data_dir: PathBuf,
}

impl Default for ClaimGraph {
    fn default() -> Self {
        let data_dir = dirs_next()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".ai_research_os")
            .join("evolution");
        Self::new(data_dir)
    }
}

impl ClaimGraph {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            next_id: 0,
            paper_claims: HashMap::new(),
            data_dir,
        }
    }

    pub fn add_claim(
        &mut self,
        paper_id: &str,
        claim_type: ClaimType,
        value: f64,
        comparison_op: ComparisonOp,
        source_text: &str,
        page_ref: Option<u32>,
        char_start: Option<u32>,
        char_end: Option<u32>,
    ) -> String {
        let claim_id = format!("n{}", self.next_id);
        self.next_id += 1;
        let node = ClaimNode {
            claim_id: claim_id.clone(),
            paper_id: paper_id.to_string(),
            claim_type,
            value,
            comparison_op,
            source_text: source_text.to_string(),
            page_ref,
            char_start,
            char_end,
        };
        self.nodes.insert(claim_id.clone(), node);
        self.paper_claims
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
        let mut by_type: HashMap<ClaimType, Vec<&ClaimNode>> = HashMap::new();
        for node in self.nodes.values() {
            by_type.entry(node.claim_type).or_default().push(node);
        }

        let mut contradictions = Vec::new();
        for (claim_type, claims) in &by_type {
            match claim_type {
                ClaimType::Accuracy | ClaimType::Speedup | ClaimType::Reduction => {}
                _ => continue,
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
                        let severity = if avg > 0.0 && (diff / avg) > 0.05 {
                            "high"
                        } else {
                            "medium"
                        };
                        contradictions.push(Contradiction {
                            paper_a: ca.paper_id.clone(),
                            paper_b: cb.paper_id.clone(),
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
                                if avg > 0.0 { diff / avg * 100.0 } else { 0.0 }
                            ),
                            severity: severity.to_string(),
                            value_a: ca.value,
                            value_b: cb.value,
                        });
                    }
                }
            }
        }
        contradictions
    }

    pub fn find_bidirectional_contradictions(&self) -> Vec<BidirectionalContradiction> {
        let mut edge_map: HashMap<(String, String), &ClaimEdge> = HashMap::new();
        for edge in &self.edges {
            edge_map.insert((edge.from_paper.clone(), edge.to_paper.clone()), edge);
        }

        let mut result = Vec::new();
        let mut seen: HashSet<(String, String)> = HashSet::new();
        for ((from_a, to_b), edge_ab) in &edge_map {
            let reverse_key = (to_b.clone(), from_a.clone());
            let Some(edge_ba) = edge_map.get(&reverse_key) else {
                continue;
            };
            let pair_key = if from_a < to_b {
                (from_a.clone(), to_b.clone())
            } else {
                (to_b.clone(), from_a.clone())
            };
            if !seen.insert(pair_key) {
                continue;
            }
            if edge_ab.claim_type != edge_ba.claim_type {
                continue;
            }
            let metric = edge_ab.claim_type.as_str();
            let max_ratio = edge_ab
                .improvement_ratio
                .max(edge_ba.improvement_ratio)
                .max(1.0);
            let diff_ratio =
                (edge_ab.improvement_ratio - edge_ba.improvement_ratio).abs() / max_ratio;
            let avg_ratio = (edge_ab.improvement_ratio + edge_ba.improvement_ratio) / 2.0;
            let severity = if avg_ratio > 1.2 && diff_ratio > 0.15 {
                "critical"
            } else if avg_ratio > 1.1 {
                "high"
            } else {
                "medium"
            };
            result.push(BidirectionalContradiction {
                paper_a: from_a.clone(),
                paper_b: to_b.clone(),
                metric: metric.to_string(),
                edge_ab_ratio: edge_ab.improvement_ratio,
                edge_ba_ratio: edge_ba.improvement_ratio,
                severity: severity.to_string(),
                description: format!(
                    "{} claims {:.2}x {} improvement over {}, but {} claims {:.2}x {} improvement over {} — {} bidirectional contradiction",
                    from_a, edge_ab.improvement_ratio, metric, to_b,
                    to_b, edge_ba.improvement_ratio, metric, from_a, severity
                ),
            });
        }
        result.sort_by_key(|c| match c.severity.as_str() {
            "critical" => 0,
            "high" => 1,
            _ => 2,
        });
        result
    }

    pub fn get_paper_claims(&self, paper_id: &str) -> Vec<&ClaimNode> {
        self.paper_claims
            .get(paper_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.nodes.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn stats(&self) -> serde_json::Value {
        let contradictions = self.find_contradictions();
        let mut by_type: HashMap<String, usize> = HashMap::new();
        for node in self.nodes.values() {
            *by_type.entry(node.claim_type.as_str().to_string()).or_default() += 1;
        }
        let unique_papers: HashSet<&str> =
            self.nodes.values().map(|n| n.paper_id.as_str()).collect();
        serde_json::json!({
            "total_claims": self.nodes.len(),
            "total_edges": self.edges.len(),
            "total_papers": unique_papers.len(),
            "by_type": by_type,
            "contradictions_count": contradictions.len(),
        })
    }

    pub fn to_graph_data(&self) -> GraphData {
        let contradictions = self.find_contradictions();
        let mut paper_stats: HashMap<&str, (usize, HashMap<&str, usize>, &str)> = HashMap::new();
        for node in self.nodes.values() {
            let entry = paper_stats
                .entry(&node.paper_id)
                .or_insert((0, HashMap::new(), ""));
            entry.0 += 1;
            *entry.1.entry(node.claim_type.as_str()).or_default() += 1;
            if entry.2.is_empty() {
                entry.2 = &node.source_text;
            }
        }

        let nodes: Vec<serde_json::Value> = paper_stats
            .iter()
            .map(|(pid, (count, by_type, top_claim))| {
                serde_json::json!({
                    "id": pid,
                    "count": count,
                    "by_type": by_type,
                    "top_claim": &top_claim[..top_claim.len().min(80)],
                })
            })
            .collect();

        let edges: Vec<serde_json::Value> = self
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

        let contra_json: Vec<serde_json::Value> = contradictions
            .iter()
            .map(|c| {
                serde_json::json!({
                    "paper_a": c.paper_a,
                    "paper_b": c.paper_b,
                    "metric": c.metric,
                    "description": c.description,
                    "severity": c.severity,
                })
            })
            .collect();

        GraphData {
            nodes,
            edges,
            contradictions: contra_json,
        }
    }

    pub fn render_html(&self) -> String {
        let data = self.to_graph_data();
        let data_json = serde_json::to_string(&data).unwrap_or_default();
        format!(r##"<!DOCTYPE html>
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
const nContra = data.contradictions.length;
contraHeader.textContent = "\u26a0 " + nContra + " Contradiction(s)";
if (nContra === 0) {{
  const p = document.createElement("p");
  p.className = "no-issues";
  p.textContent = "No contradictions found";
  contraList.appendChild(p);
}} else {{
  data.contradictions.forEach(function(c) {{
    const div = document.createElement("div");
    div.className = "claim-item";
    [c.metric, c.paper_a, c.paper_b, c.description].forEach(function(t) {{
      const el = document.createElement(["span","b","b","small"][arguments.length-1]);
      if (arguments.length === 1) el.className = "metric-tag";
      el.textContent = t;
      div.appendChild(el);
      if (arguments.length === 2 || arguments.length === 3) div.appendChild(document.createTextNode(" vs "));
      if (arguments.length === 4) {{ const br = document.createElement("br"); div.appendChild(br); }};
    }});
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
  const remain = data.nodes.length - 20;
  const div = document.createElement("div");
  const small = document.createElement("small");
  small.style.color = "#8b949e";
  small.textContent = "...and " + remain + " more";
  div.appendChild(small);
  nodeList.appendChild(div);
}}
const w = window.innerWidth - 340;
const h = window.innerHeight;
const svg = d3.select("#graph").append("svg").attr("width",w).attr("height",h);
const sim = d3.forceSimulation(data.nodes)
  .force("link", d3.forceLink(data.edges).id(function(d){{return d.id;}}).distance(120))
  .force("charge", d3.forceManyBody().strength(-300))
  .force("center", d3.forceCenter(w/2,h/2))
  .force("collision", d3.forceCollide().radius(40));
const edge = svg.append("g").selectAll("line").data(data.edges).join("line")
  .attr("stroke",function(d){{return data.contradictions.some(function(c){{return(c.paper_a===d.from&&c.paper_b===d.to)||(c.paper_a===d.to&&c.paper_b===d.from);}})?"#f85149":"#30363d";}})
  .attr("stroke-width",1.5);
const edgeLabel = svg.append("g").selectAll("text").data(data.edges).join("text")
  .attr("font-size","10px").attr("fill","#8b949e").text(function(d){{return d.label||"";}});
const node = svg.append("g").selectAll("g").data(data.nodes).join("g").attr("class","node")
  .call(d3.drag().on("start",function(e,d){{if(!e.active)sim.alphaTarget(0.3).restart();d.fx=d.x;d.fy=d.y;}})
    .on("drag",function(e,d){{d.fx=e.x;d.fy=e.y;}})
    .on("end",function(e,d){{if(!e.active)sim.alphaTarget(0);d.fx=null;d.fy=null;}}));
node.append("circle").attr("r",function(d){{return 20+d.count*3;}});
node.append("text").attr("text-anchor","middle").attr("dy","0.35em")
  .text(function(d){{return d.id.length>8?d.id.substring(0,8)+"...":d.id;}});
node.append("title").text(function(d){{return"arXiv: "+d.id+"\\n"+d.count+" claim(s)\\n"+d.top_claim;}});
sim.on("tick",function(){{
  edge.attr("x1",function(d){{return d.source.x;}}).attr("y1",function(d){{return d.source.y;}})
    .attr("x2",function(d){{return d.target.x;}}).attr("y2",function(d){{return d.target.y;}});
  edgeLabel.attr("transform",function(d){{return"translate("+((d.source.x+d.target.x)/2)+","+((d.source.y+d.target.y)/2)+")";}});
  node.attr("transform",function(d){{return"translate("+d.x+","+d.y+")";}});
}});
</script>
</body>
</html>"##)
    }

    pub fn save(&self) -> PathBuf {
        let path = self.data_dir.join("claim_graph.json");
        fs::create_dir_all(&self.data_dir).ok();
        let serde_data = ClaimGraphSerde {
            nodes: self.nodes.clone(),
            edges: self.edges.clone(),
            _next_id: self.next_id,
            _paper_claims: self.paper_claims.clone(),
            exported_at: Utc::now().to_rfc3339(),
        };
        let json = serde_json::to_string_pretty(&serde_data).unwrap_or_default();
        fs::write(&path, json).ok();
        path
    }

    pub fn load(data_dir: Option<PathBuf>) -> Self {
        let dir = data_dir.unwrap_or_else(|| {
            dirs_next()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".ai_research_os")
                .join("evolution")
        });
        let path = dir.join("claim_graph.json");
        if !path.exists() {
            return Self::new(dir);
        }
        let Ok(json) = fs::read_to_string(&path) else {
            return Self::new(dir);
        };
        let Ok(data) = serde_json::from_str::<ClaimGraphSerde>(&json) else {
            return Self::new(dir);
        };
        Self {
            nodes: data.nodes,
            edges: data.edges,
            next_id: data._next_id,
            paper_claims: data._paper_claims,
            data_dir: dir,
        }
    }
}

fn dirs_next() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| std::env::var("USERPROFILE").ok().map(PathBuf::from))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_graph() -> ClaimGraph {
        let mut g = ClaimGraph::new(PathBuf::from("/tmp/_test_cg"));
        g.add_claim("paper1", ClaimType::Accuracy, 0.95, ComparisonOp::Gte, "accuracy 95%", None, None, None);
        g.add_claim("paper2", ClaimType::Accuracy, 0.88, ComparisonOp::Lte, "accuracy <=88%", None, None, None);
        g.add_claim("paper2", ClaimType::Speedup, 2.5, ComparisonOp::Gte, "2.5x speedup", None, None, None);
        g.add_edge("paper2", "paper1", ClaimType::Speedup, 1.3, "1.3x improvement");
        g.add_edge("paper1", "paper2", ClaimType::Speedup, 1.1, "1.1x improvement");
        g
    }

    #[test]
    fn test_add_claim() {
        let mut g = ClaimGraph::new(PathBuf::from("/tmp/_test"));
        let id = g.add_claim("p1", ClaimType::Accuracy, 0.95, ComparisonOp::Gte, "test", None, None, None);
        assert_eq!(g.nodes.len(), 1);
        assert!(id.starts_with("n"));
    }

    #[test]
    fn test_find_contradictions() {
        let g = make_graph();
        let contra = g.find_contradictions();
        assert_eq!(contra.len(), 1);
        assert_eq!(contra[0].metric, "accuracy");
    }

    #[test]
    fn test_find_bidirectional() {
        let g = make_graph();
        let bi = g.find_bidirectional_contradictions();
        assert_eq!(bi.len(), 1);
        // Floating-point precision may give "high" or "critical"
        let valid = bi[0].severity == "high" || bi[0].severity == "critical";
        assert!(valid, "severity should be high or critical, got {}", bi[0].severity);
        assert_eq!(bi[0].metric, "speedup");
    }

    #[test]
    fn test_get_paper_claims() {
        let g = make_graph();
        let claims = g.get_paper_claims("paper2");
        assert_eq!(claims.len(), 2);
    }

    #[test]
    fn test_stats() {
        let g = make_graph();
        let s = g.stats();
        assert_eq!(s["total_claims"].as_u64(), Some(3));
        assert_eq!(s["total_edges"].as_u64(), Some(2));
    }

    #[test]
    fn test_save_load() {
        let dir = tempfile::tempdir().unwrap();
        let mut g = ClaimGraph::new(dir.path().to_path_buf());
        g.add_claim("p1", ClaimType::Accuracy, 0.9, ComparisonOp::Gte, "acc", None, None, None);
        g.save();
        let loaded = ClaimGraph::load(Some(dir.path().to_path_buf()));
        assert_eq!(loaded.nodes.len(), 1);
    }

    #[test]
    fn test_load_nonexistent() {
        let dir = tempfile::tempdir().unwrap();
        let g = ClaimGraph::load(Some(dir.path().to_path_buf()));
        assert!(g.nodes.is_empty());
    }

    #[test]
    fn test_render_html() {
        let g = make_graph();
        let html = g.render_html();
        assert!(html.contains("Paper Claim Graph"));
        assert!(html.contains("d3.v7.min.js"));
    }

    #[test]
    fn test_to_graph_data() {
        let g = make_graph();
        let data = g.to_graph_data();
        assert_eq!(data.nodes.len(), 2);
        assert_eq!(data.edges.len(), 2);
        assert_eq!(data.contradictions.len(), 1);
    }
}
