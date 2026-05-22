//! Event Emitter Module for framework observability.
//!
//! Based on research from:
//! - rs-event-emitter - Thread-safe event emitter
//! - async-event-emitter - Async event processing
//! - agentrs_multi InMemoryBus - Multi-agent event broadcasting

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::utils::current_timestamp;

/// Event type identifier
pub type EventType = &'static str;

/// JSON value for event data
pub type EventData = serde_json::Value;

/// A published event
#[derive(Clone)]
pub struct Event {
    /// Event type
    pub event_type: EventType,
    /// Event data payload
    pub data: EventData,
    /// Timestamp
    pub timestamp: u64,
    /// Source component
    pub source: Option<String>,
}

impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Event")
            .field("event_type", &self.event_type)
            .field("data", &self.data)
            .field("timestamp", &self.timestamp)
            .field("source", &self.source)
            .finish()
    }
}

impl Event {
    /// Create a new event
    pub fn new(event_type: EventType, data: EventData) -> Self {
        Self {
            event_type,
            data,
            timestamp: current_timestamp(),
            source: None,
        }
    }

    /// Create an event with source
    pub fn with_source(event_type: EventType, data: EventData, source: impl Into<String>) -> Self {
        Self {
            event_type,
            data,
            timestamp: current_timestamp(),
            source: Some(source.into()),
        }
    }
}

/// Event handler function type - async function that takes an event
pub type EventHandler =
    Arc<dyn Fn(Event) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> + Send + Sync>;

/// Unique handler identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HandlerId(u64);

impl HandlerId {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

/// A handler registration
struct HandlerEntry {
    id: HandlerId,
    handler: EventHandler,
}

impl std::fmt::Debug for HandlerEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandlerEntry")
            .field("id", &self.id)
            .finish()
    }
}

/// Statistics for the event emitter
#[derive(Debug, Clone, Default)]
pub struct EmitterStats {
    pub total_events: u64,
    pub total_handlers: usize,
    pub events_by_type: HashMap<String, u64>,
}

/// Thread-safe event emitter
#[derive(Debug, Clone)]
pub struct EventEmitter {
    /// Subscribers by event type
    subscribers: Arc<RwLock<HashMap<EventType, Vec<HandlerEntry>>>>,
    /// Global subscribers (receive all events)
    global_subscribers: Arc<RwLock<Vec<HandlerEntry>>>,
    /// Event history
    history: Arc<RwLock<VecDeque<Event>>>,
    /// Maximum history size
    max_history: usize,
    /// Statistics
    stats: Arc<RwLock<EmitterStats>>,
}

impl Default for EventEmitter {
    fn default() -> Self {
        Self::new(1000)
    }
}

impl EventEmitter {
    /// Create a new event emitter
    pub fn new(max_history: usize) -> Self {
        Self {
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            global_subscribers: Arc::new(RwLock::new(Vec::new())),
            history: Arc::new(RwLock::new(VecDeque::with_capacity(max_history))),
            max_history,
            stats: Arc::new(RwLock::new(EmitterStats::default())),
        }
    }

    /// Subscribe to a specific event type
    pub async fn on<F, Fut>(&self, event_type: EventType, handler: F) -> HandlerId
    where
        F: Fn(Event) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let id = HandlerId::new();
        let boxed: EventHandler = Arc::new(move |event| Box::pin(handler(event)));

        let mut subscribers = self.subscribers.write().await;
        subscribers.entry(event_type).or_default().push(HandlerEntry { id, handler: boxed });

        // Update stats
        self.update_stats().await;

        id
    }

    /// Subscribe to all events (global subscriber)
    pub async fn on_all<F, Fut>(&self, handler: F) -> HandlerId
    where
        F: Fn(Event) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let id = HandlerId::new();
        let boxed: EventHandler = Arc::new(move |event| Box::pin(handler(event)));

        let mut global = self.global_subscribers.write().await;
        global.push(HandlerEntry { id, handler: boxed });

        // Update stats
        self.update_stats().await;

        id
    }

    /// Unsubscribe a handler
    pub async fn off(&self, id: HandlerId) -> bool {
        // Check subscribers
        {
            let mut subscribers = self.subscribers.write().await;
            for handlers in subscribers.values_mut() {
                if let Some(pos) = handlers.iter().position(|h| h.id == id) {
                    handlers.remove(pos);
                    self.update_stats().await;
                    return true;
                }
            }
        }

        // Check global subscribers
        {
            let mut global = self.global_subscribers.write().await;
            if let Some(pos) = global.iter().position(|h| h.id == id) {
                global.remove(pos);
                self.update_stats().await;
                return true;
            }
        }

        false
    }

    /// Emit an event to all subscribers
    pub async fn emit(&self, event: Event) {
        // Store in history
        {
            let mut history = self.history.write().await;
            if history.len() >= self.max_history {
                history.pop_front();
            }
            history.push_back(event.clone());
        }

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_events += 1;
            *stats.events_by_type.entry(event.event_type.to_string()).or_insert(0) += 1;
        }

        // Collect handlers to call (avoid holding locks during async calls)
        let handlers: Vec<EventHandler> = {
            let mut to_call = Vec::new();

            // Get type-specific handlers
            let subscribers = self.subscribers.read().await;
            if let Some(type_handlers) = subscribers.get(event.event_type) {
                for entry in type_handlers.iter() {
                    to_call.push(EventHandler::clone(&entry.handler));
                }
            }
            to_call
        };

        // Get global handlers
        let global_handlers: Vec<EventHandler> = {
            let global = self.global_subscribers.read().await;
            global.iter().map(|entry| EventHandler::clone(&entry.handler)).collect()
        };

        // Call all handlers concurrently
        for handler in handlers.into_iter().chain(global_handlers.into_iter()) {
            let event_clone = event.clone();
            tokio::spawn(async move {
                handler(event_clone).await;
            });
        }
    }

    /// Emit a simple event with data
    pub async fn emit_simple(&self, event_type: EventType, data: EventData) {
        self.emit(Event::new(event_type, data)).await;
    }

    /// Emit an event with source
    pub async fn emit_with_source(
        &self,
        event_type: EventType,
        data: EventData,
        source: impl Into<String>,
    ) {
        self.emit(Event::with_source(event_type, data, source)).await;
    }

    /// Get event history
    pub async fn history(&self, limit: Option<usize>) -> Vec<Event> {
        let history = self.history.read().await;
        let limit = limit.unwrap_or(self.max_history).min(history.len());
        history.iter().rev().take(limit).cloned().collect()
    }

    /// Get emitter statistics
    pub async fn stats(&self) -> EmitterStats {
        self.stats.read().await.clone()
    }

    /// Get number of subscribers for an event type
    pub async fn subscriber_count(&self, event_type: EventType) -> usize {
        let subscribers = self.subscribers.read().await;
        subscribers.get(event_type).map(|v| v.len()).unwrap_or(0)
    }

    /// Clear all handlers
    pub async fn clear(&self) {
        {
            let mut subscribers = self.subscribers.write().await;
            subscribers.clear();
        }
        {
            let mut global = self.global_subscribers.write().await;
            global.clear();
        }
        self.update_stats().await;
    }

    /// Get all registered event types
    pub async fn event_types(&self) -> Vec<EventType> {
        let subscribers = self.subscribers.read().await;
        subscribers.keys().cloned().collect()
    }

    // Internal helpers

    async fn update_stats(&self) {
        let subscribers = self.subscribers.read().await;
        let global = self.global_subscribers.read().await;
        let total_handlers = subscribers.values().map(|v| v.len()).sum::<usize>() + global.len();
        
        let mut stats = self.stats.write().await;
        stats.total_handlers = total_handlers;
    }
}

// =============================================================================
// Event Types for Multi-Agent Framework
// =============================================================================

/// Framework event types
pub mod event_types {
    use super::*;

    // Task events
    pub const TASK_CREATED: EventType = "task:created";
    pub const TASK_STARTED: EventType = "task:started";
    pub const TASK_PROGRESS: EventType = "task:progress";
    pub const TASK_COMPLETED: EventType = "task:completed";
    pub const TASK_FAILED: EventType = "task:failed";
    pub const TASK_CANCELLED: EventType = "task:cancelled";

    // Worker pool events
    pub const WORKER_POOL_SUBMIT: EventType = "worker_pool:submit";
    pub const WORKER_POOL_COMPLETE: EventType = "worker_pool:complete";
    pub const WORKER_POOL_SHUTDOWN: EventType = "worker_pool:shutdown";

    // Agent events
    pub const AGENT_SPAWNED: EventType = "agent:spawned";
    pub const AGENT_MESSAGE: EventType = "agent:message";
    pub const AGENT_RESPONSE: EventType = "agent:response";
    pub const AGENT_ERROR: EventType = "agent:error";

    // Memory events
    pub const MEMORY_STORE: EventType = "memory:store";
    pub const MEMORY_RETRIEVE: EventType = "memory:retrieve";
    pub const MEMORY_COMPRESS: EventType = "memory:compress";

    // Tool events
    pub const TOOL_REGISTRY_REGISTER: EventType = "tool:registered";
    pub const TOOL_REGISTRY_UNREGISTER: EventType = "tool:unregistered";
    pub const TOOL_CALLED: EventType = "tool:called";
    pub const TOOL_RESULT: EventType = "tool:result";

    // System events
    pub const SYSTEM_START: EventType = "system:start";
    pub const SYSTEM_SHUTDOWN: EventType = "system:shutdown";
    pub const SYSTEM_ERROR: EventType = "system:error";
}

// =============================================================================
// Global Event Emitter (Singleton)
// =============================================================================

use std::sync::OnceLock;

static GLOBAL_EMITTER: OnceLock<EventEmitter> = OnceLock::new();

/// Get the global event emitter instance
pub fn global_emitter() -> &'static EventEmitter {
    GLOBAL_EMITTER.get_or_init(|| EventEmitter::default())
}

// =============================================================================
// Utilities
// =============================================================================

// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_subscription() {
        let emitter = EventEmitter::new(100);
        let received = Arc::new(RwLock::new(Vec::new()));
        let received_clone = received.clone();

        let emitter_clone = emitter.clone();
        emitter_clone
            .on("test", move |event| {
                let received_clone = received_clone.clone();
                Box::pin(async move {
                    received_clone.write().await.push(event.event_type);
                })
            })
            .await;

        emitter.emit_simple("test", serde_json::json!({})).await;

        // Give time for async handlers
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        let guard = received.read().await;
        assert_eq!(guard.len(), 1);
    }

    #[tokio::test]
    async fn test_global_subscription() {
        let emitter = EventEmitter::new(100);
        let received = Arc::new(RwLock::new(Vec::new()));
        let received_clone = received.clone();

        let emitter_clone = emitter.clone();
        emitter_clone.on_all(move |event| {
            let received_clone = received_clone.clone();
            Box::pin(async move {
                received_clone.write().await.push(event.event_type);
            })
        }).await;

        emitter.emit_simple("type1", serde_json::json!({})).await;
        emitter.emit_simple("type2", serde_json::json!({})).await;

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        let guard = received.read().await;
        assert_eq!(guard.len(), 2);
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let emitter = EventEmitter::new(100);
        let counter = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let counter_clone = counter.clone();

        let id = emitter
            .on("test", move |_| {
                let counter_clone = counter_clone.clone();
                Box::pin(async move {
                    counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                })
            })
            .await;

        emitter.emit_simple("test", serde_json::json!({})).await;
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        // Unsubscribe
        emitter.off(id).await;

        emitter.emit_simple("test", serde_json::json!({})).await;
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        // Should still be 1 because we unsubscribed
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_history() {
        let emitter = EventEmitter::new(10);

        for i in 0..15 {
            emitter.emit_simple("test", serde_json::json!({"i": i})).await;
        }

        let history = emitter.history(None).await;
        assert_eq!(history.len(), 10); // Limited to max_history
    }

    #[tokio::test]
    async fn test_stats() {
        let emitter = EventEmitter::new(100);

        emitter.emit_simple("event1", serde_json::json!({})).await;
        emitter.emit_simple("event2", serde_json::json!({})).await;
        emitter.emit_simple("event1", serde_json::json!({})).await;

        let stats = emitter.stats().await;
        assert_eq!(stats.total_events, 3);
        assert_eq!(stats.events_by_type.get("event1"), Some(&2));
        assert_eq!(stats.events_by_type.get("event2"), Some(&1));
    }

    #[tokio::test]
    async fn test_multiple_handlers_same_type() {
        let emitter = EventEmitter::new(100);
        let counter = Arc::new(std::sync::atomic::AtomicU64::new(0));

        for _ in 0..5 {
            let counter_clone = counter.clone();
            emitter
                .on("test", move |_| {
                    let counter_clone = counter_clone.clone();
                    Box::pin(async move {
                        counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    })
                })
                .await;
        }

        emitter.emit_simple("test", serde_json::json!({})).await;
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 5);
    }

    #[tokio::test]
    async fn test_global_emitter() {
        let emitter = global_emitter();
        let received = Arc::new(RwLock::new(false));
        let received_clone = received.clone();

        let emitter_clone = emitter.clone();
        emitter_clone.on("global_test", move |_| {
            let value = received_clone.clone();
            Box::pin(async move {
                *value.write().await = true;
            })
        }).await;

        emitter.emit_simple("global_test", serde_json::json!({})).await;
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        assert!(*received.read().await);
    }
}
