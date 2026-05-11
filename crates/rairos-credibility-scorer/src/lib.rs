//! rairos-credibility-scorer — Gene Pool Credibility Scorer
//!
//! Computes per-capsule novelty scores based on keyword overlap (Jaccard).
//! Flags capsules with high keyword redundancy as "trendslop".

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const CAPSULE_PATH: &str = ".ai_research_os/gene_pool/capsules.json";
const TRENDSLOP_THRESHOLD: f64 = 0.7;

fn capsule_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(CAPSULE_PATH)
}

pub fn jaccard(a: &[String], b: &[String]) -> f64 {
    let s_a: std::collections::HashSet<&str> = a.iter().map(|s| s.as_str()).collect();
    let s_b: std::collections::HashSet<&str> = b.iter().map(|s| s.as_str()).collect();
    if s_a.is_empty() || s_b.is_empty() {
        return 0.0;
    }
    let intersection = s_a.intersection(&s_b).count();
    let union = s_a.union(&s_b).count();
    intersection as f64 / union as f64
}

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
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut m = HashMap::new();
        m.insert("capsule_id".to_string(), serde_json::json!(self.capsule_id));
        m.insert("gap_title".to_string(), serde_json::json!(self.gap_title));
        m.insert("gap_type".to_string(), serde_json::json!(self.gap_type));
        m.insert("outcome_score".to_string(), serde_json::json!(self.outcome_score));
        m.insert("novelty_score".to_string(), serde_json::json!(self.novelty_score));
        m.insert("max_overlap".to_string(), serde_json::json!(self.max_overlap));
        m.insert("is_trendslop".to_string(), serde_json::json!(self.is_trendslop));
        m.insert("trigger_keywords".to_string(), serde_json::json!(self.trigger_keywords));
        m
    }
}

pub struct CredibilityScorer {
    credibility: Option<Vec<CapsuleCredibility>>,
}

impl CredibilityScorer {
    pub fn new() -> Self {
        Self { credibility: None }
    }

    pub fn compute_credibility(&mut self, force: bool) -> Vec<CapsuleCredibility> {
        if let (Some(ref c), false) = (&self.credibility, force) {
            return c.clone();
        }

        let path = capsule_path();
        if !path.exists() {
            self.credibility = Some(Vec::new());
            return Vec::new();
        }

        let data: serde_json::Value = match fs::read_to_string(&path) {
            Ok(t) => match serde_json::from_str(&t) {
                Ok(v) => v,
                Err(_) => {
                    self.credibility = Some(Vec::new());
                    return Vec::new();
                }
            },
            Err(_) => {
                self.credibility = Some(Vec::new());
                return Vec::new();
            }
        };

        let capsules = data.get("capsules").and_then(|v| v.as_array()).cloned().unwrap_or_default();

        let mut results: Vec<CapsuleCredibility> = Vec::new();

        for (i, cap) in capsules.iter().enumerate() {
            let kw: Vec<String> = cap
                .get("trigger_keywords")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();

            let cap_id = cap
                .get("capsule_id")
                .and_then(|v| v.as_str())
                .map(String::from)
                .unwrap_or_else(|| format!("cap-{}", i));

            let mut max_overlap = 0.0_f64;
            for (j, other) in capsules.iter().enumerate() {
                if i == j {
                    continue;
                }
                let other_kw: Vec<String> = other
                    .get("trigger_keywords")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                let ov = jaccard(&kw, &other_kw);
                if ov > max_overlap {
                    max_overlap = ov;
                }
            }

            let novelty = 1.0 - max_overlap;
            results.push(CapsuleCredibility {
                capsule_id: cap_id,
                gap_title: cap.get("action_gap_title").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                gap_type: cap.get("action_gap_type").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                outcome_score: cap.get("outcome_success_score").and_then(|v| v.as_f64()).unwrap_or(0.0),
                novelty_score: (novelty * 1000.0).round() / 1000.0,
                max_overlap: (max_overlap * 1000.0).round() / 1000.0,
                is_trendslop: max_overlap > TRENDSLOP_THRESHOLD,
                trigger_keywords: kw,
            });
        }

        results.sort_by(|a, b| b.novelty_score.partial_cmp(&a.novelty_score).unwrap_or(std::cmp::Ordering::Equal));
        self.credibility = Some(results.clone());
        results
    }

    pub fn get_trendslop_capsules(&mut self) -> Vec<CapsuleCredibility> {
        self.compute_credibility(false)
            .into_iter()
            .filter(|c| c.is_trendslop)
            .collect()
    }

    pub fn get_all_credibility(&mut self) -> Vec<CapsuleCredibility> {
        self.compute_credibility(false)
    }

    pub fn render_html(&mut self) -> String {
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
        lines.push("<thead><tr><th>Gap Title</th><th>Type</th><th>Outcome</th><th>Novelty</th><th>Max Overlap</th><th>Status</th></tr></thead>".to_string());
        lines.push("<tbody>".to_string());

        for c in &capsules {
            let novelty_pct = (c.novelty_score * 100.0) as i32;
            let badge = if c.is_trendslop {
                "<span style=\"background:#C4706A;color:white;padding:2px 8px;border-radius:10px;font-size:11px\">&#9888; TRENDSLOP</span>"
            } else {
                "<span style=\"background:#7A9E7A;color:white;padding:2px 8px;border-radius:10px;font-size:11px\">&#10003; Original</span>"
            };
            lines.push("<tr>".to_string());
            lines.push(format!(
                "<td style='max-width:260px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap'><code title='{}'>{}</code></td>",
                c.gap_title,
                if c.gap_title.len() > 40 { &c.gap_title[..40] } else { &c.gap_title }
            ));
            lines.push(format!("<td><code>{}</code></td>", c.gap_type));
            lines.push(format!("<td>{:.2}</td>", c.outcome_score));
            lines.push(format!("<td>{}%</td>", novelty_pct));
            lines.push(format!("<td>{}%</td>", (c.max_overlap * 100.0) as i32));
            lines.push(format!("<td>{}</td>", badge));
            lines.push("</tr>".to_string());
        }

        lines.push("</tbody></table>".to_string());
        lines.push("<style>".to_string());
        lines.push(".credibility-panel { font-family: Georgia, serif; }".to_string());
        lines.push(".credibility-table { width: 100%; border-collapse: collapse; margin-top: 1rem; }".to_string());
        lines.push(".credibility-table th, .credibility-table td { padding: 0.4rem 0.8rem; border-bottom: 1px solid #e8e4de; text-align: left; }".to_string());
        lines.push(".credibility-table th { font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em; color: #7a7570; }".to_string());
        lines.push("</style>".to_string());
        lines.push("</div>".to_string());

        lines.join("\n")
    }
}

impl Default for CredibilityScorer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jaccard_empty() {
        assert_eq!(jaccard(&[], &[]), 0.0);
        assert_eq!(jaccard(&[String::from("a")], &[]), 0.0);
        assert_eq!(jaccard(&[], &[String::from("a")]), 0.0);
    }

    #[test]
    fn test_jaccard_no_overlap() {
        let a = vec![String::from("a"), String::from("b")];
        let b = vec![String::from("c"), String::from("d")];
        assert_eq!(jaccard(&a, &b), 0.0);
    }

    #[test]
    fn test_jaccard_full_overlap() {
        let a = vec![String::from("a"), String::from("b")];
        let b = vec![String::from("a"), String::from("b")];
        assert_eq!(jaccard(&a, &b), 1.0);
    }

    #[test]
    fn test_jaccard_partial() {
        let a = vec![String::from("a"), String::from("b"), String::from("c")];
        let b = vec![String::from("b"), String::from("c"), String::from("d")];
        let result = jaccard(&a, &b);
        assert!((result - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_credibility_scorer_new() {
        let scorer = CredibilityScorer::new();
        assert!(scorer.credibility.is_none());
    }

    #[test]
    fn test_credibility_scorer_empty() {
        let mut scorer = CredibilityScorer::new();
        let result = scorer.compute_credibility(false);
        assert!(result.is_empty());
    }
}
