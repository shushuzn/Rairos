//! rairos-pathfinder — Citation Pathfinder Web Graph
//!
//! Interactive SVG citation chain visualization showing paper → cites → Gene Pool capsule chain,
//! color-coded by gap_type.
//!
//! Ported from `llm/citation_pathfinder_web.py`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

fn capsules_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ai_research_os")
        .join("gene_pool")
        .join("capsules.json")
}

static GAP_COLORS: &[(&str, &str)] = &[
    ("theoretical_gap", "#C4706A"),
    ("method_limitation", "#D4A055"),
    ("evaluation_gap", "#6B8FB5"),
    ("scalability_issue", "#7BAD7B"),
    ("dataset_gap", "#9B8EC4"),
    ("generalization_gap", "#C4946A"),
    ("contradiction", "#E07070"),
    ("unexplored_application", "#6BBF8A"),
];

fn load_capsules() -> Vec<serde_json::Value> {
    let path = capsules_path();
    if !path.exists() {
        return Vec::new();
    }
    match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|v| v.get("capsules").cloned())
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub x: i32,
    pub y: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

pub fn build_citation_graph(
    paper_id: &str,
    paper_title: &str,
    cited_paper_ids: &[String],
    cited_capsule_ids: &[String],
) -> CitationGraph {
    let capsules = load_capsules();
    let capsule_map: HashMap<String, &serde_json::Value> = capsules
        .iter()
        .filter_map(|c| {
            c.get("capsule_id")
                .and_then(|v| v.as_str())
                .map(|id| (id.to_string(), c))
        })
        .collect();

    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    nodes.push(GraphNode {
        id: paper_id.to_string(),
        label: paper_title.chars().take(50).collect(),
        node_type: "source_paper".to_string(),
        x: 300,
        y: 50,
        gap_type: None,
        color: None,
    });

    for (i, cpid) in cited_paper_ids.iter().take(8).enumerate() {
        let x = 120 + (i as i32 % 4) * 140;
        let y = 160 + (i as i32 / 4) * 80;
        nodes.push(GraphNode {
            id: cpid.clone(),
            label: format!("Paper {}", &cpid[..std::cmp::min(8, cpid.len())]),
            node_type: "cited_paper".to_string(),
            x,
            y,
            gap_type: None,
            color: None,
        });
        edges.push(GraphEdge {
            from: paper_id.to_string(),
            to: cpid.clone(),
            style: None,
        });
    }

    for (i, ccid) in cited_capsule_ids.iter().take(6).enumerate() {
        let cap = capsule_map.get(ccid);
        let gap_type = cap
            .and_then(|c| c.get("action_gap_type"))
            .and_then(|v| v.as_str())
            .or_else(|| {
                cap.and_then(|c| c.get("trigger_gap_type"))
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("unknown")
            .to_string();

        let color = GAP_COLORS
            .iter()
            .find(|(gt, _)| *gt == gap_type)
            .map(|(_, c)| *c)
            .unwrap_or("#A89E8C")
            .to_string();

        let x = 100 + (i as i32 % 3) * 220;
        let y = 360 + (i as i32 / 3) * 120;

        nodes.push(GraphNode {
            id: ccid.clone(),
            label: cap
                .and_then(|c| c.get("action_gap_title"))
                .and_then(|v| v.as_str())
                .unwrap_or(ccid)
                .chars()
                .take(40)
                .collect(),
            node_type: "capsule".to_string(),
            x,
            y,
            gap_type: Some(gap_type.clone()),
            color: Some(color.clone()),
        });
        edges.push(GraphEdge {
            from: paper_id.to_string(),
            to: ccid.clone(),
            style: Some("dashed".to_string()),
        });
    }

    CitationGraph { nodes, edges }
}

pub fn render_citation_graph_svg(
    graph_data: Option<&CitationGraph>,
    paper_id: &str,
    paper_title: &str,
    cited_paper_ids: Option<&[String]>,
    cited_capsule_ids: Option<&[String]>,
) -> String {
    let graph = match graph_data {
        Some(g) => g,
        None => {
            let g = build_citation_graph(
                paper_id,
                paper_title,
                cited_paper_ids.unwrap_or(&[]),
                cited_capsule_ids.unwrap_or(&[]),
            );
            return render_citation_graph_svg(Some(&g), paper_id, paper_title, None, None);
        }
    };

    let nodes = &graph.nodes;
    let edges = &graph.edges;
    let node_map: HashMap<&str, &GraphNode> =
        nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    let mut svg_nodes = Vec::new();
    for n in nodes {
        match n.node_type.as_str() {
            "source_paper" => {
                svg_nodes.push(format!(
                    "<g transform='translate({},{})'>\
                     <circle r='28' fill='#2a4a6a' opacity='0.9'/>\
                     <text text-anchor='middle' dy='.35em' fill='white' font-size='16'>📄</text>\
                     <text text-anchor='middle' y='42' font-size='10' fill='#444'>{}</text>\
                     </g>",
                    n.x, n.y, n.label
                ));
            }
            "cited_paper" => {
                svg_nodes.push(format!(
                    "<g transform='translate({},{})'>\
                     <rect x='-40' y='-14' width='80' height='28' rx='4' fill='#e8e4dc' stroke='#ccc'/>\
                     <text text-anchor='middle' dy='.35em' font-size='9' fill='#555'>{}</text>\
                     </g>",
                    n.x, n.y, n.label
                ));
            }
            "capsule" => {
                let color = n.color.as_deref().unwrap_or("#A89E8C");
                let gap_type = n.gap_type.as_deref().unwrap_or("unknown");
                svg_nodes.push(format!(
                    "<g transform='translate({},{})'>\
                     <rect x='-60' y='-18' width='120' height='36' rx='5' fill='{}' opacity='0.15' stroke='{}' stroke-width='1.5'/>\
                     <text text-anchor='middle' dy='-.3em' font-size='9' font-weight='600' fill='{}'>{}</text>\
                     <text text-anchor='middle' dy='1em' font-size='8' fill='#444'>{}</text>\
                     </g>",
                    n.x,
                    n.y,
                    color,
                    color,
                    color,
                    gap_type.replace('_', " "),
                    &n.label[..std::cmp::min(25, n.label.len())]
                ));
            }
            _ => {}
        }
    }

    let mut svg_edges = Vec::new();
    for e in edges {
        let frm = node_map.get(e.from.as_str());
        let to = node_map.get(e.to.as_str());
        if let (Some(f), Some(t)) = (frm, to) {
            let style = if e.style.as_deref() == Some("dashed") {
                "stroke-dasharray='4,3'"
            } else {
                ""
            };
            svg_edges.push(format!(
                "<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='#aaa' stroke-width='1.2' {}/>",
                f.x, f.y, t.x, t.y, style
            ));
        }
    }

    let all_nodes_svg = svg_nodes.join("\n    ");
    let all_edges_svg = svg_edges.join("\n    ");

    let n_legend = GAP_COLORS.len();
    let legend_items: String = GAP_COLORS
        .iter()
        .enumerate()
        .map(|(i, (gt, c))| {
            format!(
                "<rect x='0' y='{}' width='10' height='10' rx='2' fill='{}'/>\
                 <text x='14' y='{}' font-size='10' fill='#555'>{}</text>",
                i * 18,
                c,
                i * 18 + 9,
                gt.replace('_', " ")
            )
        })
        .collect();

    format!(
        "<svg width='700' height='550' xmlns='http://www.w3.org/2000/svg' style='font-family:Georgia,serif'>\
         <g transform='translate(10,10)'>\
         <text x='0' y='0' font-size='11' font-weight='700' fill='#333' dy='-2'>Legend</text>\
         {}\
         <rect x='0' y='{}' width='10' height='10' fill='#2a4a6a' rx='1'/>\
         <text x='14' y='{}' font-size='10' fill='#555'>Source Paper</text>\
         <rect x='0' y='{}' width='10' height='10' fill='#e8e4dc' stroke='#ccc' rx='1'/>\
         <text x='14' y='{}' font-size='10' fill='#555'>Cited Paper</text>\
         <rect x='0' y='{}' width='10' height='10' fill='none' stroke='#aaa' stroke-width='1' stroke-dasharray='4,3' rx='1'/>\
         <text x='14' y='{}' font-size='10' fill='#555'>→ Gene Pool Capsule</text>\
         </g>\
         <g transform='translate(10,10)'>\
         {}\
         {}\
         </g>\
         </svg>",
        legend_items,
        n_legend * 18 + 8,
        n_legend * 18 + 17,
        n_legend * 18 + 30,
        n_legend * 18 + 39,
        n_legend * 18 + 52,
        n_legend * 18 + 61,
        all_edges_svg,
        all_nodes_svg
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_citation_graph_empty() {
        let g = build_citation_graph("p1", "Test Paper", &[], &[]);
        assert_eq!(g.nodes.len(), 1);
        assert_eq!(g.nodes[0].id, "p1");
        assert!(g.edges.is_empty());
    }

    #[test]
    fn test_build_citation_graph_with_cited_papers() {
        let g = build_citation_graph(
            "p1",
            "Test Paper",
            &["cited1".to_string(), "cited2".to_string()],
            &[],
        );
        assert_eq!(g.nodes.len(), 3);
        assert_eq!(g.edges.len(), 2);
    }

    #[test]
    fn test_render_citation_graph_svg_empty() {
        let g = build_citation_graph("p1", "Test Paper", &[], &[]);
        let svg = render_citation_graph_svg(Some(&g), "p1", "Test Paper", None, None);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("Source Paper"));
    }

    #[test]
    fn test_gap_colors_defined() {
        assert_eq!(GAP_COLORS.len(), 8);
        assert!(GAP_COLORS.iter().any(|(k, _)| *k == "theoretical_gap"));
        assert!(GAP_COLORS.iter().any(|(k, _)| *k == "contradiction"));
    }
}