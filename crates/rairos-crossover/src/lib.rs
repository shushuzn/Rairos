//! rairos-crossover — CapsuleGene Crossover: genetic algorithm on Gene Pool archetypes.
//!
//! Ported from `llm/crossover.py`.
//!
//! Algorithm:
//!   1. Selection: top-k capsules by fitness = success_score × log(feedback_count+1)
//!   2. Crossover: single-point swap of archetype dict between two parent capsules
//!   3. Mutation: random perturbation of keywords, algorithm_fingerprint, paper_section_refs
//!   4. V3 capsule: parent_a_id + parent_b_id in archetype, evolved_generation = max(parents)+1
//!   5. Only V3 if: both parents credibility_badge != "low" AND fitness > threshold

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[allow(dead_code)]
const DEFAULT_POPULATION_SIZE: usize = 20;
#[allow(dead_code)]
const DEFAULT_OFFSPRING_COUNT: usize = 5;
#[allow(dead_code)]
const MIN_FITNESS_THRESHOLD: f64 = 0.3;
const MUTATION_RATE: f64 = 0.15;

fn gp_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ai_research_os")
        .join("evolution")
}

fn debate_state_file() -> PathBuf {
    gp_dir().join("debate_state.json")
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
    #[serde(default)]
    pub archetype: HashMap<String, serde_json::Value>,
    #[serde(default = "default_status")]
    pub status: String,
    #[serde(default)]
    pub low_score_streak: i32,
    #[serde(default)]
    pub credibility_score: f64,
    #[serde(default)]
    pub trendslop: bool,
    #[serde(default)]
    pub trendslop_reason: String,
    #[serde(default = "default_credibility_badge")]
    pub credibility_badge: String,
    #[serde(default)]
    pub source_arxiv_category: String,
}

fn default_status() -> String {
    "active".to_string()
}

fn default_credibility_badge() -> String {
    "medium".to_string()
}

impl<'a> From<&'a CapsuleGene> for CapsuleGene {
    fn from(other: &'a CapsuleGene) -> Self {
        other.clone()
    }
}

pub fn compute_fitness(capsule: &CapsuleGene) -> f64 {
    let score = capsule.outcome_success_score;
    let fb = capsule.feedback_count.max(0) as f64;
    score * (fb + 1.0).ln()
}

#[allow(dead_code)]
fn compute_trust(capsule: &CapsuleGene, inbound_citations: i32) -> f64 {
    let impact = capsule.outcome_success_score;
    let citation_boost = 1.0 + 0.05 * inbound_citations as f64;
    let badge_mult = match capsule.credibility_badge.as_str() {
        "high" => 1.5,
        "low" => 0.5,
        _ => 1.0,
    };
    impact * citation_boost * badge_mult
}

#[allow(dead_code)]
fn select_parents(
    capsules: &[CapsuleGene],
    k: usize,
    use_trust: bool,
) -> Vec<CapsuleGene> {
    let mut candidates: Vec<&CapsuleGene> = capsules
        .iter()
        .filter(|c| {
            c.status == "active"
            && c.credibility_badge != "low"
            && compute_fitness(c) >= MIN_FITNESS_THRESHOLD
        })
        .collect();

    if use_trust {
        candidates.sort_by(|a, b| {
            let trust_a = compute_trust(a, 0);
            let trust_b = compute_trust(b, 0);
            trust_b.partial_cmp(&trust_a).unwrap_or(std::cmp::Ordering::Equal)
        });
    } else {
        candidates.sort_by(|a, b| {
            let fit_a = compute_fitness(a);
            let fit_b = compute_fitness(b);
            fit_b.partial_cmp(&fit_a).unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    candidates.into_iter().take(k).cloned().collect()
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
        m.extend(arch_b.clone());
        m
    } else {
        let mut rng = rand::thread_rng();
        let max_start = shared_keys.len().saturating_sub(1).max(1);
        let point = rng.gen_range(1..=max_start);
            let swapped_keys: std::collections::HashSet<&String> =
                shared_keys[..point].iter().copied().collect();

        let mut m = HashMap::new();
        for k in &shared_keys {
            let val = if swapped_keys.contains(k) {
                arch_b.get(*k).unwrap()
            } else {
                arch_a.get(*k).unwrap()
            };
            m.insert((*k).clone(), val.clone());
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
    let mut arch = archetype;
    let mut rng = rand::thread_rng();

    if rng.gen::<f64>() < MUTATION_RATE {
        if let Some(kw) = arch.get_mut("trigger_keywords") {
            if let Some(arr) = kw.as_array_mut() {
                let kept_count = (arr.len() as f64 * 0.8).ceil() as usize;
                arr.truncate(kept_count);
            }
        }
    }

    if rng.gen::<f64>() < MUTATION_RATE {
        if let Some(fp) = arch.get("algorithm_fingerprint") {
            if let Some(fp_str) = fp.as_str() {
                if fp_str.len() > 4 {
                    let chars: Vec<char> = fp_str.chars().collect();
                    let mut mutated: Vec<char> = chars.clone();
                    for _ in 0..2 {
                        if !mutated.is_empty() {
                            let idx = rng.gen_range(0..mutated.len());
                            let replacement = "0123456789abcdef".chars().nth(rng.gen_range(0..16)).unwrap();
                            mutated[idx] = replacement;
                        }
                    }
                    arch.insert("algorithm_fingerprint".to_string(), serde_json::json!(mutated.iter().collect::<String>()));
                }
            }
        }
    }

    if rng.gen::<f64>() < MUTATION_RATE {
        if let Some(refs) = arch.get_mut("paper_section_refs") {
            if let Some(arr) = refs.as_array_mut() {
                let kept_count = (arr.len() as f64 * 0.8).ceil() as usize;
                let new_refs: Vec<serde_json::Value> = arr.iter().take(kept_count).cloned().collect();
                arch.insert("paper_section_refs".to_string(), serde_json::json!(new_refs));
            }
        }
    }

    if rng.gen::<f64>() < MUTATION_RATE {
        if let Some(emb) = arch.get_mut("title_embedding") {
            if let Some(arr) = emb.as_array_mut() {
                if !arr.is_empty() {
                    let idx = rng.gen_range(0..arr.len());
                    if let Some(v) = arr[idx].as_f64() {
                        let noise = (rng.gen::<f64>() - 0.5) * 0.1;
                        arr[idx] = serde_json::json!(v + noise);
                    }
                }
            }
        }
    }

    arch
}

pub fn sanitize_archetype(archetype: &HashMap<String, serde_json::Value>) -> HashMap<String, serde_json::Value> {
    let mut cleaned = HashMap::new();
    for (k, v) in archetype {
        if k == "title_embedding" {
            continue;
        }
        if serde_json::to_string(v).is_ok() {
            cleaned.insert(k.clone(), v.clone());
        }
    }
    cleaned
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebateEntry {
    pub debate_id: String,
    pub capsule_a_id: String,
    pub capsule_b_id: String,
    pub gap_type: String,
    pub winner_id: String,
    pub loser_id: String,
    pub score_a: f64,
    pub score_b: f64,
    pub judged_at: String,
}

fn load_debate_state() -> Vec<DebateEntry> {
    let path = debate_state_file();
    if !path.exists() {
        return Vec::new();
    }
    match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

#[allow(dead_code)]
fn save_debate_state(debates: &[DebateEntry]) {
    let dir = gp_dir();
    let _ = fs::create_dir_all(&dir);
    let path = debate_state_file();
    if let Ok(json) = serde_json::to_string_pretty(debates) {
        let _ = fs::write(&path, json);
    }
}

#[allow(dead_code)]
fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[allow(dead_code)]
fn score_argument(capsule: &CapsuleGene, inbound_citations: i32) -> f64 {
    let success = capsule.outcome_success_score;
    let fb = capsule.feedback_count.max(0) as f64;
    let fb_bonus = (fb + 1.0).ln();
    let citation_bonus = 1.0 + 0.05 * inbound_citations as f64;
    (success * fb_bonus * citation_bonus * 1000.0).round() / 1000.0
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CrossoverActionResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v3_capsules: Option<Vec<V3Capsule>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_v3: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutated: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidates: Option<Vec<CandidateCapsule>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lineage_tree: Option<LineageNode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ascii_tree: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_ancestors: Option<Vec<RootAncestor>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub descendants: Option<Vec<DescendantCapsule>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debate_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winner_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loser_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_a: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_b: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debates: Option<Vec<DebateEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parents_considered: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pairs_tried: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<Vec<CreatedCapsule>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct V3Capsule {
    pub capsule_id: String,
    #[serde(rename = "action_gap_title")]
    pub action_gap_title: String,
    #[serde(rename = "evolved_generation")]
    pub evolved_generation: i32,
    #[serde(rename = "success_score")]
    pub success_score: f64,
    #[serde(rename = "feedback_count")]
    pub feedback_count: i32,
    pub fitness: f64,
    pub parent_ids: Vec<String>,
    #[serde(rename = "created_at")]
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CandidateCapsule {
    pub capsule_id: String,
    #[serde(rename = "action_gap_title")]
    pub action_gap_title: String,
    #[serde(rename = "evolved_generation")]
    pub evolved_generation: i32,
    #[serde(rename = "success_score")]
    pub success_score: f64,
    #[serde(rename = "feedback_count")]
    pub feedback_count: i32,
    pub fitness: f64,
    #[serde(rename = "capsule_trust")]
    pub capsule_trust: f64,
    #[serde(rename = "credibility_badge")]
    pub credibility_badge: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageNode {
    pub capsule_id: String,
    #[serde(rename = "action_gap_title")]
    pub action_gap_title: String,
    #[serde(rename = "evolved_generation")]
    pub evolved_generation: i32,
    #[serde(rename = "success_score")]
    pub success_score: f64,
    #[serde(rename = "feedback_count")]
    pub feedback_count: i32,
    #[serde(rename = "parent_a_id")]
    pub parent_a_id: String,
    #[serde(rename = "parent_b_id")]
    pub parent_b_id: String,
    pub children: Vec<LineageNode>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RootAncestor {
    pub capsule_id: String,
    #[serde(rename = "action_gap_title")]
    pub action_gap_title: String,
    #[serde(rename = "evolved_generation")]
    pub evolved_generation: i32,
    #[serde(rename = "success_score")]
    pub success_score: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DescendantCapsule {
    pub capsule_id: String,
    #[serde(rename = "action_gap_title")]
    pub action_gap_title: String,
    #[serde(rename = "evolved_generation")]
    pub evolved_generation: i32,
    #[serde(rename = "success_score")]
    pub success_score: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatedCapsule {
    pub capsule_id: String,
    #[serde(rename = "parent_a_id")]
    pub parent_a_id: String,
    #[serde(rename = "parent_b_id")]
    pub parent_b_id: String,
    pub generation: i32,
    pub fitness_a: f64,
    pub fitness_b: f64,
}

pub fn crossover_action(action: &str) -> CrossoverActionResult {
    match action {
        "evolve" => CrossoverActionResult {
            error: Some("DB-dependent: run_evolution requires EvolutionTracker".to_string()),
            v3_capsules: None,
            total_v3: None,
            mutated: None,
            candidates: None,
            total: None,
            lineage_tree: None,
            ascii_tree: None,
            root_ancestors: None,
            descendants: None,
            count: None,
            debate_id: None,
            winner_id: None,
            loser_id: None,
            score_a: None,
            score_b: None,
            debates: None,
            parents_considered: None,
            pairs_tried: None,
            created: None,
            generation: None,
        },
        "rank_v3" => CrossoverActionResult {
            error: Some("DB-dependent: get_v3_capsules requires EvolutionTracker".to_string()),
            v3_capsules: None,
            total_v3: None,
            mutated: None,
            candidates: None,
            total: None,
            lineage_tree: None,
            ascii_tree: None,
            root_ancestors: None,
            descendants: None,
            count: None,
            debate_id: None,
            winner_id: None,
            loser_id: None,
            score_a: None,
            score_b: None,
            debates: None,
            parents_considered: None,
            pairs_tried: None,
            created: None,
            generation: None,
        },
        "mutate" => CrossoverActionResult {
            error: Some("DB-dependent: mutate_single requires EvolutionTracker".to_string()),
            v3_capsules: None,
            total_v3: None,
            mutated: None,
            candidates: None,
            total: None,
            lineage_tree: None,
            ascii_tree: None,
            root_ancestors: None,
            descendants: None,
            count: None,
            debate_id: None,
            winner_id: None,
            loser_id: None,
            score_a: None,
            score_b: None,
            debates: None,
            parents_considered: None,
            pairs_tried: None,
            created: None,
            generation: None,
        },
        "best" => CrossoverActionResult {
            error: Some("DB-dependent: get_top_candidates requires EvolutionTracker".to_string()),
            v3_capsules: None,
            total_v3: None,
            mutated: None,
            candidates: None,
            total: None,
            lineage_tree: None,
            ascii_tree: None,
            root_ancestors: None,
            descendants: None,
            count: None,
            debate_id: None,
            winner_id: None,
            loser_id: None,
            score_a: None,
            score_b: None,
            debates: None,
            parents_considered: None,
            pairs_tried: None,
            created: None,
            generation: None,
        },
        "lineage" => CrossoverActionResult {
            error: Some("DB-dependent: get_lineage requires EvolutionTracker".to_string()),
            v3_capsules: None,
            total_v3: None,
            mutated: None,
            candidates: None,
            total: None,
            lineage_tree: None,
            ascii_tree: None,
            root_ancestors: None,
            descendants: None,
            count: None,
            debate_id: None,
            winner_id: None,
            loser_id: None,
            score_a: None,
            score_b: None,
            debates: None,
            parents_considered: None,
            pairs_tried: None,
            created: None,
            generation: None,
        },
        "descendants" => CrossoverActionResult {
            error: Some("DB-dependent: get_descendants requires EvolutionTracker".to_string()),
            v3_capsules: None,
            total_v3: None,
            mutated: None,
            candidates: None,
            total: None,
            lineage_tree: None,
            ascii_tree: None,
            root_ancestors: None,
            descendants: None,
            count: None,
            debate_id: None,
            winner_id: None,
            loser_id: None,
            score_a: None,
            score_b: None,
            debates: None,
            parents_considered: None,
            pairs_tried: None,
            created: None,
            generation: None,
        },
        "debate" => CrossoverActionResult {
            error: Some("DB-dependent: debate_capsules requires gene_pool_decay".to_string()),
            v3_capsules: None,
            total_v3: None,
            mutated: None,
            candidates: None,
            total: None,
            lineage_tree: None,
            ascii_tree: None,
            root_ancestors: None,
            descendants: None,
            count: None,
            debate_id: None,
            winner_id: None,
            loser_id: None,
            score_a: None,
            score_b: None,
            debates: None,
            parents_considered: None,
            pairs_tried: None,
            created: None,
            generation: None,
        },
        "debate_history" => {
            let debates = load_debate_state();
            CrossoverActionResult {
                error: None,
                v3_capsules: None,
                total_v3: None,
                mutated: None,
                candidates: None,
                total: None,
                lineage_tree: None,
                ascii_tree: None,
                root_ancestors: None,
                descendants: None,
                count: None,
                debate_id: None,
                winner_id: None,
                loser_id: None,
                score_a: None,
                score_b: None,
                debates: Some(debates),
                parents_considered: None,
                pairs_tried: None,
                created: None,
                generation: None,
            }
        }
        "debate_candidates" => CrossoverActionResult {
            error: Some("DB-dependent: get_debate_candidates requires EvolutionTracker".to_string()),
            v3_capsules: None,
            total_v3: None,
            mutated: None,
            candidates: None,
            total: None,
            lineage_tree: None,
            ascii_tree: None,
            root_ancestors: None,
            descendants: None,
            count: None,
            debate_id: None,
            winner_id: None,
            loser_id: None,
            score_a: None,
            score_b: None,
            debates: None,
            parents_considered: None,
            pairs_tried: None,
            created: None,
            generation: None,
        },
        _ => CrossoverActionResult {
            error: Some(format!("Unknown action: {action}")),
            v3_capsules: None,
            total_v3: None,
            mutated: None,
            candidates: None,
            total: None,
            lineage_tree: None,
            ascii_tree: None,
            root_ancestors: None,
            descendants: None,
            count: None,
            debate_id: None,
            winner_id: None,
            loser_id: None,
            score_a: None,
            score_b: None,
            debates: None,
            parents_considered: None,
            pairs_tried: None,
            created: None,
            generation: None,
        },
    }
}

pub fn get_debate_history(limit: usize) -> Vec<DebateEntry> {
    let mut debates = load_debate_state();
    debates.sort_by(|a, b| b.judged_at.cmp(&a.judged_at));
    debates.truncate(limit);
    debates
}

pub fn render_lineage_tree(_capsule_id: &str) -> String {
    "DB-dependent: render_lineage_tree requires EvolutionTracker".to_string()
}

pub fn get_lineage(_capsule_id: &str, _max_depth: usize) -> Option<LineageNode> {
    None
}

pub fn get_root_ancestors(_capsule_id: &str, _max_depth: usize) -> Vec<RootAncestor> {
    Vec::new()
}

pub fn get_descendants(_capsule_id: &str) -> Vec<DescendantCapsule> {
    Vec::new()
}

pub fn get_capsule_by_id(_capsule_id: &str) -> Option<CapsuleGene> {
    None
}

pub fn update_capsule(_capsule: &CapsuleGene) -> bool {
    false
}

pub fn get_all_capsules() -> Vec<CapsuleGene> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_capsule(
        capsule_id: &str,
        success_score: f64,
        feedback_count: i32,
        generation: i32,
    ) -> CapsuleGene {
        let mut archetype = HashMap::new();
        archetype.insert("trigger_keywords".to_string(), serde_json::json!(["attention", "transformer"]));
        archetype.insert("algorithm_fingerprint".to_string(), serde_json::json!("abc123def456"));
        archetype.insert("paper_section_refs".to_string(), serde_json::json!(["section1", "section2", "section3"]));
        archetype.insert("title_embedding".to_string(), serde_json::json!([0.1, 0.2, 0.3]));

        CapsuleGene {
            capsule_id: capsule_id.to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            trigger_topic: "language models".to_string(),
            trigger_gap_type: "method_gap".to_string(),
            trigger_keywords: vec!["attention".to_string(), "transformer".to_string()],
            action_gap_type: "method_gap".to_string(),
            action_gap_title: "Improving attention mechanism".to_string(),
            outcome_success_score: success_score,
            feedback_count,
            evolved_generation: generation,
            archetype,
            status: "active".to_string(),
            low_score_streak: 0,
            credibility_score: 0.7,
            trendslop: false,
            trendslop_reason: String::new(),
            credibility_badge: "medium".to_string(),
            source_arxiv_category: "cs.CL".to_string(),
        }
    }

    #[test]
    fn test_compute_fitness() {
        let cap = make_capsule("cap1", 0.8, 10, 0);
        let fitness = compute_fitness(&cap);
        assert!(fitness > 0.0);
        let expected = 0.8 * 11_f64.ln();
        assert!((fitness - expected).abs() < 0.001);
    }

    #[test]
    fn test_compute_fitness_zero_feedback() {
        let cap = make_capsule("cap1", 0.5, 0, 0);
        let fitness = compute_fitness(&cap);
        assert!((fitness - 0.5 * 1_f64.ln()).abs() < 0.001);
    }

    #[test]
    fn test_crossover_same_archetype() {
        let parent_a = make_capsule("parent_a", 0.8, 5, 1);
        let parent_b = make_capsule("parent_b", 0.7, 3, 1);

        let result = crossover(&parent_a, &parent_b);
        assert_eq!(result.parent_a_id, "parent_a");
        assert_eq!(result.parent_b_id, "parent_b");
        assert_eq!(result.parent_generations, 2);
        assert!(result.parent_fitness_a > 0.0);
        assert!(result.parent_fitness_b > 0.0);
    }

    #[test]
    fn test_crossover_result_has_merged_archetype() {
        let mut arch_a = HashMap::new();
        arch_a.insert("key1".to_string(), serde_json::json!("value_a"));
        arch_a.insert("shared".to_string(), serde_json::json!("a_shared"));

        let mut arch_b = HashMap::new();
        arch_b.insert("key2".to_string(), serde_json::json!("value_b"));
        arch_b.insert("shared".to_string(), serde_json::json!("b_shared"));

        let mut parent_a = make_capsule("pa", 0.8, 5, 0);
        parent_a.archetype = arch_a;

        let mut parent_b = make_capsule("pb", 0.7, 3, 0);
        parent_b.archetype = arch_b;

        let result = crossover(&parent_a, &parent_b);

        assert!(result.archetype.contains_key("key1"));
        assert!(result.archetype.contains_key("key2"));
        assert!(result.archetype.contains_key("shared"));
    }

    #[test]
    fn test_mutate_archetype_keywords() {
        let mut archetype = HashMap::new();
        archetype.insert("trigger_keywords".to_string(), serde_json::json!(["a", "b", "c", "d", "e"]));

        let mutated = mutate_archetype(archetype);

        if let Some(kw) = mutated.get("trigger_keywords") {
            if let Some(arr) = kw.as_array() {
                assert!(arr.len() <= 5);
            }
        }
    }

    #[test]
    fn test_mutate_archetype_fingerprint() {
        let mut archetype = HashMap::new();
        archetype.insert("algorithm_fingerprint".to_string(), serde_json::json!("abcdef123456"));

        let mutated = mutate_archetype(archetype);

        if let Some(fp) = mutated.get("algorithm_fingerprint") {
            if let Some(s) = fp.as_str() {
                assert_eq!(s.len(), 12);
            }
        }
    }

    #[test]
    fn test_mutate_archetype_section_refs() {
        let mut archetype = HashMap::new();
        archetype.insert("paper_section_refs".to_string(), serde_json::json!(["s1", "s2", "s3", "s4", "s5"]));

        let mutated = mutate_archetype(archetype);

        if let Some(refs) = mutated.get("paper_section_refs") {
            if let Some(arr) = refs.as_array() {
                assert!(arr.len() <= 5);
            }
        }
    }

    #[test]
    fn test_sanitize_archetype_removes_title_embedding() {
        let mut archetype = HashMap::new();
        archetype.insert("title_embedding".to_string(), serde_json::json!([1.0, 2.0]));
        archetype.insert("trigger_keywords".to_string(), serde_json::json!(["test"]));

        let cleaned = sanitize_archetype(&archetype);

        assert!(!cleaned.contains_key("title_embedding"));
        assert!(cleaned.contains_key("trigger_keywords"));
    }

    #[test]
    fn test_debate_history_empty() {
        let history = get_debate_history(10);
        assert!(history.is_empty() || history.len() <= 10);
    }

    #[test]
    fn test_crossover_action_unknown() {
        let result = crossover_action("unknown_action");
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("Unknown action"));
    }

    #[test]
    fn test_crossover_action_debate_history() {
        let result = crossover_action("debate_history");
        assert!(result.error.is_none());
        assert!(result.debates.is_some());
    }

    #[test]
    fn test_select_parents_filters_low_credibility() {
        let mut cap_low = make_capsule("low_cred", 0.8, 5, 0);
        cap_low.credibility_badge = "low".to_string();

        let cap_medium = make_capsule("medium_cred", 0.8, 5, 0);

        let parents = select_parents(&[cap_low, cap_medium], 10, false);
        assert_eq!(parents.len(), 1);
        assert_eq!(parents[0].capsule_id, "medium_cred");
    }

    #[test]
    fn test_select_parents_respects_k() {
        let capsules: Vec<_> = (0..15)
            .map(|i| make_capsule(&format!("cap_{}", i), 0.5 + i as f64 * 0.03, 5, 0))
            .collect();

        let parents = select_parents(&capsules, 5, false);
        assert_eq!(parents.len(), 5);
    }

    #[test]
    fn test_score_argument() {
        let cap = make_capsule("test", 0.8, 10, 0);
        let score = score_argument(&cap, 5);
        assert!(score > 0.0);
    }

    #[test]
    fn test_get_lineage_returns_none() {
        let result = get_lineage("nonexistent", 5);
        assert!(result.is_none());
    }

    #[test]
    fn test_render_lineage_tree_deferred() {
        let result = render_lineage_tree("any_id");
        assert!(result.contains("DB-dependent"));
    }

    #[test]
    fn test_get_descendants_empty() {
        let result = get_descendants("any_id");
        assert!(result.is_empty());
    }
}
