//! rairos-crossover — CapsuleGene Crossover
//!
//! Genetic algorithm on Gene Pool archetypes.
//!
//! Preference Optimization based on arXiv:2505.08735

use rand::seq::SliceRandom;
use rand::Rng;
use rairos_core::constants::{GP_DIR_NAME, GENE_POOL_JSONL, CODE_GENE_POOL_JSONL};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub const MAX_CODE_GENES: usize = 200;

// ============================================================================
// Preference Ranker (arXiv:2505.08735)
// ============================================================================

#[derive(Debug, Clone)]
pub struct PreferenceRanker {
    win_counts: HashMap<String, usize>,
    total_comparisons: usize,
    entropy_bonus: f64,
}

impl PreferenceRanker {
    pub fn new(entropy_bonus: f64) -> Self {
        Self {
            win_counts: HashMap::new(),
            total_comparisons: 0,
            entropy_bonus,
        }
    }

    pub fn record_outcome(&mut self, winner_id: &str, _loser_id: &str) {
        *self.win_counts.entry(winner_id.to_string()).or_insert(0) += 1;
        self.total_comparisons += 1;
    }

    fn preference_score(&self, id: &str) -> f64 {
        let wins = self.win_counts.get(id).copied().unwrap_or(0) as f64;
        let total = self.total_comparisons.max(1) as f64;
        let base = wins / total;

        let loss_rate = 1.0 - base;
        let entropy = if loss_rate > 0.0 && loss_rate < 1.0 {
            -loss_rate * loss_rate.ln() - base * base.ln()
        } else {
            0.0
        };

        base + self.entropy_bonus * entropy
    }

    pub fn rank_capsules<'a>(&self, capsule_ids: &'a [&str]) -> Vec<(&'a str, f64)> {
        let mut scored: Vec<(&str, f64)> = capsule_ids
            .iter()
            .map(|id| (*id, self.preference_score(id)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored
    }

    pub fn select_parents<'a>(&self, candidates: &'a [&str], count: usize) -> Vec<&'a str> {
        let ranked = self.rank_capsules(candidates);
        ranked.into_iter().take(count).map(|(id, _)| id).collect()
    }
}

#[derive(Debug, Clone)]
pub struct UCBParentSelector {
    capsule_scores: HashMap<String, f64>,
    capsule_counts: HashMap<String, usize>,
    total_selections: usize,
    exploration_param: f64,
}

impl UCBParentSelector {
    pub fn new(exploration_param: f64) -> Self {
        Self {
            capsule_scores: HashMap::new(),
            capsule_counts: HashMap::new(),
            total_selections: 0,
            exploration_param,
        }
    }

    pub fn record_comparison(&mut self, winner_id: &str, loser_id: &str, winner_score: f64) {
        *self.capsule_scores.entry(winner_id.to_string()).or_insert(0.0) += winner_score;
        *self.capsule_counts.entry(winner_id.to_string()).or_insert(0) += 1;
        *self.capsule_counts.entry(loser_id.to_string()).or_insert(0) += 0;
    }

    fn ucb_score(&self, id: &str) -> f64 {
        let total = self.total_selections.max(1) as f64;
        let avg = self.capsule_scores.get(id).copied().unwrap_or(0.0)
            / self.capsule_counts.get(id).copied().unwrap_or(1).max(1) as f64;
        let count = self.capsule_counts.get(id).copied().unwrap_or(0).max(1) as f64;
        let exploration_bonus = self.exploration_param * (total.ln() / count).sqrt();
        avg + exploration_bonus
    }

    pub fn select<'a>(&mut self, candidates: &'a [&str]) -> Option<&'a str> {
        if candidates.is_empty() {
            return None;
        }
        self.total_selections += 1;
        candidates
            .iter()
            .max_by(|a, b| {
                self.ucb_score(a)
                    .partial_cmp(&self.ucb_score(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied()
    }

    pub fn select_idx(&mut self, candidates: &[&str]) -> Option<usize> {
        if candidates.is_empty() {
            return None;
        }
        self.total_selections += 1;
        candidates
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| {
                self.ucb_score(a)
                    .partial_cmp(&self.ucb_score(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
    }
}

pub struct CrossoverEngine {
    ucb_selector: UCBParentSelector,
    preference_ranker: PreferenceRanker,
    fitness_cache: HashMap<String, f64>,
}

impl CrossoverEngine {
    pub fn new(exploration_param: f64, entropy_bonus: f64) -> Self {
        Self {
            ucb_selector: UCBParentSelector::new(exploration_param),
            preference_ranker: PreferenceRanker::new(entropy_bonus),
            fitness_cache: HashMap::new(),
        }
    }

    pub fn select_parents_ucb(&mut self, candidates: &[&str]) -> Option<(usize, usize)> {
        if candidates.len() < 2 {
            return None;
        }
        let first_idx = self.ucb_selector.select_idx(candidates)?;
        let second_candidates: Vec<&str> = candidates
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != first_idx)
            .map(|(_, &s)| s)
            .collect();
        if second_candidates.is_empty() {
            return None;
        }
        let second_idx_opt = self.ucb_selector.select_idx(&second_candidates);
        if let Some(second_relative) = second_idx_opt {
            let actual_second = if second_relative >= first_idx {
                second_relative + 1
            } else {
                second_relative
            };
            if actual_second < candidates.len() && actual_second != first_idx {
                return Some((first_idx, actual_second));
            }
        }
        let mut rng = rand::thread_rng();
        let second_idx = (0..candidates.len())
            .filter(|&i| i != first_idx)
            .collect::<Vec<_>>()
            .choose(&mut rng)
            .copied()?;
        Some((first_idx, second_idx))
    }

    pub fn record_comparison(&mut self, winner_id: &str, loser_id: &str, winner_score: f64) {
        self.ucb_selector.record_comparison(winner_id, loser_id, winner_score);
        self.preference_ranker.record_outcome(winner_id, loser_id);
    }

    pub fn get_fitness(&mut self, capsule_id: &str, fallback: f64) -> f64 {
        *self.fitness_cache.entry(capsule_id.to_string()).or_insert(fallback)
    }
}

const DEFAULT_POPULATION_SIZE: usize = 20;
const _DEFAULT_OFFSPRING_COUNT: usize = 5;
const MIN_FITNESS_THRESHOLD: f64 = 0.3;
const MUTATION_RATE: f64 = 0.15;

fn gp_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(GP_DIR_NAME)
}

fn gene_pool_path() -> PathBuf {
    gp_dir().join(GENE_POOL_JSONL)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapsuleGene {
    pub capsule_id: String,
    pub created_at: String,
    pub trigger_topic: String,
    pub trigger_gap_type: String,
    pub trigger_keywords: Vec<String>,
    pub action_gap_type: String,
    pub action_gap_title: String,
    pub outcome_success_score: f64,
    pub feedback_count: i32,
    pub evolved_generation: i32,
    pub archetype: HashMap<String, serde_json::Value>,
    pub status: String,
    pub low_score_streak: i32,
    pub credibility_score: f64,
    pub trendslop: bool,
    pub trendslop_reason: String,
    pub source_arxiv_category: String,
    pub credibility_badge: String,
}

impl CapsuleGene {
    fn from_json(value: serde_json::Value) -> Option<Self> {
        let capsule_id = value.get("capsule_id")?.as_str()?.to_string();
        let created_at = value.get("created_at")?.as_str()?.to_string();

        let action_gap_title = value
            .get("action_gap_title")
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| {
                value.get("archetype")?.get("approach_summary")?.as_str().map(String::from)
            })
            .unwrap_or_else(|| "Untitled capsule".to_string());

        let action_gap_type = value
            .get("action_gap_type")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| "unknown".to_string());

        let trigger_topic = value
            .get("trigger_topic")
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| {
                value.get("trigger_keywords")?.as_array()?.first()?.as_str().map(String::from)
            })
            .unwrap_or_default();

        let trigger_gap_type = value
            .get("trigger_gap_type")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| action_gap_type.clone());

        let trigger_keywords: Vec<String> = value
            .get("trigger_keywords")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Some(Self {
            capsule_id,
            created_at,
            trigger_topic,
            trigger_gap_type,
            trigger_keywords,
            action_gap_type,
            action_gap_title,
            outcome_success_score: value
                .get("outcome_success_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.5),
            feedback_count: value
                .get("feedback_count")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32,
            evolved_generation: value
                .get("evolved_generation")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32,
            archetype: value
                .get("archetype")
                .and_then(|v| v.as_object().cloned())
                .map(|m| m.into_iter().collect())
                .unwrap_or_default(),
            status: value
                .get("status")
                .and_then(|v| v.as_str())
                .map(String::from)
                .unwrap_or_else(|| "active".to_string()),
            low_score_streak: value
                .get("low_score_streak")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32,
            credibility_score: value
                .get("credibility_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.5),
            trendslop: value
                .get("trendslop")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            trendslop_reason: value
                .get("trendslop_reason")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            source_arxiv_category: value
                .get("source_arxiv_category")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            credibility_badge: value
                .get("credibility_badge")
                .and_then(|v| v.as_str())
                .unwrap_or("medium")
                .to_string(),
        })
    }
}

impl CapsuleGene {
    pub fn trigger_match(&self, topic: &str, gap_type: &str, keywords: &[String]) -> f64 {
        let mut score = 0.0;

        let action_title = self.action_gap_title.trim();
        if !action_title.is_empty() && !topic.is_empty() {
            let action_lower = action_title.to_lowercase();
            let topic_lower = topic.to_lowercase();
            if action_lower.contains(&topic_lower) {
                score += 0.5;
            } else if topic_lower.contains(&action_lower) {
                score += 0.3;
            }
        }

        if !topic.is_empty() && !self.trigger_topic.is_empty() {
            if self
                .trigger_topic
                .to_lowercase()
                .contains(&topic.to_lowercase())
            {
                score += 0.3;
            } else if topic
                .to_lowercase()
                .contains(&self.trigger_topic.to_lowercase())
            {
                score += 0.2;
            }
        }

        if !gap_type.is_empty() && !self.trigger_gap_type.is_empty() {
            if gap_type == self.trigger_gap_type {
                score += 0.3;
            } else {
                let category_of = |gt: &str| -> String {
                    if ["improvement", "method_gap", "method_limitation"].contains(&gt) {
                        "method".to_string()
                    } else if ["application_gap", "exploration_gap", "capability"].contains(&gt) {
                        "content".to_string()
                    } else {
                        gt.to_string()
                    }
                };
                if category_of(gap_type) == category_of(&self.trigger_gap_type) {
                    score += 0.1;
                }
            }
        }

        if !keywords.is_empty() && !self.trigger_keywords.is_empty() {
            let kw_set: std::collections::HashSet<String> =
                keywords.iter().map(|k| k.to_lowercase()).collect();
            let trigger_set: std::collections::HashSet<String> = self
                .trigger_keywords
                .iter()
                .map(|k| k.to_lowercase())
                .collect();
            let overlap: std::collections::HashSet<_> = kw_set.intersection(&trigger_set).collect();
            if !overlap.is_empty() {
                let denom = keywords.len().max(self.trigger_keywords.len());
                score += 0.15 * (overlap.len() as f64 / denom as f64);
            }
        }

        if !action_title.is_empty() && !topic.is_empty() {
            let stopwords: std::collections::HashSet<&str> = [
                "for", "with", "and", "the", "a", "an", "of", "in", "on", "to", "is", "are",
            ]
            .into();
            let topic_tokens: std::collections::HashSet<String> = topic
                .to_lowercase()
                .split_whitespace()
                .filter(|w| !stopwords.contains(w))
                .map(String::from)
                .collect();
            let title_tokens: std::collections::HashSet<String> = action_title
                .to_lowercase()
                .split_whitespace()
                .filter(|w| !stopwords.contains(w))
                .map(String::from)
                .collect();
            if !topic_tokens.is_empty() && !title_tokens.is_empty() {
                let intersection: std::collections::HashSet<_> =
                    topic_tokens.intersection(&title_tokens).collect();
                let union: std::collections::HashSet<String> =
                    topic_tokens.union(&title_tokens).cloned().collect();
                if !union.is_empty() {
                    let jaccard = intersection.len() as f64 / union.len() as f64;
                    if jaccard > 0.0 {
                        score += 0.25 * jaccard;
                    }
                }
            }
        }

        score.min(1.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusChange {
    pub from_status: String,
    pub to_status: String,
    pub reason: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeCapsuleGene {
    pub capsule_id: String,
    pub created_at: String,
    pub trigger_topic: String,
    pub trigger_keywords: Vec<String>,
    pub source_paper_id: String,
    pub source_paper_title: String,
    pub target_crate: String,
    pub gap_type: String,
    pub gap_location: String,
    pub code_snippet: String,
    pub optimization: String,
    pub outcome_success_score: f64,
    pub feedback_count: i32,
    pub evolved_generation: i32,
    pub archetype: HashMap<String, serde_json::Value>,
    pub status: String,
    pub status_history: Vec<StatusChange>,
    pub low_score_streak: i32,
    pub credibility_score: f64,
    pub credibility_badge: String,
}

impl CodeCapsuleGene {
    pub fn trigger_match(&self, topic: &str, keywords: &[String]) -> f64 {
        let mut score = 0.0;
        let trigger_lower = self.trigger_topic.to_lowercase();
        let topic_lower = topic.to_lowercase();
        if trigger_lower.contains(&topic_lower) {
            score += 0.4;
        }
        if !keywords.is_empty() && !self.trigger_keywords.is_empty() {
            let kw_set: std::collections::HashSet<String> = keywords.iter().map(|k| k.to_lowercase()).collect();
            let trigger_set: std::collections::HashSet<String> = self.trigger_keywords.iter().map(|k| k.to_lowercase()).collect();
            let overlap: std::collections::HashSet<_> = kw_set.intersection(&trigger_set).collect();
            if !overlap.is_empty() {
                score += 0.3 * (overlap.len() as f64 / keywords.len().max(self.trigger_keywords.len()) as f64);
            }
        }
        let target_lower = self.target_crate.to_lowercase();
        if target_lower.contains(&topic_lower) {
            score += 0.3;
        }
        score.min(1.0)
    }
}

pub fn code_gene_pool_path() -> PathBuf {
    gp_dir().join(CODE_GENE_POOL_JSONL)
}

fn load_code_capsules() -> Vec<CodeCapsuleGene> {
    let path = code_gene_pool_path();
    if !path.exists() {
        return Vec::new();
    }
    let text = match fs::read_to_string(&path) {
        Ok(t) => t.trim().to_string(),
        Err(_) => return Vec::new(),
    };
    if text.is_empty() {
        return Vec::new();
    }
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter_map(|v| {
            Some(CodeCapsuleGene {
                capsule_id: v.get("capsule_id")?.as_str()?.to_string(),
                created_at: v.get("created_at")?.as_str()?.to_string(),
                trigger_topic: v.get("trigger_topic")?.as_str()?.to_string(),
                trigger_keywords: v.get("trigger_keywords")?.as_array()?.iter().filter_map(|x| x.as_str().map(String::from)).collect(),
                source_paper_id: v.get("source_paper_id").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                source_paper_title: v.get("source_paper_title").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                target_crate: v.get("target_crate").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                gap_type: v.get("gap_type").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                gap_location: v.get("gap_location").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                code_snippet: v.get("code_snippet").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                optimization: v.get("optimization").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                outcome_success_score: v.get("outcome_success_score").and_then(|x| x.as_f64()).unwrap_or(0.5),
                feedback_count: v.get("feedback_count").and_then(|x| x.as_i64()).unwrap_or(0) as i32,
                evolved_generation: v.get("evolved_generation").and_then(|x| x.as_i64()).unwrap_or(0) as i32,
                archetype: v.get("archetype").and_then(|x| x.as_object()).map(|m| m.clone().into_iter().collect()).unwrap_or_default(),
                status: v.get("status").and_then(|x| x.as_str()).unwrap_or("active").to_string(),
                status_history: v.get("status_history").and_then(|x| x.as_array()).map(|arr| {
                    arr.iter().filter_map(|item| {
                        Some(StatusChange {
                            from_status: item.get("from_status")?.as_str()?.to_string(),
                            to_status: item.get("to_status")?.as_str()?.to_string(),
                            reason: item.get("reason")?.as_str()?.to_string(),
                            timestamp: item.get("timestamp")?.as_str()?.to_string(),
                        })
                    }).collect()
                }).unwrap_or_default(),
                low_score_streak: v.get("low_score_streak").and_then(|x| x.as_i64()).unwrap_or(0) as i32,
                credibility_score: v.get("credibility_score").and_then(|x| x.as_f64()).unwrap_or(0.5),
                credibility_badge: v.get("credibility_badge").and_then(|x| x.as_str()).unwrap_or("medium").to_string(),
            })
        })
        .collect()
}

pub fn get_top_code_candidates(limit: usize) -> Vec<CodeCapsuleGene> {
    let capsules = load_code_capsules();
    let mut active: Vec<CodeCapsuleGene> = capsules
        .into_iter()
        .filter(|c| c.status == "active" && c.credibility_badge != "low")
        .collect();
    active.sort_by(|a, b| {
        let fitness_a = a.outcome_success_score * (1.0 + a.feedback_count as f64).ln();
        let fitness_b = b.outcome_success_score * (1.0 + b.feedback_count as f64).ln();
        fitness_b.partial_cmp(&fitness_a).unwrap_or(std::cmp::Ordering::Equal)
    });
    active.truncate(limit);
    active
}

pub fn get_all_code_capsules() -> Vec<CodeCapsuleGene> {
    load_code_capsules()
}

pub fn save_code_capsule(capsule: &CodeCapsuleGene) -> std::io::Result<()> {
    let path = code_gene_pool_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut capsules = load_code_capsules();
    let exists = capsules.iter().any(|c| c.capsule_id == capsule.capsule_id);

    if exists {
        return update_code_capsule(capsule);
    }

    capsules.push(capsule.clone());

    if capsules.len() > MAX_CODE_GENES {
        capsules.sort_by(|a, b| {
            let score_a = a.outcome_success_score * 100.0 + a.feedback_count as f64;
            let score_b = b.outcome_success_score * 100.0 + b.feedback_count as f64;
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        capsules.truncate(MAX_CODE_GENES);
    }

    let mut file = fs::File::create(&path)?;
    for cap in &capsules {
        let json = serde_json::to_string(cap)?;
        file.write_all(json.as_bytes())?;
        file.write_all(b"\n")?;
    }
    Ok(())
}

pub fn update_code_capsule(capsule: &CodeCapsuleGene) -> std::io::Result<()> {
    let path = code_gene_pool_path();

    let capsules = load_code_capsules();
    let mut updated: Vec<CodeCapsuleGene> = capsules
        .into_iter()
        .filter(|c| c.capsule_id != capsule.capsule_id)
        .collect();
    updated.push(capsule.clone());

    let mut file = fs::File::create(&path)?;
    for cap in &updated {
        let json = serde_json::to_string(cap)?;
        file.write_all(json.as_bytes())?;
        file.write_all(b"\n")?;
    }

    Ok(())
}

fn load_capsules() -> Vec<CapsuleGene> {
    let path = gene_pool_path();
    if !path.exists() {
        return Vec::new();
    }
    let text = match fs::read_to_string(&path) {
        Ok(t) => t.trim().to_string(),
        Err(_) => return Vec::new(),
    };
    if text.is_empty() {
        return Vec::new();
    }
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter_map(CapsuleGene::from_json)
        .collect()
}

pub fn compute_fitness(capsule: &CapsuleGene) -> f64 {
    let score = capsule.outcome_success_score;
    let fb = capsule.feedback_count as f64;
    score * fb.ln_1p()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossoverResult {
    pub archetype: HashMap<String, serde_json::Value>,
    pub parent_a_id: String,
    pub parent_b_id: String,
    pub parent_generations: i32,
    pub parent_fitness_a: f64,
    pub parent_fitness_b: f64,
}

pub fn crossover(parent_a: &CapsuleGene, parent_b: &CapsuleGene) -> CrossoverResult {
    let arch_a = parent_a.archetype.clone();
    let arch_b = parent_b.archetype.clone();

    let shared_keys: Vec<&String> = arch_a.keys().filter(|k| arch_b.contains_key(*k)).collect();
    let private_a: HashMap<String, serde_json::Value> = arch_a
        .iter()
        .filter(|(k, _)| !arch_b.contains_key(*k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let private_b: HashMap<String, serde_json::Value> = arch_b
        .iter()
        .filter(|(k, _)| !arch_a.contains_key(*k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    let merged = if shared_keys.is_empty() {
        let mut m = arch_a.clone();
        m.extend(arch_b);
        m
    } else {
        let mut rng = rand::thread_rng();
        let point = rng.gen_range(1..=shared_keys.len().max(1));
        let swapped_keys: std::collections::HashSet<&String> =
            shared_keys[..point].iter().cloned().collect();

        let mut m: HashMap<String, serde_json::Value> = HashMap::new();
        for k in &shared_keys {
            m.insert(
                (*k).to_string(),
                if swapped_keys.contains(k) {
                    arch_b.get(*k).cloned().unwrap_or(serde_json::Value::Null)
                } else {
                    arch_a.get(*k).cloned().unwrap_or(serde_json::Value::Null)
                },
            );
        }
        m.extend(private_a);
        m.extend(private_b);
        m
    };

    CrossoverResult {
        archetype: merged,
        parent_a_id: parent_a.capsule_id.clone(),
        parent_b_id: parent_b.capsule_id.clone(),
        parent_generations: parent_a.evolved_generation.max(parent_b.evolved_generation) + 1,
        parent_fitness_a: compute_fitness(parent_a),
        parent_fitness_b: compute_fitness(parent_b),
    }
}

pub fn mutate_archetype(
    archetype: HashMap<String, serde_json::Value>,
) -> HashMap<String, serde_json::Value> {
    let mut rng = rand::thread_rng();
    let mut arch = archetype;

    if let Some(kw) = arch.get("trigger_keywords").and_then(|v| v.as_array()) {
        if rng.gen::<f64>() < MUTATION_RATE && !kw.is_empty() {
            let kept: Vec<serde_json::Value> = kw
                .iter()
                .take((kw.len() as f64 * 0.8) as usize)
                .cloned()
                .collect();
            arch.insert("trigger_keywords".to_string(), serde_json::json!(kept));
        }
    }

    if rng.gen::<f64>() < MUTATION_RATE {
        if let Some(fp) = arch.get("algorithm_fingerprint").and_then(|v| v.as_str()) {
            if fp.len() > 4 {
                let chars: Vec<char> = fp.chars().collect();
                let mut new_chars = chars.clone();
                for _ in 0..2 {
                    if !new_chars.is_empty() {
                        let idx = rng.gen_range(0..new_chars.len());
                        let alternatives: Vec<char> = "0123456789abcdef".chars().collect();
                        new_chars[idx] = alternatives[rng.gen_range(0..alternatives.len())];
                    }
                }
                arch.insert(
                    "algorithm_fingerprint".to_string(),
                    serde_json::json!(new_chars.into_iter().collect::<String>()),
                );
            }
        }
    }

    if let Some(refs) = arch.get("paper_section_refs").and_then(|v| v.as_array()) {
        if rng.gen::<f64>() < MUTATION_RATE && !refs.is_empty() {
            let kept: Vec<serde_json::Value> = refs
                .iter()
                .take((refs.len() as f64 * 0.8) as usize)
                .cloned()
                .collect();
            arch.insert("paper_section_refs".to_string(), serde_json::json!(kept));
        }
    }

    if let Some(emb) = arch.get("title_embedding").and_then(|v| v.as_array()) {
        if rng.gen::<f64>() < MUTATION_RATE && !emb.is_empty() {
            let mut new_emb: Vec<serde_json::Value> = emb.to_vec();
            let idx = rng.gen_range(0..new_emb.len());
            if let Some(val) = new_emb[idx].as_f64() {
                new_emb[idx] = serde_json::json!(val + (rng.gen::<f64>() - 0.5) * 0.1);
                arch.insert("title_embedding".to_string(), serde_json::json!(new_emb));
            }
        }
    }

    arch
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct V3Capsule {
    pub capsule_id: String,
    pub action_gap_title: String,
    pub evolved_generation: i32,
    pub success_score: f64,
    pub feedback_count: i32,
    pub fitness: f64,
    pub parent_ids: Vec<String>,
    pub created_at: String,
}

pub fn get_v3_capsules() -> Vec<V3Capsule> {
    let capsules = load_capsules();
    let mut v3: Vec<CapsuleGene> = capsules
        .into_iter()
        .filter(|c| c.evolved_generation >= 1 && c.status == "active")
        .collect();
    v3.sort_by(|a, b| {
        compute_fitness(b)
            .partial_cmp(&compute_fitness(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    v3.into_iter()
        .map(|c| {
            let archetype = c.archetype.clone();
            V3Capsule {
                capsule_id: c.capsule_id.clone(),
                action_gap_title: c.action_gap_title.clone(),
                evolved_generation: c.evolved_generation,
                success_score: c.outcome_success_score,
                feedback_count: c.feedback_count,
                fitness: (compute_fitness(&c) * 1000.0).round() / 1000.0,
                parent_ids: vec![
                    archetype
                        .get("parent_capsule_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    archetype
                        .get("parent_capsule_id_b")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                ],
                created_at: c.created_at,
            }
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopCandidate {
    pub capsule_id: String,
    pub action_gap_title: String,
    pub evolved_generation: i32,
    pub success_score: f64,
    pub feedback_count: i32,
    pub fitness: f64,
    pub capsule_trust: f64,
    pub credibility_badge: String,
}

pub fn get_top_candidates(limit: usize) -> Vec<TopCandidate> {
    let capsules = load_capsules();
    let mut active: Vec<CapsuleGene> = capsules
        .into_iter()
        .filter(|c| c.status == "active" && c.credibility_badge != "low")
        .collect();
    active.sort_by(|a, b| {
        compute_fitness(b)
            .partial_cmp(&compute_fitness(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    active
        .into_iter()
        .take(limit)
        .map(|c| TopCandidate {
            capsule_id: c.capsule_id.clone(),
            action_gap_title: c.action_gap_title.chars().take(60).collect(),
            evolved_generation: c.evolved_generation,
            success_score: (c.outcome_success_score * 1000.0).round() / 1000.0,
            feedback_count: c.feedback_count,
            fitness: (compute_fitness(&c) * 1000.0).round() / 1000.0,
            capsule_trust: 0.0,
            credibility_badge: c.credibility_badge,
        })
        .collect()
}

pub fn run_evolution(
    offspring_count: usize,
    population_size: usize,
) -> HashMap<String, serde_json::Value> {
    run_evolution_with_engine(offspring_count, population_size, None)
}

pub fn run_evolution_with_engine(
    offspring_count: usize,
    population_size: usize,
    mut engine: Option<&mut CrossoverEngine>,
) -> HashMap<String, serde_json::Value> {
    let capsules = load_capsules();
    let mut parents: Vec<CapsuleGene> = capsules
        .into_iter()
        .filter(|c| {
            c.status == "active"
                && c.credibility_badge != "low"
                && (compute_fitness(c) >= MIN_FITNESS_THRESHOLD || c.feedback_count == 0)
        })
        .collect();
    parents.sort_by(|a, b| {
        compute_fitness(b)
            .partial_cmp(&compute_fitness(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    parents.truncate(population_size);

    if parents.len() < 2 {
        let mut result = HashMap::new();
        result.insert(
            "error".to_string(),
            serde_json::json!(format!(
                "Need at least 2 eligible parents, got {}",
                parents.len()
            )),
        );
        result.insert(
            "created".to_string(),
            serde_json::json!(Vec::<serde_json::Value>::new()),
        );
        return result;
    }

    let capsule_ids: Vec<&str> = parents.iter().map(|p| p.capsule_id.as_str()).collect();
    let mut rng = rand::thread_rng();
    let mut created = Vec::new();
    let mut used_pairs: std::collections::HashSet<(String, String)> =
        std::collections::HashSet::new();
    let use_ucb = engine.is_some();

    for _ in 0..offspring_count {
        let (p_a_idx, p_b_idx) = if use_ucb {
            if let Some(eng) = engine.as_mut() {
                if let Some((a_idx, b_idx)) = eng.select_parents_ucb(&capsule_ids) {
                    (a_idx, b_idx)
                } else {
                    continue;
                }
            } else {
                continue;
            }
        } else {
            let a = parents.choose(&mut rng);
            let b = parents.choose(&mut rng);
            match (a, b) {
                (Some(p_a), Some(p_b)) if p_a.capsule_id != p_b.capsule_id => {
                    let a_idx = parents.iter().position(|p| p.capsule_id == p_a.capsule_id).unwrap_or(0);
                    let b_idx = parents.iter().position(|p| p.capsule_id == p_b.capsule_id).unwrap_or(1);
                    (a_idx, b_idx)
                }
                _ => continue,
            }
        };

        let p_a = &parents[p_a_idx];
        let p_b = &parents[p_b_idx];
        if p_a.capsule_id == p_b.capsule_id {
            continue;
        }
        let pair_key = if p_a.capsule_id < p_b.capsule_id {
            (p_a.capsule_id.clone(), p_b.capsule_id.clone())
        } else {
            (p_b.capsule_id.clone(), p_a.capsule_id.clone())
        };
        if used_pairs.contains(&pair_key) && parents.len() >= 3 {
            continue;
        }
        used_pairs.insert(pair_key);

        let mut xo = crossover(p_a, p_b);
        xo.archetype = mutate_archetype(xo.archetype);

        let title_a = p_a.action_gap_title.chars().take(30).collect::<String>();
        let title_b = p_b.action_gap_title.chars().take(30).collect::<String>();
        let v3_title = format!("V3:{} x {}", title_a, title_b);

        created.push(serde_json::json!({
            "capsule_id": format!("{:x}", rng.gen::<u32>()),
            "parent_a_id": xo.parent_a_id,
            "parent_b_id": xo.parent_b_id,
            "generation": xo.parent_generations,
            "fitness_a": (xo.parent_fitness_a * 1000.0).round() / 1000.0,
            "fitness_b": (xo.parent_fitness_b * 1000.0).round() / 1000.0,
            "v3_title": v3_title,
        }));
    }

    let mut result = HashMap::new();
    result.insert(
        "parents_considered".to_string(),
        serde_json::json!(parents.len()),
    );
    result.insert(
        "pairs_tried".to_string(),
        serde_json::json!(used_pairs.len()),
    );
    result.insert("created".to_string(), serde_json::json!(created.clone()));
    result.insert(
        "generation".to_string(),
        serde_json::json!(created
            .iter()
            .filter_map(|c| c.get("generation").and_then(|v| v.as_i64()))
            .max()
            .unwrap_or(0)),
    );
    result
}

pub fn crossover_action(
    action: &str,
    offspring_count: usize,
    _capsule_id: Option<&str>,
    _capsule_id_b: Option<&str>,
    _gap_type: Option<&str>,
) -> HashMap<String, serde_json::Value> {
    let mut result = HashMap::new();

    match action {
        "evolve" => {
            let res = run_evolution(offspring_count, DEFAULT_POPULATION_SIZE);
            for (k, v) in res {
                result.insert(k, v);
            }
        }
        "rank_v3" => {
            let v3 = get_v3_capsules();
            result.insert("v3_capsules".to_string(), serde_json::json!(v3));
            result.insert("total_v3".to_string(), serde_json::json!(v3.len()));
        }
        "best" => {
            let candidates = get_top_candidates(10);
            result.insert("candidates".to_string(), serde_json::json!(candidates));
            result.insert("total".to_string(), serde_json::json!(candidates.len()));
        }
        _ => {
            result.insert(
                "error".to_string(),
                serde_json::json!(format!("Unknown action: {}", action)),
            );
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_fitness_zero_feedback() {
        let cap = CapsuleGene {
            capsule_id: "test".to_string(),
            created_at: "".to_string(),
            trigger_topic: "".to_string(),
            trigger_gap_type: "".to_string(),
            trigger_keywords: vec![],
            action_gap_type: "".to_string(),
            action_gap_title: "".to_string(),
            outcome_success_score: 0.5,
            feedback_count: 0,
            evolved_generation: 0,
            archetype: HashMap::new(),
            status: "active".to_string(),
            low_score_streak: 0,
            credibility_score: 0.5,
            trendslop: false,
            trendslop_reason: "".to_string(),
            source_arxiv_category: "".to_string(),
            credibility_badge: "medium".to_string(),
        };
        assert_eq!(compute_fitness(&cap), 0.0);
    }

    #[test]
    fn test_crossover_empty_archetypes() {
        let cap_a = CapsuleGene {
            capsule_id: "a".to_string(),
            created_at: "".to_string(),
            trigger_topic: "".to_string(),
            trigger_gap_type: "".to_string(),
            trigger_keywords: vec![],
            action_gap_type: "".to_string(),
            action_gap_title: "".to_string(),
            outcome_success_score: 0.5,
            feedback_count: 1,
            evolved_generation: 0,
            archetype: HashMap::new(),
            status: "active".to_string(),
            low_score_streak: 0,
            credibility_score: 0.5,
            trendslop: false,
            trendslop_reason: "".to_string(),
            source_arxiv_category: "".to_string(),
            credibility_badge: "medium".to_string(),
        };
        let cap_b = CapsuleGene {
            capsule_id: "b".to_string(),
            created_at: "".to_string(),
            trigger_topic: "".to_string(),
            trigger_gap_type: "".to_string(),
            trigger_keywords: vec![],
            action_gap_type: "".to_string(),
            action_gap_title: "".to_string(),
            outcome_success_score: 0.5,
            feedback_count: 1,
            evolved_generation: 0,
            archetype: HashMap::new(),
            status: "active".to_string(),
            low_score_streak: 0,
            credibility_score: 0.5,
            trendslop: false,
            trendslop_reason: "".to_string(),
            source_arxiv_category: "".to_string(),
            credibility_badge: "medium".to_string(),
        };
        let result = crossover(&cap_a, &cap_b);
        assert_eq!(result.parent_a_id, "a");
        assert_eq!(result.parent_b_id, "b");
    }

    #[test]
    fn test_mutate_archetype() {
        let arch = HashMap::new();
        let result = mutate_archetype(arch);
        assert!(result.is_empty());
    }

    #[test]
    fn test_get_v3_capsules_smoke() {
        let v3 = get_v3_capsules();
        // Check that every v3 capsule has required fields
        for capsule in &v3 {
            assert!(!capsule.capsule_id.is_empty(), "capsule_id must not be empty");
            assert!(!capsule.action_gap_title.is_empty(), "action_gap_title must not be empty");
        }
    }

    #[test]
    fn test_crossover_action_unknown() {
        let result = crossover_action("unknown", 5, None, None, None);
        assert!(result.contains_key("error"));
    }
}
