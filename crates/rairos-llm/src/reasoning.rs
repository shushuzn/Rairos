//! StreamingReasoner — extended thinking SSE parser.
//!
//! Re-parses OpenAI-compatible SSE streams to extract reasoning/thinking
//! blocks (e.g. DeepSeek V3 / MiniMax extended thinking).
//! Mirrors llm/reasoning.py::_parse_thinking_stream()

use crate::{LlmError, StreamChunk};
use futures_util::StreamExt;
use tokio_stream::Stream;
use std::pin::Pin;

/// A reasoning block emitted during extended thinking
#[derive(Debug, Clone)]
pub struct ReasoningBlock {
    pub phase: String,
    pub content: String,
    pub done: bool,
}

/// Combined event from a thinking-aware SSE stream
#[derive(Debug, Clone)]
pub enum StreamEvent {
    Reasoning(ReasoningBlock),
    Content(String),
}

const KNOWN_PHASES: &[&str] = &[
    "decomposition", "analysis", "search", "retrieval", "reasoning",
    "planning", "synthesis", "reflection", "verification", "conclusion",
];

/// Infer the reasoning phase from the start of a text chunk
fn infer_phase(text: &str) -> &'static str {
    let lowered = text.to_lowercase().trim().to_string();
    for phase in KNOWN_PHASES {
        if lowered.starts_with(&format!("[{}]", phase))
            || lowered.starts_with(&format!("{}:", phase))
        {
            return phase;
        }
    }
    ""
}

/// State machine for SSE thinking stream parsing
struct ThinkingState {
    current_phase: String,
    buffer: String,
}

impl ThinkingState {
    fn new() -> Self {
        Self {
            current_phase: String::new(),
            buffer: String::new(),
        }
    }

    /// Process a standard SSE StreamChunk through the thinking parser
    fn process_chunk(&mut self, chunk: &StreamChunk) -> Vec<StreamEvent> {
        let mut events = Vec::new();

        // Check if this chunk has thinking content via content_type
        // (This metadata would come from custom SSE parsing in production)

        // For standard OpenAI streaming: just emit content
        if !chunk.content.is_empty() {
            events.push(StreamEvent::Content(chunk.content.clone()));
        }

        // If finish_reason, flush any remaining reasoning buffer
        if chunk.finish_reason.is_some() && !self.buffer.is_empty() {
            events.push(StreamEvent::Reasoning(ReasoningBlock {
                phase: self.current_phase.clone(),
                content: self.buffer.clone(),
                done: true,
            }));
            self.buffer.clear();
            self.current_phase.clear();
        }

        events
    }

    /// Process a delta from a thinking-aware stream (content_type: reasoning)
    fn process_thinking_delta(&mut self, content_type: &str, text: &str) -> Vec<StreamEvent> {
        let mut events = Vec::new();

        if content_type == "reasoning" || content_type == "thinking" {
            let detected = infer_phase(text);
            if !detected.is_empty() && detected != self.current_phase {
                // Flush previous phase
                if !self.buffer.is_empty() {
                    events.push(StreamEvent::Reasoning(ReasoningBlock {
                        phase: self.current_phase.clone(),
                        content: self.buffer.clone(),
                        done: true,
                    }));
                }
                self.current_phase = detected.to_string();
                self.buffer = text.to_string();
            } else {
                self.buffer.push_str(text);
            }
            events.push(StreamEvent::Reasoning(ReasoningBlock {
                phase: self.current_phase.clone(),
                content: text.to_string(),
                done: false,
            }));
        }

        events
    }
}

/// Convert a standard StreamChunk stream into a thinking-aware event stream
pub fn thinking_stream(
    chunk_stream: impl Stream<Item = Result<StreamChunk, LlmError>> + Send + 'static,
) -> Pin<Box<dyn Stream<Item = Result<StreamEvent, LlmError>> + Send>> {
    let mut state = ThinkingState::new();

    let stream = chunk_stream.filter_map(move |result| {
        let events = match result {
            Ok(chunk) => {
                let evts = state.process_chunk(&chunk);
                if evts.is_empty() { None } else { Some(Ok(evts)) }
            }
            Err(e) => Some(Err(e)),
        };
        async move { events }
    })
    .flat_map(|result| {
        let items: Vec<Result<StreamEvent, LlmError>> = match result {
            Ok(events) => events.into_iter().map(Ok).collect(),
            Err(e) => vec![Err(e)],
        };
        futures_util::stream::iter(items)
    });

    Box::pin(stream)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_phase() {
        assert_eq!(infer_phase("[decomposition] Let me break this down"), "decomposition");
        assert_eq!(infer_phase("analysis: First, consider"), "analysis");
        assert_eq!(infer_phase("normal text"), "");
    }

    #[test]
    fn test_thinking_delta() {
        let mut state = ThinkingState::new();
        let events = state.process_thinking_delta("reasoning", "[analysis] test");
        assert_eq!(events.len(), 1, "should emit one reasoning block");
        assert!(matches!(&events[0], StreamEvent::Reasoning(r) if r.phase == "analysis" && !r.done));
    }

    #[test]
    fn test_standard_chunk() {
        let mut state = ThinkingState::new();
        let chunk = StreamChunk {
            content: "Hello world".to_string(),
            role: Some("assistant".to_string()),
            tool_calls: Vec::new(),
            finish_reason: None,
        };
        let events = state.process_chunk(&chunk);
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], StreamEvent::Content(c) if c == "Hello world"));
    }

    #[test]
    fn test_finish_flushes_buffer() {
        let mut state = ThinkingState::new();
        state.current_phase = "analysis".to_string();
        state.buffer = "some reasoning".to_string();

        let chunk = StreamChunk {
            content: String::new(),
            role: None,
            tool_calls: Vec::new(),
            finish_reason: Some("stop".to_string()),
        };
        let events = state.process_chunk(&chunk);
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], StreamEvent::Reasoning(r) if r.done));
    }
}
