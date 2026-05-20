//! rairos-progress-tracker — Progress Tracker for AI Research OS.
//!
//! Ported from `core/progress_tracker.py` (42 LOC, pure stdlib + chrono).

use chrono::Local;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Task Record ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub description: String,
    pub status: String,
    pub created: String,
    #[serde(default)]
    pub completed: Option<String>,
}

// ─── Progress Tracker ─────────────────────────────────────────────────────────

pub struct ProgressTracker {
    tasks: HashMap<String, TaskRecord>,
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressTracker {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
        }
    }

    pub fn add_task(&mut self, task_id: &str, description: &str) {
        self.tasks.insert(
            task_id.to_string(),
            TaskRecord {
                description: description.to_string(),
                status: "pending".to_string(),
                created: Local::now().to_rfc3339(),
                completed: None,
            },
        );
    }

    pub fn complete_task(&mut self, task_id: &str) {
        if let Some(task) = self.tasks.get_mut(task_id) {
            task.status = "completed".to_string();
            task.completed = Some(Local::now().to_rfc3339());
        }
    }

    pub fn get_progress(&self) -> f64 {
        let total = self.tasks.len();
        if total == 0 {
            return 0.0;
        }
        let completed = self.tasks.values().filter(|t| t.status == "completed").count();
        (completed as f64 / total as f64) * 100.0
    }

    pub fn get_task(&self, task_id: &str) -> Option<&TaskRecord> {
        self.tasks.get(task_id)
    }

    pub fn all_tasks(&self) -> &HashMap<String, TaskRecord> {
        &self.tasks
    }
}

// ─── Singleton ─────────────────────────────────────────────────────────────────

static TRACKER: std::sync::LazyLock<Mutex<ProgressTracker>> =
    std::sync::LazyLock::new(|| Mutex::new(ProgressTracker::new()));

pub fn get_tracker() -> parking_lot::MutexGuard<'static, ProgressTracker> {
    TRACKER.lock()
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_task() {
        let mut tracker = ProgressTracker::new();
        tracker.add_task("task1", "Test task");
        assert!(tracker.get_task("task1").is_some());
        assert_eq!(tracker.get_task("task1").unwrap().status, "pending");
    }

    #[test]
    fn test_complete_task() {
        let mut tracker = ProgressTracker::new();
        tracker.add_task("task1", "Test task");
        tracker.complete_task("task1");
        assert_eq!(tracker.get_task("task1").unwrap().status, "completed");
        assert!(tracker.get_task("task1").unwrap().completed.is_some());
    }

    #[test]
    fn test_get_progress_empty() {
        let tracker = ProgressTracker::new();
        assert_eq!(tracker.get_progress(), 0.0);
    }

    #[test]
    fn test_get_progress_partial() {
        let mut tracker = ProgressTracker::new();
        tracker.add_task("t1", "Task 1");
        tracker.add_task("t2", "Task 2");
        tracker.complete_task("t1");
        assert!((tracker.get_progress() - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_get_progress_full() {
        let mut tracker = ProgressTracker::new();
        tracker.add_task("t1", "Task 1");
        tracker.complete_task("t1");
        assert!((tracker.get_progress() - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_complete_nonexistent() {
        let mut tracker = ProgressTracker::new();
        // Should not panic
        tracker.complete_task("nonexistent");
        assert_eq!(tracker.get_progress(), 0.0);
    }
}
