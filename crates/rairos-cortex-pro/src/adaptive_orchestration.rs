//! Adaptive Orchestration Module for dynamic multi-agent topology.
//!
//! Based on research from:
//! - AdaptOrch arXiv:2602.16873 - Task-adaptive multi-agent orchestration
//! - Symphony-Coord arXiv:2602.00966 - Decentralized coordination via bandit
//! - BIGMAS arXiv:2603.15371 - Brain-inspired graph multi-agent systems
//!
//! ## Architecture
//!
//! ```text
//! Task → Dependency Analysis → Topology Selection
//!                                  │
//!         ┌─────────────────────────┼─────────────────────────┐
//!         ▼                         ▼                         ▼
//!    ┌─────────┐              ┌──────────┐              ┌──────────┐
//!    │Parallel │              │Sequential│              │Hierarchical│
//!    └─────────┘              └──────────┘              └──────────┘
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

use crate::utils::uuid_simple;

/// Orchestration topology types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Topology {
    /// Parallel execution
    Parallel,
    /// Sequential execution
    Sequential,
    /// Hierarchical/manager-worker
    Hierarchical,
    /// Hybrid/mixed
    Hybrid,
    /// Dynamic based on task
    Dynamic,
}

impl Topology {
    pub fn as_str(&self) -> &'static str {
        match self {
            Topology::Parallel => "parallel",
            Topology::Sequential => "sequential",
            Topology::Hierarchical => "hierarchical",
            Topology::Hybrid => "hybrid",
            Topology::Dynamic => "dynamic",
        }
    }
}

/// Task dependency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDependency {
    /// Task ID
    pub task_id: String,
    /// IDs of tasks this depends on
    pub depends_on: Vec<String>,
    /// Estimated complexity (0.0 - 1.0)
    pub complexity: f32,
    /// Parallelism width
    pub parallelism_width: usize,
    /// Critical path depth
    pub critical_path_depth: usize,
}

/// DAG of task dependencies
#[derive(Debug, Clone)]
pub struct TaskDag {
    /// Nodes (tasks)
    nodes: Vec<DagNode>,
    /// Edges (dependencies)
    edges: Vec<DagEdge>,
}

/// A node in the DAG
#[derive(Debug, Clone)]
pub struct DagNode {
    pub id: String,
    pub complexity: f32,
    pub parallelizable: bool,
}

/// An edge in the DAG
#[derive(Debug, Clone)]
pub struct DagEdge {
    pub from: String,
    pub to: String,
}

impl TaskDag {
    /// Create from task dependencies
    pub fn from_dependencies(dependencies: &[TaskDependency]) -> Self {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut node_ids = HashSet::new();

        for dep in dependencies {
            if !node_ids.contains(&dep.task_id) {
                node_ids.insert(dep.task_id.clone());
                nodes.push(DagNode {
                    id: dep.task_id.clone(),
                    complexity: dep.complexity,
                    parallelizable: dep.parallelism_width > 1,
                });
            }

            for parent in &dep.depends_on {
                edges.push(DagEdge {
                    from: parent.clone(),
                    to: dep.task_id.clone(),
                });
            }
        }

        Self { nodes, edges }
    }

    /// Calculate parallelism width (max concurrent tasks)
    pub fn parallelism_width(&self) -> usize {
        // Find max number of nodes with no dependencies
        let mut max_width = 0;
        for node in &self.nodes {
            let has_parent = self.edges.iter().any(|e| e.to == node.id);
            if !has_parent {
                max_width += 1;
            }
        }
        max_width.max(1)
    }

    /// Calculate critical path depth
    pub fn critical_path_depth(&self) -> usize {
        // Simplified: count longest path
        let mut depths: HashMap<String, usize> = HashMap::new();

        for node in &self.nodes {
            let depth = self.longest_path_from(&node.id, &mut depths);
            depths.insert(node.id.clone(), depth);
        }

        depths.values().copied().max().unwrap_or(1)
    }

    fn longest_path_from(&self, node_id: &str, cache: &mut HashMap<String, usize>) -> usize {
        if let Some(&cached) = cache.get(node_id) {
            return cached;
        }

        let outgoing: Vec<_> = self.edges.iter().filter(|e| e.from == node_id).collect();

        if outgoing.is_empty() {
            return 1;
        }

        let max_child = outgoing
            .iter()
            .map(|e| self.longest_path_from(&e.to, cache))
            .max()
            .unwrap_or(0);

        let result = 1 + max_child;
        cache.insert(node_id.to_string(), result);
        result
    }
}

/// Configuration for adaptive orchestration
#[derive(Debug, Clone)]
pub struct OrchestrationConfig {
    /// Enable automatic topology selection
    pub auto_topology: bool,
    /// Default topology
    pub default_topology: Topology,
    /// Max parallel agents
    pub max_parallel_agents: usize,
    /// Enable adaptive resynthesis
    pub adaptive_resynthesis: bool,
    /// Consistency threshold for synthesis
    pub consistency_threshold: f32,
}

impl Default for OrchestrationConfig {
    fn default() -> Self {
        Self {
            auto_topology: true,
            default_topology: Topology::Dynamic,
            max_parallel_agents: 4,
            adaptive_resynthesis: true,
            consistency_threshold: 0.8,
        }
    }
}

/// Adaptive orchestration engine
pub struct AdaptiveOrchestrator {
    config: OrchestrationConfig,
    /// History of task topologies
    history: VecDeque<TopologyRecord>,
    /// Current task statistics
    task_stats: HashMap<String, TaskStats>,
}

/// Record of topology selection
#[derive(Debug, Clone)]
pub struct TopologyRecord {
    pub task_type: String,
    pub topology: Topology,
    pub success_rate: f32,
    pub avg_duration_ms: u64,
    pub timestamp: DateTime<Utc>,
}

/// Statistics for a task type
#[derive(Debug, Clone, Default)]
pub struct TaskStats {
    pub total_executions: u32,
    pub successes: u32,
    pub avg_duration_ms: f64,
}

impl AdaptiveOrchestrator {
    /// Create new orchestrator
    pub fn new(config: OrchestrationConfig) -> Self {
        Self {
            config,
            history: VecDeque::new(),
            task_stats: HashMap::new(),
        }
    }

    /// Select optimal topology for task DAG
    pub fn select_topology(&self, dag: &TaskDag) -> TopologySelection {
        if !self.config.auto_topology {
            return TopologySelection {
                topology: self.config.default_topology,
                reasoning: "Auto-topology disabled".to_string(),
                confidence: 1.0,
            };
        }

        let width = dag.parallelism_width();
        let depth = dag.critical_path_depth();

        // Analyze DAG characteristics
        let complexity_score = self.calculate_complexity(dag);
        let coupling = self.calculate_coupling(dag);

        // Select topology based on characteristics
        let (topology, reasoning, confidence) =
            self.select_based_on_chars(width, depth, complexity_score, coupling);

        TopologySelection {
            topology,
            reasoning,
            confidence,
        }
    }

    /// Calculate complexity score
    fn calculate_complexity(&self, dag: &TaskDag) -> f32 {
        let avg_complexity: f32 =
            dag.nodes.iter().map(|n| n.complexity).sum::<f32>() / dag.nodes.len().max(1) as f32;

        // Factor in size
        let size_factor = (dag.nodes.len() as f32 / 10.0).min(1.0);

        (avg_complexity + size_factor) / 2.0
    }

    /// Calculate coupling (dependency density)
    fn calculate_coupling(&self, dag: &TaskDag) -> f32 {
        if dag.nodes.len() <= 1 {
            return 0.0;
        }

        let max_edges = dag.nodes.len() * (dag.nodes.len() - 1) / 2;
        dag.edges.len() as f32 / max_edges as f32
    }

    /// Select topology based on characteristics
    fn select_based_on_chars(
        &self,
        width: usize,
        depth: usize,
        complexity: f32,
        coupling: f32,
    ) -> (Topology, String, f32) {
        // High parallelism + low coupling = Parallel
        if width > 2 && coupling < 0.3 {
            return (
                Topology::Parallel,
                format!(
                    "High parallelism ({} tasks), low coupling ({:.2})",
                    width, coupling
                ),
                0.9,
            );
        }

        // High coupling + deep chain = Sequential
        if coupling > 0.6 && depth > width {
            return (
                Topology::Sequential,
                format!(
                    "High coupling ({:.2}), chain structure (depth={}, width={})",
                    coupling, depth, width
                ),
                0.85,
            );
        }

        // Complex + many agents = Hierarchical
        if complexity > 0.7 && width > 3 {
            return (
                Topology::Hierarchical,
                format!(
                    "High complexity ({:.2}), multi-agent ({})",
                    complexity, width
                ),
                0.8,
            );
        }

        // Adaptive hybrid based on DAG structure
        if width > 1 && depth > 1 {
            return (
                Topology::Hybrid,
                format!("Mixed parallelism (width={}, depth={})", width, depth),
                0.75,
            );
        }

        // Default
        (
            Topology::Dynamic,
            "Default adaptive selection".to_string(),
            0.6,
        )
    }

    /// Record execution result
    pub fn record_execution(
        &mut self,
        task_type: &str,
        topology: Topology,
        success: bool,
        duration_ms: u64,
    ) {
        // Update stats
        let stats = self.task_stats.entry(task_type.to_string()).or_default();
        stats.total_executions += 1;
        if success {
            stats.successes += 1;
        }
        stats.avg_duration_ms = (stats.avg_duration_ms * (stats.total_executions - 1) as f64
            + duration_ms as f64)
            / stats.total_executions as f64;

        // Record history
        let success_rate = stats.successes as f32 / stats.total_executions as f32;
        self.history.push_back(TopologyRecord {
            task_type: task_type.to_string(),
            topology,
            success_rate,
            avg_duration_ms: duration_ms,
            timestamp: Utc::now(),
        });

        // Trim history
        if self.history.len() > 1000 {
            self.history.pop_front();
        }
    }

    /// Get best topology for task type
    pub fn get_best_topology(&self, task_type: &str) -> Option<Topology> {
        let records: Vec<_> = self
            .history
            .iter()
            .filter(|r| r.task_type == task_type)
            .collect();

        if records.is_empty() {
            return None;
        }

        // Find topology with best success rate
        let mut topology_scores: HashMap<Topology, (f32, u32)> = HashMap::new();

        for record in records {
            let entry = topology_scores.entry(record.topology).or_insert((0.0, 0));
            entry.0 += record.success_rate;
            entry.1 += 1;
        }

        topology_scores
            .into_iter()
            .map(|(t, (rate, count))| (t, rate / count as f32))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(t, _)| t)
    }

    /// Calculate inter-subtask coupling
    pub fn calculate_coupling_score(&self, dag: &TaskDag) -> f32 {
        self.calculate_coupling(dag)
    }

    /// Get orchestrator statistics
    pub fn stats(&self) -> OrchestrationStats {
        OrchestrationStats {
            total_recorded: self.history.len(),
            task_types_seen: self.task_stats.len(),
            best_topologies: self
                .task_stats
                .iter()
                .map(|(k, _v)| {
                    let best = self.get_best_topology(k);
                    (
                        k.clone(),
                        best.map(|t| t.as_str().to_string()).unwrap_or_default(),
                    )
                })
                .collect(),
        }
    }
}

/// Result of topology selection
#[derive(Debug, Clone)]
pub struct TopologySelection {
    pub topology: Topology,
    pub reasoning: String,
    pub confidence: f32,
}

/// Orchestration statistics
#[derive(Debug, Clone)]
pub struct OrchestrationStats {
    pub total_recorded: usize,
    pub task_types_seen: usize,
    pub best_topologies: HashMap<String, String>,
}

// =============================================================================
// Symphony-Coord Style Decentralized Coordination
// =============================================================================

/// Agent capability signal
#[derive(Debug, Clone)]
pub struct CapabilitySignal {
    pub agent_id: String,
    pub capability: f32,
    pub reliability: f32,
    pub latency_ms: u32,
    pub load: f32,
}

/// Decentralized coordinator using multi-armed bandit
pub struct SymphonyCoordinator {
    /// Agent capabilities
    agents: HashMap<String, Vec<CapabilitySignal>>,
    /// Selection history for each slot
    selection_history: Vec<SelectionRecord>,
    /// Configuration
    config: SymphonyConfig,
}

/// Selection record
#[derive(Debug, Clone)]
pub struct SelectionRecord {
    pub slot_id: String,
    pub selected_agent: String,
    pub reward: f32,
    pub context_features: Vec<f32>,
}

/// Symphony configuration
#[derive(Debug, Clone)]
pub struct SymphonyConfig {
    /// Exploration constant for UCB
    pub exploration_constant: f32,
    /// Maximum candidates per slot
    pub max_candidates: usize,
    /// Beacon batch size
    pub beacon_batch_size: usize,
}

impl Default for SymphonyConfig {
    fn default() -> Self {
        Self {
            exploration_constant: 1.414, // sqrt(2)
            max_candidates: 5,
            beacon_batch_size: 3,
        }
    }
}

impl SymphonyCoordinator {
    /// Create new coordinator
    pub fn new(config: SymphonyConfig) -> Self {
        Self {
            agents: HashMap::new(),
            selection_history: Vec::new(),
            config,
        }
    }

    /// Register agent capabilities
    pub fn register_agent(&mut self, agent_id: &str, signals: Vec<CapabilitySignal>) {
        self.agents.insert(agent_id.to_string(), signals);
    }

    /// Select agents for slots using LinUCB-style selection
    pub fn select_agents(&self, slot_requirements: &[SlotRequirement]) -> Vec<AgentSelection> {
        let mut selections = Vec::new();

        for req in slot_requirements {
            let candidates = self.find_candidates(req);
            let selected = self.linucb_select(&candidates, req);
            selections.push(selected);
        }

        selections
    }

    /// Find candidate agents for a slot
    fn find_candidates(&self, req: &SlotRequirement) -> Vec<CandidateAgent> {
        let mut candidates = Vec::new();

        for (agent_id, signals) in &self.agents {
            // Find best matching signal for this requirement
            if let Some(best) = signals.iter().find(|s| s.capability >= req.min_capability) {
                candidates.push(CandidateAgent {
                    agent_id: agent_id.clone(),
                    capability: best.capability,
                    reliability: best.reliability,
                    latency_ms: best.latency_ms,
                    composite_score: self.composite_score(best, req),
                });
            }
        }

        // Sort by composite score
        candidates.sort_by(|a, b| b.composite_score.partial_cmp(&a.composite_score).unwrap());

        // Return top candidates
        candidates
            .into_iter()
            .take(self.config.max_candidates)
            .collect()
    }

    /// Calculate composite score for agent
    fn composite_score(&self, signal: &CapabilitySignal, req: &SlotRequirement) -> f32 {
        // Combine capability, reliability, latency, and load
        let capability_score = signal.capability * req.capability_weight;
        let reliability_score = signal.reliability * req.reliability_weight;
        let latency_score =
            (1.0 - (signal.latency_ms as f32 / 1000.0).min(1.0)) * req.latency_weight;
        let load_score = (1.0 - signal.load) * req.load_weight;

        capability_score + reliability_score + latency_score + load_score
    }

    /// LinUCB-style selection
    fn linucb_select(
        &self,
        candidates: &[CandidateAgent],
        req: &SlotRequirement,
    ) -> AgentSelection {
        if candidates.is_empty() {
            return AgentSelection {
                slot_id: req.slot_id.clone(),
                agent_id: None,
                confidence: 0.0,
                reasoning: "No matching agents found".to_string(),
            };
        }

        // Simple UCB-style selection with exploration
        let mut best_idx = 0;
        let mut best_score = f32::MIN;

        for (i, candidate) in candidates.iter().enumerate() {
            // Get historical performance for this agent
            let historical = self.get_agent_history(&candidate.agent_id);
            let exploitation = historical;
            let exploration =
                self.config.exploration_constant * ((1.0 / (historical + 1.0)).sqrt());

            let score = exploitation + exploration;

            if score > best_score {
                best_score = score;
                best_idx = i;
            }
        }

        AgentSelection {
            slot_id: req.slot_id.clone(),
            agent_id: Some(candidates[best_idx].agent_id.clone()),
            confidence: candidates[best_idx].composite_score,
            reasoning: format!("Selected via LinUCB (score: {:.3})", best_score),
        }
    }

    /// Get agent historical performance
    fn get_agent_history(&self, agent_id: &str) -> f32 {
        let selections: Vec<_> = self
            .selection_history
            .iter()
            .filter(|s| s.selected_agent == agent_id)
            .collect();

        if selections.is_empty() {
            return 0.5; // Prior
        }

        let total_reward: f32 = selections.iter().map(|s| s.reward).sum();
        total_reward / selections.len() as f32
    }

    /// Record selection result
    pub fn record_result(&mut self, slot_id: &str, agent_id: &str, reward: f32, context: &[f32]) {
        self.selection_history.push(SelectionRecord {
            slot_id: slot_id.to_string(),
            selected_agent: agent_id.to_string(),
            reward,
            context_features: context.to_vec(),
        });
    }
}

/// Requirement for a slot
#[derive(Debug, Clone)]
pub struct SlotRequirement {
    pub slot_id: String,
    pub min_capability: f32,
    pub capability_weight: f32,
    pub reliability_weight: f32,
    pub latency_weight: f32,
    pub load_weight: f32,
}

/// Candidate agent
#[derive(Debug, Clone)]
pub struct CandidateAgent {
    pub agent_id: String,
    pub capability: f32,
    pub reliability: f32,
    pub latency_ms: u32,
    pub composite_score: f32,
}

/// Agent selection result
#[derive(Debug, Clone)]
pub struct AgentSelection {
    pub slot_id: String,
    pub agent_id: Option<String>,
    pub confidence: f32,
    pub reasoning: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_dag_parallelism() {
        let deps = vec![
            TaskDependency {
                task_id: "a".to_string(),
                depends_on: vec![],
                complexity: 0.5,
                parallelism_width: 2,
                critical_path_depth: 1,
            },
            TaskDependency {
                task_id: "b".to_string(),
                depends_on: vec![],
                complexity: 0.5,
                parallelism_width: 2,
                critical_path_depth: 1,
            },
            TaskDependency {
                task_id: "c".to_string(),
                depends_on: vec!["a".to_string()],
                complexity: 0.5,
                parallelism_width: 1,
                critical_path_depth: 2,
            },
        ];

        let dag = TaskDag::from_dependencies(&deps);
        assert_eq!(dag.parallelism_width(), 2);
        assert_eq!(dag.critical_path_depth(), 2);
    }

    #[test]
    fn test_orchestrator_topology_selection() {
        let config = OrchestrationConfig::default();
        let orch = AdaptiveOrchestrator::new(config);

        let deps = vec![
            TaskDependency {
                task_id: "a".to_string(),
                depends_on: vec![],
                complexity: 0.3,
                parallelism_width: 3,
                critical_path_depth: 1,
            },
            TaskDependency {
                task_id: "b".to_string(),
                depends_on: vec![],
                complexity: 0.3,
                parallelism_width: 3,
                critical_path_depth: 1,
            },
            TaskDependency {
                task_id: "c".to_string(),
                depends_on: vec![],
                complexity: 0.3,
                parallelism_width: 3,
                critical_path_depth: 1,
            },
        ];

        let dag = TaskDag::from_dependencies(&deps);
        let selection = orch.select_topology(&dag);

        assert_eq!(selection.topology, Topology::Parallel);
    }

    #[test]
    fn test_symphony_coordinator() {
        let mut coord = SymphonyCoordinator::new(SymphonyConfig::default());

        coord.register_agent(
            "agent1",
            vec![CapabilitySignal {
                agent_id: "agent1".to_string(),
                capability: 0.9,
                reliability: 0.8,
                latency_ms: 100,
                load: 0.3,
            }],
        );

        let slots = vec![SlotRequirement {
            slot_id: "slot1".to_string(),
            min_capability: 0.5,
            capability_weight: 0.4,
            reliability_weight: 0.3,
            latency_weight: 0.2,
            load_weight: 0.1,
        }];

        let selections = coord.select_agents(&slots);
        assert!(!selections.is_empty());
    }
}
