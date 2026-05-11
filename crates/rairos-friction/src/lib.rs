//! rairos-friction — Research Friction Tracker
//!
//! Detects and records research efficiency bottlenecks. Friction = any event
//! that slows down or interrupts the user's research flow.
//!
//! Ported from `llm/friction_tracker.py`.

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
}

impl std::str::FromStr for FrictionType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "command" => Ok(FrictionType::Command),
            "workflow" => Ok(FrictionType::Workflow),
            "retrieval" => Ok(FrictionType::Retrieval),
            "cognitive" => Ok(FrictionType::Cognitive),
            "navigation" => Ok(FrictionType::Navigation),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
}

impl std::str::FromStr for FrictionSeverity {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "low" => Ok(FrictionSeverity::Low),
            "medium" => Ok(FrictionSeverity::Medium),
            "high" => Ok(FrictionSeverity::High),
            "critical" => Ok(FrictionSeverity::Critical),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
}

impl std::str::FromStr for Resolution {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "retried" => Ok(Resolution::Retried),
            "abandoned" => Ok(Resolution::Abandoned),
            "worked_around" => Ok(Resolution::WorkedAround),
            "self_resolved" => Ok(Resolution::SelfResolved),
            "system_helped" => Ok(Resolution::SystemHelped),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrictionEvent {
    pub id: String,
    pub timestamp: String,
    pub friction_type: String,
    pub severity: String,
    pub command: String,
    pub query: String,
    pub step: String,
    pub error: String,
    pub resolution: String,
    pub duration_seconds: i32,
    pub retry_count: i32,
    pub abandoned: bool,
    pub notes: String,
}

impl FrictionEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
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
        let id = format!("fr_{}", rand_hex(8));
        let timestamp = iso_now();
        Self {
            id,
            timestamp,
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

fn rand_hex(len: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| {
            let b: u8 = rng.gen_range(0..16);
            match b {
                0..=9 => (b'0' + b) as char,
                10..=15 => (b'a' + b - 10) as char,
                _ => '0',
            }
        })
        .collect()
}

fn iso_now() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let millis = now.subsec_millis();
    let days_since_epoch = secs / 86400;
    let secs_of_day = secs % 86400;
    let hours = secs_of_day / 3600;
    let minutes = (secs_of_day % 3600) / 60;
    let seconds = secs_of_day % 60;
    let julian_day = 2440588 + days_since_epoch as i64;
    let (year, month, day) = julian_to_ymd(julian_day);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        year, month, day, hours, minutes, seconds, millis
    )
}

fn julian_to_ymd(jd: i64) -> (i64, u32, u32) {
    let j = jd + 32044;
    let g = j / 146097;
    let dg = j % 146097;
    let c = (dg / 36524 + 1) * 3 / 4;
    let dc = dg - c * 36524;
    let b = dc / 1461;
    let db = dc % 1461;
    let a = (db / 365 + 1) * 3 / 4;
    let da = db - a * 365;
    let y = g * 400 + c * 100 + b * 4 + a;
    let m = (da * 5 + 308) / 153 - 2;
    let d = da - (m + 4) * 153 / 5 + 122;
    let year = y - 4800 + (m + 2) / 12;
    let month = ((m + 2) % 12 + 1) as u32;
    let day = d as u32 + 1;
    (year, month, day)
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
        let events_file = data_dir.join("friction_events.jsonl");
        if !data_dir.exists() {
            let _ = fs::create_dir_all(&data_dir);
        }
        if !events_file.exists() {
            let _ = fs::write(&events_file, "");
        }
        Self {
            data_dir,
            events_file,
        }
    }

    pub fn data_dir_path(&self) -> &PathBuf {
        &self.data_dir
    }

    pub fn events_file_path(&self) -> &PathBuf {
        &self.events_file
    }

    #[allow(clippy::too_many_arguments)]
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
        let event = FrictionEvent::new(
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
        let line = serde_json::to_string(&event).unwrap_or_default();
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&self.events_file) {
            let _ = writeln!(f, "{}", line);
        }
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
        let cutoff = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
            - (since_days as i64 * 86400);

        let file = match fs::File::open(&self.events_file) {
            Ok(f) => f,
            Err(_) => return Vec::new(),
        };
        let reader = BufReader::new(file);
        let all_lines: Vec<String> = reader.lines().map_while(Result::ok).collect();
        let mut events = Vec::new();
        for line in all_lines.into_iter().rev() {
            if limit > 0 && events.len() >= limit {
                break;
            }
            if line.trim().is_empty() {
                continue;
            }
            let event: FrictionEvent = match serde_json::from_str(&line) {
                Ok(e) => e,
                Err(_) => continue,
            };
            if let Ok(ts) = parse_iso_timestamp(&event.timestamp) {
                if ts < cutoff {
                    break;
                }
            }
            if let Some(ft) = friction_type {
                if event.friction_type.parse::<FrictionType>().ok() != Some(ft) {
                    continue;
                }
            }
            events.push(event);
        }
        events
    }

    pub fn get_summary(&self, since_days: i32) -> FrictionSummary {
        let events = self.get_events(None, since_days, 1000);
        if events.is_empty() {
            return FrictionSummary {
                total_events: 0,
                by_type: Default::default(),
                by_severity: Default::default(),
                top_commands: Vec::new(),
                abandon_rate: 0.0,
            };
        }

        let mut by_type: HashMap<String, i32> = Default::default();
        let mut by_severity: HashMap<String, i32> = Default::default();
        let mut command_counts: HashMap<String, i32> = Default::default();
        let mut abandoned = 0;

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

        let mut top_commands: Vec<(String, i32)> = command_counts.into_iter().collect();
        top_commands.sort_by_key(|b| std::cmp::Reverse(b.1));
        top_commands.truncate(5);

        FrictionSummary {
            total_events: events.len() as i32,
            by_type,
            by_severity,
            top_commands,
            abandon_rate: abandoned as f64 / events.len() as f64,
        }
    }
}

fn parse_iso_timestamp(s: &str) -> Result<i64, std::num::ParseIntError> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() >= 14 {
        let year: i64 = digits[0..4].parse()?;
        let month: i64 = digits[4..6].parse()?;
        let day: i64 = digits[6..8].parse()?;
        let hour: i64 = digits[8..10].parse()?;
        let minute: i64 = digits[10..12].parse()?;
        let second: i64 = digits[12..14].parse()?;
        let days = (year - 1970) * 365 + (month - 1) * 30 + day;
        let secs = hour * 3600 + minute * 60 + second;
        Ok(days * 86400 + secs)
    } else if digits.len() >= 8 {
        let year: i64 = digits[0..4].parse()?;
        let month: i64 = digits[4..6].parse()?;
        let day: i64 = digits[6..8].parse()?;
        let days = (year - 1970) * 365 + (month - 1) * 30 + day;
        Ok(days * 86400)
    } else {
        Ok(0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrictionSummary {
    pub total_events: i32,
    pub by_type: HashMap<String, i32>,
    pub by_severity: HashMap<String, i32>,
    pub top_commands: Vec<(String, i32)>,
    pub abandon_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_basic() {
        let tmp = tempfile::TempDir::new().unwrap();
        let tracker = FrictionTracker::new(Some(PathBuf::from(tmp.path())));
        let event = tracker.record(
            FrictionType::Command,
            FrictionSeverity::Medium,
            "paper-search",
            "transformer attention",
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
        assert_eq!(event.command, "paper-search");
        assert_eq!(event.retry_count, 2);
        let _ = tmp;
    }

    #[test]
    fn test_record_command_failure_high_retry() {
        let tmp = tempfile::TempDir::new().unwrap();
        let tracker = FrictionTracker::new(Some(PathBuf::from(tmp.path())));
        let event = tracker.record_command_failure("search", "query", "error", 3);
        assert_eq!(event.severity, "high");
        assert_eq!(event.resolution, "retried");
        let _ = tmp;
    }

    #[test]
    fn test_record_command_failure_low_retry() {
        let tmp = tempfile::TempDir::new().unwrap();
        let tracker = FrictionTracker::new(Some(PathBuf::from(tmp.path())));
        let event = tracker.record_command_failure("search", "query", "error", 1);
        assert_eq!(event.severity, "medium");
        assert_eq!(event.resolution, "retried");
        let _ = tmp;
    }

    #[test]
    fn test_record_workflow_abandon() {
        let tmp = tempfile::TempDir::new().unwrap();
        let tracker = FrictionTracker::new(Some(PathBuf::from(tmp.path())));
        let event = tracker.record_workflow_abandon("deep-research", "step2", "query", 300);
        assert_eq!(event.friction_type, "workflow");
        assert_eq!(event.severity, "medium");
        assert!(event.abandoned);
        assert_eq!(event.resolution, "abandoned");
        let _ = tmp;
    }

    #[test]
    fn test_get_events_empty() {
        let tmp = tempfile::TempDir::new().unwrap();
        let tracker = FrictionTracker::new(Some(PathBuf::from(tmp.path())));
        let events = tracker.get_events(None, 30, 100);
        assert!(events.is_empty());
        let _ = tmp;
    }

    #[test]
    fn test_get_events_filtered_by_type() {
        let tmp = tempfile::TempDir::new().unwrap();
        let tracker = FrictionTracker::new(Some(PathBuf::from(tmp.path())));
        tracker.record(FrictionType::Command, FrictionSeverity::Low, "c1", "", "", "", None, 0, 0, false, "");
        tracker.record(FrictionType::Workflow, FrictionSeverity::Low, "c2", "", "", "", None, 0, 0, false, "");
        tracker.record(FrictionType::Command, FrictionSeverity::Low, "c3", "", "", "", None, 0, 0, false, "");
        let cmd_events = tracker.get_events(Some(FrictionType::Command), 30, 100);
        assert_eq!(cmd_events.len(), 2);
        let _ = tmp;
    }

    #[test]
    fn test_get_summary_empty() {
        let tmp = tempfile::TempDir::new().unwrap();
        let tracker = FrictionTracker::new(Some(PathBuf::from(tmp.path())));
        let summary = tracker.get_summary(30);
        assert_eq!(summary.total_events, 0);
        assert!(summary.top_commands.is_empty());
        let _ = tmp;
    }

    #[test]
    fn test_get_summary_basic() {
        let tmp = tempfile::TempDir::new().unwrap();
        let tracker = FrictionTracker::new(Some(PathBuf::from(tmp.path())));
        tracker.record(FrictionType::Command, FrictionSeverity::High, "search", "", "", "", None, 0, 0, false, "");
        tracker.record(FrictionType::Command, FrictionSeverity::Low, "search", "", "", "", None, 0, 0, false, "");
        tracker.record(FrictionType::Retrieval, FrictionSeverity::Medium, "find", "", "", "", Some(Resolution::WorkedAround), 0, 0, false, "");
        let summary = tracker.get_summary(30);
        assert_eq!(summary.total_events, 3);
        assert_eq!(summary.by_type.get("command"), Some(&2));
        assert_eq!(summary.by_type.get("retrieval"), Some(&1));
        assert_eq!(summary.by_severity.get("high"), Some(&1));
        let _ = tmp;
    }

    #[test]
    fn test_friction_type_as_str() {
        assert_eq!(FrictionType::Command.as_str(), "command");
        assert_eq!(FrictionType::Workflow.as_str(), "workflow");
    }

    #[test]
    fn test_friction_severity_as_str() {
        assert_eq!(FrictionSeverity::Low.as_str(), "low");
        assert_eq!(FrictionSeverity::Critical.as_str(), "critical");
    }

    #[test]
    fn test_resolution_as_str() {
        assert_eq!(Resolution::Abandoned.as_str(), "abandoned");
        assert_eq!(Resolution::SystemHelped.as_str(), "system_helped");
    }
}
