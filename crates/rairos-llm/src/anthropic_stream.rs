//! Anthropic SSE stream parser — converts Anthropic streaming API events
//! into the standard StreamChunk format used by rairos-llm.
//!
//! Anthropic SSE format:
//!   event: message_start
//!   data: {"type": "message_start", "message": {...}}
//!
//!   event: content_block_delta
//!   data: {"type": "content_block_delta", "delta": {"type": "text_delta", "text": "hello"}}
//!
//!   event: message_stop
//!   data: {"type": "message_stop"}

use crate::{LlmError, StreamChunk};
use bytes::Bytes;
use futures_util::StreamExt;
use tokio_stream::Stream;
use std::pin::Pin;

/// Convert an Anthropic byte stream into a StreamChunk stream
pub fn anthropic_stream_to_chunks(
    byte_stream: impl Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
) -> Pin<Box<dyn Stream<Item = Result<StreamChunk, LlmError>> + Send>> {
    let stream = byte_stream
        .map(|result| match result {
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes);
                parse_anthropic_sse(&text)
            }
            Err(e) => vec![Err(LlmError::Http(e))],
        })
        .flat_map(|items| futures_util::stream::iter(items));

    Box::pin(stream)
}

/// Parse Anthropic SSE text lines into StreamChunks
fn parse_anthropic_sse(text: &str) -> Vec<Result<StreamChunk, LlmError>> {
    let mut results = Vec::new();
    let mut event_type = String::new();
    let mut data = String::new();

    for line in text.lines() {
        if let Some(val) = line.strip_prefix("event: ") {
            event_type = val.trim().to_string();
        } else if let Some(val) = line.strip_prefix("data: ") {
            data = val.trim().to_string();
        } else if line.trim().is_empty() && !data.is_empty() {
            // End of an SSE event — process it
            process_anthropic_event(&event_type, &data, &mut results);
            event_type.clear();
            data.clear();
        }
    }

    // Handle trailing data
    if !data.is_empty() {
        process_anthropic_event(&event_type, &data, &mut results);
    }

    results
}

fn process_anthropic_event(
    event_type: &str,
    data: &str,
    results: &mut Vec<Result<StreamChunk, LlmError>>,
) {
    match event_type {
        "content_block_delta" | "content_block_start" => {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(data) {
                let text = val["delta"]["text"]
                    .as_str()
                    .or_else(|| val["delta"]["partial_json"].as_str())
                    .unwrap_or("")
                    .to_string();
                if !text.is_empty() {
                    results.push(Ok(StreamChunk {
                        content: text,
                        role: Some("assistant".to_string()),
                        tool_calls: Vec::new(),
                        finish_reason: None,
                    }));
                }
            }
        }
        "message_stop" => {
            results.push(Ok(StreamChunk {
                content: String::new(),
                role: None,
                tool_calls: Vec::new(),
                finish_reason: Some("stop".to_string()),
            }));
        }
        "message_start" | "message_delta" | "ping" => {
            // Ignore — these carry metadata, not content
        }
        _ => {
            // Unknown event — try parsing as generic SSE data
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(data) {
                if let Some(text) = val["delta"]["text"].as_str() {
                    if !text.is_empty() {
                        results.push(Ok(StreamChunk {
                            content: text.to_string(),
                            role: Some("assistant".to_string()),
                            tool_calls: Vec::new(),
                            finish_reason: None,
                        }));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_text_delta() {
        let sse = "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}\n\n";
        let results = parse_anthropic_sse(sse);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
        assert_eq!(results[0].as_ref().unwrap().content, "Hello");
    }

    #[test]
    fn test_parse_message_stop() {
        let sse = "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n";
        let results = parse_anthropic_sse(sse);
        assert_eq!(results.len(), 1);
        let chunk = results[0].as_ref().unwrap();
        assert_eq!(chunk.finish_reason.as_deref(), Some("stop"));
    }

    #[test]
    fn test_parse_multiple_chunks() {
        let sse = "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}\n\nevent: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\" world\"}}\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}\n\n";
        let results = parse_anthropic_sse(sse);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].as_ref().unwrap().content, "Hello");
        assert_eq!(results[1].as_ref().unwrap().content, " world");
    }

    #[test]
    fn test_ignores_message_start() {
        let sse = "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_123\"}}\n\nevent: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"content\"}}\n\n";
        let results = parse_anthropic_sse(sse);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_ref().unwrap().content, "content");
    }
}
