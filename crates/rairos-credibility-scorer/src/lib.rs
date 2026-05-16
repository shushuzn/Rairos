//! rairos-credibility-scorer — Gene Pool Credibility Scorer for AI Research OS.
//!
//! Ported from `llm/credibility_scorer.py` (177 LOC, pure stdlib).
//!
//! Computes per-capsule novelty scores based on keyword overlap (Jaccard).
//! Flags capsules with high keyword redundancy as "trendslop".

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

// ─── Constants ────────────────────────────────────────────────────────────────

const CAPSULE_PATH: &str =
    ".ai_research_os/gene_pool/capsules.json";
const TRENDSLOP_THRESHOLD: f64 = 0.7;

// ─── Data Structures ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapsuleCredibility {
    pub capsule_id: String,
    pub gap_title: String,
    pub gap_type: String,
    pub outcome_score: f64,
    pub novelty_score: f64,
    pub max_overlap: f64,
    pub is_trendslop: bool,
    pub trigger_keywords: Vec<String>,
}

impl CapsuleCredibility {
    pub fn to_dict(&self) -> serde_json::Value {
        serde_json::json!({
            "capsule_id": self.capsule_id,
            "gap_title": self.gap_title,
            "gap_type": self.gap_type,
            "outcome_score": self.outcome_score,
            "novelty_score": self.novelty_score,
            "max_overlap": self.max_overlap,
            "is_trendslop": self.is_trendslop,
            "trigger_keywords": self.trigger_keywords,
        })
    }
}

// ─── Core Logic ──────────────────────────────────────────────────────────────

pub struct CredibilityScorer {
    capsules_path: PathBuf,
}

impl Default for CredibilityScorer {
    fn default() -> Self {
        Self::new()
    }
}

impl CredibilityScorer {
    pub fn new() -> Self {
        let capsules_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(CAPSULE_PATH);
        Self { capsules_path }
    }

    pub fn with_path(path: PathBuf) -> Self {
        Self { capsules_path: path }
    }

    fn jaccard(a: &[String], b: &[String]) -> f64 {
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }
        let set_a: HashSet<_> = a.iter().collect();
        let set_b: HashSet<_> = b.iter().collect();
        let intersection = set_a.intersection(&set_b).count() as f64;
        let union = set_a.union(&set_b).count() as f64;
        if union == 0.0 {
            0.0
        } else {
            intersection / union
        }
    }

    pub fn compute_credibility(&self, _force: bool) -> Vec<CapsuleCredibility> {
        let mut results: Vec<CapsuleCredibility> = Vec::new();

        if !self.capsules_path.exists() {
            return results;
        }

        let Ok(contents) = fs::read_to_string(&self.capsules_path) else {
            return results;
        };
        let Ok(data) = serde_json::from_str::<serde_json::Value>(&contents) else {
            return results;
        };
        let capsules = data.get("capsules").and_then(|v| v.as_array()).cloned();

        let Some(capsules) = capsules else {
            return results;
        };

        let n = capsules.len();

        for i in 0..n {
            let cap = &capsules[i];
            let kw = cap
                .get("trigger_keywords")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            let cap_id = cap
                .get("capsule_id")
                .and_then(|v| v.as_str())
                .unwrap_or(&format!("cap-{}", i))
                .to_string();

            // Compute max Jaccard against all other capsules
            let mut max_overlap = 0.0;
            for j in 0..n {
                if i == j {
                    continue;
                }
                let other_kw = capsules[j]
                    .get("trigger_keywords")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let ov = Self::jaccard(&kw, &other_kw);
                if ov > max_overlap {
                    max_overlap = ov;
                }
            }

            let novelty = 1.0 - max_overlap;
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
                trigger_keywords: kw,
            });
        }

        // Sort: most original first
        results.sort_by(|a, b| {
            b.novelty_score.partial_cmp(&a.novelty_score).unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }

    pub fn get_trendslop_capsules(&self) -> Vec<CapsuleCredibility> {
        self.compute_credibility(false)
            .into_iter()
            .filter(|c| c.is_trendslop)
            .collect()
    }

    pub fn get_all_credibility(&self) -> Vec<CapsuleCredibility> {
        self.compute_credibility(false)
    }

    pub fn render_html(&self) -> String {
        let capsules = self.get_all_credibility();
        if capsules.is_empty() {
            return "<p>No capsules yet. Create some capsules first.</p>".to_string();
        }

        let trendslop_count = capsules.iter().filter(|c| c.is_trendslop).count();

        let mut lines = vec!["<div class=\"credibility-panel\">".to_string()];
        lines.push(format!(
            "<h3>Gap Credibility Scores <small style='color:#888'>({} capsules, {} trendslop)</small></h3>",
            capsules.len(),
            trendslop_count
        ));
        lines.push("<table class=\"credibility-table\">".to_string());
        lines.push(
            "<thead><tr><th>Gap Title</th><th>Type</th><th>Outcome</th>\
             <th>Novelty</th><th>Max Overlap</th><th>Status</th></tr></thead>"
                .to_string(),
        );
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
            lines.push(format!(
                "<tr><td style='max-width:260px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap'><code title='{}'>{}</code></td>\
                 <td><code>{}</code></td><td>{:.2}</td><td>{}%</td><td>{}%</td><td>{}</td></tr>",
                c.gap_title,
                title_short,
                c.gap_type,
                c.outcome_score,
                novelty_pct,
                (c.max_overlap * 100.0).round() as i32,
                badge
            ));
        }

        lines.push("</tbody></table>".to_string());
        lines.push("<style>".to_string());
        lines.push(".credibility-panel { font-family: Georgia, serif; }".to_string());
        lines.push(
            ".credibility-table { width: 100%; border-collapse: collapse; margin-top: 1rem; }".to_string(),
        );
        lines.push(
            ".credibility-table th, .credibility-table td { padding: 0.4rem 0.8rem; \
             border-bottom: 1px solid #e8e4de; text-align: left; }"
                .to_string(),
        );
        lines.push(
            ".credibility-table th { font-size: 0.75rem; text-transform: uppercase; \
             letter-spacing: 0.05em; color: #7a7570; }"
                .to_string(),
        );
        lines.push("</style>".to_string());
        lines.push("</div>".to_string());

        lines.join("\n")
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jaccard_basic() {
        let a = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let b = vec!["b".to_string(), "c".to_string(), "d".to_string()];
        // intersection = {b, c} = 2, union = {a, b, c, d} = 4
        // jaccard = 2/4 = 0.5
        let result = CredibilityScorer::jaccard(&a, &b);
        assert!((result - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_jaccard_empty() {
        assert_eq!(CredibilityScorer::jaccard(&[], &["a".to_string()]), 0.0);
        assert_eq!(CredibilityScorer::jaccard(&[], &[]), 0.0);
    }

    #[test]
    fn test_jaccard_no_overlap() {
        let a = vec!["a".to_string(), "b".to_string()];
        let b = vec!["c".to_string(), "d".to_string()];
        assert_eq!(CredibilityScorer::jaccard(&a, &b), 0.0);
    }

    #[test]
    fn test_jaccard_identical() {
        let a = vec!["a".to_string(), "b".to_string()];
        let b = vec!["a".to_string(), "b".to_string()];
        assert_eq!(CredibilityScorer::jaccard(&a, &b), 1.0);
    }

    #[test]
    fn test_capsule_credibility_to_dict() {
        let cc = CapsuleCredibility {
            capsule_id: "c1".to_string(),
            gap_title: "Test gap".to_string(),
            gap_type: "theoretical".to_string(),
            outcome_score: 0.8,
            novelty_score: 0.7,
            max_overlap: 0.3,
            is_trendslop: false,
            trigger_keywords: vec!["test".to_string()],
        };
        let dict = cc.to_dict();
        assert_eq!(dict["capsule_id"], "c1");
        assert_eq!(dict["novelty_score"], 0.7);
        assert_eq!(dict["is_trendslop"], false);
    }

    #[test]
    fn test_trendslop_detection() {
        let scorer = CredibilityScorer::with_path(PathBuf::from("/nonexistent/path.json"));
        assert!(scorer.get_all_credibility().is_empty());
    }

    #[test]
    fn test_render_html_empty() {
        let scorer = CredibilityScorer::with_path(PathBuf::from("/nonexistent/path.json"));
        let html = scorer.render_html();
        assert!(html.contains("No capsules yet"));
    }

    #[test]
    fn test_credibility_serialization_roundtrip() {
        let cc = CapsuleCredibility {
            capsule_id: "c1".to_string(),
            gap_title: "Test".to_string(),
            gap_type: "eval".to_string(),
            outcome_score: 0.5,
            novelty_score: 0.5,
            max_overlap: 0.5,
            is_trendslop: true,
            trigger_keywords: vec!["ai".to_string()],
        };
        let json = serde_json::to_string(&cc).unwrap();
        let deserialized: CapsuleCredibility = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.capsule_id, "c1");
        assert!(deserialized.is_trendslop);
    }
}
