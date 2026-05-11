use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapSubscription {
    pub family: String,
    pub keywords: Vec<String>,
    pub arxiv_category: String,
    pub enabled: bool,
    pub last_checked: String,
}

impl Default for GapSubscription {
    fn default() -> Self {
        Self {
            family: "other".to_string(),
            keywords: vec!["neural network".to_string()],
            arxiv_category: "cs.LG".to_string(),
            enabled: true,
            last_checked: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherState {
    pub gap_subscriptions: Vec<GapSubscription>,
    pub last_diversity_check: String,
    pub underrepresented_families: Vec<String>,
    pub diversity_score: f64,
}

impl Default for WatcherState {
    fn default() -> Self {
        Self {
            gap_subscriptions: Vec::new(),
            last_diversity_check: String::new(),
            underrepresented_families: Vec::new(),
            diversity_score: 0.0,
        }
    }
}

pub const FAMILY_ARXIV_CONFIG: &[(&str, &[&str], &str)] = &[
    ("attention", &["transformer", "self-attention", "multi-head attention", "vision transformer", "ViT"], "cs.CL"),
    ("reinforcement", &["reinforcement learning", "policy gradient", "DQN", "PPO", "A3C", "reward"], "cs.LG"),
    ("language_model", &["language model", "LLM", "GPT", "BERT", "decoder", "autoregressive", " Transformer"], "cs.CL"),
    ("vision", &["CNN", "image classification", "object detection", "segmentation", "ViT", "vision transformer"], "cs.CV"),
    ("optimization", &["optimizer", "Adam", "SGD", "gradient descent", "loss landscape", "training dynamics"], "cs.LG"),
    ("graph", &["graph neural network", "GNN", "message passing", "node classification", "graph convolution"], "cs.SD"),
    ("reasoning", &["chain-of-thought", "reasoning", "logical inference", "planning", "theorem proving"], "cs.AI"),
    ("embodied", &["robotics", "embodied", "navigation", "control", "motor", "reinforcement learning robot"], "cs.RO"),
    ("other", &["neural network", "deep learning", "training", "representation learning"], "cs.LG"),
];

pub fn build_gap_subscription(family: &str) -> GapSubscription {
    for (fam, keywords, category) in FAMILY_ARXIV_CONFIG {
        if *fam == family {
            return GapSubscription {
                family: fam.to_string(),
                keywords: keywords.iter().map(|s| s.to_string()).collect(),
                arxiv_category: category.to_string(),
                enabled: true,
                last_checked: now_iso(),
            };
        }
    }
    GapSubscription::default()
}

pub fn diff_subscriptions(
    existing: &[GapSubscription],
    new: &[GapSubscription],
) -> (Vec<GapSubscription>, Vec<String>) {
    let existing_families: std::collections::HashSet<_> = existing.iter().map(|s| s.family.clone()).collect();
    let new_families: std::collections::HashSet<_> = new.iter().map(|s| s.family.clone()).collect();

    let to_add: Vec<GapSubscription> = new.iter()
        .filter(|s| !existing_families.contains(&s.family))
        .cloned()
        .collect();
    let to_remove: Vec<String> = existing_families.iter()
        .filter(|f| !new_families.contains(*f))
        .cloned()
        .collect();

    (to_add, to_remove)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiversityPressureResult {
    pub triggered: bool,
    pub pressure_level: f64,
    pub overrepresented_families: Vec<String>,
    pub underrepresented_families: Vec<String>,
    pub eviction_candidates: Vec<HashMap<String, serde_json::Value>>,
    pub archived_capsule_ids: Vec<String>,
    pub diversity_score: f64,
    pub saturation: f64,
}

const SATURATION_THRESHOLD: f64 = 0.80;
const DIVERSITY_THRESHOLD: f64 = 40.0;
const EVICTION_RATE: f64 = 0.20;
const MIN_CAPSULES_TO_EVICT: usize = 1;

const FAMILY_KEYWORDS: &[(&str, &[&str])] = &[
    ("attention", &["attention", "transformer", "multi-head", "self-attention", "cross-attention"]),
    ("reinforcement", &["rl", "reinforcement", "policy", "reward", "agent", "DQN", "PPO", "A3C"]),
    ("language_model", &["LM", "language model", "decoder", "autoregressive", "LLM", "GPT", "BERT"]),
    ("vision", &["CNN", "convolution", "resnet", "image", "vision", "ViT", "classification"]),
    ("optimization", &["optimizer", "Adam", "SGD", "gradient", "loss", "training"]),
    ("graph", &["GNN", "graph", "node", "edge", "message passing"]),
    ("reasoning", &["reasoning", "chain-of-thought", "logical", "inference", "planning"]),
    ("embodied", &["embodied", "robotics", "navigation", "control", "motor"]),
];

pub struct DiversityPressureEvaluator {
    capacity: usize,
}

impl DiversityPressureEvaluator {
    pub fn new(capacity: usize) -> Self {
        Self { capacity }
    }

    fn family_of_keywords(&self, keywords: &[String]) -> String {
        let kw_set: std::collections::HashSet<String> = keywords.iter()
            .map(|k| k.to_lowercase())
            .collect();

        for (fam, fam_kws) in FAMILY_KEYWORDS {
            if fam_kws.iter().any(|fk| kw_set.contains(&fk.to_lowercase())) {
                return fam.to_string();
            }
        }
        "other".to_string()
    }

    fn compute_saturation(&self, total: usize) -> f64 {
        (total as f64 / self.capacity as f64).min(1.0)
    }

    pub fn evaluate(
        &self,
        capsules: &[HashMap<String, serde_json::Value>],
        _gap_subscriptions: Option<&[GapSubscription]>,
    ) -> DiversityPressureResult {
        let total = capsules.len();
        let saturation = self.compute_saturation(total);

        let mut family_counts: HashMap<String, usize> = HashMap::new();
        for cap in capsules {
            let keywords: Vec<String> = cap.get("trigger_keywords")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            let fam = self.family_of_keywords(&keywords);
            *family_counts.entry(fam).or_insert(0) += 1;
        }

        let total_families = family_counts.len();
        let avg_per_family = if total_families > 0 {
            total as f64 / total_families as f64
        } else {
            0.0
        };

        let mut overrep: Vec<String> = Vec::new();
        let mut underrep: Vec<String> = Vec::new();

        for (fam, &count) in &family_counts {
            if count as f64 > avg_per_family * 2.0 {
                overrep.push(fam.clone());
            }
            if (count as f64) < avg_per_family * 0.5 && count < 5 {
                underrep.push(fam.clone());
            }
        }

        let diversity_score = if total_families > 0 {
            (family_counts.len() as f64 * 10.0).min(100.0)
        } else {
            0.0
        };

        let sat_pressure = ((saturation - SATURATION_THRESHOLD) / (1.0 - SATURATION_THRESHOLD)).max(0.0);
        let div_pressure = ((DIVERSITY_THRESHOLD - diversity_score) / DIVERSITY_THRESHOLD).max(0.0);
        let pressure_level = sat_pressure * 0.5 + div_pressure * 0.5;

        let triggered = saturation >= SATURATION_THRESHOLD && diversity_score < DIVERSITY_THRESHOLD;

        if !triggered {
            return DiversityPressureResult {
                triggered: false,
                pressure_level: round(pressure_level, 3),
                overrepresented_families: overrep,
                underrepresented_families: underrep,
                eviction_candidates: vec![],
                archived_capsule_ids: vec![],
                diversity_score,
                saturation,
            };
        }

        if underrep.is_empty() || overrep.is_empty() {
            return DiversityPressureResult {
                triggered: false,
                pressure_level: round(pressure_level, 3),
                overrepresented_families: overrep,
                underrepresented_families: underrep,
                eviction_candidates: vec![],
                archived_capsule_ids: vec![],
                diversity_score,
                saturation,
            };
        }

        let mut eviction_candidates: Vec<HashMap<String, serde_json::Value>> = vec![];

        for fam in &overrep {
            let fam_caps: Vec<_> = capsules.iter()
                .filter(|c| {
                    let keywords: Vec<String> = c.get("trigger_keywords")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    self.family_of_keywords(&keywords) == *fam
                })
                .collect();

            if fam_caps.is_empty() {
                continue;
            }

            let mut scored: Vec<_> = fam_caps.iter()
                .map(|c| {
                    let score = c.get("outcome_success_score")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    (c, score)
                })
                .collect();
            scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

            let n_evict = ((MIN_CAPSULES_TO_EVICT as f64).max(scored.len() as f64 * EVICTION_RATE)) as usize;

            for (cap, score) in scored.into_iter().take(n_evict) {
                let mut candidate = HashMap::new();
                candidate.insert("capsule_id".to_string(), cap.get("capsule_id").cloned().unwrap_or(serde_json::Value::String(String::new())));
                candidate.insert("family".to_string(), serde_json::json!(fam));
                candidate.insert("score".to_string(), serde_json::json!(round(score, 4)));
                candidate.insert("reason".to_string(), serde_json::json!(format!("diversity_pressure: {} is over-represented (pressure={:.2})", fam, pressure_level)));
                eviction_candidates.push(candidate);
            }
        }

        DiversityPressureResult {
            triggered: true,
            pressure_level: round(pressure_level, 3),
            overrepresented_families: overrep,
            underrepresented_families: underrep,
            eviction_candidates,
            archived_capsule_ids: vec![],
            diversity_score,
            saturation,
        }
    }
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
    fn test_gap_subscription_default() {
        let sub = GapSubscription::default();
        assert_eq!(sub.family, "other");
        assert!(sub.enabled);
    }

    #[test]
    fn test_build_gap_subscription_attention() {
        let sub = build_gap_subscription("attention");
        assert_eq!(sub.family, "attention");
        assert!(sub.keywords.contains(&"transformer".to_string()));
        assert_eq!(sub.arxiv_category, "cs.CL");
    }

    #[test]
    fn test_diff_subscriptions() {
        let existing = vec![
            GapSubscription {
                family: "attention".to_string(),
                keywords: vec!["transformer".to_string()],
                arxiv_category: "cs.CL".to_string(),
                enabled: true,
                last_checked: "".to_string(),
            },
        ];
        let new = vec![
            GapSubscription {
                family: "attention".to_string(),
                keywords: vec!["transformer".to_string()],
                arxiv_category: "cs.CL".to_string(),
                enabled: true,
                last_checked: "".to_string(),
            },
            GapSubscription {
                family: "vision".to_string(),
                keywords: vec!["CNN".to_string()],
                arxiv_category: "cs.CV".to_string(),
                enabled: true,
                last_checked: "".to_string(),
            },
        ];
        let (to_add, to_remove) = diff_subscriptions(&existing, &new);
        assert_eq!(to_add.len(), 1);
        assert_eq!(to_add[0].family, "vision");
        assert_eq!(to_remove.len(), 0);
    }

    #[test]
    fn test_diff_subscriptions_remove() {
        let existing = vec![
            GapSubscription {
                family: "attention".to_string(),
                keywords: vec!["transformer".to_string()],
                arxiv_category: "cs.CL".to_string(),
                enabled: true,
                last_checked: "".to_string(),
            },
        ];
        let new = vec![];
        let (to_add, to_remove) = diff_subscriptions(&existing, &new);
        assert_eq!(to_add.len(), 0);
        assert_eq!(to_remove, vec!["attention"]);
    }

    #[test]
    fn test_diversity_pressure_evaluator_not_triggered() {
        let evaluator = DiversityPressureEvaluator::new(50);
        let capsules: Vec<HashMap<String, serde_json::Value>> = vec![];
        let result = evaluator.evaluate(&capsules, None);
        assert!(!result.triggered);
    }

    #[test]
    fn test_diversity_pressure_evaluator_empty() {
        let evaluator = DiversityPressureEvaluator::new(50);
        let result = evaluator.evaluate(&[], None);
        assert!(!result.triggered);
        assert_eq!(result.saturation, 0.0);
    }

    #[test]
    fn test_round() {
        assert_eq!(round(1.23456, 2), 1.23);
        assert_eq!(round(1.235, 2), 1.24);
        assert_eq!(round(1.999, 2), 2.0);
    }

    #[test]
    fn test_watcher_state_default() {
        let state = WatcherState::default();
        assert!(state.gap_subscriptions.is_empty());
        assert_eq!(state.diversity_score, 0.0);
    }
}
