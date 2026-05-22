//! Graph-of-Agents: Graph-based Multi-Agent Communication Topology.
//!
//! Based on research from:
//! - Graph-of-Agents (GoA) arXiv:2604.17148 - Graph-based multi-agent collaboration
//! - CARD arXiv:2603.01089 - Conditional Agentic Graph Designer
//! - BEACOF arXiv:2603.24973 - Belief-driven adaptive collaboration
//!
//! ## Architecture
//!
//! ```text
//! Query
//!   │
//!   ▼
//! ┌─────────────────────────────────────────────┐
//! │           Communication Graph                 │
//! │  ┌─────┐      ┌─────┐      ┌─────┐        │
//! │  │ A1  │ ───▶ │ A2  │ ───▶ │ A3  │        │
//! │  └──┬──┘      └──┬──┘      └─────┘        │
//! │     │            │                           │
//! │     ▼            ▼                           │
//! │  ┌─────┐      ┌─────┐                       │
//! │  │ A4  │ ◀─── │ A5  │                       │
//! │  └─────┘      └─────┘                       │
//! └─────────────────────────────────────────────┘
//!   │                    │
//!   ▼                    ▼
//! ┌──────────┐    ┌──────────────┐
//! │  Graph   │    │   Message    │
//! │  Pooling │    │   Passing    │
//! └──────────┘    └──────────────┘
//!   │
//!   ▼
//! Final Answer
//! ```

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};

/// An agent node in the communication graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentNode {
    /// Agent ID
    pub id: String,
    /// Agent role/type
    pub role: AgentRole,
    /// Current state
    pub state: AgentState,
    /// Output embedding (for similarity calculation)
    pub embedding: Vec<f32>,
    /// Messages sent
    pub messages_sent: u32,
    /// Messages received
    pub messages_received: u32,
}

/// Agent state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    /// Current belief/position
    pub belief: String,
    /// Confidence score
    pub confidence: f32,
    /// Last updated
    pub updated_at: DateTime<Utc>,
}

/// Agent role in graph
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AgentRole {
    /// Proposer - suggests solutions
    Proposer,
    /// Critic - evaluates and critiques
    Critic,
    /// Synthesizer - aggregates information
    Synthesizer,
    /// Worker - executes tasks
    Worker,
    /// Router - directs message flow
    Router,
}

impl AgentRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentRole::Proposer => "proposer",
            AgentRole::Critic => "critic",
            AgentRole::Synthesizer => "synthesizer",
            AgentRole::Worker => "worker",
            AgentRole::Router => "router",
        }
    }
}

/// An edge in the communication graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Source agent ID
    pub from: String,
    /// Target agent ID
    pub to: String,
    /// Edge weight (communication strength)
    pub weight: f32,
    /// Message types this edge carries
    pub message_types: Vec<String>,
    /// Whether this edge is active
    pub active: bool,
}

/// Communication graph for multi-agent collaboration
#[derive(Debug, Clone)]
pub struct CommunicationGraph {
    /// Agent nodes
    nodes: HashMap<String, AgentNode>,
    /// Edges
    edges: Vec<GraphEdge>,
    /// Adjacency list for fast lookup
    adjacency: HashMap<String, Vec<String>>,
    /// Configuration
    config: GraphConfig,
}

/// Graph configuration
#[derive(Debug, Clone)]
pub struct GraphConfig {
    /// Enable directed edges (vs undirected)
    pub directed: bool,
    /// Node sampling rate for each agent
    pub sampling_rate: f32,
    /// Maximum edges per node
    pub max_edges_per_node: usize,
    /// Message pooling strategy
    pub pooling_strategy: PoolingStrategy,
    /// Enable graph pooling
    pub enable_pooling: bool,
    /// Convergence threshold
    pub convergence_threshold: f32,
}

impl Default for GraphConfig {
    fn default() -> Self {
        Self {
            directed: true,
            sampling_rate: 0.5,
            max_edges_per_node: 3,
            pooling_strategy: PoolingStrategy::Attention,
            enable_pooling: true,
            convergence_threshold: 0.95,
        }
    }
}

/// Message pooling strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolingStrategy {
    /// Mean pooling
    Mean,
    /// Max pooling
    Max,
    /// Attention-weighted pooling
    Attention,
    /// Graph pooling (SortPool)
    GraphPool,
}

/// A message passed between agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    /// Message ID
    pub id: String,
    /// Source agent
    pub from: String,
    /// Target agent
    pub to: String,
    /// Message content
    pub content: String,
    /// Message type
    pub message_type: MessageType,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// TTL (hops remaining)
    pub ttl: u8,
}

/// Message type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageType {
    /// Proposal message
    Proposal,
    /// Critique message
    Critique,
    /// Summary message
    Summary,
    /// Question/Query
    Query,
    /// Answer/Response
    Response,
    /// Control message
    Control,
}

impl CommunicationGraph {
    /// Create a new empty graph
    pub fn new(config: GraphConfig) -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            adjacency: HashMap::new(),
            config,
        }
    }

    /// Create with default config
    pub fn default_graph() -> Self {
        Self::new(GraphConfig::default())
    }

    /// Add an agent node
    pub fn add_agent(&mut self, id: String, role: AgentRole) -> &mut AgentNode {
        let node = AgentNode {
            id: id.clone(),
            role,
            state: AgentState {
                belief: String::new(),
                confidence: 0.5,
                updated_at: Utc::now(),
            },
            embedding: Vec::new(),
            messages_sent: 0,
            messages_received: 0,
        };
        self.nodes.insert(id.clone(), node);
        self.adjacency.entry(id).or_default();
        self.nodes.get_mut(&id).unwrap()
    }

    /// Add an edge between agents
    pub fn add_edge(&mut self, from: String, to: String, weight: f32) -> Option<&mut GraphEdge> {
        if !self.nodes.contains_key(&from) || !self.nodes.contains_key(&to) {
            return None;
        }

        let edge = GraphEdge {
            from: from.clone(),
            to: to.clone(),
            weight,
            message_types: vec![],
            active: true,
        };

        self.edges.push(edge);
        let edge_ref = self.edges.last_mut().unwrap();

        // Update adjacency
        self.adjacency.entry(from).or_default().push(to);

        if !self.config.directed {
            self.adjacency.entry(to).or_default().push(from);
        }

        Some(edge_ref)
    }

    /// Get neighbors of an agent
    pub fn neighbors(&self, agent_id: &str) -> Vec<&AgentNode> {
        self.adjacency
            .get(agent_id)
            .map(|neighbors| {
                neighbors
                    .iter()
                    .filter_map(|n| self.nodes.get(n))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get agents by role
    pub fn agents_by_role(&self, role: AgentRole) -> Vec<&AgentNode> {
        self.nodes
            .values()
            .filter(|n| n.role == role)
            .collect()
    }

    /// Sample relevant agents for a given agent (node sampling from GoA)
    pub fn sample_agents(&self, agent_id: &str, query_embedding: &[f32]) -> Vec<String> {
        let agent = match self.nodes.get(agent_id) {
            Some(a) => a,
            None => return Vec::new(),
        };

        // Calculate similarity with all other agents
        let mut similarities: Vec<(String, f32)> = self.nodes
            .keys()
            .filter(|id| *id != agent_id)
            .map(|id| {
                let other = &self.nodes[id];
                let sim = cosine_similarity(query_embedding, &other.embedding);
                (id.clone(), sim)
            })
            .collect();

        // Sort by similarity
        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Sample top k based on sampling rate
        let k = ((self.nodes.len() as f32) * self.config.sampling_rate).ceil() as usize;
        similarities.into_iter().take(k).map(|(id, _)| id).collect()
    }

    /// Pass messages between agents (directed message passing)
    pub fn message_pass(&mut self, messages: Vec<AgentMessage>) -> Vec<AgentMessage> {
        let mut responses = Vec::new();

        for msg in messages {
            // Deliver message
            if let Some(target) = self.nodes.get_mut(&msg.to) {
                target.messages_received += 1;
                target.state.belief = msg.content.clone();
                target.state.updated_at = Utc::now();
            }

            // Generate response if needed
            if msg.ttl > 0 && msg.message_type != MessageType::Control {
                // Decrement TTL
                let mut response = msg.clone();
                response.id = uuid_simple();
                response.from = msg.to.clone();
                response.to = msg.from.clone();
                response.ttl -= 1;
                response.timestamp = Utc::now();

                // Update source agent
                if let Some(source) = self.nodes.get_mut(&msg.from) {
                    source.messages_sent += 1;
                }

                responses.push(response);
            }
        }

        responses
    }

    /// Pool messages from multiple agents (graph pooling)
    pub fn pool_messages(&self, messages: &[AgentMessage], target_agent: &str) -> String {
        let agent_messages: Vec<_> = messages
            .iter()
            .filter(|m| m.to == target_agent)
            .collect();

        if agent_messages.is_empty() {
            return String::new();
        }

        match self.config.pooling_strategy {
            PoolingStrategy::Mean => {
                // Simple concatenation with mean weighting
                agent_messages.iter().map(|m| m.content.as_str()).collect::<Vec<_>>().join("\n---\n")
            }
            PoolingStrategy::Max => {
                // Return longest message
                agent_messages
                    .iter()
                    .max_by(|a, b| a.content.len().cmp(&b.content.len()))
                    .map(|m| m.content.clone())
                    .unwrap_or_default()
            }
            PoolingStrategy::Attention => {
                // Attention-weighted pooling (simplified)
                let mut weighted_content = String::new();
                for msg in &agent_messages {
                    let weight = self.calculate_attention(msg, target_agent);
                    weighted_content.push_str(&format!("[w{:.2}] {}", weight, msg.content));
                }
                weighted_content
            }
            PoolingStrategy::GraphPool => {
                // SortPool-style: sort by importance then pool
                let mut sorted: Vec<_> = agent_messages.to_vec();
                sorted.sort_by(|a, b| {
                    let score_a = a.content.len() as f32 + (a.ttl as f32 * 10.0);
                    let score_b = b.content.len() as f32 + (b.ttl as f32 * 10.0);
                    score_b.partial_cmp(&score_a).unwrap()
                });
                sorted.iter().map(|m| m.content.as_str()).collect::<Vec<_>>().join("\n")
            }
        }
    }

    /// Calculate attention weight for a message
    fn calculate_attention(&self, message: &AgentMessage, target: &str) -> f32 {
        // Simplified attention: based on recency and message type
        let age_seconds = (Utc::now() - message.timestamp).num_seconds() as f32;
        let recency = 1.0 / (1.0 + age_seconds / 60.0);

        let type_weight = match message.message_type {
            MessageType::Summary => 1.0,
            MessageType::Critique => 0.8,
            MessageType::Proposal => 0.6,
            _ => 0.5,
        };

        recency * type_weight
    }

    /// Check for convergence
    pub fn check_convergence(&self) -> bool {
        let beliefs: Vec<_> = self.nodes.values().map(|n| &n.state.belief).collect();
        if beliefs.len() < 2 {
            return true;
        }

        // Check if all beliefs are similar (simplified)
        let first = beliefs[0];
        let diversity = beliefs.iter().filter(|b| *b != first).count();

        let diversity_ratio = diversity as f32 / beliefs.len() as f32;
        diversity_ratio < (1.0 - self.config.convergence_threshold)
    }

    /// Get graph statistics
    pub fn stats(&self) -> GraphStats {
        let total_edges = self.edges.len();
        let active_edges = self.edges.iter().filter(|e| e.active).count();
        let avg_messages: f32 = if self.nodes.is_empty() {
            0.0
        } else {
            self.nodes.values().map(|n| n.messages_sent as f32 + n.messages_received as f32).sum::<f32>()
                / self.nodes.len() as f32
        };

        GraphStats {
            num_agents: self.nodes.len(),
            num_edges: total_edges,
            active_edges,
            average_messages_per_agent: avg_messages,
            convergence: self.check_convergence(),
        }
    }

    /// Get all agent IDs
    pub fn agent_ids(&self) -> Vec<String> {
        self.nodes.keys().cloned().collect()
    }

    /// Get agent
    pub fn get_agent(&self, id: &str) -> Option<&AgentNode> {
        self.nodes.get(id)
    }

    /// Get mutable agent
    pub fn get_agent_mut(&mut self, id: &str) -> Option<&mut AgentNode> {
        self.nodes.get_mut(id)
    }
}

/// Graph statistics
#[derive(Debug, Clone)]
pub struct GraphStats {
    pub num_agents: usize,
    pub num_edges: usize,
    pub active_edges: usize,
    pub average_messages_per_agent: f32,
    pub convergence: bool,
}

// =============================================================================
// CARD-Style Conditional Graph Designer
// =============================================================================

/// Condition for edge creation
#[derive(Debug, Clone)]
pub struct EdgeCondition {
    /// Condition type
    pub condition_type: ConditionType,
    /// Threshold value
    pub threshold: f32,
}

/// Condition type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConditionType {
    /// Similarity-based
    Similarity,
    /// Role-based
    Role,
    /// Belief agreement
    BeliefAgreement,
    /// Task dependency
    TaskDependency,
}

/// Conditional graph designer (CARD-style)
pub struct ConditionalGraphDesigner {
    /// Base graph
    graph: CommunicationGraph,
    /// Edge conditions
    conditions: Vec<EdgeCondition>,
}

impl ConditionalGraphDesigner {
    /// Create a new designer
    pub fn new(graph: CommunicationGraph) -> Self {
        Self {
            graph,
            conditions: Vec::new(),
        }
    }

    /// Add an edge condition
    pub fn add_condition(mut self, condition: EdgeCondition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// Design edges based on conditions
    pub fn design_edges(&mut self, agents: &[(&str, &str, f32)]) -> usize {
        // agents: (from, to, similarity_score)
        let mut added = 0;

        for (from, to, score) in agents {
            if self.should_connect(*from, *to, *score) {
                if self.graph.add_edge((*from).to_string(), (*to).to_string(), *score).is_some() {
                    added += 1;
                }
            }
        }

        added
    }

    /// Check if two agents should be connected
    fn should_connect(&self, from: &str, to: &str, score: f32) -> bool {
        for condition in &self.conditions {
            match condition.condition_type {
                ConditionType::Similarity => {
                    if score < condition.threshold {
                        return false;
                    }
                }
                ConditionType::Role => {
                    // Same role agents shouldn't connect directly
                    if let (Some(f), Some(t)) = (self.graph.get_agent(from), self.graph.get_agent(to)) {
                        if f.role == t.role {
                            return false;
                        }
                    }
                }
                _ => {}
            }
        }
        true
    }

    /// Get the underlying graph
    pub fn graph(&self) -> &CommunicationGraph {
        &self.graph
    }

    /// Get mutable graph
    pub fn graph_mut(&mut self) -> &mut CommunicationGraph {
        &mut self.graph
    }
}

// =============================================================================
// BEACOF-Style Belief-Driven Collaboration
// =============================================================================

/// Belief state for an agent
#[derive(Debug, Clone)]
pub struct Belief {
    /// Agent ID
    pub agent_id: String,
    /// Current belief
    pub belief: String,
    /// Belief confidence
    pub confidence: f32,
    /// Evidence supporting belief
    pub evidence: Vec<String>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Belief-driven collaboration manager
pub struct BeliefCollaborationManager {
    /// Beliefs by agent
    beliefs: HashMap<String, Belief>,
    /// Collaboration history
    history: Vec<CollaborationRecord>,
}

impl BeliefCollaborationManager {
    /// Create a new manager
    pub fn new() -> Self {
        Self {
            beliefs: HashMap::new(),
            history: Vec::new(),
        }
    }

    /// Update belief for an agent
    pub fn update_belief(&mut self, agent_id: &str, belief: &str, confidence: f32, evidence: Vec<String>) {
        self.beliefs.insert(agent_id.to_string(), Belief {
            agent_id: agent_id.to_string(),
            belief: belief.to_string(),
            confidence,
            evidence,
            timestamp: Utc::now(),
        });
    }

    /// Check if two agents have aligned beliefs
    pub fn beliefs_aligned(&self, agent1: &str, agent2: &str) -> bool {
        let b1 = match self.beliefs.get(agent1) {
            Some(b) => b,
            None => return false,
        };
        let b2 = match self.beliefs.get(agent2) {
            Some(b) => b,
            None => return false,
        };

        // Beliefs are aligned if they're similar and both confident
        let similarity = string_similarity(&b1.belief, &b2.belief);
        similarity > 0.7 && b1.confidence > 0.5 && b2.confidence > 0.5
    }

    /// Resolve conflicts between agents
    pub fn resolve_conflict(&mut self, agent1: &str, agent2: &str) -> String {
        let b1 = self.beliefs.get(agent1);
        let b2 = self.beliefs.get(agent2);

        match (b1, b2) {
            (Some(b1), Some(b2)) => {
                // Higher confidence belief wins
                if b1.confidence > b2.confidence {
                    self.record_collaboration(agent1, agent2, &b1.belief, true);
                    b1.belief.clone()
                } else {
                    self.record_collaboration(agent1, agent2, &b2.belief, true);
                    b2.belief.clone()
                }
            }
            (Some(b), None) | (None, Some(b)) => {
                b.belief.clone()
            }
            (None, None) => String::new(),
        }
    }

    /// Record collaboration
    fn record_collaboration(&mut self, agent1: &str, agent2: &str, outcome: &str, resolved: bool) {
        self.history.push(CollaborationRecord {
            agent1: agent1.to_string(),
            agent2: agent2.to_string(),
            outcome: outcome.to_string(),
            resolved,
            timestamp: Utc::now(),
        });
    }

    /// Get belief
    pub fn get_belief(&self, agent_id: &str) -> Option<&Belief> {
        self.beliefs.get(agent_id)
    }

    /// Get all beliefs
    pub fn all_beliefs(&self) -> Vec<&Belief> {
        self.beliefs.values().collect()
    }
}

impl Default for BeliefCollaborationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Record of collaboration
#[derive(Debug, Clone)]
pub struct CollaborationRecord {
    pub agent1: String,
    pub agent2: String,
    pub outcome: String,
    pub resolved: bool,
    pub timestamp: DateTime<Utc>,
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Calculate cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}

/// Simple string similarity (Jaccard index on words)
fn string_similarity(a: &str, b: &str) -> f32 {
    let words_a: HashSet<_> = a.split_whitespace().collect();
    let words_b: HashSet<_> = b.split_whitespace().collect();

    if words_a.is_empty() && words_b.is_empty() {
        return 1.0;
    }

    let intersection = words_a.intersection(&words_b).count() as f32;
    let union = words_a.union(&words_b).count() as f32;

    if union == 0.0 {
        return 0.0;
    }

    intersection / union
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", nanos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_creation() {
        let mut graph = CommunicationGraph::default_graph();

        graph.add_agent("a1".to_string(), AgentRole::Proposer);
        graph.add_agent("a2".to_string(), AgentRole::Critic);
        graph.add_agent("a3".to_string(), AgentRole::Synthesizer);

        assert_eq!(graph.nodes.len(), 3);
    }

    #[test]
    fn test_add_edge() {
        let mut graph = CommunicationGraph::default_graph();

        graph.add_agent("a1".to_string(), AgentRole::Proposer);
        graph.add_agent("a2".to_string(), AgentRole::Critic);

        let result = graph.add_edge("a1".to_string(), "a2".to_string(), 0.8);
        assert!(result.is_some());
    }

    #[test]
    fn test_neighbors() {
        let mut graph = CommunicationGraph::default_graph();

        graph.add_agent("a1".to_string(), AgentRole::Proposer);
        graph.add_agent("a2".to_string(), AgentRole::Critic);
        graph.add_agent("a3".to_string(), AgentRole::Worker);

        graph.add_edge("a1".to_string(), "a2".to_string(), 0.8).unwrap();
        graph.add_edge("a1".to_string(), "a3".to_string(), 0.6).unwrap();

        let neighbors = graph.neighbors("a1");
        assert_eq!(neighbors.len(), 2);
    }

    #[test]
    fn test_message_passing() {
        let mut graph = CommunicationGraph::default_graph();

        graph.add_agent("a1".to_string(), AgentRole::Proposer);
        graph.add_agent("a2".to_string(), AgentRole::Critic);

        graph.add_edge("a1".to_string(), "a2".to_string(), 0.8).unwrap();

        let messages = vec![AgentMessage {
            id: "m1".to_string(),
            from: "a1".to_string(),
            to: "a2".to_string(),
            content: "Proposal: use method X".to_string(),
            message_type: MessageType::Proposal,
            timestamp: Utc::now(),
            ttl: 3,
        }];

        let responses = graph.message_pass(messages);
        assert!(!responses.is_empty());
        assert_eq!(responses[0].from, "a2");
        assert_eq!(responses[0].to, "a1");
    }

    #[test]
    fn test_belief_alignment() {
        let mut manager = BeliefCollaborationManager::new();

        manager.update_belief("a1", "Use method X", 0.8, vec![]);
        manager.update_belief("a2", "Use method X", 0.7, vec![]);

        assert!(manager.beliefs_aligned("a1", "a2"));
    }
}
