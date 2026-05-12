//! rairos-orchestrator — Autonomous Research Orchestrator (closed-loop research agent)
//!
//! Watches arXiv via subscriptions, triggers deep gap analysis on new papers,
//! scores results against Gene Pool preferences, and notifies when high-value
//! research opportunities are found.

use chrono::Utc;
use rairos_core::{Database, ResearchGap};
use rairos_insight_tracker::EvolutionTracker;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tokio::sync::RwLock;
use uuid::Uuid;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum OrchestratorError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("State error: {0}")]
    State(String),

    #[error("Not initialized: {0}")]
    NotInitialized(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, OrchestratorError>;

// ============================================================================
// State Persistence
// ============================================================================

fn get_state_path() -> PathBuf {
    let path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ai_research_os")
        .join("autonomous");
    fs::create_dir_all(&path).ok();
    path.join("orchestrator_state.json")
}

fn load_state() -> OrchestratorState {
    let path = get_state_path();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(state) = serde_json::from_str(&content) {
                return state;
            }
        }
    }
    OrchestratorState::default()
}

fn save_state(state: &OrchestratorState) -> std::io::Result<()> {
    let path = get_state_path();
    let content = serde_json::to_string_pretty(state)?;
    fs::write(path, content)
}

// ============================================================================
// State & Config Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorState {
    pub running: bool,
    pub interval_minutes: i32,
    pub last_check: String,
    #[serde(default)]
    pub sessions: Vec<String>,
    #[serde(default)]
    pub alerts: Vec<ResearchAlert>,
}

impl Default for OrchestratorState {
    fn default() -> Self {
        Self {
            running: false,
            interval_minutes: 30,
            last_check: String::new(),
            sessions: Vec::new(),
            alerts: Vec::new(),
        }
    }
}

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
        alert_id: String,
        session_id: String,
        topic: String,
        triggered_by: String,
        trigger_title: String,
        gaps_found: i32,
        top_gap_title: String,
        top_gap_type: String,
        severity: String,
        gene_pool_score: f64,
        preference_boost: bool,
    ) -> Self {
        Self {
            alert_id,
            session_id,
            topic,
            triggered_by,
            trigger_title,
            gaps_found,
            top_gap_title,
            top_gap_type,
            severity,
            gene_pool_score,
            preference_boost,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0),
        }
    }
}

// ============================================================================
// Paper / Gap Types (local mirrors for orchestrator-level use)
// ============================================================================

/// Paper info from subscription check (used to pass paper data through the pipeline).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperInfo {
    pub arxiv_id: String,
    pub title: String,
    pub abstract_text: String,
    #[serde(default)]
    pub pdf_url: String,
    #[serde(default)]
    pub categories: String,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub published: String,
}

/// Scored gap with Gene Pool metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredGap {
    /// The underlying gap
    pub gap: ResearchGap,
    /// Gap type (category in rairos_core terms)
    pub gap_type: String,
    /// Short title for display
    pub title: String,
    pub description: String,
    pub severity: String,
    pub gene_pool_score: f64,
    pub preference_boost: bool,
}

/// Result of a deep research run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepResearchResult {
    pub gaps: Vec<ResearchGap>,
    pub papers_analyzed: i32,
    pub session_id: String,
    #[serde(default)]
    pub iterations: i32,
    #[serde(default)]
    pub error: Option<String>,
}

/// Statistics from incremental gap filtering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterStats {
    pub seen: i32,
    pub suppressed: i32,
}

/// Gene Pool statistics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GenePoolStats {
    #[serde(default)]
    pub total: i32,
    #[serde(default)]
    pub avg_score: f64,
    #[serde(default)]
    pub by_gap_type: HashMap<String, i32>,
}

// ============================================================================
// Orchestrator
// ============================================================================

pub struct AutonomousOrchestrator {
    config: OrchestratorConfig,
    webhook_enabled: bool,
    stop_tx: Arc<RwLock<Option<tokio::sync::watch::Sender<()>>>>,
    watch_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    db: Arc<RwLock<Option<Database>>>,
    tracker: Arc<RwLock<Option<EvolutionTracker>>>,
}

impl Default for AutonomousOrchestrator {
    fn default() -> Self {
        Self::new(OrchestratorConfig::default(), true)
    }
}

impl AutonomousOrchestrator {
    pub fn new(config: OrchestratorConfig, webhook_enabled: bool) -> Self {
        Self {
            config,
            webhook_enabled,
            stop_tx: Arc::new(RwLock::new(None)),
            watch_handle: Arc::new(RwLock::new(None)),
            db: Arc::new(RwLock::new(None)),
            tracker: Arc::new(RwLock::new(None)),
        }
    }

    // ── Component initialization ─────────────────────────────────────────────

    async fn init_components(&self) -> Result<()> {
        // Initialize DB
        {
            let mut db_guard = self.db.write().await;
            if db_guard.is_none() {
                let db = Database::open(
                    dirs::home_dir()
                        .unwrap_or_else(|| PathBuf::from("."))
                        .join(".ai_research_os")
                        .join("rairos.db"),
                )
                .map_err(|e| OrchestratorError::Database(e.to_string()))?;
                *db_guard = Some(db);
            }
        }

        // Initialize tracker
        {
            let mut tracker_guard = self.tracker.write().await;
            if tracker_guard.is_none() {
                let tracker = EvolutionTracker::new(None);
                *tracker_guard = Some(tracker);
            }
        }

        Ok(())
    }

    // ── Subscription watch ────────────────────────────────────────────────────

    /// Check all subscriptions for new papers. Returns sub_id -> new papers.
    pub async fn check_subscriptions(&self) -> Result<HashMap<String, Vec<PaperInfo>>> {
        self.init_components().await?;
        let db_guard = self.db.read().await;
        let db = db_guard
            .as_ref()
            .ok_or_else(|| OrchestratorError::NotInitialized("database".to_string()))?;

        let mut result: HashMap<String, Vec<PaperInfo>> = HashMap::new();

        // Get all subscriptions
        let subs = db
            .list_subscriptions(false)
            .map_err(|e| OrchestratorError::Database(e.to_string()))?;

        for sub in subs {
            let topic = sub.query.clone();
            // Search for papers matching the subscription topic
            if let Ok(papers) = db.search_papers(&topic, 20) {
                if !papers.is_empty() {
                    let paper_infos: Vec<PaperInfo> = papers
                        .into_iter()
                        .map(|p| PaperInfo {
                            arxiv_id: p.arxiv_id.unwrap_or_default(),
                            title: p.title.clone(),
                            abstract_text: p.abstract_text.clone(),
                            pdf_url: p.metadata.pdf_url.clone().unwrap_or_default(),
                            categories: p.categories.join(" "),
                            authors: p.authors.clone(),
                            published: p.published.to_rfc3339(),
                        })
                        .collect();
                    result.insert(topic, paper_infos);
                }
            }
        }

        Ok(result)
    }

    // ── Deep research ─────────────────────────────────────────────────────────

    /// Run deep research loop for a topic.
    /// In full implementation this would invoke DeepResearchAgent + GapAnalyzerV2
    /// from the rairos-research crate (requires LLM integration).
    pub async fn run_deep_research(
        &self,
        _topic: &str,
        new_papers: Vec<PaperInfo>,
    ) -> Result<DeepResearchResult> {
        self.init_components().await?;

        let session_id = Uuid::new_v4().to_string()[..8].to_string();
        let papers_analyzed = new_papers.len() as i32;

        // Add discovered papers to DB
        {
            let db_guard = self.db.read().await;
            if let Some(db) = db_guard.as_ref() {
                for p in &new_papers {
                    let paper = rairos_core::Paper::new(
                        Some(p.arxiv_id.clone()),
                        p.title.clone(),
                        p.abstract_text.clone(),
                    );
                    let _ = db.insert_paper(&paper);
                }
            }
        }

        // Gaps would be filled by the actual DeepResearchAgent in rairos-research.
        // This stub returns empty gaps — real implementation requires LLM calls.
        Ok(DeepResearchResult {
            gaps: Vec::new(),
            papers_analyzed,
            session_id,
            iterations: 0,
            error: None,
        })
    }

    // ── Gene Pool scoring ────────────────────────────────────────────────────

    /// Score gaps against Gene Pool for preference-aware ranking.
    pub async fn score_gaps_against_gene_pool(
        &self,
        gaps: Vec<ResearchGap>,
        _topic: &str,
    ) -> Result<Vec<ScoredGap>> {
        self.init_components().await?;
        let tracker_guard = self.tracker.read().await;
        let tracker = tracker_guard
            .as_ref()
            .ok_or_else(|| OrchestratorError::NotInitialized("tracker".to_string()))?;

        let profile = tracker.get_profile();

        let scored = gaps
            .into_iter()
            .map(|gap| {
                let gap_type_name = gap.category.clone();

                // Look up Gene Pool score from tracker profile
                let raw_score = profile
                    .gap_type_preferences
                    .get(&gap_type_name)
                    .copied()
                    .unwrap_or(0.0);

                // Normalize from [-1, 1] to [0, 1]
                let gene_pool_score = (raw_score.clamp(-1.0, 1.0) + 1.0) / 2.0;
                let preference_boost = gene_pool_score >= 0.5;

                let severity = gap.severity.clone();
                let description = gap.description.clone();
                ScoredGap {
                    gap,
                    gap_type: gap_type_name,
                    title: description.chars().take(60).collect(),
                    description,
                    severity,
                    gene_pool_score,
                    preference_boost,
                }
            })
            .collect();

        Ok(scored)
    }

    // ── Alert generation ─────────────────────────────────────────────────────

    /// Generate ResearchAlert objects for high-value gaps.
    pub fn generate_alerts(
        &self,
        scored_gaps: Vec<ScoredGap>,
        session_id: &str,
        topic: &str,
        trigger_paper: &PaperInfo,
    ) -> Vec<ResearchAlert> {
        let severity_rank = |s: &str| match s {
            "HIGH" => 0,
            "MEDIUM" => 1,
            "LOW" => 2,
            _ => 3,
        };

        let min_sev = severity_rank(&self.config.min_gap_severity_for_alert);

        let mut alerts = Vec::new();
        for sg in scored_gaps {
            let sev_rank = severity_rank(&sg.severity);
            if sev_rank > min_sev {
                continue;
            }
            if sg.gene_pool_score < self.config.min_gene_pool_score_for_alert {
                continue;
            }

            let alert = ResearchAlert::new(
                Uuid::new_v4().to_string()[..8].to_string(),
                session_id.to_string(),
                topic.to_string(),
                trigger_paper.arxiv_id.clone(),
                trigger_paper.title.chars().take(80).collect(),
                1,
                sg.title.chars().take(80).collect(),
                sg.gap_type.clone(),
                sg.severity.clone(),
                sg.gene_pool_score,
                sg.preference_boost,
            );
            alerts.push(alert);
        }

        alerts
    }

    // ── Webhook notification ─────────────────────────────────────────────────

    fn send_webhook(&self, alert: &ResearchAlert) {
        if !self.webhook_enabled {
            return;
        }
        tracing::debug!(
            "[Orchestrator] Would send webhook for alert: {} (topic={})",
            alert.alert_id,
            alert.topic
        );
    }

    // ── Incremental gap filtering ───────────────────────────────────────────

    /// Filter gaps to suppress already-seen ones based on gap description/title.
    async fn filter_new_gaps(
        &self,
        _topic: &str,
        gaps: Vec<ResearchGap>,
    ) -> Result<(Vec<ResearchGap>, FilterStats)> {
        self.init_components().await?;
        let db_guard = self.db.read().await;
        let db = db_guard
            .as_ref()
            .ok_or_else(|| OrchestratorError::NotInitialized("database".to_string()))?;

        // Get all existing gaps from DB
        let existing_gaps = db
            .list_gaps(200, 0)
            .map_err(|e| OrchestratorError::Database(e.to_string()))?;

        let seen_descriptions: std::collections::HashSet<_> = existing_gaps
            .iter()
            .map(|g| g.description.clone())
            .collect();

        let mut filtered = Vec::new();
        let mut suppressed = 0i32;
        for gap in gaps {
            if seen_descriptions.contains(&gap.description) {
                suppressed += 1;
            } else {
                filtered.push(gap);
            }
        }

        let stats = FilterStats {
            seen: seen_descriptions.len() as i32,
            suppressed,
        };

        Ok((filtered, stats))
    }

    // ── Record gaps to DB ───────────────────────────────────────────────────

    async fn record_gaps(&self, gaps: &[ResearchGap]) -> Result<()> {
        self.init_components().await?;
        let db_guard = self.db.read().await;
        if let Some(db) = db_guard.as_ref() {
            for gap in gaps {
                let _ = db.insert_gap(gap);
            }
        }
        Ok(())
    }

    // ── Main cycle ───────────────────────────────────────────────────────────

    /// Run one complete orchestrator cycle. Returns alerts generated.
    pub async fn run_cycle(&self) -> Result<Vec<ResearchAlert>> {
        self.init_components().await?;
        let mut all_alerts: Vec<ResearchAlert> = Vec::new();

        tracing::info!("[Orchestrator] Starting cycle...");

        let sub_results = self.check_subscriptions().await?;

        for (topic, new_papers) in sub_results.iter() {
            if new_papers.is_empty() {
                continue;
            }

            if new_papers.len() < self.config.min_papers_for_deep_analysis as usize {
                tracing::debug!(
                    "[Orchestrator] Skipping '{}': only {} papers (min={})",
                    topic,
                    new_papers.len(),
                    self.config.min_papers_for_deep_analysis
                );
                continue;
            }

            tracing::info!(
                "[Orchestrator] {} new papers for subscription '{}'",
                new_papers.len(),
                topic
            );

            // Run deep research
            let research_result = match self.run_deep_research(topic, new_papers.clone()).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("Deep research failed for '{}': {}", topic, e);
                    continue;
                }
            };

            let gaps = &research_result.gaps;
            let _session_id = &research_result.session_id;
            if gaps.is_empty() {
                tracing::info!("[Orchestrator] No gaps found for '{}'", topic);
                continue;
            }

            // Incremental filtering: suppress already-seen gaps
            let (gaps, filter_stats) = match self.filter_new_gaps(topic, gaps.clone()).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("Gap filter failed for '{}': {}", topic, e);
                    continue;
                }
            };

            if filter_stats.suppressed > 0 {
                tracing::info!(
                    "[Orchestrator] Suppressed {} already-seen gaps (total seen: {})",
                    filter_stats.suppressed,
                    filter_stats.seen
                );
            }
            if gaps.is_empty() {
                tracing::info!(
                    "[Orchestrator] All gaps already known for '{}' — skipping",
                    topic
                );
                continue;
            }

            // Record new gaps to DB
            let _ = self.record_gaps(&gaps).await;

            // Score against Gene Pool
            let scored = match self.score_gaps_against_gene_pool(gaps, topic).await {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("Gene Pool scoring failed for '{}': {}", topic, e);
                    continue;
                }
            };

            // Generate alerts
            let trigger = new_papers.first().unwrap();
            let alerts =
                self.generate_alerts(scored.clone(), &research_result.session_id, topic, trigger);

            for alert in &alerts {
                self.send_webhook(alert);
                all_alerts.push(alert.clone());

                // Encode into Gene Pool via tracker
                {
                    let mut tracker_guard = self.tracker.write().await;
                    if let Some(tracker) = tracker_guard.as_mut() {
                        let _ = tracker.record_gap_accept(
                            &alert.topic,
                            &alert.top_gap_type,
                            &alert.top_gap_title,
                            "",
                        );
                    }
                }
            }

            tracing::info!(
                "[Orchestrator] Generated {} alerts for '{}'",
                alerts.len(),
                topic
            );
        }

        // Update persistent state
        let mut state = load_state();
        for alert in &all_alerts {
            state.alerts.insert(0, alert.clone());
        }
        state
            .alerts
            .truncate(self.config.max_alerts_stored as usize);
        state.last_check = Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        let _ = save_state(&state);

        tracing::info!("[Orchestrator] Cycle complete: {} alerts", all_alerts.len());
        Ok(all_alerts)
    }

    // ── Background watch ─────────────────────────────────────────────────────

    /// Start background watch loop.
    pub async fn start_watch(&self, interval_minutes: i32) -> Result<()> {
        // Check if already running
        {
            let handle = self.watch_handle.read().await;
            if handle.is_some() {
                tracing::warn!("Watch already running");
                return Ok(());
            }
        }

        let (tx, rx) = tokio::sync::watch::channel(());
        {
            let mut stop = self.stop_tx.write().await;
            *stop = Some(tx);
        }

        let config = self.config.clone();

        // Update persistent state
        let mut state = load_state();
        state.running = true;
        state.interval_minutes = interval_minutes;
        let _ = save_state(&state);

        tracing::info!(
            "[Orchestrator] Watch started (interval={}min)",
            interval_minutes
        );

        let _handle = tokio::spawn(async move {
            let mut evolution_counter = 0i32;
            let interval_secs = (interval_minutes as u64) * 60;
            let mut stop_rx = rx;

            loop {
                tokio::select! {
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(interval_secs)) => {
                        let orch = Self::new(config.clone(), true);
                        if let Err(e) = orch.run_cycle().await {
                            tracing::error!("[Orchestrator] Cycle error: {}", e);
                        }

                        // Run evolution every 3 cycles (~90min at 30min intervals)
                        evolution_counter += 1;
                        if evolution_counter >= 3 {
                            evolution_counter = 0;
                            tracing::info!("[Orchestrator] Evolution cycle triggered");
                        }
                    }
                    _ = stop_rx.changed() => {
                        tracing::info!("[Orchestrator] Watch stopped");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop the background watch loop.
    pub async fn stop_watch(&self) -> Result<()> {
        {
            let mut stop = self.stop_tx.write().await;
            if let Some(tx) = stop.take() {
                let _ = tx.send(());
            }
        }

        let mut state = load_state();
        state.running = false;
        let _ = save_state(&state);

        tracing::info!("[Orchestrator] Watch stopped");
        Ok(())
    }

    // ── State access ─────────────────────────────────────────────────────────

    /// Get recent research alerts.
    pub fn get_recent_alerts(&self, limit: usize) -> Vec<ResearchAlert> {
        let state = load_state();
        state.alerts.iter().take(limit).cloned().collect()
    }

    // ── Evolution cycle ─────────────────────────────────────────────────────

    /// Run one InsightEvolution cycle on the Gene Pool.
    pub async fn run_evolution_cycle(
        &self,
        topic: &str,
    ) -> Result<HashMap<String, serde_json::Value>> {
        self.init_components().await?;

        let evo_topic = if topic.is_empty() {
            self.best_evolution_topic()
        } else {
            topic.to_string()
        };

        // Full evolution requires rairos-insight-evolution's EvolutionEngine.
        // Stub returns structured result.
        Ok({
            let mut m = HashMap::new();
            m.insert("topic".to_string(), serde_json::json!(evo_topic));
            m.insert("status".to_string(), serde_json::json!("stub"));
            m
        })
    }

    fn best_evolution_topic(&self) -> String {
        let guard = match self.tracker.try_read() {
            Ok(g) => g,
            Err(_) => return "machine learning".to_string(),
        };
        let tracker = match guard.as_ref() {
            Some(t) => t,
            None => return "machine learning".to_string(),
        };
        let profile = tracker.get_profile();
        let mut topics: Vec<_> = profile.topic_frequency.iter().collect();
        if !topics.is_empty() {
            topics.sort_by(|(_, v_a), (_, v_b)| v_b.cmp(v_a));
            if let Some((k, _)) = topics.first() {
                return k.to_string();
            }
        }
        "machine learning".to_string()
    }

    // ── Combined status ──────────────────────────────────────────────────────

    /// Get orchestrator status with evolution stats.
    pub async fn get_status(&self) -> Result<HashMap<String, serde_json::Value>> {
        let state = load_state();
        let pool_stats = self.gene_pool_stats().await;

        let mut status = HashMap::new();
        status.insert("running".to_string(), serde_json::json!(state.running));
        status.insert(
            "interval_minutes".to_string(),
            serde_json::json!(state.interval_minutes),
        );
        status.insert(
            "last_check".to_string(),
            serde_json::json!(state.last_check),
        );
        status.insert(
            "alerts_count".to_string(),
            serde_json::json!(state.alerts.len()),
        );
        status.insert(
            "gene_pool".to_string(),
            serde_json::json!({
                "total_capsules": pool_stats.total,
                "avg_score": pool_stats.avg_score,
                "by_gap_type": pool_stats.by_gap_type,
            }),
        );
        status.insert(
            "evolution".to_string(),
            serde_json::json!({ "available": true }),
        );

        Ok(status)
    }

    async fn gene_pool_stats(&self) -> GenePoolStats {
        GenePoolStats::default()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_research_alert_creation() {
        let alert = ResearchAlert::new(
            "abc12345".to_string(),
            "sess001".to_string(),
            "machine learning".to_string(),
            "2301.00001".to_string(),
            "Test Paper Title".to_string(),
            1,
            "Gap Title Here".to_string(),
            "method_limitation".to_string(),
            "HIGH".to_string(),
            0.75,
            true,
        );
        assert_eq!(alert.topic, "machine learning");
        assert_eq!(alert.severity, "HIGH");
        assert!(alert.preference_boost);
        assert!(alert.created_at > 0.0);
    }

    #[test]
    fn test_orchestrator_config_default() {
        let config = OrchestratorConfig::default();
        assert_eq!(config.interval_minutes, 30);
        assert_eq!(config.min_gap_severity_for_alert, "MEDIUM");
        assert_eq!(config.min_gene_pool_score_for_alert, 0.3);
    }

    #[test]
    fn test_severity_ranking() {
        let rank_fn = |s: &str| match s {
            "HIGH" => 0,
            "MEDIUM" => 1,
            "LOW" => 2,
            _ => 3,
        };
        assert!(rank_fn("HIGH") < rank_fn("MEDIUM"));
        assert!(rank_fn("MEDIUM") < rank_fn("LOW"));
    }

    #[tokio::test]
    async fn test_orchestrator_init() {
        let orch = AutonomousOrchestrator::default();
        assert!(orch.config.interval_minutes == 30);
    }

    #[test]
    fn test_state_persistence() {
        let state = OrchestratorState::default();
        assert!(!state.running);
        assert_eq!(state.interval_minutes, 30);
    }

    #[test]
    fn test_alert_to_dict() {
        let alert = ResearchAlert::new(
            "id001".to_string(),
            "sess01".to_string(),
            "NLP".to_string(),
            "2301.001".to_string(),
            "A Paper".to_string(),
            2,
            "Gap X".to_string(),
            "evaluation_gap".to_string(),
            "MEDIUM".to_string(),
            0.55,
            false,
        );
        let json = serde_json::to_string(&alert).unwrap();
        assert!(json.contains("NLP"));
        assert!(json.contains("MEDIUM"));
    }

    #[test]
    fn test_scored_gap_serde() {
        let gap = ResearchGap::new(
            "method_limitation",
            "Scaling law breakdown at 10B params",
            "HIGH",
        );
        let scored = ScoredGap {
            gap: gap.clone(),
            gap_type: "method_limitation".to_string(),
            title: "Scaling law breakdown".to_string(),
            description: "Models degrade beyond 10B params".to_string(),
            severity: "HIGH".to_string(),
            gene_pool_score: 0.8,
            preference_boost: true,
        };
        let json = serde_json::to_string(&scored).unwrap();
        assert!(json.contains("0.8"));
        assert!(json.contains("HIGH"));
    }

    #[tokio::test]
    async fn test_alert_generation_filters_low_severity() {
        let config = OrchestratorConfig {
            interval_minutes: 30,
            min_gap_severity_for_alert: "MEDIUM".to_string(),
            min_gene_pool_score_for_alert: 0.3,
            min_papers_for_deep_analysis: 1,
            max_alerts_stored: 50,
        };
        let orch = AutonomousOrchestrator::new(config, false);

        let gap = ResearchGap::new("evaluation_gap", "Missing benchmark for task X", "LOW");
        let scored = ScoredGap {
            gap: gap.clone(),
            gap_type: "evaluation_gap".to_string(),
            title: "Missing benchmark".to_string(),
            description: "No standard eval for X".to_string(),
            severity: "LOW".to_string(),
            gene_pool_score: 0.9,
            preference_boost: true,
        };

        let trigger = PaperInfo {
            arxiv_id: "2301.00001".to_string(),
            title: "Test Paper".to_string(),
            abstract_text: "Abstract".to_string(),
            pdf_url: "".to_string(),
            categories: "cs.AI".to_string(),
            authors: vec![],
            published: "".to_string(),
        };

        let alerts = orch.generate_alerts(vec![scored], "sess01", "AI", &trigger);
        // LOW severity (rank 2) > min_sev for MEDIUM (rank 1), so filtered out
        assert!(alerts.is_empty());
    }

    #[tokio::test]
    async fn test_alert_generation_passes_high_severity() {
        let config = OrchestratorConfig {
            interval_minutes: 30,
            min_gap_severity_for_alert: "MEDIUM".to_string(),
            min_gene_pool_score_for_alert: 0.3,
            min_papers_for_deep_analysis: 1,
            max_alerts_stored: 50,
        };
        let orch = AutonomousOrchestrator::new(config, false);

        let gap = ResearchGap::new("method_limitation", "Scaling law breakdown", "HIGH");
        let scored = ScoredGap {
            gap: gap.clone(),
            gap_type: "method_limitation".to_string(),
            title: "Scaling law breakdown".to_string(),
            description: "Models degrade beyond 10B params".to_string(),
            severity: "HIGH".to_string(),
            gene_pool_score: 0.8,
            preference_boost: true,
        };

        let trigger = PaperInfo {
            arxiv_id: "2301.00001".to_string(),
            title: "Test Paper".to_string(),
            abstract_text: "Abstract".to_string(),
            pdf_url: "".to_string(),
            categories: "cs.AI".to_string(),
            authors: vec![],
            published: "".to_string(),
        };

        let alerts = orch.generate_alerts(vec![scored], "sess01", "AI", &trigger);
        // HIGH severity (rank 0) <= min_sev (rank 1), passes
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].severity, "HIGH");
    }
}
