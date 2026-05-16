//! Rairos Research — Deep research orchestration and gap detection
#![allow(dead_code)]
//!
//! Coordinates the full research pipeline: fetch → analyze → detect gaps → evolve.
//! Replaces: research_loop/core.py, research_loop/orchestrator.py, research_loop/deep_research.py

pub mod arxiv_search;
pub mod gap_analysis;
pub mod gene_pool;
pub mod hypothesis_generator;
pub mod skill_discovery;
pub mod snapstate;

use rairos_core::{Database, Paper, ResearchGap};
use rairos_llm::{CostTracker, GapDetector, LlmClient, Message};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
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

    #[error("PDF error: {0}")]
    Pdf(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for the autonomous research loop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchLoopConfig {
    /// Max papers to process in one run
    pub limit: usize,
    /// Max characters of extracted text to send to LLM
    pub max_text_len: usize,
    /// Whether to download PDFs
    pub download_pdfs: bool,
    /// Skip papers whose note already exists
    pub skip_existing: bool,
    /// Default LLM model
    pub model: String,
    /// Base URL for LLM API
    pub base_url: Option<String>,
    /// API key for LLM
    pub api_key: Option<String>,
}

impl Default for ResearchLoopConfig {
    fn default() -> Self {
        Self {
            limit: 5,
            max_text_len: 8000,
            download_pdfs: true,
            skip_existing: true,
            model: "gpt-4o-mini".to_string(),
            base_url: None,
            api_key: None,
        }
    }
}

/// Configuration for deep research agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepResearchConfig {
    pub max_iterations: i32,
    pub max_papers_per_iteration: usize,
    pub verbose: bool,
    pub use_streaming_reasoning: bool,
    pub auto_checkpoint: bool,
    pub checkpoint_every_n_steps: i32,
    pub checkpoint_interval_seconds: i32,
}

impl Default for DeepResearchConfig {
    fn default() -> Self {
        Self {
            max_iterations: 3,
            max_papers_per_iteration: 5,
            verbose: false,
            use_streaming_reasoning: false,
            auto_checkpoint: true,
            checkpoint_every_n_steps: 1,
            checkpoint_interval_seconds: 60,
        }
    }
}

/// Configuration for the autonomous orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    pub interval_minutes: i32,
    pub min_gap_severity_for_alert: String,
    pub min_gene_pool_score_for_alert: f64,
    pub min_papers_for_deep_analysis: i32,
    pub max_alerts_stored: i32,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            interval_minutes: 30,
            min_gap_severity_for_alert: "MEDIUM".to_string(),
            min_gene_pool_score_for_alert: 0.3,
            min_papers_for_deep_analysis: 3,
            max_alerts_stored: 50,
        }
    }
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
// Agent Role & Thought Types
// ============================================================================

const ALL_GAP_TYPES: &[&str] = &[
    "capability",
    "improvement",
    "contradiction",
    "assumption",
    "extension",
    "baseline_gap",
    "evaluation_gap",
    "reproducibility_gap",
    "embodied_planning",
    "rl_pretraining",
    "scaling_laws",
    "reasoning",
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

// ============================================================================
// Paper & Gap Snapshots
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PaperSnapshot {
    pub paper_id: String,
    pub arxiv_id: Option<String>,
    pub title: String,
    pub abstract_text: String,
    pub published: String,
    pub citations: Vec<String>,
    pub extracted_text: Option<String>,
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
            extracted_text: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GapSnapshot {
    pub gap_id: String,
    pub gap_type: String,
    pub title: String,
    pub description: String,
    pub severity: String,
    pub novelty_score: f64,
    pub related_paper_ids: Vec<String>,
    pub archetype_match: f64,
    pub accepted: bool,
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
            archetype_match: 0.5,
            accepted: false,
        }
    }
}

// ============================================================================
// Deep Research Result & Status
// ============================================================================

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
    pub search_history: Vec<String>,
    pub findings: Vec<String>,
    pub archetype: HashMap<String, f64>,
}

impl ResearchSession {
    pub fn new(query: &str) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: Uuid::new_v4().to_string(),
            query: query.to_string(),
            created_at: now.clone(),
            updated_at: now,
            status: "active".to_string(),
            current_iteration: 0,
            papers_found: 0,
            gaps_found: 0,
            search_history: Vec::new(),
            findings: Vec::new(),
            archetype: HashMap::new(),
        }
    }

    pub fn duration(&self) -> f64 {
        let updated = chrono::DateTime::parse_from_rfc3339(&self.updated_at).ok();
        let created = chrono::DateTime::parse_from_rfc3339(&self.created_at).ok();
        match (updated, created) {
            (Some(dt), Some(created)) => (dt - created).num_seconds() as f64,
            _ => 0.0,
        }
    }
}

// ============================================================================
// Adaptive Query Strategy
// ============================================================================

#[derive(Debug, Clone)]
pub struct AdaptiveQueryStrategy {
    topic: String,
    query_gap_types: HashMap<String, Vec<String>>,
    gap_type_counts: HashMap<String, usize>,
    total_gaps: usize,
}

impl AdaptiveQueryStrategy {
    pub fn new(topic: &str) -> Self {
        Self {
            topic: topic.to_string(),
            query_gap_types: HashMap::new(),
            gap_type_counts: HashMap::new(),
            total_gaps: 0,
        }
    }

    pub fn record_search_result(&mut self, query: &str, gaps: &[GapSnapshot]) {
        if gaps.is_empty() {
            return;
        }

        let mut found_types: std::collections::HashSet<String> = std::collections::HashSet::new();
        for g in gaps {
            let gt = g.gap_type.clone();
            *self.gap_type_counts.entry(gt.clone()).or_insert(0) += 1;
            found_types.insert(gt);
            self.total_gaps += 1;
        }
        self.query_gap_types
            .insert(query.to_string(), found_types.into_iter().collect());
    }

    pub fn gap_type_coverage(&self) -> HashMap<String, f64> {
        if self.total_gaps == 0 {
            return ALL_GAP_TYPES
                .iter()
                .map(|&gt| (gt.to_string(), 0.0))
                .collect();
        }
        self.gap_type_counts
            .iter()
            .map(|(gt, &count)| (gt.clone(), count as f64 / self.total_gaps as f64))
            .collect()
    }

    pub fn under_represented_types(&self, threshold: f64) -> Vec<String> {
        let coverage = self.gap_type_coverage();
        coverage
            .iter()
            .filter(|(_, &ratio)| ratio > 0.0 && ratio < threshold)
            .map(|(gt, _)| gt.clone())
            .collect()
    }

    pub fn most_productive_queries(&self, top_k: usize) -> Vec<String> {
        let mut scored: Vec<(String, usize)> = self
            .query_gap_types
            .iter()
            .map(|(q, types)| (q.clone(), types.len()))
            .collect();
        scored.sort_by_key(|b| std::cmp::Reverse(b.1));
        scored.into_iter().take(top_k).map(|(q, _)| q).collect()
    }

    pub fn build_adaptive_query(
        &self,
        iteration: i32,
        latest_gap_title: &str,
        latest_gap_type: &str,
        gene_pool_hint: &str,
        confidence: f64,
    ) -> String {
        let under_rep = self.under_represented_types(0.15);

        if iteration == 0 {
            return self.topic.clone();
        }

        if !under_rep.is_empty() {
            let target = &under_rep[0];
            let productive = self.most_productive_queries(1);
            let base = productive
                .first()
                .map(|s| s.as_str())
                .unwrap_or(&self.topic);
            return format!("{} {}", base, target);
        }

        if confidence >= 0.4 && !gene_pool_hint.is_empty() {
            if !latest_gap_title.is_empty() {
                return format!("{} {}", gene_pool_hint, latest_gap_title);
            }
            return gene_pool_hint.to_string();
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
        let words1: std::collections::HashSet<String> = q1
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        let words2: std::collections::HashSet<String> = q2
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
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
pub struct DeepResearchAgent {
    pub query: String,
    config: DeepResearchConfig,
    query_strategy: AdaptiveQueryStrategy,
    thoughts: Vec<AgentThought>,
    found_papers: Vec<PaperSnapshot>,
    found_gaps: Vec<GapSnapshot>,
    iterations: i32,
    session_id: String,
    search_history: Vec<String>,
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
            session_id: Uuid::new_v4().to_string(),
            search_history: Vec::new(),
        }
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn add_thought(&mut self, role: AgentRole, content: &str) {
        self.thoughts
            .push(AgentThought::new(self.iterations, role, content));
    }

    pub fn add_paper(&mut self, paper: &PaperSnapshot) {
        self.found_papers.push(paper.clone());
    }

    pub fn add_gap(&mut self, gap: &GapSnapshot) {
        self.found_gaps.push(gap.clone());
    }

    pub fn record_search_result(&mut self, query: &str, gaps: &[GapSnapshot]) {
        self.query_strategy.record_search_result(query, gaps);
    }

    pub fn next_query(
        &self,
        latest_gap: Option<&GapSnapshot>,
        gene_pool_hint: &str,
        confidence: f64,
    ) -> String {
        let (title, gap_type) = match latest_gap {
            Some(g) => (g.title.as_str(), g.gap_type.as_str()),
            None => ("", ""),
        };
        self.query_strategy.build_adaptive_query(
            self.iterations,
            title,
            gap_type,
            gene_pool_hint,
            confidence,
        )
    }

    pub fn should_continue(&self, papers_count: usize, gaps_count: usize) -> bool {
        if self.iterations >= self.config.max_iterations {
            return false;
        }
        if papers_count
            >= (self.config.max_iterations as usize) * self.config.max_papers_per_iteration
        {
            return false;
        }
        if gaps_count == 0 && self.iterations > 1 {
            return false;
        }
        true
    }

    pub fn increment_iteration(&mut self) {
        self.iterations += 1;
    }

    pub fn build_result(
        &self,
        report: &str,
        duration_secs: f64,
        status: ResearchStatus,
    ) -> DeepResearchResult {
        DeepResearchResult {
            session_id: self.session_id.clone(),
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

    pub fn deep_config(&self) -> &DeepResearchConfig {
        &self.config
    }

    /// Plan the next search query
    pub fn plan_next_search(
        &mut self,
        latest_gap: Option<&GapSnapshot>,
        gene_pool_hint: &str,
        confidence: f64,
    ) -> String {
        let planned = self.next_query(latest_gap, gene_pool_hint, confidence);

        // Semantic deduplication: avoid near-duplicate queries
        for prev_q in &self.search_history {
            let sim = self.query_strategy.query_similarity(&planned, prev_q);
            if sim > 0.75 {
                return format!("{} variant{}", planned, self.iterations);
            }
        }

        planned
    }

    /// Reflect on whether to continue iterating
    pub fn reflect(&self, iteration: i32) -> (bool, String) {
        if iteration >= self.config.max_iterations {
            return (
                false,
                format!("max iterations ({}) reached", self.config.max_iterations),
            );
        }

        if self.found_papers.len()
            >= (self.config.max_iterations as usize) * self.config.max_papers_per_iteration
        {
            return (false, "max papers reached".to_string());
        }

        if self.found_gaps.is_empty() && iteration > 1 {
            return (false, "no gaps found after thorough search".to_string());
        }

        let recent_gaps: Vec<_> = self.found_gaps.iter().filter(|g| g.accepted).collect();
        if !recent_gaps.is_empty() {
            return (
                false,
                format!("{} gaps accepted, stopping", recent_gaps.len()),
            );
        }

        if !self.found_gaps.is_empty() {
            let avg_match: f64 = self
                .found_gaps
                .iter()
                .map(|g| g.archetype_match)
                .sum::<f64>()
                / self.found_gaps.len() as f64;
            if avg_match < 0.3 && iteration >= 2 {
                return (
                    true,
                    format!("low archetype match ({:.2}), broadening search", avg_match),
                );
            }
        }

        (true, "continue iterating".to_string())
    }
}

// ─── Research Backend Trait ─────────────────────────────────────────────────────

pub trait ResearchBackend {
    fn stream_plan(&self, query: &str, _iteration: i32) -> Result<String, String> {
        Ok(query.to_string())
    }
    fn search_papers(&self, query: &str, max: usize) -> Result<Vec<rairos_core::Paper>, String> {
        crate::arxiv_search::search(query, max)
    }
    fn extract_paper(&self, paper: &rairos_core::Paper) -> Result<PaperSnapshot, String> {
        let url = &paper.metadata.pdf_url.clone().unwrap_or_default();
        if url.is_empty() {
            return Ok(PaperSnapshot::from_paper(paper));
        }

        let resp = reqwest::blocking::get(url).map_err(|e| format!("PDF download failed: {}", e))?;
        let bytes = resp.bytes().map_err(|e| format!("PDF read failed: {}", e))?;

        // Write to temp file
        let tmp_dir = std::env::temp_dir().join("rairos_pdf");
        std::fs::create_dir_all(&tmp_dir).ok();
        let tmp_path = tmp_dir.join(format!("{}.pdf", paper.id));
        std::fs::write(&tmp_path, &bytes).ok();

        // Extract text
        let text = rairos_pdf::extract_pdf_text(&tmp_path).unwrap_or_default();
        let _ = std::fs::remove_file(&tmp_path);

        let mut snap = PaperSnapshot::from_paper(paper);
        snap.extracted_text = Some(text.chars().take(5000).collect());
        Ok(snap)
    }
    fn analyze_gaps(&self, topic: &str, snapshots: &[PaperSnapshot]) -> Result<Vec<GapSnapshot>, String> {
        Ok(crate::gap_analysis::analyze_gaps(snapshots, topic))
    }
    fn get_search_guidance(
        &self, topic: &str, gap_type: &str, gap_title: &str,
    ) -> Result<(Option<String>, f64), String> {
        let pool = crate::gene_pool::GenePool::new();
        let keywords = crate::gene_pool::extract_keywords(gap_title);
        let kws: Vec<String> = keywords.iter().map(|s| s.to_string()).collect();
        Ok(pool.find_capsule(topic, gap_type, Some(&kws), 0.1))
    }
    fn encode_accepted_gap(&self, topic: &str, gap: &GapSnapshot) -> Result<(), String> {
        let pool = crate::gene_pool::GenePool::new();
        pool.record_gap_accept(
            topic,
            &gap.gap_type,
            &gap.title,
            &gap.description,
        )
    }
    fn on_thought(&self, thought: &AgentThought) -> Result<(), String> {
        eprintln!("[{}] {}: {}", thought.iteration, thought.role, thought.content);
        Ok(())
    }
    fn find_skills(&self, query: &str) -> Result<Vec<String>, String> {
        let skills = crate::skill_discovery::discover_skills();
        let matched = crate::skill_discovery::match_skills(query, &skills);
        Ok(matched.into_iter().map(|s| s.name).collect())
    }
    fn checkpoint(&self, session_json: &str) -> Result<(), String> {
        if let Ok(session) = serde_json::from_str::<crate::snapstate::SnapSession>(session_json) {
            let store = crate::snapstate::Snapstate::new(None);
            store.create_checkpoint(&session).ok();
        }
        Ok(())
    }
    fn new_session(&self, query: &str, max_iterations: i32) -> Result<String, String> {
        let store = crate::snapstate::Snapstate::new(None);
        let session = store.new_session(query, max_iterations);
        let _ = store.save(&session);
        serde_json::to_string(&session).map_err(|e| e.to_string())
    }
}

impl DeepResearchAgent {
    pub fn build_report(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("# Deep Research Report: {}", self.query));
        lines.push(String::new());
        lines.push(format!(
            "**Session**: {} | **Iterations**: {} | **Status**: completed",
            self.session_id, self.iterations,
        ));
        lines.push(String::new());
        lines.push("## Papers Analyzed".to_string());
        for p in &self.found_papers {
            let gaps = p.citations.len();
            lines.push(format!("- [{}] {} — {} citations", p.paper_id, p.title, gaps));
        }
        lines.push(String::new());
        lines.push("## Research Gaps".to_string());
        for g in &self.found_gaps {
            let status = if g.accepted { "✅" } else { "⬜" };
            lines.push(format!("- {} [{}] {}", status, g.gap_type, g.title));
            if g.description.len() > 100 {
                lines.push(format!("  {}", &g.description[..100]));
            } else {
                lines.push(format!("  {}", g.description));
            }
        }
        lines.join("\n")
    }

    pub fn run(
        &mut self,
        backend: &dyn ResearchBackend,
        _mode: &str,
        stop_requested: bool,
    ) -> DeepResearchResult {
        let start = Instant::now();

        // Create or resume session
        let _session_json = backend.new_session(&self.query, self.config.max_iterations);

        // Signal handler for Ctrl+C would need external crate (ctrlc)

        while self.iterations < self.config.max_iterations && !stop_requested {
            self.add_thought(AgentRole::Planner, &format!("Starting iteration {}", self.iterations + 1));

            // Step 0: Find skills
            let skills = backend.find_skills(&self.query).unwrap_or_default();
            if !skills.is_empty() {
                self.add_thought(AgentRole::Planner, &format!("Matched skills: {:?}", skills));
            }

            // Step 1: Plan
            let search_query = if self.config.use_streaming_reasoning {
                backend.stream_plan(&self.query, self.iterations)
                    .unwrap_or_else(|_| self.query.clone())
            } else {
                let latest_gap = self.found_gaps.last().cloned();
                let (hint, confidence) = latest_gap.as_ref()
                    .and_then(|g| {
                        backend.get_search_guidance(&self.query, &g.gap_type, &g.title).ok()
                    })
                    .unwrap_or((None, 0.0));
                let planned = self.plan_next_search(latest_gap.as_ref(), &hint.unwrap_or_default(), confidence);
                // Dedup against search history
                let mut deduped = planned.clone();
                for prev in &self.search_history {
                    if self.query_strategy.query_similarity(&deduped, prev) > 0.75 {
                        deduped = format!("{} variant{}", planned, self.iterations);
                        break;
                    }
                }
                deduped
            };

            self.add_thought(AgentRole::Planner, &format!("Planned search: {}", search_query));

            // Checkpoint after plan
            if self.config.auto_checkpoint {
                // serialize self and checkpoint
            }

            // Step 2: Search
            let papers = backend.search_papers(&search_query, self.config.max_papers_per_iteration)
                .unwrap_or_default();
            let mut found_papers = papers;

            // Retry with original query if empty
            if found_papers.is_empty() {
                found_papers = backend.search_papers(&self.query, self.config.max_papers_per_iteration)
                    .unwrap_or_default();
            }

            self.search_history.push(search_query.clone());

            if found_papers.is_empty() {
                self.add_thought(AgentRole::Searcher, "No papers found, retrying next iteration");
                self.increment_iteration();
                continue;
            }

            self.add_thought(AgentRole::Searcher,
                &format!("Found {} papers", found_papers.len()));

            // Step 3: Extract
            let mut snapshots = Vec::new();
            for paper in &found_papers {
                match backend.extract_paper(paper) {
                    Ok(snap) => {
                        snapshots.push(snap);
                    }
                    Err(e) => {
                        self.add_thought(AgentRole::Searcher, &format!("Extraction failed: {}", e));
                    }
                }
            }

            for snap in &snapshots {
                self.add_paper(snap);
            }
            self.add_thought(AgentRole::Searcher,
                &format!("Extracted {} papers", snapshots.len()));

            // Step 4: Analyze gaps
            if !snapshots.is_empty() {
                let gaps = backend.analyze_gaps(&self.query, &snapshots).unwrap_or_default();
                for gap in &gaps {
                    self.add_gap(gap);
                }
                self.record_search_result(&search_query, &gaps);
                self.add_thought(AgentRole::Analyzer,
                    &format!("Found {} gaps", gaps.len()));
            }

            // Step 5: Reflect
            let (should_continue, reason) = self.reflect(self.iterations);
            self.add_thought(AgentRole::Reflector, &reason);

            if !should_continue {
                break;
            }

            self.increment_iteration();
        }

        // Post-loop: encode accepted gaps, finalize
        for gap in &self.found_gaps.clone() {
            if gap.accepted && gap.archetype_match > 0.0 {
                let _ = backend.encode_accepted_gap(&self.query, gap);
            }
        }

        let duration = start.elapsed().as_secs_f64();
        let report = self.build_report();
        self.build_result(&report, duration, ResearchStatus::Completed)
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
        let start = Instant::now();

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

    async fn find_relevant_papers(
        &self,
        query: &ResearchQuery,
    ) -> Result<Vec<Paper>, ResearchError> {
        let all_papers = self
            .db
            .list_papers(None, query.max_papers, 0)
            .map_err(|e| ResearchError::Database(e.to_string()))?;

        Ok(all_papers)
    }

    async fn analyze_citations(&self, papers: &[Paper]) -> usize {
        papers.len()
    }

    async fn detect_gaps(
        &self,
        papers: &[Paper],
        categories: &[String],
    ) -> Result<Vec<ResearchGap>, ResearchError> {
        let keywords: Vec<&str> = categories.iter().map(|s| s.as_str()).collect();
        let gap_descriptions = GapDetector::detect_gaps(papers, &keywords);

        let under_category = GapDetector::find_underexplored_areas(papers, 3);

        let mut gaps = Vec::new();

        for desc in gap_descriptions {
            gaps.push(ResearchGap::new("keyword_gap", &desc, "medium"));
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

    async fn generate_suggestions(
        &self,
        papers: &[Paper],
        gaps: &[ResearchGap],
    ) -> Result<Vec<String>, ResearchError> {
        if gaps.is_empty() {
            return Ok(vec![]);
        }

        let context = papers
            .iter()
            .take(5)
            .map(|p| format!("- {} ({})", p.title, p.arxiv_id.as_deref().unwrap_or("")))
            .collect::<Vec<_>>()
            .join("\n");

        let gaps_text = gaps
            .iter()
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
            Message {
                role: "system".to_string(),
                content: "You are a helpful research assistant.".to_string(),
            },
            Message {
                role: "user".to_string(),
                content: prompt,
            },
        ];

        let response = self
            .llm
            .complete(messages, "gpt-4o", 0.7, 500)
            .await
            .map_err(|e| ResearchError::Llm(e.to_string()))?;

        {
            let mut tracker = self.cost_tracker.write().await;
            tracker.record(response.usage(), response.model(), self.llm.provider_name());
        }

        let suggestions: Vec<String> = response
            .content()
            .lines()
            .filter(|l| {
                let trimmed = l.trim();
                trimmed.starts_with('-')
                    || trimmed.starts_with('1')
                    || trimmed.starts_with('2')
                    || trimmed.starts_with('3')
            })
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
// Research Loop (sync + async) — core.py port
// ============================================================================

/// Research note output
#[derive(Debug, Clone)]
pub struct ResearchNoteOutput {
    pub note_path: PathBuf,
    pub pdf_path: Option<PathBuf>,
    pub error: Option<String>,
}

/// Run the autonomous research loop synchronously
/// Search arXiv → download PDFs → extract text → generate notes
pub async fn run_research_loop(
    _query: &str,
    _config: &ResearchLoopConfig,
    output_dir: Option<PathBuf>,
    _db: Option<Arc<Database>>,
) -> Result<Vec<ResearchNoteOutput>, ResearchError> {
    // Note: ArxivSearch, PdfDownloader, PdfExtractor were removed from rairos_parser
    // This function requires reimplementation using the current rairos-parser API
    let output_dir = output_dir.unwrap_or_else(|| {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join("ai_research")
    });

    std::fs::create_dir_all(&output_dir).map_err(ResearchError::Io)?;

    // Return empty results - actual implementation requires parser integration
    Ok(vec![])
}

async fn download_pdf_with_retry(
    _pdf_url: &str,
    _max_retries: usize,
) -> Result<(PathBuf, String), ResearchError> {
    // PdfDownloader/PdfExtractor are not available in current rairos-parser
    Err(ResearchError::Pdf(
        "PDF download not implemented".to_string(),
    ))
}

fn build_research_note_markdown(paper: &Paper, extracted_text: &str) -> String {
    let mut lines = Vec::new();

    lines.push(format!("# {}", paper.title));
    lines.push(String::new());
    lines.push(format!("**ID:** `{}`", paper.id));
    if let Some(ref arxiv_id) = paper.arxiv_id {
        lines.push(format!(
            "**Source:** [arXiv](https://arxiv.org/abs/{})",
            arxiv_id
        ));
    }
    if let Some(ref pdf_url) = paper.metadata.pdf_url {
        lines.push(format!("**PDF:** [PDF]({})", pdf_url));
    }
    lines.push(format!("**Published:** {}", paper.published.to_rfc3339()));
    lines.push(String::new());

    if !paper.abstract_text.is_empty() {
        lines.push("## Abstract".to_string());
        lines.push(String::new());
        lines.push(paper.abstract_text.clone());
        lines.push(String::new());
    }

    if !extracted_text.is_empty() {
        lines.push("---".to_string());
        lines.push(String::new());
        lines.push("## Extracted Text".to_string());
        lines.push(String::new());
        lines.push(extracted_text.to_string());
        lines.push(String::new());
    } else {
        lines.push("---".to_string());
        lines.push(String::new());
        lines.push("_Note: Set API key to enable text extraction._".to_string());
        lines.push(String::new());
    }

    lines.push("---".to_string());
    lines.push(format!(
        "_Generated by rairos-research on {}_",
        chrono::Utc::now().date_naive()
    ));

    lines.join("\n")
}

fn sanitize_filename(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect()
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
        scores
            .into_iter()
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
        self.nodes
            .iter()
            .filter(|n| n.claim.to_lowercase().contains(&keyword.to_lowercase()))
            .collect()
    }

    pub fn find_contradictions(&self, claim_id: &str) -> Vec<&ClaimNode> {
        let claim = self.nodes.iter().find(|n| n.id == claim_id);
        match claim {
            Some(c) => self
                .nodes
                .iter()
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
        let total_citations: usize = papers.iter().map(|p| p.metadata.cited_by).sum();

        let avg_citations = if papers.is_empty() {
            0.0
        } else {
            total_citations as f32 / papers.len() as f32
        };

        let most_cited = papers
            .iter()
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
// Research Alert (orchestrator.py port)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchAlert {
    pub alert_id: String,
    pub session_id: String,
    pub topic: String,
    pub triggered_by: String,
    pub trigger_title: String,
    pub gaps_found: i32,
    pub top_gap_title: String,
    pub top_gap_type: String,
    pub severity: String,
    pub gene_pool_score: f64,
    pub preference_boost: bool,
    pub created_at: f64,
}

impl ResearchAlert {
    pub fn new(
        topic: &str,
        session_id: &str,
        triggered_by: &str,
        trigger_title: &str,
        gap: &GapSnapshot,
        gene_pool_score: f64,
    ) -> Self {
        Self {
            alert_id: Uuid::new_v4().to_string()[..8].to_string(),
            session_id: session_id.to_string(),
            topic: topic.to_string(),
            triggered_by: triggered_by.to_string(),
            trigger_title: trigger_title.to_string(),
            gaps_found: 1,
            top_gap_title: gap.title.clone(),
            top_gap_type: gap.gap_type.clone(),
            severity: gap.severity.clone(),
            gene_pool_score,
            preference_boost: gene_pool_score >= 0.5,
            created_at: chrono::Utc::now().timestamp_millis() as f64 / 1000.0,
        }
    }
}

// ============================================================================
// Gap Type Classification
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
            GapSnapshot {
                gap_id: "1".to_string(),
                gap_type: "capability".to_string(),
                title: "Gap 1".to_string(),
                description: "desc".to_string(),
                severity: "high".to_string(),
                novelty_score: 0.5,
                related_paper_ids: vec![],
                archetype_match: 0.5,
                accepted: false,
            },
            GapSnapshot {
                gap_id: "2".to_string(),
                gap_type: "improvement".to_string(),
                title: "Gap 2".to_string(),
                description: "desc".to_string(),
                severity: "medium".to_string(),
                novelty_score: 0.5,
                related_paper_ids: vec![],
                archetype_match: 0.5,
                accepted: false,
            },
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
        assert!(agent.should_continue(0, 0));
    }

    #[test]
    fn test_gap_type_from_str() {
        assert_eq!(GapType::from_string("capability"), GapType::Capability);
        assert_eq!(GapType::from_string("contradiction"), GapType::Contradiction);
        assert_eq!(GapType::from_string("unknown_type"), GapType::Unknown);
    }

    #[test]
    fn test_query_similarity() {
        let strategy = AdaptiveQueryStrategy::new("test");
        let sim =
            strategy.query_similarity("attention mechanism", "attention mechanism transformer");
        assert!(sim > 0.5);
    }

    #[test]
    fn test_paper_ranker() {
        let papers = vec![
            Paper {
                id: "1".to_string(),
                title: "Attention Is All You Need".to_string(),
                abstract_text: "We propose a new network architecture".to_string(),
                ..Default::default()
            },
            Paper {
                id: "2".to_string(),
                title: "BERT Pre-training".to_string(),
                abstract_text: "We present a new language model".to_string(),
                ..Default::default()
            },
        ];

        let scores = PaperRanker::rank(&papers, "attention transformer");
        assert_eq!(scores[0].0, "1"); // attention paper should rank higher
    }

    #[test]
    fn test_sanitize_filename() {
        let input = "Test: Paper <Title> (2024)";
        let output = sanitize_filename(input);
        assert!(!output.contains(':'));
        assert!(!output.contains('<'));
        assert!(!output.contains('>'));
    }

    // ── Integration tests for DeepResearchAgent ─────────────────────────────

    struct MockBackend;
    impl ResearchBackend for MockBackend {}

    #[test]
    fn test_plan_next_search_basic() {
        let config = DeepResearchConfig::default();
        let mut agent = DeepResearchAgent::new("deep learning", config);
        // No gaps → returns original query
        let q = agent.plan_next_search(None, "", 0.0);
        assert_eq!(q, "deep learning");
    }

    #[test]
    fn test_plan_next_search_with_gap() {
        let mut agent = DeepResearchAgent::new("transformer", DeepResearchConfig::default());
        let gap = GapSnapshot { gap_type: "contradiction".to_string(), ..Default::default() };
        agent.add_gap(&gap);
        // With a gap, should produce a non-empty adaptive query
        let q = agent.plan_next_search(Some(&gap), "", 0.0);
        assert!(!q.is_empty());
    }

    #[test]
    fn test_reflect_stops_at_max_iterations() {
        let config = DeepResearchConfig { max_iterations: 3, ..Default::default() };
        let agent = DeepResearchAgent::new("q", config);
        let (should, reason) = agent.reflect(3);
        assert!(!should, "should stop at max iterations");
        assert!(reason.contains("max iterations"), "reason: {reason}");
    }

    #[test]
    fn test_reflect_continues_with_unaccepted_gaps() {
        let mut agent = DeepResearchAgent::new("q", DeepResearchConfig::default());
        agent.add_gap(&GapSnapshot { gap_type: "improvement".to_string(), accepted: false, ..Default::default() });
        let (should, _) = agent.reflect(0);
        assert!(should, "should continue with unaccepted gaps");
    }

    #[test]
    fn test_build_report_basic() {
        let mut agent = DeepResearchAgent::new("my topic", DeepResearchConfig::default());
        agent.add_paper(&PaperSnapshot {
            paper_id: "2401.00001".to_string(),
            title: "Test Paper".to_string(),
            ..Default::default()
        });
        agent.add_gap(&GapSnapshot {
            gap_type: "method_limitation".to_string(),
            title: "Gap 1".to_string(),
            ..Default::default()
        });
        let report = agent.build_report();
        assert!(report.contains("my topic"), "report should contain topic");
        assert!(report.contains("2401.00001"), "report should contain paper id");
        assert!(report.contains("method_limitation"), "report should contain gap type");
    }

    #[test]
    fn test_run_loop_with_mock_backend() {
        let config = DeepResearchConfig { max_iterations: 2, ..Default::default() };
        let mut agent = DeepResearchAgent::new("test query", config);
        let result = agent.run(&MockBackend, "agent", false);
        assert_eq!(result.status, ResearchStatus::Completed);
        assert!(result.iterations > 0, "should have completed iterations");
        assert!(!result.report.is_empty(), "should have generated a report");
        assert_eq!(result.query, "test query");
    }
}
