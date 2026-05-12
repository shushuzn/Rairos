//! rairos-progress-tracker — Progress tracking for research tasks.
//!
//! Ported from `core/progress_tracker.py`.

use chrono::Utc;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

/// A task in the progress tracker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub description: String,
    pub status: String,
    pub created: String,
    pub completed: Option<String>,
}

/// Progress Tracker for research tasks.
pub struct ProgressTracker {
    tasks: HashMap<String, Task>,
}

impl ProgressTracker {
    /// Create a new progress tracker.
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
        }
    }

    /// Add a task to track.
    pub fn add_task(&mut self, task_id: &str, description: &str) {
        self.tasks.insert(
            task_id.to_string(),
            Task {
                description: description.to_string(),
                status: "pending".to_string(),
                created: Utc::now().to_rfc3339(),
                completed: None,
            },
        );
    }

    /// Mark a task as completed.
    pub fn complete_task(&mut self, task_id: &str) {
        if let Some(task) = self.tasks.get_mut(task_id) {
            task.status = "completed".to_string();
            task.completed = Some(Utc::now().to_rfc3339());
        }
    }

    /// Get progress as a percentage (0.0 to 100.0).
    pub fn get_progress(&self) -> f64 {
        let total = self.tasks.len();
        if total == 0 {
            return 0.0;
        }
        let completed = self
            .tasks
            .values()
            .filter(|t| t.status == "completed")
            .count();
        (completed as f64 / total as f64) * 100.0
    }

    /// Get task by ID.
    pub fn get_task(&self, task_id: &str) -> Option<&Task> {
        self.tasks.get(task_id)
    }

    /// Get all tasks.
    pub fn get_all_tasks(&self) -> &HashMap<String, Task> {
        &self.tasks
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

// Global tracker instance
static TRACKER: Lazy<Mutex<Option<ProgressTracker>>> = Lazy::new(|| Mutex::new(None));

/// Get the global progress tracker instance.
pub fn get_tracker() -> MutexGuard<'static, Option<ProgressTracker>> {
    TRACKER.lock().unwrap()
}

/// Initialize the global tracker.
pub fn init_tracker() {
    let mut guard = TRACKER.lock().unwrap();
    *guard = Some(ProgressTracker::new());
}

/// With the global tracker, execute a closure.
pub fn with_tracker<F, R>(f: F) -> R
where
    F: FnOnce(&mut ProgressTracker) -> R,
{
    let mut guard = TRACKER.lock().unwrap();
    if guard.is_none() {
        *guard = Some(ProgressTracker::new());
    }
    if let Some(ref mut tracker) = *guard {
        f(tracker)
    } else {
        panic!("Tracker not initialized")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_task() {
        let mut tracker = ProgressTracker::new();
        tracker.add_task("task1", "Test task");
        assert_eq!(tracker.tasks.len(), 1);
        assert_eq!(tracker.get_task("task1").unwrap().description, "Test task");
    }

    #[test]
    fn test_complete_task() {
        let mut tracker = ProgressTracker::new();
        tracker.add_task("task1", "Test task");
        tracker.complete_task("task1");
        let task = tracker.get_task("task1").unwrap();
        assert_eq!(task.status, "completed");
        assert!(task.completed.is_some());
    }

    #[test]
    fn test_get_progress_empty() {
        let tracker = ProgressTracker::new();
        assert_eq!(tracker.get_progress(), 0.0);
    }

    #[test]
    fn test_get_progress_partial() {
        let mut tracker = ProgressTracker::new();
        tracker.add_task("task1", "Task 1");
        tracker.add_task("task2", "Task 2");
        tracker.complete_task("task1");
        let progress = tracker.get_progress();
        assert!((progress - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_get_progress_complete() {
        let mut tracker = ProgressTracker::new();
        tracker.add_task("task1", "Task 1");
        tracker.add_task("task2", "Task 2");
        tracker.complete_task("task1");
        tracker.complete_task("task2");
        assert_eq!(tracker.get_progress(), 100.0);
    }
}
