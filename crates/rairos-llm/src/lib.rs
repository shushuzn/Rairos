//! Rairos LLM — LLM client wrappers and research intelligence
//!
//! Provides unified interface to multiple LLM providers.
//! Replaces: llm/client.py, llm/citation_chain.py, llm/gap_detector.py

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
            cost_usd: (prompt as f64 * 1.1 / 1_000_000.0)
                + (completion as f64 * 4.4 / 1_000_000.0),
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub usage: LlmUsage,
    pub model: String,
    pub finish_reason: String,
}

/// Trait for LLM providers
#[async_trait::async_trait]
pub trait LlmClient: Send + Sync {
    /// Send a completion request
    async fn complete(
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
// OpenAI Client
// ============================================================================

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

        Ok(LlmResponse {
            content: data.choices.into_iter().next()
                .map(|c| c.message.content)
                .unwrap_or_default(),
            usage,
            model: data.model,
            finish_reason: data.finish_reason,
        })
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

        let content = data.content
            .into_iter()
            .filter_map(|c| match c {
                ContentBlock::Text { text } => Some(text),
            })
            .collect::<Vec<_>>()
            .join("\n");

        let usage = LlmUsage::anthropic_sonnet4(data.usage.input_tokens, data.usage.output_tokens);

        Ok(LlmResponse {
            content,
            usage,
            model: data.model,
            finish_reason: data.stop_reason,
        })
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
        *self.calls_by_provider.entry(provider.to_string()).or_insert(0) += 1;
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn summary(&self) -> String {
        format!(
            "Total cost: {:.4}, Total tokens: {}, Models: {:?}, Providers: {:?}",
            self.total_cost_usd,
            self.total_tokens,
            self.calls_by_model,
            self.calls_by_provider
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
        self.edges.values()
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
                    || p.abstract_text.to_lowercase().contains(&keyword.to_lowercase())
            });

            if !has_keyword {
                gaps.push(format!(
                    "No papers found matching keyword: {}",
                    keyword
                ));
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

        category_count.into_iter()
            .filter(|(_, count)| *count < threshold)
            .map(|(cat, _)| cat.to_string())
            .collect()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_tracker() {
        let mut tracker = CostTracker::new();
        let usage = LlmUsage::openai_gpt4o(1000, 500);
        tracker.record(&usage, "gpt-4o", "openai");
        assert!(tracker.total_cost_usd > 0.0);
        assert_eq!(tracker.calls_by_model.get("gpt-4o"), Some(&1));
    }

    #[test]
    fn test_gap_detector() {
        let papers = vec![];
        let gaps = GapDetector::detect_gaps(&papers, &["transformer", "attention"]);
        assert_eq!(gaps.len(), 2);
    }
}
