#![allow(dead_code)]
//! rairos-insight-tracker — EvolutionTracker core insight evolution engine.
//!
//! Ported from `llm/insight/tracker.py`.

use rairos_insight_types::{EvolutionEvent, ExplorationAction, UserPreferenceProfile};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const CACHE_TTL_SECONDS: u64 = 300;

const EVENT_WEIGHTS: &[(ExplorationAction, f64)] = &[
    (ExplorationAction::Viewed, 0.05),
    (ExplorationAction::Accepted, 0.30),
    (ExplorationAction::Rejected, -0.30),
    (ExplorationAction::Expanded, 0.20),
    (ExplorationAction::Hypothesized, 0.40),
    (ExplorationAction::Validated, 0.40),
    (ExplorationAction::Narrated, 0.25),
    (ExplorationAction::InsightRated, 0.20),
];

const REJECT_NO_HYPOTHESIS_PENALTY: f64 = -0.10;

#[derive(Debug, Clone)]
struct CacheEntry {
    scores: CacheScores,
    timestamp: Instant,
}

#[derive(Debug, Clone)]
struct CacheScores {
    gap_types: HashMap<String, f64>,
    keywords: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ArchetypeDimension {
    raw: f64,
    norm: f64,
    label: String,
    desc: String,
}

pub struct EvolutionTracker {
    data_dir: PathBuf,
    events_file: PathBuf,
    lifecycle_events_file: PathBuf,
    profile_file: PathBuf,
    sessions_dir: PathBuf,
    score_cache: Option<CacheEntry>,
}

impl EvolutionTracker {
    pub fn new(data_dir: Option<PathBuf>) -> Self {
        let data_dir = data_dir.unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".ai_research_os")
                .join("evolution")
        });
        fs::create_dir_all(&data_dir).ok();

        let events_file = data_dir.join("events.jsonl");
        let lifecycle_events_file = data_dir.join("lifecycle_events.jsonl");
        let profile_file = data_dir.join("preference_profile.json");
        let sessions_dir = data_dir.join("sessions");
        fs::create_dir_all(&sessions_dir).ok();

        Self {
            data_dir,
            events_file,
            lifecycle_events_file,
            profile_file,
            sessions_dir,
            score_cache: None,
        }
    }

    fn get_timestamp(&self) -> String {
        chrono::Utc::now().to_rfc3339()
    }

    pub fn record_capsule_lifecycle_event(
        &self,
        capsule_id: &str,
        action: &str,
        gap_title: &str,
        gap_type: &str,
        details: &str,
    ) {
        let event = serde_json::json!({
            "timestamp": self.get_timestamp(),
            "capsule_id": capsule_id,
            "action": action,
            "gap_title": gap_title,
            "gap_type": gap_type,
            "details": details,
        });
        if let Ok(file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.lifecycle_events_file)
        {
            let mut f = file;
            writeln!(f, "{}", event).ok();
        }
    }

    pub fn get_evolution_log(&self, limit: usize) -> Vec<HashMap<String, serde_json::Value>> {
        if !self.events_file.exists() {
            return Vec::new();
        }
        let content = fs::read_to_string(&self.events_file).unwrap_or_default();
        let mut events: Vec<HashMap<String, serde_json::Value>> = content
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();
        events.reverse();
        events.truncate(limit);
        events
    }

    pub fn record_event(&mut self, event: &EvolutionEvent) -> EvolutionEvent {
        let event_data = serde_json::json!({
            "timestamp": event.timestamp,
            "topic": event.topic,
            "action": event.action.as_str(),
            "gap_type": event.gap_type,
            "gap_title": event.gap_title,
            "gap_description": event.gap_description,
            "hypothesis_id": event.hypothesis_id,
            "question_id": event.question_id,
            "paper_ids": event.paper_ids,
            "duration_seconds": event.duration_seconds,
            "notes": event.notes,
            "insight_card_id": event.insight_card_id,
        });

        if let Ok(file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.events_file)
        {
            let mut f = file;
            writeln!(f, "{}", event_data).ok();
        }

        self.update_profile(event);

        self.score_cache = None;

        event.clone()
    }

    pub fn record_gap_view(
        &mut self,
        topic: &str,
        gap_type: &str,
        gap_title: &str,
        gap_description: &str,
        duration_seconds: i32,
    ) -> EvolutionEvent {
        let event = EvolutionEvent {
            timestamp: self.get_timestamp(),
            topic: topic.to_string(),
            action: ExplorationAction::Viewed,
            gap_type: gap_type.to_string(),
            gap_title: gap_title.to_string(),
            gap_description: gap_description.to_string(),
            hypothesis_id: String::new(),
            question_id: String::new(),
            paper_ids: Vec::new(),
            duration_seconds,
            notes: String::new(),
            insight_card_id: String::new(),
        };
        self.record_event(&event)
    }

    pub fn record_gap_accept(
        &mut self,
        topic: &str,
        gap_type: &str,
        gap_title: &str,
        gap_description: &str,
    ) -> EvolutionEvent {
        let event = EvolutionEvent {
            timestamp: self.get_timestamp(),
            topic: topic.to_string(),
            action: ExplorationAction::Accepted,
            gap_type: gap_type.to_string(),
            gap_title: gap_title.to_string(),
            gap_description: gap_description.to_string(),
            hypothesis_id: String::new(),
            question_id: String::new(),
            paper_ids: Vec::new(),
            duration_seconds: 0,
            notes: String::new(),
            insight_card_id: String::new(),
        };
        self.record_event(&event)
    }

    pub fn record_gap_reject(
        &mut self,
        topic: &str,
        gap_type: &str,
        gap_title: &str,
        reason: &str,
    ) -> EvolutionEvent {
        let event = EvolutionEvent {
            timestamp: self.get_timestamp(),
            topic: topic.to_string(),
            action: ExplorationAction::Rejected,
            gap_type: gap_type.to_string(),
            gap_title: gap_title.to_string(),
            gap_description: String::new(),
            hypothesis_id: String::new(),
            question_id: String::new(),
            paper_ids: Vec::new(),
            duration_seconds: 0,
            notes: reason.to_string(),
            insight_card_id: String::new(),
        };
        self.record_event(&event)
    }

    pub fn record_expand(
        &mut self,
        topic: &str,
        gap_type: &str,
        gap_title: &str,
        sub_questions: &[String],
    ) -> EvolutionEvent {
        let event = EvolutionEvent {
            timestamp: self.get_timestamp(),
            topic: topic.to_string(),
            action: ExplorationAction::Expanded,
            gap_type: gap_type.to_string(),
            gap_title: gap_title.to_string(),
            gap_description: String::new(),
            hypothesis_id: String::new(),
            question_id: String::new(),
            paper_ids: Vec::new(),
            duration_seconds: 0,
            notes: sub_questions.join("; "),
            insight_card_id: String::new(),
        };
        self.record_event(&event)
    }

    fn update_profile(&self, event: &EvolutionEvent) {
        let mut profile = self.load_profile();

        profile.total_events += 1;
        profile.last_updated = self.get_timestamp();

        match event.action {
            ExplorationAction::Viewed => profile.views += 1,
            ExplorationAction::Accepted => profile.accepts += 1,
            ExplorationAction::Rejected => profile.rejects += 1,
            ExplorationAction::Expanded => profile.expands += 1,
            ExplorationAction::Hypothesized => profile.hypothesizes += 1,
            _ => {}
        }

        if !event.topic.is_empty() {
            *profile
                .topic_frequency
                .entry(event.topic.clone())
                .or_insert(0) += 1;
            if !profile.topics_explored.contains(&event.topic) {
                profile.topics_explored.push(event.topic.clone());
            }
            profile.recent_topics.retain(|t| t != &event.topic);
            profile.recent_topics.insert(0, event.topic.clone());
            profile.recent_topics.truncate(10);
        }

        if !event.gap_type.is_empty() {
            let weight = self.get_event_weight(event);
            let current = profile
                .gap_type_preferences
                .get(&event.gap_type)
                .copied()
                .unwrap_or(0.0);
            profile
                .gap_type_preferences
                .insert(event.gap_type.clone(), current + weight);
        }

        profile.preference_tags = self.compute_preference_tags(&profile);

        self.save_profile(&profile);
    }

    fn get_event_weight(&self, event: &EvolutionEvent) -> f64 {
        let weight = EVENT_WEIGHTS
            .iter()
            .find(|(action, _)| *action == event.action)
            .map(|(_, w)| *w)
            .unwrap_or(0.0);

        if event.action == ExplorationAction::Rejected && event.hypothesis_id.is_empty() {
            REJECT_NO_HYPOTHESIS_PENALTY
        } else {
            weight
        }
    }

    fn compute_preference_tags(&self, profile: &UserPreferenceProfile) -> HashMap<String, f64> {
        let mut tags = HashMap::new();

        if !profile.gap_type_preferences.is_empty() {
            let top_type = profile
                .gap_type_preferences
                .iter()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map(|(k, _)| k.clone())
                .unwrap_or_default();

            let total_score: f64 = profile.gap_type_preferences.values().map(|v| v.abs()).sum();
            if total_score > 0.0 {
                let top_confidence = (profile
                    .gap_type_preferences
                    .get(&top_type)
                    .unwrap_or(&0.0)
                    .abs()
                    / total_score)
                    .min(1.0);

                if top_type.contains("method") {
                    tags.insert("method_focused".to_string(), top_confidence);
                } else if top_type.contains("application") || top_type.contains("unexplored") {
                    tags.insert("app_focused".to_string(), top_confidence);
                } else if top_type.contains("theoretical") {
                    tags.insert("theory_focused".to_string(), top_confidence);
                }
            }
        }

        let total = profile.views.max(1) as f64;
        let accept_rate = profile.accepts as f64 / total;
        let reject_rate = profile.rejects as f64 / total;

        if accept_rate > 0.3 {
            tags.insert("exploratory".to_string(), accept_rate);
        }
        if reject_rate > 0.3 {
            tags.insert("low_risk".to_string(), reject_rate);
        }

        if profile.hypothesizes as f64 > profile.views as f64 * 0.2 {
            let hypo_rate = (profile.hypothesizes as f64
                / (profile.views + profile.accepts).max(1) as f64)
                .min(1.0);
            tags.insert("high_risk".to_string(), hypo_rate);
        }

        if profile.topics_explored.len() >= 3 {
            let topics_str = profile.topics_explored.join(" ").to_lowercase();
            let domain_indicators = [
                "nlp",
                "vision",
                "audio",
                "graph",
                "reinforcement",
                "supervised",
            ];
            let detected: usize = domain_indicators
                .iter()
                .filter(|d| topics_str.contains(*d))
                .count();
            if detected >= 2 {
                let confidence = (detected as f64 / domain_indicators.len() as f64).min(1.0);
                tags.insert("cross_domain".to_string(), confidence);
            }
        }

        tags
    }

    fn load_profile(&self) -> UserPreferenceProfile {
        if self.profile_file.exists() {
            if let Ok(content) = fs::read_to_string(&self.profile_file) {
                if let Ok(profile) = serde_json::from_str(&content) {
                    return profile;
                }
            }
        }
        UserPreferenceProfile::new()
    }

    fn save_profile(&self, profile: &UserPreferenceProfile) {
        if let Ok(json) = serde_json::to_string_pretty(profile) {
            fs::write(&self.profile_file, json).ok();
        }
    }

    pub fn get_profile(&self) -> UserPreferenceProfile {
        self.load_profile()
    }

    pub fn get_archetype(&self) -> serde_json::Value {
        let profile = self.load_profile();
        let event_count = profile.total_events.max(1);

        let mut dimension_scores: HashMap<&str, f64> = HashMap::new();
        dimension_scores.insert("method_focused", 0.0);
        dimension_scores.insert("app_focused", 0.0);
        dimension_scores.insert("theory_focused", 0.0);
        dimension_scores.insert("high_risk", 0.0);
        dimension_scores.insert("low_risk", 0.0);
        dimension_scores.insert("exploratory", 0.0);
        dimension_scores.insert("confirmatory", 0.0);
        dimension_scores.insert("cross_domain", 0.0);

        let gap_prefs = &profile.gap_type_preferences;
        for (gap_type, score) in gap_prefs {
            if *score <= 0.0 {
                continue;
            }
            if gap_type.contains("method") {
                *dimension_scores.entry("method_focused").or_insert(0.0) += score;
            }
            if gap_type.contains("application") || gap_type.contains("unexplored") {
                *dimension_scores.entry("app_focused").or_insert(0.0) += score;
            }
            if gap_type.contains("theoretical") {
                *dimension_scores.entry("theory_focused").or_insert(0.0) += score;
            }
        }

        let hypothesizes = profile.hypothesizes.max(0) as f64;
        let total = event_count as f64;
        if hypothesizes / total > 0.2 {
            *dimension_scores.entry("exploratory").or_insert(0.0) += hypothesizes * 0.5;
        }

        let accepts = profile.accepts.max(0) as f64;
        if accepts / total > 0.4 {
            *dimension_scores.entry("confirmatory").or_insert(0.0) += accepts * 0.3;
        }

        let topics_set: std::collections::HashSet<_> = profile.topics_explored.iter().collect();
        if topics_set.len() > 5 {
            *dimension_scores.entry("cross_domain").or_insert(0.0) =
                (topics_set.len() as f64 * 0.3).min(3.0);
        }

        let method_score = gap_prefs.get("method_limitation").copied().unwrap_or(0.0);
        if method_score > 0.5 {
            *dimension_scores.entry("high_risk").or_insert(0.0) = method_score * 2.0;
        } else if method_score < -0.2 {
            *dimension_scores.entry("low_risk").or_insert(0.0) = method_score.abs() * 2.0;
        }

        let max_raw = dimension_scores
            .values()
            .cloned()
            .fold(0.01f64, |a, b| a.max(b));

        let labels: HashMap<&str, &str> = HashMap::from([
            ("method_focused", "Method Hunter"),
            ("app_focused", "Application Pioneer"),
            ("theory_focused", "Theory Builder"),
            ("high_risk", "Risk Taker"),
            ("low_risk", "Steady Researcher"),
            ("exploratory", "Explorer"),
            ("confirmatory", "Verifier"),
            ("cross_domain", "Bridge Builder"),
        ]);

        let descs: HashMap<&str, &str> = HashMap::from([
            ("method_focused", "Focuses on methodology & theory"),
            ("app_focused", "Prioritizes real-world applications"),
            ("theory_focused", "Pursues rigorous foundations"),
            ("high_risk", "Tackles high-uncertainty problems"),
            ("low_risk", "Prefers robust, reproducible work"),
            ("exploratory", "Loves discovering new questions"),
            ("confirmatory", "Focuses on validation & replication"),
            ("cross_domain", "Interested in interdisciplinary work"),
        ]);

        let mut dimensions = HashMap::new();
        for (dim, raw) in &dimension_scores {
            let norm = (*raw / max_raw).min(1.0);
            dimensions.insert(
                dim.to_string(),
                serde_json::json!({
                    "raw": (*raw * 1000.0).round() / 1000.0,
                    "norm": (norm * 100.0).round() / 100.0,
                    "label": labels.get(dim).unwrap_or(dim),
                    "desc": descs.get(dim).unwrap_or(&""),
                }),
            );
        }

        let dominant = dimension_scores
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(k, _)| *k)
            .unwrap_or("method_focused");

        let confidence = (event_count as f64 / 20.0).min(1.0);

        serde_json::json!({
            "dimensions": dimensions,
            "dominant": dominant,
            "archetype_label": labels.get(dominant).unwrap_or(&dominant),
            "confidence": (confidence * 100.0).round() / 100.0,
            "event_count": event_count,
        })
    }

    fn is_cache_valid(&self) -> bool {
        if let Some(ref entry) = self.score_cache {
            entry.timestamp.elapsed() < Duration::from_secs(CACHE_TTL_SECONDS)
        } else {
            false
        }
    }

    fn get_all_scores_cached(&mut self) -> CacheScores {
        if self.is_cache_valid() {
            return self.score_cache.as_ref().unwrap().scores.clone();
        }

        let events = self.get_recent_events_impl(10000);
        let mut gap_scores: HashMap<String, f64> = HashMap::new();
        let mut kw_scores: HashMap<String, f64> = HashMap::new();

        for e in &events {
            let weight = self.get_event_weight(e);
            let decayed = self.decay_weight(weight, &e.timestamp, 0.01);

            if !e.gap_type.is_empty() {
                *gap_scores.entry(e.gap_type.clone()).or_insert(0.0) += decayed;
            }

            if !e.gap_title.is_empty() {
                let keywords = extract_keywords_simple(&e.gap_title);
                for kw in keywords {
                    *kw_scores.entry(kw).or_insert(0.0) += decayed * 0.5;
                }
            }
        }

        let scores = CacheScores {
            gap_types: gap_scores,
            keywords: kw_scores,
        };

        self.score_cache = Some(CacheEntry {
            scores: scores.clone(),
            timestamp: Instant::now(),
        });

        scores
    }

    fn decay_weight(&self, base_weight: f64, event_timestamp: &str, lambda: f64) -> f64 {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(event_timestamp) {
            let now = chrono::Utc::now();
            let duration = now.signed_duration_since(dt.with_timezone(&chrono::Utc));
            let age_days = duration.num_seconds() as f64 / 86400.0;
            let decay = 2.0_f64.powf(-lambda * age_days);
            return base_weight * decay;
        }
        0.0
    }

    pub fn get_preferred_gap_types(&mut self, limit: usize) -> Vec<String> {
        let scores = self.get_all_scores_cached();
        let mut sorted: Vec<_> = scores.gap_types.iter().collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
        sorted
            .into_iter()
            .take(limit)
            .filter(|(_, score)| **score > 0.0)
            .map(|(gt, _)| gt.clone())
            .collect()
    }

    pub fn get_disliked_gap_types(&mut self, limit: usize) -> Vec<String> {
        let scores = self.get_all_scores_cached();
        scores
            .gap_types
            .iter()
            .filter(|(_, score)| **score < -0.05)
            .take(limit)
            .map(|(gt, _)| gt.clone())
            .collect()
    }

    pub fn get_gap_type_score(&mut self, gap_type: &str) -> f64 {
        let scores = self.get_all_scores_cached();
        scores.gap_types.get(gap_type).copied().unwrap_or(0.0)
    }

    pub fn get_keyword_score(&mut self, keyword: &str) -> f64 {
        let scores = self.get_all_scores_cached();
        scores
            .keywords
            .get(&keyword.to_lowercase())
            .copied()
            .unwrap_or(0.0)
    }

    pub fn get_top_keywords(&mut self, limit: usize) -> Vec<String> {
        let scores = self.get_all_scores_cached();
        let mut sorted: Vec<_> = scores.keywords.iter().collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
        sorted
            .into_iter()
            .take(limit)
            .filter(|(_, score)| **score > 0.05)
            .map(|(kw, _)| kw.clone())
            .collect()
    }

    pub fn should_deprioritize_gap_type(&mut self, gap_type: &str) -> bool {
        self.get_gap_type_score(gap_type) < -0.05
    }

    pub fn get_exploration_stats(&mut self) -> HashMap<String, serde_json::Value> {
        let profile = self.load_profile();
        let recent = self.get_recent_events_impl(100);

        let mut stats = HashMap::new();
        stats.insert(
            "total_events".to_string(),
            serde_json::json!(profile.total_events),
        );
        stats.insert(
            "total_sessions".to_string(),
            serde_json::json!(profile.total_sessions),
        );
        stats.insert(
            "total_topics".to_string(),
            serde_json::json!(profile.topics_explored.len()),
        );
        stats.insert("recent_events".to_string(), serde_json::json!(recent.len()));
        stats.insert(
            "preference_tags".to_string(),
            serde_json::json!(profile.preference_tags),
        );
        stats.insert(
            "top_gap_types".to_string(),
            serde_json::json!(self.get_preferred_gap_types(5)),
        );

        let mut topic_freq = profile.topic_frequency.iter().collect::<Vec<_>>();
        topic_freq.sort_by(|a, b| b.1.cmp(a.1));
        stats.insert(
            "topic_frequency".to_string(),
            serde_json::json!(topic_freq.into_iter().take(5).collect::<HashMap<_, _>>()),
        );

        if !recent.is_empty() {
            let mut action_counts: HashMap<String, i32> = HashMap::new();
            for e in &recent {
                *action_counts
                    .entry(e.action.as_str().to_string())
                    .or_insert(0) += 1;
            }
            stats.insert(
                "recent_action_breakdown".to_string(),
                serde_json::json!(action_counts),
            );
        }

        stats
    }

    fn get_recent_events_impl(&self, limit: usize) -> Vec<EvolutionEvent> {
        if !self.events_file.exists() {
            return Vec::new();
        }

        let content = fs::read_to_string(&self.events_file).unwrap_or_default();
        let events: Vec<EvolutionEvent> = content
            .lines()
            .filter_map(|line| {
                let data: HashMap<String, serde_json::Value> = serde_json::from_str(line).ok()?;
                let action_str = data.get("action")?.as_str()?;
                let action = ExplorationAction::from_str(action_str)?;
                Some(EvolutionEvent {
                    timestamp: data.get("timestamp")?.as_str()?.to_string(),
                    topic: data.get("topic")?.as_str()?.to_string(),
                    action,
                    gap_type: data
                        .get("gap_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    gap_title: data
                        .get("gap_title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    gap_description: data
                        .get("gap_description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    hypothesis_id: data
                        .get("hypothesis_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    question_id: data
                        .get("question_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    paper_ids: data
                        .get("paper_ids")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|x| x.as_str())
                                .map(|s| s.to_string())
                                .collect()
                        })
                        .unwrap_or_default(),
                    duration_seconds: data
                        .get("duration_seconds")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0) as i32,
                    notes: data
                        .get("notes")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    insight_card_id: data
                        .get("insight_card_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                })
            })
            .collect();

        events.into_iter().rev().take(limit).collect()
    }

    pub fn get_recent_events(&self, topic: Option<&str>, limit: usize) -> Vec<EvolutionEvent> {
        let mut events = self.get_recent_events_impl(limit);
        if let Some(t) = topic {
            events.retain(|e| e.topic == t);
        }
        events
    }

    pub fn get_topic_history(&self, topic: &str) -> Vec<EvolutionEvent> {
        self.get_recent_events(Some(topic), 1000)
    }

    pub fn export_profile(&self, path: Option<PathBuf>) -> PathBuf {
        let profile = self.load_profile();
        let mut data = serde_json::to_value(&profile).unwrap_or_default();
        data["_exported_at"] = serde_json::json!(self.get_timestamp());
        data["_version"] = serde_json::json!("1.0");

        let export_path = path.unwrap_or_else(|| {
            let ts = self.get_timestamp().replace(":", "-").replace(".", "-");
            let uid = &uuid::Uuid::new_v4().to_string()[..6];
            self.data_dir
                .join(format!("profile_backup_{}_{}", &ts[..19], uid))
        });

        if let Some(parent) = export_path.parent() {
            fs::create_dir_all(parent).ok();
        }

        if let Ok(json) = serde_json::to_string_pretty(&data) {
            fs::write(&export_path, json).ok();
        }

        export_path
    }

    pub fn import_profile(&mut self, path: &Path, merge: bool) -> UserPreferenceProfile {
        if !path.exists() {
            return self.load_profile();
        }

        let content = fs::read_to_string(path).ok();
        let incoming: HashMap<String, serde_json::Value> = content
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default();

        let mut data = incoming.clone();
        data.remove("_exported_at");
        data.remove("_version");

        if merge {
            let existing = self.load_profile();
            let merged = self.merge_profiles_impl(&existing, &data);
            self.save_profile(&merged);
            self.score_cache = None;
            merged
        } else {
            let profile: UserPreferenceProfile =
                serde_json::from_value(serde_json::to_value(&data).unwrap_or_default())
                    .unwrap_or_default();
            self.save_profile(&profile);
            self.score_cache = None;
            profile
        }
    }

    fn merge_profiles_impl(
        &self,
        base: &UserPreferenceProfile,
        incoming: &HashMap<String, serde_json::Value>,
    ) -> UserPreferenceProfile {
        let mut result = base.clone();
        result.total_sessions = result.total_sessions.max(
            incoming
                .get("total_sessions")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32,
        );
        result.total_events = result.total_events.max(
            incoming
                .get("total_events")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32,
        );
        result
    }
}

fn extract_keywords_simple(text: &str) -> Vec<String> {
    let text_lower = text.to_lowercase();
    let words: Vec<&str> = text_lower.split_whitespace().collect();
    let stopwords = [
        "a", "an", "the", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by",
        "from", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had", "do",
        "does", "did", "will", "would", "could", "should", "may", "might", "must", "shall", "can",
        "need", "that", "which", "who", "whom", "this", "these", "those", "it", "its",
    ];
    words
        .into_iter()
        .filter(|w| w.len() > 2 && !stopwords.contains(w))
        .map(|w| w.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tracker() -> (EvolutionTracker, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().unwrap();
        let tracker = EvolutionTracker::new(Some(temp_dir.path().to_path_buf()));
        (tracker, temp_dir)
    }

    #[test]
    fn test_new_tracker() {
        let (tracker, _temp_dir) = create_test_tracker();
        let profile = tracker.get_profile();
        assert_eq!(profile.total_events, 0);
    }

    #[test]
    fn test_record_gap_view() {
        let (mut tracker, _temp_dir) = create_test_tracker();
        let event = tracker.record_gap_view(
            "NLP",
            "method_limitation",
            "Attention improvements",
            "Better attention",
            30,
        );
        assert_eq!(event.topic, "NLP");
        assert_eq!(event.action, ExplorationAction::Viewed);
    }

    #[test]
    fn test_record_gap_accept() {
        let (mut tracker, _temp_dir) = create_test_tracker();
        let event = tracker.record_gap_accept(
            "NLP",
            "method_limitation",
            "Attention improvements",
            "Better attention",
        );
        assert_eq!(event.action, ExplorationAction::Accepted);
    }

    #[test]
    fn test_record_gap_reject() {
        let (mut tracker, _temp_dir) = create_test_tracker();
        let event = tracker.record_gap_reject(
            "NLP",
            "method_limitation",
            "Attention improvements",
            "Not relevant",
        );
        assert_eq!(event.action, ExplorationAction::Rejected);
    }

    #[test]
    fn test_get_preferred_gap_types() {
        let (mut tracker, _temp_dir) = create_test_tracker();
        tracker.record_gap_accept("NLP", "method_limitation", "Attention", "");
        tracker.record_gap_accept("NLP", "method_limitation", "Attention2", "");
        let preferred = tracker.get_preferred_gap_types(3);
        assert!(preferred.contains(&"method_limitation".to_string()));
    }

    #[test]
    fn test_get_archetype() {
        let (mut tracker, _temp_dir) = create_test_tracker();
        tracker.record_gap_accept("NLP", "method_limitation", "Attention", "");
        tracker.record_gap_accept("NLP", "method_limitation", "Transformers", "");
        let archetype = tracker.get_archetype();
        if let Some(obj) = archetype.as_object() {
            assert!(obj.contains_key("dimensions"));
            assert!(obj.contains_key("dominant"));
        }
    }

    #[test]
    fn test_get_exploration_stats() {
        let (mut tracker, _temp_dir) = create_test_tracker();
        tracker.record_gap_view("NLP", "method_limitation", "Attention", "", 30);
        tracker.record_gap_accept("NLP", "method_limitation", "Transformers", "");
        let stats = tracker.get_exploration_stats();
        assert!(stats.contains_key("total_events"));
    }

    #[test]
    fn test_decay_weight() {
        let (mut tracker, _temp_dir) = create_test_tracker();
        let weight = tracker.decay_weight(1.0, "2024-01-01T00:00:00Z", 0.01);
        assert!(weight < 1.0);
        assert!(weight > 0.0);
    }

    #[test]
    fn test_get_keyword_score() {
        let (mut tracker, _temp_dir) = create_test_tracker();
        tracker.record_gap_accept(
            "NLP",
            "method_limitation",
            "Transformers and attention mechanism",
            "",
        );
        let score = tracker.get_keyword_score("transformers");
        assert!(score > 0.0);
    }
}
