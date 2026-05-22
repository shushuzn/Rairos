use chrono::{DateTime, Utc};
use futures::future::join_all;
use rairos_core::{Database, Paper, ResearchGap};
use rairos_crossover::CrossoverEngine;
use rairos_llm::insight::tracker::EvolutionTracker;
use rairos_llm::RegretOptimalSelector;
use rairos_rankers::{
    AdaptiveMomentum as RankerAdaptiveMomentum,
    BayesianOptimizer as RankerBayesianOptimizer, OptimalScalingLearner,
};
use rustc_hash::{FxHashMap, FxHashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::error::{OrchestratorError, Result};
use crate::persistence::{load_state, save_state};
use crate::state::{
    DeepResearchResult, FilterStats, GenePoolStats, OrchestratorConfig,
    OrchestratorState, PaperInfo, ResearchAlert, ScoredGap,
};

/// Cache for precomputed lowercase abstracts to avoid repeated to_lowercase() calls.
/// Each paper's abstract is lowercased once and cached here.
#[derive(Clone)]
struct PaperCache {
    /// Precomputed lowercase abstracts, indexed to match paper indices
    lowercase_abstracts: Vec<String>,
}

impl PaperCache {
    fn new(papers: &[rairos_core::Paper]) -> Self {
        let lowercase_abstracts = papers
            .iter()
            .map(|p| p.abstract_text.to_lowercase())
            .collect();
        Self { lowercase_abstracts }
    }

    #[inline(always)]
    fn get_lowercase(&self, idx: usize) -> &str {
        &self.lowercase_abstracts[idx]
    }
}

/// Maximum interval in seconds before cache is considered stale and reloaded from disk.
const MAX_PERSIST_INTERVAL_SECS: i64 = 60;

/// Compute keyword overlap between two text strings (Jaccard similarity on words > 4 chars).
#[inline(always)]
fn keyword_overlap(a: &str, b: &str) -> f64 {
    let a_words = compute_word_set(a);
    let b_words = compute_word_set(b);
    jaccard_similarity(&a_words, &b_words)
}

/// Compute word set from text (lowercase, filter words > 4 chars).
#[inline(always)]
fn compute_word_set(text: &str) -> FxHashSet<String> {
    text.split_whitespace()
        .map(|w| w.to_lowercase())
        .filter(|w| w.len() > 4)
        .collect()
}

/// Compute Jaccard similarity between two word sets.
#[inline(always)]
fn jaccard_similarity(a: &FxHashSet<String>, b: &FxHashSet<String>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let intersection = a.intersection(b).count() as f64;
    let union = a.union(b).count() as f64;
    intersection / union
}

const STOP_WORDS: &[&str] = &[
    "about", "after", "also", "among", "approach", "areas", "based", "between",
    "certain", "could", "different", "does", "during", "each", "effects", "etc",
    "first", "following", "forms", "found", "from", "given", "however", "into",
    "many", "moreover", "most", "need", "neither", "only", "other", "paper",
    "papers", "problem", "problems", "proposed", "provide", "provides", "results",
    "same", "should", "since", "state", "states", "study", "such", "therefore",
    "these", "those", "through", "thus", "where", "which", "while", "within",
];

/// Process a single topic's new papers and return generated alerts.
/// This function inlines the logic from run_deep_research, filter_new_gaps,
/// score_gaps_against_gene_pool, generate_alerts, record_gaps, and record_gap_outcome.
async fn process_topic(
    db: Arc<RwLock<Option<Database>>>,
    tracker: Arc<RwLock<Option<EvolutionTracker>>>,
    regret_selector: Arc<StdMutex<RegretOptimalSelector>>,
    bayesian_optimizer: Arc<StdMutex<RankerBayesianOptimizer>>,
    _crossover_engine: Arc<StdMutex<CrossoverEngine>>,
    scaling_learner: Arc<StdMutex<OptimalScalingLearner>>,
    adaptive_momentum: Arc<StdMutex<RankerAdaptiveMomentum>>,
    config: OrchestratorConfig,
    webhook_enabled: bool,
    topic: String,
    new_papers: Vec<PaperInfo>,
) -> Result<Vec<ResearchAlert>> {
    // Initialize components
    {
        let mut db_guard = db.write().await;
        if db_guard.is_none() {
            let database = Database::open(
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".ai_research_os")
                    .join("rairos.db"),
            )
            .map_err(|e| OrchestratorError::Database(e.to_string()))?;
            *db_guard = Some(database);
        }
    }
    {
        let mut tracker_guard = tracker.write().await;
        if tracker_guard.is_none() {
            let evo_tracker = EvolutionTracker::new(None);
            *tracker_guard = Some(evo_tracker);
        }
    }

    let session_id = Uuid::new_v4().to_string()[..8].to_string();
    let papers_analyzed = new_papers.len() as i32;

    let mut all_categories: FxHashSet<String> = FxHashSet::default();
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
        let db_guard = db.read().await;
        if let Some(db_ref) = db_guard.as_ref() {
            for p in &selected_papers {
                let paper = rairos_core::Paper::new(
                    Some(p.arxiv_id.clone()),
                    p.title.clone(),
                    p.abstract_text.clone(),
                );
                let _ = db_ref.insert_paper(&paper);
                papers.push(paper);

                for cat in p.categories.split_whitespace() {
                    if !cat.is_empty() {
                        all_categories.insert(cat.to_string());
                    }
                }
            }
        }
    }

    let keywords: Vec<&str> = all_categories.iter().map(|s| s.as_str()).collect();

    let _extracted_keywords: FxHashSet<String> = {
        let mut term_freq: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for paper in &papers {
            let words: FxHashSet<_> = paper.abstract_text.split_whitespace()
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
    let suggested_gap_type = {
        let mut selector = regret_selector.lock().unwrap();
        selector.select(gap_types)
    };

    let existing_papers: Vec<String> = {
        let db_guard = db.read().await;
        if let Some(db_ref) = db_guard.as_ref() {
            db_ref.search_papers_smart(&topic, 100)
                .map(|p| p.into_iter().map(|p| p.abstract_text).collect())
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    };

    // Pre-compute lowercase abstracts to avoid repeated to_lowercase() calls
    let cache = PaperCache::new(&papers);

    // Wrap papers and cache in Arc for cheap cloning in parallel gap detection
    let papers = Arc::new(papers);
    let cache = Arc::new(cache);

    let novelty_threshold = 0.3;

    // Parallelize gap detection - all 7 detectors are CPU-bound and independent
    let topic_owned = topic.clone();
    let gap_results = futures::future::join_all(vec![
        {
            let papers = Arc::clone(&papers);
            let cache = Arc::clone(&cache);
            let topic = topic_owned.clone();
            tokio::task::spawn_blocking(move || detect_pattern_gaps_impl(&*papers, &*cache, &topic))
        },
        {
            let papers = Arc::clone(&papers);
            let cache = Arc::clone(&cache);
            let topic = topic_owned.clone();
            tokio::task::spawn_blocking(move || detect_cross_paper_gaps_impl(&*papers, &*cache, &topic))
        },
        {
            let papers = Arc::clone(&papers);
            let cache = Arc::clone(&cache);
            tokio::task::spawn_blocking(move || detect_method_limitations_impl(&*papers, &*cache))
        },
        {
            let papers = Arc::clone(&papers);
            let cache = Arc::clone(&cache);
            tokio::task::spawn_blocking(move || detect_evaluation_gaps_impl(&*papers, &*cache))
        },
        {
            let papers = Arc::clone(&papers);
            let cache = Arc::clone(&cache);
            tokio::task::spawn_blocking(move || detect_resource_gaps_impl(&*papers, &*cache))
        },
        {
            let papers = Arc::clone(&papers);
            let cache = Arc::clone(&cache);
            tokio::task::spawn_blocking(move || detect_dataset_gaps_impl(&*papers, &*cache))
        },
        {
            let papers = Arc::clone(&papers);
            let cache = Arc::clone(&cache);
            tokio::task::spawn_blocking(move || detect_generalization_gaps_impl(&*papers, &*cache))
        },
    ]).await;
    let mut gaps: Vec<ResearchGap> = Vec::new();
    for result in gap_results {
        match result {
            Ok(g) => gaps.extend(g),
            Err(e) => tracing::warn!("Gap detection task panicked: {}", e),
        }
    }

    for desc in gap_descriptions {
        let novelty = if existing_papers.is_empty() {
            1.0
        } else {
            let desc_lower = desc.to_lowercase();
            let words_new: FxHashSet<String> = desc_lower.split_whitespace()
                .filter(|w| w.len() > 4)
                .map(|w| w.to_string())
                .collect();
            let max_overlap = existing_papers.iter()
                .map(|existing| {
                    let words_exist: FxHashSet<String> = existing.split_whitespace()
                        .map(|w| w.to_lowercase()).filter(|w| w.len() > 4)
                        .map(|w| w.to_string())
                        .collect();
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

    let research_result = DeepResearchResult {
        gaps,
        papers_analyzed,
        session_id,
        iterations: 1,
        error: None,
    };

    let gaps = &research_result.gaps;
    let session_id = &research_result.session_id;
    if gaps.is_empty() {
        return Ok(Vec::new());
    }

    // Filter new gaps
    let db_guard = db.read().await;
    let db_ref = db_guard
        .as_ref()
        .ok_or_else(|| OrchestratorError::NotInitialized("database".to_string()))?;

    let existing_gaps = db_ref
        .list_gaps(200, 0)
        .map_err(|e| OrchestratorError::Database(e.to_string()))?;

    let seen_descriptions: FxHashSet<_> = existing_gaps
        .iter()
        .map(|g| g.description.clone())
        .collect();

    let mut filtered = Vec::new();
    let mut suppressed = 0i32;
    for gap in gaps.iter().cloned() {
        if seen_descriptions.contains(&gap.description) {
            suppressed += 1;
        } else {
            filtered.push(gap);
        }
    }

    let filter_stats = FilterStats {
        seen: seen_descriptions.len() as i32,
        suppressed,
    };

    drop(db_guard);

    if filter_stats.suppressed > 0 {
        tracing::info!(
            "[Orchestrator] Suppressed {} already-seen gaps (total seen: {})",
            filter_stats.suppressed,
            filter_stats.seen
        );
    }
    if filtered.is_empty() {
        tracing::info!(
            "[Orchestrator] All gaps already known for '{}' — skipping",
            topic
        );
        return Ok(Vec::new());
    }

    // Record gaps
    let db_guard = db.read().await;
    if let Some(db_ref) = db_guard.as_ref() {
        for gap in &filtered {
            let _ = db_ref.insert_gap(gap);
        }
    }
    drop(db_guard);

    // Score gaps against gene pool
    let tracker_guard = tracker.read().await;
    let tracker_ref = tracker_guard
        .as_ref()
        .ok_or_else(|| OrchestratorError::NotInitialized("tracker".to_string()))?;

    let profile = tracker_ref.get_profile();
    let ucb_scores = {
        let selector = regret_selector.lock().unwrap();
        selector.get_ucb_scores()
    };
    drop(tracker_guard);

    let mut scored: Vec<ScoredGap> = filtered
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

    // Pre-compute gradients for all items
    let gradients: Vec<f64> = scored.iter()
        .map(|sg| {
            let ucb_bonus = ucb_scores.get(&sg.gap_type).copied().unwrap_or(0.0);
            -(sg.gene_pool_score - ucb_bonus)
        })
        .collect();

    // Batch momentum update - single lock acquisition (was: N lock acquisitions per iteration)
    let momenta: Vec<f64> = {
        let mut am = adaptive_momentum.lock().unwrap();
        gradients.iter().map(|&g| am.nesterov_update(g)).collect()
    };

    // Apply updates without locks
    for (sg, &momentum) in scored.iter_mut().zip(momenta.iter()) {
        sg.gene_pool_score = (sg.gene_pool_score + momentum * 0.1).clamp(0.0, 1.0);
    }

    let Some(trigger) = new_papers.first().cloned() else {
        tracing::warn!("[Orchestrator] No papers in subscription '{}' despite prior check", topic);
        return Ok(Vec::new());
    };

    let (alerts, threshold_used, scored_len) = {
        let severity_rank = |s: &str| match s {
            "HIGH" => 0,
            "MEDIUM" => 1,
            "LOW" => 2,
            _ => 3,
        };

        let min_sev = severity_rank(&config.min_gap_severity_for_alert);

        let suggested_thresholds = {
            let bo = bayesian_optimizer.lock().unwrap();
            bo.suggest(2.0)
        };
        let bo_threshold = suggested_thresholds.first().copied().unwrap_or(0.3);
        let min_threshold = (config.min_gene_pool_score_for_alert * 0.7
            + bo_threshold * 0.3)
            .clamp(0.1, 0.9);

        let scored_len = scored.len();
        let mut alerts = Vec::new();
        for sg in scored {
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
                trigger.arxiv_id.clone(),
                trigger.title.chars().take(80).collect(),
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
    };

    if !alerts.is_empty() {
        let alert_rate = alerts.len() as f64 / scored_len.max(1) as f64;
        {
            let mut bo = bayesian_optimizer.lock().unwrap();
            bo.observe(&[threshold_used], alert_rate);
        }

        let model_scale = new_papers.len() as f64;
        let dataset_tokens = new_papers.iter()
            .map(|p| p.abstract_text.len() as f64)
            .sum::<f64>();
        // Batch scaling_learner operations: predict_optimal and observe share data (lr -> opt_lr -> observe)
        let (opt_lr, opt_bs, loss) = {
            let (lr, _) = {
                let sl = scaling_learner.lock().unwrap();
                sl.predict_optimal(1.0, dataset_tokens / model_scale)
            };
            let opt_lr = {
                let am = adaptive_momentum.lock().unwrap();
                am.adaptive_lr(0.1 * lr, 1.0)
            };
            let opt_bs = opt_lr * 100.0;
            let loss = 1.0 - alert_rate;
            (opt_lr, opt_bs, loss)
        };
        {
            let mut sl = scaling_learner.lock().unwrap();
            sl.observe(opt_lr, opt_bs, loss);
        }
    }

    for alert in &alerts {
        {
            let mut selector = regret_selector.lock().unwrap();
            selector.record_outcome(&alert.top_gap_type, alert.gene_pool_score);
        }

        if webhook_enabled {
            tracing::debug!(
                "[Orchestrator] Would send webhook for alert: {} (topic={})",
                alert.alert_id,
                alert.topic
            );
        }

        {
            let mut tracker_guard = tracker.write().await;
            if let Some(t) = tracker_guard.as_mut() {
                let _ = t.record_gap_accept(
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

    Ok(alerts)
}

// Detection helper implementations (extract logic from Orchestrator methods)

struct LimitationPattern<'a> {
    pattern: &'a str,
    pattern_lower: &'a str,
    gap_type: &'a str,
    severity: &'a str,
}

fn limitation_patterns() -> [LimitationPattern<'static>; 33] {
    [
        LimitationPattern { pattern: "does not scale", pattern_lower: "does not scale", gap_type: "scalability_gap", severity: "HIGH" },
        LimitationPattern { pattern: "not scalable", pattern_lower: "not scalable", gap_type: "scalability_gap", severity: "HIGH" },
        LimitationPattern { pattern: "limited to", pattern_lower: "limited to", gap_type: "scalability_gap", severity: "MEDIUM" },
        LimitationPattern { pattern: "computational cost", pattern_lower: "computational cost", gap_type: "scalability_gap", severity: "MEDIUM" },
        LimitationPattern { pattern: "quadratic", pattern_lower: "quadratic", gap_type: "scalability_gap", severity: "MEDIUM" },
        LimitationPattern { pattern: "attention bottleneck", pattern_lower: "attention bottleneck", gap_type: "scalability_gap", severity: "HIGH" },
        LimitationPattern { pattern: "no benchmark", pattern_lower: "no benchmark", gap_type: "evaluation_gap", severity: "HIGH" },
        LimitationPattern { pattern: "unevaluated", pattern_lower: "unevaluated", gap_type: "evaluation_gap", severity: "MEDIUM" },
        LimitationPattern { pattern: "not evaluated on", pattern_lower: "not evaluated on", gap_type: "evaluation_gap", severity: "MEDIUM" },
        LimitationPattern { pattern: "missing evaluation", pattern_lower: "missing evaluation", gap_type: "evaluation_gap", severity: "MEDIUM" },
        LimitationPattern { pattern: "metric", pattern_lower: "metric", gap_type: "evaluation_gap", severity: "LOW" },
        LimitationPattern { pattern: "reproducibility", pattern_lower: "reproducibility", gap_type: "reproducibility_gap", severity: "HIGH" },
        LimitationPattern { pattern: "future work", pattern_lower: "future work", gap_type: "method_limitation", severity: "LOW" },
        LimitationPattern { pattern: "limitation", pattern_lower: "limitation", gap_type: "method_limitation", severity: "MEDIUM" },
        LimitationPattern { pattern: "cannot handle", pattern_lower: "cannot handle", gap_type: "method_limitation", severity: "MEDIUM" },
        LimitationPattern { pattern: "restricted to", pattern_lower: "restricted to", gap_type: "method_limitation", severity: "LOW" },
        LimitationPattern { pattern: "only works for", pattern_lower: "only works for", gap_type: "method_limitation", severity: "MEDIUM" },
        LimitationPattern { pattern: "fail to", pattern_lower: "fail to", gap_type: "method_limitation", severity: "MEDIUM" },
        LimitationPattern { pattern: "theoretical gap", pattern_lower: "theoretical gap", gap_type: "theoretical_gap", severity: "HIGH" },
        LimitationPattern { pattern: "not theoretically", pattern_lower: "not theoretically", gap_type: "theoretical_gap", severity: "MEDIUM" },
        LimitationPattern { pattern: "no proof", pattern_lower: "no proof", gap_type: "theoretical_gap", severity: "HIGH" },
        LimitationPattern { pattern: "lacks theory", pattern_lower: "lacks theory", gap_type: "theoretical_gap", severity: "MEDIUM" },
        LimitationPattern { pattern: "empirical only", pattern_lower: "empirical only", gap_type: "theoretical_gap", severity: "MEDIUM" },
        LimitationPattern { pattern: "unexplored", pattern_lower: "unexplored", gap_type: "unexplored_application", severity: "HIGH" },
        LimitationPattern { pattern: "not applied to", pattern_lower: "not applied to", gap_type: "unexplored_application", severity: "MEDIUM" },
        LimitationPattern { pattern: "novel application", pattern_lower: "novel application", gap_type: "unexplored_application", severity: "LOW" },
        LimitationPattern { pattern: "memory bottleneck", pattern_lower: "memory bottleneck", gap_type: "memory_gap", severity: "HIGH" },
        LimitationPattern { pattern: "context length", pattern_lower: "context length", gap_type: "context_gap", severity: "HIGH" },
        LimitationPattern { pattern: "long-context", pattern_lower: "long-context", gap_type: "context_gap", severity: "HIGH" },
        LimitationPattern { pattern: "interpretability", pattern_lower: "interpretability", gap_type: "interpretability_gap", severity: "HIGH" },
        LimitationPattern { pattern: "internal representation", pattern_lower: "internal representation", gap_type: "interpretability_gap", severity: "MEDIUM" },
        LimitationPattern { pattern: "feature extraction", pattern_lower: "feature extraction", gap_type: "feature_gap", severity: "MEDIUM" },
        LimitationPattern { pattern: "representation learning", pattern_lower: "representation learning", gap_type: "feature_gap", severity: "MEDIUM" },
    ]
}

fn detect_pattern_gaps_impl(papers: &[rairos_core::Paper], cache: &PaperCache, topic: &str) -> Vec<ResearchGap> {
    let limitation_patterns = limitation_patterns();
    let mut detected: Vec<ResearchGap> = Vec::new();
    let mut seen_patterns: FxHashSet<String> = FxHashSet::default();

    for (idx, paper) in papers.iter().enumerate() {
        let text_lower = cache.get_lowercase(idx);
        for lp in &limitation_patterns {
            if text_lower.contains(lp.pattern_lower) {
                let desc = format!("Gap in '{}': {} (found in {})", topic, lp.pattern, paper.title.chars().take(40).collect::<String>());
                if !seen_patterns.contains(lp.pattern) {
                    seen_patterns.insert(lp.pattern.to_string());
                    detected.push(ResearchGap::new_simple(lp.gap_type, &desc, lp.severity));
                }
            }
        }
    }

    detected
}

fn extract_key_phrases_impl(papers: &[rairos_core::Paper], cache: &PaperCache) -> std::collections::HashMap<String, Vec<String>> {
    let mut phrase_papers: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    let significant_bigrams = [
        "sparse autoencoder", "attention mechanism", "variational autoencoder",
        "large language", "neural network", "deep learning", "reinforcement learning",
        "machine translation", "object detection", "semantic segmentation",
        "transformer architecture", "pre-trained model", "foundation model",
        "representation learning", "self-supervised learning", "contrastive learning",
    ];

    for (idx, paper) in papers.iter().enumerate() {
        let text_lower = cache.get_lowercase(idx);
        for bigram in significant_bigrams {
            if text_lower.contains(bigram) {
                phrase_papers
                    .entry(bigram.to_string())
                    .or_default()
                    .push(paper.title.clone());
            }
        }
    }

    phrase_papers
}

fn detect_cross_paper_gaps_impl(papers: &[rairos_core::Paper], cache: &PaperCache, topic: &str) -> Vec<ResearchGap> {
    let phrase_map = extract_key_phrases_impl(papers, cache);
    let mut gaps: Vec<ResearchGap> = Vec::new();

    for (phrase, titles) in phrase_map {
        if titles.len() >= 2 {
            let gap_type = if phrase.contains("sparse") || phrase.contains("autoencoder")
                || phrase.contains("attention") || phrase.contains("transformer") {
                "architecture_gap"
            } else if phrase.contains("variational") || phrase.contains("representation") {
                "learning_gap"
            } else if phrase.contains("language") || phrase.contains("translation") {
                "application_gap"
            } else {
                "method_gap"
            };

            gaps.push(ResearchGap::new_simple(
                gap_type,
                &format!("Gap in '{}': Multiple papers on '{}' but no unified approach ({} papers)",
                    topic, phrase, titles.len()),
                "MEDIUM",
            ));
        }
    }

    gaps
}

fn detect_method_limitations_impl(papers: &[rairos_core::Paper], cache: &PaperCache) -> Vec<ResearchGap> {
    let limitation_phrases = [
        ("limited by", "scalability_gap", "HIGH", 2),
        ("struggle with", "method_limitation", "MEDIUM", 2),
        ("inefficient", "efficiency_gap", "HIGH", 2),
        ("suboptimal", "efficiency_gap", "MEDIUM", 2),
        ("no theoretical guarantee", "theoretical_gap", "HIGH", 3),
        ("lack of theoretical", "theoretical_gap", "HIGH", 3),
        ("empirical only", "theoretical_gap", "MEDIUM", 2),
        ("not robust to", "robustness_gap", "HIGH", 2),
        ("sensitive to", "robustness_gap", "MEDIUM", 2),
        ("breaks down", "robustness_gap", "HIGH", 2),
        ("collapses", "training_gap", "HIGH", 2),
        ("fails to converge", "training_gap", "HIGH", 2),
        ("gradient", "training_gap", "MEDIUM", 2),
        ("vanishing", "training_gap", "MEDIUM", 2),
        ("exploding", "training_gap", "MEDIUM", 2),
    ];

    let mut gaps: Vec<ResearchGap> = Vec::new();
    let mut seen: FxHashSet<String> = FxHashSet::default();

    for (idx, paper) in papers.iter().enumerate() {
        let text_lower = cache.get_lowercase(idx);
        for (phrase, gap_type, severity, _min_words) in &limitation_phrases {
            if text_lower.contains(phrase) {
                let title_short = paper.title.chars().take(30).collect::<String>();
                let key = format!("{}:{}", phrase, title_short);
                if !seen.contains(&key) {
                    seen.insert(key);
                    gaps.push(ResearchGap::new_simple(
                        gap_type,
                        &format!("Method limitation: '{}' in paper '{}'", phrase, title_short),
                        severity,
                    ));
                }
            }
        }
    }

    gaps
}

fn detect_evaluation_gaps_impl(papers: &[rairos_core::Paper], cache: &PaperCache) -> Vec<ResearchGap> {
    let eval_patterns = [
        ("no baseline", "evaluation_gap", "HIGH"),
        ("without comparison", "evaluation_gap", "HIGH"),
        ("compare to", "evaluation_gap", "LOW"),
        ("outperforms", "evaluation_gap", "LOW"),
        ("state-of-the-art", "evaluation_gap", "LOW"),
        ("previous methods", "evaluation_gap", "LOW"),
    ];

    let mut gaps: Vec<ResearchGap> = Vec::new();
    let mut seen: FxHashSet<String> = FxHashSet::default();

    for (idx, paper) in papers.iter().enumerate() {
        let text_lower = cache.get_lowercase(idx);
        let mut has_comparison = false;
        let mut baseline_mentioned = false;

        for (phrase, _, _) in &eval_patterns {
            if text_lower.contains(phrase) {
                if *phrase == "no baseline" || *phrase == "without comparison" {
                    baseline_mentioned = true;
                }
                if *phrase == "compare to" || *phrase == "outperforms" || *phrase == "previous methods" {
                    has_comparison = true;
                }
            }
        }

        let title_short = paper.title.chars().take(30).collect::<String>();
        let key = format!("eval:{}", title_short);

        if baseline_mentioned && !has_comparison && !seen.contains(&key) {
            seen.insert(key);
            gaps.push(ResearchGap::new_simple(
                "evaluation_gap",
                &format!("Evaluation gap: lacks comparative analysis in '{}'", title_short),
                "HIGH",
            ));
        }
    }

    gaps
}

fn detect_resource_gaps_impl(papers: &[rairos_core::Paper], cache: &PaperCache) -> Vec<ResearchGap> {
    let resource_phrases = [
        ("requires large", "resource_gap", "HIGH"),
        ("computationally expensive", "resource_gap", "HIGH"),
        ("memory intensive", "resource_gap", "HIGH"),
        ("gpu", "resource_gap", "MEDIUM"),
        ("training cost", "resource_gap", "MEDIUM"),
        ("inference time", "efficiency_gap", "MEDIUM"),
        ("latency", "efficiency_gap", "MEDIUM"),
        ("throughput", "efficiency_gap", "MEDIUM"),
        ("energy consumption", "resource_gap", "MEDIUM"),
        ("carbon footprint", "resource_gap", "LOW"),
    ];

    let mut gaps: Vec<ResearchGap> = Vec::new();
    let mut seen: FxHashSet<String> = FxHashSet::default();

    for (idx, paper) in papers.iter().enumerate() {
        let text_lower = cache.get_lowercase(idx);
        for (phrase, gap_type, severity) in &resource_phrases {
            if text_lower.contains(phrase) {
                let title_short = paper.title.chars().take(30).collect::<String>();
                let key = format!("{}:{}", phrase, title_short);
                if !seen.contains(&key) {
                    seen.insert(key);
                    gaps.push(ResearchGap::new_simple(
                        gap_type,
                        &format!("Resource gap: '{}' in '{}'", phrase, title_short),
                        severity,
                    ));
                }
            }
        }
    }

    gaps
}

fn detect_dataset_gaps_impl(papers: &[rairos_core::Paper], cache: &PaperCache) -> Vec<ResearchGap> {
    let dataset_patterns = [
        ("no dataset", "dataset_gap", "HIGH"),
        ("synthetic data", "dataset_gap", "MEDIUM"),
        ("limited data", "dataset_gap", "MEDIUM"),
        ("small dataset", "dataset_gap", "MEDIUM"),
        ("benchmark", "dataset_gap", "LOW"),
        ("real-world data", "dataset_gap", "MEDIUM"),
        ("real data", "dataset_gap", "MEDIUM"),
    ];

    let mut gaps: Vec<ResearchGap> = Vec::new();
    let mut seen: FxHashSet<String> = FxHashSet::default();

    for (idx, paper) in papers.iter().enumerate() {
        let text_lower = cache.get_lowercase(idx);
        let mut mentions_data = false;
        let mut has_real = false;

        for (phrase, _, _) in &dataset_patterns {
            if text_lower.contains(phrase) {
                mentions_data = true;
                if phrase.contains(&"real".to_string()) || phrase.contains("benchmark") {
                    has_real = true;
                }
            }
        }

        let title_short = paper.title.chars().take(30).collect::<String>();
        let key = format!("data:{}", title_short);

        if mentions_data && !has_real && !seen.contains(&key) {
            seen.insert(key);
            gaps.push(ResearchGap::new_simple(
                "dataset_gap",
                &format!("Dataset gap: limited real-world evaluation in '{}'", title_short),
                "MEDIUM",
            ));
        }
    }

    gaps
}

fn detect_generalization_gaps_impl(papers: &[rairos_core::Paper], cache: &PaperCache) -> Vec<ResearchGap> {
    let generalization_phrases = [
        ("generalize", "generalization_gap", "HIGH"),
        ("out-of-distribution", "generalization_gap", "HIGH"),
        ("distribution shift", "generalization_gap", "HIGH"),
        ("domain adaptation", "generalization_gap", "MEDIUM"),
        ("transfer learning", "generalization_gap", "MEDIUM"),
        ("cross-domain", "generalization_gap", "MEDIUM"),
        ("ood", "generalization_gap", "HIGH"),
        ("adversarial", "robustness_gap", "HIGH"),
    ];

    let mut gaps: Vec<ResearchGap> = Vec::new();
    let mut seen: FxHashSet<String> = FxHashSet::default();

    for (idx, paper) in papers.iter().enumerate() {
        let text_lower = cache.get_lowercase(idx);
        for (phrase, gap_type, severity) in &generalization_phrases {
            if text_lower.contains(phrase) {
                let title_short = paper.title.chars().take(30).collect::<String>();
                let key = format!("{}:{}", phrase, title_short);
                if !seen.contains(&key) {
                    seen.insert(key);
                    gaps.push(ResearchGap::new_simple(
                        gap_type,
                        &format!("Generalization gap: '{}' in '{}'", phrase, title_short),
                        severity,
                    ));
                }
            }
        }
    }

    gaps
}

pub struct AutonomousOrchestrator {
    config: OrchestratorConfig,
    webhook_enabled: bool,
    stop_tx: Arc<RwLock<Option<tokio::sync::watch::Sender<()>>>>,
    watch_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    db: Arc<RwLock<Option<Database>>>,
    tracker: Arc<RwLock<Option<EvolutionTracker>>>,
    regret_selector: Arc<StdMutex<RegretOptimalSelector>>,
    bayesian_optimizer: Arc<StdMutex<RankerBayesianOptimizer>>,
    crossover_engine: Arc<StdMutex<CrossoverEngine>>,
    scaling_learner: Arc<StdMutex<OptimalScalingLearner>>,
    adaptive_momentum: Arc<StdMutex<RankerAdaptiveMomentum>>,
    cached_state: Option<OrchestratorState>,
    last_persist: DateTime<Utc>,
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
            regret_selector: Arc::new(StdMutex::new(RegretOptimalSelector::new(0.1))),
            bayesian_optimizer: Arc::new(StdMutex::new(RankerBayesianOptimizer::new(
                vec![(0.1, 1.0), (0.1, 1.0), (1.0, 10.0)],
                1.0,
            ))),
            crossover_engine: Arc::new(StdMutex::new(CrossoverEngine::new(1.0, 0.1))),
            scaling_learner: Arc::new(StdMutex::new(OptimalScalingLearner::new(1.0, 1e-4))),
            adaptive_momentum: Arc::new(StdMutex::new(RankerAdaptiveMomentum::new(0.9))),
            cached_state: None,
            last_persist: Utc::now(),
        }
    }

    /// Load state from cache if valid, otherwise reload from disk.
    /// Cache is considered stale if more than MAX_PERSIST_INTERVAL_SECS have passed since last persist.
    fn load_state_cached(&mut self) -> OrchestratorState {
        let now = Utc::now();
        if self.cached_state.is_none()
            || (now - self.last_persist).num_seconds() > MAX_PERSIST_INTERVAL_SECS
        {
            self.cached_state = Some(load_state());
            self.last_persist = now;
        }
        self.cached_state.clone().unwrap()
    }

    pub fn record_gap_outcome(&mut self, gap_type: &str, score: f64) {
        self.regret_selector.lock().unwrap().record_outcome(gap_type, score);
    }

    pub fn apply_momentum_adjustment(&mut self, base_score: f64, gap_type: &str) -> f64 {
        let ucb_scores = self.regret_selector.lock().unwrap().get_ucb_scores();
        let ucb_bonus = ucb_scores.get(gap_type).copied().unwrap_or(0.0);
        let gradient = -(base_score - ucb_bonus);
        let momentum = self.adaptive_momentum.lock().unwrap().nesterov_update(gradient);
        (base_score + momentum * 0.1).clamp(0.0, 1.0)
    }

    pub fn get_adaptive_lr(&self, base_lr: f64, dataset_tokens: f64) -> f64 {
        let lr = {
            let guard = self.scaling_learner.lock().unwrap();
            guard.predict_optimal(1.0, dataset_tokens).0
        };
        let scaled_lr = base_lr * lr;
        self.adaptive_momentum.lock().unwrap().adaptive_lr(scaled_lr, 1.0)
    }

    pub fn observe_scaling(&mut self, lr: f64, batch_size: f64, loss: f64) {
        self.scaling_learner.lock().unwrap().observe(lr, batch_size, loss);
    }

    pub fn suggest_next_gap_type(&mut self, gap_types: &[&str]) -> Option<String> {
        self.regret_selector.lock().unwrap().select(gap_types)
    }

    pub fn get_optimal_thresholds(&self, beta: f64) -> Vec<f64> {
        self.bayesian_optimizer.lock().unwrap().suggest(beta)
    }

    pub fn observe_threshold_performance(&mut self, thresholds: &[f64], alert_rate: f64) {
        self.bayesian_optimizer.lock().unwrap().observe(thresholds, alert_rate);
    }

    pub fn suggest_research_direction(&mut self, available_gap_types: &[&str]) -> Option<String> {
        self.regret_selector.lock().unwrap().select(available_gap_types)
    }

    pub fn get_regret_stats(&self) -> (usize, f64) {
        let selector = self.regret_selector.lock().unwrap();
        let total = selector.total_selections();
        let empirical_best = selector.empirical_best_score();
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

    pub async fn check_subscriptions(&self) -> Result<FxHashMap<String, Vec<PaperInfo>>> {
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
            return Ok(FxHashMap::default());
        }

        // Pre-fetch existing papers for all topics to release db lock quickly
        let existing_papers: Vec<(String, Vec<Paper>)> = {
            let db_guard = self.db.read().await;
            let db = db_guard
                .as_ref()
                .ok_or_else(|| OrchestratorError::NotInitialized("database".to_string()))?;
            topics
                .iter()
                .map(|(topic, _)| {
                    let papers = db
                        .search_papers_smart(topic, 100)
                        .unwrap_or_default();
                    (topic.clone(), papers)
                })
                .collect()
        };

        // Fetch all arxiv papers in parallel (async, fast)
        use futures::future::join_all;
        let arxiv_futures = topics.iter().map(|(topic, _)| async move {
            let papers = rairos_parser::search_arxiv_recent(topic, 20).await.unwrap_or_default();
            (topic.clone(), papers)
        });
        let arxiv_results = join_all(arxiv_futures).await;
        let arxiv_map: FxHashMap<String, Vec<Paper>> =
            arxiv_results.into_iter().fold(
                FxHashMap::default(),
                |mut m, (t, p)| { m.insert(t, p); m },
            );

            // CPU-intensive filtering in spawn_blocking
        use tokio::task;
        let topic_count = topics.len();
        let search_results = task::spawn_blocking(move || {
            let mut results = FxHashMap::default();
            results.reserve(topic_count);

            // Build owned lookup map to avoid repeated iteration
            let existing_map: FxHashMap<String, Vec<Paper>> =
                existing_papers.into_iter().fold(
                    FxHashMap::default(),
                    |mut m, (t, p)| { m.insert(t, p); m },
                );

            for (topic, _sub_id) in &topics {
                let papers = arxiv_map.get(topic).cloned().unwrap_or_default();

                if papers.is_empty() {
                    continue;
                }

                let existing = existing_map.get(topic).cloned().unwrap_or_default();

                let existing_ids: FxHashSet<_> = existing
                    .iter()
                    .filter_map(|p| p.arxiv_id.clone())
                    .collect();

                let existing_word_sets: Vec<FxHashSet<String>> = existing
                    .iter()
                    .map(|p| compute_word_set(&p.abstract_text))
                    .collect();

                let similarity_threshold = 0.7;
                let new_papers: Vec<PaperInfo> = papers
                    .into_iter()
                    .filter_map(|p| {
                        let arxiv_id = p.arxiv_id.clone()?;
                        if existing_ids.contains(&arxiv_id) {
                            return None;
                        }

                        let new_words = compute_word_set(&p.abstract_text);
                        let is_too_similar = existing_word_sets.iter().any(|existing_words| {
                            jaccard_similarity(&new_words, existing_words) > similarity_threshold
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
                    results.insert(topic.clone(), new_papers);
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

        let mut all_categories: FxHashSet<String> = FxHashSet::default();
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
                        if !cat.is_empty() {
                            all_categories.insert(cat.to_string());
                        }
                    }
                }
            }
        }

        let keywords: Vec<&str> = all_categories.iter().map(|s| s.as_str()).collect();

        let _extracted_keywords: FxHashSet<String> = {
            let mut term_freq: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            let lowercase_abstracts: Vec<String> = papers.iter()
                .map(|p| p.abstract_text.to_lowercase())
                .collect();
            for lowercase_abstract in &lowercase_abstracts {
                let words: FxHashSet<String> = lowercase_abstract.split_whitespace()
                    .filter(|w| w.len() > 5)
                    .filter(|w| !STOP_WORDS.contains(w))
                    .map(|w| w.to_string())
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
        let suggested_gap_type = self.regret_selector.lock().unwrap().select(gap_types);

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

        // Pre-compute lowercase abstracts to avoid repeated to_lowercase() calls
        let cache = PaperCache::new(&papers);

        let pattern_gaps = self.detect_pattern_gaps(&papers, &cache, topic);
        let cross_paper_gaps = self.detect_cross_paper_gaps(&papers, &cache, topic);
        let method_gaps = self.detect_method_limitations(&papers, &cache);
        let eval_gaps = self.detect_evaluation_gaps(&papers, &cache);
        let resource_gaps = self.detect_resource_gaps(&papers, &cache);
        let dataset_gaps = self.detect_dataset_gaps(&papers, &cache);
        let generalization_gaps = self.detect_generalization_gaps(&papers, &cache);
        let mut gaps: Vec<ResearchGap> = pattern_gaps;
        gaps.extend(cross_paper_gaps);
        gaps.extend(method_gaps);
        gaps.extend(eval_gaps);
        gaps.extend(resource_gaps);
        gaps.extend(dataset_gaps);
        gaps.extend(generalization_gaps);

        for desc in gap_descriptions {
            // Pre-compute words_new once per desc (instead of per existing_paper)
            let words_new: FxHashSet<String> = desc.split_whitespace()
                .map(|w| w.to_lowercase())
                .filter(|w| w.len() > 4)
                .collect();

            let novelty = if existing_papers.is_empty() {
                1.0
            } else {
                let max_overlap = existing_papers.iter()
                    .map(|existing| {
                        let words_exist: FxHashSet<String> = existing.split_whitespace()
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
        let ucb_scores = self.regret_selector.lock().unwrap().get_ucb_scores();
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

        let suggested_thresholds = self.bayesian_optimizer.lock().unwrap().suggest(2.0);
        let bo_threshold = suggested_thresholds.first().copied().unwrap_or(0.3);
        let min_threshold = (self.config.min_gene_pool_score_for_alert * 0.7
            + bo_threshold * 0.3)
            .clamp(0.1, 0.9);

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
        self.bayesian_optimizer.lock().unwrap().observe(&[threshold], alert_rate);
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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

        let seen_descriptions: FxHashSet<_> = existing_gaps
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

    #[allow(dead_code)]
    async fn record_gaps(&self, gaps: &[ResearchGap]) -> Result<()> {
        self.init_components().await?;
        let db_guard = self.db.read().await;
        if let Some(db) = db_guard.as_ref() {
            for gap in gaps.iter().cloned() {
                let _ = db.insert_gap(&gap);
            }
        }
        Ok(())
    }

    pub async fn run_cycle(&mut self) -> Result<Vec<ResearchAlert>> {
        self.init_components().await?;
        let mut all_alerts: Vec<ResearchAlert> = Vec::new();

        tracing::info!("[Orchestrator] Starting cycle...");

        let sub_results = self.check_subscriptions().await?;

        // Spawn parallel tasks for each topic
        let handles: Vec<_> = sub_results
            .into_iter()
            .filter(|(_, new_papers)| !new_papers.is_empty())
            .filter(|(_, new_papers)| new_papers.len() >= self.config.min_papers_for_deep_analysis as usize)
            .map(|(topic, new_papers)| {
                let db = Arc::clone(&self.db);
                let tracker = Arc::clone(&self.tracker);
                let regret_selector = Arc::clone(&self.regret_selector);
                let bayesian_optimizer = Arc::clone(&self.bayesian_optimizer);
                let crossover_engine = Arc::clone(&self.crossover_engine);
                let scaling_learner = Arc::clone(&self.scaling_learner);
                let adaptive_momentum = Arc::clone(&self.adaptive_momentum);
                let config = self.config.clone();
                let webhook_enabled = self.webhook_enabled;

                tokio::spawn(async move {
                    process_topic(
                        db, tracker, regret_selector, bayesian_optimizer,
                        crossover_engine, scaling_learner, adaptive_momentum,
                        config, webhook_enabled, topic, new_papers,
                    ).await
                })
            })
            .collect();

        let results = join_all(handles).await;
        for result in results {
            match result {
                Ok(Ok(alerts)) => all_alerts.extend(alerts),
                Ok(Err(e)) => tracing::error!("Topic processing error: {}", e),
                Err(e) => tracing::error!("Spawn error: {}", e),
            }
        }

        let mut state = self.load_state_cached();
        for alert in &all_alerts {
            state.alerts.insert(0, alert.clone());
        }
        state
            .alerts
            .truncate(self.config.max_alerts_stored as usize);
        state.last_check = Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        let _ = save_state(&state);

        let current_topics: Vec<String> = {
            let db_guard = self.db.read().await;
            if let Some(db) = db_guard.as_ref() {
                db.list_subscriptions(false)
                    .map(|subs| subs.into_iter().map(|s| s.query).collect())
                    .unwrap_or_default()
            } else {
                Vec::new()
            }
        };

        let suggested_new = self.suggest_new_topics(&current_topics);
        if !suggested_new.is_empty() {
            tracing::info!("[Orchestrator] Suggested new topics: {:?}", suggested_new);
        }

        tracing::info!("[Orchestrator] Cycle complete: {} alerts", all_alerts.len());

        if self.config.run_evolution_in_cycle && !all_alerts.is_empty() {
            tracing::info!("[Orchestrator] Running evolution cycle...");
            if self.run_evolution_cycle("").await.is_ok() {
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

    pub fn suggest_adaptive_interval(&mut self) -> i32 {
        let state = self.load_state_cached();
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
        adjusted.clamp(5, 240)
    }

    pub fn suggest_new_topics(&mut self, current_topics: &[String]) -> Vec<String> {
        let state = self.load_state_cached();
        let mut topic_scores: std::collections::HashMap<String, f64> = std::collections::HashMap::new();

        let lowercase_titles: Vec<String> = state.alerts.iter()
            .map(|alert| alert.top_gap_title.to_lowercase())
            .collect();
        for (idx, alert) in state.alerts.iter().enumerate() {
            let words: FxHashSet<String> = lowercase_titles[idx].split_whitespace()
                .filter(|w| w.len() > 4)
                .map(|w| w.to_string())
                .collect();
            for word in words {
                *topic_scores.entry(word).or_insert(0.0) += alert.gene_pool_score;
            }
        }

        let current_set: FxHashSet<_> = current_topics.iter()
            .map(|t| t.to_lowercase())
            .collect();

        let mut suggestions: Vec<(String, f64)> = topic_scores.into_iter()
            .filter(|(topic, _)| !current_set.contains(topic))
            .filter(|(_, score)| *score > 0.5)
            .collect();
        suggestions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Less));

        suggestions.into_iter()
            .take(5)
            .map(|(topic, _)| topic)
            .collect()
    }

    fn detect_pattern_gaps(&self, papers: &[rairos_core::Paper], cache: &PaperCache, topic: &str) -> Vec<ResearchGap> {
        let limitation_patterns = limitation_patterns();
        let mut detected: Vec<ResearchGap> = Vec::new();
        let mut seen_patterns: FxHashSet<String> = FxHashSet::default();

        for (idx, paper) in papers.iter().enumerate() {
            let text_lower = cache.get_lowercase(idx);
            for lp in &limitation_patterns {
                if text_lower.contains(lp.pattern_lower) {
                    let desc = format!("Gap in '{}': {} (found in {})", topic, lp.pattern, paper.title.chars().take(40).collect::<String>());
                    if !seen_patterns.contains(lp.pattern) {
                        seen_patterns.insert(lp.pattern.to_string());
                        detected.push(ResearchGap::new_simple(lp.gap_type, &desc, lp.severity));
                    }
                }
            }
        }

        detected
    }

    fn extract_key_phrases(&self, papers: &[rairos_core::Paper], cache: &PaperCache) -> std::collections::HashMap<String, Vec<String>> {
        let mut phrase_papers: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        let significant_bigrams = [
            "sparse autoencoder", "attention mechanism", "variational autoencoder",
            "large language", "neural network", "deep learning", "reinforcement learning",
            "machine translation", "object detection", "semantic segmentation",
            "transformer architecture", "pre-trained model", "foundation model",
            "representation learning", "self-supervised learning", "contrastive learning",
        ];

        for (idx, paper) in papers.iter().enumerate() {
            let text_lower = cache.get_lowercase(idx);
            for bigram in significant_bigrams {
                if text_lower.contains(bigram) {
                    phrase_papers
                        .entry(bigram.to_string())
                        .or_default()
                        .push(paper.title.clone());
                }
            }
        }

        phrase_papers
    }

    fn detect_cross_paper_gaps(&self, papers: &[rairos_core::Paper], cache: &PaperCache, topic: &str) -> Vec<ResearchGap> {
        let phrase_map = self.extract_key_phrases(papers, cache);
        let mut gaps: Vec<ResearchGap> = Vec::new();

        for (phrase, titles) in phrase_map {
            if titles.len() >= 2 {
                let gap_type = if phrase.contains("sparse") || phrase.contains("autoencoder")
                    || phrase.contains("attention") || phrase.contains("transformer") {
                    "architecture_gap"
                } else if phrase.contains("variational") || phrase.contains("representation") {
                    "learning_gap"
                } else if phrase.contains("language") || phrase.contains("translation") {
                    "application_gap"
                } else {
                    "method_gap"
                };

                gaps.push(ResearchGap::new_simple(
                    gap_type,
                    &format!("Gap in '{}': Multiple papers on '{}' but no unified approach ({} papers)",
                        topic, phrase, titles.len()),
                    "MEDIUM",
                ));
            }
        }

        gaps
    }

    fn detect_method_limitations(&self, papers: &[rairos_core::Paper], cache: &PaperCache) -> Vec<ResearchGap> {
        let limitation_phrases = [
            ("limited by", "scalability_gap", "HIGH", 2),
            ("struggle with", "method_limitation", "MEDIUM", 2),
            ("inefficient", "efficiency_gap", "HIGH", 2),
            ("suboptimal", "efficiency_gap", "MEDIUM", 2),
            ("no theoretical guarantee", "theoretical_gap", "HIGH", 3),
            ("lack of theoretical", "theoretical_gap", "HIGH", 3),
            ("empirical only", "theoretical_gap", "MEDIUM", 2),
            ("not robust to", "robustness_gap", "HIGH", 2),
            ("sensitive to", "robustness_gap", "MEDIUM", 2),
            ("breaks down", "robustness_gap", "HIGH", 2),
            ("collapses", "training_gap", "HIGH", 2),
            ("fails to converge", "training_gap", "HIGH", 2),
            ("gradient", "training_gap", "MEDIUM", 2),
            ("vanishing", "training_gap", "MEDIUM", 2),
            ("exploding", "training_gap", "MEDIUM", 2),
        ];

        let mut gaps: Vec<ResearchGap> = Vec::new();
        let mut seen: FxHashSet<String> = FxHashSet::default();

        for (idx, paper) in papers.iter().enumerate() {
            let text_lower = cache.get_lowercase(idx);
            for (phrase, gap_type, severity, _min_words) in &limitation_phrases {
                if text_lower.contains(phrase) {
                    let title_short = paper.title.chars().take(30).collect::<String>();
                    let key = format!("{}:{}", phrase, title_short);
                    if !seen.contains(&key) {
                        seen.insert(key);
                        gaps.push(ResearchGap::new_simple(
                            gap_type,
                            &format!("Method limitation: '{}' in paper '{}'", phrase, title_short),
                            severity,
                        ));
                    }
                }
            }
        }

        gaps
    }

    fn detect_evaluation_gaps(&self, papers: &[rairos_core::Paper], cache: &PaperCache) -> Vec<ResearchGap> {
        let eval_patterns = [
            ("no baseline", "evaluation_gap", "HIGH"),
            ("without comparison", "evaluation_gap", "HIGH"),
            ("compare to", "evaluation_gap", "LOW"),
            ("outperforms", "evaluation_gap", "LOW"),
            ("state-of-the-art", "evaluation_gap", "LOW"),
            ("previous methods", "evaluation_gap", "LOW"),
        ];

        let mut gaps: Vec<ResearchGap> = Vec::new();
        let mut seen: FxHashSet<String> = FxHashSet::default();

        for (idx, paper) in papers.iter().enumerate() {
            let text_lower = cache.get_lowercase(idx);
            let mut has_comparison = false;
            let mut baseline_mentioned = false;

            for (phrase, _, _) in &eval_patterns {
                if text_lower.contains(phrase) {
                    if *phrase == "no baseline" || *phrase == "without comparison" {
                        baseline_mentioned = true;
                    }
                    if *phrase == "compare to" || *phrase == "outperforms" || *phrase == "previous methods" {
                        has_comparison = true;
                    }
                }
            }

            let title_short = paper.title.chars().take(30).collect::<String>();
            let key = format!("eval:{}", title_short);

            if baseline_mentioned && !has_comparison && !seen.contains(&key) {
                seen.insert(key);
                gaps.push(ResearchGap::new_simple(
                    "evaluation_gap",
                    &format!("Evaluation gap: lacks comparative analysis in '{}'", title_short),
                    "HIGH",
                ));
            }
        }

        gaps
    }

    fn detect_resource_gaps(&self, papers: &[rairos_core::Paper], cache: &PaperCache) -> Vec<ResearchGap> {
        let resource_phrases = [
            ("requires large", "resource_gap", "HIGH"),
            ("computationally expensive", "resource_gap", "HIGH"),
            ("memory intensive", "resource_gap", "HIGH"),
            ("gpu", "resource_gap", "MEDIUM"),
            ("training cost", "resource_gap", "MEDIUM"),
            ("inference time", "efficiency_gap", "MEDIUM"),
            ("latency", "efficiency_gap", "MEDIUM"),
            ("throughput", "efficiency_gap", "MEDIUM"),
            ("energy consumption", "resource_gap", "MEDIUM"),
            ("carbon footprint", "resource_gap", "LOW"),
        ];

        let mut gaps: Vec<ResearchGap> = Vec::new();
        let mut seen: FxHashSet<String> = FxHashSet::default();

        for (idx, paper) in papers.iter().enumerate() {
            let text_lower = cache.get_lowercase(idx);
            for (phrase, gap_type, severity) in &resource_phrases {
                if text_lower.contains(phrase) {
                    let title_short = paper.title.chars().take(30).collect::<String>();
                    let key = format!("{}:{}", phrase, title_short);
                    if !seen.contains(&key) {
                        seen.insert(key);
                        gaps.push(ResearchGap::new_simple(
                            gap_type,
                            &format!("Resource gap: '{}' in '{}'", phrase, title_short),
                            severity,
                        ));
                    }
                }
            }
        }

        gaps
    }

    fn detect_dataset_gaps(&self, papers: &[rairos_core::Paper], cache: &PaperCache) -> Vec<ResearchGap> {
        let dataset_patterns = [
            ("no dataset", "dataset_gap", "HIGH"),
            ("synthetic data", "dataset_gap", "MEDIUM"),
            ("limited data", "dataset_gap", "MEDIUM"),
            ("small dataset", "dataset_gap", "MEDIUM"),
            ("benchmark", "dataset_gap", "LOW"),
            ("real-world data", "dataset_gap", "MEDIUM"),
            ("real data", "dataset_gap", "MEDIUM"),
        ];

        let mut gaps: Vec<ResearchGap> = Vec::new();
        let mut seen: FxHashSet<String> = FxHashSet::default();

        for (idx, paper) in papers.iter().enumerate() {
            let text_lower = cache.get_lowercase(idx);
            let mut mentions_data = false;
            let mut has_real = false;

            for (phrase, _, _) in &dataset_patterns {
                if text_lower.contains(phrase) {
                    mentions_data = true;
                    if phrase.contains(&"real".to_string()) || phrase.contains("benchmark") {
                        has_real = true;
                    }
                }
            }

            let title_short = paper.title.chars().take(30).collect::<String>();
            let key = format!("data:{}", title_short);

            if mentions_data && !has_real && !seen.contains(&key) {
                seen.insert(key);
                gaps.push(ResearchGap::new_simple(
                    "dataset_gap",
                    &format!("Dataset gap: limited real-world evaluation in '{}'", title_short),
                    "MEDIUM",
                ));
            }
        }

        gaps
    }

    fn detect_generalization_gaps(&self, papers: &[rairos_core::Paper], cache: &PaperCache) -> Vec<ResearchGap> {
        let generalization_phrases = [
            ("generalize", "generalization_gap", "HIGH"),
            ("out-of-distribution", "generalization_gap", "HIGH"),
            ("distribution shift", "generalization_gap", "HIGH"),
            ("domain adaptation", "generalization_gap", "MEDIUM"),
            ("transfer learning", "generalization_gap", "MEDIUM"),
            ("cross-domain", "generalization_gap", "MEDIUM"),
            ("ood", "generalization_gap", "HIGH"),
            ("adversarial", "robustness_gap", "HIGH"),
        ];

        let mut gaps: Vec<ResearchGap> = Vec::new();
        let mut seen: FxHashSet<String> = FxHashSet::default();

        for (idx, paper) in papers.iter().enumerate() {
            let text_lower = cache.get_lowercase(idx);
            for (phrase, gap_type, severity) in &generalization_phrases {
                if text_lower.contains(phrase) {
                    let title_short = paper.title.chars().take(30).collect::<String>();
                    let key = format!("{}:{}", phrase, title_short);
                    if !seen.contains(&key) {
                        seen.insert(key);
                        gaps.push(ResearchGap::new_simple(
                            gap_type,
                            &format!("Generalization gap: '{}' in '{}'", phrase, title_short),
                            severity,
                        ));
                    }
                }
            }
        }

        gaps
    }

    pub async fn start_watch(&mut self, interval_minutes: i32) -> Result<()> {
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

        let mut state = self.load_state_cached();
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

    pub async fn stop_watch(&mut self) -> Result<()> {
        {
            let mut stop = self.stop_tx.write().await;
            if let Some(tx) = stop.take() {
                let _ = tx.send(());
            }
        }

        let mut state = self.load_state_cached();
        state.running = false;
        let _ = save_state(&state);

        tracing::info!("[Orchestrator] Watch stopped");
        Ok(())
    }

    pub fn get_recent_alerts(&mut self, limit: usize) -> Vec<ResearchAlert> {
        let state = self.load_state_cached();
        state.alerts.iter().take(limit).cloned().collect()
    }

    pub async fn run_evolution_cycle(
        &mut self,
        topic: &str,
    ) -> Result<FxHashMap<String, serde_json::Value>> {
        self.init_components().await?;

        let evo_topic = if topic.is_empty() {
            self.best_evolution_topic()
        } else {
            topic.to_string()
        };

        let gap_types: Vec<String> = self.regret_selector.lock().unwrap().get_ucb_scores().keys().cloned().collect();
        let gap_type_refs: Vec<&str> = gap_types.iter().map(|s| s.as_str()).collect();

        if gap_type_refs.len() >= 2 {
            self.regret_selector.lock().unwrap().select(&gap_type_refs);
        }

        let result = {
            let mut crossover = self.crossover_engine.lock().unwrap();
            rairos_crossover::run_evolution_with_engine(
                3,
                10,
                Some(&mut *crossover),
            )
        };

        if let Some(created) = result.get("created").and_then(|v| v.as_array()) {
            let offspring_count = created.len();
            if offspring_count > 0 {
                let avg_fitness: f64 = created.iter()
                    .map(|v| {
                        let f_a = v.get("fitness_a").and_then(|f| f.as_f64()).unwrap_or(0.5);
                        let f_b = v.get("fitness_b").and_then(|f| f.as_f64()).unwrap_or(0.5);
                        (f_a + f_b) / 2.0
                    })
                    .sum::<f64>() / offspring_count as f64;

                if let (Some(p_a), Some(p_b)) = (
                    result.get("parent_a_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    result.get("parent_b_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
                ) {
                    self.crossover_engine.lock().unwrap().record_comparison(&p_a, &p_b, avg_fitness);
                }
            }
        }

        let mut output = FxHashMap::default();
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

    pub async fn get_status(&mut self) -> Result<FxHashMap<String, serde_json::Value>> {
        let state = self.load_state_cached();
        let pool_stats = self.gene_pool_stats().await;

        let mut status = FxHashMap::default();
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