//! DAG Workflow Executor for parallel task execution.
//!
//! Based on research from:
//! - ROMA (arXiv:2602.01848) - Recursive open meta-agents
//! - AdaptOrch (arXiv:2602.16873) - DAG topology routing
//! - ParaManager (arXiv:2604.17009) - Parallel task execution

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

/// A node in the execution DAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagNode {
    /// Node ID
    pub id: String,
    /// Task description
    pub task: String,
    /// Tool to use (empty for agent-only)
    pub tool: Option<String>,
    /// Node IDs this depends on
    pub depends_on: Vec<String>,
    /// Whether this node has completed
    pub is_completed: bool,
    /// Result from execution
    pub result: Option<String>,
    /// Error if failed
    pub error: Option<String>,
}

impl DagNode {
    /// Create a new DAG node
    pub fn new(id: &str, task: &str) -> Self {
        Self {
            id: id.to_string(),
            task: task.to_string(),
            tool: None,
            depends_on: Vec::new(),
            is_completed: false,
            result: None,
            error: None,
        }
    }

    /// Set tool for this node
    pub fn with_tool(mut self, tool: &str) -> Self {
        self.tool = Some(tool.to_string());
        self
    }

    /// Add dependency
    pub fn depends_on(mut self, node_id: &str) -> Self {
        self.depends_on.push(node_id.to_string());
        self
    }
}

/// A directed acyclic graph for task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDag {
    /// Nodes by ID
    pub nodes: HashMap<String, DagNode>,
}

impl TaskDag {
    /// Create a new empty DAG
    pub fn new() -> Self {
        Self { nodes: HashMap::new() }
    }

    /// Add a node to the DAG
    pub fn add_node(&mut self, node: DagNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    /// Get a node by ID
    pub fn get_node(&self, id: &str) -> Option<&DagNode> {
        self.nodes.get(id)
    }

    /// Get mutable node by ID
    pub fn get_node_mut(&mut self, id: &str) -> Option<&mut DagNode> {
        self.nodes.get_mut(id)
    }

    /// Compute topological execution order using Kahn's algorithm
    /// Returns Vec of levels where each level can execute in parallel
    pub fn compute_execution_order(&self) -> Vec<Vec<String>> {
        let mut in_degree: HashMap<String, usize> = self
            .nodes
            .keys()
            .map(|id| (id.clone(), 0))
            .collect();

        // Correct in-degree: number of prerequisites (incoming edges) for each node
        // in_degree[x] = number of nodes that x depends on
        for node in self.nodes.values() {
            if let Some(count) = in_degree.get_mut(&node.id) {
                *count = node.depends_on.len();
            }
        }

        // Build reverse adjacency list: for each node, which nodes depend on it
        // This converts O(V*E) lookup to O(V+E)
        let mut dependents: HashMap<String, Vec<String>> = HashMap::new();
        for node in self.nodes.values() {
            for dep in &node.depends_on {
                dependents.entry(dep.clone()).or_default().push(node.id.clone());
            }
        }

        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter(|(_, &count)| count == 0)
            .map(|(id, _)| id.clone())
            .collect();

        let mut execution_order: Vec<Vec<String>> = Vec::new();

        while !queue.is_empty() {
            let level_size = queue.len();
            let mut current_level = Vec::new();

            for _ in 0..level_size {
                if let Some(node_id) = queue.pop_front() {
                    current_level.push(node_id.clone());
                    // Use reverse adjacency list instead of scanning all nodes
                    if let Some(deps) = dependents.get(&node_id) {
                        for dependent_id in deps {
                            if let Some(deg) = in_degree.get_mut(dependent_id) {
                                *deg -= 1;
                                if *deg == 0 {
                                    queue.push_back(dependent_id.clone());
                                }
                            }
                        }
                    }
                }
            }

            if !current_level.is_empty() {
                execution_order.push(current_level);
            }
        }

        execution_order
    }

    /// Check if all nodes are completed
    pub fn is_complete(&self) -> bool {
        self.nodes.values().all(|n| n.is_completed)
    }

    /// Get total node count
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get critical path length (number of levels)
    pub fn critical_path_length(&self) -> usize {
        self.compute_execution_order().len()
    }

    /// Get maximum parallelism (largest level width)
    pub fn max_parallelism(&self) -> usize {
        self.compute_execution_order()
            .iter()
            .map(|level| level.len())
            .max()
            .unwrap_or(0)
    }
}

impl Default for TaskDag {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of DAG execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagExecutionResult {
    /// Whether all tasks succeeded
    pub success: bool,
    /// Results by node ID
    pub node_results: HashMap<String, NodeResult>,
    /// Total execution time in ms
    pub total_time_ms: u64,
    /// Critical path time in ms
    pub critical_path_time_ms: u64,
}

/// Result from a single node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeResult {
    /// Node ID
    pub node_id: String,
    /// Whether this node succeeded
    pub success: bool,
    /// Output result
    pub output: Option<String>,
    /// Error message
    pub error: Option<String>,
    /// Execution time in ms
    pub execution_time_ms: u64,
}

/// Simple callback-based DAG Executor
/// User provides a closure that executes a single node
pub struct DagExecutor<F> {
    executor: F,
}

impl<F> DagExecutor<F>
where
    F: Fn(&str, &str, Option<&str>) -> Result<String, String>,
{
    /// Create a new DAG executor with the given executor function
    pub fn new(executor: F) -> Self {
        Self { executor }
    }

    /// Execute the DAG synchronously
    pub fn execute(&self, dag: &mut TaskDag) -> DagExecutionResult {
        let start_time = std::time::Instant::now();
        let mut node_results: HashMap<String, NodeResult> = HashMap::new();
        let execution_order = dag.compute_execution_order();

        for level in &execution_order {
            let mut level_results = Vec::new();

            for node_id in level {
                let node = dag.get_node(node_id).unwrap();
                let node_start = std::time::Instant::now();

                let result = (self.executor)(&node.id, &node.task, node.tool.as_deref());

                let execution_time_ms = node_start.elapsed().as_millis() as u64;

                let node_result = match result {
                    Ok(output) => NodeResult {
                        node_id: node_id.clone(),
                        success: true,
                        output: Some(output),
                        error: None,
                        execution_time_ms,
                    },
                    Err(e) => NodeResult {
                        node_id: node_id.clone(),
                        success: false,
                        output: None,
                        error: Some(e),
                        execution_time_ms,
                    },
                };

                level_results.push((node_id.clone(), node_result));
            }

            // Collect results and update DAG
            for (node_id, result) in level_results {
                if let Some(node) = dag.get_node_mut(&node_id) {
                    node.is_completed = true;
                    node.result = result.output.clone();
                    node.error = result.error.clone();
                }
                node_results.insert(node_id, result);
            }
        }

        let total_time_ms = start_time.elapsed().as_millis() as u64;
        let success = node_results.values().all(|r| r.success);

        DagExecutionResult {
            success,
            node_results,
            total_time_ms,
            critical_path_time_ms: total_time_ms,
        }
    }
}

/// Helper to build a DAG from plan steps
pub fn dag_from_plan_steps(steps: &[super::sparks_crew::PlanStep]) -> TaskDag {
    let mut dag = TaskDag::new();

    for step in steps {
        let mut node = DagNode::new(&format!("step_{}", step.step), &step.task);

        if !step.tool.is_empty() {
            node = node.with_tool(&step.tool);
        }

        for dep in &step.depends_on {
            node = node.depends_on(&format!("step_{}", dep));
        }

        dag.add_node(node);
    }

    dag
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dag_node_creation() {
        let node = DagNode::new("1", "Analyze data")
            .with_tool("python")
            .depends_on("0");

        assert_eq!(node.id, "1");
        assert_eq!(node.task, "Analyze data");
        assert_eq!(node.tool, Some("python".to_string()));
        assert_eq!(node.depends_on, vec!["0"]);
    }

    #[test]
    fn test_execution_order_parallel() {
        let mut dag = TaskDag::new();
        dag.add_node(DagNode::new("a", "Task A"));
        dag.add_node(DagNode::new("b", "Task B"));
        dag.add_node(DagNode::new("c", "Task C").depends_on("a").depends_on("b"));

        let order = dag.compute_execution_order();

        assert!(order[0].contains(&"a".to_string()));
        assert!(order[0].contains(&"b".to_string()));
        assert!(order[1].contains(&"c".to_string()));
    }

    #[test]
    fn test_dag_max_parallelism() {
        let mut dag = TaskDag::new();
        dag.add_node(DagNode::new("a", "Task A"));
        dag.add_node(DagNode::new("b", "Task B"));
        dag.add_node(DagNode::new("c", "Task C"));

        assert_eq!(dag.max_parallelism(), 3);
    }

    #[test]
    fn test_dag_executor() {
        let mut dag = TaskDag::new();
        dag.add_node(DagNode::new("1", "Simple task"));

        let executor = DagExecutor::new(|id, _task, _tool| {
            Ok(format!("Result from {}", id))
        });

        let result = executor.execute(&mut dag);
        assert!(result.success);
        assert!(result.node_results.get("1").unwrap().output.is_some());
    }

    #[test]
    fn test_dag_from_plan_steps() {
        let steps = vec![
            super::super::sparks_crew::PlanStep {
                step: 1,
                task: "Search Materials Project".to_string(),
                tool: "mp_search".to_string(),
                inputs: HashMap::new(),
                depends_on: vec![],
            },
            super::super::sparks_crew::PlanStep {
                step: 2,
                task: "Run DFT calculation".to_string(),
                tool: "dft_calc".to_string(),
                inputs: HashMap::new(),
                depends_on: vec![1],
            },
        ];

        let dag = dag_from_plan_steps(&steps);
        assert_eq!(dag.node_count(), 2);
    }
}
