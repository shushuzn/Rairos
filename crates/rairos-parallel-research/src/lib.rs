//! Parallel Deep Research Coordinator — multiple agents investigating different gap directions concurrently.
//!
//! Architecture
//! ────────────
//! Given a list of gap clusters from GapClusterer, this coordinator:
//!
//! 1. Groups agents by gap type / cluster
//! 2. Launches N concurrent agent threads (max_concurrency agents in parallel)
//! 3. Each agent runs DeepResearchAgent independently on its gap sub-direction
//! 4. Results are merged: gaps deduplicated, papers merged, insights combined
//!
//! This turns a sequential N-gap research run into a parallel O(1) wall-clock research pass.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{Barrier, Semaphore};
use uuid::Uuid;

// ─── Error Types ─────────────────────────────────────────────────────────────

#[derive(Error, Debug)]
pub enum ParallelResearchError {
    #[error("Agent execution failed: {0}")]
    AgentFailed(String),
    #[error("Timeout waiting for agent results")]
    Timeout,
    #[error("No valid gap clusters provided")]
    NoClusters,
}

// ─── Gap Representation ──────────────────────────────────────────────────────

/// A research gap — title, type, and novelty score for deduplication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchGap {
    pub title: String,
    pub gap_type: String,
    pub novelty_score: f64,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub sources: Vec<String>,
}

impl ResearchGap {
    pub fn gap_hash(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(format!("{}:{}", self.gap_type, self.title));
        let result = hasher.finalize();
        let hash_bytes = &result[..8];
        let hash_str: String = hash_bytes.iter().map(|b| format!("{:02x}", b)).collect();
        hash_str
    }
}

// ─── Insight Representation ───────────────────────────────────────────────────

/// A research insight extracted by an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default)]
    pub confidence: f64,
}

// ─── Agent Result ────────────────────────────────────────────────────────────

/// Result from a single parallel agent run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    pub agent_id: String,
    pub cluster_id: String,
    #[serde(default)]
    pub gaps: Vec<ResearchGap>,
    #[serde(default)]
    pub papers_analyzed: usize,
    #[serde(default)]
    pub iterations: usize,
    #[serde(default)]
    pub insights: Vec<Insight>,
    #[serde(default)]
    pub error: Option<String>,
    pub duration_seconds: f64,
}

impl AgentResult {
    pub fn error_result(
        agent_id: String,
        cluster_id: String,
        err: String,
        start: DateTime<Utc>,
    ) -> Self {
        AgentResult {
            agent_id,
            cluster_id,
            gaps: vec![],
            papers_analyzed: 0,
            iterations: 0,
            insights: vec![],
            error: Some(err),
            duration_seconds: (Utc::now() - start).num_milliseconds() as f64 / 1000.0,
        }
    }
}

// ─── Parallel Research Result ─────────────────────────────────────────────────

/// Combined result from all parallel agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelResearchResult {
    pub total_gaps: usize,
    pub unique_gaps: usize,
    pub total_papers_analyzed: usize,
    pub total_iterations: usize,
    #[serde(default)]
    pub agent_results: Vec<AgentResult>,
    #[serde(default)]
    pub merged_insights: Vec<Insight>,
    pub duration_seconds: f64,
}

impl ParallelResearchResult {
    pub fn empty() -> Self {
        ParallelResearchResult {
            total_gaps: 0,
            unique_gaps: 0,
            total_papers_analyzed: 0,
            total_iterations: 0,
            agent_results: vec![],
            merged_insights: vec![],
            duration_seconds: 0.0,
        }
    }
}

// ─── Gap Cluster ──────────────────────────────────────────────────────────────

/// A gap cluster from GapClusterer, passed to the coordinator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapCluster {
    pub cluster_id: String,
    #[serde(default)]
    pub gaps: Vec<ResearchGap>,
    pub gap_type: String,
    #[serde(default)]
    pub keywords: Vec<String>,
}

// ─── Merge Logic ──────────────────────────────────────────────────────────────

/// Compute a hash key for a gap to deduplicate across agents.
fn gap_hash(gap: &ResearchGap) -> String {
    gap.gap_hash()
}

/// Deduplicate gaps across all agent results.
///
/// Uses title+gap_type hash to identify duplicates.
/// Keeps the gap with highest novelty_score.
fn merge_gaps(all_results: &[AgentResult]) -> Vec<ResearchGap> {
    let mut seen: HashMap<String, &ResearchGap> = HashMap::new();
    let mut seen_novelty: HashMap<String, f64> = HashMap::new();

    for result in all_results {
        for gap in &result.gaps {
            let key = gap_hash(gap);
            let novelty = gap.novelty_score;
            if !seen.contains_key(&key) || novelty > seen_novelty[&key] {
                seen.insert(key.clone(), gap);
                seen_novelty.insert(key, novelty);
            }
        }
    }

    seen.into_values().cloned().collect()
}

/// Collect all insights from all agents, deduplicated by title.
fn merge_insights(all_results: &[AgentResult]) -> Vec<Insight> {
    let mut seen_titles: HashSet<String> = HashSet::new();
    let mut merged: Vec<Insight> = Vec::new();

    for result in all_results {
        for insight in &result.insights {
            let title = &insight.title;
            if title.is_empty() {
                merged.push(insight.clone());
            } else if !seen_titles.contains(title) {
                seen_titles.insert(title.clone());
                merged.push(insight.clone());
            }
        }
    }

    merged
}

// ─── Deep Research Agent Trait ────────────────────────────────────────────────

/// Trait for a deep research agent that can be run per gap cluster.
/// This allows integration with different backends (local, LLM, etc.).
pub trait DeepResearchAgent: Send + Sync {
    /// Run deep research on a specific topic/sub-topic.
    /// Returns a map with keys: "gaps", "papers_analyzed", "iterations".
    fn run_deep_research(
        &self,
        topic: &str,
        new_papers: Vec<serde_json::Value>,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, ParallelResearchError>> + Send;
}

// ─── Orchestrator Trait ───────────────────────────────────────────────────────

/// A simple orchestrator that can run deep research.
/// Implementors provide the actual research logic.
pub trait Orchestrator: Send + Sync {
    fn run_deep_research(
        &self,
        topic: &str,
        new_papers: Vec<serde_json::Value>,
    ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, ParallelResearchError>> + Send + '_>>;
}

// ─── Tokio-based Agent Runner ─────────────────────────────────────────────────

async fn run_single_agent_tokio(
    cluster_id: String,
    gap_sub_topic: String,
    initial_papers: Vec<serde_json::Value>,
    _max_iterations: usize,
    agent_id: String,
    barrier: Option<Arc<Barrier>>,
    orchestrator: Arc<dyn Orchestrator>,
) -> AgentResult {
    use tokio::time::{timeout, Duration};

    let start = Utc::now();
    if let Some(ref b) = barrier {
        let _ = b.wait().await;
    }

    match timeout(
        Duration::from_secs(300),
        orchestrator.run_deep_research(&gap_sub_topic, initial_papers),
    )
    .await
    {
        Ok(Ok(agent_result)) => {
            let gaps: Vec<ResearchGap> = agent_result
                .get("gaps")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();

            let papers_analyzed: usize = agent_result
                .get("papers_analyzed")
                .and_then(|v| v.as_u64().map(|n| n as usize))
                .unwrap_or(0);

            let iterations: usize = agent_result
                .get("iterations")
                .and_then(|v| v.as_u64().map(|n| n as usize))
                .unwrap_or(0);

            // Collect insights from gaps as dict-based insights
            let insights: Vec<Insight> = gaps
                .iter()
                .map(|g| Insight {
                    title: g.title.clone(),
                    summary: String::new(),
                    sources: vec![],
                    confidence: 0.5,
                })
                .collect();

            AgentResult {
                agent_id,
                cluster_id,
                gaps,
                papers_analyzed,
                iterations,
                insights,
                error: None,
                duration_seconds: (Utc::now() - start).num_milliseconds() as f64 / 1000.0,
            }
        }
        Ok(Err(e)) => {
            tracing::warn!("Parallel agent {} failed: {}", agent_id, e);
            AgentResult::error_result(agent_id, cluster_id, e.to_string(), start)
        }
        Err(_) => {
            tracing::warn!("Parallel agent {} timed out", agent_id);
            AgentResult::error_result(
                agent_id,
                cluster_id,
                "Agent timed out after 300 seconds".to_string(),
                start,
            )
        }
    }
}

// ─── Agent Task ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct AgentTask {
    cluster_id: String,
    sub_topic: String,
    gaps: Vec<ResearchGap>,
    gap_type: String,
    initial_papers: Vec<serde_json::Value>,
}

// ─── Main Coordinator ─────────────────────────────────────────────────────────

/// Coordinate multiple DeepResearchAgent instances for concurrent gap investigation.
///
/// Usage:
/// ```ignore
/// let coordinator = ParallelResearchCoordinator::new(
///     max_concurrency=3,
///     max_iterations_per_agent=2,
/// );
/// let result = coordinator.run(topic, gap_clusters, existing_papers).await;
/// ```
#[derive(Clone)]
pub struct ParallelResearchCoordinator {
    max_concurrency: usize,
    max_iterations_per_agent: usize,
    agent_timeout_seconds: u64,
}

impl Default for ParallelResearchCoordinator {
    fn default() -> Self {
        Self::new(3, 2, 300)
    }
}

impl ParallelResearchCoordinator {
    /// Create a new coordinator.
    ///
    /// - `max_concurrency`: max parallel agents (default 3 — avoid rate limiting)
    /// - `max_iterations_per_agent`: iterations per agent (default 2)
    /// - `agent_timeout_seconds`: kill agent if it exceeds this timeout
    pub fn new(max_concurrency: usize, max_iterations_per_agent: usize, agent_timeout_seconds: u64) -> Self {
        Self {
            max_concurrency,
            max_iterations_per_agent,
            agent_timeout_seconds,
        }
    }

    /// Run parallel deep research across multiple gap clusters.
    ///
    /// - `topic`: overall research topic
    /// - `gap_clusters`: list of GapCluster dicts
    /// - `existing_papers`: papers already collected to pass to agents
    pub async fn run(
        &self,
        topic: &str,
        gap_clusters: Vec<GapCluster>,
        existing_papers: Option<Vec<serde_json::Value>>,
        orchestrator: Arc<dyn Orchestrator>,
    ) -> ParallelResearchResult {
        if gap_clusters.is_empty() {
            return ParallelResearchResult::empty();
        }

        let existing_papers = existing_papers.unwrap_or_default();
        let start = Utc::now();

        // Build sub-topics for each cluster
        let agent_tasks: Vec<AgentTask> = gap_clusters
            .into_iter()
            .filter_map(|cluster| {
                let cluster_id = if cluster.cluster_id.is_empty() {
                    Uuid::new_v4().to_string()[..8].to_string()
                } else {
                    cluster.cluster_id
                };

                if cluster.gaps.is_empty() {
                    return None;
                }

                let gap_titles: Vec<String> = cluster
                    .gaps
                    .iter()
                    .map(|g| g.title.clone())
                    .collect();

                let sub_topic = if cluster.keywords.is_empty() {
                    format!(
                        "{} — {}: {}",
                        topic,
                        cluster.gap_type,
                        gap_titles.first().map(|t| t[..80.min(t.len())].to_string()).unwrap_or_default()
                    )
                } else {
                    format!(
                        "{} — {}: {}",
                        topic,
                        cluster.gap_type,
                        cluster.keywords.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
                    )
                };

                Some(AgentTask {
                    cluster_id,
                    sub_topic,
                    gaps: cluster.gaps,
                    gap_type: cluster.gap_type,
                    initial_papers: existing_papers.iter().take(5).cloned().collect(),
                })
            })
            .collect();

        if agent_tasks.is_empty() {
            return ParallelResearchResult::empty();
        }

        // Run agents in parallel with controlled concurrency
        let barrier: Arc<Barrier> = Arc::new(Barrier::new(agent_tasks.len()));
        let semaphore = Arc::new(Semaphore::new(self.max_concurrency));
        let max_iters = self.max_iterations_per_agent;

        let handles: Vec<_> = agent_tasks
            .into_iter()
            .enumerate()
            .map(|(i, task)| {
                let barrier = Arc::clone(&barrier);
                let semaphore = Arc::clone(&semaphore);
                let orchestrator = Arc::clone(&orchestrator);
                let agent_id = format!("agent_{}", i);

                tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.expect("semaphore closed");
                    run_single_agent_tokio(
                        task.cluster_id,
                        task.sub_topic,
                        task.initial_papers,
                        max_iters,
                        agent_id,
                        Some(barrier),
                        orchestrator,
                    )
                    .await
                })
            })
            .collect();

        let mut results: Vec<AgentResult> = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => {
                    tracing::warn!("Agent future failed: {}", e);
                }
            }
        }

        // Merge results
        let total_gaps = results.iter().map(|r| r.gaps.len()).sum();
        let unique_gaps_list = merge_gaps(&results);
        let merged_insights = merge_insights(&results);

        ParallelResearchResult {
            total_gaps,
            unique_gaps: unique_gaps_list.len(),
            total_papers_analyzed: results.iter().map(|r| r.papers_analyzed).sum(),
            total_iterations: results.iter().map(|r| r.iterations).sum(),
            agent_results: results,
            merged_insights,
            duration_seconds: (Utc::now() - start).num_milliseconds() as f64 / 1000.0,
        }
    }

    /// Convenience: run parallel research on GapClusterer clusters directly.
    ///
    /// - `topic`: overall research topic
    /// - `clusters`: list of GapCluster (cluster_id, gaps, gap_type, keywords)
    pub async fn run_on_gap_clusters(
        &self,
        topic: &str,
        clusters: Vec<GapCluster>,
        existing_papers: Option<Vec<serde_json::Value>>,
        orchestrator: Arc<dyn Orchestrator>,
    ) -> ParallelResearchResult {
        self.run(topic, clusters, existing_papers, orchestrator).await
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    struct MockOrchestrator;

    impl Orchestrator for MockOrchestrator {
        fn run_deep_research(
            &self,
            topic: &str,
            _new_papers: Vec<serde_json::Value>,
        ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, ParallelResearchError>> + Send + '_>> {
            let topic = topic.to_string();
            Box::pin(async move {
                Ok(serde_json::json!({
                    "gaps": [
                        {"title": format!("Mock gap for {}", topic), "gap_type": "method_limitation", "novelty_score": 0.8}
                    ],
                    "papers_analyzed": 10,
                    "iterations": 2
                }))
            })
        }
    }

    #[tokio::test]
    async fn test_coordinator_empty_clusters() {
        let coordinator = ParallelResearchCoordinator::default();
        let orchestrator: Arc<dyn Orchestrator> = Arc::new(MockOrchestrator);
        let result = coordinator.run("test topic", vec![], None, orchestrator).await;
        assert_eq!(result.total_gaps, 0);
        assert_eq!(result.unique_gaps, 0);
    }

    #[tokio::test]
    async fn test_coordinator_single_cluster() {
        let coordinator = ParallelResearchCoordinator::default();
        let orchestrator: Arc<dyn Orchestrator> = Arc::new(MockOrchestrator);
        let clusters = vec![GapCluster {
            cluster_id: "c0".to_string(),
            gaps: vec![ResearchGap {
                title: "Test gap".to_string(),
                gap_type: "method_limitation".to_string(),
                novelty_score: 0.8,
                description: "".to_string(),
                sources: vec![],
            }],
            gap_type: "method_limitation".to_string(),
            keywords: vec!["transformer".to_string()],
        }];
        let result = coordinator.run("transformer efficiency", clusters, None, orchestrator).await;
        assert_eq!(result.total_gaps, 1);
        assert_eq!(result.unique_gaps, 1);
        assert_eq!(result.total_papers_analyzed, 10);
    }

    #[tokio::test]
    async fn test_gap_deduplication() {
        let results = vec![
            AgentResult {
                agent_id: "a0".to_string(),
                cluster_id: "c0".to_string(),
                gaps: vec![
                    ResearchGap {
                        title: "Same Gap".to_string(),
                        gap_type: "method".to_string(),
                        novelty_score: 0.5,
                        description: "".to_string(),
                        sources: vec![],
                    },
                ],
                papers_analyzed: 5,
                iterations: 1,
                insights: vec![],
                error: None,
                duration_seconds: 1.0,
            },
            AgentResult {
                agent_id: "a1".to_string(),
                cluster_id: "c1".to_string(),
                gaps: vec![
                    ResearchGap {
                        title: "Same Gap".to_string(),
                        gap_type: "method".to_string(),
                        novelty_score: 0.9,
                        description: "".to_string(),
                        sources: vec![],
                    },
                ],
                papers_analyzed: 7,
                iterations: 2,
                insights: vec![],
                error: None,
                duration_seconds: 1.5,
            },
        ];

        let merged = merge_gaps(&results);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].novelty_score, 0.9); // Higher score kept
    }

    #[tokio::test]
    async fn test_insight_deduplication() {
        let results = vec![
            AgentResult {
                agent_id: "a0".to_string(),
                cluster_id: "c0".to_string(),
                gaps: vec![],
                papers_analyzed: 5,
                iterations: 1,
                insights: vec![
                    Insight {
                        title: "Duplicate Insight".to_string(),
                        summary: "".to_string(),
                        sources: vec![],
                        confidence: 0.5,
                    },
                ],
                error: None,
                duration_seconds: 1.0,
            },
            AgentResult {
                agent_id: "a1".to_string(),
                cluster_id: "c1".to_string(),
                gaps: vec![],
                papers_analyzed: 7,
                iterations: 2,
                insights: vec![
                    Insight {
                        title: "Duplicate Insight".to_string(),
                        summary: "".to_string(),
                        sources: vec![],
                        confidence: 0.8,
                    },
                    Insight {
                        title: "Unique Insight".to_string(),
                        summary: "".to_string(),
                        sources: vec![],
                        confidence: 0.6,
                    },
                ],
                error: None,
                duration_seconds: 1.5,
            },
        ];

        let merged = merge_insights(&results);
        assert_eq!(merged.len(), 2); // Duplicate removed
        let titles: Vec<_> = merged.iter().map(|i| i.title.clone()).collect();
        assert!(titles.contains(&"Unique Insight".to_string()));
        assert!(titles.contains(&"Duplicate Insight".to_string()));
    }

    #[tokio::test]
    async fn test_run_on_gap_clusters() {
        let coordinator = ParallelResearchCoordinator::default();
        let orchestrator: Arc<dyn Orchestrator> = Arc::new(MockOrchestrator);
        let clusters = vec![GapCluster {
            cluster_id: "c0".to_string(),
            gaps: vec![],
            gap_type: "unknown".to_string(),
            keywords: vec![],
        }];
        let result = coordinator.run_on_gap_clusters("topic", clusters, None, orchestrator).await;
        assert_eq!(result.total_gaps, 0);
    }
}
