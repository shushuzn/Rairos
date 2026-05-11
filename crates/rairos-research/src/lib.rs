//! Rairos Research — Deep research orchestration and gap detection
//!
//! Coordinates the full research pipeline: fetch → analyze → detect gaps → evolve.
//! Replaces: research_loop/core.py, llm/gap_detector.py

use rairos_core::{Database, Paper, ResearchGap};
use rairos_llm::{LlmClient, Message, CostTracker, GapDetector};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use uuid::Uuid;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum ResearchError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("LLM error: {0}")]
    Llm(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("No papers found for research query: {0}")]
    NoPapers(String),

    #[error("Rate limited: retry after {0}s")]
    RateLimited(u64),

    #[error("Invalid state: {0}")]
    InvalidState(String),
}

// ============================================================================
// Research Query
// ============================================================================

/// A research query to execute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchQuery {
    pub id: String,
    pub query: String,
    pub categories: Vec<String>,
    pub max_papers: usize,
    pub min_relevance: f32,
    pub include_citations: bool,
}

impl ResearchQuery {
    pub fn new(query: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            query: query.to_string(),
            categories: Vec::new(),
            max_papers: 50,
            min_relevance: 0.5,
            include_citations: true,
        }
    }

    pub fn with_categories(mut self, cats: Vec<String>) -> Self {
        self.categories = cats;
        self
    }

    pub fn with_max_papers(mut self, n: usize) -> Self {
        self.max_papers = n;
        self
    }
}

// ============================================================================
// Research Result
// ============================================================================

/// Output of a research session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchResult {
    pub query_id: String,
    pub papers_found: usize,
    pub gaps: Vec<ResearchGap>,
    pub citations_analyzed: usize,
    pub evolution_suggestions: Vec<String>,
    pub cost_usd: f64,
    pub duration_secs: f64,
}

impl ResearchResult {
    pub fn summary(&self) -> String {
        format!(
            "Research completed: {} papers found, {} gaps detected, {:.4} cost, {:.1}s",
            self.papers_found,
            self.gaps.len(),
            self.cost_usd,
            self.duration_secs
        )
    }
}

// ============================================================================
// Research Orchestrator
// ============================================================================

/// The main research orchestrator
pub struct ResearchOrchestrator {
    db: Arc<Database>,
    llm: Arc<dyn LlmClient>,
    cost_tracker: Arc<RwLock<CostTracker>>,
}

impl ResearchOrchestrator {
    pub fn new(db: Arc<Database>, llm: Arc<dyn LlmClient>) -> Self {
        Self {
            db,
            llm,
            cost_tracker: Arc::new(RwLock::new(CostTracker::new())),
        }
    }

    /// Execute a full research query
    pub async fn research(&self, query: &ResearchQuery) -> Result<ResearchResult, ResearchError> {
        let start = std::time::Instant::now();

        // Step 1: Fetch papers from database matching query
        let papers = self.find_relevant_papers(query).await?;

        if papers.is_empty() {
            return Err(ResearchError::NoPapers(query.query.clone()));
        }

        // Step 2: Analyze citation chains
        let citations_analyzed = if query.include_citations {
            self.analyze_citations(&papers).await
        } else {
            0
        };

        // Step 3: Detect research gaps
        let gaps = self.detect_gaps(&papers, &query.categories).await?;

        // Step 4: Generate evolution suggestions via LLM
        let evolution_suggestions = self.generate_suggestions(&papers, &gaps).await?;

        // Step 5: Save gaps to database
        for gap in &gaps {
            if let Err(e) = self.db.insert_gap(gap) {
                tracing::warn!("Failed to save gap: {}", e);
            }
        }

        let duration_secs = start.elapsed().as_secs_f64();
        let cost_usd = {
            let tracker = self.cost_tracker.read().await;
            tracker.total_cost_usd
        };

        Ok(ResearchResult {
            query_id: query.id.clone(),
            papers_found: papers.len(),
            gaps,
            citations_analyzed,
            evolution_suggestions,
            cost_usd,
            duration_secs,
        })
    }

    async fn find_relevant_papers(&self, query: &ResearchQuery) -> Result<Vec<Paper>, ResearchError> {
        // Get papers from database
        let all_papers = self.db.list_papers(None, query.max_papers, 0)
            .map_err(|e| ResearchError::Database(e.to_string()))?;

        Ok(all_papers)
    }

    async fn analyze_citations(&self, papers: &[Paper]) -> usize {
        papers.len()
    }

    async fn detect_gaps(&self, papers: &[Paper], categories: &[String]) -> Result<Vec<ResearchGap>, ResearchError> {
        // Use keyword-based gap detection
        let keywords: Vec<&str> = categories.iter().map(|s| s.as_str()).collect();
        let gap_descriptions = GapDetector::detect_gaps(papers, &keywords);

        // Also find under-explored categories
        let under_category = GapDetector::find_underexplored_areas(papers, 3);

        let mut gaps = Vec::new();

        for desc in gap_descriptions {
            gaps.push(ResearchGap::new(
                "keyword_gap",
                &desc,
                "medium",
            ));
        }

        for cat in under_category {
            gaps.push(ResearchGap::new(
                "category_gap",
                &format!("Under-explored category: {}", cat),
                "low",
            ));
        }

        Ok(gaps)
    }

    async fn generate_suggestions(&self, papers: &[Paper], gaps: &[ResearchGap]) -> Result<Vec<String>, ResearchError> {
        if gaps.is_empty() {
            return Ok(vec![]);
        }

        // Build context from papers
        let context = papers.iter()
            .take(5)
            .map(|p| format!("- {} ({})", p.title, p.arxiv_id.as_deref().unwrap_or("")))
            .collect::<Vec<_>>()
            .join("\n");

        let gaps_text = gaps.iter()
            .take(5)
            .map(|g| format!("- {} [{}]", g.description, g.severity))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "You are a research assistant. Based on these papers:\n{}\n\nAnd these research gaps:\n{}\n\nSuggest 3 novel research directions that could address these gaps. Be specific and innovative.",
            context,
            gaps_text
        );

        let messages = vec![
            Message { role: "system".to_string(), content: "You are a helpful research assistant.".to_string() },
            Message { role: "user".to_string(), content: prompt },
        ];

        let response = self.llm.complete(
            messages,
            "gpt-4o",
            0.7,
            500,
        ).await.map_err(|e| ResearchError::Llm(e.to_string()))?;

        // Record cost
        {
            let mut tracker = self.cost_tracker.write().await;
            tracker.record(&response.usage, response.model.as_str(), self.llm.provider_name());
        }

        // Parse response into suggestions
        let suggestions: Vec<String> = response.content
            .lines()
            .filter(|l| l.trim().starts_with('-') || l.trim().starts_with('1') || l.trim().starts_with('2') || l.trim().starts_with('3'))
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();

        Ok(suggestions)
    }

    /// Get cost summary
    pub async fn cost_summary(&self) -> String {
        let tracker = self.cost_tracker.read().await;
        tracker.summary()
    }

    /// Reset cost tracker
    pub async fn reset_costs(&self) {
        let mut tracker = self.cost_tracker.write().await;
        tracker.reset();
    }
}

// ============================================================================
// Paper Ranker
// ============================================================================

/// Ranks papers by relevance for a given query
pub struct PaperRanker;

impl PaperRanker {
    /// Simple TF-IDF-like ranking
    pub fn rank(papers: &[Paper], query: &str) -> Vec<(String, f32)> {
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        let mut scores: Vec<(String, f32)> = Vec::new();
        for p in papers {
            let title_lower = p.title.to_lowercase();
            let abstract_lower = p.abstract_text.to_lowercase();
            let mut score = 0.0;
            for term in &query_terms {
                let title_count = title_lower.matches(term).count() as f32;
                let abstract_count = abstract_lower.matches(term).count() as f32;
                score += title_count * 2.0 + abstract_count;
            }
            scores.push((p.id.clone(), score));
        }

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores
    }

    /// Filter papers by minimum relevance score
    pub fn filter_by_threshold(papers: &[Paper], query: &str, threshold: f32) -> Vec<Paper> {
        let scores = Self::rank(papers, query);
        scores.into_iter()
            .filter(|(_, score)| *score >= threshold)
            .filter_map(|(id, _)| papers.iter().find(|p| p.id == id))
            .cloned()
            .collect()
    }
}

// ============================================================================
// Claim Graph (from Python's research_loop/claim_graph.py)
// ============================================================================

/// A node in the claim graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimNode {
    pub id: String,
    pub paper_id: String,
    pub claim: String,
    pub supporting_papers: Vec<String>,
    pub contradicting_papers: Vec<String>,
    pub confidence: f32,
}

impl ClaimNode {
    pub fn new(paper_id: &str, claim: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            paper_id: paper_id.to_string(),
            claim: claim.to_string(),
            supporting_papers: Vec::new(),
            contradicting_papers: Vec::new(),
            confidence: 0.5,
        }
    }
}

/// A claim graph connecting papers via their claims
#[derive(Debug, Default)]
pub struct ClaimGraph {
    nodes: Vec<ClaimNode>,
}

impl ClaimGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_claim(&mut self, node: ClaimNode) {
        self.nodes.push(node);
    }

    pub fn nodes(&self) -> &[ClaimNode] {
        &self.nodes
    }

    pub fn find_claims_about(&self, keyword: &str) -> Vec<&ClaimNode> {
        self.nodes.iter()
            .filter(|n| n.claim.to_lowercase().contains(&keyword.to_lowercase()))
            .collect()
    }

    pub fn find_contradictions(&self, claim_id: &str) -> Vec<&ClaimNode> {
        let claim = self.nodes.iter().find(|n| n.id == claim_id);
        match claim {
            Some(c) => self.nodes.iter()
                .filter(|n| n.id != claim_id && c.contradicting_papers.contains(&n.paper_id))
                .collect(),
            None => Vec::new(),
        }
    }
}

// ============================================================================
// Benchmark Runner
// ============================================================================

/// Runs benchmarks on paper datasets
pub struct BenchmarkRunner;

impl BenchmarkRunner {
    /// Run a citation count benchmark
    pub fn citation_benchmark(papers: &[Paper]) -> CitationBenchmarkResult {
        let total_citations: usize = papers.iter()
            .map(|p| p.metadata.cited_by)
            .sum();

        let avg_citations = if papers.is_empty() {
            0.0
        } else {
            total_citations as f32 / papers.len() as f32
        };

        let most_cited = papers.iter()
            .max_by_key(|p| p.metadata.cited_by)
            .map(|p| (p.title.clone(), p.metadata.cited_by as u32));

        CitationBenchmarkResult {
            total_papers: papers.len(),
            total_citations,
            avg_citations,
            most_cited,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CitationBenchmarkResult {
    pub total_papers: usize,
    pub total_citations: usize,
    pub avg_citations: f32,
    pub most_cited: Option<(String, u32)>,
}

// ============================================================================
// Deep Research Agent Types
// ============================================================================

const ALL_GAP_TYPES: &[&str] = &[
    "capability", "improvement", "contradiction",
    "assumption", "extension", "baseline_gap",
    "evaluation_gap", "reproducibility_gap", "embodied_planning",
    "rl_pretraining", "scaling_laws", "reasoning",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    Planner,
    Searcher,
    Analyzer,
    Reflector,
}

impl std::fmt::Display for AgentRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentRole::Planner => write!(f, "planner"),
            AgentRole::Searcher => write!(f, "searcher"),
            AgentRole::Analyzer => write!(f, "analyzer"),
            AgentRole::Reflector => write!(f, "reflector"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentThought {
    pub iteration: i32,
    pub role: AgentRole,
    pub content: String,
    pub timestamp: f64,
}

impl AgentThought {
    pub fn new(iteration: i32, role: AgentRole, content: &str) -> Self {
        Self {
            iteration,
            role,
            content: content.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis() as f64 / 1000.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperSnapshot {
    pub paper_id: String,
    pub arxiv_id: Option<String>,
    pub title: String,
    pub abstract_text: String,
    pub published: String,
    pub citations: Vec<String>,
}

impl PaperSnapshot {
    pub fn from_paper(paper: &Paper) -> Self {
        Self {
            paper_id: paper.id.clone(),
            arxiv_id: paper.arxiv_id.clone(),
            title: paper.title.clone(),
            abstract_text: paper.abstract_text.clone(),
            published: paper.published.to_rfc3339(),
            citations: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapSnapshot {
    pub gap_id: String,
    pub gap_type: String,
    pub title: String,
    pub description: String,
    pub severity: String,
    pub novelty_score: f64,
    pub related_paper_ids: Vec<String>,
}

impl GapSnapshot {
    pub fn from_gap(gap: &ResearchGap) -> Self {
        Self {
            gap_id: gap.id.clone(),
            gap_type: gap.category.clone(),
            title: gap.description.chars().take(100).collect(),
            description: gap.description.clone(),
            severity: gap.severity.clone(),
            novelty_score: 0.5,
            related_paper_ids: gap.paper_ids.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchStatus {
    Completed,
    Paused,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepResearchResult {
    pub session_id: String,
    pub query: String,
    pub iterations: i32,
    pub papers: Vec<PaperSnapshot>,
    pub gaps: Vec<GapSnapshot>,
    pub thoughts: Vec<AgentThought>,
    pub report: String,
    pub duration_seconds: f64,
    pub status: ResearchStatus,
}

impl DeepResearchResult {
    pub fn summary(&self) -> String {
        format!(
            "Deep research '{}': {} iterations, {} papers, {} gaps, {:.1}s, {}",
            self.query,
            self.iterations,
            self.papers.len(),
            self.gaps.len(),
            self.duration_seconds,
            match self.status {
                ResearchStatus::Completed => "completed",
                ResearchStatus::Paused => "paused",
                ResearchStatus::Failed => "failed",
            }
        )
    }
}

// ============================================================================
// Adaptive Query Strategy
// ============================================================================

#[derive(Debug, Clone)]
pub struct AdaptiveQueryStrategy {
    topic: String,
    query_gap_types: std::collections::HashMap<String, Vec<String>>,
    gap_type_counts: std::collections::HashMap<String, usize>,
    total_gaps: usize,
}

impl AdaptiveQueryStrategy {
    pub fn new(topic: &str) -> Self {
        Self {
            topic: topic.to_string(),
            query_gap_types: std::collections::HashMap::new(),
            gap_type_counts: std::collections::HashMap::new(),
            total_gaps: 0,
        }
    }

    pub fn record_search_result(&mut self, query: &str, gaps: &[ResearchGap]) {
        if gaps.is_empty() {
            return;
        }

        let mut found_types: std::collections::HashSet<String> = std::collections::HashSet::new();
        for g in gaps {
            let gt = g.category.clone();
            *self.gap_type_counts.entry(gt.clone()).or_insert(0) += 1;
            found_types.insert(gt);
            self.total_gaps += 1;
        }
        self.query_gap_types.insert(query.to_string(), found_types.into_iter().collect());
    }

    pub fn gap_type_coverage(&self) -> std::collections::HashMap<String, f64> {
        if self.total_gaps == 0 {
            return ALL_GAP_TYPES.iter()
                .map(|&gt| (gt.to_string(), 0.0))
                .collect();
        }
        self.gap_type_counts.iter()
            .map(|(gt, &count)| {
                (gt.clone(), count as f64 / self.total_gaps as f64)
            })
            .collect()
    }

    pub fn under_represented_types(&self, threshold: f64) -> Vec<String> {
        let coverage = self.gap_type_coverage();
        coverage.iter()
            .filter(|(_, &ratio)| ratio > 0.0 && ratio < threshold)
            .map(|(gt, _)| gt.clone())
            .collect()
    }

    pub fn most_productive_queries(&self, top_k: usize) -> Vec<String> {
        let mut scored: Vec<(String, usize)> = self.query_gap_types.iter()
            .map(|(q, types)| (q.clone(), types.len()))
            .collect();
        scored.sort_by_key(|b| std::cmp::Reverse(b.1));
        scored.into_iter()
            .take(top_k)
            .map(|(q, _)| q)
            .collect()
    }

    pub fn build_adaptive_query(
        &self,
        iteration: i32,
        latest_gap_title: &str,
        latest_gap_type: &str,
        _gene_pool_hint: &str,
        confidence: f64,
    ) -> String {
        let under_rep = self.under_represented_types(0.15);

        if iteration == 0 {
            return self.topic.clone();
        }

        if !under_rep.is_empty() {
            let target = &under_rep[0];
            let productive = self.most_productive_queries(1);
            let base = productive.first().map(|s| s.as_str()).unwrap_or(&self.topic);
            return format!("{} {}", base, target);
        }

        if confidence >= 0.4 && !_gene_pool_hint.is_empty() {
            if !latest_gap_title.is_empty() {
                return format!("{} {}", _gene_pool_hint, latest_gap_title);
            }
            return _gene_pool_hint.to_string();
        }

        if latest_gap_type == "Contradiction" {
            return format!("{} {} disagreement", self.topic, latest_gap_title);
        } else if ["improvement", "capability", "extension", ""].contains(&latest_gap_type) {
            return format!("{} {} improvement", self.topic, latest_gap_title);
        } else if !latest_gap_title.is_empty() {
            return format!("{} {}", self.topic, latest_gap_title);
        }

        self.topic.clone()
    }

    pub fn query_similarity(&self, q1: &str, q2: &str) -> f64 {
        let words1: std::collections::HashSet<String> = q1.to_lowercase().split_whitespace().map(|s| s.to_string()).collect();
        let words2: std::collections::HashSet<String> = q2.to_lowercase().split_whitespace().map(|s| s.to_string()).collect();
        if words1.is_empty() || words2.is_empty() {
            return 0.0;
        }
        let intersection: std::collections::HashSet<_> = words1.intersection(&words2).collect();
        let union: std::collections::HashSet<_> = words1.union(&words2).collect();
        intersection.len() as f64 / union.len() as f64
    }
}

// ============================================================================
// Deep Research Agent
// ============================================================================

#[derive(Debug, Clone)]
pub struct DeepResearchConfig {
    pub max_iterations: i32,
    pub max_papers_per_iteration: usize,
    pub verbose: bool,
}

impl Default for DeepResearchConfig {
    fn default() -> Self {
        Self {
            max_iterations: 3,
            max_papers_per_iteration: 5,
            verbose: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeepResearchAgent {
    query: String,
    config: DeepResearchConfig,
    query_strategy: AdaptiveQueryStrategy,
    thoughts: Vec<AgentThought>,
    found_papers: Vec<PaperSnapshot>,
    found_gaps: Vec<GapSnapshot>,
    iterations: i32,
}

impl DeepResearchAgent {
    pub fn new(query: &str, config: DeepResearchConfig) -> Self {
        Self {
            query: query.to_string(),
            config,
            query_strategy: AdaptiveQueryStrategy::new(query),
            thoughts: Vec::new(),
            found_papers: Vec::new(),
            found_gaps: Vec::new(),
            iterations: 0,
        }
    }

    pub fn add_thought(&mut self, role: AgentRole, content: &str) {
        self.thoughts.push(AgentThought::new(self.iterations, role, content));
    }

    pub fn add_paper(&mut self, paper: &Paper) {
        self.found_papers.push(PaperSnapshot::from_paper(paper));
    }

    pub fn add_gap(&mut self, gap: &ResearchGap) {
        self.found_gaps.push(GapSnapshot::from_gap(gap));
    }

    pub fn record_search_result(&mut self, query: &str, gaps: &[ResearchGap]) {
        self.query_strategy.record_search_result(query, gaps);
    }

    pub fn next_query(&self) -> String {
        let latest_gap = self.found_gaps.last();
        let (title, gap_type) = match latest_gap {
            Some(g) => (g.title.as_str(), g.gap_type.as_str()),
            None => ("", ""),
        };
        self.query_strategy.build_adaptive_query(
            self.iterations,
            title,
            gap_type,
            "",
            0.0,
        )
    }

    pub fn should_continue(&self) -> bool {
        self.iterations < self.config.max_iterations
    }

    pub fn increment_iteration(&mut self) {
        self.iterations += 1;
    }

    pub fn build_result(&self, report: &str, duration_secs: f64, status: ResearchStatus) -> DeepResearchResult {
        DeepResearchResult {
            session_id: uuid::Uuid::new_v4().to_string(),
            query: self.query.clone(),
            iterations: self.iterations,
            papers: self.found_papers.clone(),
            gaps: self.found_gaps.clone(),
            thoughts: self.thoughts.clone(),
            report: report.to_string(),
            duration_seconds: duration_secs,
            status,
        }
    }

    pub fn papers_count(&self) -> usize {
        self.found_papers.len()
    }

    pub fn gaps_count(&self) -> usize {
        self.found_gaps.len()
    }
}

// ============================================================================
// Research Session State
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchSession {
    pub id: String,
    pub query: String,
    pub created_at: String,
    pub updated_at: String,
    pub status: String,
    pub current_iteration: i32,
    pub papers_found: usize,
    pub gaps_found: usize,
}

impl ResearchSession {
    pub fn new(query: &str) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            query: query.to_string(),
            created_at: now.clone(),
            updated_at: now,
            status: "active".to_string(),
            current_iteration: 0,
            papers_found: 0,
            gaps_found: 0,
        }
    }
}

// ============================================================================
// Gap Types (for classification)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GapType {
    Capability,
    Improvement,
    Contradiction,
    Assumption,
    Extension,
    BaselineGap,
    EvaluationGap,
    ReproducibilityGap,
    EmbodiedPlanning,
    RlPretraining,
    ScalingLaws,
    Reasoning,
    Unknown,
}

impl GapType {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "capability" => GapType::Capability,
            "improvement" => GapType::Improvement,
            "contradiction" => GapType::Contradiction,
            "assumption" => GapType::Assumption,
            "extension" => GapType::Extension,
            "baseline_gap" => GapType::BaselineGap,
            "evaluation_gap" => GapType::EvaluationGap,
            "reproducibility_gap" => GapType::ReproducibilityGap,
            "embodied_planning" => GapType::EmbodiedPlanning,
            "rl_pretraining" => GapType::RlPretraining,
            "scaling_laws" => GapType::ScalingLaws,
            "reasoning" => GapType::Reasoning,
            _ => GapType::Unknown,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            GapType::Capability => "capability",
            GapType::Improvement => "improvement",
            GapType::Contradiction => "contradiction",
            GapType::Assumption => "assumption",
            GapType::Extension => "extension",
            GapType::BaselineGap => "baseline_gap",
            GapType::EvaluationGap => "evaluation_gap",
            GapType::ReproducibilityGap => "reproducibility_gap",
            GapType::EmbodiedPlanning => "embodied_planning",
            GapType::RlPretraining => "rl_pretraining",
            GapType::ScalingLaws => "scaling_laws",
            GapType::Reasoning => "reasoning",
            GapType::Unknown => "unknown",
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_query_strategy() {
        let mut strategy = AdaptiveQueryStrategy::new("transformer");

        let gaps = vec![
            ResearchGap::new("capability", "Gap 1", "high"),
            ResearchGap::new("improvement", "Gap 2", "medium"),
        ];
        strategy.record_search_result("transformer attention", &gaps);

        let next = strategy.build_adaptive_query(1, "Gap 1", "capability", "", 0.0);
        assert!(next.contains("transformer"));
    }

    #[test]
    fn test_deep_research_agent() {
        let mut agent = DeepResearchAgent::new("RL", DeepResearchConfig::default());

        agent.add_thought(AgentRole::Planner, "Starting research on RL");
        agent.increment_iteration();

        assert_eq!(agent.iterations, 1);
        assert_eq!(agent.thoughts.len(), 1);
        assert!(agent.should_continue());
    }

    #[test]
    fn test_gap_type_from_str() {
        assert_eq!(GapType::from_str("capability"), GapType::Capability);
        assert_eq!(GapType::from_str("contradiction"), GapType::Contradiction);
        assert_eq!(GapType::from_str("unknown_type"), GapType::Unknown);
    }

    #[test]
    fn test_query_similarity() {
        let strategy = AdaptiveQueryStrategy::new("test");
        let sim = strategy.query_similarity("attention mechanism", "attention mechanism transformer");
        assert!(sim > 0.5);
    }
}
