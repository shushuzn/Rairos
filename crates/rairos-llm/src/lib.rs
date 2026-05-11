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
pub enum CapsuleStatus {
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

impl Default for CapsuleStatus {
    fn default() -> Self {
        CapsuleStatus::Active
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
        self.impact_score = self.impact_score.max(0.0).min(10.0);
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
    pub fn compute_impact(&self, lambda: f64, inbound_citations: i32, citation_boost: f64) -> CapsuleImpact {
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
                format!("Impact {:.4} < threshold {:.4}", base_impact, DEFAULT_MIN_IMPACT)
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
        ("attention", &["attention", "transformer", "multi-head", "self-attention", "cross-attention"]),
        ("reinforcement", &["rl", "reinforcement", "policy", "reward", "agent", "DQN", "PPO", "A3C"]),
        ("language_model", &["LM", "language model", "decoder", "autoregressive", "LLM", "GPT", "BERT"]),
        ("vision", &["CNN", "convolution", "resnet", "image", "vision", "ViT", "classification"]),
        ("optimization", &["optimizer", "Adam", "SGD", "gradient", "loss", "training"]),
        ("graph", &["GNN", "graph", "node", "edge", "message passing"]),
        ("reasoning", &["reasoning", "chain-of-thought", "logical", "inference", "planning"]),
        ("embodied", &["embodied", "robotics", "navigation", "control", "motor"]),
    ];

    fn family_of(keywords: &[String]) -> String {
        let kw_set: std::collections::HashSet<String> = keywords.iter()
            .map(|k| k.to_lowercase())
            .collect();
        for (fam, fam_kws) in Self::FAMILY_KEYWORDS {
            if fam_kws.iter().any(|fk| kw_set.contains(&fk.to_string())) {
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
            *gap_type_counts.entry(cap.action_gap_type.clone()).or_insert(0) += 1;
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
        let normalized_shannon = if max_entropy > 0.0 { shannon / max_entropy } else { 0.0 };

        let family_coverage = family_count as f64 / Self::FAMILY_KEYWORDS.len() as f64;
        let diversity_score = ((normalized_shannon * 0.6 + family_coverage * 0.4) * 100.0) as i32;

        let mut sorted_counts: Vec<usize> = family_counts.values().cloned().collect();
        sorted_counts.sort();
        let median_count = sorted_counts.get(sorted_counts.len() / 2).copied().unwrap_or(1);

        let underrep: Vec<String> = family_counts.iter()
            .filter(|(_, &c)| c < median_count / 10)
            .map(|(f, _)| f.clone())
            .collect();
        let overrep: Vec<String> = family_counts.iter()
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
        self.capsules.iter()
            .filter(|c| c.status == CapsuleStatus::Active && !c.archived)
            .collect()
    }

    pub fn find_by_paper(&self, paper_id: &str) -> Option<&Capsule> {
        self.capsules.iter()
            .filter(|c| c.archetype.source_paper_id.as_deref() == Some(paper_id))
            .max_by(|a, b| a.created_at.cmp(&b.created_at))
    }

    pub fn find_by_fingerprint(&self, fingerprint: &str) -> Option<&Capsule> {
        self.capsules.iter()
            .filter(|c| c.archetype.algorithm_fingerprint.as_deref() == Some(fingerprint))
            .find(|c| c.status == CapsuleStatus::Active)
    }

    pub fn search(&self, keywords: &[&str], gap_type: Option<&str>) -> Vec<&Capsule> {
        self.capsules.iter()
            .filter(|c| {
                if c.status == CapsuleStatus::Archived {
                    return false;
                }
                if let Some(gt) = gap_type {
                    if &c.action_gap_type != gt {
                        return false;
                    }
                }
                let kw_lower: std::collections::HashSet<String> = c.trigger_keywords.iter()
                    .map(|s| s.to_lowercase())
                    .collect();
                keywords.iter().any(|kw| kw_lower.contains(&kw.to_lowercase()))
            })
            .collect()
    }

    pub fn record_feedback(&mut self, capsule_id: &str, is_positive: bool) {
        if let Some(cap) = self.capsules.iter_mut().find(|c| c.capsule_id == capsule_id) {
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
        let candidates: Vec<&Capsule> = self.capsules.iter()
            .filter(|c| c.action_gap_type == gap_type && c.status == CapsuleStatus::Active && !c.archived)
            .take(count * 2)
            .collect();

        let mut pairs = Vec::new();
        for i in 0..candidates.len() {
            for j in (i + 1)..candidates.len() {
                if pairs.len() >= count {
                    return pairs;
                }
                pairs.push((candidates[i].capsule_id.clone(), candidates[j].capsule_id.clone()));
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
        self.capsules.iter()
            .map(|c| serde_json::to_string(c).unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn from_jsonl(text: &str) -> Self {
        let capsules = text.lines()
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
        pool.add_capsule(Capsule::new("Attention approach", "nlp", vec!["attention".to_string(), "transformer".to_string()]));
        pool.add_capsule(Capsule::new("RL approach", "rl", vec!["reinforcement".to_string(), "policy".to_string()]));

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
            pool.add_capsule(Capsule::new(&format!("Capsule {}", i), "test_gap", vec![format!("kw{}", i)]));
        }

        let pairs = pool.suggest_crossover("test_gap", 3);
        assert!(!pairs.is_empty());
        assert!(pairs.len() <= 3);
    }
}
