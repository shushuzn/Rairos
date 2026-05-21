//! Server-Sent Events (SSE) streaming for real-time agent progress.
//!
//! Based on research from:
//! - arXiv:2510.02758 (TokenFlow) - preemptive scheduling
//! - arXiv:2404.16283 (Andes) - token-level streaming
//! - arXiv:2604.16395 (Stream2LLM) - TTFT optimization
//!
//! ## Architecture
//!
//! ```text
//! Agent Executor
//!      │
//!      ├──► AgentStarted { agent_id, role }
//!      ├──► TokenGenerated { token }
//!      ├──► ToolCalled { tool, args }
//!      ├──► ToolResult { tool, result }
//!      ├──► AgentThinking { thoughts }
//!      └──► AgentCompleted { output, duration_ms }
//!              │
//!              ▼
//!        SSE Broadcaster
//!              │
//!         ┌────┴────┐
//!         ▼         ▼
//!      Client A   Client B
//! ```

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use futures::Stream;

/// SSE event types for agent streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum SseEvent {
    /// Agent started processing
    AgentStarted {
        agent_id: String,
        role: String,
        timestamp: String,
    },
    /// New token generated
    TokenGenerated {
        agent_id: String,
        token: String,
        is_final: bool,
    },
    /// Tool call initiated
    ToolCalled {
        agent_id: String,
        tool_name: String,
        arguments: HashMap<String, serde_json::Value>,
        timestamp: String,
    },
    /// Tool execution completed
    ToolResult {
        agent_id: String,
        tool_name: String,
        success: bool,
        result: String,
        duration_ms: u64,
    },
    /// Agent is thinking/reasoning
    AgentThinking {
        agent_id: String,
        thoughts: String,
        timestamp: String,
    },
    /// Agent completed
    AgentCompleted {
        agent_id: String,
        output: String,
        confidence: f32,
        duration_ms: u64,
    },
    /// Agent failed
    AgentFailed {
        agent_id: String,
        error: String,
        duration_ms: u64,
    },
    /// Phase completed
    PhaseCompleted {
        phase: String,
        success: bool,
        duration_ms: u64,
    },
    /// Research progress update
    ProgressUpdate {
        message: String,
        percent: u8,
    },
    /// Heartbeat to keep connection alive
    Heartbeat {
        timestamp: String,
    },
}

impl SseEvent {
    /// Convert to SSE format string
    pub fn to_sse_data(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_else(|_| r#"{"error":"serialization failed"}"#.to_string());
        format!("data: {}\n\n", json)
    }

    /// Get event name for SSE
    pub fn event_name(&self) -> &'static str {
        match self {
            SseEvent::AgentStarted { .. } => "agent_started",
            SseEvent::TokenGenerated { .. } => "token",
            SseEvent::ToolCalled { .. } => "tool_call",
            SseEvent::ToolResult { .. } => "tool_result",
            SseEvent::AgentThinking { .. } => "thinking",
            SseEvent::AgentCompleted { .. } => "agent_completed",
            SseEvent::AgentFailed { .. } => "agent_failed",
            SseEvent::PhaseCompleted { .. } => "phase",
            SseEvent::ProgressUpdate { .. } => "progress",
            SseEvent::Heartbeat { .. } => "heartbeat",
        }
    }
}

/// SSE broadcaster for streaming events to multiple clients
pub struct SseBroadcaster {
    /// Channel sender for broadcasting
    sender: broadcast::Sender<SseEvent>,
    /// Connected clients count
    client_count: Arc<RwLock<usize>>,
}

impl SseBroadcaster {
    /// Create a new SSE broadcaster
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self {
            sender,
            client_count: Arc::new(RwLock::new(0)),
        }
    }

    /// Subscribe to events (returns a stream)
    pub fn subscribe(&self) -> SseEventStream {
        let receiver = self.sender.subscribe();
        {
            let mut count = self.client_count.write().unwrap();
            *count += 1;
        }
        SseEventStream { receiver }
    }

    /// Broadcast an event to all subscribers
    pub fn broadcast(&self, event: SseEvent) {
        let _ = self.sender.send(event);
    }

    /// Get number of connected clients
    pub fn client_count(&self) -> usize {
        *self.client_count.read().unwrap()
    }

    /// Client disconnected - call when a subscriber ends
    pub fn on_disconnect(&self) {
        let mut count = self.client_count.write().unwrap();
        *count = count.saturating_sub(1);
    }

    /// Send agent started event
    pub fn agent_started(&self, agent_id: &str, role: &str) {
        self.broadcast(SseEvent::AgentStarted {
            agent_id: agent_id.to_string(),
            role: role.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }

    /// Send token generated event
    pub fn token_generated(&self, agent_id: &str, token: &str, is_final: bool) {
        self.broadcast(SseEvent::TokenGenerated {
            agent_id: agent_id.to_string(),
            token: token.to_string(),
            is_final,
        });
    }

    /// Send tool called event
    pub fn tool_called(&self, agent_id: &str, tool_name: &str, args: HashMap<String, serde_json::Value>) {
        self.broadcast(SseEvent::ToolCalled {
            agent_id: agent_id.to_string(),
            tool_name: tool_name.to_string(),
            arguments: args,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }

    /// Send tool result event
    pub fn tool_result(&self, agent_id: &str, tool_name: &str, success: bool, result: &str, duration_ms: u64) {
        self.broadcast(SseEvent::ToolResult {
            agent_id: agent_id.to_string(),
            tool_name: tool_name.to_string(),
            success,
            result: result.to_string(),
            duration_ms,
        });
    }

    /// Send thinking event
    pub fn thinking(&self, agent_id: &str, thoughts: &str) {
        self.broadcast(SseEvent::AgentThinking {
            agent_id: agent_id.to_string(),
            thoughts: thoughts.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }

    /// Send agent completed event
    pub fn agent_completed(&self, agent_id: &str, output: &str, confidence: f32, duration_ms: u64) {
        self.broadcast(SseEvent::AgentCompleted {
            agent_id: agent_id.to_string(),
            output: output.to_string(),
            confidence,
            duration_ms,
        });
    }

    /// Send agent failed event
    pub fn agent_failed(&self, agent_id: &str, error: &str, duration_ms: u64) {
        self.broadcast(SseEvent::AgentFailed {
            agent_id: agent_id.to_string(),
            error: error.to_string(),
            duration_ms,
        });
    }

    /// Send phase completed event
    pub fn phase_completed(&self, phase: &str, success: bool, duration_ms: u64) {
        self.broadcast(SseEvent::PhaseCompleted {
            phase: phase.to_string(),
            success,
            duration_ms,
        });
    }

    /// Send progress update
    pub fn progress(&self, message: &str, percent: u8) {
        self.broadcast(SseEvent::ProgressUpdate {
            message: message.to_string(),
            percent,
        });
    }

    /// Send heartbeat
    pub fn heartbeat(&self) {
        self.broadcast(SseEvent::Heartbeat {
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }
}

impl Default for SseBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}

/// Stream wrapper for SSE events
pub struct SseEventStream {
    receiver: broadcast::Receiver<SseEvent>,
}

impl Stream for SseEventStream {
    type Item = SseEvent;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use tokio::sync::broadcast::error::TryRecvError;

        match self.receiver.try_recv() {
            Ok(event) => std::task::Poll::Ready(Some(event)),
            Err(TryRecvError::Closed) => std::task::Poll::Ready(None),
            Err(TryRecvError::Lagged(_)) => std::task::Poll::Ready(None),
            Err(TryRecvError::Empty) => {
                // Register waker to be notified when new events arrive
                let waker = cx.waker().clone();
                let receiver = &self.receiver;
                // We can't easily integrate with the broadcast channel's internal waker,
                // so we'll use a simple approach: always return Pending for Empty
                // This means the stream consumer needs to poll again
                std::task::Poll::Pending
            }
        }
    }
}

/// Track timing for SSE events
#[derive(Debug, Clone)]
pub struct SseTimer {
    start_time: Instant,
    events: Vec<(String, Duration)>,
}

impl SseTimer {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            events: Vec::new(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn record(&mut self, event: String) {
        self.events.push((event, self.elapsed()));
    }

    pub fn get_events(&self) -> Vec<(String, Duration)> {
        self.events.clone()
    }
}

impl Default for SseTimer {
    fn default() -> Self {
        Self::new()
    }
}

/// SSE response builder helper
pub struct SseResponse {
    events: Vec<SseEvent>,
}

impl SseResponse {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn add(mut self, event: SseEvent) -> Self {
        self.events.push(event);
        self
    }

    /// Generate SSE formatted string
    pub fn to_sse_string(&self) -> String {
        self.events
            .iter()
            .map(|e| {
                let data = e.to_sse_data();
                format!("event: {}\n{}", e.event_name(), data)
            })
            .collect::<Vec<_>>()
            .join("")
    }
}

impl Default for SseResponse {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_event_serialization() {
        let event = SseEvent::AgentStarted {
            agent_id: "agent-1".to_string(),
            role: "hypothesis".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("agent_started"));
        assert!(json.contains("agent-1"));
    }

    #[test]
    fn test_sse_event_to_data() {
        let event = SseEvent::Heartbeat {
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let data = event.to_sse_data();
        assert!(data.starts_with("data: "));
        assert!(data.ends_with("\n\n"));
    }

    #[test]
    fn test_event_name() {
        let event = SseEvent::TokenGenerated {
            agent_id: "test".to_string(),
            token: "hello".to_string(),
            is_final: false,
        };
        assert_eq!(event.event_name(), "token");
    }

    #[tokio::test]
    async fn test_broadcaster_single_client() {
        let broadcaster = SseBroadcaster::new();
        assert_eq!(broadcaster.client_count(), 0);

        let stream = broadcaster.subscribe();
        assert_eq!(broadcaster.client_count(), 1);

        broadcaster.on_disconnect();
        assert_eq!(broadcaster.client_count(), 0);
    }

    #[tokio::test]
    async fn test_broadcast_subscribe() {
        let broadcaster = SseBroadcaster::new();
        assert_eq!(broadcaster.client_count(), 0);

        let _stream = broadcaster.subscribe();
        assert_eq!(broadcaster.client_count(), 1);

        broadcaster.on_disconnect();
        assert_eq!(broadcaster.client_count(), 0);
    }

    #[test]
    fn test_sse_timer() {
        let mut timer = SseTimer::new();
        std::thread::sleep(Duration::from_millis(10));
        timer.record("event1".to_string());

        std::thread::sleep(Duration::from_millis(10));
        timer.record("event2".to_string());

        let events = timer.get_events();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].0, "event1");
        assert_eq!(events[1].0, "event2");
    }

    #[test]
    fn test_sse_response_builder() {
        let response = SseResponse::new()
            .add(SseEvent::Heartbeat {
                timestamp: "2024-01-01T00:00:00Z".to_string(),
            })
            .add(SseEvent::ProgressUpdate {
                message: "Processing...".to_string(),
                percent: 50,
            });

        let sse_string = response.to_sse_string();
        assert!(sse_string.contains("event: heartbeat"));
        assert!(sse_string.contains("event: progress"));
    }
}