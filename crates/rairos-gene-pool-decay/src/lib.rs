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
}
