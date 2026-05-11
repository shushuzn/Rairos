//! rairos-text-utils — Shared text processing utilities for research analysis.
//!
//! Ported from `llm/text_utils.py`.

use regex::Regex;
use std::collections::HashSet;

const KEYWORD_STOPWORDS: &[&str] = &[
    "the", "and", "for", "are", "but", "not", "you", "all", "can", "had",
    "her", "was", "one", "our", "out", "has", "have", "been", "with", "they",
    "this", "that", "from", "will", "would", "there", "their", "what", "about",
    "which", "when", "make", "just", "over", "such", "into", "than", "null",
    "none", "also", "how", "may", "does", "method", "approach", "gap", "issue",
    "problem", "limitation", "study", "work", "paper", "research", "based", "using",
];

pub fn extract_keywords(text: &str, min_len: usize) -> Vec<String> {
    let word_regex = Regex::new(r"[A-Za-z0-9]+").unwrap();
    let stopwords: HashSet<&str> = KEYWORD_STOPWORDS.iter().copied().collect();

    word_regex
        .find_iter(&text.to_lowercase())
        .map(|m| m.as_str().to_string())
        .filter(|w| w.len() >= min_len && !stopwords.contains(w.as_str()))
        .collect()
}

pub fn cosine_sim(a: &[f64], b: &[f64]) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|y| y * y).sum::<f64>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}

pub fn jaccard(a: &[String], b: &[String]) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let set_a: HashSet<&str> = a.iter().map(|s| s.as_str()).collect();
    let set_b: HashSet<&str> = b.iter().map(|s| s.as_str()).collect();

    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_keywords_basic() {
        let text = "The transformer architecture with self attention mechanisms";
        let keywords = extract_keywords(text, 3);
        assert!(keywords.contains(&"transformer".to_string()));
        assert!(keywords.contains(&"self".to_string()));
        assert!(keywords.contains(&"attention".to_string()));
        assert!(!keywords.contains(&"the".to_string()));
        assert!(!keywords.contains(&"with".to_string()));
    }

    #[test]
    fn test_extract_keywords_min_len() {
        let text = "AI agents use RL for decision making";
        let keywords_3 = extract_keywords(text, 3);
        let keywords_5 = extract_keywords(text, 5);
        assert!(keywords_3.len() >= keywords_5.len());
    }

    #[test]
    fn test_extract_keywords_empty() {
        let keywords = extract_keywords("", 3);
        assert!(keywords.is_empty());
    }

    #[test]
    fn test_cosine_sim_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_sim(&a, &b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_sim_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!((cosine_sim(&a, &b) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_sim_normalized() {
        let a = vec![0.5_f64.sqrt(), 0.5_f64.sqrt()];
        let b = vec![0.5_f64.sqrt(), 0.5_f64.sqrt()];
        assert!((cosine_sim(&a, &b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_sim_empty() {
        assert!((cosine_sim(&[], &[])).abs() < 1e-6);
        assert!((cosine_sim(&[1.0], &[])).abs() < 1e-6);
    }

    #[test]
    fn test_jaccard_identical() {
        let a = vec!["attention".to_string(), "transformer".to_string()];
        let b = vec!["attention".to_string(), "transformer".to_string()];
        assert!((jaccard(&a, &b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_jaccard_disjoint() {
        let a = vec!["a".to_string(), "b".to_string()];
        let b = vec!["c".to_string(), "d".to_string()];
        assert!((jaccard(&a, &b) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_jaccard_partial() {
        let a = vec!["attention".to_string(), "transformer".to_string(), "NLP".to_string()];
        let b = vec!["attention".to_string(), "transformer".to_string()];
        let result = jaccard(&a, &b);
        assert!(result > 0.0 && result < 1.0);
    }

    #[test]
    fn test_jaccard_empty() {
        assert!((jaccard(&[], &[])).abs() < 1e-6);
        assert!((jaccard(&["a".to_string()], &[])).abs() < 1e-6);
    }
}
