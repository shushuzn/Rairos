//! Task Tracker Module for multi-agent workflow state management.
//!
//! Based on research from:
//! - Aime (arXiv:2507.11988) - Progress Management Module as single source of truth
//! - Magentic-One (arXiv:2411.04468) - Orchestrator tracks progress and re-plans
//! - AgentOrchestra - Central planning agent with subtask tracking
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │              TaskTracker (Single Source of Truth)    │
//! │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  │
//! │  │ Task A  │  │ Task B  │  │ Task C  │  │ Task D  │  │
//! │  │███████░░│  │█████████│  │░░░░░░░░░│  │████░░░░░│  │
//! │  │pending  │  │done     │  │blocked  │  │running  │  │
//! │  └─────────┘  └─────────┘  └─────────┘  └─────────┘  │
//! └─────────────────────────────────────────────────────┘
//! ```

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::utils::current_timestamp;

/// Task execution state
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskState {
    /// Task is pending, not yet started
    Pending,
    /// Task is currently being executed
    InProgress,
    /// Task is waiting for dependencies to complete
    WaitingForDeps,
    /// Task completed successfully
    Completed,
    /// Task failed with an error
    Failed,
    /// Task was cancelled
    Cancelled,
    /// Task output needs verification
    NeedsVerification,
}

impl TaskState {
    /// Check if task is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TaskState::Completed | TaskState::Failed | TaskState::Cancelled
        )
    }

    /// Check if task can transition to another state
    pub fn can_transition_to(&self, next: TaskState) -> bool {
        match self {
            TaskState::Pending => matches!(
                next,
                TaskState::InProgress | TaskState::WaitingForDeps | TaskState::Cancelled
            ),
            TaskState::InProgress => matches!(
                next,
                TaskState::Completed
                    | TaskState::Failed
                    | TaskState::Cancelled
                    | TaskState::NeedsVerification
            ),
            TaskState::WaitingForDeps => {
                matches!(next, TaskState::InProgress | TaskState::Cancelled)
            }
            TaskState::NeedsVerification => {
                matches!(next, TaskState::Completed | TaskState::Failed | TaskState::InProgress)
            }
            TaskState::Completed | TaskState::Failed | TaskState::Cancelled => false,
        }
    }
}

/// Priority level for task ordering
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrackPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// A tracked task in the workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedTask {
    /// Unique task ID
    pub id: String,
    /// Human-readable description
    pub description: String,
    /// Current state
    pub state: TaskState,
    /// Priority level
    pub priority: TrackPriority,
    /// Task dependencies (IDs of tasks that must complete first)
    pub dependencies: Vec<String>,
    /// Subtask IDs (for composite tasks)
    pub subtasks: Vec<String>,
    /// Parent task ID (if this is a subtask)
    pub parent_id: Option<String>,
    /// Progress percentage (0-100)
    pub progress: u8,
    /// Number of retry attempts
    pub retry_count: u8,
    /// Maximum retries allowed
    pub max_retries: u8,
    /// Error message if failed
    pub error: Option<String>,
    /// Created at timestamp
    pub created_at: u64,
    /// Started at timestamp
    pub started_at: Option<u64>,
    /// Completed at timestamp
    pub completed_at: Option<u64>,
    /// Worker/agent ID assigned to this task
    pub assigned_to: Option<String>,
    /// Metadata (arbitrary key-value pairs)
    pub metadata: HashMap<String, String>,
}

impl TrackedTask {
    /// Create a new tracked task
    pub fn new(id: String, description: String) -> Self {
        Self {
            id,
            description,
            state: TaskState::Pending,
            priority: TrackPriority::Normal,
            dependencies: Vec::new(),
            subtasks: Vec::new(),
            parent_id: None,
            progress: 0,
            retry_count: 0,
            max_retries: 3,
            error: None,
            created_at: current_timestamp(),
            started_at: None,
            completed_at: None,
            assigned_to: None,
            metadata: HashMap::new(),
        }
    }

    /// Add a dependency
    pub fn add_dependency(&mut self, dep_id: String) {
        if !self.dependencies.contains(&dep_id) {
            self.dependencies.push(dep_id);
        }
    }

    /// Add a subtask
    pub fn add_subtask(&mut self, subtask_id: String) {
        if !self.subtasks.contains(&subtask_id) {
            self.subtasks.push(subtask_id);
        }
    }

    /// Mark task as in progress
    pub fn start(&mut self, worker_id: Option<String>) {
        if self.state == TaskState::Pending || self.state == TaskState::WaitingForDeps {
            self.state = TaskState::InProgress;
            self.started_at = Some(current_timestamp());
            self.assigned_to = worker_id;
        }
    }

    /// Mark task as completed
    pub fn complete(&mut self) {
        self.state = TaskState::Completed;
        self.progress = 100;
        self.completed_at = Some(current_timestamp());
    }

    /// Mark task as failed
    pub fn fail(&mut self, error: Option<String>) {
        self.state = TaskState::Failed;
        self.error = error;
        self.completed_at = Some(current_timestamp());
    }

    /// Check if all dependencies are satisfied
    pub fn dependencies_satisfied(&self, task_states: &HashMap<String, TaskState>) -> bool {
        self.dependencies.iter().all(|dep_id| {
            task_states
                .get(dep_id)
                .map(|s| s.is_terminal() && s == &TaskState::Completed)
                .unwrap_or(false)
        })
    }
}

/// Summary of task tracker statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackerStats {
    pub total_tasks: usize,
    pub pending: usize,
    pub in_progress: usize,
    pub waiting_for_deps: usize,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
}

/// Event emitted by task tracker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackerEvent {
    pub event_type: TrackerEventType,
    pub task_id: String,
    pub timestamp: u64,
    pub details: Option<String>,
}

/// Types of tracker events
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrackerEventType {
    TaskCreated,
    TaskStarted,
    TaskProgress,
    TaskCompleted,
    TaskFailed,
    TaskCancelled,
    TaskRetried,
    TaskBlocked,
    TaskUnblocked,
}

/// Task Tracker - central state manager for multi-agent workflows
pub struct TaskTracker {
    /// All tracked tasks
    tasks: RwLock<HashMap<String, TrackedTask>>,
    /// Event history (circular buffer)
    event_history: RwLock<VecDeque<TrackerEvent>>,
    /// Maximum events to keep
    max_events: usize,
    /// Root task IDs (top-level tasks)
    root_tasks: RwLock<HashSet<String>>,
}

impl TaskTracker {
    /// Create a new task tracker
    pub fn new(max_events: usize) -> Self {
        Self {
            tasks: RwLock::new(HashMap::new()),
            event_history: RwLock::new(VecDeque::new()),
            max_events,
            root_tasks: RwLock::new(HashSet::new()),
        }
    }

    /// Create a new task and add it to the tracker
    pub async fn create_task(&self, id: String, description: String) -> TrackedTask {
        let task_id = id.clone();
        let task = TrackedTask::new(task_id.clone(), description);
        let is_root = task.dependencies.is_empty();

        let event = TrackerEvent {
            event_type: TrackerEventType::TaskCreated,
            task_id: task_id.clone(),
            timestamp: current_timestamp(),
            details: None,
        };

        {
            let mut tasks = self.tasks.write().await;
            tasks.insert(task_id.clone(), task);
        }

        if is_root {
            let mut roots = self.root_tasks.write().await;
            roots.insert(task_id.clone());
        }

        self.add_event(event).await;

        let tasks = self.tasks.read().await;
        tasks.get(&task_id).unwrap().clone()
    }

    /// Create a task with full configuration
    pub async fn create_task_with(
        &self,
        id: String,
        description: String,
        priority: TrackPriority,
        dependencies: Vec<String>,
        parent_id: Option<String>,
    ) -> TrackedTask {
        let task_id = id.clone();
        let mut task = TrackedTask::new(task_id.clone(), description);
        task.priority = priority;
        task.dependencies = dependencies.clone();
        task.parent_id = parent_id.clone();

        let event = TrackerEvent {
            event_type: TrackerEventType::TaskCreated,
            task_id: task_id.clone(),
            timestamp: current_timestamp(),
            details: None,
        };

        let is_root = dependencies.is_empty() && parent_id.is_none();

        {
            let mut tasks = self.tasks.write().await;
            // Update parent subtasks
            if let Some(pid) = &parent_id {
                if let Some(parent) = tasks.get_mut(pid) {
                    parent.add_subtask(task_id.clone());
                }
            }
            tasks.insert(task_id.clone(), task);
        }

        if is_root {
            let mut roots = self.root_tasks.write().await;
            roots.insert(task_id.clone());
        }

        self.add_event(event).await;

        let tasks = self.tasks.read().await;
        tasks.get(&task_id).unwrap().clone()
    }

    /// Get a task by ID
    pub async fn get_task(&self, id: &str) -> Option<TrackedTask> {
        let tasks = self.tasks.read().await;
        tasks.get(id).cloned()
    }

    /// Get all tasks
    pub async fn get_all_tasks(&self) -> Vec<TrackedTask> {
        let tasks = self.tasks.read().await;
        tasks.values().cloned().collect()
    }

    /// Get tasks by state
    pub async fn get_tasks_by_state(&self, state: TaskState) -> Vec<TrackedTask> {
        let tasks = self.tasks.read().await;
        tasks
            .values()
            .filter(|t| t.state == state)
            .cloned()
            .collect()
    }

    /// Get root tasks (tasks without parents)
    pub async fn get_root_tasks(&self) -> Vec<TrackedTask> {
        let tasks = self.tasks.read().await;
        let roots = self.root_tasks.read().await;
        roots
            .iter()
            .filter_map(|id| tasks.get(id).cloned())
            .collect()
    }

    /// Get pending tasks that can be executed (dependencies satisfied)
    pub async fn get_runnable_tasks(&self) -> Vec<TrackedTask> {
        let tasks = self.tasks.read().await;
        
        // Build state map and filter in single lock acquisition
        let task_states: HashMap<String, TaskState> = tasks
            .iter()
            .map(|(id, t)| (id.clone(), t.state))
            .collect();
        
        let mut runnable: Vec<TrackedTask> = tasks
            .values()
            .filter(|t| {
                t.state == TaskState::Pending
                    && t.dependencies_satisfied(&task_states)
            })
            .cloned()
            .collect();
        
        drop(task_states);
        drop(tasks);

        // Sort by priority
        runnable.sort_by(|a, b| b.priority.cmp(&a.priority));
        runnable
    }

    /// Get blocked tasks (pending but dependencies not satisfied)
    pub async fn get_blocked_tasks(&self) -> Vec<TrackedTask> {
        let tasks = self.tasks.read().await;
        let task_states: HashMap<String, TaskState> = tasks
            .iter()
            .map(|(id, t)| (id.clone(), t.state))
            .collect();
        drop(tasks);

        self.tasks
            .read()
            .await
            .values()
            .filter(|t| {
                t.state == TaskState::Pending
                    && !t.dependencies.is_empty()
                    && !t.dependencies_satisfied(&task_states)
            })
            .cloned()
            .collect()
    }

    /// Start a task
    pub async fn start_task(&self, id: &str, worker_id: Option<String>) -> bool {
        let task_id_str = id.to_string();
        let details = worker_id.as_ref().map(|w| format!("worker:{}", w));

        let transitioned = {
            let mut tasks = self.tasks.write().await;
            if let Some(task) = tasks.get_mut(id) {
                if task.state.can_transition_to(TaskState::InProgress) {
                    task.start(worker_id);
                    true
                } else {
                    false
                }
            } else {
                false
            }
        };

        if transitioned {
            let event = TrackerEvent {
                event_type: TrackerEventType::TaskStarted,
                task_id: task_id_str,
                timestamp: current_timestamp(),
                details,
            };
            self.add_event(event).await;
            true
        } else {
            false
        }
    }

    /// Update task progress
    pub async fn update_progress(&self, id: &str, progress: u8) -> bool {
        let task_id_str = id.to_string();
        let details = Some(format!("progress:{}", progress));

        let transitioned = {
            let mut tasks = self.tasks.write().await;
            if let Some(task) = tasks.get_mut(id) {
                if task.state == TaskState::InProgress {
                    task.progress = progress.min(100);
                    true
                } else {
                    false
                }
            } else {
                false
            }
        };

        if transitioned {
            let event = TrackerEvent {
                event_type: TrackerEventType::TaskProgress,
                task_id: task_id_str,
                timestamp: current_timestamp(),
                details,
            };
            self.add_event(event).await;
            true
        } else {
            false
        }
    }

    /// Complete a task
    pub async fn complete_task(&self, id: &str) -> bool {
        let task_id_str = id.to_string();
        let parent_id = {
            let mut tasks = self.tasks.write().await;
            if let Some(task) = tasks.get_mut(id) {
                if task.state.can_transition_to(TaskState::Completed) {
                    task.complete();
                    task.parent_id.clone()
                } else {
                    return false;
                }
            } else {
                return false;
            }
        };

        // Update parent progress
        if let Some(pid) = parent_id {
            self.update_parent_progress(&pid).await;
        }

        let event = TrackerEvent {
            event_type: TrackerEventType::TaskCompleted,
            task_id: task_id_str,
            timestamp: current_timestamp(),
            details: None,
        };
        self.add_event(event).await;

        // Unblock dependent tasks
        self.unblock_dependents(id).await;
        true
    }

    /// Fail a task
    pub async fn fail_task(&self, id: &str, error: Option<String>) -> bool {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(id) {
            if task.state.can_transition_to(TaskState::Failed) {
                task.fail(error.clone());

                let event = TrackerEvent {
                    event_type: TrackerEventType::TaskFailed,
                    task_id: id.to_string(),
                    timestamp: current_timestamp(),
                    details: error,
                };
                drop(tasks);
                self.add_event(event).await;
                return true;
            }
        }
        false
    }

    /// Retry a failed task
    pub async fn retry_task(&self, id: &str) -> bool {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(id) {
            if task.state == TaskState::Failed && task.retry_count < task.max_retries {
                task.retry_count += 1;
                task.state = TaskState::Pending;
                task.error = None;
                task.progress = 0;

                let event = TrackerEvent {
                    event_type: TrackerEventType::TaskRetried,
                    task_id: id.to_string(),
                    timestamp: current_timestamp(),
                    details: Some(format!("attempt:{}", task.retry_count)),
                };
                drop(tasks);
                self.add_event(event).await;
                return true;
            }
        }
        false
    }

    /// Cancel a task
    pub async fn cancel_task(&self, id: &str) -> bool {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(id) {
            if task.state.can_transition_to(TaskState::Cancelled) {
                task.state = TaskState::Cancelled;
                task.completed_at = Some(current_timestamp());

                let event = TrackerEvent {
                    event_type: TrackerEventType::TaskCancelled,
                    task_id: id.to_string(),
                    timestamp: current_timestamp(),
                    details: None,
                };
                drop(tasks);
                self.add_event(event).await;
                return true;
            }
        }
        false
    }

    /// Get tracker statistics
    pub async fn get_stats(&self) -> TrackerStats {
        let tasks = self.tasks.read().await;
        let mut stats = TrackerStats {
            total_tasks: tasks.len(),
            pending: 0,
            in_progress: 0,
            waiting_for_deps: 0,
            completed: 0,
            failed: 0,
            cancelled: 0,
        };

        for task in tasks.values() {
            match task.state {
                TaskState::Pending => stats.pending += 1,
                TaskState::InProgress => stats.in_progress += 1,
                TaskState::WaitingForDeps => stats.waiting_for_deps += 1,
                TaskState::Completed => stats.completed += 1,
                TaskState::Failed => stats.failed += 1,
                TaskState::Cancelled => stats.cancelled += 1,
                TaskState::NeedsVerification => {}
            }
        }
        stats
    }

    /// Get event history
    pub async fn get_events(&self, limit: Option<usize>) -> Vec<TrackerEvent> {
        let events = self.event_history.read().await;
        let limit = limit.unwrap_or(self.max_events);
        events.iter().rev().take(limit).cloned().collect()
    }

    /// Check if all root tasks are complete
    pub async fn is_complete(&self) -> bool {
        let roots = self.get_root_tasks().await;
        roots.iter().all(|t| t.state == TaskState::Completed)
    }

    /// Check if any root task failed
    pub async fn has_failures(&self) -> bool {
        let roots = self.get_root_tasks().await;
        roots.iter().any(|t| t.state == TaskState::Failed)
    }

    /// Get workflow completion percentage
    pub async fn get_overall_progress(&self) -> u8 {
        let roots = self.get_root_tasks().await;
        if roots.is_empty() {
            return 0;
        }
        let total: u32 = roots.iter().map(|t| t.progress as u32).sum();
        (total / roots.len() as u32) as u8
    }

    // Internal helpers

    async fn update_parent_progress(&self, parent_id: &str) {
        // Get subtask IDs first
        let subtask_ids: Vec<String> = {
            let tasks = self.tasks.read().await;
            tasks.get(parent_id).map(|p| p.subtasks.clone()).unwrap_or_default()
        };

        if subtask_ids.is_empty() {
            return;
        }

        // Count completed subtasks
        let completed: usize = {
            let tasks = self.tasks.read().await;
            subtask_ids.iter().filter(|sid| {
                tasks.get(sid.as_str())
                    .map(|t| t.state == TaskState::Completed)
                    .unwrap_or(false)
            }).count()
        };

        let total = subtask_ids.len();
        let progress = ((completed * 100) / total) as u8;

        // Update parent progress
        {
            let mut tasks = self.tasks.write().await;
            if let Some(parent) = tasks.get_mut(parent_id) {
                parent.progress = progress;
            }
        }

        // Propagate to grandparent (non-recursive, single level)
        let grandparent_id = {
            let tasks = self.tasks.read().await;
            tasks.get(parent_id).and_then(|p| p.parent_id.clone())
        };

        if let Some(gpid) = grandparent_id {
            // Get grandparent's subtask info
            let gp_subtask_ids: Vec<String> = {
                let tasks = self.tasks.read().await;
                tasks.get(&gpid).map(|p| p.subtasks.clone()).unwrap_or_default()
            };

            if !gp_subtask_ids.is_empty() {
                let gp_completed: usize = {
                    let tasks = self.tasks.read().await;
                    gp_subtask_ids.iter().filter(|sid| {
                        tasks.get(sid.as_str())
                            .map(|t| t.state == TaskState::Completed)
                            .unwrap_or(false)
                    }).count()
                };
                let gp_total = gp_subtask_ids.len();
                let gp_progress = ((gp_completed * 100) / gp_total) as u8;

                let mut tasks = self.tasks.write().await;
                if let Some(gp) = tasks.get_mut(&gpid) {
                    gp.progress = gp_progress;
                }
            }
        }
    }

    async fn add_event(&self, event: TrackerEvent) {
        let mut history = self.event_history.write().await;
        if history.len() >= self.max_events {
            history.pop_front();
        }
        history.push_back(event);
    }

    async fn unblock_dependents(&self, completed_id: &str) {
        let completed_id_str = completed_id.to_string();

        // First pass: collect task states
        let task_states: HashMap<String, TaskState> = {
            let tasks = self.tasks.read().await;
            tasks.iter().map(|(k, v)| (k.clone(), v.state)).collect()
        };

        // Second pass: update blocked tasks
        {
            let mut tasks = self.tasks.write().await;
            for task in tasks.values_mut() {
                if task.dependencies.iter().any(|d| d == &completed_id_str) {
                    if task.dependencies_satisfied(&task_states) {
                        if task.state == TaskState::WaitingForDeps {
                            task.state = TaskState::Pending;
                        }
                    }
                }
            }
        }
    }
}

impl Default for TaskTracker {
    fn default() -> Self {
        Self::new(1000)
    }
}

// =============================================================================
// Async Task Tracker Handle (for sharing across agents)
// =============================================================================

/// Shared handle to a task tracker
pub struct TaskTrackerHandle {
    inner: Arc<TaskTracker>,
}

impl TaskTrackerHandle {
    pub fn new(tracker: TaskTracker) -> Self {
        Self {
            inner: Arc::new(tracker),
        }
    }

    pub fn into_inner(self) -> Arc<TaskTracker> {
        self.inner
    }

    pub fn inner(&self) -> &Arc<TaskTracker> {
        &self.inner
    }
}

impl Clone for TaskTrackerHandle {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl From<TaskTracker> for TaskTrackerHandle {
    fn from(tracker: TaskTracker) -> Self {
        Self::new(tracker)
    }
}

// =============================================================================
// Utilities
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_and_track_task() {
        let tracker = TaskTracker::default();

        let task = tracker
            .create_task("task-1".to_string(), "Test task".to_string())
            .await;
        assert_eq!(task.id, "task-1");
        assert_eq!(task.state, TaskState::Pending);

        let retrieved = tracker.get_task("task-1").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().description, "Test task");
    }

    #[tokio::test]
    async fn test_task_lifecycle() {
        let tracker = TaskTracker::default();

        tracker
            .create_task("task-1".to_string(), "Test task".to_string())
            .await;

        // Start task
        let started = tracker.start_task("task-1", Some("worker-1".to_string())).await;
        assert!(started);

        let task = tracker.get_task("task-1").await.unwrap();
        assert_eq!(task.state, TaskState::InProgress);
        assert_eq!(task.assigned_to, Some("worker-1".to_string()));

        // Update progress
        let updated = tracker.update_progress("task-1", 50).await;
        assert!(updated);

        let task = tracker.get_task("task-1").await.unwrap();
        assert_eq!(task.progress, 50);

        // Complete task
        let completed = tracker.complete_task("task-1").await;
        assert!(completed);

        let task = tracker.get_task("task-1").await.unwrap();
        assert_eq!(task.state, TaskState::Completed);
        assert_eq!(task.progress, 100);
    }

    #[tokio::test]
    async fn test_dependencies() {
        let tracker = TaskTracker::default();

        // Create dependent task
        tracker
            .create_task_with(
                "task-2".to_string(),
                "Dependent task".to_string(),
                TrackPriority::High,
                vec!["task-1".to_string()],
                None,
            )
            .await;

        // Initially blocked
        let blocked = tracker.get_blocked_tasks().await;
        assert_eq!(blocked.len(), 1);

        // Complete dependency
        tracker
            .create_task("task-1".to_string(), "Dependency".to_string())
            .await;
        tracker.start_task("task-1", None).await;
        tracker.complete_task("task-1").await;

        // Now task-2 should be runnable
        let runnable = tracker.get_runnable_tasks().await;
        assert!(runnable.iter().any(|t| t.id == "task-2"));
    }

    #[tokio::test]
    async fn test_parent_child_progress() {
        let tracker = TaskTracker::default();

        // Create parent task
        tracker
            .create_task("parent".to_string(), "Parent task".to_string())
            .await;

        // Create subtasks
        tracker
            .create_task_with(
                "child-1".to_string(),
                "Child 1".to_string(),
                TrackPriority::Normal,
                vec![],
                Some("parent".to_string()),
            )
            .await;

        tracker
            .create_task_with(
                "child-2".to_string(),
                "Child 2".to_string(),
                TrackPriority::Normal,
                vec![],
                Some("parent".to_string()),
            )
            .await;

        // Complete one child
        tracker.start_task("child-1", None).await;
        tracker.complete_task("child-1").await;

        let parent = tracker.get_task("parent").await.unwrap();
        assert_eq!(parent.progress, 50); // 1 of 2 complete

        // Complete second child
        tracker.start_task("child-2", None).await;
        tracker.complete_task("child-2").await;

        let parent = tracker.get_task("parent").await.unwrap();
        assert_eq!(parent.progress, 100);
    }

    #[tokio::test]
    async fn test_stats() {
        let tracker = TaskTracker::default();

        tracker.create_task("t1".to_string(), "Task 1".to_string()).await;
        tracker.create_task("t2".to_string(), "Task 2".to_string()).await;
        tracker.create_task("t3".to_string(), "Task 3".to_string()).await;

        tracker.start_task("t1", None).await;
        tracker.complete_task("t1").await;
        tracker.fail_task("t2", Some("Error".to_string())).await;

        let stats = tracker.get_stats().await;
        assert_eq!(stats.total_tasks, 3);
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.failed, 1);
        assert_eq!(stats.pending, 1);
    }
}
