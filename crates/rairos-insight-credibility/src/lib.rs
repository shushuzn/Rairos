//! rairos-insight-credibility — Credibility scoring and trendslop detection for Gene Pool capsules.
//!
//! Ported from `llm/insight/credibility.py`.

pub use rairos_crossover::CapsuleGene;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub const TRENDSLOP_KEYWORD_OVERLAP_THRESHOLD: f64 = 0.70;
pub const CREDIBILITY_HIGH_THRESHOLD: f64 = 0.60;
pub const CREDIBILITY_LOW_THRESHOLD: f64 = 0.30;

pub const EVIDENCE_WEIGHT: f64 = 0.35;
pub const NOVELTY_WEIGHT: f64 = 0.30;
pub const SOURCE_TRUST_WEIGHT: f64 = 0.20;
pub const CONSISTENCY_WEIGHT: f64 = 0.15;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredibilityScore {
    pub capsule_id: String,
    pub overall: f64,
    pub novelty_v2: f64,
    pub evidence_strength: f64,
    pub source_trust: f64,
    pub consistency: f64,
    pub trendslop: bool,
    #[serde(default)]
    pub trendslop_reason: String,
    #[serde(default)]
    pub badge: String,
}

impl CredibilityScore {
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        map.insert("capsule_id".to_string(), serde_json::json!(self.capsule_id));
        map.insert("overall".to_string(), serde_json::json!(round(self.overall, 3)));
        map.insert("novelty_v2".to_string(), serde_json::json!(round(self.novelty_v2, 3)));
        map.insert("evidence_strength".to_string(), serde_json::json!(round(self.evidence_strength, 3)));
        map.insert("source_trust".to_string(), serde_json::json!(round(self.source_trust, 3)));
        map.insert("consistency".to_string(), serde_json::json!(round(self.consistency, 3)));
        map.insert("trendslop".to_string(), serde_json::json!(self.trendslop));
        map.insert("trendslop_reason".to_string(), serde_json::json!(self.trendslop_reason));
        map.insert("badge".to_string(), serde_json::json!(self.badge));
        map
    }
}

fn round(value: f64, decimals: usize) -> f64 {
    let multiplier = 10_f64.powi(decimals as i32);
    (value * multiplier).round() / multiplier
}

pub struct CredibilityScorer {
    source_trust: HashMap<String, f64>,
}

impl CredibilityScorer {
    pub fn new(source_trust: Option<HashMap<String, f64>>) -> Self {
        Self {
            source_trust: source_trust.unwrap_or_default(),
        }
    }

    pub fn compute_novelty_scores(&self, capsules: &[CapsuleGene]) -> HashMap<String, CredibilityScore> {
        let active: Vec<&CapsuleGene> = capsules.iter().filter(|c| c.status == "active").collect();
        if active.is_empty() {
            return HashMap::new();
        }

        let mut overlaps: HashMap<String, (f64, String)> = HashMap::new();
        for c in &active {
            let kws: HashSet<String> = c.trigger_keywords.iter()
                .map(|kw| kw.to_lowercase().trim().to_string())
                .filter(|kw| !kw.is_empty())
                .collect();
            if kws.is_empty() {
                overlaps.insert(c.capsule_id.clone(), (0.0, String::new()));
                continue;
            }

            let mut max_overlap = 0.0_f64;
            let mut worst_match = String::new();
            for other in &active {
                if other.capsule_id == c.capsule_id {
                    continue;
                }
                let other_kws: HashSet<String> = other.trigger_keywords.iter()
                    .map(|kw| kw.to_lowercase().trim().to_string())
                    .filter(|kw| !kw.is_empty())
                    .collect();
                if other_kws.is_empty() {
                    continue;
                }
                let intersection = kws.intersection(&other_kws).count();
                let union = kws.union(&other_kws).count();
                let jaccard = if union > 0 { intersection as f64 / union as f64 } else { 0.0 };
                if jaccard > max_overlap {
                    max_overlap = jaccard;
                    worst_match = other.capsule_id.clone();
                }
            }
            overlaps.insert(c.capsule_id.clone(), (max_overlap, worst_match));
        }

        let mut similar_counts: HashMap<String, usize> = HashMap::new();
        for cid in overlaps.keys() {
            let count = overlaps.iter()
                .filter(|(o_cid, (o_ov, _))| {
                    *o_ov >= TRENDSLOP_KEYWORD_OVERLAP_THRESHOLD && *o_cid != cid
                })
                .count();
            similar_counts.insert(cid.clone(), count);
        }

        let mut results: HashMap<String, CredibilityScore> = HashMap::new();
        for c in &active {
            let overlap_ratio = overlaps.get(&c.capsule_id).map(|(o, _)| *o).unwrap_or(0.0);
            let trendslop = overlap_ratio >= TRENDSLOP_KEYWORD_OVERLAP_THRESHOLD;
            let novelty_v2 = (1.0 - overlap_ratio).max(0.0);

            let evidence = c.outcome_success_score
                * ((c.feedback_count as f64 + 2.0).ln() / 12.0_f64.ln());

            let source = self.source_trust
                .get(&c.archetype.get("source_arxiv_category")
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .unwrap_or_default())
                .copied()
                .unwrap_or(0.5);

            let consistency = if trendslop {
                0.5
            } else if c.outcome_success_score > 0.7 && c.feedback_count > 2 {
                0.9
            } else if c.outcome_success_score < 0.3 {
                0.3
            } else {
                0.7
            };

            let overall = EVIDENCE_WEIGHT * evidence
                + NOVELTY_WEIGHT * novelty_v2
                + SOURCE_TRUST_WEIGHT * source
                + CONSISTENCY_WEIGHT * consistency;

            let badge = if overall >= CREDIBILITY_HIGH_THRESHOLD {
                "high".to_string()
            } else if overall < CREDIBILITY_LOW_THRESHOLD {
                "low".to_string()
            } else {
                "medium".to_string()
            };

            let mut reason_parts: Vec<String> = Vec::new();
            if trendslop {
                let count = similar_counts.get(&c.capsule_id).copied().unwrap_or(0);
                reason_parts.push(format!("{}% keyword overlap with {} other capsule(s)",
                    (overlap_ratio * 100.0) as i32, count));
            }
            if evidence < 0.3 {
                reason_parts.push("low evidence (few feedbacks)".to_string());
            }
            if c.feedback_count == 0 {
                reason_parts.push("unvalidated (no feedback yet)".to_string());
            }

            let score = CredibilityScore {
                capsule_id: c.capsule_id.clone(),
                overall,
                novelty_v2,
                evidence_strength: evidence,
                source_trust: source,
                consistency,
                trendslop,
                trendslop_reason: if reason_parts.is_empty() { String::new() } else { reason_parts.join("; ") },
                badge,
            };
            results.insert(c.capsule_id.clone(), score);
        }

        results
    }

    pub fn is_trendslop(&self, capsule: &CapsuleGene, all_capsules: &[CapsuleGene]) -> (bool, f64, String) {
        let kws: HashSet<String> = capsule.trigger_keywords.iter()
            .map(|kw| kw.to_lowercase().trim().to_string())
            .filter(|kw| !kw.is_empty())
            .collect();
        if kws.is_empty() {
            return (false, 0.0, String::new());
        }

        let mut max_overlap = 0.0_f64;
        let mut similar_count = 0_usize;
        for other in all_capsules {
            if other.capsule_id == capsule.capsule_id || other.status == "archived" {
                continue;
            }
            let other_kws: HashSet<String> = other.trigger_keywords.iter()
                .map(|kw| kw.to_lowercase().trim().to_string())
                .filter(|kw| !kw.is_empty())
                .collect();
            if other_kws.is_empty() {
                continue;
            }
            let intersection = kws.intersection(&other_kws).count();
            let union = kws.union(&other_kws).count();
            let jaccard = if union > 0 { intersection as f64 / union as f64 } else { 0.0 };
            if jaccard >= TRENDSLOP_KEYWORD_OVERLAP_THRESHOLD {
                similar_count += 1;
                if jaccard > max_overlap {
                    max_overlap = jaccard;
                }
            }
        }

        let trendslop = max_overlap >= TRENDSLOP_KEYWORD_OVERLAP_THRESHOLD;
        let reason = if trendslop {
            format!("{}% keyword overlap with {} other capsule(s)", (max_overlap * 100.0) as i32, similar_count)
        } else {
            String::new()
        };
        (trendslop, max_overlap, reason)
    }

    pub fn render_html(&self, capsules: &[CapsuleGene]) -> String {
        if capsules.is_empty() {
            return "<p>No capsules in Gene Pool yet.</p>".to_string();
        }

        let scores = self.compute_novelty_scores(capsules);
        if scores.is_empty() {
            return "<p>No active capsules to assess.</p>".to_string();
        }

        let capsule_map: HashMap<_, _> = capsules.iter()
            .map(|c| (c.capsule_id.clone(), c))
            .collect();
        let score_list: Vec<_> = scores.values().collect();
        let trendslop_count = score_list.iter().filter(|s| s.trendslop).count();

        let mut lines = vec!["<div class=\"credibility-panel\">".to_string()];
        lines.push(format!(
            "<h3>Gap Credibility Scores <small style='color:#888'>({} capsules, {} trendslop)</small></h3>",
            capsules.len(),
            trendslop_count
        ));
        lines.push(
            "<p style='color:#666;font-size:13px;'>Novelty = inverse of keyword overlap. \
             Capsules with >70% Jaccard similarity are flagged as trendslop.</p>".to_string()
        );

        let high_count = score_list.iter().filter(|s| s.badge == "high").count();
        let low_count = score_list.iter().filter(|s| s.badge == "low").count();
        let medium_count = score_list.len() - high_count - low_count;
        lines.push(format!(
            "<div style='display:flex;gap:16px;margin-bottom:16px;'>\
             <span style='background:#7A9E7A;color:white;padding:4px 12px;border-radius:12px;font-size:12px'>High: {}</span>\
             <span style='background:#D4A059;color:white;padding:4px 12px;border-radius:12px;font-size:12px'>Medium: {}</span>\
             <span style='background:#C4706A;color:white;padding:4px 12px;border-radius:12px;font-size:12px'>Low: {}</span>\
             <span style='background:#D9534F;color:white;padding:4px 12px;border-radius:12px;font-size:12px'>Trendslop: {}</span>\
             </div>",
            high_count, medium_count, low_count, trendslop_count
        ));

        lines.push("<table class=\"credibility-table\">".to_string());
        lines.push(
            "<thead><tr>\
             <th>Gap Title</th>\
             <th>Type</th>\
             <th>Outcome</th>\
             <th>Novelty V2</th>\
             <th>Evidence</th>\
             <th>Badge</th>\
             <th>Status</th>\
             </tr></thead>".to_string()
        );
        lines.push("<tbody>".to_string());

        let mut sorted_scores = score_list.clone();
        sorted_scores.sort_by(|a, b| b.overall.partial_cmp(&a.overall).unwrap());
        for s in sorted_scores {
            let c = capsule_map.get(&s.capsule_id);
            let title = c.map(|cap| &cap.action_gap_title[..cap.action_gap_title.len().min(40)]).unwrap_or(&s.capsule_id);
            let gap_type = c.map(|cap| cap.action_gap_type.as_str()).unwrap_or("?");

            let novelty_pct = (s.novelty_v2 * 100.0) as i32;
            let evidence_pct = (s.evidence_strength * 100.0) as i32;

            let color_map = HashMap::from([
                ("high", "#7A9E7A"),
                ("medium", "#D4A059"),
                ("low", "#C4706A"),
            ]);
            let badge_color = color_map.get(s.badge.as_str()).unwrap_or(&"#888");
            let badge_html = format!(
                "<span style=\"background:{};color:white;padding:2px 8px;border-radius:10px;font-size:11px\">{}</span>",
                badge_color,
                s.badge.to_uppercase()
            );
            let status_html = if s.trendslop {
                "<span style=\"background:#D9534F;color:white;padding:2px 8px;border-radius:10px;font-size:11px\">TRENDSLOP</span>"
            } else {
                "<span style=\"background:#7A9E7A;color:white;padding:2px 8px;border-radius:10px;font-size:11px\">Original</span>"
            };

            lines.push("<tr>".to_string());
            lines.push(format!(
                "<td style='max-width:260px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap'><code title='{}'>{}</code></td>",
                title, title
            ));
            lines.push(format!("<td><code>{}</code></td>", gap_type));
            if let Some(cap) = c {
                lines.push(format!("<td>{:.2}</td>", cap.outcome_success_score));
            } else {
                lines.push("<td>?</td>".to_string());
            }
            lines.push(format!("<td>{}%</td>", novelty_pct));
            lines.push(format!("<td>{}%</td>", evidence_pct));
            lines.push(format!("<td>{}</td>", badge_html));
            lines.push(format!("<td>{}</td>", status_html));
            lines.push("</tr>".to_string());
        }

        lines.push("</tbody></table>".to_string());
        lines.push("<style>".to_string());
        lines.push(".credibility-panel { font-family: Georgia, serif; }".to_string());
        lines.push(".credibility-table { width: 100%; border-collapse: collapse; margin-top: 1rem; }".to_string());
        lines.push(".credibility-table th, .credibility-table td { padding: 0.4rem 0.8rem; border-bottom: 1px solid #e8e4de; text-align: left; font-size: 13px; }".to_string());
        lines.push(".credibility-table th { font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em; color: #7a7570; }".to_string());
        lines.push("</style>".to_string());
        lines.push("</div>".to_string());
        lines.join("\n")
    }

    pub fn render_credibility_report(
        &self,
        scores: &HashMap<String, CredibilityScore>,
        capsules: &[CapsuleGene],
        top_n: usize,
    ) -> String {
        let mut lines = vec!["=== Gene Pool Credibility Report ===".to_string(), String::new()];

        let capsule_map: HashMap<_, _> = capsules.iter()
            .map(|c| (c.capsule_id.clone(), c))
            .collect();

        let all_scores: Vec<_> = scores.values().collect();
        if all_scores.is_empty() {
            return "No capsules to assess.".to_string();
        }

        let avg = all_scores.iter().map(|s| s.overall).sum::<f64>() / all_scores.len() as f64;
        let trendslop_count = all_scores.iter().filter(|s| s.trendslop).count();
        let high_count = all_scores.iter().filter(|s| s.badge == "high").count();
        let low_count = all_scores.iter().filter(|s| s.badge == "low").count();

        lines.push(format!("  Total capsules:     {}", all_scores.len()));
        lines.push(format!("  Average credibility: {:.3}", avg));
        lines.push(format!("  High credibility:    {}", high_count));
        lines.push(format!("  Low credibility:     {}", low_count));
        lines.push(format!("  Trendslop flagged:   {}", trendslop_count));
        lines.push(String::new());

        let mut trendslop_list: Vec<_> = all_scores.iter().filter(|s| s.trendslop).collect();
        if !trendslop_list.is_empty() {
            lines.push("── Trendslop Capsules ──".to_string());
            trendslop_list.sort_by(|a, b| a.overall.partial_cmp(&b.overall).unwrap());
            for s in trendslop_list.iter().take(top_n) {
                let c = capsule_map.get(&s.capsule_id);
                let title = c.map(|cap| &cap.action_gap_title[..cap.action_gap_title.len().min(50)]).unwrap_or(&s.capsule_id);
                lines.push(format!(
                    "  [LOW]  {:<50}  novelty={:.2}  evidence={:.2}  {}",
                    title,
                    s.novelty_v2,
                    s.evidence_strength,
                    s.trendslop_reason
                ));
            }
            lines.push(String::new());
        }

        let mut high_list: Vec<_> = all_scores.iter().filter(|s| s.badge == "high").collect();
        if !high_list.is_empty() {
            lines.push("── Top Credible Capsules ──".to_string());
            high_list.sort_by(|a, b| b.overall.partial_cmp(&a.overall).unwrap());
            for s in high_list.iter().take(10) {
                let c = capsule_map.get(&s.capsule_id);
                let title = c.map(|cap| &cap.action_gap_title[..cap.action_gap_title.len().min(50)]).unwrap_or(&s.capsule_id);
                lines.push(format!(
                    "  [HIGH] {:<50}  overall={:.3}  novelty={:.2}  evidence={:.2}",
                    title,
                    s.overall,
                    s.novelty_v2,
                    s.evidence_strength
                ));
            }
            lines.push(String::new());
        }

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_capsule(
        capsule_id: &str,
        trigger_keywords: Vec<String>,
        outcome_score: f64,
        feedback_count: i32,
    ) -> CapsuleGene {
        CapsuleGene {
            capsule_id: capsule_id.to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            trigger_topic: "NLP".to_string(),
            trigger_gap_type: "method_limitation".to_string(),
            trigger_keywords,
            action_gap_type: "method_limitation".to_string(),
            action_gap_title: format!("Test capsule {}", capsule_id),
            outcome_success_score: outcome_score,
            feedback_count,
            evolved_generation: 0,
            archetype: HashMap::new(),
            status: "active".to_string(),
            low_score_streak: 0,
            credibility_score: 0.5,
            trendslop: false,
            trendslop_reason: String::new(),
            credibility_badge: "medium".to_string(),
            source_arxiv_category: "cs.CL".to_string(),
        }
    }

    #[test]
    fn test_credibility_score_to_dict() {
        let score = CredibilityScore {
            capsule_id: "cap1".to_string(),
            overall: 0.75,
            novelty_v2: 0.8,
            evidence_strength: 0.6,
            source_trust: 0.5,
            consistency: 0.7,
            trendslop: false,
            trendslop_reason: String::new(),
            badge: "high".to_string(),
        };
        let dict = score.to_dict();
        assert_eq!(dict["capsule_id"], serde_json::json!("cap1"));
        assert_eq!(dict["overall"], serde_json::json!(0.75));
        assert_eq!(dict["badge"], serde_json::json!("high"));
    }

    #[test]
    fn test_compute_novelty_scores_empty() {
        let scorer = CredibilityScorer::new(None);
        let capsules: Vec<CapsuleGene> = vec![];
        let scores = scorer.compute_novelty_scores(&capsules);
        assert!(scores.is_empty());
    }

    #[test]
    fn test_compute_novelty_scores_single_capsule() {
        let scorer = CredibilityScorer::new(None);
        let capsules = vec![make_capsule("cap1", vec!["transformer".to_string()], 0.8, 3)];
        let scores = scorer.compute_novelty_scores(&capsules);
        assert_eq!(scores.len(), 1);
        let score = scores.get("cap1").unwrap();
        assert!(score.overall > 0.0);
        assert!(!score.trendslop);
    }

    #[test]
    fn test_compute_novelty_scores_trendslop_detection() {
        let scorer = CredibilityScorer::new(None);
        let capsules = vec![
            make_capsule("cap1", vec!["transformer".to_string(), "attention".to_string(), "NLP".to_string(), "LLM".to_string()], 0.8, 3),
            make_capsule("cap2", vec!["transformer".to_string(), "attention".to_string(), "NLP".to_string(), "LLM".to_string(), "model".to_string()], 0.7, 2),
        ];
        let scores = scorer.compute_novelty_scores(&capsules);
        let score1 = scores.get("cap1").unwrap();
        assert!(score1.trendslop);
    }

    #[test]
    fn test_is_trendslop_no_keywords() {
        let scorer = CredibilityScorer::new(None);
        let capsule = make_capsule("cap1", vec![], 0.8, 3);
        let capsules = vec![capsule.clone()];
        let (is_ts, overlap, reason) = scorer.is_trendslop(&capsule, &capsules);
        assert!(!is_ts);
        assert_eq!(overlap, 0.0);
        assert!(reason.is_empty());
    }

    #[test]
    fn test_is_trendslop_with_overlap() {
        let scorer = CredibilityScorer::new(None);
        let capsule1 = make_capsule("cap1", vec!["transformer".to_string(), "attention".to_string(), "NLP".to_string()], 0.8, 3);
        let capsule2 = make_capsule("cap2", vec!["transformer".to_string(), "attention".to_string(), "NLP".to_string(), "LLM".to_string()], 0.7, 2);
        let capsules = vec![capsule1.clone(), capsule2.clone()];
        let (is_ts, overlap, reason) = scorer.is_trendslop(&capsule1, &capsules);
        assert!(is_ts);
        assert!(overlap > 0.0);
        assert!(!reason.is_empty());
    }

    #[test]
    fn test_credibility_high_threshold() {
        let scorer = CredibilityScorer::new(None);
        let capsules = vec![
            make_capsule("high1", vec!["novel".to_string()], 0.9, 10),
        ];
        let scores = scorer.compute_novelty_scores(&capsules);
        let score = scores.get("high1").unwrap();
        assert_eq!(score.badge, "high");
    }

    #[test]
    fn test_credibility_low_threshold() {
        let scorer = CredibilityScorer::new(None);
        let capsules = vec![
            make_capsule("low1", vec!["transformer".to_string(), "attention".to_string(), "NLP".to_string(), "LLM".to_string()], 0.2, 0),
            make_capsule("low2", vec!["transformer".to_string(), "attention".to_string(), "NLP".to_string(), "LLM".to_string()], 0.2, 0),
        ];
        let scores = scorer.compute_novelty_scores(&capsules);
        let score = scores.get("low1").unwrap();
        assert!(score.trendslop);
        assert!(score.overall < CREDIBILITY_HIGH_THRESHOLD);
    }

    #[test]
    fn test_render_credibility_report() {
        let scorer = CredibilityScorer::new(None);
        let capsules = vec![
            make_capsule("cap1", vec!["novel".to_string()], 0.8, 5),
            make_capsule("cap2", vec!["common".to_string()], 0.5, 2),
        ];
        let scores = scorer.compute_novelty_scores(&capsules);
        let report = scorer.render_credibility_report(&scores, &capsules, 10);
        assert!(report.contains("Gene Pool Credibility Report"));
        assert!(report.contains("Total capsules"));
    }

    #[test]
    fn test_render_html() {
        let scorer = CredibilityScorer::new(None);
        let capsules = vec![
            make_capsule("cap1", vec!["transformer".to_string()], 0.8, 3),
        ];
        let html = scorer.render_html(&capsules);
        assert!(html.contains("credibility-panel"));
        assert!(html.contains("cap1"));
    }
}
