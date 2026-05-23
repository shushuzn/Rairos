//! Memory Intelligence Agent (MIA) Module
//!
//! Based on arXiv:2604.04503 - Memory Intelligence Agent for Deep Research Agents
//!
//! ## Architecture
//!
//! ```text
//! Query → Memory Manager (non-parametric)
//!              │ compressed trajectories
//!              ▼
//!          Planner (parametric) → generates search plans
//!              │
//!              ▼
//!          Executor → executes search plan
//!              │
//!              ▼
//!         ┌────┴────┐
//!         │         │
//!         ▼         ▼
//!    [Reflection] [Unsupervised Judgment]
//!         │
//!         ▼
//!    Memory Evolution (bidirectional parametric ↔ non-parametric)
//! ```
//!
//! ## Key Innovations
//!
//! 1. **Memory Manager**: Non-parametric memory storing compressed historical trajectories
//! 2. **Planner**: Parametric memory agent producing search plans
//! 3. **Executor**: Agent executing search-guided information retrieval
//! 4. **Bidirectional Conversion Loop**: Between parametric and non-parametric memory
//! 5. **Reflection & Judgment**: Boosting reasoning and self-evolution

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::Hash;

use crate::utils::uuid_simple;

/// Maximum trajectory length to store
const MAX_TRAJECTORY_LEN: usize = 100;

/// Compression ratio for trajectory summarization
const COMPRESSION_RATIO: f32 = 0.3;

// =============================================================================
// Memory Manager (Non-Parametric Memory)
// =============================================================================

/// A compressed trajectory entry in memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedTrajectory {
    /// Trajectory ID
    pub id: String,
    /// Compressed representation
    pub summary: String,
    /// Key actions taken
    pub key_actions: Vec<String>,
    /// Query type this trajectory handled
    pub query_pattern: String,
    /// Success score (0.0 - 1.0)
    pub success_score: f32,
    /// Token cost
    pub token_cost: u32,
}

/// Memory Manager - non-parametric memory system
pub struct MemoryManager {
    /// Compressed trajectories
    trajectories: VecDeque<CompressedTrajectory>,
    /// Index by query pattern
    pattern_index: HashMap<String, VecDeque<usize>>,
    /// Maximum trajectories to store
    max_trajectories: usize,
    /// Total token cost
    total_token_cost: u64,
}

impl MemoryManager {
    /// Create new memory manager
    pub fn new(max_trajectories: usize) -> Self {
        Self {
            trajectories: VecDeque::new(),
            pattern_index: HashMap::new(),
            max_trajectories,
            total_token_cost: 0,
        }
    }

    /// Store a compressed trajectory
    pub fn store(&mut self, trajectory: CompressedTrajectory) {
        // Update pattern index
        let pattern = &trajectory.query_pattern;
        if let Some(indices) = self.pattern_index.get_mut(pattern) {
            indices.push_back(self.trajectories.len());
        } else {
            self.pattern_index.insert(pattern.clone(), VecDeque::from(vec![self.trajectories.len()]));
        }

        self.trajectories.push_back(trajectory);

        // Evict if over capacity
        if self.trajectories.len() > self.max_trajectories {
            self.evict_oldest();
        }
    }

    /// Retrieve similar trajectories by pattern
    pub fn retrieve(&self, pattern: &str, limit: usize) -> Vec<&CompressedTrajectory> {
        let mut results = Vec::new();

        // Direct match
        if let Some(indices) = self.pattern_index.get(pattern) {
            for &idx in indices.iter().take(limit) {
                if let Some(traj) = self.trajectories.get(idx) {
                    results.push(traj);
                }
            }
        }

        // Fuzzy match - include similar patterns
        if results.len() < limit {
            for (p, indices) in &self.pattern_index {
                if p.contains(pattern) || pattern.contains(p) {
                    for &idx in indices.iter().take(limit - results.len()) {
                        if !results.iter().any(|t| t.id == self.trajectories[idx].id) {
                            if let Some(traj) = self.trajectories.get(idx) {
                                results.push(traj);
                            }
                        }
                    }
                }
            }
        }

        results
    }

    /// Compress a trajectory
    pub fn compress(&self, trajectory: &[String]) -> CompressedTrajectory {
        let summary = if trajectory.len() > 3 {
            // Take first, middle, and last elements
            let first = trajectory.first().unwrap();
            let middle = trajectory.get(trajectory.len() / 2).unwrap();
            let last = trajectory.last().unwrap();
            format!("{} ... {} ... {}", first, middle, last)
        } else {
            trajectory.join(" → ")
        };

        CompressedTrajectory {
            id: uuid_simple(),
            summary,
            key_actions: trajectory.iter().take(5).cloned().collect(),
            query_pattern: String::new(),
            success_score: 0.5,
            token_cost: 0,
        }
    }

    /// Evict oldest trajectory
    fn evict_oldest(&mut self) {
        if let Some(oldest) = self.trajectories.front() {
            // Remove from pattern index
            let pattern = &oldest.query_pattern;
            if let Some(indices) = self.pattern_index.get_mut(pattern) {
                indices.pop_front();
                if indices.is_empty() {
                    self.pattern_index.remove(pattern);
                }
            }
            self.trajectories.pop_front();
        }
    }

    /// Get statistics
    pub fn stats(&self) -> MemoryStats {
        MemoryStats {
            trajectory_count: self.trajectories.len(),
            total_token_cost: self.total_token_cost,
            pattern_diversity: self.pattern_index.len(),
        }
    }
}

/// Memory statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub trajectory_count: usize,
    pub total_token_cost: u64,
    pub pattern_diversity: usize,
}

// =============================================================================
// Planner (Parametric Memory Agent)
// =============================================================================

/// A search plan generated by the planner
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchPlan {
    /// Plan ID
    pub id: String,
    /// Steps in the plan
    pub steps: Vec<SearchStep>,
    /// Estimated token cost
    pub estimated_cost: u32,
    /// Confidence score
    pub confidence: f32,
}

/// A single step in a search plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchStep {
    /// Step number
    pub step: usize,
    /// Action type
    pub action: SearchAction,
    /// Target query
    pub query: String,
    /// Expected output type
    pub output_type: String,
    /// Dependencies (step indices)
    pub depends_on: Vec<usize>,
}

/// Search actions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SearchAction {
    /// Search for papers
    PaperSearch,
    /// Extract information
    Extract,
    /// Analyze data
    Analyze,
    /// Synthesize findings
    Synthesize,
    /// Verify claim
    Verify,
}

/// Planner configuration
#[derive(Debug, Clone)]
pub struct PlannerConfig {
    /// Model for planning
    pub model: String,
    /// Max steps per plan
    pub max_steps: usize,
    /// Enable reflection
    pub enable_reflection: bool,
}

impl Default for PlannerConfig {
    fn default() -> Self {
        Self {
            model: "default".to_string(),
            max_steps: 10,
            enable_reflection: true,
        }
    }
}

/// Planner with parametric memory
pub struct Planner {
    config: PlannerConfig,
    /// Learned plan templates
    templates: Vec<PlanTemplate>,
    /// Reflection history
    reflection_history: Vec<Reflection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanTemplate {
    pub query_type: String,
    pub template_steps: Vec<SearchStep>,
    pub usage_count: u32,
    pub success_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reflection {
    pub plan_id: String,
    pub issue: String,
    pub suggestion: String,
    pub improvement: f32,
}

impl Planner {
    pub fn new(config: PlannerConfig) -> Self {
        Self {
            config,
            templates: Vec::new(),
            reflection_history: Vec::new(),
        }
    }

    /// Generate a search plan
    pub fn plan(&self, query: &str, context: &[String]) -> SearchPlan {
        let query_type = self.classify_query(query);

        // Try to use existing template
        if let Some(template) = self.templates.iter().find(|t| t.query_type == query_type) {
            return self.apply_template(template, query);
        }

        // Generate new plan
        self.generate_plan(query, context)
    }

    /// Classify query type
    fn classify_query(&self, query: &str) -> String {
        let query_lower = query.to_lowercase();

        if query_lower.contains("compare") || query_lower.contains("difference") {
            "comparison".to_string()
        } else if query_lower.contains("latest") || query_lower.contains("recent") {
            "update_search".to_string()
        } else if query_lower.contains("how") || query_lower.contains("why") {
            "explanatory".to_string()
        } else if query_lower.contains("list") || query_lower.contains("what are") {
            "enumerative".to_string()
        } else {
            "general".to_string()
        }
    }

    /// Apply existing template
    fn apply_template(&self, template: &PlanTemplate, query: &str) -> SearchPlan {
        let steps: Vec<SearchStep> = template
            .template_steps
            .iter()
            .enumerate()
            .map(|(i, s)| SearchStep {
                step: i,
                action: s.action.clone(),
                query: query.to_string(),
                output_type: s.output_type.clone(),
                depends_on: s.depends_on.clone(),
            })
            .collect();

        SearchPlan {
            id: uuid_simple(),
            steps,
            estimated_cost: template.template_steps.len() as u32 * 100,
            confidence: template.success_rate,
        }
    }

    /// Generate new plan
    fn generate_plan(&self, query: &str, _context: &[String]) -> SearchPlan {
        let query_type = self.classify_query(query);

        let steps = match query_type.as_str() {
            "comparison" => vec![
                SearchStep {
                    step: 0,
                    action: SearchAction::PaperSearch,
                    query: format!("{} comparison", query),
                    output_type: "list".to_string(),
                    depends_on: vec![],
                },
                SearchStep {
                    step: 1,
                    action: SearchAction::Extract,
                    query: "key differences".to_string(),
                    output_type: "table".to_string(),
                    depends_on: vec![0],
                },
                SearchStep {
                    step: 2,
                    action: SearchAction::Synthesize,
                    query: format!("compare: {}", query),
                    output_type: "analysis".to_string(),
                    depends_on: vec![1],
                },
            ],
            "update_search" => vec![
                SearchStep {
                    step: 0,
                    action: SearchAction::PaperSearch,
                    query: format!("latest {}", query),
                    output_type: "papers".to_string(),
                    depends_on: vec![],
                },
                SearchStep {
                    step: 1,
                    action: SearchAction::Verify,
                    query: "recency verification".to_string(),
                    output_type: "confirmation".to_string(),
                    depends_on: vec![0],
                },
            ],
            _ => vec![
                SearchStep {
                    step: 0,
                    action: SearchAction::PaperSearch,
                    query: query.to_string(),
                    output_type: "papers".to_string(),
                    depends_on: vec![],
                },
                SearchStep {
                    step: 1,
                    action: SearchAction::Extract,
                    query: "main findings".to_string(),
                    output_type: "summary".to_string(),
                    depends_on: vec![0],
                },
                SearchStep {
                    step: 2,
                    action: SearchAction::Synthesize,
                    query: format!("synthesize: {}", query),
                    output_type: "report".to_string(),
                    depends_on: vec![1],
                },
            ],
        };

        SearchPlan {
            id: uuid_simple(),
            estimated_cost: steps.len() as u32 * 100,
            confidence: 0.7,
            steps,
        }
    }

    /// Record plan outcome for learning
    pub fn record_outcome(&mut self, plan: &SearchPlan, success: f32) {
        let query_type = self.classify_query(&plan.steps.first().map(|s| s.query.as_str()).unwrap_or(""));

        // Update or create template
        if let Some(template) = self.templates.iter_mut().find(|t| t.query_type == query_type) {
            template.usage_count += 1;
            let n = template.usage_count as f32;
            template.success_rate = (template.success_rate * (n - 1.0) + success) / n;
        } else {
            self.templates.push(PlanTemplate {
                query_type,
                template_steps: plan.steps.clone(),
                usage_count: 1,
                success_rate: success,
            });
        }
    }

    /// Reflect on plan execution
    pub fn reflect(&self, plan: &SearchPlan, issues: Vec<String>) -> Vec<Reflection> {
        issues
            .into_iter()
            .map(|issue| Reflection {
                plan_id: plan.id.clone(),
                issue: issue.clone(),
                suggestion: format!("Consider alternative approach for: {}", issue),
                improvement: 0.1,
            })
            .collect()
    }
}

// =============================================================================
// Executor
// =============================================================================

/// Executor configuration
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    /// Tools available to executor
    pub available_tools: Vec<String>,
    /// Max retries per step
    pub max_retries: usize,
    /// Timeout per step (ms)
    pub step_timeout_ms: u64,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            available_tools: vec![
                "paper_search".to_string(),
                "database_query".to_string(),
                "web_search".to_string(),
                "file_extract".to_string(),
            ],
            max_retries: 3,
            step_timeout_ms: 30000,
        }
    }
}

/// Execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub step: usize,
    pub action: SearchAction,
    pub output: String,
    pub success: bool,
    pub token_cost: u32,
    pub error: Option<String>,
}

/// Executor agent
pub struct Executor {
    config: ExecutorConfig,
}

impl Executor {
    pub fn new(config: ExecutorConfig) -> Self {
        Self { config }
    }

    /// Execute a search step
    pub async fn execute_step(&self, step: &SearchStep) -> ExecutionResult {
        // Simulate execution
        let output = match step.action {
            SearchAction::PaperSearch => format!("Search results for: {}", step.query),
            SearchAction::Extract => format!("Extracted: {}", step.output_type),
            SearchAction::Analyze => format!("Analysis of: {}", step.query),
            SearchAction::Synthesize => format!("Synthesis: {}", step.query),
            SearchAction::Verify => format!("Verification: {}", step.query),
        };

        ExecutionResult {
            step: step.step,
            action: step.action.clone(),
            output,
            success: true,
            token_cost: 50,
            error: None,
        }
    }

    /// Execute full plan
    pub async fn execute_plan(&self, plan: &SearchPlan) -> PlanExecution {
        let mut results = Vec::new();
        let mut token_cost = 0u32;

        for step in &plan.steps {
            // Check dependencies
            let deps_satisfied = step.depends_on.iter().all(|&d| {
                results.iter().any(|r: &ExecutionResult| r.step == d && r.success)
            });

            if !deps_satisfied {
                results.push(ExecutionResult {
                    step: step.step,
                    action: step.action.clone(),
                    output: String::new(),
                    success: false,
                    token_cost: 0,
                    error: Some("Dependencies not satisfied".to_string()),
                });
                continue;
            }

            let result = self.execute_step(step).await;
            token_cost += result.token_cost;
            results.push(result);
        }

        let all_success = results.iter().all(|r| r.success);

        PlanExecution {
            plan_id: plan.id.clone(),
            results,
            total_token_cost: token_cost,
            success: all_success,
        }
    }
}

/// Result of executing a full plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanExecution {
    pub plan_id: String,
    pub results: Vec<ExecutionResult>,
    pub total_token_cost: u32,
    pub success: bool,
}

// =============================================================================
// Bidirectional Memory Conversion
// =============================================================================

/// Memory converter between parametric and non-parametric
pub struct MemoryConverter;

impl MemoryConverter {
    /// Convert parametric (plan templates) to non-parametric (compressed trajectories)
    pub fn parametric_to_nonparametric(templates: &[PlanTemplate]) -> Vec<CompressedTrajectory> {
        templates
            .iter()
            .map(|t| CompressedTrajectory {
                id: uuid_simple(),
                summary: format!("{:?}: {} steps", t.query_type, t.template_steps.len()),
                key_actions: t
                    .template_steps
                    .iter()
                    .take(3)
                    .map(|s| format!("{:?}", s.action))
                    .collect(),
                query_pattern: t.query_type.clone(),
                success_score: t.success_rate,
                token_cost: t.usage_count * 100,
            })
            .collect()
    }

    /// Convert non-parametric to parametric (learn templates from trajectories)
    pub fn nonparametric_to_parametric(
        trajectories: &[CompressedTrajectory],
    ) -> Vec<PlanTemplate> {
        let mut pattern_groups: HashMap<String, Vec<&CompressedTrajectory>> = HashMap::new();

        for traj in trajectories {
            pattern_groups
                .entry(traj.query_pattern.clone())
                .or_default()
                .push(traj);
        }

        pattern_groups
            .into_iter()
            .map(|(query_type, group)| {
                let avg_success = group.iter().map(|t| t.success_score).sum::<f32>() / group.len() as f32;
                let total_usage: u32 = group.iter().map(|t| t.token_cost / 100).sum();

                PlanTemplate {
                    query_type,
                    template_steps: vec![SearchStep {
                        step: 0,
                        action: SearchAction::PaperSearch,
                        query: "general".to_string(),
                        output_type: "papers".to_string(),
                        depends_on: vec![],
                    }],
                    usage_count: total_usage.max(1),
                    success_rate: avg_success,
                }
            })
            .collect()
    }
}

// =============================================================================
// MIA Main Agent
// =============================================================================

/// Memory Intelligence Agent
pub struct MemoryIntelligenceAgent {
    memory_manager: MemoryManager,
    planner: Planner,
    executor: Executor,
    config: MIAConfig,
}

/// MIA configuration
#[derive(Debug, Clone)]
pub struct MIAConfig {
    /// Max trajectories in memory
    pub max_trajectories: usize,
    /// Planner config
    pub planner_config: PlannerConfig,
    /// Executor config
    pub executor_config: ExecutorConfig,
    /// Enable bidirectional memory conversion
    pub enable_memory_evolution: bool,
    /// Enable reflection
    pub enable_reflection: bool,
}

impl Default for MIAConfig {
    fn default() -> Self {
        Self {
            max_trajectories: 1000,
            planner_config: PlannerConfig::default(),
            executor_config: ExecutorConfig::default(),
            enable_memory_evolution: true,
            enable_reflection: true,
        }
    }
}

impl MemoryIntelligenceAgent {
    /// Create new MIA
    pub fn new(config: MIAConfig) -> Self {
        Self {
            memory_manager: MemoryManager::new(config.max_trajectories),
            planner: Planner::new(config.planner_config.clone()),
            executor: Executor::new(config.executor_config.clone()),
            config,
        }
    }

    /// Run query through MIA
    pub async fn run(&mut self, query: &str) -> MIAResult {
        // 1. Check memory for similar queries
        let query_type = self.planner.classify_query(query);
        let similar_trajectories = self.memory_manager.retrieve(&query_type, 3);

        // 2. Generate plan
        let context: Vec<String> = similar_trajectories
            .iter()
            .map(|t| t.summary.clone())
            .collect();
        let plan = self.planner.plan(query, &context);

        // 3. Execute plan
        let execution = self.executor.execute_plan(&plan).await;

        // 4. Reflect if enabled
        let reflections = if self.config.enable_reflection && !execution.success {
            let issues = execution
                .results
                .iter()
                .filter(|r| !r.success)
                .map(|r| r.error.clone().unwrap_or_default())
                .collect();
            self.planner.reflect(&plan, issues)
        } else {
            vec![]
        };

        // 5. Store in memory
        let trajectory = CompressedTrajectory {
            id: uuid_simple(),
            summary: format!("Query: {} | Steps: {} | Success: {}",
                query, execution.results.len(), execution.success),
            key_actions: execution
                .results
                .iter()
                .filter(|r| r.success)
                .map(|r| format!("{:?}", r.action))
                .collect(),
            query_pattern: query_type,
            success_score: if execution.success { 1.0 } else { 0.5 },
            token_cost: execution.total_token_cost,
        };
        self.memory_manager.store(trajectory);

        // 6. Record planner outcome
        self.planner.record_outcome(
            &plan,
            if execution.success { 1.0 } else { 0.5 },
        );

        // 7. Memory evolution if enabled
        if self.config.enable_memory_evolution {
            self.evolve_memory();
        }

        MIAResult {
            query: query.to_string(),
            plan,
            execution,
            reflections,
            memory_stats: self.memory_manager.stats(),
        }
    }

    /// Evolve memory bidirectionally
    fn evolve_memory(&mut self) {
        // This would involve alternating RL training between planner and executor
        // For now, we just perform the bidirectional conversion
        let _non_param = MemoryConverter::parametric_to_nonparametric(&self.planner.templates);
        // In a full implementation, we would update planner.templates from non-parametric memory
    }

    /// Get agent statistics
    pub fn stats(&self) -> MIAStats {
        MIAStats {
            memory_stats: self.memory_manager.stats(),
            planner_templates: self.planner.templates.len(),
            reflection_count: self.planner.reflection_history.len(),
        }
    }
}

/// MIA result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MIAResult {
    pub query: String,
    pub plan: SearchPlan,
    pub execution: PlanExecution,
    pub reflections: Vec<Reflection>,
    pub memory_stats: MemoryStats,
}

/// MIA statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MIAStats {
    pub memory_stats: MemoryStats,
    pub planner_templates: usize,
    pub reflection_count: usize,
}

// =============================================================================
// Utilities
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mia_basic() {
        let config = MIAConfig::default();
        let mut mia = MemoryIntelligenceAgent::new(config);

        let result = mia.run("What are the latest developments in solar cells?").await;
        assert!(!result.plan.steps.is_empty());
        assert_eq!(result.memory_stats.trajectory_count, 1);
    }

    #[test]
    fn test_memory_manager() {
        let mut manager = MemoryManager::new(10);

        let traj = CompressedTrajectory {
            id: "test1".to_string(),
            summary: "Test trajectory".to_string(),
            key_actions: vec!["search".to_string()],
            query_pattern: "general".to_string(),
            success_score: 0.8,
            token_cost: 100,
        };

        manager.store(traj);
        assert_eq!(manager.stats().trajectory_count, 1);

        let retrieved = manager.retrieve("general", 5);
        assert!(!retrieved.is_empty());
    }

    #[test]
    fn test_planner_query_classification() {
        let planner = Planner::new(PlannerConfig::default());

        assert_eq!(planner.classify_query("compare A and B"), "comparison");
        assert_eq!(planner.classify_query("latest research"), "update_search");
        assert_eq!(planner.classify_query("how does X work"), "explanatory");
    }

    #[test]
    fn test_bidirectional_conversion() {
        let templates = vec![PlanTemplate {
            query_type: "test".to_string(),
            template_steps: vec![],
            usage_count: 5,
            success_rate: 0.8,
        }];

        let non_param = MemoryConverter::parametric_to_nonparametric(&templates);
        assert_eq!(non_param.len(), 1);
        assert_eq!(non_param[0].success_score, 0.8);
    }
}
