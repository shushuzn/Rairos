//! Rairos KG — Knowledge Graph Manager
//!
//! Manages the paper knowledge graph: nodes, edges, and queries.
//! Replaces: kg/manager.py

use rairos_core::Paper;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum KgError {
    #[error("Node not found: {0}")]
    NodeNotFound(String),

    #[error("Edge not found")]
    EdgeNotFound,

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

// ============================================================================
// Node Types
// ============================================================================

/// A node in the knowledge graph (represents a paper)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KgNode {
    pub id: String,
    pub paper_id: String,
    pub label: String,
    pub node_type: String, // "paper", "author", "concept"
    pub properties: HashMap<String, String>,
}

impl KgNode {
    pub fn from_paper(paper: &Paper) -> Self {
        let mut props = HashMap::new();
        props.insert("title".to_string(), paper.title.clone());
        props.insert("arxiv_id".to_string(), paper.arxiv_id.clone().unwrap_or_default());
        props.insert("cited_by".to_string(), paper.metadata.cited_by.to_string());

        Self {
            id: paper.id.clone(),
            paper_id: paper.id.clone(),
            label: paper.title.clone(),
            node_type: "paper".to_string(),
            properties: props,
        }
    }
}

/// An edge in the knowledge graph (relationship between papers)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KgEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub relation: String, // "cites", "references", "related_to", "contradicts"
    pub weight: f32,
    pub properties: HashMap<String, String>,
}

impl KgEdge {
    pub fn cites(source: &str, target: &str) -> Self {
        Self {
            id: format!("{}->{}", source, target),
            source: source.to_string(),
            target: target.to_string(),
            relation: "cites".to_string(),
            weight: 1.0,
            properties: HashMap::new(),
        }
    }

    pub fn related_to(source: &str, target: &str, weight: f32) -> Self {
        Self {
            id: format!("{}~{}", source, target),
            source: source.to_string(),
            target: target.to_string(),
            relation: "related_to".to_string(),
            weight,
            properties: HashMap::new(),
        }
    }
}

// ============================================================================
// Knowledge Graph
// ============================================================================

/// The knowledge graph
#[derive(Debug, Default)]
pub struct KnowledgeGraph {
    nodes: HashMap<String, KgNode>,
    edges: Vec<KgEdge>,
    // Adjacency list for fast lookups
    outgoing: HashMap<String, Vec<String>>,
    incoming: HashMap<String, Vec<String>>,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a paper as a node
    pub fn add_paper(&mut self, paper: &Paper) {
        let node = KgNode::from_paper(paper);
        self.add_node(node);
    }

    /// Add a node
    pub fn add_node(&mut self, node: KgNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    /// Add an edge
    pub fn add_edge(&mut self, edge: KgEdge) {
        // Check nodes exist
        if !self.nodes.contains_key(&edge.source) {
            tracing::warn!("Edge references unknown source node: {}", edge.source);
            return;
        }
        if !self.nodes.contains_key(&edge.target) {
            tracing::warn!("Edge references unknown target node: {}", edge.target);
            return;
        }

        self.edges.push(edge.clone());
        self.outgoing.entry(edge.source.clone()).or_default().push(edge.target.clone());
        self.incoming.entry(edge.target.clone()).or_default().push(edge.source.clone());
    }

    /// Add a citation edge
    pub fn add_citation(&mut self, source_id: &str, target_id: &str) {
        self.add_edge(KgEdge::cites(source_id, target_id));
    }

    /// Get a node by ID
    pub fn get_node(&self, id: &str) -> Option<&KgNode> {
        self.nodes.get(id)
    }

    /// Get all nodes
    pub fn nodes(&self) -> &HashMap<String, KgNode> {
        &self.nodes
    }

    /// Get all edges
    pub fn edges(&self) -> &[KgEdge] {
        &self.edges
    }

    /// Get papers that cite a given paper
    pub fn get_citing(&self, paper_id: &str) -> Vec<&KgNode> {
        self.incoming.get(paper_id)
            .map(|ids| ids.iter().filter_map(|id| self.nodes.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get papers cited by a given paper
    pub fn get_references(&self, paper_id: &str) -> Vec<&KgNode> {
        self.outgoing.get(paper_id)
            .map(|ids| ids.iter().filter_map(|id| self.nodes.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get papers related to a given paper
    pub fn get_related(&self, paper_id: &str) -> Vec<&KgNode> {
        self.edges.iter()
            .filter(|e| e.source == paper_id && e.relation == "related_to")
            .filter_map(|e| self.nodes.get(&e.target))
            .collect()
    }

    /// Find path between two papers (BFS)
    pub fn find_path(&self, start: &str, end: &str) -> Option<Vec<String>> {
        if !self.nodes.contains_key(start) || !self.nodes.contains_key(end) {
            return None;
        }

        let mut visited = HashSet::new();
        let mut queue = vec![vec![start.to_string()]];

        while let Some(path) = queue.pop() {
            let current = path.last().unwrap();
            if current == end {
                return Some(path);
            }

            if visited.contains(current) {
                continue;
            }
            visited.insert(current.clone());

            if let Some(neighbors) = self.outgoing.get(current) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        let mut new_path = path.clone();
                        new_path.push(neighbor.clone());
                        queue.push(new_path);
                    }
                }
            }
        }

        None
    }

    /// Get graph statistics
    pub fn stats(&self) -> KgStats {
        let node_count = self.nodes.len();
        let edge_count = self.edges.len();
        let avg_degree = if node_count > 0 {
            self.edges.len() as f32 / node_count as f32
        } else {
            0.0
        };

        let paper_nodes = self.nodes.values().filter(|n| n.node_type == "paper").count();
        let concept_nodes = self.nodes.values().filter(|n| n.node_type == "concept").count();

        KgStats {
            total_nodes: node_count,
            total_edges: edge_count,
            avg_degree,
            paper_nodes,
            concept_nodes,
        }
    }

    /// Export graph as JSON for visualization
    pub fn export_json(&self) -> serde_json::Value {
        serde_json::json!({
            "nodes": self.nodes.values().collect::<Vec<_>>(),
            "edges": self.edges,
        })
    }
}

/// Graph statistics
#[derive(Debug, Serialize)]
pub struct KgStats {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub avg_degree: f32,
    pub paper_nodes: usize,
    pub concept_nodes: usize,
}

// ============================================================================
// Graph Algorithms
// ============================================================================

pub struct GraphAlgorithms;

impl GraphAlgorithms {
    /// PageRank-like scoring for papers
    pub fn rank_papers(graph: &KnowledgeGraph) -> HashMap<String, f32> {
        let mut scores: HashMap<String, f32> = graph.nodes.keys()
            .map(|id| (id.clone(), 1.0))
            .collect();

        let damping = 0.85;
        let iterations = 20;

        for _ in 0..iterations {
            let mut new_scores: HashMap<String, f32> = HashMap::new();

            for (node_id, _) in &scores {
                let incoming = graph.incoming.get(node_id);
                let mut contribution = 0.0;

                if let Some(incoming_ids) = incoming {
                    for inc_id in incoming_ids {
                        let out_degree = graph.outgoing.get(inc_id).map(|v| v.len()).unwrap_or(1);
                        if out_degree > 0 {
                            contribution += scores.get(inc_id).unwrap_or(&0.0) / out_degree as f32;
                        }
                    }
                }

                new_scores.insert(
                    node_id.clone(),
                    (1.0 - damping) + damping * contribution,
                );
            }

            scores = new_scores;
        }

        scores
    }

    /// Find the most central paper (highest score)
    pub fn most_central(graph: &KnowledgeGraph) -> Option<(String, f32)> {
        let scores = Self::rank_papers(graph);
        scores.into_iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// Community detection (simple label propagation)
    pub fn detect_communities(graph: &KnowledgeGraph) -> HashMap<String, usize> {
        let mut communities: HashMap<String, usize> = graph.nodes.keys()
            .enumerate()
            .map(|(i, id)| (id.clone(), i))
            .collect();

        let mut changed = true;
        let mut iterations = 0;
        let max_iterations = 10;

        while changed && iterations < max_iterations {
            changed = false;
            iterations += 1;

            for node_id in graph.nodes.keys() {
                let neighbors = graph.outgoing.get(node_id)
                    .map(|v| v.as_slice())
                    .unwrap_or(&[]);

                if neighbors.is_empty() {
                    continue;
                }

                let mut label_counts: HashMap<usize, usize> = HashMap::new();
                for neighbor_id in neighbors {
                    if let Some(&label) = communities.get(neighbor_id) {
                        *label_counts.entry(label).or_insert(0) += 1;
                    }
                }

                if let Some(&current_label) = communities.get(node_id) {
                    let most_common = label_counts.into_iter().max_by_key(|(_, c)| *c);
                    if let Some((new_label, count)) = most_common {
                        if count > 1 && new_label != current_label {
                            communities.insert(node_id.clone(), new_label);
                            changed = true;
                        }
                    }
                }
            }
        }

        communities
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_paper() {
        let paper = Paper::new(
            Some("2301.00001".into()),
            "Test Paper".into(),
            "Abstract".into(),
        );

        let mut graph = KnowledgeGraph::new();
        graph.add_paper(&paper);

        assert_eq!(graph.nodes.len(), 1);
        let node = graph.get_node(&paper.id).unwrap();
        assert_eq!(node.label, "Test Paper");
    }

    #[test]
    fn test_citation_chain() {
        let p1 = Paper::new(Some("1".into()), "Paper 1".into(), "A".into());
        let p2 = Paper::new(Some("2".into()), "Paper 2".into(), "B".into());

        let mut graph = KnowledgeGraph::new();
        graph.add_paper(&p1);
        graph.add_paper(&p2);
        graph.add_citation(&p2.id, &p1.id);

        let citing = graph.get_citing(&p1.id);
        assert_eq!(citing.len(), 1);
        assert_eq!(citing[0].paper_id, p2.id);

        let refs = graph.get_references(&p2.id);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].paper_id, p1.id);
    }

    #[test]
    fn test_find_path() {
        let p1 = Paper::new(Some("1".into()), "Paper 1".into(), "A".into());
        let p2 = Paper::new(Some("2".into()), "Paper 2".into(), "B".into());
        let p3 = Paper::new(Some("3".into()), "Paper 3".into(), "C".into());

        let mut graph = KnowledgeGraph::new();
        graph.add_paper(&p1);
        graph.add_paper(&p2);
        graph.add_paper(&p3);
        graph.add_citation(&p2.id, &p1.id);
        graph.add_citation(&p3.id, &p2.id);

        let path = graph.find_path(&p3.id, &p1.id);
        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(path.len(), 3);
    }

    #[test]
    fn test_community_detection() {
        let p1 = Paper::new(Some("1".into()), "Paper 1".into(), "A".into());
        let p2 = Paper::new(Some("2".into()), "Paper 2".into(), "B".into());
        let p3 = Paper::new(Some("3".into()), "Paper 3".into(), "C".into());

        let mut graph = KnowledgeGraph::new();
        graph.add_paper(&p1);
        graph.add_paper(&p2);
        graph.add_paper(&p3);
        // p1 and p2 strongly connected, p3 isolated
        graph.add_citation(&p2.id, &p1.id);
        graph.add_citation(&p1.id, &p2.id);

        let communities = GraphAlgorithms::detect_communities(&graph);
        // p1 and p2 should share a community
        assert_eq!(communities.get(&p1.id), communities.get(&p2.id));
    }
}
