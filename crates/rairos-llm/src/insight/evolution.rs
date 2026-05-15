//! Feedback-descent closed-loop optimization of insight capsule quality.
//!
//! Inlined from `rairos-insight-evolution`.
//!
//! Ported from `llm/insight/evolution.py`.

use crate::insight::credibility::CapsuleGene;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const HIGH_QUALITY_THRESHOLD: f64 = 0.70;
pub const LOW_QUALITY_THRESHOLD: f64 = 0.30;
pub const RETIRE_COUNT_THRESHOLD: usize = 3;
pub const MAX_CANDIDATES_PER_EVOLVE: usize = 5;
pub const MAX_GENE_POOL_SIZE: usize = 500;
pub const LOW_SCORE_THRESHOLD: f64 = 0.30;
pub const STREAK_THRESHOLD: usize = 3;
pub const OVERLAP_THRESHOLD: f64 = 0.80;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapsuleQuality {
    pub capsule_id: String,
    pub quality_score: f64,
    pub novelty: f64,
    pub utility: f64,
    pub freshness: f64,
    pub overall: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResult {
    pub total_capsules: usize,
    pub avg_quality: f64,
    #[serde(default)]
    pub high_quality: Vec<CapsuleQuality>,
    #[serde(default)]
    pub low_quality: Vec<CapsuleQuality>,
    #[serde(default)]
    pub candidate_ids: Vec<String>,
    #[serde(default)]
    pub retire_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapsuleCandidate {
    pub original_id: String,
    pub candidate_id: String,
    pub trigger_topic: String,
    pub trigger_gap_type: String,
    #[serde(default)]
    pub trigger_keywords: Vec<String>,
    pub action_gap_type: String,
    pub action_gap_title: String,
    pub mutation_description: String,
    pub confidence: f64,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub winner_id: String,
    pub loser_id: String,
    pub reasoning: String,
    pub confidence: f64,
}

pub struct EvolutionEngine {
    capsules: Vec<CapsuleGene>,
}

impl EvolutionEngine {
    pub fn new(capsules: Vec<CapsuleGene>) -> Self {
        Self { capsules }
    }

    pub fn audit(&self, min_capsules: usize) -> AuditResult {
        if self.capsules.len() < min_capsules {
            return AuditResult {
                total_capsules: self.capsules.len(),
                avg_quality: 0.0,
                high_quality: Vec::new(),
                low_quality: Vec::new(),
                candidate_ids: Vec::new(),
                retire_ids: Vec::new(),
            };
        }

        let mut scored = Vec::new();
        for capsule in &self.capsules {
            let q = self.score_capsule(capsule);
            scored.push(q);
        }

        let avg_q = if scored.is_empty() {
            0.0
        } else {
            scored.iter().map(|q| q.overall).sum::<f64>() / scored.len() as f64
        };

        let high_q: Vec<_> = scored
            .iter()
            .filter(|q| q.overall >= HIGH_QUALITY_THRESHOLD)
            .cloned()
            .collect();
        let low_q: Vec<_> = scored
            .iter()
            .filter(|q| q.overall < LOW_QUALITY_THRESHOLD)
            .cloned()
            .collect();
        let candidates: Vec<_> = scored
            .iter()
            .filter(|q| q.overall >= 0.5)
            .map(|q| q.capsule_id.clone())
            .collect();
        let retire: Vec<_> = low_q
            .iter()
            .filter(|q| q.novelty < 0.3 && q.freshness < 0.3)
            .map(|q| q.capsule_id.clone())
            .collect();

        AuditResult {
            total_capsules: self.capsules.len(),
            avg_quality: avg_q,
            high_quality: high_q,
            low_quality: low_q,
            candidate_ids: candidates,
            retire_ids: retire,
        }
    }

    fn score_capsule(&self, capsule: &CapsuleGene) -> CapsuleQuality {
        let novelty = if capsule.feedback_count > 0 {
            (capsule.feedback_count as f64 / 10.0).min(1.0)
        } else {
            0.5
        };

        let utility = capsule.outcome_success_score;

        let freshness = match chrono::DateTime::parse_from_rfc3339(&capsule.created_at) {
            Ok(dt) => {
                let now = chrono::Utc::now();
                let duration = now.signed_duration_since(dt.with_timezone(&chrono::Utc));
                let age_days = duration.num_days() as f64;
                (1.0 - age_days / 365.0).max(0.0)
            }
            Err(_) => 0.5,
        };

        let mut credibility = capsule.credibility_score;
        if capsule.trendslop {
            credibility *= 0.6;
        }

        let overall = 0.35 * utility
            + 0.15 * novelty
            + 0.15 * freshness
            + 0.10 * capsule.outcome_success_score
            + 0.25 * credibility;

        CapsuleQuality {
            capsule_id: capsule.capsule_id.clone(),
            quality_score: overall,
            novelty,
            utility,
            freshness,
            overall,
        }
    }

    pub fn propose(
        &self,
        topic: &str,
        _gap_type: Option<&str>,
        limit: usize,
    ) -> Vec<CapsuleCandidate> {
        let mut candidates = Vec::new();

        for capsule in self.capsules.iter().take(MAX_CANDIDATES_PER_EVOLVE) {
            if let Some(c1) = self.mutate_trigger_broaden(capsule, topic) {
                candidates.push(c1);
            }

            if let Some(c2) = self.mutate_gap_type_transfer(capsule) {
                candidates.push(c2);
            }

            if let Some(c3) = self.mutate_keyword_expand(capsule, topic) {
                candidates.push(c3);
            }
        }

        if candidates.len() < 2 {
            let high_quality: Vec<_> = self
                .capsules
                .iter()
                .filter(|c| c.outcome_success_score >= 0.7)
                .collect();

            let seen_topics: std::collections::HashSet<_> = self
                .capsules
                .iter()
                .take(5)
                .map(|c| c.trigger_topic.clone())
                .collect();

            let cross_topic: Vec<_> = high_quality
                .iter()
                .filter(|c| !seen_topics.contains(&c.trigger_topic))
                .collect();

            for c in cross_topic.into_iter().take(2) {
                let topic_words: Vec<_> = topic
                    .split_whitespace()
                    .filter(|w| w.len() > 3)
                    .take(3)
                    .map(|w| w.to_string())
                    .collect();

                let candidate = CapsuleCandidate {
                    original_id: c.capsule_id.clone(),
                    candidate_id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
                    trigger_topic: topic.to_string(),
                    trigger_gap_type: c.trigger_gap_type.clone(),
                    trigger_keywords: c
                        .trigger_keywords
                        .iter()
                        .chain(topic_words.iter())
                        .cloned()
                        .collect(),
                    action_gap_type: c.action_gap_type.clone(),
                    action_gap_title: c.action_gap_title.clone(),
                    mutation_description: format!(
                        "cross_topic_seed: from '{}' -> '{}'",
                        c.trigger_topic, topic
                    ),
                    confidence: c.outcome_success_score * 0.6,
                    source: "cross_topic_seed".to_string(),
                };
                candidates.push(candidate);
            }
        }

        candidates.truncate(limit);
        candidates
    }

    fn mutate_trigger_broaden(
        &self,
        capsule: &CapsuleGene,
        topic: &str,
    ) -> Option<CapsuleCandidate> {
        if capsule.trigger_topic.is_empty() {
            return None;
        }

        let broader = if topic.contains('-') || topic.contains('/') {
            topic.replace("-", " ").replace("/", " ")
        } else if !capsule
            .trigger_topic
            .to_lowercase()
            .contains(&topic.to_lowercase())
        {
            topic.to_string()
        } else {
            capsule.trigger_topic.clone()
        };

        Some(CapsuleCandidate {
            original_id: capsule.capsule_id.clone(),
            candidate_id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
            trigger_topic: broader.clone(),
            trigger_gap_type: capsule.trigger_gap_type.clone(),
            trigger_keywords: capsule.trigger_keywords.clone(),
            action_gap_type: capsule.action_gap_type.clone(),
            action_gap_title: capsule.action_gap_title.clone(),
            mutation_description: format!(
                "trigger_refine: broadened from '{}' to '{}'",
                capsule.trigger_topic, broader
            ),
            confidence: 0.7,
            source: "trigger_refine".to_string(),
        })
    }

    fn mutate_gap_type_transfer(&self, capsule: &CapsuleGene) -> Option<CapsuleCandidate> {
        let all_types = [
            "method_limitation",
            "contradiction",
            "evaluation_gap",
            "scalability_issue",
            "unexplored_application",
            "theoretical_gap",
            "dataset_gap",
            "generalization_gap",
        ];

        let current = capsule.trigger_gap_type.as_str();
        if let Some(idx) = all_types.iter().position(|&t| t == current) {
            let new_type = all_types[(idx + 1) % all_types.len()];
            Some(CapsuleCandidate {
                original_id: capsule.capsule_id.clone(),
                candidate_id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
                trigger_topic: capsule.trigger_topic.clone(),
                trigger_gap_type: new_type.to_string(),
                trigger_keywords: capsule.trigger_keywords.clone(),
                action_gap_type: capsule.action_gap_type.clone(),
                action_gap_title: capsule.action_gap_title.clone(),
                mutation_description: format!(
                    "gap_type_transfer: {} -> {}",
                    capsule.trigger_gap_type, new_type
                ),
                confidence: 0.5,
                source: "gap_type_transfer".to_string(),
            })
        } else {
            Some(CapsuleCandidate {
                original_id: capsule.capsule_id.clone(),
                candidate_id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
                trigger_topic: capsule.trigger_topic.clone(),
                trigger_gap_type: all_types[0].to_string(),
                trigger_keywords: capsule.trigger_keywords.clone(),
                action_gap_type: capsule.action_gap_type.clone(),
                action_gap_title: capsule.action_gap_title.clone(),
                mutation_description: format!(
                    "gap_type_transfer: {} -> {}",
                    capsule.trigger_gap_type, all_types[0]
                ),
                confidence: 0.5,
                source: "gap_type_transfer".to_string(),
            })
        }
    }

    fn mutate_keyword_expand(
        &self,
        capsule: &CapsuleGene,
        topic: &str,
    ) -> Option<CapsuleCandidate> {
        let topic_words: Vec<String> = topic
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .map(|w| w.to_string())
            .collect();

        let existing: std::collections::HashSet<_> = capsule
            .trigger_keywords
            .iter()
            .map(|k| k.to_lowercase())
            .collect();

        let new_kws: Vec<_> = topic_words
            .iter()
            .filter(|w| !existing.contains(&w.to_lowercase()))
            .cloned()
            .collect();

        if new_kws.is_empty() {
            return None;
        }

        let mut expanded_kws = capsule.trigger_keywords.clone();
        expanded_kws.extend(new_kws.iter().take(3).cloned());

        Some(CapsuleCandidate {
            original_id: capsule.capsule_id.clone(),
            candidate_id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
            trigger_topic: capsule.trigger_topic.clone(),
            trigger_gap_type: capsule.trigger_gap_type.clone(),
            trigger_keywords: expanded_kws,
            action_gap_type: capsule.action_gap_type.clone(),
            action_gap_title: capsule.action_gap_title.clone(),
            mutation_description: format!("keyword_expand: added {:?} to keywords", new_kws),
            confidence: 0.6,
            source: "keyword_expand".to_string(),
        })
    }

    pub fn evaluate(&self, candidates: &[CapsuleCandidate]) -> Vec<EvaluationResult> {
        if candidates.len() < 2 {
            return Vec::new();
        }

        let mut results = Vec::new();

        for i in 0..candidates.len() {
            for j in (i + 1)..candidates.len() {
                let a = &candidates[i];
                let b = &candidates[j];

                let (winner, loser, reasoning, confidence) = if a.confidence >= b.confidence {
                    (
                        a.candidate_id.clone(),
                        b.candidate_id.clone(),
                        format!("fallback: confidence {} vs {}", a.confidence, b.confidence),
                        (a.confidence - b.confidence).abs(),
                    )
                } else {
                    (
                        b.candidate_id.clone(),
                        a.candidate_id.clone(),
                        format!("fallback: confidence {} vs {}", b.confidence, a.confidence),
                        (b.confidence - a.confidence).abs(),
                    )
                };

                results.push(EvaluationResult {
                    winner_id: winner,
                    loser_id: loser,
                    reasoning,
                    confidence,
                });
            }
        }

        results.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        results
    }

    pub fn apply(
        &mut self,
        candidates: &[CapsuleCandidate],
        evaluations: &[EvaluationResult],
        audit: &AuditResult,
    ) -> HashMap<String, serde_json::Value> {
        let mut retired = 0;
        let mut added = 0;

        for cid in &audit.retire_ids {
            if let Some(pos) = self.capsules.iter().position(|c| c.capsule_id == *cid) {
                self.capsules.remove(pos);
                retired += 1;
            }
        }

        if self.capsules.len() > MAX_GENE_POOL_SIZE {
            let excess = self.capsules.len() - MAX_GENE_POOL_SIZE;
            let mut capsule_scores: Vec<_> = self
                .capsules
                .iter()
                .map(|c| {
                    let q = self.score_capsule(c);
                    (c.capsule_id.clone(), q.overall)
                })
                .collect();
            capsule_scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

            let ids_to_remove: Vec<_> = capsule_scores
                .iter()
                .take(excess)
                .map(|(id, _)| id.clone())
                .collect();

            for cid in ids_to_remove {
                if let Some(pos) = self.capsules.iter().position(|c| c.capsule_id == cid) {
                    self.capsules.remove(pos);
                    retired += 1;
                }
            }
        }

        let (merged_count, updated) = self.merge_capsules();
        retired += merged_count;
        self.capsules = updated;

        let (archived_count, updated) = self.auto_archive_low_score();
        retired += archived_count;
        self.capsules = updated;

        if !evaluations.is_empty() {
            let winner_ids: std::collections::HashSet<_> =
                evaluations.iter().map(|e| e.winner_id.clone()).collect();

            let winners: Vec<_> = candidates
                .iter()
                .filter(|c| winner_ids.contains(&c.candidate_id))
                .collect();

            for c in winners.iter().take(3) {
                let (was_added, new_capsules) = self.add_candidate(c);
                if was_added {
                    self.capsules = new_capsules;
                    added += 1;
                }
            }
        }

        let mut result = HashMap::new();
        result.insert("added".to_string(), serde_json::json!(added));
        result.insert("retired".to_string(), serde_json::json!(retired));
        result.insert(
            "total_capsules".to_string(),
            serde_json::json!(self.capsules.len()),
        );
        result.insert(
            "avg_quality".to_string(),
            serde_json::json!(audit.avg_quality),
        );
        result
    }

    fn merge_capsules(&self) -> (usize, Vec<CapsuleGene>) {
        let mut merged_count = 0;
        let mut to_archive = std::collections::HashSet::new();
        let mut winners: std::collections::HashMap<String, CapsuleGene> =
            std::collections::HashMap::new();

        for i in 0..self.capsules.len() {
            let a = &self.capsules[i];
            if to_archive.contains(&a.capsule_id) {
                continue;
            }
            for j in (i + 1)..self.capsules.len() {
                let b = &self.capsules[j];
                if to_archive.contains(&b.capsule_id) {
                    continue;
                }
                if a.trigger_gap_type != b.trigger_gap_type {
                    continue;
                }

                let set_a: std::collections::HashSet<_> = a
                    .trigger_keywords
                    .iter()
                    .map(|k| k.to_lowercase())
                    .collect();
                let set_b: std::collections::HashSet<_> = b
                    .trigger_keywords
                    .iter()
                    .map(|k| k.to_lowercase())
                    .collect();

                if set_a.is_empty() || set_b.is_empty() {
                    continue;
                }

                let intersection = set_a.intersection(&set_b).count();
                let union = set_a.union(&set_b).count();
                let jaccard = intersection as f64 / union as f64;

                if jaccard >= OVERLAP_THRESHOLD {
                    let (loser, winner) = if a.outcome_success_score >= b.outcome_success_score {
                        (b, a)
                    } else {
                        (a, b)
                    };

                    to_archive.insert(loser.capsule_id.clone());

                    let mut winner_kws: Vec<String> = winner.trigger_keywords.clone();
                    for kw in loser.trigger_keywords.iter() {
                        if !winner_kws
                            .iter()
                            .any(|w| w.to_lowercase() == kw.to_lowercase())
                        {
                            winner_kws.push(kw.clone());
                        }
                    }
                    winner_kws.truncate(20);

                    let mut updated_winner = winner.clone();
                    updated_winner.trigger_keywords = winner_kws;
                    updated_winner.feedback_count += loser.feedback_count;

                    winners.insert(winner.capsule_id.clone(), updated_winner);
                    merged_count += 1;
                }
            }
        }

        if to_archive.is_empty() {
            return (0, self.capsules.clone());
        }

        let result: Vec<CapsuleGene> = self
            .capsules
            .iter()
            .filter(|c| !to_archive.contains(&c.capsule_id))
            .map(|c| {
                if let Some(updated) = winners.get(&c.capsule_id) {
                    updated.clone()
                } else {
                    c.clone()
                }
            })
            .collect();

        (merged_count, result)
    }

    fn auto_archive_low_score(&self) -> (usize, Vec<CapsuleGene>) {
        let mut to_archive = std::collections::HashSet::new();
        let mut updated = Vec::new();

        for c in &self.capsules {
            if c.status != "active" {
                updated.push(c.clone());
                continue;
            }

            let mut c = c.clone();
            if c.outcome_success_score < LOW_SCORE_THRESHOLD {
                c.low_score_streak += 1;
            } else {
                c.low_score_streak = 0;
            }

            if c.low_score_streak >= STREAK_THRESHOLD as i32 {
                c.status = "archived".to_string();
                to_archive.insert(c.capsule_id.clone());
            }

            updated.push(c);
        }

        (to_archive.len(), updated)
    }

    fn add_candidate(&self, candidate: &CapsuleCandidate) -> (bool, Vec<CapsuleGene>) {
        for c in &self.capsules {
            if c.trigger_topic == candidate.trigger_topic
                && c.trigger_gap_type == candidate.trigger_gap_type
                && c.action_gap_title == candidate.action_gap_title
            {
                return (false, self.capsules.clone());
            }

            if c.trigger_topic == candidate.trigger_topic
                && !c.trigger_keywords.is_empty()
                && !candidate.trigger_keywords.is_empty()
            {
                let overlap = c
                    .trigger_keywords
                    .iter()
                    .map(|k| k.to_lowercase())
                    .collect::<std::collections::HashSet<_>>()
                    .intersection(
                        &candidate
                            .trigger_keywords
                            .iter()
                            .map(|k| k.to_lowercase())
                            .collect::<std::collections::HashSet<_>>(),
                    )
                    .count();

                let union = c
                    .trigger_keywords
                    .len()
                    .max(candidate.trigger_keywords.len());
                let jaccard = overlap as f64 / union as f64;

                if jaccard > OVERLAP_THRESHOLD {
                    return (false, self.capsules.clone());
                }
            }
        }

        let new_capsule = CapsuleGene {
            capsule_id: candidate.candidate_id.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            trigger_topic: candidate.trigger_topic.clone(),
            trigger_gap_type: candidate.trigger_gap_type.clone(),
            trigger_keywords: candidate.trigger_keywords.clone(),
            action_gap_type: candidate.action_gap_type.clone(),
            action_gap_title: candidate.action_gap_title.clone(),
            outcome_success_score: candidate.confidence * 0.7,
            feedback_count: 0,
            evolved_generation: 1,
            archetype: std::collections::HashMap::new(),
            status: "active".to_string(),
            low_score_streak: 0,
            credibility_score: 0.5,
            trendslop: false,
            trendslop_reason: String::new(),
            credibility_badge: "medium".to_string(),
            source_arxiv_category: String::new(),
        };

        let mut new_capsules = self.capsules.clone();
        new_capsules.push(new_capsule);
        (true, new_capsules)
    }

    pub fn evolve(
        &mut self,
        topic: &str,
        gap_type: Option<&str>,
    ) -> HashMap<String, serde_json::Value> {
        let audit = self.audit(3);

        let candidates = self.propose(topic, gap_type, MAX_CANDIDATES_PER_EVOLVE);

        let evaluations = self.evaluate(&candidates);

        let result = self.apply(&candidates, &evaluations, &audit);

        let mut final_result = HashMap::new();
        final_result.insert(
            "audit".to_string(),
            serde_json::json!({
                "total": audit.total_capsules,
                "avg_quality": (audit.avg_quality * 1000.0).round() / 1000.0,
                "candidates": audit.candidate_ids.len(),
                "to_retire": audit.retire_ids.len(),
            }),
        );
        final_result.insert("proposed".to_string(), serde_json::json!(candidates.len()));
        final_result.insert(
            "evaluations".to_string(),
            serde_json::json!(evaluations.len()),
        );
        final_result.insert("result".to_string(), serde_json::json!(result));
        final_result
    }

    pub fn get_capsules(&self) -> &[CapsuleGene] {
        &self.capsules
    }

    pub fn set_capsules(&mut self, capsules: Vec<CapsuleGene>) {
        self.capsules = capsules;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_capsule(
        capsule_id: &str,
        trigger_topic: &str,
        trigger_gap_type: &str,
        trigger_keywords: Vec<String>,
        outcome_score: f64,
        feedback_count: i32,
    ) -> CapsuleGene {
        CapsuleGene {
            capsule_id: capsule_id.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            trigger_topic: trigger_topic.to_string(),
            trigger_gap_type: trigger_gap_type.to_string(),
            trigger_keywords,
            action_gap_type: trigger_gap_type.to_string(),
            action_gap_title: format!("Test capsule {}", capsule_id),
            outcome_success_score: outcome_score,
            feedback_count,
            evolved_generation: 0,
            archetype: std::collections::HashMap::new(),
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
    fn test_audit_empty() {
        let engine = EvolutionEngine::new(vec![]);
        let result = engine.audit(3);
        assert_eq!(result.total_capsules, 0);
        assert_eq!(result.avg_quality, 0.0);
    }

    #[test]
    fn test_audit_with_capsules() {
        let capsules = vec![
            make_capsule(
                "cap1",
                "NLP",
                "method_limitation",
                vec!["transformer".to_string()],
                0.8,
                5,
            ),
            make_capsule(
                "cap2",
                "NLP",
                "method_limitation",
                vec!["attention".to_string()],
                0.7,
                3,
            ),
            make_capsule(
                "cap3",
                "Vision",
                "unexplored_application",
                vec!["CNN".to_string()],
                0.3,
                0,
            ),
        ];
        let engine = EvolutionEngine::new(capsules);
        let result = engine.audit(1);
        assert_eq!(result.total_capsules, 3);
        assert!(result.avg_quality > 0.0);
        assert!(!result.high_quality.is_empty() || !result.low_quality.is_empty());
    }

    #[test]
    fn test_propose() {
        let capsules = vec![make_capsule(
            "cap1",
            "NLP",
            "method_limitation",
            vec!["transformer".to_string()],
            0.8,
            5,
        )];
        let engine = EvolutionEngine::new(capsules);
        let candidates = engine.propose("NLP", Some("method_limitation"), 5);
        assert!(!candidates.is_empty());
    }

    #[test]
    fn test_evaluate() {
        let candidates = vec![
            CapsuleCandidate {
                original_id: "cap1".to_string(),
                candidate_id: "cand1".to_string(),
                trigger_topic: "NLP".to_string(),
                trigger_gap_type: "method_limitation".to_string(),
                trigger_keywords: vec!["transformer".to_string()],
                action_gap_type: "method_limitation".to_string(),
                action_gap_title: "Test 1".to_string(),
                mutation_description: "Test mutation".to_string(),
                confidence: 0.8,
                source: "test".to_string(),
            },
            CapsuleCandidate {
                original_id: "cap2".to_string(),
                candidate_id: "cand2".to_string(),
                trigger_topic: "NLP".to_string(),
                trigger_gap_type: "method_limitation".to_string(),
                trigger_keywords: vec!["attention".to_string()],
                action_gap_type: "method_limitation".to_string(),
                action_gap_title: "Test 2".to_string(),
                mutation_description: "Test mutation".to_string(),
                confidence: 0.6,
                source: "test".to_string(),
            },
        ];
        let engine = EvolutionEngine::new(vec![]);
        let results = engine.evaluate(&candidates);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].winner_id, "cand1");
        assert_eq!(results[0].loser_id, "cand2");
    }

    #[test]
    fn test_merge_capsules() {
        let capsules = vec![
            make_capsule(
                "cap1",
                "NLP",
                "method_limitation",
                vec!["transformer".to_string(), "attention".to_string()],
                0.8,
                5,
            ),
            make_capsule(
                "cap2",
                "NLP",
                "method_limitation",
                vec!["transformer".to_string(), "attention".to_string()],
                0.7,
                3,
            ),
        ];
        let engine = EvolutionEngine::new(capsules);
        let (merged, _) = engine.merge_capsules();
        assert_eq!(merged, 1);
    }

    #[test]
    fn test_auto_archive_low_score() {
        let mut capsules = vec![
            make_capsule(
                "cap1",
                "NLP",
                "method_limitation",
                vec!["transformer".to_string()],
                0.2,
                0,
            ),
            make_capsule(
                "cap2",
                "NLP",
                "method_limitation",
                vec!["attention".to_string()],
                0.8,
                5,
            ),
        ];
        capsules[0].low_score_streak = 3;

        let engine = EvolutionEngine::new(capsules);
        let (archived, updated) = engine.auto_archive_low_score();
        assert_eq!(archived, 1);
        assert_eq!(updated.len(), 2);
        // Verify exactly 1 is archived and it's cap1
        let archived_capsules: Vec<_> = updated.iter().filter(|c| c.status == "archived").collect();
        assert_eq!(archived_capsules.len(), 1);
        assert_eq!(archived_capsules[0].capsule_id, "cap1");
    }

    #[test]
    fn test_evolve() {
        let capsules = vec![
            make_capsule(
                "cap1",
                "NLP",
                "method_limitation",
                vec!["transformer".to_string()],
                0.8,
                5,
            ),
            make_capsule(
                "cap2",
                "NLP",
                "method_limitation",
                vec!["attention".to_string()],
                0.5,
                2,
            ),
        ];
        let mut engine = EvolutionEngine::new(capsules);
        let result = engine.evolve("NLP", Some("method_limitation"));
        assert!(result.contains_key("audit"));
        assert!(result.contains_key("result"));
    }

    #[test]
    fn test_score_capsule() {
        let capsule = make_capsule(
            "cap1",
            "NLP",
            "method_limitation",
            vec!["transformer".to_string()],
            0.8,
            5,
        );
        let engine = EvolutionEngine::new(vec![capsule.clone()]);
        let quality = engine.score_capsule(&capsule);
        assert!(quality.overall > 0.0);
        assert!(quality.novelty > 0.0);
        assert!(quality.utility > 0.0);
    }
}
