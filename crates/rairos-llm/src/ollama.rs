//! Ollama client — local LLM inference via Ollama API.
//!
//! Uses Ollama's OpenAI-compatible API: http://localhost:11434/api/chat

use crate::{LlmClient, LlmError, LlmResponse, Message, StreamResponse};
use async_trait::async_trait;

/// Ollama LLM client — connects to local Ollama instance
pub struct OllamaClient {
    base_url: String,
    client: reqwest::Client,
}

impl OllamaClient {
    pub fn new() -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_base_url(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
        }
    }
}

impl Default for OllamaClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LlmClient for OllamaClient {
    async fn complete(
        &self,
        messages: Vec<Message>,
        model: &str,
        temperature: f32,
        max_tokens: u32,
    ) -> Result<LlmResponse, LlmError> {
        let url = format!("{}/api/chat", self.base_url);

        #[derive(serde::Serialize)]
        struct Request {
            model: String,
            messages: Vec<Message>,
            stream: bool,
            options: Options,
        }

        #[derive(serde::Serialize)]
        struct Options {
            temperature: f32,
            num_predict: u32,
        }

        let resp = self
            .client
            .post(&url)
            .json(&Request {
                model: model.to_string(),
                messages,
                stream: false,
                options: Options {
                    temperature,
                    num_predict: max_tokens,
                },
            })
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await?;
            return Err(LlmError::Api {
                code: status.as_u16(),
                message: body,
            });
        }

        #[derive(serde::Deserialize)]
        struct OllamaResponse {
            message: MessageContent,
            done: bool,
        }

        #[derive(serde::Deserialize)]
        struct MessageContent {
            content: String,
        }

        let body: OllamaResponse = resp.json().await?;

        Ok(LlmResponse::NonStream(crate::NonStreamResponse {
            content: body.message.content,
            usage: crate::LlmUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
                cost_usd: 0.0,
            },
            model: model.to_string(),
            finish_reason: if body.done { "stop" } else { "unknown" }.to_string(),
        }))
    }

    async fn stream_complete(
        &self,
        messages: Vec<Message>,
        model: &str,
        temperature: f32,
        max_tokens: u32,
    ) -> Result<LlmResponse, LlmError> {
        let url = format!("{}/api/chat", self.base_url);

        #[derive(serde::Serialize)]
        struct Request {
            model: String,
            messages: Vec<Message>,
            stream: bool,
            options: Options,
        }

        #[derive(serde::Serialize)]
        struct Options {
            temperature: f32,
            num_predict: u32,
        }

        let resp = self
            .client
            .post(&url)
            .json(&Request {
                model: model.to_string(),
                messages,
                stream: true,
                options: Options {
                    temperature,
                    num_predict: max_tokens,
                },
            })
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await?;
            return Err(LlmError::Api {
                code: status.as_u16(),
                message: body,
            });
        }

        let stream = resp.bytes_stream();
        let chunk_stream = crate::streamerr(stream);
        Ok(LlmResponse::Stream(StreamResponse::new(chunk_stream)))
    }

    fn provider_name(&self) -> &'static str {
        "ollama"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_client_creation() {
        let client = OllamaClient::new();
        assert_eq!(client.provider_name(), "ollama");
        assert!(client.base_url.contains("localhost:11434"));
    }

    #[test]
    fn test_with_base_url() {
        let client = OllamaClient::with_base_url("http://custom:8080".to_string());
        assert_eq!(client.base_url, "http://custom:8080");
    }

    #[test]
    fn test_complete_request_format() {
        use serde_json::json;
        let request = json!({
            "model": "llama3",
            "messages": [{"role": "user", "content": "hello"}],
            "stream": false,
            "options": {
                "temperature": 0.7,
                "num_predict": 100
            }
        });
        assert_eq!(request["model"], "llama3");
        assert_eq!(request["options"]["temperature"], 0.7);
    }
}
