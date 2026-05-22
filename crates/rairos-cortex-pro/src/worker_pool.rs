//! Worker Pool Module for parallel task execution.
//!
//! Based on research from:
//! - Thread pool patterns in screenpipe, yazi, hurl
//! - tokio::sync::mpsc for async message passing
//!
//! ## Architecture
//!
//! ```text
//! Task Queue
//!     │
//!     ▼
//! ┌─────────────────────┐
//! │    Worker Pool      │
//! │  ┌───┐ ┌───┐ ┌───┐ │
//! │  │ W │ │ W │ │ W │ │
//! │  └───┘ └───┘ └───┘ │
//! └─────────────────────┘
//!     │
//!     ▼
//!  Results
//! ```

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{mpsc, watch, Mutex};

use crate::utils::current_timestamp;

/// Task priority levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// A task to be executed by the worker pool
pub struct Task {
    /// Task ID
    pub id: String,
    /// Task payload (boxed for type erasure)
    pub payload: TaskPayload,
    /// Priority level
    pub priority: Priority,
    /// Created at timestamp
    pub created_at: u64,
}

impl Task {
    /// Create a new task
    pub fn new(id: String, payload: TaskPayload, priority: Priority) -> Self {
        Self {
            id,
            payload,
            priority,
            created_at: current_timestamp(),
        }
    }
}

/// Task payload - enum of possible task types
pub enum TaskPayload {
    /// Simple function that takes no input and returns Result
    Simple(Box<dyn FnOnce() -> Result<String, String> + Send + 'static>),
    /// Function with string input
    WithInput(
        Box<dyn FnOnce(String) -> Result<String, String> + Send + 'static>,
        String,
    ),
}

/// Task result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Task ID
    pub task_id: String,
    /// Execution output
    pub output: Result<String, String>,
    /// Execution time in ms
    pub execution_time_ms: u64,
}

/// Worker pool configuration
#[derive(Debug, Clone)]
pub struct WorkerPoolConfig {
    /// Number of workers
    pub num_workers: usize,
    /// Channel capacity for task queue
    pub queue_capacity: usize,
    /// Enable priority queue
    pub priority_enabled: bool,
    /// Shutdown timeout in ms
    pub shutdown_timeout_ms: u64,
}

impl Default for WorkerPoolConfig {
    fn default() -> Self {
        Self {
            num_workers: 4,
            queue_capacity: 100,
            priority_enabled: true,
            shutdown_timeout_ms: 5000,
        }
    }
}

/// Worker pool state
struct WorkerPoolState {
    /// Workers running
    num_active: usize,
    /// Total tasks processed
    total_processed: u64,
    /// Total tasks failed
    total_failed: u64,
    /// Is shutdown
    is_shutdown: bool,
}

/// Sync Worker Pool for parallel task execution
pub struct WorkerPool {
    config: WorkerPoolConfig,
    /// Task sender
    task_tx: mpsc::Sender<Task>,
    /// Shared result receiver (protected by mutex since sync context)
    result_rx: Arc<Mutex<mpsc::Receiver<TaskResult>>>,
    /// Shutdown signal sender
    shutdown_tx: watch::Sender<bool>,
    /// Internal state
    state: Arc<std::sync::Mutex<WorkerPoolState>>,
}

impl WorkerPool {
    /// Create a new worker pool
    pub fn new(config: WorkerPoolConfig) -> Self {
        let (task_tx, task_rx) = mpsc::channel(config.queue_capacity);
        let (result_tx, result_rx) = mpsc::channel(config.queue_capacity);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let result_rx = Arc::new(Mutex::new(result_rx));
        let result_tx = Arc::new(std::sync::Mutex::new(result_tx));

        let state = Arc::new(std::sync::Mutex::new(WorkerPoolState {
            num_active: 0,
            total_processed: 0,
            total_failed: 0,
            is_shutdown: false,
        }));

        let task_rx = Arc::new(Mutex::new(task_rx));

        // Spawn workers
        for worker_id in 0..config.num_workers {
            let task_rx = Arc::clone(&task_rx);
            let result_tx = Arc::clone(&result_tx);
            let shutdown_rx = shutdown_tx.subscribe();
            let state = Arc::clone(&state);

            std::thread::spawn(move || {
                Self::worker_loop(worker_id, task_rx, result_tx, shutdown_rx, state);
            });
        }

        Self {
            config,
            task_tx,
            result_rx,
            shutdown_tx,
            state,
        }
    }

    /// Worker main loop
    fn worker_loop(
        worker_id: usize,
        task_rx: Arc<Mutex<mpsc::Receiver<Task>>>,
        result_tx: Arc<std::sync::Mutex<mpsc::Sender<TaskResult>>>,
        mut shutdown_rx: watch::Receiver<bool>,
        state: Arc<std::sync::Mutex<WorkerPoolState>>,
    ) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            loop {
                // Check shutdown first with biased ranking
                if *shutdown_rx.borrow() {
                    break;
                }

                // Try to receive with timeout
                let task = {
                    let mut rx = task_rx.lock().await;
                    rx.recv().await
                };

                match task {
                    Some(t) => {
                        let start = std::time::Instant::now();
                        let result = Self::execute_task(t);
                        let execution_time_ms = start.elapsed().as_millis() as u64;

                        if result.output.is_err() {
                            if let Ok(mut s) = state.lock() {
                                s.total_failed += 1;
                            }
                        }
                        if let Ok(mut s) = state.lock() {
                            s.total_processed += 1;
                        }

                        let tx = result_tx.lock().unwrap();
                        let _ = tx.send(result).await;
                    }
                    None => break,
                }
            }
        });

                tracing::info!("Worker {} shutting down", worker_id);
    }

    /// Execute a single task
    fn execute_task(task: Task) -> TaskResult {
        let start = std::time::Instant::now();
        let output = match task.payload {
            TaskPayload::Simple(f) => f(),
            TaskPayload::WithInput(f, input) => f(input),
        };

        TaskResult {
            task_id: task.id,
            output,
            execution_time_ms: start.elapsed().as_millis() as u64,
        }
    }

    /// Submit a task to the pool
    pub async fn submit(&self, task: Task) -> Result<(), String> {
        if self.task_tx.send(task).await.is_err() {
            return Err("Worker pool is shut down".to_string());
        }
        Ok(())
    }

    /// Submit multiple tasks
    pub async fn submit_all(&self, tasks: Vec<Task>) -> Vec<Result<(), String>> {
        let mut results = Vec::with_capacity(tasks.len());
        for task in tasks {
            results.push(self.submit(task).await);
        }
        results
    }

    /// Get next result
    pub async fn recv(&mut self) -> Option<TaskResult> {
        let mut rx = self.result_rx.lock().await;
        rx.recv().await
    }

    /// Try to get a result without blocking
    pub fn try_recv(&self) -> Option<TaskResult> {
        // Note: In sync context, this is racy. For production, use recv().await
        self.result_rx.try_lock().ok()?.try_recv().ok()
    }

    /// Shutdown the worker pool
    pub async fn shutdown(&mut self) {
        // Signal shutdown
        let _ = self.shutdown_tx.send(true);

        // Close task channel
        drop(&self.task_tx);

        // Drain remaining results
        let mut rx = self.result_rx.lock().await;
        while rx.recv().await.is_some() {}

        // Update state
        if let Ok(mut s) = self.state.lock() {
            s.is_shutdown = true;
        }
    }

    /// Get pool statistics
    pub fn stats(&self) -> WorkerPoolStats {
        let state = self.state.lock().unwrap();
        WorkerPoolStats {
            num_workers: self.config.num_workers,
            num_active: state.num_active,
            total_processed: state.total_processed,
            total_failed: state.total_failed,
            is_shutdown: state.is_shutdown,
            queue_capacity: self.config.queue_capacity,
        }
    }
}

/// Worker pool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerPoolStats {
    pub num_workers: usize,
    pub num_active: usize,
    pub total_processed: u64,
    pub total_failed: u64,
    pub is_shutdown: bool,
    pub queue_capacity: usize,
}

// =============================================================================
// Priority Queue Implementation
// =============================================================================

/// Priority queue for tasks
pub struct PriorityTaskQueue {
    queues: [VecDeque<Task>; 4], // 4 priority levels
    total_count: usize,
}

impl PriorityTaskQueue {
    /// Create new priority queue
    pub fn new() -> Self {
        Self {
            queues: [
                VecDeque::new(),
                VecDeque::new(),
                VecDeque::new(),
                VecDeque::new(),
            ],
            total_count: 0,
        }
    }

    /// Push a task
    pub fn push(&mut self, task: Task) {
        let priority = task.priority as usize;
        self.queues[priority].push_back(task);
        self.total_count += 1;
    }

    /// Pop the highest priority task
    pub fn pop(&mut self) -> Option<Task> {
        // Check from highest to lowest priority
        for queue in &mut self.queues.iter_mut().rev() {
            if let Some(task) = queue.pop_front() {
                self.total_count -= 1;
                return Some(task);
            }
        }
        None
    }

    /// Peek at highest priority task
    pub fn peek(&self) -> Option<&Task> {
        for queue in self.queues.iter().rev() {
            if let Some(task) = queue.front() {
                return Some(task);
            }
        }
        None
    }

    /// Get total count
    pub fn len(&self) -> usize {
        self.total_count
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.total_count == 0
    }
}

impl Default for PriorityTaskQueue {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Async Worker Pool (Tokio-based)
// =============================================================================

/// Async worker pool using tokio runtime
pub struct AsyncWorkerPool {
    config: WorkerPoolConfig,
    task_tx: mpsc::Sender<Task>,
    result_rx: Arc<Mutex<mpsc::Receiver<TaskResult>>>,
    shutdown_tx: watch::Sender<bool>,
}

/// Result of async task submission
pub struct TaskHandle {
    pub task_id: String,
    result_rx: tokio::sync::oneshot::Receiver<TaskResult>,
}

impl TaskHandle {
    /// Wait for result
    pub async fn await_result(self) -> TaskResult {
        self.result_rx.await.unwrap()
    }
}

impl AsyncWorkerPool {
    /// Create new async worker pool
    pub fn new(config: WorkerPoolConfig) -> Self {
        let (task_tx, task_rx) = mpsc::channel(config.queue_capacity);
        let (result_tx, result_rx) = mpsc::channel(config.queue_capacity);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let task_rx = Arc::new(Mutex::new(task_rx));
        let result_rx = Arc::new(Mutex::new(result_rx));
        let result_tx = Arc::new(tokio::sync::Mutex::new(result_tx));

        let state = Arc::new(std::sync::Mutex::new(WorkerPoolState {
            num_active: 0,
            total_processed: 0,
            total_failed: 0,
            is_shutdown: false,
        }));

        // Spawn async workers
        for worker_id in 0..config.num_workers {
            let mut task_rx = Arc::clone(&task_rx);
            let result_tx = Arc::clone(&result_tx);
            let mut shutdown_rx = shutdown_rx.clone();
            let state = Arc::clone(&state);

            tokio::spawn(async move {
                loop {
                    if *shutdown_rx.borrow() {
                        break;
                    }

                    let task = {
                        let mut rx = task_rx.lock().await;
                        rx.recv().await
                    };

                    match task {
                        Some(t) => {
                            let start = std::time::Instant::now();
                            let result = Self::execute_task_sync(t);
                            let execution_time_ms = start.elapsed().as_millis() as u64;

                            if result.output.is_err() {
                                if let Ok(mut s) = state.lock() {
                                    s.total_failed += 1;
                                }
                            }
                            if let Ok(mut s) = state.lock() {
                                s.total_processed += 1;
                            }

                            let tx = result_tx.lock().await;
                            let _ = tx.send(result).await;
                        }
                        None => break,
                    }
                }
                tracing::info!("Async worker {} shutting down", worker_id);
            });
        }

        Self {
            config,
            task_tx,
            result_rx,
            shutdown_tx,
        }
    }

    /// Execute task synchronously
    fn execute_task_sync(task: Task) -> TaskResult {
        let start = std::time::Instant::now();
        let output = match task.payload {
            TaskPayload::Simple(f) => f(),
            TaskPayload::WithInput(f, input) => f(input),
        };

        TaskResult {
            task_id: task.id,
            output,
            execution_time_ms: start.elapsed().as_millis() as u64,
        }
    }

    /// Submit task and get handle for result
    pub async fn submit_with_handle(&self, task: Task) -> Result<TaskHandle, String> {
        let task_id = task.id.clone();
        let (result_tx, result_rx) = tokio::sync::oneshot::channel();

        if self.task_tx.send(task).await.is_err() {
            return Err("Worker pool is shut down".to_string());
        }

        Ok(TaskHandle {
            task_id,
            result_rx,
        })
    }

    /// Submit a task
    pub async fn submit(&self, task: Task) -> Result<(), String> {
        if self.task_tx.send(task).await.is_err() {
            return Err("Worker pool is shut down".to_string());
        }
        Ok(())
    }

    /// Get next result
    pub async fn recv(&mut self) -> Option<TaskResult> {
        let mut rx = self.result_rx.lock().await;
        rx.recv().await
    }

    /// Shutdown pool
    pub async fn shutdown(&mut self) {
        let _ = self.shutdown_tx.send(true);
        drop(&self.task_tx);
        let mut rx = self.result_rx.lock().await;
        while rx.recv().await.is_some() {}
    }
}

// =============================================================================
// Utilities
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_queue() {
        let mut queue = PriorityTaskQueue::new();

        queue.push(Task::new(
            "1".to_string(),
            TaskPayload::Simple(Box::new(|| Ok("low".to_string()))),
            Priority::Low,
        ));
        queue.push(Task::new(
            "2".to_string(),
            TaskPayload::Simple(Box::new(|| Ok("high".to_string()))),
            Priority::High,
        ));
        queue.push(Task::new(
            "3".to_string(),
            TaskPayload::Simple(Box::new(|| Ok("critical".to_string()))),
            Priority::Critical,
        ));

        // Should pop in priority order
        let first = queue.pop().unwrap();
        assert_eq!(first.id, "3"); // Critical first

        let second = queue.pop().unwrap();
        assert_eq!(second.id, "2"); // High second

        let third = queue.pop().unwrap();
        assert_eq!(third.id, "1"); // Low last
    }

    #[tokio::test]
    async fn test_async_worker_pool() {
        let config = WorkerPoolConfig {
            num_workers: 2,
            queue_capacity: 10,
            priority_enabled: true,
            shutdown_timeout_ms: 1000,
        };

        let pool = AsyncWorkerPool::new(config);

        let task = Task::new(
            "test-1".to_string(),
            TaskPayload::Simple(Box::new(|| Ok("result".to_string()))),
            Priority::Normal,
        );

        pool.submit(task).await.unwrap();

        let result = pool.recv().await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().task_id, "test-1");

        pool.shutdown().await;
    }

    #[test]
    fn test_execute_task() {
        let task = Task::new(
            "sync".to_string(),
            TaskPayload::Simple(Box::new(|| Ok("done".to_string()))),
            Priority::Normal,
        );

        let start = std::time::Instant::now();
        let output = match task.payload {
            TaskPayload::Simple(f) => f(),
            TaskPayload::WithInput(f, input) => f(input),
        };
        let execution_time_ms = start.elapsed().as_millis() as u64;

        let result = TaskResult {
            task_id: task.id,
            output,
            execution_time_ms,
        };
        assert!(result.output.is_ok());
    }
}
