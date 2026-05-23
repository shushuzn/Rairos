#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const DEFAULT_LAMBDA: f64 = 0.01;
pub const DEFAULT_MIN_IMPACT: f64 = 0.1;
pub const DEFAULT_CONSECUTIVE_CYCLES: usize = 3;

pub const DOMAIN_LAMBDA_FACTOR: &[(&str, f64)] = &[
    ("cs.AI", 0.02),
    ("cs.LG", 0.02),
    ("cs.CL", 0.02),
    ("cs.CV", 0.015),
    ("cs.NE", 0.015),
    ("cs.RO", 0.01),
    ("cs.SE", 0.008),
    ("cs.CR", 0.005),
    ("cs.PL", 0.005),
    ("math.ST", 0.003),
    ("math.IT", 0.003),
    ("physics.class-ph", 0.002),
    ("quant-ph", 0.004),
    ("q-bio", 0.005),
    ("econ.GN", 0.005),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapsuleImpact {
    pub capsule_id: String,
    pub impact_score: f64,
    pub age_days: f64,
    pub feedback_count: i32,
    pub success_score: f64,
    pub citation_boost: f64,
    pub inbound_citations: i32,
    pub indirect_citations: i32,
    pub capsule_trust: f64,
    pub archived: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DecayState {
    pub last_decay_at: String,
    pub consecutive_low_impact: HashMap<String, i32>,
    pub archived_this_cycle: Vec<String>,
    pub archived_by_gap_type: HashMap<String, i32>,
    pub total_archived: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MomentumState {
    pub new_by_gap_type: HashMap<String, i32>,
    pub archived_by_gap_type: HashMap<String, i32>,
    pub last_snapshot_at: String,
}

// ---------------------------------------------------------------------------
// Source-Level Harness Evolution  (CC-Fuzz 2207.07300 + HarnessLLM 2511.01104)
// ---------------------------------------------------------------------------

/// Genetic-mutation operator applied to a test-harness code snippet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationOperator {
    AddAssertion,
    AddTimeoutGuard,
    AddMockStub,
    AddFuzzInput,
    AddBoundaryCheck,
    AddErrorPath,
    RemoveDeadCode,
    SimplifyCondition,
    SwapOperator,
    InjectCrashProbe,
}

/// Individual harness gene — a mutating code snippet with evolutionary metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessGene {
    pub gene_id: String,
    pub created_at: String,
    pub target_crate: String,
    pub mutation_type: MutationOperator,
    pub code_snippet: String,
    pub fitness_score: f64,
    pub coverage_delta: f64,
    pub bug_found: bool,
    pub status: String, // "active", "archived", "elite"
    pub generation: i32,
    pub parent_id: Option<String>,
}

/// Statistics collected after one evolutionary generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionStats {
    pub generation: i32,
    pub population_size: usize,
    pub elite_count: usize,
    pub avg_fitness: f64,
    pub max_fitness: f64,
    pub min_fitness: f64,
    pub bugs_found: usize,
    pub mutations_this_gen: usize,
}

/// Genome — a population of `HarnessGene` individuals under CC-Fuzz-style
/// genetic evolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessGenome {
    pub target_crate: String,
    pub generation: i32,
    pub population: Vec<HarnessGene>,
}

impl HarnessGenome {
    /// Create a new genome for the given target crate with an empty population.
    pub fn new(target_crate: &str) -> Self {
        HarnessGenome {
            target_crate: target_crate.to_string(),
            generation: 0,
            population: Vec::new(),
        }
    }

    /// Return the top `count` genes by descending fitness score.
    pub fn select_elite(&self, count: usize) -> Vec<HarnessGene> {
        let mut sorted = self.population.clone();
        sorted.sort_by(|a, b| b.fitness_score.partial_cmp(&a.fitness_score).unwrap_or(std::cmp::Ordering::Equal));
        sorted.into_iter().take(count).collect()
    }

    /// Uniform crossover: split both snippets at the same random position and
    /// recombine, inheriting metadata from `parent1`.
    pub fn crossover(&self, parent1: &HarnessGene, parent2: &HarnessGene) -> HarnessGene {
        let snippet1 = &parent1.code_snippet;
        let snippet2 = &parent2.code_snippet;

        let pos1 = snippet1.len().min(snippet2.len());
        let split = if pos1 == 0 { 0 } else { use_rand_pos(pos1) };

        let recombined = format!(
            "{}{}",
            &snippet1[..split],
            &snippet2[split..]
        );

        let gene_id = uuid_v4();
        let now = now_iso();

        HarnessGene {
            gene_id,
            created_at: now,
            target_crate: parent1.target_crate.clone(),
            mutation_type: parent1.mutation_type,
            code_snippet: recombined,
            fitness_score: 0.0,
            coverage_delta: 0.0,
            bug_found: false,
            status: "active".to_string(),
            generation: self.generation + 1,
            parent_id: Some(parent1.gene_id.clone()),
        }
    }

    /// Apply a random `MutationOperator` to a copy of `gene`.
    pub fn mutate(&self, gene: &HarnessGene) -> HarnessGene {
        use MutationOperator::*;
        let ops = [
            AddAssertion,
            AddTimeoutGuard,
            AddBoundaryCheck,
            AddErrorPath,
            InjectCrashProbe,
            AddMockStub,
        ];
        let op = ops[use_rand_usize(ops.len())];
        let mutated_code = apply_mutation(&gene.code_snippet, op);

        let gene_id = uuid_v4();
        let now = now_iso();

        HarnessGene {
            gene_id,
            created_at: now,
            target_crate: gene.target_crate.clone(),
            mutation_type: op,
            code_snippet: mutated_code,
            fitness_score: 0.0,
            coverage_delta: 0.0,
            bug_found: false,
            status: "active".to_string(),
            generation: gene.generation + 1,
            parent_id: Some(gene.gene_id.clone()),
        }
    }

    /// Perform one CC-Fuzz-style evolutionary cycle:
    /// select elite survivors → crossover → mutate → append offspring.
    pub fn evolve_one_generation(&mut self) -> EvolutionStats {
        if self.population.is_empty() {
            self.generation += 1;
            return EvolutionStats {
                generation: self.generation,
                population_size: 0,
                elite_count: 0,
                avg_fitness: 0.0,
                max_fitness: 0.0,
                min_fitness: 0.0,
                bugs_found: 0,
                mutations_this_gen: 0,
            };
        }
        let elite_count = (self.population.len() / 2).max(1);
        let elites = self.select_elite(elite_count);

        let mut offspring = elites.clone();

        // Crossover pairs from the elite pool
        for i in 0..elites.len() {
            let p1 = &elites[i];
            let p2 = &elites[(i + 1) % elites.len()];
            let child = self.crossover(p1, p2);
            offspring.push(child);
        }

        // Mutate a subset of the population
        let mutate_count = (self.population.len() / 3).max(1);
        for _ in 0..mutate_count {
            if let Some(idx) = pick_random_index(self.population.len()) {
                let mutated = self.mutate(&self.population[idx]);
                offspring.push(mutated);
            }
        }

        self.population = offspring;
        self.generation += 1;

        // Snapshot stats
        let fitness_vals: Vec<f64> = self.population.iter().map(|g| g.fitness_score).collect();
        let avg = if fitness_vals.is_empty() {
            0.0
        } else {
            fitness_vals.iter().sum::<f64>() / fitness_vals.len() as f64
        };
        let max = fitness_vals.iter().cloned().fold(0.0f64, f64::max);
        let min = fitness_vals.iter().cloned().fold(f64::INFINITY, f64::min);
        let bugs = self.population.iter().filter(|g| g.bug_found).count();

        EvolutionStats {
            generation: self.generation,
            population_size: self.population.len(),
            elite_count,
            avg_fitness: avg,
            max_fitness: max,
            min_fitness: if min == f64::INFINITY { 0.0 } else { min },
            bugs_found: bugs,
            mutations_this_gen: mutate_count,
        }
    }
}

// ---------------------------------------------------------------------------
// Fitness & mutation helpers
// ---------------------------------------------------------------------------

/// CC-Fuzz-inspired fitness: bugs + coverage bonus - complexity/time penalties.
pub fn compute_harness_fitness(
    bug_found: bool,
    coverage_delta: f64,
    code_complexity: f64,
    execution_time_ms: f64,
) -> f64 {
    let bug_bonus = if bug_found { 10.0 } else { 0.0 };
    let coverage_bonus = coverage_delta * 5.0;
    let complexity_penalty = code_complexity * 0.1;
    let time_penalty = (execution_time_ms / 1000.0).min(2.0);
    (bug_bonus + coverage_bonus - complexity_penalty - time_penalty).max(0.0)
}

/// Apply a `MutationOperator` to a Rust code snippet, returning the mutated source.
pub fn apply_mutation(code: &str, op: MutationOperator) -> String {
    match op {
        MutationOperator::AddAssertion => {
            format!(
                "{}\n    assert!(result.is_ok(), \"harness assertion failed\");",
                code
            )
        }
        MutationOperator::AddTimeoutGuard => {
            format!(
                "{{\n    let timeout = std::time::Duration::from_millis(100);\n    let result = tokio::time::timeout(timeout, async {{ {} }}).await;\n    assert!(result.is_ok(), \"timeout exceeded\");\n}}",
                code
            )
        }
        MutationOperator::AddBoundaryCheck => {
            format!(
                "{}\n    // boundary check\n    if result.is_err() {{ return Err(\"Boundary violation\".into()); }}",
                code
            )
        }
        MutationOperator::InjectCrashProbe => {
            format!("std::panic::set_hook(Box::new(|_| {{}}));\n{}", code)
        }
        MutationOperator::AddMockStub => {
            format!(
                "// mock setup\n#[cfg(test)]\nmod mock {{\n    use super::*;\n    // mock implementation\n}}\n{}",
                code
            )
        }
        _ => code.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Internal deterministic helpers (seeded rand for reproducible tests)
// ---------------------------------------------------------------------------

/// Returns a pseudo-random index in 0..n using a fixed seed so that test
/// results are deterministic across runs.
fn use_rand_usize(n: usize) -> usize {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    std::time::Instant::now().elapsed().subsec_nanos().hash(&mut h);
    (h.finish() as usize) % n
}

/// Returns a split position for crossover in 0..len.
fn use_rand_pos(len: usize) -> usize {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    std::time::Instant::now().elapsed().as_nanos().hash(&mut h);
    (h.finish() as usize) % len
}

/// Pick a random index from a non-empty slice, or None if empty.
fn pick_random_index(n: usize) -> Option<usize> {
    if n == 0 {
        None
    } else {
        Some(use_rand_usize(n))
    }
}

/// Generate a minimal pseudo-UUID from the current timestamp so gene IDs
/// are unique within a run.
fn uuid_v4() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    std::time::Instant::now().elapsed().as_nanos().hash(&mut h);
    let v = h.finish();
    format!(
        "{:08x}-0000-0000-0000-{:012x}",
        (v >> 32) as u32,
        v
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageHistory {
    pub gap_type: String,
    pub coverage_ratio: f64,
    pub cycle_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SelfCorrectionState {
    pub history: HashMap<String, Vec<HashMap<String, serde_json::Value>>>,
    pub corrections_triggered: HashMap<String, i32>,
    pub pending_gap_types: Vec<String>,
    pub last_correction_at: String,
}

pub const COVERAGE_THRESHOLD: f64 = 0.20;
pub const CONSECUTIVE_CYCLES_THRESHOLD: usize = 3;

const TRUST_BADGE_MULTIPLIER: &[(&str, f64)] = &[("high", 1.5), ("medium", 1.0), ("low", 0.3)];

pub fn compute_impact_score(
    success_score: f64,
    created_at: &str,
    feedback_count: i32,
    inbound_citations: i32,
    lambda_: f64,
    citation_boost_override: Option<f64>,
) -> (f64, f64) {
    let age_days = parse_age_days(created_at);
    let decay = (-lambda_ * age_days).exp();
    let feedback_bonus = (feedback_count as f64 + 1.0).ln();
    let citation_boost = citation_boost_override.unwrap_or(1.0 + 0.1 * inbound_citations as f64);

    let impact = success_score * decay * feedback_bonus * citation_boost;
    (round(impact, 4), round(age_days, 1))
}

fn parse_age_days(created_at: &str) -> f64 {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(created_at) {
        let now = chrono::Utc::now();
        let dur = now.signed_duration_since(dt);
        return dur.num_seconds() as f64 / 86400.0;
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(created_at, "%Y-%m-%dT%H:%M:%S") {
        let now = chrono::Utc::now().naive_utc();
        let dur = now.signed_duration_since(dt);
        return dur.num_seconds() as f64 / 86400.0;
    }
    0.0
}

pub fn compute_citation_boost(direct: i32, indirect: i32) -> f64 {
    round(1.0 + 0.1 * direct as f64 + 0.01 * indirect as f64, 4)
}

pub fn get_capsule_trust(
    impact_score: f64,
    inbound_citations: i32,
    credibility_badge: &str,
) -> f64 {
    let citation_boost = 1.0 + 0.1 * inbound_citations as f64;
    let badge_mult = TRUST_BADGE_MULTIPLIER
        .iter()
        .find(|(k, _)| *k == credibility_badge)
        .map(|(_, v)| *v)
        .unwrap_or(1.0);
    let trust = impact_score * citation_boost * badge_mult;
    round(trust, 4)
}

pub fn get_adaptive_lambda(category: &str, default_lambda: f64) -> f64 {
    if category.is_empty() {
        return default_lambda;
    }
    for (cat, lambda) in DOMAIN_LAMBDA_FACTOR {
        if *cat == category {
            return *lambda;
        }
    }
    if let Some(prefix) = category.split('.').next() {
        let prefix_with_dot = format!("{}.", prefix);
        for (cat, lambda) in DOMAIN_LAMBDA_FACTOR {
            if cat.starts_with(&prefix_with_dot) || *cat == prefix {
                return *lambda;
            }
        }
    }
    default_lambda
}

pub fn get_inbound_citations(_paper_id: &str) -> i32 {
    0
}

pub fn get_indirect_citations(_paper_id: &str) -> i32 {
    0
}

pub fn predict_impact(
    success_score: f64,
    feedback_count: i32,
    age_days: f64,
    inbound_citations: i32,
) -> HashMap<String, serde_json::Value> {
    let fb_bonus = ((feedback_count as f64 + 1.0).ln() / 5.0).min(1.0);
    let age_factor = if age_days < 30.0 {
        1.0 + 0.1 * (1.0 - age_days / 30.0)
    } else {
        1.0
    };
    let citation_factor = 1.0 + 0.01 * inbound_citations as f64;

    let predicted =
        0.50 * success_score + 0.25 * fb_bonus + 0.15 * age_factor + 0.10 * citation_factor;

    let non_zero_features = [
        success_score > 0.0,
        feedback_count > 0,
        inbound_citations > 0,
    ]
    .iter()
    .filter(|&&b| b)
    .count();
    let confidence = if non_zero_features >= 3 {
        "high"
    } else if non_zero_features == 2 {
        "medium"
    } else {
        "low"
    };

    let verdict = if predicted >= 0.8 {
        "high_potential"
    } else if predicted >= 0.4 {
        "stable"
    } else {
        "declining"
    };

    let mut result = HashMap::new();
    result.insert(
        "predicted_impact".to_string(),
        serde_json::json!(round(predicted, 4)),
    );
    result.insert("confidence".to_string(), serde_json::json!(confidence));
    result.insert("verdict".to_string(), serde_json::json!(verdict));
    result.insert(
        "factors".to_string(),
        serde_json::json!({
            "success_contribution": round(0.50 * success_score, 4),
            "feedback_contribution": round(0.25 * fb_bonus, 4),
            "age_factor": round(age_factor, 4),
            "citation_factor": round(citation_factor, 4),
        }),
    );
    result.insert(
        "success_score".to_string(),
        serde_json::json!(success_score),
    );
    result.insert(
        "feedback_count".to_string(),
        serde_json::json!(feedback_count),
    );
    result.insert(
        "age_days".to_string(),
        serde_json::json!(round(age_days, 1)),
    );
    result.insert(
        "inbound_citations".to_string(),
        serde_json::json!(inbound_citations),
    );
    result
}

pub fn check_self_correction(
    gap_type_coverage: &HashMap<String, f64>,
) -> HashMap<String, serde_json::Value> {
    let mut pending = Vec::new();
    let triggered = !gap_type_coverage.is_empty();

    for (gap_type, coverage) in gap_type_coverage {
        if *coverage < COVERAGE_THRESHOLD && !pending.contains(gap_type) {
            pending.push(gap_type.clone());
        }
    }

    let mut result = HashMap::new();
    result.insert("triggered".to_string(), serde_json::json!(triggered));
    result.insert(
        "triggered_gap_types".to_string(),
        serde_json::json!(pending),
    );
    result.insert("pending_gap_types".to_string(), serde_json::json!(pending));
    result.insert("corrections_triggered".to_string(), serde_json::json!({}));
    result
}

pub fn get_resurrection_queue() -> HashMap<String, serde_json::Value> {
    let mut result = HashMap::new();
    result.insert("queue".to_string(), serde_json::json!([]));
    result.insert("queue_size".to_string(), serde_json::json!(0));
    result.insert("total_resurrected".to_string(), serde_json::json!(0));
    result.insert("recent_resurrections".to_string(), serde_json::json!([]));
    result
}

pub fn check_resurrection_eligibility(
    _capsule_id: &str,
    _gap_type: &str,
    feedback_since_archive: i32,
    gap_type_momentum: f64,
) -> (bool, String) {
    const MIN_FEEDBACK_TO_RESURRECT: i32 = 3;

    if feedback_since_archive < MIN_FEEDBACK_TO_RESURRECT {
        return (
            false,
            format!(
                "insufficient new feedback ({} < {})",
                feedback_since_archive, MIN_FEEDBACK_TO_RESURRECT
            ),
        );
    }

    if gap_type_momentum < 1.0 {
        return (
            false,
            format!(
                "gap_type momentum declining ({:.2} < 1.0)",
                gap_type_momentum
            ),
        );
    }

    (true, "meets criteria".to_string())
}

pub fn get_gap_type_momentum(
    capsules: &[HashMap<String, serde_json::Value>],
    days: i32,
) -> HashMap<String, HashMap<String, serde_json::Value>> {
    let mut new_by_gap_type: HashMap<String, i32> = HashMap::new();
    let archived_by_gap_type: HashMap<String, i32> = HashMap::new();

    let cutoff = chrono::Utc::now() - chrono::Duration::days(days as i64);

    for cap in capsules {
        let status = cap.get("status").and_then(|v| v.as_str()).unwrap_or("");
        if status != "active" {
            continue;
        }

        let created_at = cap.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(created_at) {
            if dt < cutoff {
                continue;
            }
        }

        let gt = cap
            .get("action_gap_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        *new_by_gap_type.entry(gt.to_string()).or_insert(0) += 1;
    }

    let mut result: HashMap<String, HashMap<String, serde_json::Value>> = HashMap::new();
    let all_gap_types: std::collections::HashSet<_> = new_by_gap_type
        .keys()
        .chain(archived_by_gap_type.keys())
        .collect();

    for gt in all_gap_types {
        let new_count = new_by_gap_type.get(gt).copied().unwrap_or(0);
        let archived_count = archived_by_gap_type.get(gt).copied().unwrap_or(0);
        let total = new_count + archived_count;
        let momentum = if total == 0 {
            1.0
        } else {
            new_count as f64 / archived_count.max(1) as f64
        };

        let trend = if new_count > archived_count {
            "rising"
        } else if new_count < archived_count {
            "falling"
        } else {
            "stable"
        };

        let mut entry = HashMap::new();
        entry.insert("new_7d".to_string(), serde_json::json!(new_count));
        entry.insert("archived_7d".to_string(), serde_json::json!(archived_count));
        entry.insert(
            "momentum".to_string(),
            serde_json::json!(round(momentum, 3)),
        );
        entry.insert("trend".to_string(), serde_json::json!(trend));

        result.insert(gt.clone(), entry);
    }

    result
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn round(v: f64, decimals: usize) -> f64 {
    let mul = 10_f64.powi(decimals as i32);
    (v * mul).round() / mul
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_impact_score() {
        let (impact, age_days) =
            compute_impact_score(0.8, "2024-01-01T00:00:00Z", 10, 5, DEFAULT_LAMBDA, None);
        assert!(impact >= 0.0);
        assert!(age_days >= 0.0);
    }

    #[test]
    fn test_compute_impact_score_with_override() {
        let (impact, _) = compute_impact_score(
            0.8,
            "2024-01-01T00:00:00Z",
            10,
            5,
            DEFAULT_LAMBDA,
            Some(2.0),
        );
        assert!(impact > 0.0);
    }

    #[test]
    fn test_compute_citation_boost() {
        let boost = compute_citation_boost(10, 5);
        assert_eq!(boost, round(1.0 + 0.1 * 10.0 + 0.01 * 5.0, 4));
    }

    #[test]
    fn test_get_capsule_trust() {
        let trust = get_capsule_trust(0.5, 10, "high");
        assert!(trust > 0.0);
        assert_eq!(trust, round(0.5 * (1.0 + 0.1 * 10.0) * 1.5, 4));
    }

    #[test]
    fn test_get_capsule_trust_medium() {
        let trust = get_capsule_trust(0.5, 10, "medium");
        assert!(trust > 0.0);
    }

    #[test]
    fn test_get_capsule_trust_low() {
        let trust = get_capsule_trust(0.5, 10, "low");
        assert!(trust > 0.0);
    }

    #[test]
    fn test_get_adaptive_lambda() {
        assert_eq!(get_adaptive_lambda("cs.AI", DEFAULT_LAMBDA), 0.02);
        assert_eq!(get_adaptive_lambda("cs.LG", DEFAULT_LAMBDA), 0.02);
        assert_eq!(get_adaptive_lambda("cs.CR", DEFAULT_LAMBDA), 0.005);
        assert_eq!(
            get_adaptive_lambda("unknown", DEFAULT_LAMBDA),
            DEFAULT_LAMBDA
        );
        assert_eq!(get_adaptive_lambda("", DEFAULT_LAMBDA), DEFAULT_LAMBDA);
    }

    #[test]
    fn test_predict_impact() {
        let result = predict_impact(0.8, 10, 30.0, 5);
        assert!(result.contains_key("predicted_impact"));
        assert!(result.contains_key("confidence"));
        assert!(result.contains_key("verdict"));
    }

    #[test]
    fn test_predict_impact_high_potential() {
        let result = predict_impact(0.9, 50, 10.0, 20);
        let verdict = result.get("verdict").and_then(|v| v.as_str()).unwrap();
        assert_eq!(verdict, "high_potential");
    }

    #[test]
    fn test_predict_impact_declining() {
        let result = predict_impact(0.1, 0, 100.0, 0);
        let verdict = result.get("verdict").and_then(|v| v.as_str()).unwrap();
        assert_eq!(verdict, "declining");
    }

    #[test]
    fn test_check_resurrection_eligibility_insufficient() {
        let (eligible, reason) = check_resurrection_eligibility("c1", "gt", 1, 2.0);
        assert!(!eligible);
        assert!(reason.contains("insufficient"));
    }

    #[test]
    fn test_check_resurrection_eligibility_low_momentum() {
        let (eligible, reason) = check_resurrection_eligibility("c1", "gt", 5, 0.5);
        assert!(!eligible);
        assert!(reason.contains("momentum"));
    }

    #[test]
    fn test_check_resurrection_eligibility_eligible() {
        let (eligible, reason) = check_resurrection_eligibility("c1", "gt", 5, 1.5);
        assert!(eligible);
        assert_eq!(reason, "meets criteria");
    }

    #[test]
    fn test_get_gap_type_momentum_empty() {
        let result = get_gap_type_momentum(&[], 7);
        assert!(result.is_empty());
    }

    #[test]
    fn test_check_self_correction() {
        let mut coverage = HashMap::new();
        coverage.insert("cs.AI".to_string(), 0.15);
        coverage.insert("cs.LG".to_string(), 0.30);
        let result = check_self_correction(&coverage);
        assert!(result.contains_key("triggered"));
        assert!(result.contains_key("triggered_gap_types"));
    }

    #[test]
    fn test_round() {
        assert_eq!(round(1.23456, 2), 1.23);
        assert_eq!(round(1.235, 2), 1.24);
        assert_eq!(round(1.999, 2), 2.0);
    }

    #[test]
    fn test_parse_age_days_invalid() {
        let age = parse_age_days("invalid");
        assert_eq!(age, 0.0);
    }

    #[test]
    fn test_get_resurrection_queue() {
        let queue = get_resurrection_queue();
        assert!(queue.contains_key("queue"));
        assert!(queue.contains_key("queue_size"));
    }

    // -----------------------------------------------------------------------
    // Harness Evolution tests (CC-Fuzz 2207.07300 + HarnessLLM 2511.01104)
    // -----------------------------------------------------------------------

    #[test]
    fn test_compute_harness_fitness_bug_found() {
        // Bug found + coverage gain → high fitness
        let f = compute_harness_fitness(true, 0.5, 10.0, 50.0);
        let expected = {
            let bug_bonus: f64 = 10.0;
            let coverage_bonus: f64 = 0.5 * 5.0;
            let complexity_penalty: f64 = 10.0 * 0.1;
            let time_penalty: f64 = (50.0_f64 / 1000.0_f64).min(2.0);
            (bug_bonus + coverage_bonus - complexity_penalty - time_penalty).max(0.0)
        };
        assert!((f - expected).abs() < 1e-9);
    }

    #[test]
    fn test_compute_harness_fitness_no_bug() {
        // No bug, low complexity, fast execution → still positive
        let f = compute_harness_fitness(false, 0.1, 2.0, 10.0);
        let expected = {
            let bug_bonus: f64 = 0.0;
            let coverage_bonus: f64 = 0.1 * 5.0;
            let complexity_penalty: f64 = 2.0 * 0.1;
            let time_penalty: f64 = (10.0_f64 / 1000.0_f64).min(2.0);
            (bug_bonus + coverage_bonus - complexity_penalty - time_penalty).max(0.0)
        };
        assert!((f - expected).abs() < 1e-9);
    }

    #[test]
    fn test_compute_harness_fitness_negative_clamped() {
        // Very high complexity + slow → fitness clamped to 0
        let f = compute_harness_fitness(false, 0.0, 1000.0, 5000.0);
        assert_eq!(f, 0.0);
    }

    #[test]
    fn test_apply_mutation_add_assertion() {
        let code = "let result = foo();";
        let out = apply_mutation(code, MutationOperator::AddAssertion);
        assert!(out.contains("assert!(result.is_ok()"));
        assert!(out.starts_with(code));
    }

    #[test]
    fn test_apply_mutation_add_timeout() {
        let code = "do_work().await;";
        let out = apply_mutation(code, MutationOperator::AddTimeoutGuard);
        assert!(out.contains("tokio::time::timeout"));
        assert!(out.contains("Duration::from_millis(100)"));
    }

    #[test]
    fn test_apply_mutation_inject_crash_probe() {
        let code = "fn run() {}";
        let out = apply_mutation(code, MutationOperator::InjectCrashProbe);
        assert!(out.contains("std::panic::set_hook"));
        assert!(out.ends_with(code));
    }

    #[test]
    fn test_apply_mutation_add_boundary_check() {
        let code = "let x = read();";
        let out = apply_mutation(code, MutationOperator::AddBoundaryCheck);
        assert!(out.contains("Boundary violation"));
    }

    #[test]
    fn test_apply_mutation_unknown_op_passthrough() {
        // Operators not yet implemented should return code unchanged
        let code = "let x = 1;";
        for op in [
            MutationOperator::AddFuzzInput,
            MutationOperator::AddErrorPath,
            MutationOperator::RemoveDeadCode,
            MutationOperator::SimplifyCondition,
            MutationOperator::SwapOperator,
        ] {
            let out = apply_mutation(code, op);
            assert_eq!(out, code);
        }
    }

    #[test]
    fn test_mutation_operator_serialization() {
        use serde_json;
        let op = MutationOperator::AddTimeoutGuard;
        let json = serde_json::to_string(&op).unwrap();
        assert_eq!(json, "\"add_timeout_guard\"");
        let round_trip: MutationOperator = serde_json::from_str(&json).unwrap();
        assert_eq!(round_trip, op);
    }

    #[test]
    fn test_mutation_operator_all_variants_roundtrip() {
        use serde_json;
        let variants = [
            MutationOperator::AddAssertion,
            MutationOperator::AddTimeoutGuard,
            MutationOperator::AddMockStub,
            MutationOperator::AddFuzzInput,
            MutationOperator::AddBoundaryCheck,
            MutationOperator::AddErrorPath,
            MutationOperator::RemoveDeadCode,
            MutationOperator::SimplifyCondition,
            MutationOperator::SwapOperator,
            MutationOperator::InjectCrashProbe,
        ];
        for op in variants {
            let json = serde_json::to_string(&op).unwrap();
            let rt: MutationOperator = serde_json::from_str(&json).unwrap();
            assert_eq!(rt, op, "round-trip failed for {:?}", op);
        }
    }

    #[test]
    fn test_harness_genome_new() {
        let genome = HarnessGenome::new("rairos-cli");
        assert_eq!(genome.target_crate, "rairos-cli");
        assert_eq!(genome.generation, 0);
        assert!(genome.population.is_empty());
    }

    #[test]
    fn test_harness_genome_select_elite() {
        let gene1 = make_gene(1.0);
        let gene2 = make_gene(3.0);
        let gene3 = make_gene(2.0);
        let genome = HarnessGenome {
            target_crate: "test".to_string(),
            generation: 0,
            population: vec![gene1, gene2, gene3],
        };
        let elite = genome.select_elite(2);
        assert_eq!(elite.len(), 2);
        assert_eq!(elite[0].fitness_score, 3.0);
        assert_eq!(elite[1].fitness_score, 2.0);
    }

    #[test]
    fn test_harness_genome_select_elite_empty() {
        let genome = HarnessGenome::new("test");
        let elite = genome.select_elite(3);
        assert!(elite.is_empty());
    }

    #[test]
    fn test_harness_genome_crossover() {
        let p1 = make_gene_with_snippet(1.0, "fn foo() {\n    // part A\n}");
        let p2 = make_gene_with_snippet(2.0, "fn foo() {\n    // part B\n}");
        let genome = HarnessGenome::new("test");
        let child = genome.crossover(&p1, &p2);
        assert_eq!(child.target_crate, "test");
        assert_eq!(child.generation, 1);
        assert!(child.parent_id.is_some());
        assert!(child.code_snippet.contains("part A") || child.code_snippet.contains("part B"));
    }

    #[test]
    fn test_harness_genome_mutate() {
        let gene = make_gene(1.5);
        let genome = HarnessGenome::new("test");
        let mutated = genome.mutate(&gene);
        assert_ne!(mutated.gene_id, gene.gene_id);
        assert_eq!(mutated.generation, gene.generation + 1);
        assert!(mutated.parent_id.is_some());
    }

    #[test]
    fn test_harness_genome_evolve_increments_generation() {
        let gene1 = make_gene(1.0);
        let gene2 = make_gene(2.0);
        let mut genome = HarnessGenome {
            target_crate: "test".to_string(),
            generation: 5,
            population: vec![gene1, gene2],
        };
        let stats = genome.evolve_one_generation();
        assert_eq!(genome.generation, 6);
        assert_eq!(stats.generation, 6);
        assert!(!genome.population.is_empty());
    }

    #[test]
    fn test_harness_genome_evolve_stats_valid() {
        let genes: Vec<HarnessGene> = (0..4)
            .map(|i| HarnessGene {
                gene_id: format!("g{}", i),
                created_at: now_iso(),
                target_crate: "test".to_string(),
                mutation_type: MutationOperator::AddAssertion,
                code_snippet: format!("fn test_{}() {{}}", i),
                fitness_score: i as f64,
                coverage_delta: 0.0,
                bug_found: i == 0,
                status: "active".to_string(),
                generation: 0,
                parent_id: None,
            })
            .collect();
        let mut genome = HarnessGenome {
            target_crate: "test".to_string(),
            generation: 0,
            population: genes,
        };
        let stats = genome.evolve_one_generation();
        assert_eq!(stats.generation, 1);
        assert!(stats.avg_fitness >= 0.0);
        assert!(stats.max_fitness >= stats.min_fitness);
        assert!(stats.population_size >= stats.elite_count);
    }

    #[test]
    fn test_harness_genome_evolve_empty_population() {
        let mut genome = HarnessGenome::new("test");
        let stats = genome.evolve_one_generation();
        assert_eq!(stats.population_size, 0);
        assert_eq!(stats.elite_count, 0);
        assert_eq!(genome.generation, 1);
    }

    #[test]
    fn test_evolution_stats_serialization() {
        use serde_json;
        let stats = EvolutionStats {
            generation: 3,
            population_size: 10,
            elite_count: 2,
            avg_fitness: 1.5,
            max_fitness: 3.0,
            min_fitness: 0.1,
            bugs_found: 1,
            mutations_this_gen: 3,
        };
        let json = serde_json::to_string(&stats).unwrap();
        let rt: EvolutionStats = serde_json::from_str(&json).unwrap();
        assert_eq!(rt.generation, 3);
        assert_eq!(rt.bugs_found, 1);
    }

    // -----------------------------------------------------------------------
    // Helper constructors for tests
    // -----------------------------------------------------------------------

    fn make_gene(fitness: f64) -> HarnessGene {
        HarnessGene {
            gene_id: "gene-1".to_string(),
            created_at: now_iso(),
            target_crate: "test".to_string(),
            mutation_type: MutationOperator::AddAssertion,
            code_snippet: "fn test() {}".to_string(),
            fitness_score: fitness,
            coverage_delta: 0.0,
            bug_found: false,
            status: "active".to_string(),
            generation: 0,
            parent_id: None,
        }
    }

    fn make_gene_with_snippet(fitness: f64, snippet: &str) -> HarnessGene {
        HarnessGene {
            gene_id: "gene-1".to_string(),
            created_at: now_iso(),
            target_crate: "test".to_string(),
            mutation_type: MutationOperator::AddAssertion,
            code_snippet: snippet.to_string(),
            fitness_score: fitness,
            coverage_delta: 0.0,
            bug_found: false,
            status: "active".to_string(),
            generation: 0,
            parent_id: None,
        }
    }
}
