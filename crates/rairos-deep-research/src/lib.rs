//! Deep Research Agent — iterative research with gap detection and archetype-aware refinement.
//!
//! Architecture inspired by:
//! - gpt-researcher: multi-agent research with planning
//! - deer-flow: sandbox + memory + tool use
//! - snapstate: session persistence for pause/resume

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::RwLock;
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum DeepResearchError {
    #[error("Session not initialized")]
    NoSession,
    #[error("Database error: {0}")]
    Database(String),
    #[error("MCP tool error: {0}")]
    McpTool(String),
    #[error("Search failed: {0}")]
    SearchFailed(String),
    #[error("Extraction failed: {0}")]
    ExtractionFailed(String),
    #[error("Analysis failed: {0}")]
    AnalysisFailed(String),
    #[error("Checkpoint error: {0}")]
    Checkpoint(String),
    #[error("Signal handler error: {0}")]
    Signal(String),
}

// ============================================================================
// Data Structures
// ============================================================================

/// A single reasoning step in the agent loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentThought {
    pub iteration: usize,
    pub role: String,
    pub content: String,
    pub timestamp: f64,
}

impl AgentThought {
    pub fn new(iteration: usize, role: &str, content: &str) -> Self {
        Self {
            iteration,
            role: role.to_string(),
            content: content.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis() as f64 / 1000.0,
        }
    }
}

/// Final result of a deep research agent run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepResearchResult {
    pub session_id: String,
    pub query: String,
    pub iterations: usize,
    pub papers: Vec<PaperSnapshot>,
    pub gaps: Vec<GapSnapshot>,
    pub thoughts: Vec<AgentThought>,
    pub report: String,
    pub duration_seconds: f64,
    pub status: String,
}

/// Paper snapshot for session persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperSnapshot {
    pub arxiv_id: String,
    pub title: String,
    pub abstract_text: String,
    pub url: String,
    pub extracted_text: String,
    #[serde(default)]
    pub gaps_found: usize,
}

/// Gap snapshot for session persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapSnapshot {
    pub gap_type: String,
    pub title: String,
    pub description: String,
    pub matched_papers: Vec<String>,
    #[serde(default)]
    pub archetype_match: f64,
    #[serde(default)]
    pub accepted: bool,
}

/// Research session state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchSession {
    pub session_id: String,
    pub query: String,
    pub iteration: usize,
    pub max_iterations: usize,
    pub status: String,
    pub archetype: HashMap<String, f64>,
    pub papers: Vec<PaperSnapshot>,
    pub gaps: Vec<GapSnapshot>,
    pub findings: Vec<String>,
    pub search_history: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ResearchSession {
    pub fn new(session_id: &str, query: &str, max_iterations: usize) -> Self {
        let now = Utc::now();
        Self {
            session_id: session_id.to_string(),
            query: query.to_string(),
            iteration: 0,
            max_iterations,
            status: "active".to_string(),
            archetype: HashMap::new(),
            papers: Vec::new(),
            gaps: Vec::new(),
            findings: Vec::new(),
            search_history: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// Paper data structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paper {
    pub uid: String,
    pub title: String,
    pub abstract_text: String,
    pub authors: Vec<String>,
    pub source: String,
    pub pdf_url: String,
    pub published: String,
    pub updated: String,
    pub abs_url: String,
    #[serde(default)]
    pub primary_category: Option<String>,
    #[serde(default)]
    pub categories: Option<String>,
}

// ============================================================================
// Adaptive Query Strategy
// ============================================================================

/// Adaptive query planning: evolve search strategy based on gap coverage.
pub struct AdaptiveQueryStrategy {
    topic: String,
    /// query → list of gap_type found
    query_gap_types: HashMap<String, Vec<String>>,
    /// gap_type → how many times it appeared
    gap_type_counts: HashMap<String, usize>,
    total_gaps: usize,
}

impl AdaptiveQueryStrategy {
    const ALL_GAP_TYPES: &'static [&'static str] = &[
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

    pub fn new(topic: &str) -> Self {
        Self {
            topic: topic.to_string(),
            query_gap_types: HashMap::new(),
            gap_type_counts: HashMap::new(),
            total_gaps: 0,
        }
    }

    /// Record gap types found from a search result.
    pub fn record_search_result(&mut self, query: &str, gaps: &[GapSnapshot]) {
        if gaps.is_empty() {
            return;
        }
        let mut found_types = HashSet::new();
        for g in gaps {
            let gt = &g.gap_type;
            *self.gap_type_counts.entry(gt.clone()).or_insert(0) += 1;
            self.total_gaps += 1;
            found_types.insert(gt.clone());
        }
        self.query_gap_types
            .insert(query.to_string(), found_types.into_iter().collect());
    }

    /// Return coverage ratio for each gap type (0.0–1.0).
    pub fn gap_type_coverage(&self) -> HashMap<String, f64> {
        if self.total_gaps == 0 {
            return Self::ALL_GAP_TYPES
                .iter()
                .map(|&gt| (gt.to_string(), 0.0))
                .collect();
        }
        let mut result: HashMap<String, f64> = self
            .gap_type_counts
            .iter()
            .map(|(gt, &count)| (gt.clone(), count as f64 / self.total_gaps as f64))
            .collect();
        for &gt in Self::ALL_GAP_TYPES {
            result.entry(gt.to_string()).or_insert(0.0);
        }
        result
    }

    /// Return gap types that appear in < threshold of all gaps.
    pub fn under_represented_types(&self, threshold: f64) -> Vec<String> {
        self.gap_type_coverage()
            .into_iter()
            .filter(|(_, ratio)| *ratio > 0.0 && *ratio < threshold)
            .map(|(gt, _)| gt)
            .collect()
    }

    /// Return queries that produced the most diverse gap types.
    pub fn most_productive_queries(&self, top_k: usize) -> Vec<String> {
        let mut scored: Vec<(String, usize)> = self
            .query_gap_types
            .iter()
            .map(|(q, types)| (q.clone(), types.len()))
            .collect();
        scored.sort_by(|a, b| b.1.cmp(&a.1));
        scored.into_iter().take(top_k).map(|(q, _)| q).collect()
    }

    /// Build next search query adaptively.
    pub fn build_adaptive_query(
        &self,
        iteration: usize,
        latest_gap_title: &str,
        latest_gap_type: &str,
        gene_pool_hint: &str,
        confidence: f64,
    ) -> String {
        let under_rep = self.under_represented_types(0.15);

        if iteration == 0 {
            return self.topic.clone();
        }

        // Case 1: have under-represented types → target them
        if let Some(target) = under_rep.first() {
            let productive = self.most_productive_queries(1);
            let base = productive
                .first()
                .map(|s| s.as_str())
                .unwrap_or(&self.topic);
            return format!("{} {}", base, target);
        }

        // Case 2: high-confidence GenePool hint
        if !gene_pool_hint.is_empty() && confidence >= 0.4 {
            if !latest_gap_title.is_empty() {
                return format!("{} {}", gene_pool_hint, latest_gap_title);
            }
            return gene_pool_hint.to_string();
        }

        // Case 3: latest gap context
        match latest_gap_type {
            "Contradiction" => format!("{} {} disagreement", self.topic, latest_gap_title),
            "improvement" | "capability" | "extension" | "Missing" | "Unknown" | "" => {
                format!("{} {} improvement", self.topic, latest_gap_title)
            }
            _ => {
                if !latest_gap_title.is_empty() {
                    format!("{} {}", self.topic, latest_gap_title)
                } else {
                    self.topic.clone()
                }
            }
        }
    }

    /// Simple word-overlap similarity between two queries (0.0–1.0).
    pub fn query_similarity(&self, q1: &str, q2: &str) -> f64 {
        let words1: HashSet<String> = q1
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        let words2: HashSet<String> = q2
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        if words1.is_empty() || words2.is_empty() {
            return 0.0;
        }
        let intersection: HashSet<_> = words1.intersection(&words2).collect();
        let union: HashSet<_> = words1.union(&words2).collect();
        intersection.len() as f64 / union.len() as f64
    }
}

// ============================================================================
// Gap Analyzer (Stub)
// ============================================================================

/// Stub gap analyzer - real implementation would use rairos-llm GapAnalyzerV2
pub struct GapAnalyzerV2;

impl Default for GapAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl GapAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze(
        &self,
        _topic: &str,
        _use_insights: bool,
        _min_papers: usize,
        _use_llm: bool,
    ) -> GapAnalysisResult {
        GapAnalysisResult { gaps: Vec::new() }
    }
}

pub struct GapAnalysisResult {
    pub gaps: Vec<Gap>,
}

pub struct Gap {
    pub gap_type: GapType,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq)]
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
    pub fn value(&self) -> &str {
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
// MCP Tool Types
// ============================================================================

/// MCP tool definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

impl McpTool {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            input_schema: serde_json::json!({}),
        }
    }
}

/// MCP tool call result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub papers: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gaps: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ============================================================================
// Progress Tracking
// ============================================================================

/// Progress tracking for observability.
#[derive(Debug, Default)]
pub struct Progress {
    pub papers_found: usize,
    pub papers_extracted: usize,
    pub gaps_found: usize,
    pub searches_done: usize,
    pub iterations_done: usize,
}

// ============================================================================
// Deep Research Agent Configuration
// ============================================================================

/// Configuration for the Deep Research Agent.
#[derive(Debug, Clone)]
pub struct DeepResearchConfig {
    pub query: String,
    pub max_iterations: usize,
    pub max_papers_per_iteration: usize,
    pub verbose: bool,
    pub mode: String,
    pub auto_checkpoint: bool,
    pub checkpoint_every_n_steps: usize,
    pub checkpoint_interval_seconds: u64,
    pub use_streaming_reasoning: bool,
}

impl Default for DeepResearchConfig {
    fn default() -> Self {
        Self {
            query: String::new(),
            max_iterations: 3,
            max_papers_per_iteration: 5,
            verbose: false,
            mode: "agent".to_string(),
            auto_checkpoint: true,
            checkpoint_every_n_steps: 1,
            checkpoint_interval_seconds: 60,
            use_streaming_reasoning: false,
        }
    }
}

// ============================================================================
// Deep Research Agent
// ============================================================================

/// Deep Research Agent with iterative gap-aware refinement loop.
///
/// Loop:
/// 1. PLANNER: decide search strategy based on gaps found
/// 2. SEARCHER: fetch papers from arXiv
/// 3. EXTRACTOR: pull abstracts/full text
/// 4. ANALYZER: detect gaps via GapAnalyzerV2
/// 5. REFLECTOR: assess progress, decide to iterate or stop
/// 6. GENETIC: encode accepted gaps into Gene Pool
pub struct DeepResearchAgent {
    config: DeepResearchConfig,
    session: Option<ResearchSession>,
    thoughts: Vec<AgentThought>,
    gap_analyzer: GapAnalyzerV2,
    adaptive_strategy: RwLock<AdaptiveQueryStrategy>,
    progress: Progress,
    stop_requested: AtomicBool,
    checkpoint_counter: AtomicUsize,
    last_checkpoint_time: RwLock<f64>,
}

impl DeepResearchAgent {
    /// Create a new DeepResearchAgent with configuration.
    pub fn new(config: DeepResearchConfig) -> Self {
        Self {
            config,
            session: None,
            thoughts: Vec::new(),
            gap_analyzer: GapAnalyzerV2::new(),
            adaptive_strategy: RwLock::new(AdaptiveQueryStrategy::new("")),
            progress: Progress::default(),
            stop_requested: AtomicBool::new(false),
            checkpoint_counter: AtomicUsize::new(0),
            last_checkpoint_time: RwLock::new(0.0),
        }
    }

    /// Create with default configuration.
    pub fn with_query(query: &str) -> Self {
        let mut config = DeepResearchConfig::default();
        config.query = query.to_string();
        Self::new(config)
    }

    // -------------------------------------------------------------------------
    // Session lifecycle
    // -------------------------------------------------------------------------

    /// Start a new research session.
    pub fn start(&mut self) -> Result<ResearchSession, DeepResearchError> {
        let session_id = generate_session_id();
        let strategy = AdaptiveQueryStrategy::new(&self.config.query);

        *self.adaptive_strategy.write().unwrap() = strategy;

        let session =
            ResearchSession::new(&session_id, &self.config.query, self.config.max_iterations);

        self.session = Some(session.clone());
        Ok(session)
    }

    /// Resume an existing session.
    #[allow(dead_code)]
    pub fn resume(
        &mut self,
        _session_id: &str,
    ) -> Result<Option<ResearchSession>, DeepResearchError> {
        // In a real implementation, this would load from snapstate
        Ok(self.session.clone())
    }

    /// Pause and persist current session state.
    #[allow(dead_code)]
    pub fn pause(&mut self) -> Result<(), DeepResearchError> {
        if let Some(ref mut session) = self.session {
            session.status = "paused".to_string();
            session.updated_at = Utc::now();
        }
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Thought recording
    // -------------------------------------------------------------------------

    fn record_thought(&mut self, role: &str, content: &str, iteration: usize) {
        let thought = AgentThought::new(iteration, role, content);
        self.thoughts.push(thought.clone());

        if let Some(ref mut session) = self.session {
            session
                .findings
                .push(format!("[{}] {}", role.to_uppercase(), content));
        }

        if self.config.verbose {
            println!(
                "[DeepResearchAgent] [{}] iter{} | {}",
                role.to_uppercase(),
                iteration,
                content
            );
        }
    }

    // -------------------------------------------------------------------------
    // Core iteration steps
    // -------------------------------------------------------------------------

    /// Get search guidance from GenePool (stub for now).
    #[allow(dead_code)]
    fn get_search_guidance(
        &self,
        _topic: &str,
        _gap_type: &str,
        _gap_title: &str,
    ) -> (Option<String>, f64) {
        // TODO: Integrate with rairos-llm EvolutionTracker
        (None, 0.0)
    }

    /// PLANNER: decide next search query using adaptive strategy + GenePool.
    fn plan_next_search(&self, iteration: usize) -> String {
        let gaps = self
            .session
            .as_ref()
            .map(|s| s.gaps.clone())
            .unwrap_or_default();
        let search_history = self
            .session
            .as_ref()
            .map(|s| s.search_history.clone())
            .unwrap_or_default();

        let planned = if iteration == 0 {
            self.config.query.clone()
        } else if let Some(latest_gap) = gaps.last() {
            // Get GenePool guidance
            let (hint, confidence) = self.get_search_guidance(
                &self.config.query,
                &latest_gap.gap_type,
                &latest_gap.title,
            );

            // Use adaptive strategy to build query
            let strategy = self.adaptive_strategy.read().unwrap();
            let planned = strategy.build_adaptive_query(
                iteration,
                &latest_gap.title,
                &latest_gap.gap_type,
                hint.as_deref().unwrap_or(""),
                confidence,
            );

            // Semantic deduplication
            for prev_q in &search_history {
                let sim = strategy.query_similarity(&planned, prev_q);
                if sim > 0.75 {
                    return format!("{} variant{}", planned, iteration);
                }
            }

            planned
        } else {
            self.config.query.clone()
        };

        // Fallback duplicate guard
        if search_history.contains(&planned) {
            return format!("{} exploration{}", self.config.query, iteration);
        }

        planned
    }

    /// SEARCHER: fetch papers (stub for now).
    #[allow(dead_code)]
    fn search_papers(&self, _search_query: &str, _iteration: usize) -> Vec<Paper> {
        // TODO: Integrate with rairos-llm for arXiv search or MCP paper_search
        Vec::new()
    }

    /// EXTRACTOR: extract text from papers and build snapshots.
    #[allow(dead_code)]
    fn extract_papers(&self, papers: &[Paper], _iteration: usize) -> Vec<PaperSnapshot> {
        papers
            .iter()
            .map(|p| PaperSnapshot {
                arxiv_id: p.uid.clone(),
                title: p.title.clone(),
                abstract_text: p.abstract_text.clone(),
                url: p.pdf_url.clone(),
                extracted_text: String::new(), // TODO: extract from PDF
                gaps_found: 0,
            })
            .collect()
    }

    /// ANALYZER: detect research gaps (stub for now).
    #[allow(dead_code)]
    fn analyze_gaps(&self, _snapshots: &[PaperSnapshot], _iteration: usize) -> Vec<GapSnapshot> {
        // TODO: Integrate with rairos-llm GapAnalyzerV2
        Vec::new()
    }

    /// REFLECTOR: decide whether to continue iterating or stop.
    fn reflect(&self, iteration: usize) -> (bool, String) {
        let Some(ref session) = self.session else {
            return (false, "no session".to_string());
        };

        let gaps = &session.gaps;
        let papers = &session.papers;

        // Stop conditions
        if iteration >= self.config.max_iterations {
            return (
                false,
                format!("max iterations ({}) reached", self.config.max_iterations),
            );
        }

        if papers.len() >= self.config.max_iterations * self.config.max_papers_per_iteration {
            return (false, "max papers reached".to_string());
        }

        if gaps.is_empty() && iteration > 1 {
            return (false, "no gaps found after thorough search".to_string());
        }

        // Continue conditions
        let recent_gaps: Vec<_> = gaps.iter().filter(|g| g.accepted).collect();
        if !recent_gaps.is_empty() {
            return (
                false,
                format!("{} gaps accepted, stopping", recent_gaps.len()),
            );
        }

        // Check archetype alignment
        if !gaps.is_empty() {
            let avg_match: f64 =
                gaps.iter().map(|g| g.archetype_match).sum::<f64>() / gaps.len() as f64;
            if avg_match < 0.3 && iteration >= 2 {
                // Broadening search would happen on next iteration
            }
        }

        (true, "continue iterating".to_string())
    }

    /// GENETIC: encode all accepted gaps into the Gene Pool.
    #[allow(dead_code)]
    fn encode_accepted_gaps(&self) {
        // TODO: Integrate with rairos-llm EvolutionTracker
    }

    // -------------------------------------------------------------------------
    // Checkpointing
    // -------------------------------------------------------------------------

    #[allow(dead_code)]
    fn auto_checkpoint(&self) -> Result<(), DeepResearchError> {
        if self.session.is_none() || !self.config.auto_checkpoint {
            return Ok(());
        }
        // TODO: Integrate with rairos-snapstate
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Report building
    // -------------------------------------------------------------------------

    fn build_report(&self) -> String {
        let Some(ref session) = self.session else {
            return "No session".to_string();
        };

        let mut lines = vec![
            format!("# Deep Research Report: {}", session.query),
            String::new(),
            format!(
                "**Session**: {} | **Iterations**: {} | **Status**: {}",
                session.session_id, session.iteration, session.status
            ),
            String::new(),
            "## Papers Analyzed".to_string(),
        ];

        for p in &session.papers {
            lines.push(format!("- [{}] {}", p.arxiv_id, p.title));
        }

        lines.push(String::new());
        lines.push("## Research Gaps".to_string());

        for g in &session.gaps {
            let status = if g.accepted { "✅" } else { "⬜" };
            lines.push(format!("- {} [{}] {}", status, g.gap_type, g.title));
            if !g.description.is_empty() {
                lines.push(format!(
                    "  {}",
                    &g.description[..g.description.len().min(100)]
                ));
            }
        }

        lines.push(String::new());
        lines.push("## Findings".to_string());

        for f in session.findings.iter().rev().take(10) {
            lines.push(format!("- {}", f));
        }

        lines.join("\n")
    }

    // -------------------------------------------------------------------------
    // Main run loop
    // -------------------------------------------------------------------------

    /// Run the deep research agent synchronously.
    pub fn run(&mut self) -> Result<DeepResearchResult, DeepResearchError> {
        if self.session.is_none() {
            self.start()?;
        }

        let start_time = chrono::Utc::now().timestamp_millis() as f64 / 1000.0;
        let mut iteration = self
            .session
            .as_ref()
            .ok_or(DeepResearchError::NoSession)?
            .iteration;

        self.record_thought(
            "planner",
            &format!(
                "topic={:?}, max_iter={}",
                self.config.query, self.config.max_iterations
            ),
            iteration,
        );

        while iteration < self.config.max_iterations && !self.stop_requested.load(Ordering::SeqCst)
        {
            if let Some(ref mut session) = self.session {
                session.iteration = iteration;
            }

            // Step 1: Plan
            let search_query = self.plan_next_search(iteration);
            self.record_thought(
                "planner",
                &format!("Planned search: {}", search_query),
                iteration,
            );

            // Step 2: Search
            if self.config.verbose {
                println!("[DR] [SEARCH] q={:?}", search_query);
            }

            let papers = self.search_papers(&search_query, iteration);

            let has_papers = !papers.is_empty();
            if !has_papers {
                // Fallback to topic query
                let fallback = self.search_papers(&self.config.query, iteration);
                if fallback.is_empty() {
                    iteration += 1;
                    continue;
                }
            }

            if let Some(ref mut session) = self.session {
                session.search_history.push(search_query);
            }

            // Step 3: Extract
            if self.config.verbose {
                println!("[DR] [EXTRACT] {} papers", papers.len());
            }

            let snapshots = self.extract_papers(&papers, iteration);

            if let Some(ref mut session) = self.session {
                session.papers.extend(snapshots.clone());
            }

            // Step 4: Analyze gaps
            if self.config.verbose {
                println!("[DR] [ANALYZE] {} snapshots", snapshots.len());
            }

            let gap_snapshots = self.analyze_gaps(&snapshots, iteration);

            if let Some(ref mut session) = self.session {
                session.gaps.extend(gap_snapshots.clone());
            }

            // Step 5: Reflect
            let (should_continue, reason) = self.reflect(iteration);
            self.record_thought("reflector", &reason, iteration);

            if self.config.verbose {
                println!(
                    "[DeepResearchAgent] [iter {}] Reflect: {}",
                    iteration, reason
                );
            }

            if !should_continue {
                break;
            }

            iteration += 1;

            if let Some(ref mut session) = self.session {
                session.updated_at = Utc::now();
            }
        }

        // Encode accepted gaps into Gene Pool
        self.encode_accepted_gaps();

        // Finalize session
        let duration = chrono::Utc::now().timestamp_millis() as f64 / 1000.0 - start_time;

        if let Some(ref mut session) = self.session {
            session.status = if self.stop_requested.load(Ordering::SeqCst) {
                "paused".to_string()
            } else {
                "completed".to_string()
            };
            session.iteration = iteration;
        }

        let result = DeepResearchResult {
            session_id: self
                .session
                .as_ref()
                .map(|s| s.session_id.clone())
                .unwrap_or_default(),
            query: self.config.query.clone(),
            iterations: iteration + 1,
            papers: self
                .session
                .as_ref()
                .map(|s| s.papers.clone())
                .unwrap_or_default(),
            gaps: self
                .session
                .as_ref()
                .map(|s| s.gaps.clone())
                .unwrap_or_default(),
            thoughts: self.thoughts.clone(),
            report: self.build_report(),
            duration_seconds: duration,
            status: self
                .session
                .as_ref()
                .map(|s| s.status.clone())
                .unwrap_or_else(|| "failed".to_string()),
        };

        if self.config.verbose {
            self.print_summary(&result);
        }

        Ok(result)
    }

    #[allow(dead_code)]
    fn print_summary(&self, result: &DeepResearchResult) {
        let papers = result.papers.len();
        let gaps = result.gaps.len();
        let accepted = result.gaps.iter().filter(|g| g.accepted).count();
        let duration = result.duration_seconds;

        println!();
        println!("{}", "=".repeat(60));
        println!("  DeepResearch Complete — {}", result.status.to_uppercase());
        println!("  Iterations : {}", result.iterations);
        println!("  Papers     : {} found, {} extracted", papers, papers);
        println!("  Gaps       : {} found, {} accepted", gaps, accepted);
        println!("  Duration   : {:.1}s", duration);
        println!("{}", "=".repeat(60));
    }

    /// Request the agent to stop at next reflection point.
    #[allow(dead_code)]
    pub fn stop(&mut self) {
        self.stop_requested.store(true, Ordering::SeqCst);
        if let Some(ref mut session) = self.session {
            session.status = "paused".to_string();
        }
    }

    /// Get current session.
    #[allow(dead_code)]
    pub fn get_session(&self) -> Option<&ResearchSession> {
        self.session.as_ref()
    }

    /// Get thoughts recorded so far.
    #[allow(dead_code)]
    pub fn get_thoughts(&self) -> &[AgentThought] {
        &self.thoughts
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

fn generate_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("session_{}_{}", duration.as_secs(), duration.subsec_nanos())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_query_strategy_new() {
        let strategy = AdaptiveQueryStrategy::new("machine learning");
        assert_eq!(strategy.topic, "machine learning");
    }

    #[test]
    fn test_adaptive_query_strategy_gap_coverage() {
        let strategy = AdaptiveQueryStrategy::new("test");
        let coverage = strategy.gap_type_coverage();
        assert!(coverage.values().all(|&v| v == 0.0));
    }

    #[test]
    fn test_adaptive_query_strategy_under_represented() {
        let strategy = AdaptiveQueryStrategy::new("test");
        let under_rep = strategy.under_represented_types(0.15);
        // When no gaps, should be empty
        assert!(under_rep.is_empty());
    }

    #[test]
    fn test_query_similarity() {
        let strategy = AdaptiveQueryStrategy::new("test");
        let sim = strategy.query_similarity("hello world", "hello world");
        assert!((sim - 1.0).abs() < f64::EPSILON);

        let sim2 = strategy.query_similarity("hello world", "world hello");
        assert!((sim2 - 1.0).abs() < f64::EPSILON);

        let sim3 = strategy.query_similarity("hello", "world");
        assert!(sim3 < 0.5);
    }

    #[test]
    fn test_build_adaptive_query_iteration_0() {
        let strategy = AdaptiveQueryStrategy::new("machine learning");
        let query = strategy.build_adaptive_query(0, "", "", "", 0.0);
        assert_eq!(query, "machine learning");
    }

    #[test]
    fn test_build_adaptive_query_with_under_represented() {
        let mut strategy = AdaptiveQueryStrategy::new("machine learning");
        // Add some gaps to make capability under-represented
        strategy.record_search_result(
            "MLP training",
            &[GapSnapshot {
                gap_type: "improvement".to_string(),
                title: "test".to_string(),
                description: "".to_string(),
                matched_papers: vec![],
                archetype_match: 0.5,
                accepted: false,
            }],
        );
        let query = strategy.build_adaptive_query(1, "Neural architecture", "improvement", "", 0.0);
        assert!(query.contains("improvement"));
    }

    #[test]
    fn test_deep_research_agent_new() {
        let config = DeepResearchConfig {
            query: "test query".to_string(),
            max_iterations: 3,
            ..Default::default()
        };
        let agent = DeepResearchAgent::new(config);
        assert_eq!(agent.config.query, "test query");
        assert_eq!(agent.config.max_iterations, 3);
    }

    #[test]
    fn test_agent_thought_new() {
        let thought = AgentThought::new(1, "planner", "test content");
        assert_eq!(thought.iteration, 1);
        assert_eq!(thought.role, "planner");
        assert_eq!(thought.content, "test content");
    }

    #[test]
    fn test_generate_session_id() {
        let id1 = generate_session_id();
        let id2 = generate_session_id();
        assert_ne!(id1, id2);
        assert!(id1.starts_with("session_"));
    }

    #[test]
    fn test_gap_analyzer_new() {
        let analyzer = GapAnalyzerV2::new();
        let result = analyzer.analyze("test", false, 3, false);
        assert!(result.gaps.is_empty());
    }

    #[test]
    fn test_paper_snapshot_serde() {
        let snapshot = PaperSnapshot {
            arxiv_id: "2301.00001".to_string(),
            title: "Test Paper".to_string(),
            abstract_text: "Abstract".to_string(),
            url: "https://arxiv.org/abs/2301.00001".to_string(),
            extracted_text: "Full text".to_string(),
            gaps_found: 2,
        };
        let json = serde_json::to_string(&snapshot).unwrap();
        let deserialized: PaperSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.arxiv_id, snapshot.arxiv_id);
        assert_eq!(deserialized.title, snapshot.title);
    }

    #[test]
    fn test_gap_snapshot_serde() {
        let gap = GapSnapshot {
            gap_type: "improvement".to_string(),
            title: "Test Gap".to_string(),
            description: "Description".to_string(),
            matched_papers: vec!["2301.00001".to_string()],
            archetype_match: 0.7,
            accepted: true,
        };
        let json = serde_json::to_string(&gap).unwrap();
        let deserialized: GapSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.gap_type, gap.gap_type);
        assert_eq!(deserialized.archetype_match, gap.archetype_match);
    }
}
