//! Rairos LLM — LLM client wrappers and research intelligence
//!
//! Provides unified interface to multiple LLM providers.
//! Replaces: llm/client.py, llm/citation_chain.py, llm/gap_detector.py

pub mod anthropic_stream;
pub mod briefing;
pub mod cache;
pub mod citation_chain;
pub mod client_async;
pub mod gap_detector;
pub mod impact;
pub mod ollama;
pub mod paper_analyzer;
pub mod reasoning;
pub mod retry;

use rairos_core::Paper;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum LlmError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("API error: {code} {message}")]
    Api { code: u16, message: String },

    #[error("Rate limited")]
    RateLimited,

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("No API key configured")]
    NoApiKey,
}

// ============================================================================
// LLM Credentials Resolution
// ============================================================================

/// API key and base URL resolved from all known sources.
/// Priority: explicit args > MINIMAX_CN_* > MINIMAX_* > OPENAI_*
#[derive(Debug, Clone)]
pub struct LlmCredentials {
    pub api_key: String,
    pub base_url: String,
}

impl LlmCredentials {
    /// Resolve (base_url, api_key) from all known sources, in priority order.
    ///
    /// Priority: explicit args > MINIMAX_CN_* > MINIMAX_* > OPENAI_*
    ///
    /// Reads from:
    /// - Environment variables (os env takes precedence over .env file)
    /// - ~/.hermes/.env file
    pub fn resolve(explicit_base_url: Option<&str>, explicit_api_key: Option<&str>) -> Self {
        // Read credentials from ~/.hermes/.env (only MINIMAX-related keys)
        let hermes_env = read_hermes_env();

        // Resolve API key with priority: explicit > MINIMAX_CN > MINIMAX > OPENAI
        let resolved_key = explicit_api_key
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| {
                std::env::var("OPENAI_API_KEY")
                    .ok()
                    .filter(|s| !s.is_empty())
            })
            .or_else(|| {
                std::env::var("MINIMAX_CN_API_KEY")
                    .ok()
                    .filter(|s| !s.is_empty())
                    .or_else(|| hermes_env.get("MINIMAX_CN_API_KEY").cloned())
            })
            .or_else(|| {
                std::env::var("MINIMAX_API_KEY")
                    .ok()
                    .filter(|s| !s.is_empty())
            })
            .unwrap_or_default();

        // Resolve base URL with priority: explicit > MINIMAX_CN > MINIMAX > default
        let default_openai = "https://api.openai.com/v1";
        let resolved_url = explicit_base_url
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty() && s != default_openai)
            .or_else(|| {
                std::env::var("MINIMAX_CN_BASE_URL")
                    .ok()
                    .filter(|s| !s.is_empty())
                    .or_else(|| hermes_env.get("MINIMAX_CN_BASE_URL").cloned())
            })
            .or_else(|| {
                std::env::var("MINIMAX_BASE_URL")
                    .ok()
                    .filter(|s| !s.is_empty())
            })
            .or_else(|| {
                std::env::var("OPENAI_BASE_URL")
                    .ok()
                    .filter(|s| !s.is_empty())
            })
            .unwrap_or_else(|| "https://api.minimaxi.com/v1".to_string());

        Self {
            api_key: resolved_key,
            base_url: resolved_url,
        }
    }
}

/// Read MINIMAX-related credentials from ~/.hermes/.env
fn read_hermes_env() -> std::collections::HashMap<String, String> {
    use std::collections::HashMap;
    use std::path::PathBuf;

    let mut result: HashMap<String, String> = HashMap::new();

    let hermes_home: PathBuf = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".hermes");
    let env_file = hermes_home.join(".env");

    if !env_file.exists() {
        return result;
    }

    // Read file content, replacing CRLF with LF
    let text = match std::fs::read_to_string(&env_file) {
        Ok(t) => t.replace("\r\n", "\n").replace("\r", "\n"),
        Err(_) => return result,
    };

    // Only care about MINIMAX-related keys
    let relevant_keys: std::collections::HashSet<&str> = [
        "MINIMAX_CN_API_KEY",
        "MINIMAX_CN_BASE_URL",
        "MINIMAX_API_KEY",
        "MINIMAX_BASE_URL",
    ]
    .iter()
    .cloned()
    .collect();

    for line in text.split('\n') {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || !line.contains('=') {
            continue;
        }

        if let Some((k, v)) = line.split_once('=') {
            let k = k.trim();
            let v = v.trim().trim_matches('"').trim_matches('\'');
            if relevant_keys.contains(k) && !result.contains_key(k) {
                result.insert(k.to_string(), v.to_string());
            }
        }
    }

    result
}

// ============================================================================
// Query Type Classification
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryType {
    Factual,
    Conceptual,
    Comparative,
    Temporal,
    General,
}

impl QueryType {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "factual" => QueryType::Factual,
            "conceptual" => QueryType::Conceptual,
            "comparative" => QueryType::Comparative,
            "temporal" => QueryType::Temporal,
            _ => QueryType::General,
        }
    }
}

#[allow(dead_code)]
pub fn bm25_weight(qt: QueryType) -> f64 {
    match qt {
        QueryType::Factual => 0.65,
        QueryType::Conceptual => 0.20,
        QueryType::Comparative => 0.50,
        QueryType::Temporal => 0.55,
        QueryType::General => 0.40,
    }
}

#[allow(dead_code)]
pub fn mmr_lambda(qt: QueryType) -> f64 {
    match qt {
        QueryType::Factual => 0.8,
        QueryType::Conceptual => 0.6,
        QueryType::Comparative => 0.5,
        QueryType::Temporal => 0.7,
        QueryType::General => 0.6,
    }
}

// ============================================================================
// LLM Provider Traits
// ============================================================================

/// Cost tracking for LLM calls
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LlmUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub cost_usd: f64,
}

impl LlmUsage {
    pub fn openai_gpt4o(prompt: u32, completion: u32) -> Self {
        // GPT-4o: $5/1M prompt, $15/1M completion
        Self {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: prompt + completion,
            cost_usd: (prompt as f64 * 5.0 / 1_000_000.0)
                + (completion as f64 * 15.0 / 1_000_000.0),
        }
    }

    pub fn openai_o3_mini(prompt: u32, completion: u32) -> Self {
        // o3-mini: $1.1/1M prompt, $4.4/1M completion
        Self {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: prompt + completion,
            cost_usd: (prompt as f64 * 1.1 / 1_000_000.0) + (completion as f64 * 4.4 / 1_000_000.0),
        }
    }

    pub fn anthropic_sonnet4(prompt: u32, completion: u32) -> Self {
        // Claude Sonnet 4: $3/1M input, $15/1M output
        Self {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: prompt + completion,
            cost_usd: (prompt as f64 * 3.0 / 1_000_000.0)
                + (completion as f64 * 15.0 / 1_000_000.0),
        }
    }
}

/// A single message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String, // "system", "user", "assistant"
    pub content: String,
}

/// Response from an LLM call
#[derive(Debug, Clone)]
pub enum LlmResponse {
    /// Non-streaming response
    NonStream(NonStreamResponse),
    /// Streaming response — yields chunks as they arrive
    Stream(StreamResponse),
}

impl LlmResponse {
    /// Returns a reference to the content (panics if streaming).
    pub fn content(&self) -> &str {
        match self {
            LlmResponse::NonStream(r) => &r.content,
            LlmResponse::Stream(_) => panic!("content() not available on streaming response"),
        }
    }

    /// Returns a reference to the usage (panics if streaming).
    pub fn usage(&self) -> &LlmUsage {
        match self {
            LlmResponse::NonStream(r) => &r.usage,
            LlmResponse::Stream(_) => panic!("usage() not available on streaming response"),
        }
    }

    /// Returns a reference to the model name (panics if streaming).
    pub fn model(&self) -> &str {
        match self {
            LlmResponse::NonStream(r) => &r.model,
            LlmResponse::Stream(_) => panic!("model() not available on streaming response"),
        }
    }

    /// Unwrap the non-streaming response (panics if streaming).
    pub fn unwrap_nonstream(self) -> NonStreamResponse {
        match self {
            LlmResponse::NonStream(r) => r,
            LlmResponse::Stream(_) => panic!("unwrap_nonstream called on streaming response"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NonStreamResponse {
    pub content: String,
    pub usage: LlmUsage,
    pub model: String,
    pub finish_reason: String,
}

/// A single chunk in a streaming response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub content: String,
    pub role: Option<String>,
    pub tool_calls: Vec<StreamToolCall>,
    pub finish_reason: Option<String>,
}

/// Tool call delta within a stream chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamToolCall {
    pub index: usize,
    pub id: Option<String>,
    pub name: Option<String>,
    pub arguments: String,
}

/// Streaming response iterator
pub struct StreamResponse {
    chunks:
        std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<StreamChunk, LlmError>> + Send>>,
}

impl std::fmt::Debug for StreamResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "StreamResponse {{ ... }}")
    }
}

impl Clone for StreamResponse {
    fn clone(&self) -> Self {
        panic!("StreamResponse cannot be cloned")
    }
}

impl StreamResponse {
    pub fn new(
        chunks: impl tokio_stream::Stream<Item = Result<StreamChunk, LlmError>> + Send + 'static,
    ) -> Self {
        Self {
            chunks: Box::pin(chunks),
        }
    }

    pub fn into_inner(
        self,
    ) -> impl tokio_stream::Stream<Item = Result<StreamChunk, LlmError>> + Send {
        self.chunks
    }
}

/// Trait for LLM providers
#[async_trait::async_trait]
pub trait LlmClient: Send + Sync {
    /// Send a completion request (non-streaming)
    async fn complete(
        &self,
        messages: Vec<Message>,
        model: &str,
        temperature: f32,
        max_tokens: u32,
    ) -> Result<LlmResponse, LlmError>;

    /// Send a streaming completion request
    async fn stream_complete(
        &self,
        messages: Vec<Message>,
        model: &str,
        temperature: f32,
        max_tokens: u32,
    ) -> Result<LlmResponse, LlmError>;

    /// Get provider name
    fn provider_name(&self) -> &'static str;
}

// ============================================================================
// SSE Streaming Helpers
// ============================================================================

/// Parse a single SSE data line into a StreamChunk.
/// Handles: data: {...} and data: [DONE]
fn parse_sse_event(line: &str) -> Option<StreamChunk> {
    let line = line.trim();
    if !line.starts_with("data:") {
        return None;
    }

    let data = line["data:".len()..].trim();
    if data == "[DONE]" {
        return None; // End of stream
    }

    #[derive(Deserialize)]
    struct SseChunk {
        choices: Vec<SseChoice>,
    }

    #[derive(Deserialize)]
    struct SseChoice {
        delta: SseDelta,
        finish_reason: Option<String>,
    }

    #[derive(Deserialize, Default)]
    struct SseDelta {
        content: Option<String>,
        role: Option<String>,
        tool_calls: Option<Vec<SseToolCall>>,
    }

    #[derive(Deserialize, Default)]
    struct SseToolCall {
        index: Option<usize>,
        id: Option<String>,
        function: Option<SseFunction>,
    }

    #[derive(Deserialize, Default)]
    struct SseFunction {
        name: Option<String>,
        arguments: Option<String>,
    }

    let chunk: SseChunk = match serde_json::from_str(data) {
        Ok(c) => c,
        Err(_) => return None,
    };

    let choice = chunk.choices.into_iter().next()?;

    let mut tool_calls = Vec::new();
    if let Some(tcs) = choice.delta.tool_calls {
        for tc in tcs {
            tool_calls.push(StreamToolCall {
                index: tc.index.unwrap_or(0),
                id: tc.id,
                name: tc.function.as_ref().and_then(|f| f.name.clone()),
                arguments: tc
                    .function
                    .as_ref()
                    .and_then(|f| f.arguments.clone())
                    .unwrap_or_default(),
            });
        }
    }

    Some(StreamChunk {
        content: choice.delta.content.unwrap_or_default(),
        role: choice.delta.role,
        tool_calls,
        finish_reason: choice.finish_reason,
    })
}

/// Convert a byte stream into a stream of SSE-parsed StreamChunks
fn streamerr(
    stream: impl tokio_stream::Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
) -> impl tokio_stream::Stream<Item = Result<StreamChunk, LlmError>> + Send + 'static {
    use futures_util::StreamExt;

    let stream = stream.map(|result| {
        let bytes = match result {
            Ok(b) => b,
            Err(e) => return vec![Err(LlmError::Http(e))],
        };
        let text = String::from_utf8_lossy(&bytes).to_string();
        text.lines()
            .filter_map(|line| parse_sse_event(line).map(Ok))
            .collect::<Vec<_>>()
    });

    stream.flat_map(futures_util::stream::iter)
}

pub struct OpenAiClient {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl OpenAiClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.openai.com/v1".to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_base_url(api_key: String, base_url: String) -> Self {
        Self {
            api_key,
            base_url,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl LlmClient for OpenAiClient {
    async fn complete(
        &self,
        messages: Vec<Message>,
        model: &str,
        temperature: f32,
        max_tokens: u32,
    ) -> Result<LlmResponse, LlmError> {
        let url = format!("{}/chat/completions", self.base_url);

        #[derive(Serialize)]
        struct Request {
            model: String,
            messages: Vec<Message>,
            temperature: f32,
            max_tokens: u32,
        }

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&Request {
                model: model.to_string(),
                messages,
                temperature,
                max_tokens,
            })
            .send()
            .await?;

        let status = resp.status();
        if status.as_u16() == 429 {
            return Err(LlmError::RateLimited);
        }
        if !status.is_success() {
            let body = resp.text().await?;
            return Err(LlmError::Api {
                code: status.as_u16(),
                message: body,
            });
        }

        #[derive(Deserialize)]
        struct Response {
            choices: Vec<Choice>,
            usage: Usage,
            model: String,
            finish_reason: String,
        }

        #[derive(Deserialize)]
        struct Choice {
            message: Message,
        }

        #[derive(Deserialize)]
        struct Usage {
            prompt_tokens: u32,
            completion_tokens: u32,
        }

        let data: Response = resp.json().await?;

        let usage = if model.contains("gpt-4o") || model.contains("gpt-4-turbo") {
            LlmUsage::openai_gpt4o(data.usage.prompt_tokens, data.usage.completion_tokens)
        } else {
            LlmUsage::openai_o3_mini(data.usage.prompt_tokens, data.usage.completion_tokens)
        };

        Ok(LlmResponse::NonStream(NonStreamResponse {
            content: data
                .choices
                .into_iter()
                .next()
                .map(|c| c.message.content)
                .unwrap_or_default(),
            usage,
            model: data.model,
            finish_reason: data.finish_reason,
        }))
    }

    async fn stream_complete(
        &self,
        messages: Vec<Message>,
        model: &str,
        temperature: f32,
        max_tokens: u32,
    ) -> Result<LlmResponse, LlmError> {
        let url = format!("{}/chat/completions", self.base_url);

        #[derive(Serialize)]
        struct Request {
            model: String,
            messages: Vec<Message>,
            temperature: f32,
            max_tokens: u32,
            stream: bool,
        }

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&Request {
                model: model.to_string(),
                messages,
                temperature,
                max_tokens,
                stream: true,
            })
            .send()
            .await?;

        let status = resp.status();
        if status.as_u16() == 429 {
            return Err(LlmError::RateLimited);
        }
        if !status.is_success() {
            let body = resp.text().await?;
            return Err(LlmError::Api {
                code: status.as_u16(),
                message: body,
            });
        }

        // Create a stream from the SSE response
        let stream = resp.bytes_stream();
        let chunk_stream = streamerr(stream);

        Ok(LlmResponse::Stream(StreamResponse::new(chunk_stream)))
    }

    fn provider_name(&self) -> &'static str {
        "openai"
    }
}

// ============================================================================
// Anthropic Client
// ============================================================================

pub struct AnthropicClient {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl AnthropicClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.anthropic.com/v1".to_string(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl LlmClient for AnthropicClient {
    async fn complete(
        &self,
        messages: Vec<Message>,
        model: &str,
        temperature: f32,
        max_tokens: u32,
    ) -> Result<LlmResponse, LlmError> {
        let url = format!("{}/messages", self.base_url);

        // Anthropic uses a different format
        #[derive(Serialize)]
        struct Request<'a> {
            model: &'a str,
            messages: Vec<Message>,
            max_tokens: u32,
            temperature: f32,
        }

        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&Request {
                model,
                messages,
                temperature,
                max_tokens,
            })
            .send()
            .await?;

        let status = resp.status();
        if status.as_u16() == 429 {
            return Err(LlmError::RateLimited);
        }
        if !status.is_success() {
            let body = resp.text().await?;
            return Err(LlmError::Api {
                code: status.as_u16(),
                message: body,
            });
        }

        #[derive(Deserialize)]
        struct Response {
            content: Vec<ContentBlock>,
            usage: AnthropicUsage,
            model: String,
            stop_reason: String,
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ContentBlock {
            Text { text: String },
            // other types ignored for now
        }

        #[derive(Deserialize)]
        struct AnthropicUsage {
            input_tokens: u32,
            output_tokens: u32,
        }

        let data: Response = resp.json().await?;

        let content = data
            .content
            .into_iter()
            .map(|c| match c {
                ContentBlock::Text { text } => text,
            })
            .collect::<Vec<_>>()
            .join("\n");

        let usage = LlmUsage::anthropic_sonnet4(data.usage.input_tokens, data.usage.output_tokens);

        Ok(LlmResponse::NonStream(NonStreamResponse {
            content,
            usage,
            model: data.model,
            finish_reason: data.stop_reason,
        }))
    }

    async fn stream_complete(
        &self,
        messages: Vec<Message>,
        model: &str,
        temperature: f32,
        max_tokens: u32,
    ) -> Result<LlmResponse, LlmError> {
        let url = format!("{}/messages", self.base_url);

        #[derive(Serialize)]
        struct Request<'a> {
            model: &'a str,
            messages: Vec<Message>,
            max_tokens: u32,
            temperature: f32,
            stream: bool,
        }

        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&Request {
                model,
                messages,
                temperature,
                max_tokens,
                stream: true,
            })
            .send()
            .await?;

        let status = resp.status();
        if status.as_u16() == 429 {
            return Err(LlmError::RateLimited);
        }
        if !status.is_success() {
            let body = resp.text().await?;
            return Err(LlmError::Api {
                code: status.as_u16(),
                message: body,
            });
        }

        // Parse Anthropic SSE stream
        let stream = resp.bytes_stream();
        let chunk_stream = crate::anthropic_stream::anthropic_stream_to_chunks(stream);
        Ok(LlmResponse::Stream(StreamResponse::new(chunk_stream)))
    }

    fn provider_name(&self) -> &'static str {
        "anthropic"
    }
}

// ============================================================================
// Cost Tracker
// ============================================================================

/// Tracks cumulative LLM costs
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CostTracker {
    pub total_cost_usd: f64,
    pub total_tokens: u64,
    pub calls_by_model: HashMap<String, u32>,
    pub calls_by_provider: HashMap<String, u32>,
}

impl CostTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, usage: &LlmUsage, model: &str, provider: &str) {
        self.total_cost_usd += usage.cost_usd;
        self.total_tokens += usage.total_tokens as u64;
        *self.calls_by_model.entry(model.to_string()).or_insert(0) += 1;
        *self
            .calls_by_provider
            .entry(provider.to_string())
            .or_insert(0) += 1;
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn summary(&self) -> String {
        format!(
            "Total cost: {:.4}, Total tokens: {}, Models: {:?}, Providers: {:?}",
            self.total_cost_usd, self.total_tokens, self.calls_by_model, self.calls_by_provider
        )
    }
}

// ============================================================================
// Citation Chain Analyzer
// ============================================================================

/// Analyzes citation relationships between papers
pub struct CitationAnalyzer;

impl CitationAnalyzer {
    /// Find papers that cite a given paper (placeholder - references is count field)
    pub fn find_citing_papers<'a>(_paper_id: &str, _papers: &'a [Paper]) -> Vec<&'a Paper> {
        Vec::new()
    }

    /// Find papers cited by a given paper (placeholder - references is count field)
    pub fn find_referenced_papers<'a>(_paper_id: &str, _papers: &'a [Paper]) -> Vec<&'a Paper> {
        Vec::new()
    }

    /// Build a simple citation graph (placeholder)
    pub fn build_citation_graph(_papers: &[Paper]) -> CitationGraph {
        let graph: HashMap<String, Vec<String>> = HashMap::new();
        CitationGraph { edges: graph }
    }
}

#[derive(Debug)]
pub struct CitationGraph {
    edges: HashMap<String, Vec<String>>,
}

impl CitationGraph {
    pub fn paper_ids(&self) -> Vec<&String> {
        self.edges.keys().collect()
    }

    pub fn references_of(&self, paper_id: &str) -> Option<&Vec<String>> {
        self.edges.get(paper_id)
    }

    pub fn citation_count(&self, paper_id: &str) -> usize {
        self.edges
            .values()
            .filter(|refs| refs.iter().any(|r| r == paper_id))
            .count()
    }
}

// ============================================================================
// Research Gap Detector
// ============================================================================

/// Detects research gaps from paper corpus
pub struct GapDetector;

impl GapDetector {
    /// Simple keyword-based gap detection
    pub fn detect_gaps(papers: &[Paper], keywords: &[&str]) -> Vec<String> {
        let mut gaps = Vec::new();

        for keyword in keywords {
            let has_keyword = papers.iter().any(|p| {
                p.title.to_lowercase().contains(&keyword.to_lowercase())
                    || p.abstract_text
                        .to_lowercase()
                        .contains(&keyword.to_lowercase())
            });

            if !has_keyword {
                gaps.push(format!("No papers found matching keyword: {}", keyword));
            }
        }

        gaps
    }

    /// Find under-explored research areas based on category distribution
    pub fn find_underexplored_areas(papers: &[Paper], threshold: usize) -> Vec<String> {
        let mut category_count: HashMap<&str, usize> = HashMap::new();

        for paper in papers {
            for cat in &paper.categories {
                *category_count.entry(cat).or_insert(0) += 1;
            }
        }

        category_count
            .into_iter()
            .filter(|(_, count)| *count < threshold)
            .map(|(cat, _)| cat.to_string())
            .collect()
    }
}

// ============================================================================
// Gene Pool - Core Types
// ============================================================================

#[allow(dead_code)]
const DEFAULT_LAMBDA: f64 = 0.01;
const DEFAULT_MIN_IMPACT: f64 = 0.1;
#[allow(dead_code)]
const DEFAULT_CONSECUTIVE_CYCLES: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capsule {
    pub capsule_id: String,
    pub archetype: CapsuleArchetype,
    pub trigger_keywords: Vec<String>,
    pub action_gap_type: String,
    pub status: CapsuleStatus,
    pub impact_score: f64,
    pub success_count: i32,
    pub failure_count: i32,
    pub created_at: String,
    pub updated_at: String,
    pub last_used_at: Option<String>,
    pub archived: bool,
    pub archive_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapsuleArchetype {
    pub source_paper_id: Option<String>,
    pub algorithm_fingerprint: Option<String>,
    pub approach_summary: String,
    pub novelty_score: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum CapsuleStatus {
    #[default]
    Active,
    Dormant,
    Archived,
}

impl std::fmt::Display for CapsuleStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CapsuleStatus::Active => write!(f, "active"),
            CapsuleStatus::Dormant => write!(f, "dormant"),
            CapsuleStatus::Archived => write!(f, "archived"),
        }
    }
}

impl Capsule {
    pub fn new(approach_summary: &str, gap_type: &str, keywords: Vec<String>) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            capsule_id: uuid::Uuid::new_v4().to_string(),
            archetype: CapsuleArchetype {
                source_paper_id: None,
                algorithm_fingerprint: None,
                approach_summary: approach_summary.to_string(),
                novelty_score: 0.5,
                confidence: 0.5,
            },
            trigger_keywords: keywords,
            action_gap_type: gap_type.to_string(),
            status: CapsuleStatus::Active,
            impact_score: 1.0,
            success_count: 0,
            failure_count: 0,
            created_at: now.clone(),
            updated_at: now,
            last_used_at: None,
            archived: false,
            archive_reason: None,
        }
    }

    pub fn with_paper(mut self, paper_id: &str) -> Self {
        self.archetype.source_paper_id = Some(paper_id.to_string());
        self
    }

    pub fn with_fingerprint(mut self, fingerprint: &str) -> Self {
        self.archetype.algorithm_fingerprint = Some(fingerprint.to_string());
        self
    }

    pub fn record_success(&mut self) {
        self.success_count += 1;
        self.update_impact();
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.update_impact();
    }

    fn update_impact(&mut self) {
        let total = self.success_count + self.failure_count;
        if total == 0 {
            self.impact_score = 0.5;
            return;
        }
        let success_rate = self.success_count as f64 / total as f64;
        self.impact_score = success_rate * (1.0 + (total as f64).ln().min(5.0));
        self.impact_score = self.impact_score.clamp(0.0, 10.0);
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.success_count + self.failure_count;
        if total == 0 {
            return 0.5;
        }
        self.success_count as f64 / total as f64
    }

    pub fn total_feedback(&self) -> i32 {
        self.success_count + self.failure_count
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapsuleImpact {
    pub capsule_id: String,
    pub impact_score: f64,
    pub age_days: f64,
    pub feedback_count: i32,
    pub success_score: f64,
    pub citation_boost: f64,
    pub inbound_citations: i32,
    pub capsule_trust: f64,
    pub archived: bool,
    pub reason: String,
}

impl Capsule {
    pub fn compute_impact(
        &self,
        lambda: f64,
        inbound_citations: i32,
        citation_boost: f64,
    ) -> CapsuleImpact {
        let created = chrono::DateTime::parse_from_rfc3339(&self.created_at)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());
        let age_days = (chrono::Utc::now() - created).num_days() as f64;

        let decay_factor = (-lambda * age_days).exp();
        let feedback_bonus = (self.total_feedback() as f64 + 1.0).ln().max(1.0);
        let base_impact = self.success_rate() * decay_factor * feedback_bonus * citation_boost;

        let capsule_trust = base_impact * citation_boost;

        CapsuleImpact {
            capsule_id: self.capsule_id.clone(),
            impact_score: base_impact,
            age_days,
            feedback_count: self.total_feedback(),
            success_score: self.success_rate(),
            citation_boost,
            inbound_citations,
            capsule_trust,
            archived: base_impact < DEFAULT_MIN_IMPACT,
            reason: if base_impact < DEFAULT_MIN_IMPACT {
                format!(
                    "Impact {:.4} < threshold {:.4}",
                    base_impact, DEFAULT_MIN_IMPACT
                )
            } else {
                String::new()
            },
        }
    }
}

// ============================================================================
// Gene Pool - Diversity Metrics
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenePoolDiversity {
    pub shannon_index: f64,
    pub shannon_normalized: f64,
    pub capsule_count: usize,
    pub family_counts: HashMap<String, usize>,
    pub gap_type_counts: HashMap<String, usize>,
    pub diversity_score: i32,
    pub underrepresented_families: Vec<String>,
    pub overrepresented_families: Vec<String>,
    pub median_family_count: usize,
    pub family_coverage: f64,
}

pub struct GenePoolDiversityCalculator;

impl GenePoolDiversityCalculator {
    const FAMILY_KEYWORDS: &'static [(&'static str, &'static [&'static str])] = &[
        (
            "attention",
            &[
                "attention",
                "transformer",
                "multi-head",
                "self-attention",
                "cross-attention",
            ],
        ),
        (
            "reinforcement",
            &[
                "rl",
                "reinforcement",
                "policy",
                "reward",
                "agent",
                "DQN",
                "PPO",
                "A3C",
            ],
        ),
        (
            "language_model",
            &[
                "LM",
                "language model",
                "decoder",
                "autoregressive",
                "LLM",
                "GPT",
                "BERT",
            ],
        ),
        (
            "vision",
            &[
                "CNN",
                "convolution",
                "resnet",
                "image",
                "vision",
                "ViT",
                "classification",
            ],
        ),
        (
            "optimization",
            &["optimizer", "Adam", "SGD", "gradient", "loss", "training"],
        ),
        (
            "graph",
            &["GNN", "graph", "node", "edge", "message passing"],
        ),
        (
            "reasoning",
            &[
                "reasoning",
                "chain-of-thought",
                "logical",
                "inference",
                "planning",
            ],
        ),
        (
            "embodied",
            &["embodied", "robotics", "navigation", "control", "motor"],
        ),
    ];

    fn family_of(keywords: &[String]) -> String {
        let kw_set: std::collections::HashSet<String> =
            keywords.iter().map(|k| k.to_lowercase()).collect();
        for (fam, fam_kws) in Self::FAMILY_KEYWORDS {
            if fam_kws.iter().any(|fk| kw_set.contains(*fk)) {
                return fam.to_string();
            }
        }
        "other".to_string()
    }

    pub fn calculate(capsules: &[Capsule]) -> GenePoolDiversity {
        if capsules.is_empty() {
            return GenePoolDiversity {
                shannon_index: 0.0,
                shannon_normalized: 0.0,
                capsule_count: 0,
                family_counts: HashMap::new(),
                gap_type_counts: HashMap::new(),
                diversity_score: 0,
                underrepresented_families: vec![],
                overrepresented_families: vec![],
                median_family_count: 1,
                family_coverage: 0.0,
            };
        }

        let mut family_counts: HashMap<String, usize> = HashMap::new();
        let mut gap_type_counts: HashMap<String, usize> = HashMap::new();

        for cap in capsules {
            let fam = Self::family_of(&cap.trigger_keywords);
            *family_counts.entry(fam).or_insert(0) += 1;
            *gap_type_counts
                .entry(cap.action_gap_type.clone())
                .or_insert(0) += 1;
        }

        let total = capsules.len() as f64;
        let mut shannon = 0.0;
        for count in family_counts.values() {
            let p = *count as f64 / total;
            if p > 0.0 {
                shannon -= p * p.ln();
            }
        }

        let family_count = family_counts.len();
        let max_entropy = (family_count as f64).ln().max(1.0);
        let normalized_shannon = if max_entropy > 0.0 {
            shannon / max_entropy
        } else {
            0.0
        };

        let family_coverage = family_count as f64 / Self::FAMILY_KEYWORDS.len() as f64;
        let diversity_score = ((normalized_shannon * 0.6 + family_coverage * 0.4) * 100.0) as i32;

        let mut sorted_counts: Vec<usize> = family_counts.values().cloned().collect();
        sorted_counts.sort();
        let median_count = sorted_counts
            .get(sorted_counts.len() / 2)
            .copied()
            .unwrap_or(1);

        let underrep: Vec<String> = family_counts
            .iter()
            .filter(|(_, &c)| c < median_count / 10)
            .map(|(f, _)| f.clone())
            .collect();
        let overrep: Vec<String> = family_counts
            .iter()
            .filter(|(_, &c)| c > median_count * 2)
            .map(|(f, _)| f.clone())
            .collect();

        GenePoolDiversity {
            shannon_index: shannon,
            shannon_normalized: normalized_shannon,
            capsule_count: capsules.len(),
            family_counts,
            gap_type_counts,
            diversity_score,
            underrepresented_families: underrep,
            overrepresented_families: overrep,
            median_family_count: median_count,
            family_coverage,
        }
    }
}

// ============================================================================
// Evolution - Feedback & Learning
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackType {
    Positive,
    Negative,
    Neutral,
}

impl std::fmt::Display for FeedbackType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeedbackType::Positive => write!(f, "positive"),
            FeedbackType::Negative => write!(f, "negative"),
            FeedbackType::Neutral => write!(f, "neutral"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalType {
    ChatSuccess,
    ChatFailure,
    RetrievalHit,
    RetrievalMiss,
    SlideQuality,
    SearchSuccess,
}

impl std::fmt::Display for SignalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignalType::ChatSuccess => write!(f, "chat_success"),
            SignalType::ChatFailure => write!(f, "chat_failure"),
            SignalType::RetrievalHit => write!(f, "retrieval_hit"),
            SignalType::RetrievalMiss => write!(f, "retrieval_miss"),
            SignalType::SlideQuality => write!(f, "slide_quality"),
            SignalType::SearchSuccess => write!(f, "search_success"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feedback {
    pub id: String,
    pub feedback_type: FeedbackType,
    pub command: String,
    pub query: String,
    pub paper_ids: Vec<String>,
    pub outcome: String,
    pub score: f64,
    pub note: String,
    pub timestamp: String,
}

impl Feedback {
    pub fn new(
        feedback_type: FeedbackType,
        command: &str,
        query: &str,
        paper_ids: Vec<String>,
        outcome: &str,
        score: f64,
    ) -> Self {
        Self {
            id: format!("fb_{}", chrono::Utc::now().timestamp_millis()),
            feedback_type,
            command: command.to_string(),
            query: query.to_string(),
            paper_ids,
            outcome: outcome.to_string(),
            score: score.clamp(0.0, 1.0),
            note: String::new(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionEvent {
    pub id: String,
    pub signal_type: SignalType,
    pub trigger: HashMap<String, serde_json::Value>,
    pub action: String,
    pub outcome: String,
    pub score: f64,
    pub genes_applied: Vec<String>,
    pub timestamp: String,
}

impl EvolutionEvent {
    pub fn new(signal_type: SignalType, action: &str, outcome: &str, score: f64) -> Self {
        Self {
            id: format!("ev_{}", chrono::Utc::now().timestamp_millis()),
            signal_type,
            trigger: HashMap::new(),
            action: action.to_string(),
            outcome: outcome.to_string(),
            score: score.clamp(0.0, 1.0),
            genes_applied: Vec::new(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedPattern {
    pub name: String,
    pub signal_type: SignalType,
    pub trigger_conditions: HashMap<String, serde_json::Value>,
    pub success_count: i32,
    pub failure_count: i32,
    pub last_used: String,
    pub effectiveness: f64,
}

impl LearnedPattern {
    pub fn new(name: &str, signal_type: SignalType) -> Self {
        Self {
            name: name.to_string(),
            signal_type,
            trigger_conditions: HashMap::new(),
            success_count: 0,
            failure_count: 0,
            last_used: chrono::Utc::now().to_rfc3339(),
            effectiveness: 0.0,
        }
    }

    pub fn total_attempts(&self) -> i32 {
        self.success_count + self.failure_count
    }

    pub fn is_reliable(&self) -> bool {
        self.total_attempts() >= 3 && self.effectiveness >= 0.7
    }

    pub fn record_success(&mut self) {
        self.success_count += 1;
        self.last_used = chrono::Utc::now().to_rfc3339();
        self.update_effectiveness();
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_used = chrono::Utc::now().to_rfc3339();
        self.update_effectiveness();
    }

    fn update_effectiveness(&mut self) {
        let total = self.total_attempts();
        if total > 0 {
            self.effectiveness = self.success_count as f64 / total as f64;
        }
    }
}

// ============================================================================
// Gene Pool Manager
// ============================================================================

#[derive(Debug, Clone)]
pub struct GenePool {
    capsules: Vec<Capsule>,
    feedback_events: Vec<Feedback>,
    evolution_events: Vec<EvolutionEvent>,
    #[allow(dead_code)]
    patterns: Vec<LearnedPattern>,
}

impl Default for GenePool {
    fn default() -> Self {
        Self::new()
    }
}

impl GenePool {
    pub fn new() -> Self {
        Self {
            capsules: Vec::new(),
            feedback_events: Vec::new(),
            evolution_events: Vec::new(),
            patterns: Vec::new(),
        }
    }

    pub fn add_capsule(&mut self, capsule: Capsule) {
        self.capsules.push(capsule);
    }

    pub fn get_capsule(&self, id: &str) -> Option<&Capsule> {
        self.capsules.iter().find(|c| c.capsule_id == id)
    }

    pub fn get_capsule_mut(&mut self, id: &str) -> Option<&mut Capsule> {
        self.capsules.iter_mut().find(|c| c.capsule_id == id)
    }

    pub fn capsules(&self) -> &[Capsule] {
        &self.capsules
    }

    pub fn capsules_mut(&mut self) -> &mut Vec<Capsule> {
        &mut self.capsules
    }

    pub fn active_capsules(&self) -> Vec<&Capsule> {
        self.capsules
            .iter()
            .filter(|c| c.status == CapsuleStatus::Active && !c.archived)
            .collect()
    }

    pub fn find_by_paper(&self, paper_id: &str) -> Option<&Capsule> {
        self.capsules
            .iter()
            .filter(|c| c.archetype.source_paper_id.as_deref() == Some(paper_id))
            .max_by(|a, b| a.created_at.cmp(&b.created_at))
    }

    pub fn find_by_fingerprint(&self, fingerprint: &str) -> Option<&Capsule> {
        self.capsules
            .iter()
            .filter(|c| c.archetype.algorithm_fingerprint.as_deref() == Some(fingerprint))
            .find(|c| c.status == CapsuleStatus::Active)
    }

    pub fn search(&self, keywords: &[&str], gap_type: Option<&str>) -> Vec<&Capsule> {
        self.capsules
            .iter()
            .filter(|c| {
                if c.status == CapsuleStatus::Archived {
                    return false;
                }
                if let Some(gt) = gap_type {
                    if c.action_gap_type != gt {
                        return false;
                    }
                }
                let kw_lower: std::collections::HashSet<String> = c
                    .trigger_keywords
                    .iter()
                    .map(|s| s.to_lowercase())
                    .collect();
                keywords
                    .iter()
                    .any(|kw| kw_lower.contains(&kw.to_lowercase()))
            })
            .collect()
    }

    pub fn record_feedback(&mut self, capsule_id: &str, is_positive: bool) {
        if let Some(cap) = self
            .capsules
            .iter_mut()
            .find(|c| c.capsule_id == capsule_id)
        {
            if is_positive {
                cap.record_success();
            } else {
                cap.record_failure();
            }
        }
    }

    pub fn add_feedback(&mut self, feedback: Feedback) {
        self.feedback_events.push(feedback);
    }

    pub fn add_evolution_event(&mut self, event: EvolutionEvent) {
        self.evolution_events.push(event);
    }

    pub fn diversity(&self) -> GenePoolDiversity {
        GenePoolDiversityCalculator::calculate(&self.capsules)
    }

    pub fn run_decay_cycle(&mut self, lambda: f64) -> Vec<String> {
        let mut archived = Vec::new();
        for cap in &mut self.capsules {
            if cap.archived {
                continue;
            }
            let impact = cap.compute_impact(lambda, 0, 1.0);
            if impact.archived {
                cap.archived = true;
                cap.archive_reason = Some(impact.reason.clone());
                archived.push(cap.capsule_id.clone());
            }
        }
        archived
    }

    pub fn suggest_crossover(&self, gap_type: &str, count: usize) -> Vec<(String, String)> {
        let candidates: Vec<&Capsule> = self
            .capsules
            .iter()
            .filter(|c| {
                c.action_gap_type == gap_type && c.status == CapsuleStatus::Active && !c.archived
            })
            .take(count * 2)
            .collect();

        let mut pairs = Vec::new();
        for i in 0..candidates.len() {
            for j in (i + 1)..candidates.len() {
                if pairs.len() >= count {
                    return pairs;
                }
                pairs.push((
                    candidates[i].capsule_id.clone(),
                    candidates[j].capsule_id.clone(),
                ));
            }
        }
        pairs
    }
}

// ============================================================================
// Gene Pool Persistence
// ============================================================================

impl GenePool {
    pub fn to_jsonl(&self) -> String {
        self.capsules
            .iter()
            .map(|c| serde_json::to_string(c).unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn from_jsonl(text: &str) -> Self {
        let capsules = text
            .lines()
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();
        Self {
            capsules,
            feedback_events: Vec::new(),
            evolution_events: Vec::new(),
            patterns: Vec::new(),
        }
    }

    pub fn default_path() -> std::path::PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".ai_research_os")
            .join("evolution")
    }

    pub fn gene_pool_path() -> std::path::PathBuf {
        Self::default_path().join("gene_pool.jsonl")
    }

    pub fn load() -> std::io::Result<Self> {
        let path = Self::gene_pool_path();
        if !path.exists() {
            return Ok(Self::new());
        }
        let text = std::fs::read_to_string(&path)?;
        Ok(Self::from_jsonl(&text))
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::gene_pool_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, self.to_jsonl())?;
        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capsule_creation() {
        let capsule = Capsule::new("Test approach", "test_gap", vec!["attention".to_string()]);
        assert_eq!(capsule.action_gap_type, "test_gap");
        assert_eq!(capsule.status, CapsuleStatus::Active);
        assert!(!capsule.archived);
    }

    #[test]
    fn test_capsule_feedback() {
        let mut capsule = Capsule::new("Test", "gap", vec![]);
        assert_eq!(capsule.success_rate(), 0.5);

        capsule.record_success();
        assert_eq!(capsule.success_count, 1);
        assert_eq!(capsule.total_feedback(), 1);

        capsule.record_failure();
        assert_eq!(capsule.failure_count, 1);
        assert_eq!(capsule.success_rate(), 0.5);
    }

    #[test]
    fn test_gene_pool_search() {
        let mut pool = GenePool::new();
        pool.add_capsule(Capsule::new(
            "Attention approach",
            "nlp",
            vec!["attention".to_string(), "transformer".to_string()],
        ));
        pool.add_capsule(Capsule::new(
            "RL approach",
            "rl",
            vec!["reinforcement".to_string(), "policy".to_string()],
        ));

        let results = pool.search(&["attention"], None);
        assert_eq!(results.len(), 1);
        assert!(results[0].archetype.approach_summary.contains("Attention"));

        let results = pool.search(&["attention", "transformer"], None);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_gene_pool_diversity() {
        let mut pool = GenePool::new();
        pool.add_capsule(Capsule::new("A1", "g1", vec!["attention".to_string()]));
        pool.add_capsule(Capsule::new("A2", "g1", vec!["transformer".to_string()]));
        pool.add_capsule(Capsule::new("R1", "g2", vec!["reinforcement".to_string()]));

        let div = pool.diversity();
        assert_eq!(div.capsule_count, 3);
        assert!(div.diversity_score > 0);
    }

    #[test]
    fn test_crossover_suggestion() {
        let mut pool = GenePool::new();
        for i in 0..5 {
            pool.add_capsule(Capsule::new(
                &format!("Capsule {}", i),
                "test_gap",
                vec![format!("kw{}", i)],
            ));
        }

        let pairs = pool.suggest_crossover("test_gap", 3);
        assert!(!pairs.is_empty());
        assert!(pairs.len() <= 3);
    }

    #[test]
    fn test_llm_usage_costs() {
        let usage = LlmUsage::openai_gpt4o(1000, 500);
        assert_eq!(usage.total_tokens, 1500);
        assert!(usage.cost_usd > 0.0);

        let usage2 = LlmUsage::anthropic_sonnet4(1000, 500);
        assert_eq!(usage2.total_tokens, 1500);
    }

    #[test]
    fn test_cost_tracker() {
        let mut tracker = CostTracker::new();
        let usage = LlmUsage::openai_gpt4o(1000, 500);
        tracker.record(&usage, "gpt-4o", "openai");
        assert_eq!(tracker.total_cost_usd, usage.cost_usd);
        assert_eq!(tracker.calls_by_model.get("gpt-4o"), Some(&1));
    }

    #[test]
    fn test_feedback_creation() {
        let fb = Feedback::new(
            FeedbackType::Positive,
            "test_cmd",
            "test query",
            vec!["paper1".to_string()],
            "good result",
            0.8,
        );
        assert_eq!(fb.feedback_type, FeedbackType::Positive);
        assert_eq!(fb.command, "test_cmd");
    }

    #[test]
    fn test_learned_pattern() {
        let mut pattern = LearnedPattern::new("test_pattern", SignalType::ChatSuccess);
        assert!(!pattern.is_reliable());
        pattern.record_success();
        pattern.record_success();
        pattern.record_success();
        assert!(pattern.is_reliable());
        assert_eq!(pattern.success_count, 3);
    }

    #[test]
    fn test_capsule_status_display() {
        assert_eq!(CapsuleStatus::Active.to_string(), "active");
        assert_eq!(CapsuleStatus::Dormant.to_string(), "dormant");
        assert_eq!(CapsuleStatus::Archived.to_string(), "archived");
    }

    #[test]
    fn test_gene_pool_find() {
        let mut pool = GenePool::new();
        pool.add_capsule(Capsule::new("test1", "gap1", vec!["kw1".to_string()]));
        pool.add_capsule(Capsule::new("test2", "gap2", vec!["kw2".to_string()]));

        let found = pool.find_by_paper("nonexistent");
        assert!(found.is_none());
    }

    #[test]
    fn test_feedback_type_display() {
        assert_eq!(FeedbackType::Positive.to_string(), "positive");
        assert_eq!(FeedbackType::Negative.to_string(), "negative");
        assert_eq!(FeedbackType::Neutral.to_string(), "neutral");
    }

    #[test]
    fn test_signal_type_display() {
        assert_eq!(SignalType::ChatSuccess.to_string(), "chat_success");
        assert_eq!(SignalType::RetrievalHit.to_string(), "retrieval_hit");
    }

    #[test]
    fn test_contains_research_keyword() {
        assert!(super::contains_research_keyword(
            "The transformer model uses attention"
        ));
        assert!(super::contains_research_keyword("RLHF training"));
        assert!(super::contains_research_keyword(" diffusion model"));
        assert!(!super::contains_research_keyword("xyz abcdef")); // no keywords
    }

    #[test]
    fn test_research_keywords_defined() {
        assert!(super::AI_RESEARCH_KEYWORDS.contains(&"transformer"));
        assert!(super::AI_RESEARCH_KEYWORDS.contains(&"attention"));
        assert!(super::SMART_FOLLOWUP_BASE.contains(&"vs"));
    }

    #[test]
    fn test_query_type() {
        assert_eq!(
            super::QueryType::from_str("factual"),
            super::QueryType::Factual
        );
        assert_eq!(
            super::QueryType::from_str("conceptual"),
            super::QueryType::Conceptual
        );
        assert_eq!(
            super::QueryType::from_str("unknown"),
            super::QueryType::General
        );
        assert_eq!(super::bm25_weight(super::QueryType::Factual), 0.65);
        assert_eq!(super::mmr_lambda(super::QueryType::Factual), 0.8);
    }
}

// ============================================================================
// Research Keywords
// ============================================================================

pub const AI_RESEARCH_KEYWORDS: &[&str] = &[
    "transformer",
    "attention",
    "bert",
    "gpt",
    "llm",
    "language model",
    "neural",
    "network",
    "embedding",
    "fine-tuning",
    "rlhf",
    "rag",
    "retrieval",
    "generative",
    "diffusion",
    "gan",
    "clip",
    "vit",
    "reinforcement",
    "policy",
    "reward",
    "rl",
    "dpo",
    "ppo",
    "reward model",
    "training",
    "optimization",
    "pre-training",
    "instruction",
    "alignment",
    "multimodal",
    "vision",
    "language",
    "speech",
    "audio",
    "constitutional",
    "reasoning",
    "chain-of-thought",
    "cot",
    "synthetic data",
    "model",
    "learning",
];

pub const SMART_FOLLOWUP_BASE: &[&str] = &[
    "attention",
    "transformer",
    "bert",
    "gpt",
    "llm",
    "language model",
    "neural",
    "network",
    "embedding",
    "fine-tuning",
    "rlhf",
    "rag",
    "retrieval",
    "generative",
    "diffusion",
    "gan",
    "clip",
    "vit",
    "weight",
    "layer",
    "parameter",
    "gradient",
    "loss",
    "optimize",
    "softmax",
    "matrix",
    "dot",
    "product",
    "mechanism",
    "reinforcement",
    "policy",
    "reward",
    "rl",
    "dpo",
    "ppo",
    "training",
    "pre-training",
    "instruction",
    "alignment",
    "multimodal",
    "vision",
    "language",
    "speech",
    "audio",
    "constitutional",
    "reasoning",
    "chain-of-thought",
    "cot",
    "implement",
    "code",
    "function",
    "class",
    "api",
    "library",
    "pytorch",
    "tensorflow",
    "module",
    "algorithm",
    "vs",
    "versus",
    "better",
    "worse",
    "compare",
    "advantage",
    "disadvantage",
    "based on",
    "follow",
    "extend",
    "improve",
    "build upon",
    "later",
    "previous",
    "next",
    "evolution",
    "derived",
    "succeed",
    "apply",
    "use",
    "application",
    "industry",
    "practical",
    "deploy",
    "production",
    "real-world",
    "benchmark",
];

/// Check if a text contains any AI research keywords.
pub fn contains_research_keyword(text: &str) -> bool {
    let text_lower = text.to_lowercase();
    AI_RESEARCH_KEYWORDS
        .iter()
        .any(|kw| text_lower.contains(&kw.to_lowercase()))
}

// ============================================================================
// At-Risk Capsule Scanner
// ============================================================================

const STREAK_THRESHOLD: usize = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtRiskCapsule {
    pub capsule_id: String,
    pub gap_title: String,
    pub gap_type: String,
    pub outcome_score: f64,
    pub low_score_streak: usize,
    pub status: String,
    pub pinned_ttl: usize,
    pub trigger_keywords: Vec<String>,
}

fn capsule_path() -> std::path::PathBuf {
    dirs::home_dir()
        .map(|p| {
            p.join(".ai_research_os")
                .join("gene_pool")
                .join("capsules.json")
        })
        .unwrap_or_else(|| std::path::PathBuf::from("capsules.json"))
}

#[derive(Debug, Deserialize)]
struct CapsuleFile {
    capsules: Vec<CapsuleEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CapsuleEntry {
    #[serde(rename = "capsule_id")]
    capsule_id: Option<String>,
    #[serde(rename = "action_gap_title")]
    action_gap_title: Option<String>,
    #[serde(rename = "action_gap_type")]
    action_gap_type: Option<String>,
    #[serde(rename = "outcome_success_score")]
    outcome_success_score: Option<f64>,
    #[serde(rename = "low_score_streak")]
    low_score_streak: Option<usize>,
    status: Option<String>,
    #[serde(rename = "pinned_ttl")]
    pinned_ttl: Option<usize>,
    #[serde(rename = "trigger_keywords")]
    trigger_keywords: Option<Vec<String>>,
}

fn load_capsules() -> Vec<CapsuleEntry> {
    let path = capsule_path();
    if !path.exists() {
        return Vec::new();
    }
    match std::fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str::<CapsuleFile>(&contents)
            .map(|f| f.capsules)
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

fn save_capsules(capsules: &[serde_json::Value]) -> std::io::Result<()> {
    let path = capsule_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let data = serde_json::json!({ "capsules": capsules });
    std::fs::write(&path, serde_json::to_string_pretty(&data)?)?;
    Ok(())
}

/// Return active capsules with low_score_streak >= threshold.
pub fn get_at_risk_capsules(threshold: usize) -> Vec<AtRiskCapsule> {
    let all_caps = load_capsules();
    let mut results: Vec<AtRiskCapsule> = Vec::new();

    for cap in all_caps {
        let status = cap.status.as_deref().unwrap_or("");
        if status != "active" && !status.is_empty() {
            continue;
        }
        let streak = cap.low_score_streak.unwrap_or(0);
        if streak < threshold {
            continue;
        }
        results.push(AtRiskCapsule {
            capsule_id: cap.capsule_id.unwrap_or_default(),
            gap_title: cap.action_gap_title.unwrap_or_default(),
            gap_type: cap.action_gap_type.unwrap_or_default(),
            outcome_score: cap.outcome_success_score.unwrap_or(0.0),
            low_score_streak: streak,
            status: cap.status.unwrap_or_else(|| "active".to_string()),
            pinned_ttl: cap.pinned_ttl.unwrap_or(0),
            trigger_keywords: cap.trigger_keywords.unwrap_or_default(),
        });
    }

    results.sort_by_key(|a| a.low_score_streak);
    results
}

/// Reset low_score_streak to 0 for a capsule.
pub fn keep_active(capsule_id: &str) -> bool {
    let all_caps = load_capsules();
    let mut found = false;
    let updated: Vec<serde_json::Value> = all_caps
        .into_iter()
        .map(|mut cap| {
            if cap.capsule_id.as_deref() == Some(capsule_id) {
                cap.low_score_streak = Some(0);
                cap.pinned_ttl = Some(0);
                found = true;
            }
            serde_json::to_value(&cap).unwrap_or_default()
        })
        .collect();

    if found {
        let _ = save_capsules(&updated);
    }
    found
}

/// Pin a capsule to TTL cycles (resets streak, sets pinned_ttl).
pub fn pin_to_ttl(capsule_id: &str, ttl: usize) -> bool {
    let all_caps = load_capsules();
    let mut found = false;
    let updated: Vec<serde_json::Value> = all_caps
        .into_iter()
        .map(|mut cap| {
            if cap.capsule_id.as_deref() == Some(capsule_id) {
                cap.pinned_ttl = Some(ttl);
                cap.low_score_streak = Some(0);
                found = true;
            }
            serde_json::to_value(&cap).unwrap_or_default()
        })
        .collect();

    if found {
        let _ = save_capsules(&updated);
    }
    found
}

/// Render at-risk capsules as HTML table.
pub fn render_at_risk_html() -> String {
    let capsules = get_at_risk_capsules(STREAK_THRESHOLD);

    if capsules.is_empty() {
        return "<p>No at-risk capsules. All capsules are healthy.</p>".to_string();
    }

    let mut html = String::new();
    html.push_str("<div class=\"at-risk-panel\">");
    html.push_str(&format!(
        "<h3>🚨 At-Risk Capsules <small style='color:#888'>({} need attention)</small></h3>",
        capsules.len()
    ));
    html.push_str("<table class=\"at-risk-table\">");
    html.push_str("<thead><tr><th>Gap Title</th><th>Type</th><th>Score</th><th>Streak</th><th>Pinned</th><th>Action</th></tr></thead>");
    html.push_str("<tbody>");

    for cap in &capsules {
        let streak_bar = "🔴".repeat(cap.low_score_streak);
        let pinned = if cap.pinned_ttl > 0 {
            format!("TTL {}", cap.pinned_ttl)
        } else {
            "—".to_string()
        };
        let title_short = if cap.gap_title.len() > 35 {
            format!("{}...", &cap.gap_title[..35])
        } else {
            cap.gap_title.clone()
        };

        html.push_str("<tr>");
        html.push_str(&format!(
            "<td style='max-width:220px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap'><code title='{}'>{}</code></td>",
            cap.gap_title, title_short
        ));
        html.push_str(&format!("<td><code>{}</code></td>", cap.gap_type));
        html.push_str(&format!("<td>{:.2}</td>", cap.outcome_score));
        html.push_str(&format!(
            "<td>{} <small>{}</small></td>",
            streak_bar, cap.low_score_streak
        ));
        html.push_str(&format!("<td>{}</td>", pinned));
        html.push_str("<td>");
        html.push_str(&format!(
            "<button class=\"btn btn-small btn-keep\" onclick=\"keepActive('{}')\">✓ Keep Active</button>",
            cap.capsule_id
        ));
        html.push_str(&format!(
            "<button class=\"btn btn-small btn-pin\" onclick=\"pinToTTL('{}')\">📌 Pin TTL</button>",
            cap.capsule_id
        ));
        html.push_str("</td>");
        html.push_str("</tr>");
    }

    html.push_str("</tbody></table>");
    html.push_str("<style>");
    html.push_str(".at-risk-panel { font-family: Georgia, serif; }");
    html.push_str(".at-risk-table { width: 100%; border-collapse: collapse; margin-top: 1rem; }");
    html.push_str(".at-risk-table th, .at-risk-table td { padding: 0.4rem 0.8rem; border-bottom: 1px solid #e8e4de; text-align: left; }");
    html.push_str(".at-risk-table th { font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em; color: #7a7570; }");
    html.push_str(
        ".btn-small { padding: 3px 10px; font-size: 12px; border-radius: 4px; cursor: pointer; }",
    );
    html.push_str(".btn-keep { background: #7A9E7A; color: white; border: none; }");
    html.push_str(
        ".btn-pin { background: #6B8FB5; color: white; border: none; margin-left: 4px; }",
    );
    html.push_str("</style>");
    html.push_str("</div>");

    html
}
