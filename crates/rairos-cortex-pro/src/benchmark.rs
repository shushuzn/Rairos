//! Benchmark module for evaluating multi-agent research systems.
//!
//! Based on research from:
//! - BattleAgentBench (arXiv:2408.15971) - collaboration/competition evaluation
//! - MultiAgentBench (ACL 2025) - milestone KPIs, planning scores
//! - GAIA (arXiv:2311.12983) - real-world question answering
//! - AgentBench (arXiv:2308.03688) - multi-environment evaluation
//!
//! ## Architecture
//!
//! ```text
//! Benchmark Runner
//!     │
//!     ├──► Task Set (GAIA-style questions)
//!     ├──► Metrics Collection
//!     │       ├──► Task Success Rate
//!     │       ├──► Planning Quality
//!     │       ├──► Communication Score
//!     │       └──► Tool Use Accuracy
//!     └──► Report Generation
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Benchmark task result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Task ID
    pub task_id: String,
    /// Whether task succeeded
    pub success: bool,
    /// Score (0.0 - 1.0)
    pub score: f32,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Tokens used
    pub tokens_used: u64,
    /// Error message if failed
    pub error: Option<String>,
    /// Milestone achievements
    pub milestones: Vec<MilestoneResult>,
}

/// Result for a single milestone
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MilestoneResult {
    /// Milestone name
    pub name: String,
    /// Whether achieved
    pub achieved: bool,
    /// Score for this milestone
    pub score: f32,
}

/// Benchmark metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BenchmarkMetrics {
    /// Overall success rate (0.0 - 1.0)
    pub success_rate: f32,
    /// Average score
    pub avg_score: f32,
    /// Planning quality score
    pub planning_quality: f32,
    /// Communication effectiveness score
    pub communication_score: f32,
    /// Tool use accuracy
    pub tool_use_accuracy: f32,
    /// Average execution time (ms)
    pub avg_execution_time_ms: f64,
    /// Total tokens used
    pub total_tokens: u64,
    /// Tasks completed
    pub tasks_completed: u32,
    /// Tasks failed
    pub tasks_failed: u32,
    // =====================================================================
    // Multi-Agent Metrics (Based on MASEval, MAESTRO, Silo-Bench research)
    // =====================================================================
    /// Score variance (lower = more stable)
    pub score_variance: f32,
    /// Run-to-run variance (reproducibility measure)
    pub reproducibility_score: f32,
    /// Communication efficiency (tokens per successful task)
    pub communication_efficiency: f64,
    /// Tool call efficiency (successful calls / total calls)
    pub tool_call_efficiency: f32,
    /// Coordination cost (overhead from multi-agent coordination)
    pub coordination_cost: f32,
    /// Self-correction rate (how often agents self-correct)
    pub self_correction_rate: f32,
}

impl BenchmarkMetrics {
    /// Calculate metrics from task results with enhanced multi-agent metrics
    pub fn from_results_with_detailed(results: &[TaskResult], total_tool_calls: u32, successful_tool_calls: u32) -> Self {
        // Start with basic metrics
        let mut metrics = Self::from_results(results);

        // Calculate score variance
        let scores: Vec<f32> = results.iter().map(|r| r.score).collect();
        let mean = metrics.avg_score;
        let variance = if scores.len() > 1 {
            scores.iter().map(|s| (s - mean).powi(2)).sum::<f32>() / scores.len() as f32
        } else {
            0.0
        };

        // Reproducibility score (inverse of variance, normalized to 0-1)
        let reproducibility_score = (1.0 - variance.min(1.0)).max(0.0);

        // Communication efficiency (tokens per successful task)
        let successful_count = results.iter().filter(|r| r.success).count() as u64;
        let communication_efficiency = if successful_count > 0 {
            metrics.total_tokens as f64 / successful_count as f64
        } else {
            0.0
        };

        // Tool call efficiency
        let tool_call_efficiency = if total_tool_calls > 0 {
            successful_tool_calls as f32 / total_tool_calls as f32
        } else {
            0.8 // Default if no data
        };

        // Coordination cost (estimated as overhead from parallel execution)
        let coordination_cost = if results.len() > 1 {
            // Estimate based on execution time variance
            let times: Vec<f64> = results.iter().map(|r| r.execution_time_ms as f64).collect();
            let time_variance = if times.len() > 1 {
                let time_mean = times.iter().sum::<f64>() / times.len() as f64;
                times.iter().map(|t| (t - time_mean).powi(2)).sum::<f64>() / times.len() as f64
            } else {
                0.0
            };
            (time_variance / 1000000.0).min(1.0) as f32 // Normalize
        } else {
            0.0
        };

        // Self-correction rate (estimated from milestones with "retry" or "correct")
        let self_correction_rate = if !results.is_empty() {
            let corrections: usize = results.iter()
                .flat_map(|r| &r.milestones)
                .filter(|m| m.name.to_lowercase().contains("correct") || m.name.to_lowercase().contains("retry"))
                .count();
            corrections as f32 / results.len() as f32
        } else {
            0.0
        };

        // Update metrics
        metrics.score_variance = variance;
        metrics.reproducibility_score = reproducibility_score;
        metrics.communication_efficiency = communication_efficiency;
        metrics.tool_call_efficiency = tool_call_efficiency;
        metrics.coordination_cost = coordination_cost;
        metrics.self_correction_rate = self_correction_rate;

        metrics
    }
}

impl BenchmarkMetrics {
    /// Calculate metrics from task results
    pub fn from_results(results: &[TaskResult]) -> Self {
        if results.is_empty() {
            return Self::default();
        }

        let tasks_completed = results.iter().filter(|r| r.success).count() as u32;
        let tasks_failed = results.iter().filter(|r| !r.success).count() as u32;

        let success_rate = tasks_completed as f32 / results.len() as f32;
        let avg_score = results.iter().map(|r| r.score).sum::<f32>() / results.len() as f32;

        let avg_execution_time_ms = results.iter()
            .map(|r| r.execution_time_ms as f64)
            .sum::<f64>() / results.len() as f64;

        let total_tokens = results.iter().map(|r| r.tokens_used).sum();

        // Calculate planning quality from milestones
        let planning_quality = if results.iter().all(|r| !r.milestones.is_empty()) {
            let planning_milestones: Vec<_> = results.iter()
                .flat_map(|r| r.milestones.iter().filter(|m| m.name.contains("plan")))
                .collect();
            if !planning_milestones.is_empty() {
                planning_milestones.iter().map(|m| m.score).sum::<f32>() / planning_milestones.len() as f32
            } else {
                0.5
            }
        } else {
            0.5
        };

        // Communication score (placeholder - would need agent interaction data)
        let communication_score = 0.75;

        // Tool use accuracy (placeholder - would need tool call data)
        let tool_use_accuracy = 0.8;

        Self {
            success_rate,
            avg_score,
            planning_quality,
            communication_score,
            tool_use_accuracy,
            avg_execution_time_ms,
            total_tokens,
            tasks_completed,
            tasks_failed,
            // Multi-agent metrics - default values (use from_results_with_detailed for actual values)
            score_variance: 0.0,
            reproducibility_score: 1.0,
            communication_efficiency: 0.0,
            tool_call_efficiency: 0.8,
            coordination_cost: 0.0,
            self_correction_rate: 0.0,
        }
    }
}

/// Benchmark task definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkTask {
    /// Task ID
    pub id: String,
    /// Task description
    pub query: String,
    /// Expected answer or solution type
    pub expected: String,
    /// Difficulty level
    pub difficulty: DifficultyLevel,
    /// Required capabilities
    pub required_capabilities: Vec<Capability>,
    /// Ground truth answer
    pub ground_truth: Option<String>,
}

/// Difficulty levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DifficultyLevel {
    Easy,
    Medium,
    Hard,
}

/// Capabilities required for a task
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Capability {
    Reasoning,
    ToolUse,
    Planning,
    MultiStep,
    WebBrowse,
    CodeGeneration,
    Scientific,
    Math,
}

/// Benchmark report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkReport {
    /// Report generation timestamp
    pub generated_at: DateTime<Utc>,
    /// Benchmark version
    pub version: String,
    /// Tasks evaluated
    pub tasks_evaluated: usize,
    /// Overall metrics
    pub metrics: BenchmarkMetrics,
    /// Per-task results
    pub task_results: Vec<TaskResult>,
    /// Breakdown by capability
    pub capability_breakdown: HashMap<String, f32>,
    /// Recommendations
    pub recommendations: Vec<String>,
}

impl BenchmarkReport {
    /// Generate a report from task results
    pub fn generate(results: Vec<TaskResult>, tasks: &[BenchmarkTask]) -> Self {
        let metrics = BenchmarkMetrics::from_results(&results);

        // Calculate capability breakdown
        let mut capability_scores: HashMap<String, Vec<f32>> = HashMap::new();
        for (result, task) in results.iter().zip(tasks.iter()) {
            for cap in &task.required_capabilities {
                let cap_name = format!("{:?}", cap);
                capability_scores
                    .entry(cap_name)
                    .or_default()
                    .push(result.score);
            }
        }

        let capability_breakdown: HashMap<String, f32> = capability_scores
            .into_iter()
            .map(|(cap, scores)| {
                let avg = scores.iter().sum::<f32>() / scores.len() as f32;
                (cap, avg)
            })
            .collect();

        // Generate recommendations
        let mut recommendations = Vec::new();
        if metrics.success_rate < 0.5 {
            recommendations.push("Low success rate - consider improving agent planning capabilities".to_string());
        }
        if metrics.planning_quality < 0.6 {
            recommendations.push("Planning quality below threshold - review plan generation logic".to_string());
        }
        if metrics.avg_execution_time_ms > 60000.0 {
            recommendations.push("High execution time - consider optimizing tool selection".to_string());
        }
        if capability_breakdown.get("ToolUse").copied().unwrap_or(1.0) < 0.7 {
            recommendations.push("Tool use accuracy low - review tool integration".to_string());
        }

        Self {
            generated_at: Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            tasks_evaluated: results.len(),
            metrics,
            task_results: results,
            capability_breakdown,
            recommendations,
        }
    }
}

/// Standard benchmark tasks (GAIA-style)
pub fn gaia_benchmark_tasks() -> Vec<BenchmarkTask> {
    vec![
        BenchmarkTask {
            id: "gaia_001".to_string(),
            query: "Find thermoelectric materials with ZT > 1.5 and describe their crystal structure".to_string(),
            expected: "materials_list".to_string(),
            difficulty: DifficultyLevel::Medium,
            required_capabilities: vec![Capability::Scientific, Capability::ToolUse],
            ground_truth: None,
        },
        BenchmarkTask {
            id: "gaia_002".to_string(),
            query: "Compare the band gap of Bi2Te3 vs PbTe using computational methods".to_string(),
            expected: "comparison".to_string(),
            difficulty: DifficultyLevel::Medium,
            required_capabilities: vec![Capability::Scientific, Capability::Planning],
            ground_truth: None,
        },
        BenchmarkTask {
            id: "gaia_003".to_string(),
            query: "Design a research plan to discover new half-Heusler thermoelectrics".to_string(),
            expected: "research_plan".to_string(),
            difficulty: DifficultyLevel::Hard,
            required_capabilities: vec![Capability::Planning, Capability::MultiStep, Capability::Scientific],
            ground_truth: None,
        },
    ]
}

/// MultiAgentBench-style collaboration tasks
pub fn collaboration_tasks() -> Vec<BenchmarkTask> {
    vec![
        BenchmarkTask {
            id: "collab_001".to_string(),
            query: "Hypothesis generation + critic review for Mg3Sb2 thermoelectric material".to_string(),
            expected: "validated_hypothesis".to_string(),
            difficulty: DifficultyLevel::Medium,
            required_capabilities: vec![Capability::Planning, Capability::ToolUse],
            ground_truth: None,
        },
        BenchmarkTask {
            id: "collab_002".to_string(),
            query: "Multi-agent plan execution: search literature → extract data → analyze gaps".to_string(),
            expected: "gap_analysis".to_string(),
            difficulty: DifficultyLevel::Hard,
            required_capabilities: vec![Capability::MultiStep, Capability::ToolUse, Capability::Planning],
            ground_truth: None,
        },
    ]
}

/// Evaluate a single task
pub async fn evaluate_task<F, Fut>(
    task: &BenchmarkTask,
    executor: F,
) -> TaskResult
where
    F: FnOnce(&str) -> Fut,
    Fut: std::future::Future<Output = Result<String, String>>,
{
    let start = std::time::Instant::now();

    let result = executor(&task.query).await;

    let execution_time_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(output) => {
            // Simple scoring based on output presence and length
            let score = if output.len() > 100 { 0.8 } else { 0.5 };

            TaskResult {
                task_id: task.id.clone(),
                success: true,
                score,
                execution_time_ms,
                tokens_used: (output.len() as f64 * 1.3) as u64, // Rough estimate
                error: None,
                milestones: vec![
                    MilestoneResult {
                        name: "plan_created".to_string(),
                        achieved: true,
                        score: 0.8,
                    },
                ],
            }
        }
        Err(e) => TaskResult {
            task_id: task.id.clone(),
            success: false,
            score: 0.0,
            execution_time_ms,
            tokens_used: 0,
            error: Some(e),
            milestones: vec![],
        },
    }
}

/// Run a full benchmark
pub async fn run_benchmark<F, Fut>(
    tasks: &[BenchmarkTask],
    executor: F,
) -> BenchmarkReport
where
    F: Fn(&str) -> Fut + Clone,
    Fut: std::future::Future<Output = Result<String, String>>,
{
    let mut results = Vec::new();

    for task in tasks {
        let result = evaluate_task(task, &executor).await;
        results.push(result);
    }

    BenchmarkReport::generate(results, tasks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_evaluate_task() {
        let task = BenchmarkTask {
            id: "test_001".to_string(),
            query: "Find thermoelectric materials".to_string(),
            expected: "materials_list".to_string(),
            difficulty: DifficultyLevel::Easy,
            required_capabilities: vec![Capability::Scientific],
            ground_truth: None,
        };

        let result = evaluate_task(&task, |_| async {
            Ok("Bi2Te3, PbTe, Mg2Si are thermoelectric materials".to_string())
        }).await;

        assert!(result.success);
        assert!(result.score > 0.0);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_benchmark_metrics() {
        let results = vec![
            TaskResult {
                task_id: "t1".to_string(),
                success: true,
                score: 0.9,
                execution_time_ms: 1000,
                tokens_used: 500,
                error: None,
                milestones: vec![],
            },
            TaskResult {
                task_id: "t2".to_string(),
                success: true,
                score: 0.7,
                execution_time_ms: 2000,
                tokens_used: 800,
                error: None,
                milestones: vec![],
            },
            TaskResult {
                task_id: "t3".to_string(),
                success: false,
                score: 0.0,
                execution_time_ms: 500,
                tokens_used: 100,
                error: Some("Failed".to_string()),
                milestones: vec![],
            },
        ];

        let metrics = BenchmarkMetrics::from_results(&results);

        assert!((metrics.success_rate - 0.666).abs() < 0.01);
        assert!((metrics.avg_score - 0.533).abs() < 0.01);
        assert_eq!(metrics.tasks_completed, 2);
        assert_eq!(metrics.tasks_failed, 1);
    }

    #[tokio::test]
    async fn test_run_benchmark() {
        let tasks = gaia_benchmark_tasks();
        let executor = |q: &str| {
            let query = q.to_string();
            async move {
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                Ok(format!("Response to: {}", query))
            }
        };

        let report = run_benchmark(&tasks, executor).await;

        assert_eq!(report.tasks_evaluated, 3);
        assert!(report.metrics.avg_execution_time_ms > 0.0);
    }

    #[test]
    fn test_benchmark_report_generation() {
        let tasks = vec![
            BenchmarkTask {
                id: "t1".to_string(),
                query: "Test query".to_string(),
                expected: "test".to_string(),
                difficulty: DifficultyLevel::Easy,
                required_capabilities: vec![Capability::ToolUse],
                ground_truth: None,
            },
        ];

        let results = vec![
            TaskResult {
                task_id: "t1".to_string(),
                success: true,
                score: 0.8,
                execution_time_ms: 1000,
                tokens_used: 500,
                error: None,
                milestones: vec![],
            },
        ];

        let report = BenchmarkReport::generate(results, &tasks);

        assert_eq!(report.tasks_evaluated, 1);
        assert!(!report.capability_breakdown.is_empty());
        assert!(report.recommendations.len() <= 4); // Based on metrics
    }
}