//! rairos-friction-tracker — Research Friction Tracker.
//!
//! Ported from `llm/friction_tracker.py`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FrictionType {
    Command,
    Workflow,
    Retrieval,
    Cognitive,
    Navigation,
}

impl FrictionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FrictionType::Command => "command",
            FrictionType::Workflow => "workflow",
            FrictionType::Retrieval => "retrieval",
            FrictionType::Cognitive => "cognitive",
            FrictionType::Navigation => "navigation",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "command" => Some(FrictionType::Command),
            "workflow" => Some(FrictionType::Workflow),
            "retrieval" => Some(FrictionType::Retrieval),
            "cognitive" => Some(FrictionType::Cognitive),
            "navigation" => Some(FrictionType::Navigation),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FrictionSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl FrictionSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            FrictionSeverity::Low => "low",
            FrictionSeverity::Medium => "medium",
            FrictionSeverity::High => "high",
            FrictionSeverity::Critical => "critical",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "low" => Some(FrictionSeverity::Low),
            "medium" => Some(FrictionSeverity::Medium),
            "high" => Some(FrictionSeverity::High),
            "critical" => Some(FrictionSeverity::Critical),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Resolution {
    Retried,
    Abandoned,
    WorkedAround,
    SelfResolved,
    SystemHelped,
}

impl Resolution {
    pub fn as_str(&self) -> &'static str {
        match self {
            Resolution::Retried => "retried",
            Resolution::Abandoned => "abandoned",
            Resolution::WorkedAround => "worked_around",
            Resolution::SelfResolved => "self_resolved",
            Resolution::SystemHelped => "system_helped",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "retried" => Some(Resolution::Retried),
            "abandoned" => Some(Resolution::Abandoned),
            "worked_around" => Some(Resolution::WorkedAround),
            "self_resolved" => Some(Resolution::SelfResolved),
            "system_helped" => Some(Resolution::SystemHelped),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrictionEvent {
    pub id: String,
    pub timestamp: String,
    pub friction_type: String,
    pub severity: String,
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub step: String,
    #[serde(default)]
    pub error: String,
    #[serde(default)]
    pub resolution: String,
    #[serde(default)]
    pub duration_seconds: i32,
    #[serde(default)]
    pub retry_count: i32,
    #[serde(default)]
    pub abandoned: bool,
    #[serde(default)]
    pub notes: String,
}

impl FrictionEvent {
    pub fn new(
        id: &str,
        friction_type: FrictionType,
        severity: FrictionSeverity,
        command: &str,
        query: &str,
        step: &str,
        error: &str,
        resolution: Option<Resolution>,
        duration_seconds: i32,
        retry_count: i32,
        abandoned: bool,
        notes: &str,
    ) -> Self {
        Self {
            id: id.to_string(),
            timestamp: Utc::now().to_rfc3339(),
            friction_type: friction_type.as_str().to_string(),
            severity: severity.as_str().to_string(),
            command: command.to_string(),
            query: query.to_string(),
            step: step.to_string(),
            error: error.to_string(),
            resolution: resolution.map(|r| r.as_str().to_string()).unwrap_or_default(),
            duration_seconds,
            retry_count,
            abandoned,
            notes: notes.to_string(),
        }
    }
}

pub struct FrictionTracker {
    data_dir: PathBuf,
    events_file: PathBuf,
}

impl FrictionTracker {
    pub fn new(data_dir: Option<PathBuf>) -> Self {
        let data_dir = data_dir.unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".ai_research_os")
                .join("friction")
        });
        let _ = fs::create_dir_all(&data_dir);
        let events_file = data_dir.join("friction_events.jsonl");
        if !events_file.exists() {
            let _ = fs::write(&events_file, "");
        }
        Self { data_dir, events_file }
    }

    pub fn record(
        &self,
        friction_type: FrictionType,
        severity: FrictionSeverity,
        command: &str,
        query: &str,
        step: &str,
        error: &str,
        resolution: Option<Resolution>,
        duration_seconds: i32,
        retry_count: i32,
        abandoned: bool,
        notes: &str,
    ) -> FrictionEvent {
        let id = format!("fr_{}", uuid::Uuid::new_v4().to_string()[..8].to_string());
        let event = FrictionEvent::new(
            &id,
            friction_type,
            severity,
            command,
            query,
            step,
            error,
            resolution,
            duration_seconds,
            retry_count,
            abandoned,
            notes,
        );

        let json = serde_json::to_string(&event).unwrap_or_default();
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.events_file)
            .unwrap_or_else(|_| {
                fs::File::create(&self.events_file).expect("Failed to create events file")
            });
        use std::io::Write;
        writeln!(file, "{}", json).ok();
        event
    }

    pub fn record_command_failure(
        &self,
        command: &str,
        query: &str,
        error: &str,
        retry_count: i32,
    ) -> FrictionEvent {
        let severity = if retry_count >= 3 {
            FrictionSeverity::High
        } else {
            FrictionSeverity::Medium
        };
        let resolution = if retry_count > 0 {
            Some(Resolution::Retried)
        } else {
            Some(Resolution::SelfResolved)
        };
        self.record(
            FrictionType::Command,
            severity,
            command,
            query,
            "",
            error,
            resolution,
            0,
            retry_count,
            false,
            "",
        )
    }

    pub fn record_workflow_abandon(
        &self,
        command: &str,
        step: &str,
        query: &str,
        duration_seconds: i32,
    ) -> FrictionEvent {
        self.record(
            FrictionType::Workflow,
            FrictionSeverity::Medium,
            command,
            query,
            step,
            "",
            Some(Resolution::Abandoned),
            duration_seconds,
            0,
            true,
            "",
        )
    }

    pub fn record_retrieval_failure(
        &self,
        command: &str,
        query: &str,
        notes: &str,
    ) -> FrictionEvent {
        self.record(
            FrictionType::Retrieval,
            FrictionSeverity::Medium,
            command,
            query,
            "",
            "",
            Some(Resolution::WorkedAround),
            0,
            0,
            false,
            notes,
        )
    }

    pub fn get_events(
        &self,
        friction_type: Option<FrictionType>,
        since_days: i32,
        limit: usize,
    ) -> Vec<FrictionEvent> {
        let cutoff = Utc::now() - chrono::Duration::days(since_days as i64);
        let cutoff_ts = cutoff.timestamp();

        let mut events = Vec::new();
        if let Ok(text) = fs::read_to_string(&self.events_file) {
            for line in text.lines().rev() {
                if limit > 0 && events.len() >= limit {
                    break;
                }
                if let Ok(event) = serde_json::from_str::<FrictionEvent>(line) {
                    if let Ok(event_time) = DateTime::parse_from_rfc3339(&event.timestamp) {
                        if event_time.timestamp() < cutoff_ts {
                            break;
                        }
                    }
                    if let Some(ft) = friction_type {
                        if event.friction_type != ft.as_str() {
                            continue;
                        }
                    }
                    events.push(event);
                }
            }
        }
        events
    }

    pub fn get_summary(&self, since_days: i32) -> HashMap<String, serde_json::Value> {
        let events = self.get_events(None, since_days, 1000);
        if events.is_empty() {
            let mut map = HashMap::new();
            map.insert("total_events".to_string(), serde_json::json!(0));
            map.insert("by_type".to_string(), serde_json::json!({}));
            map.insert("by_severity".to_string(), serde_json::json!({}));
            map.insert("top_commands".to_string(), serde_json::json!([]));
            map.insert("abandon_rate".to_string(), serde_json::json!(0.0));
            return map;
        }

        let mut by_type: HashMap<&str, usize> = HashMap::new();
        let mut by_severity: HashMap<&str, usize> = HashMap::new();
        let mut command_counts: HashMap<&str, usize> = HashMap::new();
        let mut abandoned = 0usize;

        for e in &events {
            *by_type.entry(&e.friction_type).or_insert(0) += 1;
            *by_severity.entry(&e.severity).or_insert(0) += 1;
            if !e.command.is_empty() {
                *command_counts.entry(&e.command).or_insert(0) += 1;
            }
            if e.abandoned {
                abandoned += 1;
            }
        }

        let mut top_commands: Vec<(String, usize)> = command_counts
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        top_commands.sort_by_key(|b| std::cmp::Reverse(b.1));
        top_commands.truncate(5);

        let mut summary = HashMap::new();
        summary.insert("total_events".to_string(), serde_json::json!(events.len()));
        summary.insert("by_type".to_string(), serde_json::json!(by_type));
        summary.insert("by_severity".to_string(), serde_json::json!(by_severity));
        summary.insert("top_commands".to_string(), serde_json::json!(top_commands));
        summary.insert("abandon_rate".to_string(), serde_json::json!(abandoned as f64 / events.len() as f64));

        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_tracker() -> (FrictionTracker, tempfile::TempDir) {
        let temp = tempfile::TempDir::new().unwrap();
        let tracker = FrictionTracker::new(Some(temp.path().to_path_buf()));
        (tracker, temp)
    }

    #[test]
    fn test_friction_type_as_str() {
        assert_eq!(FrictionType::Command.as_str(), "command");
        assert_eq!(FrictionType::Workflow.as_str(), "workflow");
        assert_eq!(FrictionType::Retrieval.as_str(), "retrieval");
        assert_eq!(FrictionType::Cognitive.as_str(), "cognitive");
        assert_eq!(FrictionType::Navigation.as_str(), "navigation");
    }

    #[test]
    fn test_friction_type_from_str() {
        assert_eq!(FrictionType::from_str("command"), Some(FrictionType::Command));
        assert_eq!(FrictionType::from_str("workflow"), Some(FrictionType::Workflow));
        assert_eq!(FrictionType::from_str("invalid"), None);
    }

    #[test]
    fn test_friction_severity_as_str() {
        assert_eq!(FrictionSeverity::Low.as_str(), "low");
        assert_eq!(FrictionSeverity::Medium.as_str(), "medium");
        assert_eq!(FrictionSeverity::High.as_str(), "high");
        assert_eq!(FrictionSeverity::Critical.as_str(), "critical");
    }

    #[test]
    fn test_resolution_as_str() {
        assert_eq!(Resolution::Retried.as_str(), "retried");
        assert_eq!(Resolution::Abandoned.as_str(), "abandoned");
        assert_eq!(Resolution::WorkedAround.as_str(), "worked_around");
        assert_eq!(Resolution::SelfResolved.as_str(), "self_resolved");
        assert_eq!(Resolution::SystemHelped.as_str(), "system_helped");
    }

    #[test]
    fn test_friction_event_record() {
        let (tracker, _temp) = temp_tracker();
        let event = tracker.record(
            FrictionType::Command,
            FrictionSeverity::Medium,
            "search",
            "test query",
            "",
            "timeout",
            Some(Resolution::Retried),
            10,
            2,
            false,
            "",
        );
        assert_eq!(event.friction_type, "command");
        assert_eq!(event.severity, "medium");
        assert!(!event.id.is_empty());
    }

    #[test]
    fn test_record_command_failure() {
        let (tracker, _temp) = temp_tracker();
        let event = tracker.record_command_failure("search", "test", "timeout", 3);
        assert_eq!(event.friction_type, "command");
        assert_eq!(event.severity, "high");
        assert_eq!(event.retry_count, 3);
    }

    #[test]
    fn test_record_workflow_abandon() {
        let (tracker, _temp) = temp_tracker();
        let event = tracker.record_workflow_abandon("analyze", "step2", "query", 60);
        assert_eq!(event.friction_type, "workflow");
        assert!(event.abandoned);
    }

    #[test]
    fn test_record_retrieval_failure() {
        let (tracker, _temp) = temp_tracker();
        let event = tracker.record_retrieval_failure("search", "query", "no results");
        assert_eq!(event.friction_type, "retrieval");
        assert_eq!(event.notes, "no results");
    }

    #[test]
    fn test_get_events_empty() {
        let (tracker, _temp) = temp_tracker();
        let events = tracker.get_events(None, 30, 100);
        assert!(events.is_empty());
    }

    #[test]
    fn test_get_events_with_data() {
        let (tracker, _temp) = temp_tracker();
        tracker.record(
            FrictionType::Command,
            FrictionSeverity::Medium,
            "search",
            "",
            "",
            "",
            None,
            0,
            0,
            false,
            "",
        );
        let events = tracker.get_events(None, 30, 100);
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_get_events_filter_type() {
        let (tracker, _temp) = temp_tracker();
        tracker.record(FrictionType::Command, FrictionSeverity::Medium, "c1", "", "", "", None, 0, 0, false, "");
        tracker.record(FrictionType::Retrieval, FrictionSeverity::Medium, "c2", "", "", "", None, 0, 0, false, "");
        let events = tracker.get_events(Some(FrictionType::Command), 30, 100);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].friction_type, "command");
    }

    #[test]
    fn test_get_summary_empty() {
        let (tracker, _temp) = temp_tracker();
        let summary = tracker.get_summary(30);
        assert_eq!(summary["total_events"], serde_json::json!(0));
    }

    #[test]
    fn test_get_summary_with_data() {
        let (tracker, _temp) = temp_tracker();
        tracker.record(FrictionType::Command, FrictionSeverity::Medium, "search", "", "", "", None, 0, 0, false, "");
        tracker.record(FrictionType::Retrieval, FrictionSeverity::High, "search", "", "", "", None, 0, 0, true, "");
        let summary = tracker.get_summary(30);
        assert_eq!(summary["total_events"], serde_json::json!(2));
    }
}
