//! rairos-text — Shared text processing utilities for research analysis.
//!
//! Ported from `llm/text_utils.py`.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const KEYWORD_STOPWORDS: &[&str] = &[
    "the", "and", "for", "are", "but", "not", "you", "all", "can", "had", "her", "was", "one",
    "our", "out", "has", "have", "been", "with", "they", "this", "that", "from", "will",
    "would", "there", "their", "what", "about", "which", "when", "make", "just", "over",
    "such", "into", "than", "null", "none", "also", "how", "may", "does", "method",
    "approach", "gap", "issue", "problem", "limitation", "study", "work", "paper",
    "research", "based", "using",
];

fn is_stopword(w: &str) -> bool {
    KEYWORD_STOPWORDS.contains(&w)
}

fn word_chars(text: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b.is_ascii_alphanumeric() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_alphanumeric() {
                i += 1;
            }
            ranges.push((start, i));
        } else {
            i += 1;
        }
    }
    ranges
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordResult {
    pub keywords: Vec<String>,
}

pub fn extract_keywords(text: &str, min_len: usize) -> Vec<String> {
    let text_lower = text.to_lowercase();
    let ranges = word_chars(&text_lower);
    ranges
        .iter()
        .map(|(start, end)| {
            let w = &text_lower[*start..*end];
            w.to_string()
        })
        .filter(|w| w.len() >= min_len && !is_stopword(w))
        .collect()
}

pub fn cosine_sim(a: &[f64], b: &[f64]) -> f64 {
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|y| y * y).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

pub fn jaccard(a: &[&str], b: &[&str]) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let set_a: HashSet<_> = a.iter().collect();
    let set_b: HashSet<_> = b.iter().collect();
    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();
    if union == 0 {
        return 0.0;
    }
    intersection as f64 / union as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_keywords_basic() {
        let text = "The transformer attention mechanism is used in language models";
        let kw = extract_keywords(text, 3);
        assert!(kw.contains(&"transformer".to_string()));
        assert!(kw.contains(&"attention".to_string()));
        assert!(kw.contains(&"language".to_string()));
        assert!(kw.contains(&"models".to_string()));
        assert!(!kw.contains(&"the".to_string()));
    }

    #[test]
    fn test_extract_keywords_min_len() {
        let kw = extract_keywords("ai ml llm", 3);
        assert!(kw.contains(&"llm".to_string()));
        assert!(!kw.contains(&"ai".to_string()));
        assert!(!kw.contains(&"ml".to_string()));
    }

    #[test]
    fn test_cosine_sim_identical() {
        let a = vec![1.0, 0.0, 0.0];
        assert!((cosine_sim(&a, &a) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_cosine_sim_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!((cosine_sim(&a, &b) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_cosine_sim_zero_vector() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 2.0, 3.0];
        assert!((cosine_sim(&a, &b) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_jaccard_identical() {
        let a = vec!["transformer", "attention", "llm"];
        let b = vec!["transformer", "attention", "llm"];
        assert!((jaccard(&a, &b) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_jaccard_disjoint() {
        let a = vec!["transformer"];
        let b = vec!["llm"];
        assert!((jaccard(&a, &b) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_jaccard_partial() {
        let a = vec!["transformer", "attention", "llm"];
        let b = vec!["transformer", "rl", "policy"];
        let result = jaccard(&a, &b);
        assert!((result - 1.0 / 5.0).abs() < 1e-9);
    }

    #[test]
    fn test_jaccard_empty() {
        assert!((jaccard(&[], &["x"]).abs()) < 1e-9);
        assert!((jaccard(&["x"], &[]).abs()) < 1e-9);
    }
}
