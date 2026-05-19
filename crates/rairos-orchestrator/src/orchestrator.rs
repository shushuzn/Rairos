use chrono::Utc;
use rairos_core::{Database, ResearchGap};
use rairos_llm::insight::tracker::EvolutionTracker;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::error::{OrchestratorError, Result};
use crate::persistence::{load_state, save_state};
use crate::state::{
    DeepResearchResult, FilterStats, GenePoolStats, OrchestratorConfig,
    PaperInfo, ResearchAlert, ScoredGap,
};

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

    async fn init_components(&self) -> Result<()> {
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

        {
            let mut tracker_guard = self.tracker.write().await;
            if tracker_guard.is_none() {
                let tracker = EvolutionTracker::new(None);
                *tracker_guard = Some(tracker);
            }
        }

        Ok(())
    }

    pub async fn check_subscriptions(&self) -> Result<HashMap<String, Vec<PaperInfo>>> {
        self.init_components().await?;

        let topics: Vec<(String, String)> = {
            let db_guard = self.db.read().await;
            let db = db_guard
                .as_ref()
                .ok_or_else(|| OrchestratorError::NotInitialized("database".to_string()))?;
            let subs = db
                .list_subscriptions(false)
                .map_err(|e| OrchestratorError::Database(e.to_string()))?;
            subs.into_iter().map(|s| (s.query.clone(), s.id.clone())).collect()
        };

        if topics.is_empty() {
            return Ok(HashMap::new());
        }

        let db_arc = Arc::clone(&self.db);

        use tokio::task;
        let search_results = task::spawn_blocking(move || {
            let db_guard = db_arc.blocking_read();
            let db = db_guard.as_ref().unwrap();
            let mut results = HashMap::new();
            for (topic, _sub_id) in &topics {
                if let Ok(papers) = db.search_papers_smart(topic, 20) {
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
                        results.insert(topic.clone(), paper_infos);
                    }
                }
            }
            results
        })
        .await
        .map_err(|e| OrchestratorError::Other(e.to_string()))?;

        Ok(search_results)
    }

    pub async fn run_deep_research(
        &self,
        _topic: &str,
        new_papers: Vec<PaperInfo>,
    ) -> Result<DeepResearchResult> {
        self.init_components().await?;

        let session_id = Uuid::new_v4().to_string()[..8].to_string();
        let papers_analyzed = new_papers.len() as i32;

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

        Ok(DeepResearchResult {
            gaps: Vec::new(),
            papers_analyzed,
            session_id,
            iterations: 0,
            error: None,
        })
    }

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

                let raw_score = profile
                    .gap_type_preferences
                    .get(&gap_type_name)
                    .copied()
                    .unwrap_or(0.0);

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

            let _ = self.record_gaps(&gaps).await;

            let scored = match self.score_gaps_against_gene_pool(gaps, topic).await {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("Gene Pool scoring failed for '{}': {}", topic, e);
                    continue;
                }
            };

            let Some(trigger) = new_papers.first().cloned() else {
                tracing::warn!("[Orchestrator] No papers in subscription '{}' despite prior check", topic);
                continue;
            };
            let alerts =
                self.generate_alerts(scored.clone(), &research_result.session_id, topic, &trigger);

            for alert in &alerts {
                self.send_webhook(alert);
                all_alerts.push(alert.clone());

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

    pub async fn start_watch(&self, interval_minutes: i32) -> Result<()> {
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

    pub fn get_recent_alerts(&self, limit: usize) -> Vec<ResearchAlert> {
        let state = load_state();
        state.alerts.iter().take(limit).cloned().collect()
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::OrchestratorState;

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
        let gap = ResearchGap::new_simple(
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

        let gap = ResearchGap::new_simple("evaluation_gap", "Missing benchmark for task X", "LOW");
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

        let gap = ResearchGap::new_simple("method_limitation", "Scaling law breakdown", "HIGH");
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
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].severity, "HIGH");
    }
}