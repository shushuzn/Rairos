//! rairos-crossover — CapsuleGene Crossover
//!
//! Genetic algorithm on Gene Pool archetypes.

use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const GP_DIR: &str = ".ai_research_os/evolution";
const DEFAULT_POPULATION_SIZE: usize = 20;
const _DEFAULT_OFFSPRING_COUNT: usize = 5;
const MIN_FITNESS_THRESHOLD: f64 = 0.3;
const MUTATION_RATE: f64 = 0.15;

fn gp_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(GP_DIR)
}

fn gene_pool_path() -> PathBuf {
    gp_dir().join("gene_pool.jsonl")
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
        Some(Self {
            capsule_id: value.get("capsule_id")?.as_str()?.to_string(),
            created_at: value.get("created_at")?.as_str()?.to_string(),
            trigger_topic: value.get("trigger_topic")?.as_str()?.to_string(),
            trigger_gap_type: value.get("trigger_gap_type")?.as_str()?.to_string(),
            trigger_keywords: value.get("trigger_keywords")?.as_array()?.iter().filter_map(|v| v.as_str().map(String::from)).collect(),
            action_gap_type: value.get("action_gap_type")?.as_str()?.to_string(),
            action_gap_title: value.get("action_gap_title")?.as_str()?.to_string(),
            outcome_success_score: value.get("outcome_success_score")?.as_f64().unwrap_or(0.0),
            feedback_count: value.get("feedback_count")?.as_i64().unwrap_or(0) as i32,
            evolved_generation: value.get("evolved_generation")?.as_i64().unwrap_or(0) as i32,
            archetype: value.get("archetype").and_then(|v| v.as_object().cloned()).map(|m| m.into_iter().collect()).unwrap_or_default(),
            status: value.get("status")?.as_str()?.to_string(),
            low_score_streak: value.get("low_score_streak").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            credibility_score: value.get("credibility_score").and_then(|v| v.as_f64()).unwrap_or(0.5),
            trendslop: value.get("trendslop").and_then(|v| v.as_bool()).unwrap_or(false),
            trendslop_reason: value.get("trendslop_reason").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            source_arxiv_category: value.get("source_arxiv_category").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            credibility_badge: value.get("credibility_badge").and_then(|v| v.as_str()).unwrap_or("medium").to_string(),
        })
    }

    pub fn trigger_match(&self, topic: &str, gap_type: &str, keywords: &[String]) -> f64 {
        let mut score = 0.0;

        // 1. action_gap_title substring match — strongest recall signal
        let action_title = self.action_gap_title.trim();
        if !action_title.is_empty() && !topic.is_empty() {
            if action_title.to_lowercase().contains(&topic.to_lowercase()) {
                score += 0.5;
            } else if topic.to_lowercase().contains(&action_title.to_lowercase()) {
                score += 0.3;
            }
        }

        // 2. trigger_topic substring match
        if !topic.is_empty() && !self.trigger_topic.is_empty() {
            if self.trigger_topic.to_lowercase().contains(&topic.to_lowercase()) {
                score += 0.3;
            } else if topic.to_lowercase().contains(&self.trigger_topic.to_lowercase()) {
                score += 0.2;
            }
        }

        // 3. Gap type exact match + partial category match
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

        // 4. Keyword overlap
        if !keywords.is_empty() && !self.trigger_keywords.is_empty() {
            let kw_set: std::collections::HashSet<String> =
                keywords.iter().map(|k| k.to_lowercase()).collect();
            let trigger_set: std::collections::HashSet<String> =
                self.trigger_keywords.iter().map(|k| k.to_lowercase()).collect();
            let overlap: std::collections::HashSet<_> = kw_set.intersection(&trigger_set).collect();
            if !overlap.is_empty() {
                let denom = keywords.len().max(self.trigger_keywords.len());
                score += 0.15 * (overlap.len() as f64 / denom as f64);
            }
        }

        // 5. Token-level Jaccard on topic vs action_gap_title
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
    let private_a: HashMap<String, serde_json::Value> = arch_a.iter().filter(|(k, _)| !arch_b.contains_key(*k)).map(|(k, v)| (k.clone(), v.clone())).collect();
    let private_b: HashMap<String, serde_json::Value> = arch_b.iter().filter(|(k, _)| !arch_a.contains_key(*k)).map(|(k, v)| (k.clone(), v.clone())).collect();

    let merged = if shared_keys.is_empty() {
        let mut m = arch_a.clone();
        m.extend(arch_b);
        m
    } else {
        let mut rng = rand::thread_rng();
        let point = rng.gen_range(1..=shared_keys.len().max(1));
        let swapped_keys: std::collections::HashSet<&String> = shared_keys[..point].iter().cloned().collect();

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

pub fn mutate_archetype(archetype: HashMap<String, serde_json::Value>) -> HashMap<String, serde_json::Value> {
    let mut rng = rand::thread_rng();
    let mut arch = archetype;

    if let Some(kw) = arch.get("trigger_keywords").and_then(|v| v.as_array()) {
        if rng.gen::<f64>() < MUTATION_RATE && !kw.is_empty() {
            let kept: Vec<serde_json::Value> = kw.iter().take((kw.len() as f64 * 0.8) as usize).cloned().collect();
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
                arch.insert("algorithm_fingerprint".to_string(), serde_json::json!(new_chars.into_iter().collect::<String>()));
            }
        }
    }

    if let Some(refs) = arch.get("paper_section_refs").and_then(|v| v.as_array()) {
        if rng.gen::<f64>() < MUTATION_RATE && !refs.is_empty() {
            let kept: Vec<serde_json::Value> = refs.iter().take((refs.len() as f64 * 0.8) as usize).cloned().collect();
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
    let mut v3: Vec<CapsuleGene> = capsules.into_iter()
        .filter(|c| c.evolved_generation >= 1 && c.status == "active")
        .collect();
    v3.sort_by(|a, b| compute_fitness(b).partial_cmp(&compute_fitness(a)).unwrap_or(std::cmp::Ordering::Equal));

    v3.into_iter().map(|c| {
        let archetype = c.archetype.clone();
        V3Capsule {
            capsule_id: c.capsule_id.clone(),
            action_gap_title: c.action_gap_title.clone(),
            evolved_generation: c.evolved_generation,
            success_score: c.outcome_success_score,
            feedback_count: c.feedback_count,
            fitness: (compute_fitness(&c) * 1000.0).round() / 1000.0,
            parent_ids: vec![
                archetype.get("parent_capsule_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                archetype.get("parent_capsule_id_b").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            ],
            created_at: c.created_at,
        }
    }).collect()
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
    let mut active: Vec<CapsuleGene> = capsules.into_iter()
        .filter(|c| c.status == "active" && c.credibility_badge != "low")
        .collect();
    active.sort_by(|a, b| compute_fitness(b).partial_cmp(&compute_fitness(a)).unwrap_or(std::cmp::Ordering::Equal));

    active.into_iter().take(limit).map(|c| {
        TopCandidate {
            capsule_id: c.capsule_id.clone(),
            action_gap_title: c.action_gap_title.chars().take(60).collect(),
            evolved_generation: c.evolved_generation,
            success_score: (c.outcome_success_score * 1000.0).round() / 1000.0,
            feedback_count: c.feedback_count,
            fitness: (compute_fitness(&c) * 1000.0).round() / 1000.0,
            capsule_trust: 0.0,
            credibility_badge: c.credibility_badge,
        }
    }).collect()
}

pub fn run_evolution(offspring_count: usize, population_size: usize) -> HashMap<String, serde_json::Value> {
    let capsules = load_capsules();
    let mut parents: Vec<CapsuleGene> = capsules.into_iter()
        .filter(|c| c.status == "active" && c.credibility_badge != "low" && compute_fitness(c) >= MIN_FITNESS_THRESHOLD)
        .collect();
    parents.sort_by(|a, b| compute_fitness(b).partial_cmp(&compute_fitness(a)).unwrap_or(std::cmp::Ordering::Equal));
    parents.truncate(population_size);

    if parents.len() < 2 {
        let mut result = HashMap::new();
        result.insert("error".to_string(), serde_json::json!(format!("Need at least 2 eligible parents, got {}", parents.len())));
        result.insert("created".to_string(), serde_json::json!(Vec::<serde_json::Value>::new()));
        return result;
    }

    let mut rng = rand::thread_rng();
    let mut created = Vec::new();
    let mut used_pairs: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();

    for _ in 0..offspring_count {
        let p_a = parents.choose(&mut rng).unwrap();
        let p_b = parents.choose(&mut rng).unwrap();
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
    result.insert("parents_considered".to_string(), serde_json::json!(parents.len()));
    result.insert("pairs_tried".to_string(), serde_json::json!(used_pairs.len()));
    result.insert("created".to_string(), serde_json::json!(created.clone()));
    result.insert("generation".to_string(), serde_json::json!(created.iter().filter_map(|c| c.get("generation").and_then(|v| v.as_i64())).max().unwrap_or(0)));
    result
}

pub fn crossover_action(action: &str, offspring_count: usize, _capsule_id: Option<&str>, _capsule_id_b: Option<&str>, _gap_type: Option<&str>) -> HashMap<String, serde_json::Value> {
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
            result.insert("error".to_string(), serde_json::json!(format!("Unknown action: {}", action)));
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
            archetype: None,
            status: "active".to_string(),
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
            archetype: Some(HashMap::new()),
            status: "active".to_string(),
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
            archetype: Some(HashMap::new()),
            status: "active".to_string(),
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
    fn test_get_v3_capsules_empty() {
        let v3 = get_v3_capsules();
        assert!(v3.is_empty());
    }

    #[test]
    fn test_crossover_action_unknown() {
        let result = crossover_action("unknown", 5, None, None, None);
        assert!(result.contains_key("error"));
    }
}
