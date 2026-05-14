//! Citation Chain Builder — builds citation graphs using Semantic Scholar API.
//!
//! Mirrors llm/citation_chain.py

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

const S2_API: &str = "https://api.semanticscholar.org/graph/v1";

// ─── Data Types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationNode {
    pub paper_id: String,
    pub title: String,
    pub year: Option<i32>,
    pub citations: Vec<String>,
    pub references: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationChain {
    pub root_id: String,
    pub nodes: Vec<CitationNode>,
    pub edges: Vec<(String, String, String)>, // (source, target, relation)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchFamily {
    pub name: String,
    pub members: Vec<String>,
    pub common_topic: String,
}

// ─── Fetch paper data from Semantic Scholar ──────────────────────────────────

pub async fn fetch_paper(paper_id: &str) -> Result<CitationNode, String> {
    let url = format!("{}/paper/{}?fields=title,year,citations,references", S2_API, paper_id);
    let resp = reqwest::get(&url).await.map_err(|e| format!("S2 request failed: {}", e))?;
    let text = resp.text().await.map_err(|e| format!("S2 read failed: {}", e))?;

    #[derive(Deserialize)]
    struct S2Paper {
        paper_id: Option<String>,
        title: Option<String>,
        year: Option<i32>,
        citations: Option<Vec<S2Ref>>,
        references: Option<Vec<S2Ref>>,
    }
    #[derive(Deserialize)]
    struct S2Ref {
        paper_id: Option<String>,
        title: Option<String>,
    }

    let paper: S2Paper = serde_json::from_str(&text).map_err(|e| format!("S2 parse failed: {}", e))?;

    Ok(CitationNode {
        paper_id: paper.paper_id.unwrap_or_default(),
        title: paper.title.unwrap_or_default(),
        year: paper.year,
        citations: paper.citations.unwrap_or_default().into_iter()
            .filter_map(|c| c.paper_id).collect(),
        references: paper.references.unwrap_or_default().into_iter()
            .filter_map(|r| r.paper_id).collect(),
    })
}

/// Build a citation chain starting from a root paper, up to `depth` levels.
pub async fn build_chain(root_id: &str, depth: u32) -> Result<CitationChain, String> {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut visited = HashSet::new();
    let mut current_level = vec![root_id.to_string()];

    for _ in 0..=depth {
        let mut next_level = Vec::new();
        for pid in &current_level {
            if visited.contains(pid) { continue; }
            visited.insert(pid.clone());

            match fetch_paper(pid).await {
                Ok(node) => {
                    for ref_id in &node.references {
                        edges.push((pid.clone(), ref_id.clone(), "cites".to_string()));
                        if !visited.contains(ref_id) { next_level.push(ref_id.clone()); }
                    }
                    for cit_id in &node.citations {
                        edges.push((cit_id.clone(), pid.clone(), "cites".to_string()));
                        if !visited.contains(cit_id) { next_level.push(cit_id.clone()); }
                    }
                    nodes.push(node);
                }
                Err(_) => continue,
            }
        }
        current_level = next_level;
    }

    Ok(CitationChain {
        root_id: root_id.to_string(),
        nodes,
        edges,
    })
}

/// Detect research families from a citation chain (simple topic clustering).
pub fn find_families(chain: &CitationChain) -> Vec<ResearchFamily> {
    let mut families = Vec::new();
    let mut assigned: HashSet<String> = HashSet::new();

    for node in &chain.nodes {
        if assigned.contains(&node.paper_id) { continue; }

        // Find papers that cite or are cited by this paper
        let mut members = vec![node.paper_id.clone()];
        assigned.insert(node.paper_id.clone());

        for edge in &chain.edges {
            if edge.0 == node.paper_id && !assigned.contains(&edge.1) {
                members.push(edge.1.clone());
                assigned.insert(edge.1.clone());
            }
            if edge.1 == node.paper_id && !assigned.contains(&edge.0) {
                members.push(edge.0.clone());
                assigned.insert(edge.0.clone());
            }
        }

        if members.len() >= 2 {
            families.push(ResearchFamily {
                name: format!("Family {}", families.len() + 1),
                members,
                common_topic: chain.nodes.iter()
                    .find(|n| n.paper_id == node.paper_id)
                    .map(|n| n.title.clone())
                    .unwrap_or_default(),
            });
        }
    }

    families
}

/// Find silent citations (papers that cite without explicit mention).
pub fn find_silent(chain: &CitationChain) -> Vec<String> {
    let mut silent = Vec::new();
    let mut citation_counts: HashMap<String, usize> = HashMap::new();

    for edge in &chain.edges {
        *citation_counts.entry(edge.1.clone()).or_default() += 1;
    }

    for (pid, count) in &citation_counts {
        if *count >= 3 {
            silent.push(pid.clone());
        }
    }

    silent
}

/// Render chain as text.
pub fn render_text(chain: &CitationChain, max_nodes: usize) -> String {
    let mut lines = vec![format!("Citation Chain ({})", chain.root_id)];
    for node in chain.nodes.iter().take(max_nodes) {
        let cite_count = chain.edges.iter().filter(|e| e.0 == node.paper_id).count();
        let ref_count = chain.edges.iter().filter(|e| e.1 == node.paper_id).count();
        lines.push(format!("  {} — {} citations, {} references", node.title.chars().take(60).collect::<String>(), cite_count, ref_count));
    }
    if chain.nodes.len() > max_nodes {
        lines.push(format!("  ... and {} more", chain.nodes.len() - max_nodes));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_smoke() {
        let chain = CitationChain {
            root_id: "test".to_string(),
            nodes: vec![
                CitationNode {
                    paper_id: "p1".to_string(), title: "Paper 1".to_string(),
                    year: Some(2024), citations: vec!["p2".to_string()], references: vec![],
                },
                CitationNode {
                    paper_id: "p2".to_string(), title: "Paper 2".to_string(),
                    year: Some(2023), citations: vec![], references: vec!["p1".to_string()],
                },
            ],
            edges: vec![
                ("p2".to_string(), "p1".to_string(), "cites".to_string()),
            ],
        };

        let families = find_families(&chain);
        assert!(!families.is_empty(), "should find at least one family");

        let silent = find_silent(&chain);
        assert!(silent.is_empty(), "no silent citations expected");

        let text = render_text(&chain, 10);
        assert!(text.contains("Paper 1"));
    }

    #[test]
    #[ignore] // requires network access
    fn test_fetch_paper_invalid_id() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(fetch_paper("invalid_id_xyz"));
        assert!(result.is_err(), "invalid ID should return error");
    }
}
