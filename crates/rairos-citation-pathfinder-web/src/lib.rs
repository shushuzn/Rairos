use std::collections::HashMap;

pub const CAPSULES_PATH: &str = ".ai_research_os/gene_pool/capsules.json";

pub static GAP_COLORS: Lazy<HashMap<&str, &str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("theoretical_gap", "#C4706A");
    m.insert("method_limitation", "#D4A055");
    m.insert("evaluation_gap", "#6B8FB5");
    m.insert("scalability_issue", "#7BAD7B");
    m.insert("dataset_gap", "#9B8EC4");
    m.insert("generalization_gap", "#C4946A");
    m.insert("contradiction", "#E07070");
    m.insert("unexplored_application", "#6BBF8A");
    m
});

use once_cell::sync::Lazy;

#[derive(Debug, Clone)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub node_type: String,
    pub x: i32,
    pub y: i32,
    pub gap_type: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub style: Option<String>,
}

#[derive(Debug, Clone)]
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
    let mut nodes: Vec<GraphNode> = Vec::new();
    let mut edges: Vec<GraphEdge> = Vec::new();

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
            label: format!("Paper {}", &cpid[..cpid.len().min(8)]),
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
        let x = 100 + (i as i32 % 3) * 220;
        let y = 360 + (i as i32 / 3) * 120;
        let gap_type = "unknown";
        let color = GAP_COLORS.get(gap_type).copied().unwrap_or("#A89E8C");
        nodes.push(GraphNode {
            id: ccid.clone(),
            label: ccid.chars().take(40).collect(),
            node_type: "capsule".to_string(),
            x,
            y,
            gap_type: Some(gap_type.to_string()),
            color: Some(color.to_string()),
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
    graph: Option<&CitationGraph>,
    paper_id: &str,
    paper_title: &str,
    cited_paper_ids: &[String],
    cited_capsule_ids: &[String],
) -> String {
    let graph = match graph {
        Some(g) => g,
        None => {
            let g = build_citation_graph(paper_id, paper_title, cited_paper_ids, cited_capsule_ids);
            return render_citation_graph_svg(Some(&g), paper_id, paper_title, cited_paper_ids, cited_capsule_ids);
        }
    };

    let node_map: HashMap<&str, &GraphNode> = graph.nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    let svg_nodes: Vec<String> = graph.nodes.iter().map(|n| {
        match n.node_type.as_str() {
            "source_paper" => format!(
                "<g transform='translate({},{})'><circle r='28' fill='#2a4a6a' opacity='0.9'/><text text-anchor='middle' dy='.35em' fill='white' font-size='16'>P</text><text text-anchor='middle' y='42' font-size='10' fill='#444'>{}</text></g>",
                n.x, n.y, n.label
            ),
            "cited_paper" => format!(
                "<g transform='translate({},{})'><rect x='-40' y='-14' width='80' height='28' rx='4' fill='#e8e4dc' stroke='#ccc'/><text text-anchor='middle' dy='.35em' font-size='9' fill='#555'>{}</text></g>",
                n.x, n.y, n.label
            ),
            "capsule" => {
                let color = n.color.as_deref().unwrap_or("#A89E8C");
                let gap_type = n.gap_type.as_deref().unwrap_or("unknown");
                let label = n.label.chars().take(25).collect::<String>();
                format!(
                    "<g transform='translate({},{})'><rect x='-60' y='-18' width='120' height='36' rx='5' fill='{}' opacity='0.15' stroke='{}' stroke-width='1.5'/><text text-anchor='middle' dy='-.3em' font-size='9' font-weight='600' fill='{}'>{}</text><text text-anchor='middle' dy='1em' font-size='8' fill='#444'>{}</text></g>",
                    n.x, n.y, color, color, color, gap_type.replace('_', " "), label
                )
            },
            _ => String::new(),
        }
    }).collect();

    let svg_edges: Vec<String> = graph.edges.iter().filter_map(|e| {
        let from_node = node_map.get(e.from.as_str())?;
        let to_node = node_map.get(e.to.as_str())?;
        let style = if e.style.as_deref() == Some("dashed") {
            "stroke-dasharray='4,3'"
        } else {
            ""
        };
        Some(format!(
            "<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='#aaa' stroke-width='1.2' {}/>",
            from_node.x, from_node.y, to_node.x, to_node.y, style
        ))
    }).collect();

    let all_nodes_svg = svg_nodes.join("\n    ");
    let all_edges_svg = svg_edges.join("\n    ");

    let legend_items: String = GAP_COLORS.iter().map(|(gt, c)| {
        format!(
            "<rect x='0' y='{}' width='10' height='10' rx='2' fill='{}'/><text x='14' y='{}' font-size='10' fill='#555'>{}</text>",
            0,
            c,
            0,
            gt.replace('_', " ")
        )
    }).collect::<Vec<_>>().join("\n    ");

    let n_legend = GAP_COLORS.len();
    format!(
        "<svg width='700' height='550' xmlns='http://www.w3.org/2000/svg' style='font-family:Georgia,serif'><g transform='translate(10,10)'><text x='0' y='0' font-size='11' font-weight='700' fill='#333' dy='-2'>Legend</text>{}<rect x='0' y='{}' width='10' height='10' fill='#2a4a6a' rx='1'/><text x='14' y='{}' font-size='10' fill='#555'>Source Paper</text><rect x='0' y='{}' width='10' height='10' fill='#e8e4dc' stroke='#ccc' rx='1'/><text x='14' y='{}' font-size='10' fill='#555'>Cited Paper</text></g><g>{}</g><g>{}</g></svg>",
        legend_items,
        n_legend * 18 + 8,
        n_legend * 18 + 17,
        n_legend * 18 + 22,
        n_legend * 18 + 31,
        all_edges_svg,
        all_nodes_svg
    )
}

pub fn render_citation_chain_html(
    paper_id: &str,
    paper_title: &str,
    cited_paper_ids: &[String],
    cited_capsule_ids: &[String],
) -> String {
    let graph_svg = render_citation_graph_svg(
        None,
        paper_id,
        paper_title,
        cited_paper_ids,
        cited_capsule_ids,
    );

    let mut lines: Vec<String> = Vec::new();
    lines.push("<div class=\"citation-pathfinder\">".to_string());
    lines.push("<h3>Citation Pathfinder</h3>".to_string());
    lines.push(
        "<p style='font-size:13px;color:#A89E8C;margin-bottom:14px'>Paper -> cited references -> Gene Pool capsules</p>".to_string()
    );
    lines.push(format!("<div style='overflow:auto'>{}</div>", graph_svg));

    if !cited_capsule_ids.is_empty() {
        lines.push("<div style='margin-top:16px'>".to_string());
        lines.push("<h4 style='font-size:13px;font-weight:700;color:#333;margin-bottom:8px'>Gene Pool Capsules Cited</h4>".to_string());
        for ccid in cited_capsule_ids.iter().take(6) {
            lines.push(format!(
                "<div style='border-left:3px solid #A89E8C;padding-left:10px;margin-bottom:8px'><div style='font-size:12px;font-weight:600;color:#2a2a2a'>{}</div><div style='font-size:11px;color:#A89E8C'>unknown</div></div>",
                &ccid[..ccid.len().min(70)]
            ));
        }
        lines.push("</div>".to_string());
    }

    lines.push("<style>.citation-pathfinder { font-family: Georgia, serif; }</style>".to_string());
    lines.push("</div>".to_string());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gap_colors_not_empty() {
        assert!(!GAP_COLORS.is_empty());
        assert!(GAP_COLORS.contains_key("theoretical_gap"));
        assert!(GAP_COLORS.contains_key("method_limitation"));
    }

    #[test]
    fn test_build_citation_graph_empty() {
        let g = build_citation_graph("paper1", "Test Paper", &[], &[]);
        assert_eq!(g.nodes.len(), 1);
        assert_eq!(g.edges.len(), 0);
        assert_eq!(g.nodes[0].node_type, "source_paper");
    }

    #[test]
    fn test_build_citation_graph_with_cited() {
        let cited = vec!["paper2".to_string(), "paper3".to_string()];
        let g = build_citation_graph("paper1", "Test Paper", &cited, &[]);
        assert_eq!(g.nodes.len(), 3);
        assert_eq!(g.edges.len(), 2);
    }

    #[test]
    fn test_build_citation_graph_with_capsules() {
        let capsules = vec!["cap1".to_string(), "cap2".to_string()];
        let g = build_citation_graph("paper1", "Test Paper", &[], &capsules);
        assert_eq!(g.nodes.len(), 3);
        assert_eq!(g.edges.len(), 2);
    }

    #[test]
    fn test_render_citation_graph_svg_empty() {
        let g = build_citation_graph("paper1", "Test Paper", &[], &[]);
        let svg = render_citation_graph_svg(Some(&g), "paper1", "Test Paper", &[], &[]);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("citation-pathfinder") == false);
    }

    #[test]
    fn test_render_citation_chain_html() {
        let html = render_citation_chain_html("paper1", "Test Paper", &[], &[]);
        assert!(html.contains("citation-pathfinder"));
        assert!(html.contains("Citation Pathfinder"));
    }

    #[test]
    fn test_render_citation_chain_html_with_capsules() {
        let capsules = vec!["cap1".to_string()];
        let html = render_citation_chain_html("paper1", "Test Paper", &[], &capsules);
        assert!(html.contains("Gene Pool Capsules Cited"));
    }
}
