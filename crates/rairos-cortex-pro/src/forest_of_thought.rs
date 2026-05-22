//! Forest-of-Thought Reasoning Module.
//!
//! Based on research from:
//! - Forest-of-Thought (FoT) (arXiv:2412.09078) - Multiple reasoning trees with sparse activation
//! - Dynamic Parallel Tree Search (ACL 2025) - Efficient parallel ToT
//! - Diagram of Thought (DoT) (arXiv:2409.10038) - DAG reasoning with role tokens
//!
//! ## Architecture
//!
//! ```text
//! Query
//!   │
//!   ▼
//! ┌─────────────────────────────────────────────────┐
//! │  Spawn Multiple Reasoning Trees (Forest)         │
//! └─────────────────────────────────────────────────┘
//!   │                    │                    │
//!   ▼                    ▼                    ▼
//! ┌─────────┐      ┌─────────────┐      ┌─────────────┐
//! │ Tree 1  │      │   Tree 2    │      │   Tree 3    │
//! │Chain    │      │  branching  │      │  Parallel    │
//! └────┬────┘      └──────┬──────┘      └──────┬──────┘
//!      │                  │                    │
//!      └──────────────────┼────────────────────┘
//!                         ▼
//!              ┌─────────────────────┐
//!              │  Consensus Builder  │ ◄── Vote/merge answers
//!              └─────────────────────┘
//!                         │
//!                         ▼
//!                 ┌───────────────┐
//!                 │ Final Answer  │ ◄── Selected via weighted voting
//!                 └───────────────┘
//! ```

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};

/// A thought node in a reasoning tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtNode {
    /// Node ID
    pub id: String,
    /// Thought content
    pub content: String,
    /// Parent node ID
    pub parent_id: Option<String>,
    /// Children node IDs
    pub children: Vec<String>,
    /// Node type
    pub node_type: NodeType,
    /// Score (quality/vote)
    pub score: f32,
    /// Depth in tree
    pub depth: u32,
    /// Whether this is active (sparse activation)
    pub active: bool,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Node type in reasoning
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum NodeType {
    /// Proposer node - suggests next thought
    Propose,
    /// Critic node - evaluates/thorns
    Critic,
    /// Summary node - synthesizes
    Summary,
    /// Vote node - consensus
    Vote,
}

/// A reasoning tree
#[derive(Debug, Clone)]
pub struct ReasoningTree {
    /// Tree ID
    pub id: String,
    /// Root node
    pub root_id: String,
    /// All nodes
    pub nodes: HashMap<String, ThoughtNode>,
    /// Total score
    pub total_score: f32,
    /// Depth
    pub depth: u32,
    /// Whether this tree is complete
    pub complete: bool,
}

impl ReasoningTree {
    /// Get root node
    pub fn root(&self) -> Option<&ThoughtNode> {
        self.nodes.get(&self.root_id)
    }

    /// Get node by ID
    pub fn get(&self, id: &str) -> Option<&ThoughtNode> {
        self.nodes.get(id)
    }

    /// Get children of a node
    pub fn children_of(&self, node_id: &str) -> Vec<&ThoughtNode> {
        let node = match self.nodes.get(node_id) {
            Some(n) => n,
            None => return Vec::new(),
        };
        node.children.iter().filter_map(|id| self.nodes.get(id)).collect()
    }

    /// Score path from root to node
    pub fn path_score(&self, node_id: &str) -> f32 {
        let mut score = 0.0;
        let mut current_id = Some(node_id.to_string());

        while let Some(id) = current_id {
            if let Some(node) = self.nodes.get(&id) {
                score += node.score;
                current_id = node.parent_id.clone();
            } else {
                break;
            }
        }

        score
    }
}

/// Forest configuration
#[derive(Debug, Clone)]
pub struct ForestConfig {
    /// Number of trees to spawn
    pub num_trees: usize,
    /// Maximum depth per tree
    pub max_depth: usize,
    /// Branching factor
    pub branching_factor: usize,
    /// Activation threshold (sparse)
    pub activation_threshold: f32,
    /// Consensus threshold
    pub consensus_threshold: f32,
    /// Enable parallel expansion
    pub parallel: bool,
}

impl Default for ForestConfig {
    fn default() -> Self {
        Self {
            num_trees: 3,
            max_depth: 5,
            branching_factor: 2,
            activation_threshold: 0.3,
            consensus_threshold: 0.6,
            parallel: true,
        }
    }
}

/// The Forest-of-Thought reasoner
pub struct ForestReasoner {
    /// Configuration
    config: ForestConfig,
    /// Reasoning trees
    trees: Vec<ReasoningTree>,
    /// Active nodes across all trees
    active_nodes: HashSet<String>,
    /// Consensus history
    consensus_history: Vec<ConsensusRecord>,
}

/// A node in the forest with expanded info
#[derive(Debug, Clone)]
pub struct ForestNode {
    /// Tree this node belongs to
    pub tree_id: String,
    /// Node data
    pub node: ThoughtNode,
}

/// Consensus record
#[derive(Debug, Clone)]
pub struct ConsensusRecord {
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Agreed answer
    pub answer: String,
    /// Confidence
    pub confidence: f32,
    /// Supporting trees
    pub supporting_trees: Vec<String>,
}

impl ForestReasoner {
    /// Create a new forest reasoner
    pub fn new(config: ForestConfig) -> Self {
        Self {
            config,
            trees: Vec::new(),
            active_nodes: HashSet::new(),
            consensus_history: Vec::new(),
        }
    }

    /// Reason about a query
    pub fn reason(&mut self, query: &str) -> ReasoningResult {
        // Step 1: Spawn initial trees
        self.spawn_trees(query);

        // Step 2: Grow trees (expand nodes)
        self.grow_trees(query);

        // Step 3: Apply sparse activation
        self.apply_sparse_activation();

        // Step 4: Build consensus
        let consensus = self.build_consensus();

        ReasoningResult {
            query: query.to_string(),
            trees: self.trees.clone(),
            consensus: consensus.clone(),
            active_nodes: self.active_nodes.clone(),
            final_answer: consensus.answer.clone(),
            confidence: consensus.confidence,
        }
    }

    /// Spawn initial reasoning trees
    fn spawn_trees(&mut self, query: &str) {
        self.trees.clear();

        let strategies = vec![
            vec![NodeType::Propose, NodeType::Critic, NodeType::Summary],
            vec![NodeType::Propose, NodeType::Propose, NodeType::Critic],
            vec![NodeType::Propose, NodeType::Summary, NodeType::Vote],
        ];

        for (i, strategy) in strategies.iter().enumerate() {
            let tree_id = format!("tree_{}", i);
            let root_id = uuid_simple();

            let root = ThoughtNode {
                id: root_id.clone(),
                content: query.to_string(),
                parent_id: None,
                children: Vec::new(),
                node_type: NodeType::Propose,
                score: 1.0,
                depth: 0,
                active: true,
                timestamp: Utc::now(),
            };

            let mut nodes = HashMap::new();
            nodes.insert(root_id.clone(), root);

            self.trees.push(ReasoningTree {
                id: tree_id,
                root_id,
                nodes,
                total_score: 1.0,
                depth: 0,
                complete: false,
            });

            self.active_nodes.insert(root_id);
        }
    }

    /// Grow trees by expanding nodes
    fn grow_trees(&mut self, _query: &str) {
        for tree in &mut self.trees {
            if tree.complete {
                continue;
            }

            // Expand active nodes
            let active_ids: Vec<_> = tree.nodes
                .values()
                .filter(|n| n.active && n.depth < self.config.max_depth as u32)
                .map(|n| n.id.clone())
                .collect();

            for parent_id in active_ids {
                let children = self.expand_node(tree, &parent_id);

                if let Some(parent) = tree.nodes.get_mut(&parent_id) {
                    parent.children = children.iter().map(|c| c.id.clone()).collect();
                }
            }
        }
    }

    /// Expand a single node
    fn expand_node(&self, tree: &mut ReasoningTree, parent_id: &str) -> Vec<ThoughtNode> {
        let parent = tree.nodes.get(parent_id).cloned();
        if parent.is_none() {
            return Vec::new();
        }
        let parent = parent.unwrap();

        let mut children = Vec::new();

        // Generate child thoughts based on node type
        for i in 0..self.config.branching_factor {
            let child_id = uuid_simple();
            let child_type = match parent.node_type {
                NodeType::Propose => {
                    if i == 0 { NodeType::Critic } else { NodeType::Propose }
                }
                NodeType::Critic => NodeType::Propose,
                NodeType::Summary => NodeType::Vote,
                NodeType::Vote => NodeType::Summary,
            };

            let child_content = generate_child_thought(&parent.content, child_type, i);

            children.push(ThoughtNode {
                id: child_id,
                content: child_content,
                parent_id: Some(parent_id.to_string()),
                children: Vec::new(),
                node_type: child_type,
                score: calculate_thought_score(&child_content, child_type),
                depth: parent.depth + 1,
                active: true,
                timestamp: Utc::now(),
            });
        }

        // Add children to tree
        for child in &children {
            tree.nodes.insert(child.id.clone(), child.clone());
        }

        // Update tree depth
        tree.depth = tree.depth.max(parent.depth + 1);

        // Mark complete if max depth
        if parent.depth + 1 >= self.config.max_depth as u32 {
            tree.complete = true;
        }

        children
    }

    /// Apply sparse activation (only keep high-scoring nodes)
    fn apply_sparse_activation(&mut self) {
        for tree in &mut self.trees {
            for node in tree.nodes.values_mut() {
                node.active = node.score >= self.config.activation_threshold;
            }
        }

        // Update active nodes set
        self.active_nodes.clear();
        for tree in &self.trees {
            for (id, node) in &tree.nodes {
                if node.active {
                    self.active_nodes.insert(format!("{}:{}", tree.id, id));
                }
            }
        }
    }

    /// Build consensus from all trees
    fn build_consensus(&mut self) -> Consensus {
        let mut answer_scores: HashMap<String, Vec<(String, f32)>> = HashMap::new();

        // Collect leaf node answers from each tree
        for tree in &self.trees {
            let leaves = tree.nodes.values()
                .filter(|n| n.children.is_empty())
                .collect::<Vec<_>>();

            for leaf in leaves {
                let answer = extract_answer(&leaf.content);
                answer_scores
                    .entry(answer.clone())
                    .or_default()
                    .push((tree.id.clone(), leaf.score));
            }
        }

        // Calculate weighted scores
        let mut weighted_scores: Vec<(String, f32)> = Vec::new();
        for (answer, scores) in answer_scores {
            let total_score: f32 = scores.iter().map(|(_, s)| s).sum();
            let tree_count = scores.len() as f32;
            let weighted = total_score / tree_count.max(1.0);
            weighted_scores.push((answer, weighted));
        }

        // Sort by score
        weighted_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Get top answer
        let (answer, score) = weighted_scores.first()
            .cloned()
            .unwrap_or_else(|| ("No consensus reached".to_string(), 0.0));

        let confidence = (score * self.trees.len() as f32 / self.config.num_trees as f32).min(1.0);

        let consensus = Consensus {
            answer: answer.clone(),
            confidence,
            supporting_trees: weighted_scores.iter().map(|(a, _)| a.clone()).take(3).collect(),
            all_scores: weighted_scores,
        };

        self.consensus_history.push(ConsensusRecord {
            timestamp: Utc::now(),
            answer: answer.clone(),
            confidence,
            supporting_trees: consensus.supporting_trees.clone(),
        });

        consensus
    }

    /// Get statistics
    pub fn stats(&self) -> ForestStats {
        let total_nodes: usize = self.trees.iter().map(|t| t.nodes.len()).sum();
        let avg_depth: f32 = if self.trees.is_empty() {
            0.0
        } else {
            self.trees.iter().map(|t| t.depth as f32).sum::<f32>() / self.trees.len() as f32
        };

        ForestStats {
            num_trees: self.trees.len(),
            total_nodes,
            active_nodes: self.active_nodes.len(),
            average_depth: avg_depth,
            consensus_history_size: self.consensus_history.len(),
        }
    }
}

/// Consensus result
#[derive(Debug, Clone)]
pub struct Consensus {
    /// Agreed answer
    pub answer: String,
    /// Confidence score
    pub confidence: f32,
    /// Supporting trees
    pub supporting_trees: Vec<String>,
    /// All answer scores
    pub all_scores: Vec<(String, f32)>,
}

/// Reasoning result
#[derive(Debug, Clone)]
pub struct ReasoningResult {
    /// Original query
    pub query: String,
    /// Reasoning trees
    pub trees: Vec<ReasoningTree>,
    /// Consensus
    pub consensus: Consensus,
    /// Active nodes
    pub active_nodes: HashSet<String>,
    /// Final answer
    pub final_answer: String,
    /// Confidence
    pub confidence: f32,
}

/// Forest statistics
#[derive(Debug, Clone)]
pub struct ForestStats {
    pub num_trees: usize,
    pub total_nodes: usize,
    pub active_nodes: usize,
    pub average_depth: f32,
    pub consensus_history_size: usize,
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Generate child thought content (simplified - would use LLM in practice)
fn generate_child_thought(parent_content: &str, node_type: NodeType, branch: usize) -> String {
    match node_type {
        NodeType::Propose => format!("{} - alternative approach {}", parent_content, branch + 1),
        NodeType::Critic => format!("{} - potential issue to consider", parent_content),
        NodeType::Summary => format!("{} - synthesized insight", parent_content),
        NodeType::Vote => format!("{} - final assessment", parent_content),
    }
}

/// Calculate thought score based on type and content
fn calculate_thought_score(content: &str, node_type: NodeType) -> f32 {
    let mut score: f32 = 0.5;

    // Bonus for certain keywords
    let content_lower = content.to_lowercase();
    if content_lower.contains("however") || content_lower.contains("but") {
        score += 0.1;
    }
    if content_lower.contains("therefore") || content_lower.contains("conclude") {
        score += 0.15;
    }
    if content_lower.contains("because") || content_lower.contains("since") {
        score += 0.1;
    }

    // Type-based adjustment
    match node_type {
        NodeType::Critic => score += 0.05, // Critics get slight bonus for evaluation
        NodeType::Summary => score += 0.1, // Summaries are valuable
        NodeType::Vote => score += 0.15, // Votes indicate confidence
        _ => {}
    }

    score.min(1.0).max(0.0)
}

/// Extract answer from thought content
fn extract_answer(content: &str) -> String {
    // Simple extraction - would use more sophisticated method in practice
    let sentences: Vec<&str> = content.split(&['.', '!', '?'][..])
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    sentences.last().map(|s| s.to_string()).unwrap_or_else(|| content.to_string())
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", nanos)
}

// =============================================================================
// Diagram of Thought (DoT) Integration
// =============================================================================

/// A DoT-style DAG with role tokens
pub struct DiagramOfThoughtReasoner {
    /// DAG nodes
    nodes: HashMap<String, ThoughtNode>,
    /// Role tokens
    role_tokens: HashMap<String, RoleToken>,
}

/// Role token for DoT
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RoleToken {
    /// Proposer role
    Proposer,
    /// Critic role
    Critic,
    /// Summarizer role
    Summarizer,
}

impl DiagramOfThoughtReasoner {
    /// Create new DoT reasoner
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            role_tokens: HashMap::new(),
        }
    }

    /// Add a node with role
    pub fn add(&mut self, content: &str, role: RoleToken, parent_id: Option<&str>) -> String {
        let id = uuid_simple();

        let node = ThoughtNode {
            id: id.clone(),
            content: content.to_string(),
            parent_id: parent_id.map(String::from),
            children: Vec::new(),
            node_type: match role {
                RoleToken::Proposer => NodeType::Propose,
                RoleToken::Critic => NodeType::Critic,
                RoleToken::Summarizer => NodeType::Summary,
            },
            score: 0.5,
            depth: 0,
            active: true,
            timestamp: Utc::now(),
        };

        self.role_tokens.insert(id.clone(), role);
        self.nodes.insert(id.clone(), node);

        // Update parent children
        if let Some(pid) = parent_id {
            if let Some(parent) = self.nodes.get_mut(pid) {
                parent.children.push(id.clone());
            }
        }

        id
    }

    /// Get DAG structure
    pub fn dag(&self) -> &HashMap<String, ThoughtNode> {
        &self.nodes
    }
}

impl Default for DiagramOfThoughtReasoner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forest_reasoner_basic() {
        let config = ForestConfig::default();
        let mut reasoner = ForestReasoner::new(config);

        let result = reasoner.reason("What is the best approach for machine learning?");

        assert!(!result.final_answer.is_empty());
        assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
    }

    #[test]
    fn test_forest_stats() {
        let config = ForestConfig {
            num_trees: 3,
            max_depth: 4,
            ..Default::default()
        };
        let mut reasoner = ForestReasoner::new(config);

        reasoner.reason("Test query");

        let stats = reasoner.stats();
        assert!(stats.num_trees == 3);
        assert!(stats.total_nodes > 0);
    }

    #[test]
    fn test_diagram_of_thought() {
        let mut dot = DiagramOfThoughtReasoner::new();

        let n1 = dot.add("Initial problem", RoleToken::Proposer, None);
        let n2 = dot.add("Critique of initial", RoleToken::Critic, Some(&n1));
        let n3 = dot.add("Synthesis", RoleToken::Summarizer, Some(&n2));

        assert_eq!(dot.dag().len(), 3);
        assert!(dot.dag().get(&n3).is_some());
    }
}
