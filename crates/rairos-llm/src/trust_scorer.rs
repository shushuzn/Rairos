//! Trust Scorer — per-category trust scores from capsule quality history.
//!
//! Mirrors llm/trust_scorer.py (pure computation subset)

use serde::{Deserialize, Serialize};

const TRUST_THRESHOLD: f64 = 0.5;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryTrust {
    pub category: String,
    pub total_capsules: usize,
    pub trusted_capsules: usize,
    pub avg_score: f64,
    pub trust_ratio: f64,
}

/// Compute per-category trust scores from capsule score data.
///
/// `scores`: slice of `(category, outcome_success_score)` tuples.
/// Returns categories sorted by trust_ratio descending.
pub fn compute_trust(scores: &[(&str, f64)]) -> Vec<CategoryTrust> {
    use std::collections::HashMap;

    let mut by_cat: HashMap<&str, (usize, usize, f64)> = HashMap::new();

    for &(cat, score) in scores {
        let entry = by_cat.entry(cat).or_insert((0, 0, 0.0));
        entry.0 += 1; // total
        entry.2 += score; // sum
        if score >= TRUST_THRESHOLD {
            entry.1 += 1; // trusted
        }
    }

    let mut result: Vec<CategoryTrust> = by_cat
        .into_iter()
        .map(|(cat, (total, trusted, sum))| {
            let avg = sum / total as f64;
            CategoryTrust {
                category: cat.to_string(),
                total_capsules: total,
                trusted_capsules: trusted,
                avg_score: format_3(avg),
                trust_ratio: format_3(trusted as f64 / total as f64),
            }
        })
        .collect();

    result.sort_by(|a, b| b.trust_ratio.partial_cmp(&a.trust_ratio).unwrap_or(std::cmp::Ordering::Equal));
    result
}

fn format_3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_scores() {
        let result = compute_trust(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_single_category() {
        let scores = &[("cs.LG", 0.8), ("cs.LG", 0.6), ("cs.LG", 0.3)];
        let result = compute_trust(scores);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].category, "cs.LG");
        assert_eq!(result[0].total_capsules, 3);
        assert_eq!(result[0].trusted_capsules, 2);
        assert!((result[0].avg_score - 0.567).abs() < 0.001);
        assert!((result[0].trust_ratio - 0.667).abs() < 0.001);
    }

    #[test]
    fn test_multiple_categories_sorted() {
        let scores = &[("cs.AI", 0.9), ("cs.AI", 0.8), ("cs.LG", 0.2)];
        let result = compute_trust(scores);
        assert_eq!(result.len(), 2);
        // cs.AI has higher trust_ratio → should be first
        assert_eq!(result[0].category, "cs.AI");
        assert_eq!(result[1].category, "cs.LG");
    }

    #[test]
    fn test_threshold_boundary() {
        let scores = &[("cs.CL", 0.5), ("cs.CL", 0.49)];
        let result = compute_trust(scores);
        assert_eq!(result[0].trusted_capsules, 1); // 0.5 >= threshold
    }

    #[test]
    fn test_all_trusted() {
        let scores = &[("cs.CV", 0.9), ("cs.CV", 0.8), ("cs.CV", 0.7)];
        let result = compute_trust(scores);
        assert_eq!(result[0].trust_ratio, 1.0);
    }

    #[test]
    fn test_none_trusted() {
        let scores = &[("math.NA", 0.1), ("math.NA", 0.2)];
        let result = compute_trust(scores);
        assert_eq!(result[0].trusted_capsules, 0);
        assert_eq!(result[0].trust_ratio, 0.0);
    }

    #[test]
    fn test_format_3_precision() {
        assert!((format_3(0.6666) - 0.667).abs() < 0.0001);
        assert!((format_3(0.3333) - 0.333).abs() < 0.0001);
    }
}
