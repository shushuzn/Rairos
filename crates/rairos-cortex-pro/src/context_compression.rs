//! Context compression module for reducing token usage in agent interactions.
//!
//! Based on research from:
//! - ECoRAG (ACL 2025) - Evidentiality-guided compression
//! - ACC-RAG (EMNLP 2025) - Adaptive context compression
//! - ATTNCOMP (EMNLP 2025) - Attention-guided compression
//!
//! ## Architecture
//!
//! ```text
//! Input Context
//!      │
//!      ▼
//! ┌─────────────────┐
//! │ Relevance Filter │ ← Remove irrelevant content
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │Evidence Score   │ ← Rate supporting evidence
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │  Compression    │ ← Summarize or truncate
//! └────────┬────────┘
//!          │
//!          ▼
//! Compressed Output
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Compression ratio target
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CompressionRatio {
    /// Target compression ratio (0.0 - 1.0, lower = more compression)
    pub target: f32,
    /// Minimum tokens to keep
    pub min_tokens: usize,
    /// Maximum tokens to keep
    pub max_tokens: usize,
}

impl CompressionRatio {
    pub fn aggressive() -> Self {
        Self {
            target: 0.3,
            min_tokens: 100,
            max_tokens: 500,
        }
    }

    pub fn moderate() -> Self {
        Self {
            target: 0.5,
            min_tokens: 200,
            max_tokens: 1000,
        }
    }

    pub fn light() -> Self {
        Self {
            target: 0.7,
            min_tokens: 300,
            max_tokens: 2000,
        }
    }
}

impl Default for CompressionRatio {
    fn default() -> Self {
        Self::moderate()
    }
}

/// Content chunk with relevance scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredChunk {
    /// Chunk content
    pub content: String,
    /// Relevance score (0.0 - 1.0)
    pub relevance: f32,
    /// Evidence strength (0.0 - 1.0)
    pub evidence: f32,
    /// Source/timestamp if available
    pub source: Option<String>,
}

/// Compressed context result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedContext {
    /// Compressed content
    pub content: String,
    /// Original token count
    pub original_tokens: usize,
    /// Compressed token count
    pub compressed_tokens: usize,
    /// Compression ratio achieved
    pub ratio: f32,
    /// Chunks included
    pub chunks_used: usize,
    /// Summary if condensed
    pub summary: Option<String>,
}

/// Context compressor for agent interactions
pub struct ContextCompressor {
    /// Default compression ratio
    default_ratio: CompressionRatio,
    /// Enable evidence-guided compression
    evidence_guided: bool,
    /// Enable adaptive compression
    adaptive: bool,
}

impl Default for ContextCompressor {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextCompressor {
    /// Create a new context compressor
    pub fn new() -> Self {
        Self {
            default_ratio: CompressionRatio::default(),
            evidence_guided: true,
            adaptive: true,
        }
    }

    /// Configure with compression ratio
    pub fn with_ratio(mut self, ratio: CompressionRatio) -> Self {
        self.default_ratio = ratio;
        self
    }

    /// Enable/disable evidence-guided compression
    pub fn with_evidence_guidance(mut self, enabled: bool) -> Self {
        self.evidence_guided = enabled;
        self
    }

    /// Enable/disable adaptive compression
    pub fn with_adaptive(mut self, enabled: bool) -> Self {
        self.adaptive = enabled;
        self
    }

    /// Estimate token count (rough)
    fn estimate_tokens(&self, text: &str) -> usize {
        // Rough estimate: 4 characters per token on average
        text.len() / 4
    }

    /// Split text into chunks
    fn split_chunks(&self, text: &str, chunk_size: usize) -> Vec<String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut chunks = Vec::new();
        let mut current_chunk = Vec::new();
        let mut current_len = 0;

        for word in words {
            let word_len = word.len();
            if current_len + word_len > chunk_size && !current_chunk.is_empty() {
                chunks.push(current_chunk.join(" "));
                current_chunk = Vec::new();
                current_len = 0;
            }
            current_chunk.push(word);
            current_len += word_len + 1;
        }

        if !current_chunk.is_empty() {
            chunks.push(current_chunk.join(" "));
        }

        chunks
    }

    /// Score chunks by relevance (simplified keyword matching)
    fn score_chunks(&self, chunks: &[String], keywords: &[&str]) -> Vec<ScoredChunk> {
        chunks
            .iter()
            .map(|chunk| {
                let chunk_lower = chunk.to_lowercase();
                let keyword_matches = keywords
                    .iter()
                    .filter(|kw| chunk_lower.contains(&kw.to_lowercase()))
                    .count();

                let relevance = if keywords.is_empty() {
                    0.5
                } else {
                    keyword_matches as f32 / keywords.len() as f32
                };

                // Evidence is based on length (longer chunks = more evidence)
                let evidence = (chunk.len() as f32 / 500.0).min(1.0);

                ScoredChunk {
                    content: chunk.clone(),
                    relevance,
                    evidence,
                    source: None,
                }
            })
            .collect()
    }

    /// Compress context using evidence-guided selection
    pub fn compress(&self, text: &str, keywords: &[&str]) -> CompressedContext {
        let original_tokens = self.estimate_tokens(text);
        let target_tokens = (original_tokens as f32 * self.default_ratio.target) as usize;
        let target_tokens = target_tokens.clamp(
            self.default_ratio.min_tokens,
            self.default_ratio.max_tokens,
        );

        // Split into chunks
        let chunk_size = 200; // words
        let chunks = self.split_chunks(text, chunk_size);

        // Score chunks
        let mut scored = self.score_chunks(&chunks, keywords);

        // Sort by combined score
        scored.sort_by(|a, b| {
            let score_a = if self.evidence_guided {
                a.relevance * 0.7 + a.evidence * 0.3
            } else {
                a.relevance
            };
            let score_b = if self.evidence_guided {
                b.relevance * 0.7 + b.evidence * 0.3
            } else {
                b.relevance
            };
            score_b.partial_cmp(&score_a).unwrap()
        });

        // Select top chunks until target
        let mut selected = Vec::new();
        let mut total_tokens = 0;

        for chunk in &scored {
            let chunk_tokens = self.estimate_tokens(&chunk.content);
            if total_tokens + chunk_tokens <= target_tokens {
                selected.push(chunk.clone());
                total_tokens += chunk_tokens;
            }
        }

        // Sort back to original order
        selected.sort_by(|a, b| {
            let idx_a = chunks.iter().position(|c| c == &a.content).unwrap_or(0);
            let idx_b = chunks.iter().position(|c| c == &b.content).unwrap_or(0);
            idx_a.cmp(&idx_b)
        });

        let content = selected.iter().map(|c| c.content.clone()).collect::<Vec<_>>().join(" ");
        let compressed_tokens = self.estimate_tokens(&content);
        let ratio = if original_tokens > 0 {
            compressed_tokens as f32 / original_tokens as f32
        } else {
            1.0
        };

        CompressedContext {
            content,
            original_tokens,
            compressed_tokens,
            ratio,
            chunks_used: selected.len(),
            summary: None,
        }
    }

    /// Compress with summary generation
    pub fn compress_with_summary(&self, text: &str, keywords: &[&str]) -> CompressedContext {
        let mut compressed = self.compress(text, keywords);

        // If we compressed significantly, add a summary
        if compressed.ratio < 0.5 {
            let summary = self.generate_summary(&compressed.content, keywords);
            compressed.summary = Some(summary);
        }

        compressed
    }

    /// Generate a brief summary (simplified)
    fn generate_summary(&self, text: &str, keywords: &[&str]) -> String {
        let sentences: Vec<&str> = text.split('.').filter(|s| !s.trim().is_empty()).collect();

        // Score sentences by keyword presence
        let mut scored: Vec<(&str, f32)> = sentences
            .iter()
            .map(|s| {
                let s_lower = s.to_lowercase();
                let kw_score = keywords
                    .iter()
                    .filter(|kw| s_lower.contains(&kw.to_lowercase()))
                    .count() as f32
                    / keywords.len().max(1) as f32;
                (*s, kw_score)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Take top 2 sentences as summary
        let summary: String = scored
            .iter()
            .take(2)
            .map(|(s, _)| s.trim())
            .collect::<Vec<_>>()
            .join(". ");

        if summary.is_empty() {
            "Summary: Key information selected from compressed context.".to_string()
        } else {
            format!("Summary: {}.", summary)
        }
    }

    /// Compress research context
    pub fn compress_context(&self, context: &super::state::ResearchContext) -> CompressedContext {
        let mut content = String::new();

        content.push_str(&format!("Topic: {}\n", context.topic));
        if !context.keywords.is_empty() {
            content.push_str(&format!("Keywords: {}\n", context.keywords.join(", ")));
        }
        if !context.constraints.is_empty() {
            content.push_str(&format!("Constraints: {}\n", context.constraints.join(", ")));
        }

        // Extract keywords from topic
        let keywords: Vec<&str> = context.topic.split_whitespace().take(5).collect();

        self.compress(&content, &keywords)
    }

    /// Compress crew context (SparksCrew state)
    pub fn compress_crew_context(&self, context: &super::state::CrewContext) -> CompressedContext {
        let mut content = String::new();

        if let Some(ref query) = context.query {
            content.push_str(&format!("Query: {}\n", query));
        }
        if let Some(ref hypothesis) = context.hypothesis {
            content.push_str(&format!("Hypothesis: {}\n", hypothesis));
        }
        if let Some(ref plan) = context.plan {
            content.push_str(&format!("Plan: {}\n", plan));
        }

        // Extract keywords from query
        let keywords: Vec<&str> = context
            .query
            .as_ref()
            .map(|q| q.split_whitespace().take(5).collect())
            .unwrap_or_default();

        self.compress(&content, &keywords)
    }
}

/// Token budget manager for agent interactions
pub struct TokenBudget {
    /// Maximum tokens per agent call
    max_tokens: usize,
    /// Current usage
    used_tokens: usize,
    /// Budget tracking
    history: Vec<BudgetEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetEntry {
    pub role: String,
    pub tokens: usize,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl TokenBudget {
    /// Create a new budget manager
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            used_tokens: 0,
            history: Vec::new(),
        }
    }

    /// Check if we can afford an agent call
    pub fn can_afford(&self, tokens: usize) -> bool {
        self.used_tokens + tokens <= self.max_tokens
    }

    /// Reserve tokens for an agent call
    pub fn reserve(&mut self, role: &str, tokens: usize) {
        self.used_tokens += tokens;
        self.history.push(BudgetEntry {
            role: role.to_string(),
            tokens,
            timestamp: chrono::Utc::now(),
        });
    }

    /// Get remaining budget
    pub fn remaining(&self) -> usize {
        self.max_tokens.saturating_sub(self.used_tokens)
    }

    /// Reset for new interaction
    pub fn reset(&mut self) {
        self.used_tokens = 0;
        self.history.clear();
    }

    /// Get utilization percentage
    pub fn utilization(&self) -> f32 {
        if self.max_tokens == 0 {
            0.0
        } else {
            self.used_tokens as f32 / self.max_tokens as f32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_ratio() {
        let aggressive = CompressionRatio::aggressive();
        assert!(aggressive.target < 0.5);

        let moderate = CompressionRatio::moderate();
        assert!(moderate.target > aggressive.target && moderate.target < 0.6);

        let light = CompressionRatio::light();
        assert!(light.target > moderate.target);
    }

    #[test]
    fn test_compress() {
        let compressor = ContextCompressor::new();
        let text = "This is a test document about thermoelectric materials. Bi2Te3 is a good thermoelectric. The ZT value is around 1.1. Materials science involves studying material properties.";

        let result = compressor.compress(text, &["thermoelectric", "Bi2Te3"]);
        assert!(result.compressed_tokens < result.original_tokens);
        assert!(result.ratio < 1.0);
    }

    #[test]
    fn test_compress_with_summary() {
        let compressor = ContextCompressor::new();
        let text = "This is a test document about thermoelectric materials. Bi2Te3 is a good thermoelectric. The ZT value is around 1.1. Materials science involves studying material properties. Quantum mechanics describes electron behavior.";

        let result = compressor.compress_with_summary(text, &["thermoelectric"]);
        assert!(result.ratio < 1.0);
    }

    #[test]
    fn test_token_budget() {
        let mut budget = TokenBudget::new(1000);

        assert!(budget.can_afford(500));
        assert!(budget.can_afford(1000));

        budget.reserve("hypothesis", 300);
        assert!(!budget.can_afford(800));
        assert!(budget.can_afford(700));
        assert_eq!(budget.remaining(), 700);
    }

    #[test]
    fn test_budget_utilization() {
        let mut budget = TokenBudget::new(1000);
        assert_eq!(budget.utilization(), 0.0);

        budget.reserve("test", 250);
        assert_eq!(budget.utilization(), 0.25);

        budget.reserve("test", 250);
        assert_eq!(budget.utilization(), 0.5);
    }

    #[test]
    fn test_budget_reset() {
        let mut budget = TokenBudget::new(1000);
        budget.reserve("test", 500);
        assert_eq!(budget.utilization(), 0.5);

        budget.reset();
        assert_eq!(budget.utilization(), 0.0);
        assert_eq!(budget.remaining(), 1000);
    }

    #[test]
    fn test_estimate_tokens() {
        let compressor = ContextCompressor::new();
        // 100 characters ≈ 25 tokens
        let text = "a b c d e f g h i j k l m n o p q r s t u v w x y z a b c d e f g h i j";
        assert_eq!(compressor.estimate_tokens(text), 27);
    }
}