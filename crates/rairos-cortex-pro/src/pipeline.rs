//! LangGraph-style pipeline orchestration.

use std::collections::{HashMap, VecDeque};
use serde::{Deserialize, Serialize};
use crate::error::CortexProError;

/// A node in the pipeline graph
#[derive(Debug, Clone)]
pub struct PipelineNode {
    /// Node ID
    pub id: String,
    /// Node type (agent role or conditional)
    pub node_type: PipelineNodeType,
    /// Next nodes (for sequential) or conditional branches
    pub next: Vec<String>,
    /// Whether this is a terminal node
    pub is_terminal: bool,
}

impl PipelineNode {
    pub fn new(id: impl Into<String>, node_type: PipelineNodeType) -> Self {
        Self {
            id: id.into(),
            node_type,
            next: vec![],
            is_terminal: false,
        }
    }

    pub fn with_next(mut self, next: impl Into<String>) -> Self {
        self.next.push(next.into());
        self
    }

    pub fn terminal(mut self) -> Self {
        self.is_terminal = true;
        self
    }
}

/// Type of pipeline node
#[derive(Debug, Clone)]
pub enum PipelineNodeType {
    /// An agent execution node
    Agent(String),
    /// A conditional branch
    Conditional(String),
    /// A state update node
    Update(String),
    /// A merge node (join)
    Merge,
    /// Start node
    Start,
    /// End node
    End,
}

/// An edge in the pipeline graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineEdge {
    /// Source node ID
    pub from: String,
    /// Target node ID
    pub to: String,
    /// Condition for this edge (for conditional edges)
    pub condition: Option<String>,
}

impl PipelineEdge {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: impl Into<String>) -> Self {
        self.condition = Some(condition.into());
        self
    }
}

/// A directed acyclic graph (DAG) pipeline for orchestration
#[derive(Debug, Clone)]
pub struct Pipeline {
    /// Pipeline ID
    pub id: String,
    /// Nodes in the pipeline
    nodes: HashMap<String, PipelineNode>,
    /// Edges in the pipeline
    edges: Vec<PipelineEdge>,
    /// Entry node ID
    pub entry: String,
    /// Terminal node IDs
    pub terminals: Vec<String>,
}

impl Pipeline {
    /// Create a new empty pipeline
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            nodes: HashMap::new(),
            edges: vec![],
            entry: String::new(),
            terminals: vec![],
        }
    }

    /// Add a node to the pipeline
    pub fn add_node(&mut self, node: PipelineNode) -> &mut Self {
        if self.entry.is_empty() {
            self.entry = node.id.clone();
        }
        if node.is_terminal {
            self.terminals.push(node.id.clone());
        }
        self.nodes.insert(node.id.clone(), node);
        self
    }

    /// Add an edge to the pipeline
    pub fn add_edge(&mut self, edge: PipelineEdge) -> &mut Self {
        // Ensure both nodes exist
        if !self.nodes.contains_key(&edge.from) {
            tracing::warn!("Edge from unknown node: {}", edge.from);
        }
        if !self.nodes.contains_key(&edge.to) {
            tracing::warn!("Edge to unknown node: {}", edge.to);
        }
        self.edges.push(edge);
        self
    }

    /// Get a node by ID
    pub fn get_node(&self, id: &str) -> Option<&PipelineNode> {
        self.nodes.get(id)
    }

    /// Get all edges from a node
    pub fn get_edges_from(&self, node_id: &str) -> Vec<&PipelineEdge> {
        self.edges.iter().filter(|e| e.from == node_id).collect()
    }

    /// Validate the pipeline (check for cycles, missing nodes, etc.)
    pub fn validate(&self) -> Result<(), CortexProError> {
        // Check entry exists
        if self.entry.is_empty() {
            return Err(CortexProError::InvalidPipeline("Pipeline has no entry node".to_string()));
        }

        if !self.nodes.contains_key(&self.entry) {
            return Err(CortexProError::InvalidPipeline(format!(
                "Entry node '{}' not found",
                self.entry
            )));
        }

        // Check all edges reference existing nodes
        for edge in &self.edges {
            if !self.nodes.contains_key(&edge.from) {
                return Err(CortexProError::InvalidPipeline(format!(
                    "Edge from unknown node: {}",
                    edge.from
                )));
            }
            if !self.nodes.contains_key(&edge.to) {
                return Err(CortexProError::InvalidPipeline(format!(
                    "Edge to unknown node: {}",
                    edge.to
                )));
            }
        }

        // Check for cycles using DFS
        self.detect_cycles()?;

        Ok(())
    }

    /// Detect cycles in the pipeline graph
    fn detect_cycles(&self) -> Result<(), CortexProError> {
        let mut visited: HashMap<String, bool> = HashMap::new();
        let mut rec_stack: HashMap<String, bool> = HashMap::new();

        for node_id in self.nodes.keys() {
            if self.has_cycle_dfs(node_id, &mut visited, &mut rec_stack) {
                return Err(CortexProError::InvalidPipeline(format!(
                    "Cycle detected in pipeline, starting from node: {}",
                    node_id
                )));
            }
        }

        Ok(())
    }

    fn has_cycle_dfs(
        &self,
        node_id: &str,
        visited: &mut HashMap<String, bool>,
        rec_stack: &mut HashMap<String, bool>,
    ) -> bool {
        visited.insert(node_id.to_string(), true);
        rec_stack.insert(node_id.to_string(), true);

        for edge in self.get_edges_from(node_id) {
            let target = &edge.to;
            let is_visited = visited.get(target).copied().unwrap_or(false);
            let in_rec = rec_stack.get(target).copied().unwrap_or(false);

            if is_visited && in_rec {
                return true; // Cycle found
            }

            if !is_visited && self.has_cycle_dfs(target, visited, rec_stack) {
                return true;
            }
        }

        rec_stack.insert(node_id.to_string(), false);
        false
    }

    /// Get execution order (topological sort)
    pub fn execution_order(&self) -> Result<Vec<String>, CortexProError> {
        self.validate()?;

        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();

        // Initialize
        for node_id in self.nodes.keys() {
            in_degree.insert(node_id.clone(), 0);
            adjacency.insert(node_id.clone(), vec![]);
        }

        // Build graph
        for edge in &self.edges {
            *in_degree.entry(edge.to.clone()).or_insert(0) += 1;
            adjacency.entry(edge.from.clone()).or_default().push(edge.to.clone());
        }

        // Kahn's algorithm
        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(n, _)| n.clone())
            .collect();

        let mut result = Vec::new();

        while let Some(node_id) = queue.pop_front() {
            result.push(node_id.clone());

            if let Some(neighbors) = adjacency.get(&node_id) {
                for neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(neighbor) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(neighbor.clone());
                        }
                    }
                }
            }
        }

        if result.len() != self.nodes.len() {
            return Err(CortexProError::InvalidPipeline(
                "Could not compute topological order (graph may have cycles)".to_string(),
            ));
        }

        Ok(result)
    }
}

/// Predefined pipeline templates
pub mod templates {
    use super::*;

    /// Create a simple sequential research pipeline
    pub fn sequential_research() -> Pipeline {
        let mut pipeline = Pipeline::new("research_sequential");

        pipeline.add_node(PipelineNode::new("start", PipelineNodeType::Start));
        pipeline.add_node(PipelineNode::new("search", PipelineNodeType::Agent("researcher".to_string())));
        pipeline.add_node(PipelineNode::new("extract", PipelineNodeType::Agent("researcher".to_string())));
        pipeline.add_node(PipelineNode::new("analyze", PipelineNodeType::Agent("gap_analyzer".to_string())));
        pipeline.add_node(PipelineNode::new("write", PipelineNodeType::Agent("report_writer".to_string())));
        pipeline.add_node(PipelineNode::new("end", PipelineNodeType::End).terminal());

        pipeline.add_edge(PipelineEdge::new("start", "search"));
        pipeline.add_edge(PipelineEdge::new("search", "extract"));
        pipeline.add_edge(PipelineEdge::new("extract", "analyze"));
        pipeline.add_edge(PipelineEdge::new("analyze", "write"));
        pipeline.add_edge(PipelineEdge::new("write", "end"));

        pipeline
    }

    /// Create a parallel research pipeline with merging
    pub fn parallel_research() -> Pipeline {
        let mut pipeline = Pipeline::new("research_parallel");

        // Entry
        pipeline.add_node(PipelineNode::new("start", PipelineNodeType::Start));
        pipeline.add_node(PipelineNode::new("plan", PipelineNodeType::Agent("researcher".to_string())));

        // Parallel branches
        pipeline.add_node(PipelineNode::new("search", PipelineNodeType::Agent("researcher".to_string())));
        pipeline.add_node(PipelineNode::new("cite_graph", PipelineNodeType::Agent("citation_graph".to_string())));
        pipeline.add_node(PipelineNode::new("index", PipelineNodeType::Agent("vector_indexer".to_string())));

        // Merge
        pipeline.add_node(PipelineNode::new("merge", PipelineNodeType::Merge));
        pipeline.add_node(PipelineNode::new("analyze", PipelineNodeType::Agent("gap_analyzer".to_string())));
        pipeline.add_node(PipelineNode::new("write", PipelineNodeType::Agent("report_writer".to_string())));
        pipeline.add_node(PipelineNode::new("validate", PipelineNodeType::Agent("qa_agent".to_string())));
        pipeline.add_node(PipelineNode::new("end", PipelineNodeType::End).terminal());

        // Edges
        pipeline.add_edge(PipelineEdge::new("start", "plan"));
        pipeline.add_edge(PipelineEdge::new("plan", "search"));
        pipeline.add_edge(PipelineEdge::new("plan", "cite_graph"));
        pipeline.add_edge(PipelineEdge::new("plan", "index"));
        pipeline.add_edge(PipelineEdge::new("search", "merge"));
        pipeline.add_edge(PipelineEdge::new("cite_graph", "merge"));
        pipeline.add_edge(PipelineEdge::new("index", "merge"));
        pipeline.add_edge(PipelineEdge::new("merge", "analyze"));
        pipeline.add_edge(PipelineEdge::new("analyze", "write"));
        pipeline.add_edge(PipelineEdge::new("write", "validate"));
        pipeline.add_edge(PipelineEdge::new("validate", "end"));

        pipeline
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_validate_empty_pipeline() {
        let pipeline = Pipeline::new("empty");
        assert!(pipeline.validate().is_err());
    }

    #[test]
    fn test_pipeline_validate_valid_pipeline() {
        let mut pipeline = Pipeline::new("valid");
        pipeline.add_node(PipelineNode::new("start", PipelineNodeType::Start));
        pipeline.add_node(PipelineNode::new("end", PipelineNodeType::End).terminal());
        pipeline.add_edge(PipelineEdge::new("start", "end"));

        assert!(pipeline.validate().is_ok());
    }

    #[test]
    fn test_pipeline_validate_missing_entry() {
        let mut pipeline = Pipeline::new("no_entry");
        pipeline.add_node(PipelineNode::new("node", PipelineNodeType::Agent("test".to_string())));
        pipeline.add_edge(PipelineEdge::new("nonexistent", "node"));

        assert!(pipeline.validate().is_err());
    }

    #[test]
    fn test_pipeline_validate_edge_to_unknown_node() {
        let mut pipeline = Pipeline::new("unknown_target");
        pipeline.add_node(PipelineNode::new("start", PipelineNodeType::Start));
        pipeline.add_edge(PipelineEdge::new("start", "unknown"));

        assert!(pipeline.validate().is_err());
    }

    #[test]
    fn test_cycle_detection() {
        let mut pipeline = Pipeline::new("cycle_test");
        pipeline.add_node(PipelineNode::new("a", PipelineNodeType::Start));
        pipeline.add_node(PipelineNode::new("b", PipelineNodeType::Agent("test".to_string())));
        pipeline.add_node(PipelineNode::new("c", PipelineNodeType::End).terminal());

        pipeline.add_edge(PipelineEdge::new("a", "b"));
        pipeline.add_edge(PipelineEdge::new("b", "a")); // Creates cycle

        assert!(pipeline.validate().is_err());
    }

    #[test]
    fn test_cycle_detection_complex() {
        let mut pipeline = Pipeline::new("complex_cycle");
        pipeline.add_node(PipelineNode::new("a", PipelineNodeType::Start));
        pipeline.add_node(PipelineNode::new("b", PipelineNodeType::Agent("test".to_string())));
        pipeline.add_node(PipelineNode::new("c", PipelineNodeType::Agent("test".to_string())));
        pipeline.add_node(PipelineNode::new("d", PipelineNodeType::End).terminal());

        pipeline.add_edge(PipelineEdge::new("a", "b"));
        pipeline.add_edge(PipelineEdge::new("b", "c"));
        pipeline.add_edge(PipelineEdge::new("c", "b")); // Cycle back to b
        pipeline.add_edge(PipelineEdge::new("c", "d"));

        assert!(pipeline.validate().is_err());
    }

    #[test]
    fn test_execution_order_simple() {
        let mut pipeline = Pipeline::new("simple");
        pipeline.add_node(PipelineNode::new("start", PipelineNodeType::Start));
        pipeline.add_node(PipelineNode::new("middle", PipelineNodeType::Agent("test".to_string())));
        pipeline.add_node(PipelineNode::new("end", PipelineNodeType::End).terminal());

        pipeline.add_edge(PipelineEdge::new("start", "middle"));
        pipeline.add_edge(PipelineEdge::new("middle", "end"));

        let order = pipeline.execution_order().unwrap();
        assert_eq!(order.len(), 3);
        // start should come before middle, middle before end
        let start_idx = order.iter().position(|x| x == "start").unwrap();
        let middle_idx = order.iter().position(|x| x == "middle").unwrap();
        let end_idx = order.iter().position(|x| x == "end").unwrap();
        assert!(start_idx < middle_idx);
        assert!(middle_idx < end_idx);
    }

    #[test]
    fn test_execution_order_sequential_pipeline() {
        let pipeline = templates::sequential_research();
        assert!(pipeline.validate().is_ok());

        let order = pipeline.execution_order().unwrap();
        assert_eq!(order.len(), 6);

        // Verify order: start -> search -> extract -> analyze -> write -> end
        let expected = vec!["start", "search", "extract", "analyze", "write", "end"];
        assert_eq!(order, expected);
    }

    #[test]
    fn test_sequential_research_template() {
        let pipeline = templates::sequential_research();

        assert_eq!(pipeline.id, "research_sequential");
        assert!(pipeline.validate().is_ok());

        // Verify nodes exist
        assert!(pipeline.get_node("start").is_some());
        assert!(pipeline.get_node("search").is_some());
        assert!(pipeline.get_node("extract").is_some());
        assert!(pipeline.get_node("analyze").is_some());
        assert!(pipeline.get_node("write").is_some());
        assert!(pipeline.get_node("end").is_some());

        // Verify 6 nodes total
        assert_eq!(pipeline.nodes.len(), 6);
    }

    #[test]
    fn test_parallel_research_template() {
        let pipeline = templates::parallel_research();

        assert_eq!(pipeline.id, "research_parallel");
        assert!(pipeline.validate().is_ok());

        // Verify all nodes exist
        assert!(pipeline.get_node("start").is_some());
        assert!(pipeline.get_node("plan").is_some());
        assert!(pipeline.get_node("search").is_some());
        assert!(pipeline.get_node("cite_graph").is_some());
        assert!(pipeline.get_node("index").is_some());
        assert!(pipeline.get_node("merge").is_some());
        assert!(pipeline.get_node("analyze").is_some());
        assert!(pipeline.get_node("write").is_some());
        assert!(pipeline.get_node("validate").is_some());
        assert!(pipeline.get_node("end").is_some());

        // Verify 10 nodes total
        assert_eq!(pipeline.nodes.len(), 10);
    }

    #[test]
    fn test_parallel_research_execution_order() {
        let pipeline = templates::parallel_research();
        let order = pipeline.execution_order().unwrap();

        // start and plan must be early
        let start_idx = order.iter().position(|x| x == "start").unwrap();
        let plan_idx = order.iter().position(|x| x == "plan").unwrap();
        assert!(start_idx < plan_idx);

        // Parallel branches (search, cite_graph, index) should all come after plan
        let plan_idx = order.iter().position(|x| x == "plan").unwrap();
        let search_idx = order.iter().position(|x| x == "search").unwrap();
        let cite_idx = order.iter().position(|x| x == "cite_graph").unwrap();
        let index_idx = order.iter().position(|x| x == "index").unwrap();
        assert!(plan_idx < search_idx);
        assert!(plan_idx < cite_idx);
        assert!(plan_idx < index_idx);

        // merge should come after all parallel branches
        let merge_idx = order.iter().position(|x| x == "merge").unwrap();
        assert!(search_idx < merge_idx);
        assert!(cite_idx < merge_idx);
        assert!(index_idx < merge_idx);

        // end should be last
        let end_idx = order.iter().position(|x| x == "end").unwrap();
        assert!(end_idx == order.len() - 1);
    }

    #[test]
    fn test_pipeline_node_builder() {
        let node = PipelineNode::new("test", PipelineNodeType::Start)
            .with_next("next1")
            .with_next("next2")
            .terminal();

        assert_eq!(node.id, "test");
        assert!(matches!(node.node_type, PipelineNodeType::Start));
        assert_eq!(node.next, vec!["next1", "next2"]);
        assert!(node.is_terminal);
    }

    #[test]
    fn test_pipeline_edge_builder() {
        let edge = PipelineEdge::new("from", "to")
            .with_condition("condition");

        assert_eq!(edge.from, "from");
        assert_eq!(edge.to, "to");
        assert_eq!(edge.condition, Some("condition".to_string()));
    }

    #[test]
    fn test_pipeline_get_node() {
        let mut pipeline = Pipeline::new("test");
        pipeline.add_node(PipelineNode::new("node1", PipelineNodeType::Start));

        assert!(pipeline.get_node("node1").is_some());
        assert!(pipeline.get_node("nonexistent").is_none());
    }

    #[test]
    fn test_pipeline_get_edges_from() {
        let mut pipeline = Pipeline::new("test");
        pipeline.add_node(PipelineNode::new("a", PipelineNodeType::Start));
        pipeline.add_node(PipelineNode::new("b", PipelineNodeType::End).terminal());
        pipeline.add_edge(PipelineEdge::new("a", "b"));

        let edges = pipeline.get_edges_from("a");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].to, "b");

        let edges = pipeline.get_edges_from("b");
        assert!(edges.is_empty());
    }
}
