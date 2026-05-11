//! Rairos Credibility — Gene Pool Credibility Scorer
//!
//! Computes per-capsule novelty scores based on keyword overlap (Jaccard).
//! Flags capsules with high keyword redundancy as "trendslop".

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

const TRENDSLOP_THRESHOLD: f64 = 0.7;

fn jaccard(a: &[String], b: &[String]) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let set_a: HashSet<&str> = a.iter().map(|s| s.as_str()).collect();
    let set_b: HashSet<&str> = b.iter().map(|s| s.as_str()).collect();
    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();
    if union == 0 {
        return 0.0;
    }
    intersection as f64 / union as f64
}

fn capsule_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".ai_research_os")
        .join("gene_pool")
        .join("capsules.json")
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

#[derive(Debug, Clone, Deserialize)]
struct CapsuleFile {
    capsules: Vec<serde_json::Value>,
}

pub struct CredibilityScorer {
    cached: Option<Vec<CapsuleCredibility>>,
}

impl Default for CredibilityScorer {
    fn default() -> Self {
        Self::new()
    }
}

impl CredibilityScorer {
    pub fn new() -> Self {
        Self { cached: None }
    }

    fn load_capsules(&self) -> Vec<serde_json::Value> {
        let path = capsule_path();
        if !path.exists() {
            return Vec::new();
        }
        match std::fs::read_to_string(&path) {
            Ok(contents) => {
                match serde_json::from_str::<CapsuleFile>(&contents) {
                    Ok(file) => file.capsules,
                    Err(_) => Vec::new(),
                }
            }
            Err(_) => Vec::new(),
        }
    }

    pub fn compute_credibility(&mut self, force: bool) -> Vec<CapsuleCredibility> {
        if let (Some(ref c), false) = (&self.cached, force) {
            return c.clone();
        }

        let capsules = self.load_capsules();
        if capsules.is_empty() {
            self.cached = Some(Vec::new());
            return Vec::new();
        }

        let mut results: Vec<CapsuleCredibility> = Vec::new();
        let n = capsules.len();

        for i in 0..n {
            let cap = &capsules[i];
            let kw = cap
                .get("trigger_keywords")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect::<Vec<String>>()
                })
                .unwrap_or_default();

            let cap_id = cap
                .get("capsule_id")
                .and_then(|v| v.as_str())
                .unwrap_or(&format!("cap-{}", i))
                .to_string();

            let mut max_overlap = 0.0_f64;
            for (j, other) in capsules.iter().enumerate() {
                if i == j {
                    continue;
                }
                let other_kw = other
                    .get("trigger_keywords")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect::<Vec<String>>()
                    })
                    .unwrap_or_default();

                let ov = jaccard(&kw, &other_kw);
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

        results.sort_by(|a, b| b.novelty_score.partial_cmp(&a.novelty_score).unwrap_or(std::cmp::Ordering::Equal));
        self.cached = Some(results.clone());
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

    pub fn render_html(&self, capsules: &[CapsuleCredibility]) -> String {
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
            "<thead><tr><th>Gap Title</th><th>Type</th><th>Outcome</th><th>Novelty</th><th>Max Overlap</th><th>Status</th></tr></thead>".to_string()
        );
        lines.push("<tbody>".to_string());

        for c in capsules {
            let novelty_pct = (c.novelty_score * 100.0).round() as i32;
            let badge = if c.is_trendslop {
                "<span style=\"background:#C4706A;color:white;padding:2px 8px;border-radius:10px;font-size:11px\">⚠️ TRENDSLOP</span>"
            } else {
                "<span style=\"background:#7A9E7A;color:white;padding:2px 8px;border-radius:10px;font-size:11px\">✓ Original</span>"
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
            lines.push(format!("<td>{}%</td>", (c.max_overlap * 100.0).round() as i32));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jaccard() {
        let a = vec!["transformer".to_string(), "attention".to_string()];
        let b = vec!["transformer".to_string(), "bert".to_string()];
        let result = jaccard(&a, &b);
        assert!((result - 0.333).abs() < 0.01);
    }

    #[test]
    fn test_jaccard_empty() {
        let a: Vec<String> = vec![];
        let b = vec!["transformer".to_string()];
        assert!((jaccard(&a, &b) - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_trendslop_threshold() {
        assert!(0.75 > TRENDSLOP_THRESHOLD);
        assert!(0.65 < TRENDSLOP_THRESHOLD);
    }

    #[test]
    fn test_credibility_scorer_empty() {
        let mut scorer = CredibilityScorer::new();
        let result = scorer.compute_credibility(false);
        assert!(result.is_empty());
    }
}
