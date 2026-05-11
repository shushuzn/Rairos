//! # rairos-daemon
//!
//! Async event bus + SSE streaming for the autonomous orchestrator.
//!
//! Provides:
//! - [`EventBus`]: Thread-safe pub/sub singleton for daemon events.
//! - [`DaemonEvent`]: Standard event payload published by the daemon.
//! - [`SseServer`]: Async HTTP server streaming events via Server-Sent Events.
//!
//! ## Event Types
//!
//! - `session_started`  : new orchestrator session began
//! - `session_completed`: orchestrator session finished (no alerts)
//! - `cycle_start`     : a cycle has started
//! - `cycle_complete`  : a cycle finished; data = `{"alerts": [...], "duration_s": float}`
//! - `alert_found`     : a ResearchAlert was generated; data = alert dict with severity
//! - `error`           : an exception occurred; data = `{"message": str, "exc": str}`

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tokio::sync::{broadcast, Mutex};
use tokio_stream::wrappers::BroadcastStream;
use warp::Filter;

// ─── Error types ───────────────────────────────────────────────────────────

#[derive(Error, Debug)]
pub enum DaemonError {
    #[error("event bus error: {0}")]
    EventBus(String),
    #[error("SSE server error: {0}")]
    SseServer(String),
    #[error("orchestrator error: {0}")]
    Orchestrator(String),
}

/// Result alias for daemon operations.
pub type DaemonResult<T> = Result<T, DaemonError>;

// ─── DaemonEvent ───────────────────────────────────────────────────────────

/// Standard event payload published by the ResearchDaemon.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DaemonEvent {
    /// Event type name (e.g. "cycle_start", "alert_found").
    pub event_type: String,
    /// Arbitrary event payload.
    pub data: Value,
    /// Unix timestamp in seconds.
    pub timestamp: f64,
}

impl DaemonEvent {
    /// Create a new event with the current timestamp.
    pub fn new(event_type: impl Into<String>, data: Value) -> Self {
        Self {
            event_type: event_type.into(),
            data,
            timestamp: current_timestamp(),
        }
    }

    /// Format as a Server-Sent Events string.
    pub fn to_sse(&self) -> String {
        let data = serde_json::to_string(&self.to_dict()).unwrap_or_else(|_| {
            serde_json::to_string(&serde_json::json!({
                "event_type": self.event_type,
                "data": Value::String(self.data.to_string()),
                "timestamp": self.timestamp,
            }))
            .unwrap_or_default()
        });
        format!("event: {}\ndata: {}\n\n", self.event_type, data)
    }

    /// Convert to a JSON-safe dictionary.
    pub fn to_dict(&self) -> Value {
        serde_json::json!({
            "event_type": self.event_type,
            "data": self.data,
            "timestamp": self.timestamp,
        })
    }
}

// ─── EventBus ──────────────────────────────────────────────────────────────

/// Thread-safe pub/sub event bus (singleton).
///
/// Subscribers register callbacks for specific event types. When
/// [`EventBus::publish`] is called, all matching callbacks are invoked
/// with the [`DaemonEvent`] payload.
pub struct EventBus {
    subscribers: Arc<Mutex<HashMap<String, Vec<broadcast::Sender<DaemonEvent>>>>>,
    history: Arc<Mutex<Vec<DaemonEvent>>>,
    max_history: usize,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(200)
    }
}

impl EventBus {
    /// Create a new EventBus with the given max history size.
    pub fn new(max_history: usize) -> Self {
        Self {
            subscribers: Arc::new(Mutex::new(HashMap::new())),
            history: Arc::new(Mutex::new(Vec::new())),
            max_history,
        }
    }

    /// Subscribe to an event type, returning a broadcast receiver.
    pub async fn subscribe(&self, event_type: &str) -> broadcast::Receiver<DaemonEvent> {
        let (tx, rx) = broadcast::channel(100);
        let mut subs = self.subscribers.lock().await;
        subs.entry(event_type.to_string())
            .or_default()
            .push(tx);
        rx
    }

    /// Publish an event to all subscribers.
    pub async fn publish(&self, event_type: &str, data: Value) {
        let event = DaemonEvent::new(event_type, data);

        // Store in history
        {
            let mut history = self.history.lock().await;
            history.push(event.clone());
            let len = history.len();
            if len > self.max_history {
                let drain_count = len - self.max_history;
                history.drain(0..drain_count);
            }
        }

        // Broadcast to subscribers (both wildcard and type-specific)
        let subs = self.subscribers.lock().await;

        // Notify wildcard listeners
        if let Some(wildcards) = subs.get("*") {
            for sender in wildcards {
                let _ = sender.send(event.clone());
            }
        }

        // Notify type-specific listeners
        if let Some(handlers) = subs.get(event_type) {
            for sender in handlers {
                let _ = sender.send(event.clone());
            }
        }
    }

    /// Get recent events, optionally filtered by type.
    pub async fn get_history(&self, event_type: Option<&str>, limit: usize) -> Vec<DaemonEvent> {
        let history = self.history.lock().await;
        let filtered: Vec<_> = match event_type {
            Some(et) => history.iter().filter(|e| e.event_type == et).cloned().collect(),
            None => history.clone(),
        };
        filtered.into_iter().rev().take(limit).rev().collect()
    }

    /// All registered event type names.
    pub async fn event_types(&self) -> Vec<String> {
        let subs = self.subscribers.lock().await;
        subs.keys().cloned().collect()
    }
}

// ─── SSE Server ────────────────────────────────────────────────────────────

/// Async HTTP server that streams DaemonEvent payloads as text/event-stream.
///
/// Serves `GET /events` to any connected client (browser, curl, etc.).
/// Clients may optionally filter by `?type=<event_type>` query param.
pub struct SseServer {
    port: u16,
    event_bus: Arc<EventBus>,
}

impl SseServer {
    /// Create a new SSE server on the given port.
    pub fn new(port: u16, event_bus: Arc<EventBus>) -> Self {
        Self { port, event_bus }
    }

    /// Start the SSE server and run until shutdown.
    pub async fn serve(&self) -> DaemonResult<()> {
        let event_bus = self.event_bus.clone();

        let events_route = warp::path("events")
            .and(warp::get())
            .and(warp::query::<HashMap<String, String>>())
            .and(with_event_bus(event_bus.clone()))
            .and_then(handle_events);

        let health_route = warp::path("health")
            .and(warp::get())
            .and(warp::query::<HashMap<String, String>>())
            .and(with_event_bus(event_bus.clone()))
            .and_then(handle_health);

        let routes = events_route
            .or(health_route)
            .with(
                warp::cors()
                    .allow_any_origin()
                    .allow_headers(vec!["*"])
                    .allow_methods(vec!["GET"]),
            );

        let addr: std::net::SocketAddr = ([0, 0, 0, 0], self.port).into();
        tracing::info!("[SseServer] listening on http://{}/events", addr);
        warp::serve(routes).run(addr).await;
        Ok(())
    }
}

fn with_event_bus(
    eb: Arc<EventBus>,
) -> impl Filter<Extract = (Arc<EventBus>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || eb.clone())
}

async fn handle_events(
    query: HashMap<String, String>,
    event_bus: Arc<EventBus>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let filter_type = query.get("type").map(|s| s.as_str()).unwrap_or("*");

    let rx = event_bus.subscribe(filter_type).await;

    let stream = BroadcastStream::new(rx).map(|result| {
        match result {
            Ok(event) => {
                let sse_data = event.to_sse();
                Ok::<_, std::convert::Infallible>(warp::sse::Event::default().data(sse_data))
            }
            Err(e) => {
                tracing::warn!("broadcast error: {}", e);
                Ok::<_, std::convert::Infallible>(warp::sse::Event::default().data("event: error\ndata: broadcast error\n\n"))
            }
        }
    });

    Ok(warp::sse::reply(stream))
}

async fn handle_health(
    _query: HashMap<String, String>,
    event_bus: Arc<EventBus>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let types = event_bus.event_types().await;
    Ok(warp::reply::json(&serde_json::json!({
        "status": "ok",
        "event_types": types.len(),
    })))
}

// ─── Utility ───────────────────────────────────────────────────────────────

fn current_timestamp() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_bus_publish_subscribe() {
        let bus = Arc::new(EventBus::new(50));
        let mut rx = bus.subscribe("test_event").await;

        bus.publish("test_event", serde_json::json!({"key": "value"})).await;

        let received = rx.recv().await.unwrap();
        assert_eq!(received.event_type, "test_event");
        assert_eq!(received.data["key"], "value");
    }

    #[tokio::test]
    async fn test_event_bus_wildcard() {
        let bus = Arc::new(EventBus::new(50));
        let mut rx = bus.subscribe("*").await;

        bus.publish("any_event", serde_json::json!({"foo": "bar"})).await;

        let received = rx.recv().await.unwrap();
        assert_eq!(received.event_type, "any_event");
    }

    #[tokio::test]
    async fn test_event_bus_history() {
        let bus = Arc::new(EventBus::new(10));

        bus.publish("a", serde_json::json!({"n": 1})).await;
        bus.publish("b", serde_json::json!({"n": 2})).await;
        bus.publish("a", serde_json::json!({"n": 3})).await;

        let all = bus.get_history(None, 50).await;
        assert_eq!(all.len(), 3);

        let filtered = bus.get_history(Some("a"), 50).await;
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_daemon_event_to_sse() {
        let event = DaemonEvent::new("cycle_start", serde_json::json!({"cycle": 1}));
        let sse = event.to_sse();
        assert!(sse.contains("event: cycle_start\n"));
        assert!(sse.contains("data: {"));
        assert!(sse.ends_with("\n\n"));
    }
}
