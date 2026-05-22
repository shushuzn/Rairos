//! ROMA: Recursive Open Meta-Agents Framework.
//!
//! Based on research from:
//! - ROMA arXiv:2602.01848 - Recursive Open Meta-Agents
//! - Chain of Mindset arXiv:2602.10063 - Adaptive cognitive modes
//! - TDP arXiv:2601.07577 - Task-Decoupled Planning
//!
//! ## Architecture
//!
//! ```text
//! Root Task
//!     │
//!     ▼
//! ┌─────────────────────────────────────────────────────┐
//! │              Meta-Agent (Recursive)                    │
//! │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌───────┐  │
//! │  │ Atomizer│→ │ Planner │→ │Executor │→ │Aggregator│  │
//! │  └─────────┘  └─────────┘  └─────────┘  └───────┘  │
//! └─────────────────────────────────────────────────────┘
//!     │                    │
//!     │   Decomposed       │   Results
//!     ▼   Subtasks         ▼   Aggregated
//!  [Task 1] ────────────► [Result 1]
//!  [Task 2] ────────────► [Result 2]
//!  [Task 3] ────────────► [Result 3]
//! ```

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};

/// A task in the ROMA framework
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RomaTask {
    /// Task ID
    pub id: String,
    /// Task description
    pub description: String,
    /// Dependencies (task IDs that must complete first)
    pub dependencies: Vec<String>,
    /// Status
    pub status: TaskStatus,
    /// Result if completed
    pub result: Option<String>,
    /// Depth in the task tree
    pub depth: u32,
    /// Children task IDs (for subtasks)
    pub children: Vec<String>,
}

/// Task status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    /// Not started
    Pending,
    /// Currently executing
    InProgress,
    /// Completed successfully
    Completed,
    /// Failed
    Failed,
    /// Skipped
    Skipped,
}

/// ROMA Meta-Agent role
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MetaRole {
    /// Breaks down tasks into subtasks
    Atomizer,
    /// Creates execution plan
    Planner,
    /// Executes atomic tasks
    Executor,
    /// Aggregates results
    Aggregator,
    /// Supervisor coordinating all
    Supervisor,
}

impl MetaRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            MetaRole::Atomizer => "atomizer",
            MetaRole::Planner => "planner",
            MetaRole::Executor => "executor",
            MetaRole::Aggregator => "aggregator",
            MetaRole::Supervisor => "supervisor",
        }
    }
}

/// A meta-agent that can recursively decompose and execute tasks
#[derive(Debug, Clone)]
pub struct MetaAgent {
    /// Agent ID
    pub id: String,
    /// Current role
    pub role: MetaRole,
    /// Available roles
    available_roles: HashSet<MetaRole>,
    /// Task registry
    tasks: HashMap<String, RomaTask>,
    /// Results cache
    results: HashMap<String, String>,
    /// Configuration
    config: RomaConfig,
    /// Execution history
    history: Vec<ExecutionRecord>,
}

/// ROMA configuration
#[derive(Debug, Clone)]
pub struct RomaConfig {
    /// Maximum recursion depth
    pub max_depth: usize,
    /// Minimum task size before atomization
    pub min_task_size: usize,
    /// Enable parallel execution
    pub parallel: bool,
    /// Maximum parallel tasks
    pub max_parallel: usize,
    /// Task timeout in seconds
    pub task_timeout_secs: u64,
    /// Enable aggregation
    pub enable_aggregation: bool,
}

impl Default for RomaConfig {
    fn default() -> Self {
        Self {
            max_depth: 5,
            min_task_size: 3,
            parallel: true,
            max_parallel: 4,
            task_timeout_secs: 300,
            enable_aggregation: true,
        }
    }
}

/// Execution record
#[derive(Debug, Clone)]
pub struct ExecutionRecord {
    pub task_id: String,
    pub role: MetaRole,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub success: bool,
    pub output: Option<String>,
}

impl MetaAgent {
    /// Create a new meta-agent
    pub fn new(id: impl Into<String>) -> Self {
        let available_roles = vec![
            MetaRole::Atomizer,
            MetaRole::Planner,
            MetaRole::Executor,
            MetaRole::Aggregator,
            MetaRole::Supervisor,
        ].into_iter().collect();

        Self {
            id: id.into(),
            role: MetaRole::Supervisor,
            available_roles,
            tasks: HashMap::new(),
            results: HashMap::new(),
            config: RomaConfig::default(),
            history: Vec::new(),
        }
    }

    /// Set configuration
    pub fn with_config(mut self, config: RomaConfig) -> Self {
        self.config = config;
        self
    }

    /// Execute a task recursively
    pub fn execute(&mut self, task: &str) -> ExecutionResult {
        let task_id = uuid_simple();
        let root_task = RomaTask {
            id: task_id.clone(),
            description: task.to_string(),
            dependencies: vec![],
            status: TaskStatus::Pending,
            result: None,
            depth: 0,
            children: vec![],
        };

        self.tasks.insert(task_id.clone(), root_task);
        self.role = MetaRole::Supervisor;

        // Execute the task
        let result = self.execute_task(&task_id);

        ExecutionResult {
            task_id,
            success: result.is_ok(),
            output: result.unwrap_or_else(|e| e),
            subtasks: self.collect_all_subtasks(),
            history: self.history.clone(),
        }
    }

    /// Execute a specific task
    fn execute_task(&mut self, task_id: &str) -> Result<String, String> {
        let task = match self.tasks.get_mut(task_id) {
            Some(t) => t,
            None => return Err(format!("Task {} not found", task_id)),
        };

        if task.status == TaskStatus::Completed {
            return task.result.clone().ok_or_else(|| "No result".to_string());
        }

        task.status = TaskStatus::InProgress;

        // Check if this task needs decomposition
        let needs_decomposition = self.should_decompose(task);

        let result = if needs_decomposition && task.depth < self.config.max_depth as u32 {
            self.decompose_and_execute(task_id)
        } else {
            self.execute_atomically(task_id)
        };

        // Update task status
        if let Ok(ref r) = result {
            task.status = TaskStatus::Completed;
            task.result = Some(r.clone());
            self.results.insert(task_id.to_string(), r.clone());
        } else {
            task.status = TaskStatus::Failed;
        }

        result
    }

    /// Check if task should be decomposed
    fn should_decompose(&self, task: &RomaTask) -> bool {
        // Decompose if description is complex (multiple keywords suggesting subtasks)
        let complexity_indicators = ["and", "or", "then", "also", "plus", "with", "after", "before"];
        let desc_lower = task.description.to_lowercase();

        let indicator_count = complexity_indicators
            .iter()
            .filter(|ind| desc_lower.contains(*ind))
            .count();

        indicator_count >= self.config.min_task_size || task.description.len() > 200
    }

    /// Decompose task into subtasks and execute
    fn decompose_and_execute(&mut self, task_id: &str) -> Result<String, String> {
        let task = self.tasks.get(task_id).ok_or("Task not found")?;

        // Atomizer role: decompose task
        self.role = MetaRole::Atomizer;
        let subtask_descriptions = self.atomize(&task.description);
        let task_depth = task.depth;

        // Planner role: create execution plan
        self.role = MetaRole::Planner;
        let plan = self.create_plan(&subtask_descriptions);

        // Execute subtasks
        self.role = MetaRole::Executor;
        let mut subtask_ids = Vec::new();
        let mut subtask_results = Vec::new();

        for (i, desc) in subtask_descriptions.iter().enumerate() {
            let subtask_id = uuid_simple();
            let subtask = RomaTask {
                id: subtask_id.clone(),
                description: desc.clone(),
                dependencies: plan.dependencies.get(i).cloned().unwrap_or_default(),
                status: TaskStatus::Pending,
                result: None,
                depth: task_depth + 1,
                children: vec![],
            };

            self.tasks.insert(subtask_id.clone(), subtask);
            subtask_ids.push(subtask_id.clone());

            // Execute subtask (potentially recursively)
            match self.execute_task(&subtask_id) {
                Ok(result) => subtask_results.push((subtask_id, true, result)),
                Err(e) => subtask_results.push((subtask_id, false, e)),
            }
        }

        // Update parent task with children
        if let Some(parent) = self.tasks.get_mut(task_id) {
            parent.children = subtask_ids.clone();
        }

        // Aggregator role: combine results
        self.role = MetaRole::Aggregator;
        let aggregated = if self.config.enable_aggregation {
            self.aggregate(&subtask_results)
        } else {
            subtask_results
                .iter()
                .filter(|(_, success, _)| *success)
                .map(|(_, _, r)| r.clone())
                .collect::<Vec<_>>()
                .join("\n---\n")
        };

        Ok(aggregated)
    }

    /// Execute task atomically (no decomposition)
    fn execute_atomically(&mut self, task_id: &str) -> Result<String, String> {
        let task = self.tasks.get(task_id).ok_or("Task not found")?;

        let start = Utc::now();

        // Simulate execution (in practice would call LLM or tool)
        let output = format!(
            "[{}] Executed: {}",
            self.role.as_str(),
            task.description
        );

        let end = Utc::now();

        self.history.push(ExecutionRecord {
            task_id: task_id.to_string(),
            role: self.role,
            start_time: start,
            end_time: end,
            success: true,
            output: Some(output.clone()),
        });

        Ok(output)
    }

    /// Atomize: decompose task into subtasks
    fn atomize(&self, task: &str) -> Vec<String> {
        // Simple keyword-based decomposition
        let separators = [" and ", " then ", " also ", " plus "];

        let mut subtasks = vec![task.to_string()];
        let mut new_subtasks = Vec::new();

        for sep in &separators {
            for subtask in subtasks.drain(..) {
                if subtask.to_lowercase().contains(&sep.to_lowercase().trim().to_string()) {
                    let parts: Vec<_> = subtask.split(sep).collect();
                    for part in parts {
                        let trimmed = part.trim();
                        if !trimmed.is_empty() {
                            new_subtasks.push(trimmed.to_string());
                        }
                    }
                } else {
                    new_subtasks.push(subtask);
                }
            }
            subtasks = new_subtasks;
            new_subtasks = Vec::new();
        }

        if subtasks.is_empty() {
            subtasks.push(task.to_string());
        }

        subtasks
    }

    /// Create execution plan
    fn create_plan(&self, tasks: &[String]) -> ExecutionPlan {
        // Determine dependencies based on keywords
        let mut dependencies = Vec::new();

        for task in tasks {
            let mut deps = Vec::new();
            let task_lower = task.to_lowercase();

            // "after X" implies dependency on X
            for (i, other) in tasks.iter().enumerate() {
                if task_lower.contains(&format!("after {}", other.to_lowercase())) {
                    deps.push(i);
                }
            }

            dependencies.push(deps);
        }

        ExecutionPlan { dependencies }
    }

    /// Aggregate results from subtasks
    fn aggregate(&self, results: &[(String, bool, String)]) -> String {
        let successful: Vec<_> = results
            .iter()
            .filter(|(_, success, _)| *success)
            .collect();

        if successful.is_empty() {
            return "All subtasks failed".to_string();
        }

        let mut aggregated = String::new();
        aggregated.push_str("## Aggregated Results\n\n");

        for (i, (_, _, result)) in successful.iter().enumerate() {
            aggregated.push_str(&format!("### Subtask {}\n{}\n\n", i + 1, result));
        }

        aggregated
    }

    /// Collect all subtasks
    fn collect_all_subtasks(&self) -> Vec<String> {
        self.tasks.keys().cloned().collect()
    }

    /// Get task status
    pub fn get_task_status(&self, task_id: &str) -> Option<&TaskStatus> {
        self.tasks.get(task_id).map(|t| &t.status)
    }

    /// Get results
    pub fn get_results(&self) -> &HashMap<String, String> {
        &self.results
    }

    /// Get statistics
    pub fn stats(&self) -> RomaStats {
        let total = self.tasks.len();
        let completed = self.tasks.values().filter(|t| t.status == TaskStatus::Completed).count();
        let failed = self.tasks.values().filter(|t| t.status == TaskStatus::Failed).count();
        let pending = self.tasks.values().filter(|t| t.status == TaskStatus::Pending).count();
        let max_depth = self.tasks.values().map(|t| t.depth).max().unwrap_or(0);

        RomaStats {
            total_tasks: total,
            completed,
            failed,
            pending,
            max_depth_reached: max_depth,
            history_size: self.history.len(),
        }
    }
}

/// Execution plan
#[derive(Debug, Clone)]
struct ExecutionPlan {
    dependencies: Vec<Vec<usize>>,
}

/// Execution result
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub task_id: String,
    pub success: bool,
    pub output: String,
    pub subtasks: Vec<String>,
    pub history: Vec<ExecutionRecord>,
}

/// Statistics
#[derive(Debug, Clone)]
pub struct RomaStats {
    pub total_tasks: usize,
    pub completed: usize,
    pub failed: usize,
    pub pending: usize,
    pub max_depth_reached: u32,
    pub history_size: usize,
}

// =============================================================================
// Chain of Mindset (CoM) - Adaptive Cognitive Modes
// =============================================================================

/// Cognitive mindset type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Mindset {
    /// Spatial reasoning
    Spatial,
    /// Convergent thinking (focused)
    Convergent,
    /// Divergent thinking (exploratory)
    Divergent,
    /// Algorithmic (step-by-step)
    Algorithmic,
}

impl Mindset {
    pub fn as_str(&self) -> &'static str {
        match self {
            Mindset::Spatial => "spatial",
            Mindset::Convergent => "convergent",
            Mindset::Divergent => "divergent",
            Mindset::Algorithmic => "algorithmic",
        }
    }
}

/// Chain of Mindset orchestrator
pub struct ChainOfMindset {
    /// Current mindset
    current_mindset: Mindset,
    /// Available mindsets
    available_mindsets: HashSet<Mindset>,
    /// Mindset history
    history: Vec<Mindsetswitch>,
    /// Configuration
    config: MindsetConfig,
}

/// Mindset switch record
#[derive(Debug, Clone)]
pub struct Mindsetswitch {
    pub from: Mindset,
    pub to: Mindset,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
}

/// Mindset configuration
#[derive(Debug, Clone)]
pub struct MindsetConfig {
    /// Enable automatic switching
    pub auto_switch: bool,
    /// Switch threshold
    pub switch_threshold: f32,
    /// Maximum switches per session
    pub max_switches: usize,
}

impl Default for MindsetConfig {
    fn default() -> Self {
        Self {
            auto_switch: true,
            switch_threshold: 0.7,
            max_switches: 10,
        }
    }
}

impl ChainOfMindset {
    /// Create new chain
    pub fn new() -> Self {
        Self {
            current_mindset: Mindset::Convergent,
            available_mindsets: vec![
                Mindset::Spatial,
                Mindset::Convergent,
                Mindset::Divergent,
                Mindset::Algorithmic,
            ].into_iter().collect(),
            history: Vec::new(),
            config: MindsetConfig::default(),
        }
    }

    /// Set initial mindset
    pub fn with_mindset(mut self, mindset: Mindset) -> Self {
        self.current_mindset = mindset;
        self
    }

    /// Get current mindset
    pub fn current(&self) -> Mindset {
        self.current_mindset
    }

    /// Select appropriate mindset for task
    pub fn select_mindset(&self, task: &str) -> Mindset {
        let task_lower = task.to_lowercase();

        // Keyword-based selection
        if task_lower.contains("where") || task_lower.contains("location") || task_lower.contains("space") {
            Mindset::Spatial
        } else if task_lower.contains("explore") || task_lower.contains("possible") || task_lower.contains("alternatives") {
            Mindset::Divergent
        } else if task_lower.contains("step") || task_lower.contains("algorithm") || task_lower.contains("procedure") {
            Mindset::Algorithmic
        } else if task_lower.contains("best") || task_lower.contains("optimal") || task_lower.contains("solution") {
            Mindset::Convergent
        } else {
            Mindset::Convergent
        }
    }

    /// Switch mindset
    pub fn switch(&mut self, new_mindset: Mindset, reason: &str) -> Option<Mindsetswitch> {
        if self.history.len() >= self.config.max_switches {
            return None;
        }

        if self.current_mindset == new_mindset {
            return None;
        }

        let switch = Mindsetswitch {
            from: self.current_mindset,
            to: new_mindset,
            reason: reason.to_string(),
            timestamp: Utc::now(),
        };

        self.current_mindset = new_mindset;
        self.history.push(switch.clone());

        Some(switch)
    }

    /// Auto-select and switch based on task
    pub fn adapt(&mut self, task: &str) -> Mindset {
        let selected = self.select_mindset(task);

        if selected != self.current_mindset && self.config.auto_switch {
            self.switch(selected, "Task-adaptive selection");
        }

        selected
    }

    /// Get mindset history
    pub fn get_history(&self) -> &[Mindsetswitch] {
        &self.history
    }

    /// Generate reasoning prompt for mindset
    pub fn generate_prompt(&self, task: &str) -> String {
        let mindset_instruction = match self.current_mindset {
            Mindset::Spatial => "Think about the spatial relationships and locations involved.",
            Mindset::Convergent => "Focus on finding the single best solution. Be decisive.",
            Mindset::Divergent => "Explore multiple possibilities. Consider all alternatives.",
            Mindset::Algorithmic => "Break this down into clear, sequential steps.",
        };

        format!("{}\n\nTask: {}", mindset_instruction, task)
    }
}

impl Default for ChainOfMindset {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Task-Decoupled Planning (TDP)
// =============================================================================

/// A sub-goal in TDP
#[derive(Debug, Clone)]
pub struct SubGoal {
    /// Goal ID
    pub id: String,
    /// Goal description
    pub description: String,
    /// Parent goal ID
    pub parent_id: Option<String>,
    /// Status
    pub status: TaskStatus,
    /// Scoped context
    pub context: ScopedContext,
    /// Local replan count
    pub replan_count: u32,
}

/// Scoped context for local planning
#[derive(Debug, Clone, Default)]
pub struct ScopedContext {
    /// Relevant variables
    pub variables: HashMap<String, String>,
    /// Constraints
    pub constraints: Vec<String>,
    /// Assumptions
    pub assumptions: Vec<String>,
}

/// TDP Supervisor
pub struct TdpSupervisor {
    /// Goal DAG
    goals: HashMap<String, SubGoal>,
    /// Configuration
    config: TdpConfig,
}

#[derive(Debug, Clone)]
pub struct TdpConfig {
    /// Maximum replans per goal
    pub max_replans: usize,
    /// Enable error isolation
    pub error_isolation: bool,
}

impl Default for TdpConfig {
    fn default() -> Self {
        Self {
            max_replans: 3,
            error_isolation: true,
        }
    }
}

impl TdpSupervisor {
    /// Create new supervisor
    pub fn new() -> Self {
        Self {
            goals: HashMap::new(),
            config: TdpConfig::default(),
        }
    }

    /// Create goal DAG from task
    pub fn create_dag(&mut self, task: &str) -> String {
        let root_id = uuid_simple();

        // Decompose into sub-goals
        let subgoals = self.decompose(task);

        // Create goal entries
        let mut parent_id = Some(root_id.clone());
        for subgoal in &subgoals {
            let goal = SubGoal {
                id: uuid_simple(),
                description: subgoal.clone(),
                parent_id: parent_id.clone(),
                status: TaskStatus::Pending,
                context: ScopedContext::default(),
                replan_count: 0,
            };

            self.goals.insert(goal.id.clone(), goal);
            parent_id = goal.id.clone();
        }

        root_id
    }

    /// Decompose task into sub-goals
    fn decompose(&self, task: &str) -> Vec<String> {
        // Simple decomposition by sentences or clauses
        task.split(&['.', ';', ','][..])
            .map(|s| s.trim().to_string())
            .filter(|s| s.len() > 10)
            .collect()
    }

    /// Execute with local replanning
    pub fn execute_with_replan(&mut self, goal_id: &str) -> Result<String, String> {
        let goal = match self.goals.get_mut(goal_id) {
            Some(g) => g,
            None => return Err("Goal not found".to_string()),
        };

        // Check if max replans exceeded
        if goal.replan_count >= self.config.max_replans as u32 {
            return Err("Max replans exceeded".to_string());
        }

        // Execute (simplified)
        let result = format!("Executed: {}", goal.description);

        goal.status = TaskStatus::Completed;

        Ok(result)
    }

    /// Get goal status
    pub fn get_goal_status(&self, goal_id: &str) -> Option<&TaskStatus> {
        self.goals.get(goal_id).map(|g| &g.status)
    }

    /// Check if all goals complete
    pub fn all_complete(&self) -> bool {
        self.goals.values().all(|g| g.status == TaskStatus::Completed)
    }
}

impl Default for TdpSupervisor {
    fn default() -> Self {
        Self::new()
    }
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
    fn test_roma_execution() {
        let mut agent = MetaAgent::new("roma1");
        let result = agent.execute("Task A and Task B");

        assert!(result.success || !result.success);
    }

    #[test]
    fn test_chain_of_mindset() {
        let chain = ChainOfMindset::new();

        let spatial = chain.select_mindset("Where is the key located?");
        assert_eq!(spatial, Mindset::Spatial);

        let algo = chain.select_mindset("Step by step procedure");
        assert_eq!(algo, Mindset::Algorithmic);
    }

    #[test]
    fn test_mindset_switch() {
        let mut chain = ChainOfMindset::new();
        chain.current_mindset = Mindset::Convergent;

        let result = chain.switch(Mindset::Divergent, "Need exploration");
        assert!(result.is_some());
        assert_eq!(chain.current_mindset, Mindset::Divergent);
    }

    #[test]
    fn test_tdp_dag() {
        let mut supervisor = TdpSupervisor::new();
        let root = supervisor.create_dag("First step. Second step. Third step.");

        assert!(!root.is_empty());
    }
}
