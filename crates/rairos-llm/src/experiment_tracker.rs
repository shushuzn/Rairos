//! Experiment Tracker — simple in-memory experiment logging and analysis.
//!
//! Mirrors llm/experiment_tracker.py

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experiment {
    pub id: String,
    pub name: String,
    pub status: ExperimentStatus,
    pub metrics: HashMap<String, f64>,
    pub tags: Vec<String>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExperimentStatus {
    Planned,
    Running,
    Completed,
    Failed,
}

impl ExperimentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentTracker {
    experiments: Vec<Experiment>,
}

impl ExperimentTracker {
    pub fn new() -> Self {
        Self { experiments: Vec::new() }
    }

    pub fn add(&mut self, name: &str, tags: Vec<String>) -> String {
        let id = format!("exp-{}", self.experiments.len() + 1);
        self.experiments.push(Experiment {
            id: id.clone(),
            name: name.to_string(),
            status: ExperimentStatus::Planned,
            metrics: HashMap::new(),
            tags,
            notes: String::new(),
        });
        id
    }

    pub fn update_status(&mut self, id: &str, status: ExperimentStatus) -> bool {
        if let Some(exp) = self.experiments.iter_mut().find(|e| e.id == id) {
            exp.status = status;
            true
        } else {
            false
        }
    }

    pub fn record_metric(&mut self, id: &str, key: &str, value: f64) -> bool {
        if let Some(exp) = self.experiments.iter_mut().find(|e| e.id == id) {
            exp.metrics.insert(key.to_string(), value);
            true
        } else {
            false
        }
    }

    pub fn add_note(&mut self, id: &str, note: &str) -> bool {
        if let Some(exp) = self.experiments.iter_mut().find(|e| e.id == id) {
            exp.notes.push_str(note);
            exp.notes.push('\n');
            true
        } else {
            false
        }
    }

    pub fn list(&self, status_filter: Option<ExperimentStatus>) -> Vec<&Experiment> {
        self.experiments.iter()
            .filter(|e| status_filter.as_ref().map_or(true, |s| e.status == *s))
            .collect()
    }

    pub fn summary(&self) -> HashMap<&'static str, usize> {
        let mut map = HashMap::new();
        for s in &[ExperimentStatus::Planned, ExperimentStatus::Running, ExperimentStatus::Completed, ExperimentStatus::Failed] {
            let count = self.experiments.iter().filter(|e| e.status == *s).count();
            map.insert(s.as_str(), count);
        }
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_list() {
        let mut tracker = ExperimentTracker::new();
        let id = tracker.add("Test A", vec![]);
        assert_eq!(tracker.list(None).len(), 1);
        assert_eq!(tracker.list(Some(ExperimentStatus::Planned)).len(), 1);
        assert_eq!(tracker.list(Some(ExperimentStatus::Completed)).len(), 0);
        assert!(tracker.update_status(&id, ExperimentStatus::Completed));
        assert_eq!(tracker.list(Some(ExperimentStatus::Completed)).len(), 1);
    }

    #[test]
    fn test_metrics_and_notes() {
        let mut tracker = ExperimentTracker::new();
        let id = tracker.add("Test B", vec!["vision".to_string()]);
        assert!(tracker.record_metric(&id, "accuracy", 0.95));
        assert!(tracker.add_note(&id, "Good initial results."));
        let exps = tracker.list(None);
        assert!((exps[0].metrics.get("accuracy").unwrap() - 0.95).abs() < 1e-6);
    }

    #[test]
    fn test_summary() {
        let mut tracker = ExperimentTracker::new();
        tracker.add("E1", vec![]);
        tracker.add("E2", vec![]);
        let summary = tracker.summary();
        assert_eq!(*summary.get("planned").unwrap(), 2);
    }
}
