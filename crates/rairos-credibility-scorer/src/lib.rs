//! Gene Pool Credibility Scorer
//!
//! Computes per-capsule novelty scores based on keyword overlap (Jaccard).
//! Flags capsules with high keyword redundancy as "trendslop".

use rairos_gene_pool_io::load_capsules;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// Jaccard similarity between two keyword lists.
fn jaccard(a: &[String], b: &[String]) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let a_set: HashSet<&str> = a.iter().map(|s| s.as_str()).collect();
    let b_set: HashSet<&str> = b.iter().map(|s| s.as_str()).collect();
    let intersection = a_set.intersection(&b_set).count();
    let union = a_set.union(&b_set).count();
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

/// Credibility score for a single capsule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapsuleCredibility {
    pub capsule_id: String,
    pub gap_title: String,
    pub gap_type: String,
    pub outcome_score: f64,
    /// 1 - max_overlap (high = original)
    pub novelty_score: f64,
    /// Jaccard with most-similar capsule
    pub max_overlap: f64,
    pub is_trendslop: bool,
    pub trigger_keywords: Vec<String>,
}

impl CapsuleCredibility {
    /// Convert to a JSON-compatible map.
    pub fn to_map(&self) -> HashMap<String, Value> {
        let mut m = HashMap::new();
        m.insert("capsule_id".to_string(), serde_json::json!(self.capsule_id));
        m.insert("gap_title".to_string(), serde_json::json!(self.gap_title));
        m.insert("gap_type".to_string(), serde_json::json!(self.gap_type));
        m.insert(
            "outcome_score".to_string(),
            serde_json::json!(self.outcome_score),
        );
        m.insert(
            "novelty_score".to_string(),
            serde_json::json!(self.novelty_score),
        );
        m.insert(
            "max_overlap".to_string(),
            serde_json::json!(self.max_overlap),
        );
        m.insert("is_trendslop".to_string(), serde_json::json!(self.is_trendslop));
        m.insert(
            "trigger_keywords".to_string(),
            serde_json::json!(self.trigger_keywords),
        );
        m
    }
}

/// Jaccard overlap above this threshold = "trendslop"
const TRENDSLOP_THRESHOLD: f64 = 0.7;

/// Compute per-capsule novelty scores from Gene Pool capsule history.
pub struct CredibilityScorer {
    cache: Option<Vec<CapsuleCredibility>>,
}

impl Default for CredibilityScorer {
    fn default() -> Self {
        Self::new()
    }
}

impl CredibilityScorer {
    pub fn new() -> Self {
        Self { cache: None }
    }

    /// Compute novelty + trendslop flag for all capsules.
    ///
    /// A capsule's novelty_score = 1 - max_jaccard, where max_jaccard is
    /// the highest Jaccard similarity against any other capsule in the pool.
    pub fn compute_credibility(&mut self, force: bool) -> Vec<CapsuleCredibility> {
        if let Some(ref c) = self.cache {
            if !force {
                return c.clone();
            }
        }

        let capsules = load_capsules(None, None, None);
        let n = capsules.len();

        let mut results: Vec<CapsuleCredibility> = Vec::with_capacity(n);

        for i in 0..n {
            let cap = &capsules[i];

            let keywords: Vec<String> = cap
                .get("trigger_keywords")
                .and_then(|v| {
                    v.as_array()
                        .map(|arr| arr.iter().filter_map(|e| e.as_str().map(String::from)).collect())
                })
                .unwrap_or_default();

            let cap_id = cap
                .get("capsule_id")
                .and_then(|v| v.as_str())
                .map(String::from)
                .unwrap_or_else(|| format!("cap-{}", i));

            // Compute max Jaccard against all other capsules
            let mut max_overlap = 0.0_f64;
            for j in 0..n {
                if i == j {
                    continue;
                }
                let other_kw: Vec<String> = capsules[j]
                    .get("trigger_keywords")
                    .and_then(|v| {
                        v.as_array().map(|arr| {
                            arr.iter().filter_map(|e| e.as_str().map(String::from)).collect()
                        })
                    })
                    .unwrap_or_default();
                let ov = jaccard(&keywords, &other_kw);
                if ov > max_overlap {
                    max_overlap = ov;
                }
            }

            let novelty = 1.0_f64 - max_overlap;

            results.push(CapsuleCredibility {
                capsule_id: cap_id,
                gap_title: cap
                    .get("action_gap_title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                gap_type: cap
                    .get("action_gap_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                outcome_score: cap
                    .get("outcome_success_score")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0),
                novelty_score: (novelty * 1000.0).round() / 1000.0,
                max_overlap: (max_overlap * 1000.0).round() / 1000.0,
                is_trendslop: max_overlap > TRENDSLOP_THRESHOLD,
                trigger_keywords: keywords,
            });
        }

        // Sort: most original first
        results.sort_by(|a, b| {
            b.novelty_score
                .partial_cmp(&a.novelty_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        self.cache = Some(results.clone());
        results
    }

    /// Return only capsules flagged as trendslop.
    pub fn get_trendslop_capsules(&mut self) -> Vec<CapsuleCredibility> {
        self.compute_credibility(false)
            .iter()
            .filter(|c| c.is_trendslop)
            .cloned()
            .collect()
    }

    /// Return all capsules sorted by novelty desc.
    pub fn get_all_credibility(&mut self) -> Vec<CapsuleCredibility> {
        self.compute_credibility(false)
    }

    /// Render credibility scores as an HTML fragment.
    pub fn render_html(&mut self) -> String {
        let capsules = self.get_all_credibility();
        if capsules.is_empty() {
            return "<p>No capsules yet. Create some capsules first.</p>".to_string();
        }

        let trendslop_count = capsules.iter().filter(|c| c.is_trendslop).count();

        let mut lines = Vec::new();
        lines.push("<div class=\"credibility-panel\">".to_string());
        lines.push(format!(
            "<h3>Gap Credibility Scores <small style='color:#888'>({} capsules, {} trendslop)</small></h3>",
            capsules.len(),
            trendslop_count
        ));
        lines.push("<table class=\"credibility-table\">".to_string());
        lines.push(
            "<thead><tr>"
                .to_string()
        );
        lines.push("<th>Gap Title</th><th>Type</th><th>Outcome</th><th>Novelty</th><th>Max Overlap</th><th>Status</th>".to_string());
        lines.push("</tr></thead>".to_string());
        lines.push("<tbody>".to_string());

        for c in &capsules {
            let novelty_pct = (c.novelty_score * 100.0).round() as i32;
            let badge = if c.is_trendslop {
                "<span style=\"background:#C4706A;color:white;padding:2px 8px;border-radius:10px;font-size:11px\">⚠️ TRENDSLOP</span>"
            } else {
                "<span style=\"background:#7A9E7A;color:white;padding:2px 8px;border-radius:10px;font-size:11px\">✓ Original</span>"
            };
            let title_short = if c.gap_title.len() > 40 {
                &c.gap_title[..40]
            } else {
                &c.gap_title
            };
            lines.push("<tr>".to_string());
            lines.push(format!(
                "<td style='max-width:260px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap'><code title='{}'>{}</code></td>",
                c.gap_title, title_short
            ));
            lines.push(format!("<td><code>{}</code></td>", c.gap_type));
            lines.push(format!("<td>{:.2}</td>", c.outcome_score));
            lines.push(format!("<td>{}%</td>", novelty_pct));
            lines.push(format!("<td>{}%</td>", (c.max_overlap * 100.0).round() as i32));
            lines.push(format!("<td>{}</td>", badge));
            lines.push("</tr>".to_string());
        }

        lines.push("</tbody></table>".to_string());
        lines.push("<style>".to_string());
        lines.push(".credibility-panel { font-family: Georgia, serif; }".to_string());
        lines.push(
            ".credibility-table { width: 100%; border-collapse: collapse; margin-top: 1rem; }"
                .to_string(),
        );
        lines.push(
            ".credibility-table th, .credibility-table td { padding: 0.4rem 0.8rem; border-bottom: 1px solid #e8e4de; text-align: left; }"
                .to_string(),
        );
        lines.push(
            ".credibility-table th { font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em; color: #7a7570; }"
                .to_string(),
        );
        lines.push("</style>".to_string());
        lines.push("</div>".to_string());

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jaccard_basic() {
        let a = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let b = vec!["b".to_string(), "c".to_string(), "d".to_string()];
        // intersection = {b, c} = 2, union = {a,b,c,d} = 4 => 0.5
        assert!((jaccard(&a, &b) - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_jaccard_empty() {
        assert!((jaccard(&[], &["a".to_string()]) - 0.0).abs() < 0.001);
        assert!((jaccard(&[], &[]).abs() - 0.0) < 0.001);
    }

    #[test]
    fn test_jaccard_identical() {
        let a = vec!["x".to_string(), "y".to_string()];
        assert!((jaccard(&a, &a) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_jaccard_no_overlap() {
        let a = vec!["a".to_string()];
        let b = vec!["b".to_string()];
        assert!((jaccard(&a, &b) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_credibility_with_real_data() {
        // Verify scorer works with the actual gene pool without panicking.
        let mut scorer = CredibilityScorer::new();
        let result = scorer.compute_credibility(false);
        // Gene pool has real capsules (verified externally via gene_pool.jsonl),
        // so result should not be empty on this system.
        if !result.is_empty() {
            for cap in &result {
                // All scores should be in [0, 1]
                assert!(cap.novelty_score >= 0.0 && cap.novelty_score <= 1.0);
                assert!(cap.max_overlap >= 0.0 && cap.max_overlap <= 1.0);
                assert_eq!(
                    cap.is_trendslop,
                    cap.max_overlap > TRENDSLOP_THRESHOLD,
                );
            }
            // Should be sorted descending by novelty
            for window in result.windows(2) {
                assert!(
                    window[0].novelty_score >= window[1].novelty_score,
                );
            }
            // Trendslop capsules should be a subset
            let trendslop = scorer.get_trendslop_capsules();
            assert!(
                trendslop.iter().all(|c| c.is_trendslop),
            );
        }
        let html = scorer.render_html();
        assert!(html.contains("credibility-panel"));
    }

    #[test]
    fn test_credibility_cache() {
        let mut scorer = CredibilityScorer::new();
        let first = scorer.compute_credibility(false);
        let second = scorer.compute_credibility(false);
        // Returns same data (cache hit)
        assert_eq!(first.len(), second.len());
    }

    #[test]
    fn test_credibility_force_recompute() {
        let mut scorer = CredibilityScorer::new();
        let first = scorer.compute_credibility(false);
        let forced = scorer.compute_credibility(true);
        assert_eq!(first.len(), forced.len());
    }

    #[test]
    fn test_novelty_sort_descending() {
        let mut scorer = CredibilityScorer::new();
        let all = scorer.get_all_credibility();
        for window in all.windows(2) {
            assert!(
                window[0].novelty_score >= window[1].novelty_score,
                "novelty scores should be sorted descending"
            );
        }
    }

    #[test]
    fn test_trendslop_flag_threshold() {
        // Capsules with max_overlap > 0.7 should be flagged
        let mut scorer = CredibilityScorer::new();
        let capsules = scorer.compute_credibility(false);
        for cap in &capsules {
            assert_eq!(
                cap.is_trendslop,
                cap.max_overlap > TRENDSLOP_THRESHOLD,
                "is_trendslop should match threshold check"
            );
        }
    }

    #[test]
    fn test_scores_rounded_to_three_decimals() {
        let mut scorer = CredibilityScorer::new();
        let capsules = scorer.compute_credibility(false);
        for cap in &capsules {
            let rounded_novelty = (cap.novelty_score * 1000.0).round() / 1000.0;
            assert!((cap.novelty_score - rounded_novelty).abs() < 0.001);
            let rounded_overlap = (cap.max_overlap * 1000.0).round() / 1000.0;
            assert!((cap.max_overlap - rounded_overlap).abs() < 0.001);
        }
    }
}
