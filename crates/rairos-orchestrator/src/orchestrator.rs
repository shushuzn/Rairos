use chrono::Utc;
use rairos_core::{Database, ResearchGap};
use rairos_crossover::CrossoverEngine;
use rairos_llm::insight::tracker::EvolutionTracker;
use rairos_llm::RegretOptimalSelector;
use rairos_rankers::{
    AdaptiveMomentum as RankerAdaptiveMomentum,
    BayesianOptimizer as RankerBayesianOptimizer, OptimalScalingLearner,
};
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

const STOP_WORDS: &[&str] = &[
    "about", "after", "also", "among", "approach", "areas", "based", "between",
    "certain", "could", "different", "does", "during", "each", "effects", "etc",
    "first", "following", "forms", "found", "from", "given", "however", "into",
    "many", "moreover", "most", "need", "neither", "only", "other", "paper",
    "papers", "problem", "problems", "proposed", "provide", "provides", "results",
    "same", "should", "since", "state", "states", "study", "such", "therefore",
    "these", "those", "through", "thus", "where", "which", "while", "within",
];

pub struct AutonomousOrchestrator {
    config: OrchestratorConfig,
    webhook_enabled: bool,
    stop_tx: Arc<RwLock<Option<tokio::sync::watch::Sender<()>>>>,
    watch_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    db: Arc<RwLock<Option<Database>>>,
    tracker: Arc<RwLock<Option<EvolutionTracker>>>,
    regret_selector: RegretOptimalSelector,
    bayesian_optimizer: RankerBayesianOptimizer,
    crossover_engine: CrossoverEngine,
    scaling_learner: OptimalScalingLearner,
    adaptive_momentum: RankerAdaptiveMomentum,
}

impl Default for AutonomousOrchestrator {
    fn default() -> Self {
        Self::new(OrchestratorConfig::default(), true)
    }
}

impl AutonomousOrchestrator {
    pub fn new(config: OrchestratorConfig, webhook_enabled: bool) -> Self {
        Self {
            config: config.clone(),
            webhook_enabled,
            stop_tx: Arc::new(RwLock::new(None)),
            watch_handle: Arc::new(RwLock::new(None)),
            db: Arc::new(RwLock::new(None)),
            tracker: Arc::new(RwLock::new(None)),
            regret_selector: RegretOptimalSelector::new(0.1),
            bayesian_optimizer: RankerBayesianOptimizer::new(
                vec![(0.1, 1.0), (0.1, 1.0), (1.0, 10.0)],
                1.0,
            ),
            crossover_engine: CrossoverEngine::new(1.0, 0.1),
            scaling_learner: OptimalScalingLearner::new(1.0, 1e-4),
            adaptive_momentum: RankerAdaptiveMomentum::new(0.9),
        }
    }

    pub fn record_gap_outcome(&mut self, gap_type: &str, score: f64) {
        self.regret_selector.record_outcome(gap_type, score);
    }

    pub fn apply_momentum_adjustment(&mut self, base_score: f64, gap_type: &str) -> f64 {
        let ucb_scores = self.regret_selector.get_ucb_scores();
        let ucb_bonus = ucb_scores.get(gap_type).copied().unwrap_or(0.0);
        let gradient = -(base_score - ucb_bonus);
        let momentum = self.adaptive_momentum.nesterov_update(gradient);
        (base_score + momentum * 0.1).clamp(0.0, 1.0)
    }

    pub fn get_adaptive_lr(&self, base_lr: f64, dataset_tokens: f64) -> f64 {
        let (lr, _) = self.scaling_learner.predict_optimal(1.0, dataset_tokens);
        self.adaptive_momentum.adaptive_lr(base_lr * lr, 1.0)
    }

    pub fn observe_scaling(&mut self, lr: f64, batch_size: f64, loss: f64) {
        self.scaling_learner.observe(lr, batch_size, loss);
    }

    pub fn suggest_next_gap_type(&mut self, gap_types: &[&str]) -> Option<String> {
        self.regret_selector.select(gap_types)
    }

    pub fn get_optimal_thresholds(&self, beta: f64) -> Vec<f64> {
        self.bayesian_optimizer.suggest(beta)
    }

    pub fn observe_threshold_performance(&mut self, thresholds: &[f64], alert_rate: f64) {
        self.bayesian_optimizer.observe(thresholds, alert_rate);
    }

    pub fn suggest_research_direction(&mut self, available_gap_types: &[&str]) -> Option<String> {
        self.regret_selector.select(available_gap_types)
    }

    pub fn get_regret_stats(&self) -> (usize, f64) {
        let total = self.regret_selector.total_selections();
        let empirical_best = self.regret_selector.empirical_best_score();
        (total, empirical_best)
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

            let rt = tokio::runtime::Handle::current();
            let mut handles = Vec::new();

            for (topic, _sub_id) in &topics {
                let topic_clone = topic.clone();
                let rt_clone = rt.clone();
                let handle = std::thread::spawn(move || {
                    let papers = rt_clone.block_on(async {
                        rairos_parser::search_arxiv_recent(&topic_clone, 20).await.unwrap_or_default()
                    });
                    (topic_clone, papers)
                });
                handles.push(handle);
            }

            for handle in handles {
                let (topic, papers) = handle.join().unwrap();

                if papers.is_empty() {
                    continue;
                }

                let existing_ids: std::collections::HashSet<_> = {
                    if let Ok(existing) = db.search_papers_smart(&topic, 100) {
                        existing.iter()
                            .filter_map(|p| p.arxiv_id.clone())
                            .collect()
                    } else {
                        std::collections::HashSet::new()
                    }
                };

                let similarity_threshold = 0.7;
                let existing_abstracts: Vec<String> = if let Ok(existing) = db.search_papers_smart(&topic, 50) {
                    existing.iter().map(|p| p.abstract_text.clone()).collect()
                } else {
                    Vec::new()
                };

                fn keyword_overlap(a: &str, b: &str) -> f64 {
                    let a_words: std::collections::HashSet<_> = a.split_whitespace()
                        .map(|w| w.to_lowercase())
                        .filter(|w| w.len() > 4)
                        .collect();
                    let b_words: std::collections::HashSet<_> = b.split_whitespace()
                        .map(|w| w.to_lowercase())
                        .filter(|w| w.len() > 4)
                        .collect();
                    if a_words.is_empty() || b_words.is_empty() {
                        return 0.0;
                    }
                    let intersection = a_words.intersection(&b_words).count() as f64;
                    let union = a_words.union(&b_words).count() as f64;
                    intersection / union
                }

                let new_papers: Vec<PaperInfo> = papers
                    .into_iter()
                    .filter_map(|p| {
                        let arxiv_id = p.arxiv_id.clone()?;
                        if existing_ids.contains(&arxiv_id) {
                            return None;
                        }

                        let is_too_similar = existing_abstracts.iter().any(|existing_abs| {
                            keyword_overlap(&p.abstract_text, existing_abs) > similarity_threshold
                        });
                        if is_too_similar {
                            return None;
                        }

                        Some(PaperInfo {
                            arxiv_id,
                            title: p.title.clone(),
                            abstract_text: p.abstract_text.clone(),
                            pdf_url: p.metadata.pdf_url.clone().unwrap_or_default(),
                            categories: p.categories.join(" "),
                            authors: p.authors.clone(),
                            published: p.published.to_rfc3339(),
                        })
                    })
                    .collect();

                if !new_papers.is_empty() {
                    results.insert(topic, new_papers);
                }
            }
            results
        })
        .await
        .map_err(|e| OrchestratorError::Other(e.to_string()))?;

        Ok(search_results)
    }

    pub async fn run_deep_research(
        &mut self,
        topic: &str,
        new_papers: Vec<PaperInfo>,
    ) -> Result<DeepResearchResult> {
        self.init_components().await?;

        let session_id = Uuid::new_v4().to_string()[..8].to_string();
        let papers_analyzed = new_papers.len() as i32;

        let mut all_categories: Vec<String> = Vec::new();
        let mut papers: Vec<rairos_core::Paper> = Vec::new();

        let max_papers_for_analysis = 10;
        let selected_papers: Vec<PaperInfo> = if new_papers.len() > max_papers_for_analysis {
            let mut scored: Vec<(&PaperInfo, f64)> = new_papers.iter().map(|p| {
                let abstract_len = p.abstract_text.len() as f64;
                let title_words = p.title.split_whitespace().count() as f64;
                let category_count = p.categories.split_whitespace().count() as f64;
                let author_count = p.authors.len() as f64;

                let recency_score = 1.0;
                let quality_score = (abstract_len / 500.0).min(2.0) * 0.3
                    + (title_words / 15.0).min(1.5) * 0.2
                    + (category_count / 3.0).min(1.5) * 0.2
                    + (author_count / 5.0).min(1.0) * 0.1;
                let relevance_score = abstract_len / 1000.0 + title_words / 20.0;
                let combined = recency_score * 0.3 + relevance_score * 0.4 + quality_score * 0.3;
                (p, combined)
            }).collect();
            scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Less));
            scored.into_iter().take(max_papers_for_analysis).map(|(p, _)| p.clone()).collect()
        } else {
            new_papers.clone()
        };

        {
            let db_guard = self.db.read().await;
            if let Some(db) = db_guard.as_ref() {
                for p in &selected_papers {
                    let paper = rairos_core::Paper::new(
                        Some(p.arxiv_id.clone()),
                        p.title.clone(),
                        p.abstract_text.clone(),
                    );
                    let _ = db.insert_paper(&paper);
                    papers.push(paper);

                    for cat in p.categories.split_whitespace() {
                        if !cat.is_empty() && !all_categories.contains(&cat.to_string()) {
                            all_categories.push(cat.to_string());
                        }
                    }
                }
            }
        }

        let keywords: Vec<&str> = all_categories.iter().map(|s| s.as_str()).collect();

        let _extracted_keywords: std::collections::HashSet<String> = {
            let mut term_freq: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            for paper in &papers {
                let words: std::collections::HashSet<_> = paper.abstract_text.split_whitespace()
                    .map(|w| w.to_lowercase())
                    .filter(|w| w.len() > 5)
                    .filter(|w| !STOP_WORDS.contains(&w.as_str()))
                    .collect();
                for word in words {
                    *term_freq.entry(word).or_insert(0) += 1;
                }
            }
            term_freq.into_iter()
                .filter(|(_, count)| *count >= 2)
                .map(|(word, _)| word)
                .collect()
        };

        let gap_descriptions = rairos_llm::GapDetector::detect_gaps(&papers, &keywords);
        let under_explored = rairos_llm::GapDetector::find_underexplored_areas(&papers, 3);

        let gap_types = &["unexplored_application", "scalability_issue", "evaluation_gap",
                          "method_limitation", "theoretical_gap", "reproducibility_gap"];
        let suggested_gap_type = self.regret_selector.select(gap_types);

        let existing_papers: Vec<String> = {
            let db_guard = self.db.read().await;
            if let Some(db) = db_guard.as_ref() {
                db.search_papers_smart(topic, 100)
                    .map(|p| p.into_iter().map(|p| p.abstract_text).collect())
                    .unwrap_or_default()
            } else {
                Vec::new()
            }
        };

        let novelty_threshold = 0.3;

        let mut gaps: Vec<ResearchGap> = Vec::new();

        for desc in gap_descriptions {
            let novelty = if existing_papers.is_empty() {
                1.0
            } else {
                let max_overlap = existing_papers.iter()
                    .map(|existing| {
                        let words_new: std::collections::HashSet<_> = desc.split_whitespace()
                            .map(|w| w.to_lowercase()).filter(|w| w.len() > 4).collect();
                        let words_exist: std::collections::HashSet<_> = existing.split_whitespace()
                            .map(|w| w.to_lowercase()).filter(|w| w.len() > 4).collect();
                        if words_new.is_empty() || words_exist.is_empty() {
                            return 0.0;
                        }
                        let intersection = words_new.intersection(&words_exist).count() as f64;
                        intersection / words_new.len() as f64
                    })
                    .fold(0.0f64, |a, b| a.max(b));
                1.0 - max_overlap
            };

            if novelty < novelty_threshold && !existing_papers.is_empty() {
                continue;
            }

            let severity = if novelty > 0.7 { "HIGH" } else if novelty > 0.4 { "MEDIUM" } else { "LOW" };
            let gap_type = suggested_gap_type.clone().unwrap_or_else(|| "keyword_gap".to_string());
            gaps.push(ResearchGap::new_simple(&gap_type, &desc, severity));
        }

        for cat in under_explored {
            let gap_type = suggested_gap_type.clone().unwrap_or_else(|| "category_gap".to_string());
            gaps.push(ResearchGap::new_simple(
                &gap_type,
                &format!("Under-explored category in {}: {}", topic, cat),
                "low",
            ));
        }

        tracing::info!("[DeepResearch] Detected {} gaps from {} papers for '{}'",
            gaps.len(), papers.len(), topic);

        Ok(DeepResearchResult {
            gaps,
            papers_analyzed,
            session_id,
            iterations: 1,
            error: None,
        })
    }

    pub async fn score_gaps_against_gene_pool(
        &mut self,
        gaps: Vec<ResearchGap>,
        _topic: &str,
    ) -> Result<Vec<ScoredGap>> {
        self.init_components().await?;
        let tracker_guard = self.tracker.read().await;
        let tracker = tracker_guard
            .as_ref()
            .ok_or_else(|| OrchestratorError::NotInitialized("tracker".to_string()))?;

        let profile = tracker.get_profile();
        let ucb_scores = self.regret_selector.get_ucb_scores();
        drop(tracker_guard);

        let mut scored: Vec<ScoredGap> = gaps
            .into_iter()
            .map(|gap| {
                let gap_type_name = gap.category.clone();

                let raw_score = profile
                    .gap_type_preferences
                    .get(&gap_type_name)
                    .copied()
                    .unwrap_or(0.0);

                let mut gene_pool_score = (raw_score.clamp(-1.0, 1.0) + 1.0) / 2.0;
                let preference_boost = gene_pool_score >= 0.5;

                if let Some(ucb) = ucb_scores.get(&gap_type_name) {
                    let ucb_normalized = (ucb / 10.0).min(1.0);
                    gene_pool_score = gene_pool_score * 0.7 + ucb_normalized * 0.3;
                }

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

        for sg in &mut scored {
            sg.gene_pool_score = self.apply_momentum_adjustment(sg.gene_pool_score, &sg.gap_type);
        }

        Ok(scored)
    }

    pub fn generate_alerts(
        &self,
        scored_gaps: Vec<ScoredGap>,
        session_id: &str,
        topic: &str,
        trigger_paper: &PaperInfo,
    ) -> (Vec<ResearchAlert>, f64, usize) {
        let severity_rank = |s: &str| match s {
            "HIGH" => 0,
            "MEDIUM" => 1,
            "LOW" => 2,
            _ => 3,
        };

        let min_sev = severity_rank(&self.config.min_gap_severity_for_alert);

        let suggested_thresholds = self.bayesian_optimizer.suggest(2.0);
        let bo_threshold = suggested_thresholds.first().copied().unwrap_or(0.3);
        let min_threshold = (self.config.min_gene_pool_score_for_alert as f64 * 0.7
            + bo_threshold * 0.3)
            .clamp(0.1, 0.9) as f64;

        let scored_len = scored_gaps.len();
        let mut alerts = Vec::new();
        for sg in scored_gaps {
            let sev_rank = severity_rank(&sg.severity);
            if sev_rank > min_sev {
                continue;
            }
            if sg.gene_pool_score < min_threshold {
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

        (alerts, min_threshold, scored_len)
    }

    pub fn record_threshold_feedback(&mut self, threshold: f64, alert_rate: f64) {
        self.bayesian_optimizer.observe(&[threshold], alert_rate);
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

    pub async fn run_cycle(&mut self) -> Result<Vec<ResearchAlert>> {
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
            let (alerts, threshold_used, scored_len) =
                self.generate_alerts(scored.clone(), &research_result.session_id, topic, &trigger);

            if !alerts.is_empty() {
                let alert_rate = alerts.len() as f64 / scored_len.max(1) as f64;
                self.observe_threshold_performance(&[threshold_used], alert_rate);

                let model_scale = new_papers.len() as f64;
                let dataset_tokens = new_papers.iter()
                    .map(|p| p.abstract_text.len() as f64)
                    .sum::<f64>();
                let opt_lr = self.get_adaptive_lr(0.1, dataset_tokens / model_scale);
                let opt_bs = opt_lr * 100.0;
                let loss = 1.0 - alert_rate;
                self.observe_scaling(opt_lr, opt_bs, loss);
            }

            for alert in &alerts {
                self.send_webhook(alert);
                all_alerts.push(alert.clone());
                self.record_gap_outcome(&alert.top_gap_type, alert.gene_pool_score);

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

        if self.config.run_evolution_in_cycle && !all_alerts.is_empty() {
            tracing::info!("[Orchestrator] Running evolution cycle...");
            if let Ok(_) = self.run_evolution_cycle("").await {
                tracing::info!("[Orchestrator] Evolution cycle complete");
            }
        }

        let high_quality_count = all_alerts.iter()
            .filter(|a| a.gene_pool_score > 0.6)
            .count();

        if self.config.adaptive_interval && high_quality_count > 0 {
            let interval = self.suggest_adaptive_interval();
            tracing::info!("[Orchestrator] Suggested adaptive interval: {} minutes", interval);
        }

        Ok(all_alerts)
    }

    pub fn suggest_adaptive_interval(&self) -> i32 {
        let state = load_state();
        let base_interval = self.config.interval_minutes;

        if state.alerts.is_empty() {
            return base_interval;
        }

        let recent_window = 10.min(state.alerts.len());
        let recent_alerts = &state.alerts[..recent_window];

        let high_quality: Vec<_> = recent_alerts.iter()
            .filter(|a| a.gene_pool_score > 0.6)
            .collect();

        if high_quality.is_empty() {
            return (base_interval as f64 * 1.5) as i32;
        }

        let avg_interval = if recent_alerts.len() > 1 {
            let intervals: Vec<i64> = recent_alerts.windows(2)
                .filter_map(|w| {
                    let t1 = chrono::DateTime::parse_from_rfc3339(&w[0].created_at.to_string()).ok()?;
                    let t0 = chrono::DateTime::parse_from_rfc3339(&w[1].created_at.to_string()).ok()?;
                    Some((t1 - t0).num_minutes())
                })
                .collect();
            if intervals.is_empty() {
                base_interval as i64
            } else {
                intervals.iter().sum::<i64>() / intervals.len() as i64
            }
        } else {
            base_interval as i64
        };

        let quality_ratio = high_quality.len() as f64 / recent_window as f64;
        let adjusted = (avg_interval as f64 * quality_ratio).max(5.0) as i32;
        adjusted.min(240).max(5)
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
                        let mut orch = Self::new(config.clone(), true);
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
        &mut self,
        topic: &str,
    ) -> Result<HashMap<String, serde_json::Value>> {
        self.init_components().await?;

        let evo_topic = if topic.is_empty() {
            self.best_evolution_topic()
        } else {
            topic.to_string()
        };

        let gap_types: Vec<String> = self.regret_selector.get_ucb_scores().keys().cloned().collect();
        let gap_type_refs: Vec<&str> = gap_types.iter().map(|s| s.as_str()).collect();

        if gap_type_refs.len() >= 2 {
            self.regret_selector.select(&gap_type_refs);
        }

        let result = rairos_crossover::run_evolution_with_engine(
            3,
            10,
            Some(&mut self.crossover_engine),
        );

        if let Some(created) = result.get("created").and_then(|v| v.as_array()) {
            let offspring_count = created.len();
            if offspring_count > 0 {
                let avg_fitness: f64 = created.iter()
                    .filter_map(|v| {
                        let f_a = v.get("fitness_a").and_then(|f| f.as_f64()).unwrap_or(0.5);
                        let f_b = v.get("fitness_b").and_then(|f| f.as_f64()).unwrap_or(0.5);
                        Some((f_a + f_b) / 2.0)
                    })
                    .sum::<f64>() / offspring_count as f64;

                if let (Some(p_a), Some(p_b)) = (
                    result.get("parent_a_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    result.get("parent_b_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
                ) {
                    self.crossover_engine.record_comparison(&p_a, &p_b, avg_fitness);
                }
            }
        }

        let mut output = HashMap::new();
        output.insert("topic".to_string(), serde_json::json!(evo_topic));
        for (k, v) in result {
            output.insert(k, v);
        }
        Ok(output)
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