//! rairos-evolution — Evolution Memory: User Feedback & Pattern Learning.
//!
//! Ported from `llm/evolution.py`.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FeedbackType {
    Positive,
    Negative,
    Neutral,
}

impl FeedbackType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FeedbackType::Positive => "positive",
            FeedbackType::Negative => "negative",
            FeedbackType::Neutral => "neutral",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "positive" => Some(FeedbackType::Positive),
            "negative" => Some(FeedbackType::Negative),
            "neutral" => Some(FeedbackType::Neutral),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalType {
    ChatSuccess,
    ChatFailure,
    RetrievalHit,
    RetrievalMiss,
    SlideQuality,
    SearchSuccess,
}

impl SignalType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SignalType::ChatSuccess => "chat_success",
            SignalType::ChatFailure => "chat_failure",
            SignalType::RetrievalHit => "retrieval_hit",
            SignalType::RetrievalMiss => "retrieval_miss",
            SignalType::SlideQuality => "slide_quality",
            SignalType::SearchSuccess => "search_success",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feedback {
    pub id: String,
    #[serde(rename = "type")]
    pub feedback_type: String,
    pub command: String,
    pub query: String,
    #[serde(default)]
    pub paper_ids: Vec<String>,
    pub outcome: String,
    pub score: f64,
    #[serde(default)]
    pub note: String,
    pub timestamp: String,
}

impl Feedback {
    pub fn new(
        id: &str,
        feedback_type: FeedbackType,
        command: &str,
        query: &str,
        paper_ids: Vec<String>,
        outcome: &str,
        score: f64,
        note: &str,
    ) -> Self {
        Self {
            id: id.to_string(),
            feedback_type: feedback_type.as_str().to_string(),
            command: command.to_string(),
            query: query.to_string(),
            paper_ids,
            outcome: outcome.to_string(),
            score,
            note: note.to_string(),
            timestamp: Utc::now().to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionEvent {
    pub id: String,
    pub signal_type: String,
    #[serde(default)]
    pub trigger: HashMap<String, serde_json::Value>,
    pub action: String,
    pub outcome: String,
    pub score: f64,
    #[serde(default)]
    pub genes_applied: Vec<String>,
    pub timestamp: String,
}

impl EvolutionEvent {
    pub fn new(
        id: &str,
        signal_type: SignalType,
        trigger: HashMap<String, serde_json::Value>,
        action: &str,
        outcome: &str,
        score: f64,
        genes_applied: Vec<String>,
    ) -> Self {
        Self {
            id: id.to_string(),
            signal_type: signal_type.as_str().to_string(),
            trigger,
            action: action.to_string(),
            outcome: outcome.to_string(),
            score,
            genes_applied,
            timestamp: Utc::now().to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedPattern {
    pub name: String,
    pub signal_type: String,
    #[serde(default)]
    pub trigger_conditions: HashMap<String, serde_json::Value>,
    pub success_count: i32,
    pub failure_count: i32,
    #[serde(default)]
    pub last_used: String,
    pub effectiveness: f64,
}

impl LearnedPattern {
    pub fn total_attempts(&self) -> i32 {
        self.success_count + self.failure_count
    }

    pub fn is_reliable(&self) -> bool {
        self.total_attempts() >= 3 && self.effectiveness >= 0.7
    }
}

pub struct EvolutionMemory {
    memory_dir: PathBuf,
    feedback_file: PathBuf,
    events_file: PathBuf,
    patterns_file: PathBuf,
}

impl EvolutionMemory {
    pub fn new(memory_dir: Option<PathBuf>) -> Self {
        let memory_dir = memory_dir.unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".ai_research_os")
                .join("memory")
                .join("evolution")
        });

        let feedback_file = memory_dir.join("feedback.jsonl");
        let events_file = memory_dir.join("evolution_events.jsonl");
        let patterns_file = memory_dir.join("learned_patterns.json");

        if let Some(parent) = memory_dir.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::create_dir_all(&memory_dir);

        if !feedback_file.exists() {
            let _ = fs::write(&feedback_file, "");
        }
        if !events_file.exists() {
            let _ = fs::write(&events_file, "");
        }
        if !patterns_file.exists() {
            let _ = fs::write(&patterns_file, "{}");
        }

        Self {
            memory_dir,
            feedback_file,
            events_file,
            patterns_file,
        }
    }

    pub fn feedback_file(&self) -> &PathBuf {
        &self.feedback_file
    }

    pub fn add_feedback(&self, feedback: &Feedback) -> std::io::Result<()> {
        let json = serde_json::to_string(feedback).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(&self.feedback_file, format!("{}\n", json))?;
        Ok(())
    }

    pub fn record_chat_feedback(
        &self,
        query: &str,
        paper_ids: Vec<String>,
        is_positive: bool,
        outcome: &str,
        score: f64,
        note: &str,
    ) -> std::io::Result<()> {
        let fb_type = if is_positive { FeedbackType::Positive } else { FeedbackType::Negative };
        let id = format!("fb_{}", Utc::now().timestamp_millis());
        let feedback = Feedback::new(&id, fb_type, "chat", query, paper_ids, outcome, score, note);
        self.add_feedback(&feedback)?;

        let signal = if is_positive { SignalType::ChatSuccess } else { SignalType::ChatFailure };
        let mut trigger = HashMap::new();
        trigger.insert("query".to_string(), serde_json::json!(query));
        trigger.insert("papers".to_string(), serde_json::json!(feedback.paper_ids));

        let event = EvolutionEvent::new(
            &format!("ev_{}", Utc::now().timestamp_millis()),
            signal,
            trigger,
            "chat_response",
            outcome,
            score,
            vec![],
        );
        self.record_evolution_event(&event)
    }

    pub fn record_evolution_event(&self, event: &EvolutionEvent) -> std::io::Result<()> {
        let json = serde_json::to_string(event).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(&self.events_file, format!("{}\n", json))?;
        self.update_pattern_from_event(event);
        Ok(())
    }

    fn update_pattern_from_event(&self, event: &EvolutionEvent) {
        let mut patterns = self.load_patterns();
        let pattern_key = format!("{}_{}", event.signal_type, event.action);

        if !patterns.contains_key(&pattern_key) {
            patterns.insert(pattern_key.clone(), LearnedPattern {
                name: pattern_key.clone(),
                signal_type: event.signal_type.clone(),
                trigger_conditions: event.trigger.clone(),
                success_count: 0,
                failure_count: 0,
                last_used: String::new(),
                effectiveness: 0.0,
            });
        }

        if let Some(p) = patterns.get_mut(&pattern_key) {
            if event.score >= 0.6 {
                p.success_count += 1;
            } else {
                p.failure_count += 1;
            }
            let total = p.success_count + p.failure_count;
            p.effectiveness = if total > 0 { p.success_count as f64 / total as f64 } else { 0.0 };
            p.last_used = event.timestamp.clone();
        }

        let _ = self.save_patterns(&patterns);
    }

    fn load_patterns(&self) -> HashMap<String, LearnedPattern> {
        match fs::read_to_string(&self.patterns_file) {
            Ok(text) => {
                if text.trim().is_empty() {
                    return HashMap::new();
                }
                serde_json::from_str(&text).unwrap_or_else(|_| HashMap::new())
            }
            Err(_) => HashMap::new(),
        }
    }

    fn save_patterns(&self, patterns: &HashMap<String, LearnedPattern>) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(patterns).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(&self.patterns_file, json)
    }

    pub fn get_reliable_patterns(&self) -> Vec<LearnedPattern> {
        self.load_patterns()
            .into_values()
            .filter(|p| p.total_attempts() >= 3 && p.effectiveness >= 0.7)
            .collect()
    }

    pub fn get_all_patterns(&self) -> Vec<LearnedPattern> {
        self.load_patterns().into_values().collect()
    }

    pub fn get_stats(&self) -> HashMap<String, serde_json::Value> {
        let patterns = self.load_patterns();
        let mut feedback_count = 0i32;
        let mut positive_count = 0i32;

        if let Ok(text) = fs::read_to_string(&self.feedback_file) {
            for line in text.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                if let Ok(data) = serde_json::from_str::<HashMap<String, serde_json::Value>>(line) {
                    feedback_count += 1;
                    if let Some(t) = data.get("type").and_then(|v| v.as_str()) {
                        if t == "positive" {
                            positive_count += 1;
                        }
                    }
                }
            }
        }

        let event_count = fs::read_to_string(&self.events_file)
            .map(|text| text.lines().filter(|l| !l.trim().is_empty()).count() as i32)
            .unwrap_or(0);

        let reliable_count = self.get_reliable_patterns().len() as i32;
        let total_patterns = patterns.len() as i32;

        let mut stats = HashMap::new();
        stats.insert("total_feedback".to_string(), serde_json::json!(feedback_count));
        stats.insert("positive_feedback".to_string(), serde_json::json!(positive_count));
        stats.insert("negative_feedback".to_string(), serde_json::json!(feedback_count - positive_count));
        stats.insert("positive_rate".to_string(), serde_json::json!(if feedback_count > 0 { positive_count as f64 / feedback_count as f64 } else { 0.0 }));
        stats.insert("total_events".to_string(), serde_json::json!(event_count));
        stats.insert("total_patterns".to_string(), serde_json::json!(total_patterns));
        stats.insert("reliable_patterns".to_string(), serde_json::json!(reliable_count));
        stats.insert("learning_progress".to_string(), serde_json::json!(if reliable_count < 10 { reliable_count as f64 / 10.0 } else { 1.0 }));
        stats
    }
}

pub fn get_evolution_memory() -> EvolutionMemory {
    EvolutionMemory::new(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_memory() -> (EvolutionMemory, TempDir) {
        let temp = TempDir::new().unwrap();
        let memory = EvolutionMemory::new(Some(temp.path().to_path_buf()));
        (memory, temp)
    }

    #[test]
    fn test_feedback_type_as_str() {
        assert_eq!(FeedbackType::Positive.as_str(), "positive");
        assert_eq!(FeedbackType::Negative.as_str(), "negative");
        assert_eq!(FeedbackType::Neutral.as_str(), "neutral");
    }

    #[test]
    fn test_feedback_type_from_str() {
        assert_eq!(FeedbackType::from_str("positive"), Some(FeedbackType::Positive));
        assert_eq!(FeedbackType::from_str("negative"), Some(FeedbackType::Negative));
        assert_eq!(FeedbackType::from_str("neutral"), Some(FeedbackType::Neutral));
        assert_eq!(FeedbackType::from_str("invalid"), None);
    }

    #[test]
    fn test_signal_type_as_str() {
        assert_eq!(SignalType::ChatSuccess.as_str(), "chat_success");
        assert_eq!(SignalType::ChatFailure.as_str(), "chat_failure");
        assert_eq!(SignalType::RetrievalHit.as_str(), "retrieval_hit");
    }

    #[test]
    fn test_learned_pattern_total_attempts() {
        let pattern = LearnedPattern {
            name: "test".to_string(),
            signal_type: "chat_success".to_string(),
            trigger_conditions: HashMap::new(),
            success_count: 5,
            failure_count: 3,
            last_used: String::new(),
            effectiveness: 0.0,
        };
        assert_eq!(pattern.total_attempts(), 8);
    }

    #[test]
    fn test_learned_pattern_is_reliable() {
        let reliable = LearnedPattern {
            name: "test".to_string(),
            signal_type: "chat_success".to_string(),
            trigger_conditions: HashMap::new(),
            success_count: 7,
            failure_count: 3,
            last_used: String::new(),
            effectiveness: 0.7,
        };
        assert!(reliable.is_reliable());

        let unreliable = LearnedPattern {
            name: "test".to_string(),
            signal_type: "chat_success".to_string(),
            trigger_conditions: HashMap::new(),
            success_count: 1,
            failure_count: 0,
            last_used: String::new(),
            effectiveness: 1.0,
        };
        assert!(!unreliable.is_reliable());
    }

    #[test]
    fn test_evolution_memory_add_feedback() {
        let (memory, _temp) = temp_memory();
        let feedback = Feedback::new(
            "fb_123",
            FeedbackType::Positive,
            "chat",
            "test query",
            vec!["paper1".to_string()],
            "success",
            0.9,
            "great!",
        );
        assert!(memory.add_feedback(&feedback).is_ok());
    }

    #[test]
    fn test_evolution_memory_record_chat_feedback() {
        let (memory, _temp) = temp_memory();
        assert!(memory.record_chat_feedback(
            "test query",
            vec!["paper1".to_string()],
            true,
            "success",
            0.9,
            "great!"
        ).is_ok());
    }

    #[test]
    fn test_evolution_memory_get_stats() {
        let (memory, _temp) = temp_memory();
        let stats = memory.get_stats();
        assert_eq!(stats["total_feedback"], serde_json::json!(0));
        assert_eq!(stats["reliable_patterns"], serde_json::json!(0));
    }

    #[test]
    fn test_evolution_memory_get_reliable_patterns() {
        let (memory, _temp) = temp_memory();
        let patterns = memory.get_reliable_patterns();
        assert!(patterns.is_empty());
    }

    #[test]
    fn test_evolution_memory_get_all_patterns() {
        let (memory, _temp) = temp_memory();
        let patterns = memory.get_all_patterns();
        assert!(patterns.is_empty());
    }
}
