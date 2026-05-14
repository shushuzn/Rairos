//! Gene Pool — capsule-based research guidance from historical feedback.
//!
//! Mirrors llm/insight/gene.py + storage.py for Rust-side search and logging.
//! Enhanced: get_capsule_by_title, encode_capsule, preference_profile integration.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

// ─── Stopwords ─────────────────────────────────────────────────────────────────

const STOPWORDS: &[&str] = &[
    "the", "and", "for", "with", "and", "the", "a", "an", "of", "in", "on", "to",
    "is", "are", "was", "were", "be", "been", "being", "have", "has", "had", "do",
    "does", "did", "will", "would", "could", "should", "may", "might", "shall",
    "can", "not", "or", "but", "if", "so", "as", "at", "by", "from", "about",
    "into", "through", "during", "before", "after", "above", "below", "between",
    "more", "less", "very", "just", "also", "well", "too", "only", "however",
    "method", "approach", "gap", "issue", "problem", "limitation", "paper",
    "research", "study", "work", "novel", "propose", "show", "based", "using",
    "proposed", "our", "we", "their", "this", "that", "these", "those",
];

const KEYWORD_MIN_LEN: usize = 3;

// ─── UserPreferenceProfile (from preference_profile.json) ─────────────────────

#[derive(Debug, Clone, Deserialize, Default)]
struct UserPreferenceProfile {
    #[serde(default)]
    accepts: i32,
    #[serde(default)]
    rejects: i32,
    #[serde(default)]
    views: i32,
}

fn load_preference_profile(base: &PathBuf) -> UserPreferenceProfile {
    let path = base.join("preference_profile.json");
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn compute_success_score(profile: &UserPreferenceProfile) -> f64 {
    let total = (profile.accepts + profile.rejects + profile.views).max(1);
    profile.accepts as f64 / total as f64
}

// ─── Gap type normalization ────────────────────────────────────────────────────

fn normalize_gap_type(gap_type: &str) -> String {
    match gap_type.to_lowercase().as_str() {
        "method_limitation" | "contradiction" | "scalability_issue" => "method_limitation".into(),
        "unexplored_application" | "application_gap" | "exploration_gap" => "application_gap".into(),
        "evaluation_gap" | "baseline_gap" => "evaluation_gap".into(),
        "theoretical_gap" => "theoretical_gap".into(),
        "dataset_gap" => "dataset_gap".into(),
        "generalization_gap" => "generalization_gap".into(),
        _ => "improvement".into(),
    }
}

// ─── CapsuleGene (read + write) ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapsuleGene {
    #[serde(default)]
    pub capsule_id: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub trigger_topic: String,
    #[serde(default)]
    pub trigger_gap_type: String,
    #[serde(default)]
    pub trigger_keywords: Vec<String>,
    #[serde(default)]
    pub action_gap_type: String,
    #[serde(default)]
    pub action_gap_title: String,
    #[serde(default)]
    pub outcome_success_score: f64,
    #[serde(default)]
    pub feedback_count: i32,
    #[serde(default)]
    pub evolved_generation: i32,
    #[serde(default)]
    pub archetype: serde_json::Value,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub low_score_streak: i32,
    #[serde(default)]
    pub credibility_score: f64,
    #[serde(default)]
    pub trendslop: bool,
    #[serde(default)]
    pub trendslop_reason: String,
    #[serde(default)]
    pub credibility_badge: String,
    #[serde(default)]
    pub source_arxiv_category: String,
    #[serde(default)]
    pub hypothesis_id: String,
}

// ─── Category grouping ─────────────────────────────────────────────────────────

fn gap_category(gap_type: &str) -> &str {
    match gap_type {
        "improvement" | "method_gap" | "method_limitation" => "method",
        "application_gap" | "exploration_gap" | "capability" => "content",
        _ => gap_type,
    }
}

// ─── Keyword extraction ────────────────────────────────────────────────────────

pub fn extract_keywords(text: &str) -> Vec<String> {
    let stop: HashSet<&str> = STOPWORDS.iter().copied().collect();
    let mut result = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            current.push(ch);
        } else if !current.is_empty() {
            let lower = current.to_lowercase();
            if lower.len() >= KEYWORD_MIN_LEN && !stop.contains(lower.as_str()) {
                result.push(lower);
            }
            current.clear();
        }
    }
    if !current.is_empty() {
        let lower = current.to_lowercase();
        if lower.len() >= KEYWORD_MIN_LEN && !stop.contains(lower.as_str()) {
            result.push(lower);
        }
    }
    result
}

// ─── trigger_match ─────────────────────────────────────────────────────────────

impl CapsuleGene {
    fn trigger_match(&self, topic: &str, gap_type: &str, keywords: &[String]) -> f64 {
        let mut score = 0.0;
        let title_lower = self.action_gap_title.to_lowercase();
        let topic_lower = topic.to_lowercase();

        if topic_lower.contains(&title_lower) && !title_lower.is_empty() {
            score += 0.5;
        } else if title_lower.contains(&topic_lower) && !topic_lower.is_empty() {
            score += 0.3;
        }

        let trig_lower = self.trigger_topic.to_lowercase();
        if topic_lower.contains(&trig_lower) && !trig_lower.is_empty() {
            score += 0.3;
        } else if trig_lower.contains(&topic_lower) && !topic_lower.is_empty() {
            score += 0.2;
        }

        let gap_lower = gap_type.to_lowercase();
        let trig_gap_lower = self.trigger_gap_type.to_lowercase();
        if !gap_lower.is_empty() && !trig_gap_lower.is_empty() {
            if gap_lower == trig_gap_lower {
                score += 0.3;
            } else if gap_category(&gap_lower) == gap_category(&trig_gap_lower) {
                score += 0.1;
            }
        }

        if !keywords.is_empty() && !self.trigger_keywords.is_empty() {
            let kw_set: HashSet<&str> = keywords.iter().map(|s| s.as_str()).collect();
            let trig_set: HashSet<&str> = self.trigger_keywords.iter().map(|s| s.as_str()).collect();
            let overlap = kw_set.intersection(&trig_set).count();
            let denom = keywords.len().max(self.trigger_keywords.len());
            if denom > 0 {
                score += 0.15 * (overlap as f64 / denom as f64);
            }
        }

        let topic_tokens: HashSet<&str> = topic_lower
            .split_whitespace()
            .filter(|w| w.len() >= KEYWORD_MIN_LEN && !STOPWORDS.contains(w))
            .collect();
        let title_tokens: HashSet<&str> = title_lower
            .split_whitespace()
            .filter(|w| w.len() >= KEYWORD_MIN_LEN && !STOPWORDS.contains(w))
            .collect();
        let intersection = topic_tokens.intersection(&title_tokens).count();
        let union = topic_tokens.union(&title_tokens).count();
        if union > 0 {
            score += 0.25 * (intersection as f64 / union as f64);
        }

        score.min(1.0)
    }

    fn matches_title(&self, title: &str, topic: &str) -> bool {
        self.action_gap_title.to_lowercase() == title.to_lowercase()
            && (topic.is_empty() || self.trigger_topic.to_lowercase() == topic.to_lowercase())
    }
}

// ─── GenePool Manager ──────────────────────────────────────────────────────────

pub struct GenePool {
    base_dir: PathBuf,
    jsonl_path: PathBuf,
    events_path: PathBuf,
}

impl Default for GenePool {
    fn default() -> Self {
        let base = dirs_next()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".ai_research_os")
            .join("evolution");
        Self {
            base_dir: base.clone(),
            jsonl_path: base.join("gene_pool.jsonl"),
            events_path: base.join("events.jsonl"),
        }
    }
}

impl GenePool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn base_path(&self) -> &PathBuf {
        &self.base_dir
    }

    pub fn load_capsules(&self) -> Vec<CapsuleGene> {
        let content = match std::fs::read_to_string(&self.jsonl_path) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        content
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    return None;
                }
                serde_json::from_str::<CapsuleGene>(trimmed).ok()
            })
            .collect()
    }

    pub fn find_capsule(
        &self,
        topic: &str,
        gap_type: &str,
        keywords: Option<&[String]>,
        min_score: f64,
    ) -> (Option<String>, f64) {
        let capsules = self.load_capsules();
        let keywords = keywords.unwrap_or(&[]);
        let kw_vec: Vec<String> = if keywords.is_empty() {
            extract_keywords(topic)
        } else {
            keywords.to_vec()
        };

        let mut best_score = 0.0_f64;
        let mut best_hint: Option<String> = None;

        for cap in &capsules {
            if cap.status == "archived" {
                continue;
            }
            let score = cap.trigger_match(topic, gap_type, &kw_vec);
            if score >= min_score && score > best_score {
                best_score = score;
                best_hint = Some(cap.action_gap_title.clone());
            }
        }

        (best_hint, best_score)
    }

    /// Find a capsule by its action_gap_title (case-insensitive, optional topic filter)
    fn get_capsule_by_title(&self, title: &str, topic: &str) -> Option<CapsuleGene> {
        self.load_capsules()
            .into_iter()
            .find(|c| c.status == "active" && c.matches_title(title, topic))
    }

    /// Generate a new capsule ID: uuid hex[:12]
    fn generate_capsule_id(&self) -> String {
        uuid::Uuid::new_v4().to_string().chars().filter(|c| *c != '-').take(12).collect()
    }

    /// Create a new capsule and append it to gene_pool.jsonl.
    /// Mirrors EvolutionTracker.encode_capsule() from llm/insight/storage.py
    pub fn encode_capsule(
        &self,
        topic: &str,
        gap_type: &str,
        gap_title: &str,
        _gap_description: &str,
        success_score: f64,
        hypothesis_id: &str,
    ) -> Result<String, String> {
        let normalized_gap = normalize_gap_type(gap_type);
        let keywords = extract_keywords(gap_title);
        let now = chrono_now();
        let capsule_id = self.generate_capsule_id();
        let norm_clone = normalized_gap.clone();

        let capsule = CapsuleGene {
            capsule_id: capsule_id.clone(),
            created_at: now.clone(),
            trigger_topic: topic.to_string(),
            trigger_gap_type: normalized_gap.clone(),
            trigger_keywords: keywords,
            action_gap_type: normalized_gap,
            action_gap_title: gap_title.to_string(),
            outcome_success_score: success_score,
            feedback_count: 1,
            evolved_generation: 0,
            archetype: serde_json::json!({
                "gap_type": normalize_gap_type(gap_type),
            }),
            status: "active".to_string(),
            low_score_streak: 0,
            credibility_score: 0.5,
            trendslop: false,
            trendslop_reason: String::new(),
            credibility_badge: "medium".to_string(),
            source_arxiv_category: String::new(),
            hypothesis_id: hypothesis_id.to_string(),
        };

        // Ensure directory exists
        if let Some(parent) = self.jsonl_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create dir: {}", e))?;
        }

        // Append to JSONL
        let json_line = serde_json::to_string(&capsule)
            .map_err(|e| format!("Failed to serialize capsule: {}", e))?;

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.jsonl_path)
            .map_err(|e| format!("Failed to open gene_pool.jsonl: {}", e))?;

        use std::io::Write;
        writeln!(file, "{}", json_line)
            .map_err(|e| format!("Failed to write capsule: {}", e))?;

        // Record lifecycle event
        self.record_capsule_lifecycle(&capsule_id, "created", gap_title, &norm_clone, "")?;

        Ok(capsule_id)
    }

    /// Record a capsule lifecycle event to lifecycle_events.jsonl
    fn record_capsule_lifecycle(
        &self,
        capsule_id: &str,
        action: &str,
        gap_title: &str,
        gap_type: &str,
        details: &str,
    ) -> Result<(), String> {
        let evt = serde_json::json!({
            "timestamp": chrono_now(),
            "capsule_id": capsule_id,
            "action": action,
            "gap_title": gap_title,
            "gap_type": gap_type,
            "details": details,
        });

        let lifecycle_path = self.base_dir.join("lifecycle_events.jsonl");
        if let Some(parent) = lifecycle_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&lifecycle_path)
            .map_err(|e| format!("Failed to open lifecycle_events.jsonl: {}", e))?;

        use std::io::Write;
        writeln!(file, "{}", evt).map_err(|e| format!("Failed to write lifecycle event: {}", e))?;
        Ok(())
    }

    /// Update a capsule's outcome_success_score and feedback_count by hypothesis_id.
    /// Loads all capsules, finds the matching one, updates it, and rewrites the entire JSONL.
    /// Returns Ok(()) if found and updated, Err if not found.
    pub fn update_capsule_by_hypothesis_id(
        &self,
        hypothesis_id: &str,
        new_score: f64,
        feedback_delta: i32,
    ) -> Result<(), String> {
        let capsules = self.load_capsules();
        if capsules.is_empty() {
            return Err("Gene Pool is empty".into());
        }

        let target_idx = capsules.iter().position(|c| c.hypothesis_id == hypothesis_id);
        let idx = match target_idx {
            Some(i) => i,
            None => return Err(format!("No capsule found with hypothesis_id '{}'", hypothesis_id)),
        };

        let mut updated = capsules[idx].clone();
        // Exponential moving average: blend old score with new feedback
        updated.outcome_success_score =
            updated.outcome_success_score * 0.7 + new_score * 0.3;
        updated.feedback_count += feedback_delta;

        let mut all_capsules = capsules;
        all_capsules[idx] = updated.clone();
        self.write_all_capsules(&all_capsules)?;

        self.record_capsule_lifecycle(
            &updated.capsule_id,
            "experiment_feedback",
            &updated.action_gap_title,
            &updated.action_gap_type,
            &format!(
                "Score updated to {:.2} (delta: +{}) via hypothesis '{}'",
                updated.outcome_success_score, feedback_delta, hypothesis_id,
            ),
        )?;

        Ok(())
    }

    /// Record a gap accept event — enhanced: creates/updates capsule + logs event
    pub fn record_gap_accept(
        &self,
        topic: &str,
        gap_type: &str,
        gap_title: &str,
        gap_description: &str,
    ) -> Result<(), String> {
        // Step 1: Append to events.jsonl
        let evt = serde_json::json!({
            "timestamp": chrono_now(),
            "topic": topic,
            "action": "accepted",
            "gap_type": gap_type,
            "gap_title": gap_title,
            "gap_description": gap_description,
            "hypothesis_id": "",
            "question_id": "",
            "paper_ids": [],
            "duration_seconds": 0,
            "notes": "",
            "insight_card_id": "",
        });
        if let Some(parent) = self.events_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        {
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.events_path)
                .map_err(|e| format!("Failed to open events.jsonl: {}", e))?;
            use std::io::Write;
            writeln!(file, "{}", evt).map_err(|e| format!("Failed to write event: {}", e))?;
        }

        // Step 2: Check for existing capsule by title
        let existing = self.get_capsule_by_title(gap_title, topic);
        let profile = load_preference_profile(&self.base_dir);
        let new_score = compute_success_score(&profile);

        if let Some(mut cap) = existing {
            // Update existing capsule
            cap.feedback_count += 1;
            cap.outcome_success_score = cap.outcome_success_score * 0.7 + new_score * 0.3;

            // Write updated capsule back: rewrite entire JSONL with this capsule updated
            let capsules: Vec<CapsuleGene> = self
                .load_capsules()
                .into_iter()
                .map(|c| {
                    if c.capsule_id == cap.capsule_id {
                        cap.clone()
                    } else {
                        c
                    }
                })
                .collect();
            self.write_all_capsules(&capsules)?;

            self.record_capsule_lifecycle(
                &cap.capsule_id, "consumed", gap_title, gap_type,
                &format!("Re-accepted (feedback_count={})", cap.feedback_count),
            )?;
        } else {
            // Create new capsule
            self.encode_capsule(topic, gap_type, gap_title, gap_description, new_score, "")?;
        }

        Ok(())
    }

    /// Rewrite entire gene_pool.jsonl with given capsules
    fn write_all_capsules(&self, capsules: &[CapsuleGene]) -> Result<(), String> {
        if let Some(parent) = self.jsonl_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let mut file = std::fs::File::create(&self.jsonl_path)
            .map_err(|e| format!("Failed to write gene_pool.jsonl: {}", e))?;
        use std::io::Write;
        for cap in capsules {
            let line = serde_json::to_string(cap)
                .map_err(|e| format!("Failed to serialize: {}", e))?;
            writeln!(file, "{}", line)
                .map_err(|e| format!("Failed to write line: {}", e))?;
        }
        Ok(())
    }
}

fn chrono_now() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.f").to_string()
}

fn dirs_next() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| std::env::var("USERPROFILE").ok().map(PathBuf::from))
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pool() -> (GenePool, PathBuf) {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("rairos_gp_{}_{}", std::process::id(), unique));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let pool = GenePool {
            base_dir: dir.clone(),
            jsonl_path: dir.join("gene_pool.jsonl"),
            events_path: dir.join("events.jsonl"),
        };
        (pool, dir)
    }

    fn sample_capsule() -> CapsuleGene {
        CapsuleGene {
            capsule_id: "test001".to_string(),
            created_at: "2024-01-01".to_string(),
            trigger_topic: "reinforcement learning".to_string(),
            trigger_gap_type: "method_limitation".to_string(),
            trigger_keywords: vec!["rl".to_string(), "reward".to_string()],
            action_gap_type: "method_limitation".to_string(),
            action_gap_title: "RL sample efficiency".to_string(),
            outcome_success_score: 0.8,
            feedback_count: 5,
            evolved_generation: 0,
            archetype: serde_json::Value::Object(serde_json::Map::new()),
            status: "active".to_string(),
            low_score_streak: 0,
            credibility_score: 0.5,
            trendslop: false,
            trendslop_reason: String::new(),
            credibility_badge: "medium".to_string(),
            source_arxiv_category: String::new(),
            hypothesis_id: String::new(),
        }
    }

    #[test]
    fn test_new_capsule_creation() {
        let (pool, dir) = make_pool();
        let result = pool.encode_capsule("test topic", "method_gap", "test capsule", "desc", 0.7, "");
        assert!(result.is_ok());
        let content = std::fs::read_to_string(pool.jsonl_path).unwrap_or_default();
        assert!(content.contains("test topic"));
        assert!(content.contains("test capsule"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_get_capsule_by_title_found() {
        let (pool, dir) = make_pool();
        pool.encode_capsule("test topic", "method_gap", "My Gap Title", "desc", 0.7, "").unwrap();
        let found = pool.get_capsule_by_title("My Gap Title", "test topic");
        assert!(found.is_some());
        assert_eq!(found.unwrap().action_gap_title, "My Gap Title");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_get_capsule_by_title_not_found() {
        let (pool, dir) = make_pool();
        pool.encode_capsule("test topic", "method_gap", "Existing Title", "desc", 0.7, "").unwrap();
        let found = pool.get_capsule_by_title("Nonexistent", "");
        assert!(found.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_enhanced_record_gap_accept_creates_new() {
        let (pool, dir) = make_pool();
        let r = pool.record_gap_accept("test topic", "method_gap", "New Gap", "desc");
        assert!(r.is_ok());
        let content = std::fs::read_to_string(&pool.jsonl_path).unwrap_or_default();
        assert!(content.contains("New Gap"), "capsule should be created in JSONL");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_enhanced_record_gap_accept_updates_existing() {
        let (pool, dir) = make_pool();
        pool.encode_capsule("topic", "method_gap", "Existing Gap", "desc", 0.5, "").unwrap();
        let initial_count = pool.load_capsules().len();
        assert_eq!(initial_count, 1);

        // Call record_gap_accept with same title
        let r = pool.record_gap_accept("topic", "method_gap", "Existing Gap", "desc");
        assert!(r.is_ok());

        // Should NOT create a new capsule (same count)
        let capsules = pool.load_capsules();
        assert_eq!(capsules.len(), 1, "should still be 1 capsule, not duplicated");
        assert_eq!(capsules[0].feedback_count, 2, "feedback_count should increment");
        assert!(
            capsules[0].outcome_success_score < 0.7,
            "score should be blended (got {})",
            capsules[0].outcome_success_score
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_compute_success_score() {
        let profile = UserPreferenceProfile { accepts: 3, rejects: 1, views: 1 };
        let score = compute_success_score(&profile);
        assert!((score - 0.6).abs() < 0.001, "expected 0.6, got {}", score);
    }

    #[test]
    fn test_normalize_gap_type() {
        assert_eq!(normalize_gap_type("method_limitation"), "method_limitation");
        assert_eq!(normalize_gap_type("contradiction"), "method_limitation");
        assert_eq!(normalize_gap_type("unknown"), "improvement");
    }

    #[test]
    fn test_generate_capsule_id() {
        let pool = GenePool::new();
        let id = pool.generate_capsule_id();
        assert_eq!(id.len(), 12);
    }
}
