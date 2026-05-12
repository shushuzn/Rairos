//! rairos-friction-tracker — Research Friction Tracker for AI Research OS.
//!
//! Ported from `llm/friction_tracker.py` (248 LOC, pure stdlib).

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

// ─── Enums ─────────────────────────────────────────────────────────────────────

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

// ─── FrictionEvent ─────────────────────────────────────────────────────────────

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
    pub duration_seconds: i64,
    #[serde(default)]
    pub retry_count: i32,
    #[serde(default)]
    pub abandoned: bool,
    #[serde(default)]
    pub notes: String,
}

impl FrictionEvent {
    pub fn new(friction_type: &str, severity: &str) -> Self {
        let now = Local::now().to_rfc3339();
        Self {
            id: format!("fr_{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
            timestamp: now,
            friction_type: friction_type.to_string(),
            severity: severity.to_string(),
            command: String::new(),
            query: String::new(),
            step: String::new(),
            error: String::new(),
            resolution: String::new(),
            duration_seconds: 0,
            retry_count: 0,
            abandoned: false,
            notes: String::new(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn from_json(line: &str) -> Option<Self> {
        serde_json::from_str(line).ok()
    }
}

// ─── FrictionTracker ───────────────────────────────────────────────────────────

pub struct FrictionTracker {
    data_dir: PathBuf,
    events_file: PathBuf,
}

impl Default for FrictionTracker {
    fn default() -> Self {
        Self::new(None)
    }
}

impl FrictionTracker {
    pub fn new(data_dir: Option<&str>) -> Self {
        let data_dir = match data_dir {
            Some(d) => PathBuf::from(d),
            None => dirs::home_dir().unwrap_or_default().join(".ai_research_os/friction"),
        };
        let events_file = data_dir.join("friction_events.jsonl");
        // Ensure directory exists
        let _ = std::fs::create_dir_all(&data_dir);
        // Ensure file exists
        if !events_file.exists() {
            let _ = std::fs::write(&events_file, "");
        }
        Self {
            data_dir,
            events_file,
        }
    }

    pub fn record(&self, event: &FrictionEvent) {
        if let Ok(mut fh) = OpenOptions::new().append(true).open(&self.events_file) {
            let line = event.to_json();
            let _ = writeln!(fh, "{}", line);
        }
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
            Resolution::Retried
        } else {
            Resolution::SelfResolved
        };
        let mut event = FrictionEvent::new(FrictionType::Command.as_str(), severity.as_str());
        event.command = command.to_string();
        event.query = query.to_string();
        event.error = error.to_string();
        event.retry_count = retry_count;
        event.resolution = resolution.as_str().to_string();
        self.record(&event);
        event
    }

    pub fn record_workflow_abandon(
        &self,
        command: &str,
        step: &str,
        query: &str,
        duration_seconds: i64,
    ) -> FrictionEvent {
        let mut event = FrictionEvent::new(FrictionType::Workflow.as_str(), FrictionSeverity::Medium.as_str());
        event.command = command.to_string();
        event.step = step.to_string();
        event.query = query.to_string();
        event.duration_seconds = duration_seconds;
        event.abandoned = true;
        event.resolution = Resolution::Abandoned.as_str().to_string();
        self.record(&event);
        event
    }

    pub fn record_retrieval_failure(
        &self,
        command: &str,
        query: &str,
        notes: &str,
    ) -> FrictionEvent {
        let mut event = FrictionEvent::new(FrictionType::Retrieval.as_str(), FrictionSeverity::Medium.as_str());
        event.command = command.to_string();
        event.query = query.to_string();
        event.notes = notes.to_string();
        event.resolution = Resolution::WorkedAround.as_str().to_string();
        self.record(&event);
        event
    }

    pub fn get_events(&self, since_days: i64, limit: usize) -> Vec<FrictionEvent> {
        let cutoff = Local::now().timestamp() - (since_days * 86400);
        let content = match std::fs::read_to_string(&self.events_file) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        let mut events = Vec::new();
        for line in content.lines().rev() {
            if limit > 0 && events.len() >= limit {
                break;
            }
            let event = match FrictionEvent::from_json(line.trim()) {
                Some(e) => e,
                None => continue,
            };
            if let Ok(ts) = DateTime::parse_from_rfc3339(&event.timestamp) {
                if ts.timestamp() < cutoff {
                    break;
                }
            }
            events.push(event);
        }
        events
    }

    pub fn get_summary(&self, since_days: i64) -> serde_json::Value {
        let events = self.get_events(since_days, 1000);
        if events.is_empty() {
            return serde_json::json!({
                "total_events": 0,
                "by_type": {},
                "by_severity": {},
                "top_commands": [],
                "abandon_rate": 0.0f64,
            });
        }

        let mut by_type: HashMap<String, i64> = HashMap::new();
        let mut by_severity: HashMap<String, i64> = HashMap::new();
        let mut command_counts: HashMap<String, i64> = HashMap::new();
        let mut abandoned: i64 = 0;

        for e in &events {
            *by_type.entry(e.friction_type.clone()).or_insert(0) += 1;
            *by_severity.entry(e.severity.clone()).or_insert(0) += 1;
            if !e.command.is_empty() {
                *command_counts.entry(e.command.clone()).or_insert(0) += 1;
            }
            if e.abandoned {
                abandoned += 1;
            }
        }

        let mut top_commands: Vec<_> = command_counts.into_iter().collect();
        top_commands.sort_by(|a, b| b.1.cmp(&a.1));
        let top_commands: Vec<_> = top_commands.into_iter().take(5).collect();

        serde_json::json!({
            "total_events": events.len(),
            "by_type": by_type,
            "by_severity": by_severity,
            "top_commands": top_commands,
            "abandon_rate": abandoned as f64 / events.len() as f64,
        })
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_tracker() -> FrictionTracker {
        let tmp = std::env::temp_dir().join(format!("friction_test_{}", uuid::Uuid::new_v4()));
        FrictionTracker::new(Some(tmp.to_str().unwrap()))
    }

    #[test]
    fn test_friction_type_as_str() {
        assert_eq!(FrictionType::Command.as_str(), "command");
        assert_eq!(FrictionSeverity::High.as_str(), "high");
        assert_eq!(Resolution::Abandoned.as_str(), "abandoned");
    }

    #[test]
    fn test_record_command_failure() {
        let tracker = temp_tracker();
        let event = tracker.record_command_failure("llm search", "query", "timeout", 2);
        assert_eq!(event.friction_type, "command");
        assert_eq!(event.severity, "medium");
        assert_eq!(event.retry_count, 2);
        assert_eq!(event.resolution, "retried");
    }

    #[test]
    fn test_get_events_empty() {
        let tracker = temp_tracker();
        let events = tracker.get_events(30, 100);
        assert!(events.is_empty());
    }

    #[test]
    fn test_get_summary_empty() {
        let tracker = temp_tracker();
        let summary = tracker.get_summary(30);
        assert_eq!(summary["total_events"], 0);
    }

    #[test]
    fn test_friction_event_json_roundtrip() {
        let event = FrictionEvent::new("command", "high");
        let json = event.to_json();
        let parsed = FrictionEvent::from_json(&json).unwrap();
        assert_eq!(parsed.friction_type, "command");
        assert_eq!(parsed.severity, "high");
    }
}
