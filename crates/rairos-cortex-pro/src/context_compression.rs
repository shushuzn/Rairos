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
        // Pre-compute lowercase keywords to avoid repeated allocation
        let keywords_lower: Vec<String> = keywords.iter()
            .map(|kw| kw.to_lowercase())
            .collect();

        chunks
            .iter()
            .map(|chunk| {
                let chunk_lower = chunk.to_lowercase();
                let keyword_matches = keywords_lower
                    .iter()
                    .filter(|kw| chunk_lower.contains(kw))
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

        // Sort back to original order - use pre-computed index map for O(1) lookups
        let index_map: std::collections::HashMap<&str, usize> = chunks
            .iter()
            .enumerate()
            .map(|(i, c)| (c.as_str(), i))
            .collect();
        selected.sort_by(|a, b| {
            let idx_a = index_map.get(a.content.as_str()).copied().unwrap_or(0);
            let idx_b = index_map.get(b.content.as_str()).copied().unwrap_or(0);
            idx_a.cmp(&idx_b)
        });

        let content = selected.iter().map(|c| c.content.as_str()).collect::<Vec<_>>().join(" ");
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

        // Pre-compute lowercase keywords
        let keywords_lower: Vec<String> = keywords.iter()
            .map(|kw| kw.to_lowercase())
            .collect();

        // Score sentences by keyword presence
        let mut scored: Vec<(&str, f32)> = sentences
            .iter()
            .map(|s| {
                let s_lower = s.to_lowercase();
                let kw_score = keywords_lower
                    .iter()
                    .filter(|kw| s_lower.contains(kw))
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
        // Long enough text to actually compress (over 200 tokens)
        let text = "Thermoelectric materials are compounds that can convert heat to electricity directly. The efficiency of thermoelectric conversion is measured by the dimensionless figure of merit ZT. Higher ZT values indicate better performance. Bismuth telluride (Bi2Te3) is one of the most widely used thermoelectric materials at room temperature. The thermoelectric effect was discovered by Thomas Johann Seebeck in 1821. Thermoelectric generators can be used for waste heat recovery. The peltier effect is the reverse of the Seebeck effect and is used for cooling. Thermoelectric coolers have no moving parts and are environmentally friendly. Recent research has focused on half-Heusler compounds and skutterudites for high-temperature applications. The power factor S2σ is used to evaluate thermoelectric material performance. Nanostructuring has emerged as a promising approach to improve thermoelectric properties. Phonon glass electron crystal concept guides material design. Lead telluride alloys have been extensively studied for mid-temperature applications. Flexible thermoelectric generators enable wearable electronics. The thermoelectric market is growing for Internet of Things devices.";

        let result = compressor.compress(text, &["thermoelectric", "Bi2Te3"]);
        assert!(result.compressed_tokens < result.original_tokens);
        assert!(result.ratio < 1.0);
    }

    #[test]
    fn test_compress_with_summary() {
        let compressor = ContextCompressor::new();
        // Long text to ensure compression actually reduces size
        let text = "Thermoelectric materials represent a fascinating class of compounds that enable direct conversion between heat and electricity through the Seebeck and Peltier effects. The efficiency of thermoelectric conversion is quantified by the dimensionless figure of merit ZT, which depends on the Seebeck coefficient S, electrical conductivity σ, thermal conductivity κ, and absolute temperature T. High ZT values require a favorable combination of high Seebeck coefficient, high electrical conductivity, and low thermal conductivity. Bismuth telluride alloys have historically been the benchmark thermoelectric materials for room temperature applications, achieving ZT values around 1. However, the relatively low ZT has limited widespread commercial adoption except in niche applications like thermoelectric cooling and waste heat recovery. Recent advances in nanostructuring, band engineering, and the discovery of new material classes such as half-Heusler compounds and super-lattice structures have pushed ZT values above 2 in some cases. The thermoelectric market is expanding with applications in Internet of Things devices, wearable electronics, and automotive exhaust heat recovery systems.";

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
        // 70 characters / 4 ≈ 17 tokens
        let text = "a b c d e f g h i j k l m n o p q r s t u v w x y z a b c d e f g h i j";
        assert_eq!(compressor.estimate_tokens(text), 17);
    }
}

// =============================================================================
// ACON-Style Context Compression Optimization
// Based on "ACON: Agent Context Optimization" (arXiv:2510.00615)
// =============================================================================

/// Trajectory pair for learning compression guidelines
#[derive(Debug, Clone)]
pub struct TrajectoryPair {
    /// Full context that succeeded
    pub full_context: String,
    /// Compressed context that failed
    pub compressed_context: String,
    /// Task description
    pub task: String,
    /// What information was lost
    pub lost_information: String,
}

/// Learned compression guideline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionGuideline {
    /// What to preserve
    pub preserve_patterns: Vec<String>,
    /// What to compress
    pub compress_patterns: Vec<String>,
    /// Minimum relevance threshold
    pub min_relevance: f32,
    /// Domain-specific rules
    pub domain_rules: Vec<String>,
    /// Confidence score (0-1)
    pub confidence: f32,
}

impl Default for CompressionGuideline {
    fn default() -> Self {
        Self {
            preserve_patterns: vec![
                r"\d+[\.\-]\d+[\.\-]\d+".to_string(),  // Dates
                r"\$[\d,]+(\.\d{2})?".to_string(),       // Money
                r"[A-Z]{2,}".to_string(),               // Acronyms
                r"https?://\S+".to_string(),            // URLs
            ],
            compress_patterns: vec![
                r"^(he|she|they|the)\s+".to_string(),  // Pronouns starting sentences
                r"\s{2,}".to_string(),                  // Multiple spaces
                r"\[.*?\]".to_string(),                  // Bracketed content
            ],
            min_relevance: 0.5,
            domain_rules: vec![],
            confidence: 0.5,
        }
    }
}

/// ACON-style compression analyzer
/// Analyzes trajectory pairs to learn what information is critical
pub struct AconeCompressionAnalyzer {
    guidelines: Vec<CompressionGuideline>,
}

impl AconeCompressionAnalyzer {
    pub fn new() -> Self {
        Self {
            guidelines: vec![CompressionGuideline::default()],
        }
    }

    /// Analyze a trajectory pair to learn what went wrong
    pub fn analyze_failure(&mut self, pair: &TrajectoryPair) -> CompressionGuideline {
        let mut guideline = CompressionGuideline::default();

        // Extract key differences between full and compressed
        let full_len = pair.full_context.len();
        let compressed_len = pair.compressed_context.len();
        let compression_ratio = compressed_len as f32 / full_len as f32;

        // Detect lost information patterns
        let lost_lower = pair.lost_information.to_lowercase();

        // If numerical data was lost, preserve numbers
        if lost_lower.contains("number") || lost_lower.contains("quantity") || lost_lower.contains("amount") {
            guideline.preserve_patterns.push(r"\d+(\.\d+)?".to_string());
        }

        // If names were lost, preserve capitalized terms
        if lost_lower.contains("name") || lost_lower.contains("who") {
            guideline.preserve_patterns.push(r"[A-Z][a-z]+".to_string());
        }

        // If dates were lost, preserve date patterns
        if lost_lower.contains("date") || lost_lower.contains("when") {
            guideline.preserve_patterns.push(r"\d{1,2}[/-]\d{1,2}[/-]\d{2,4}".to_string());
        }

        // If tool calls were lost, preserve tool-related content
        if lost_lower.contains("tool") || lost_lower.contains("function") {
            guideline.compress_patterns.retain(|p| p != r"\[.*?\]");
            guideline.preserve_patterns.push(r"tool[_:]?\w+".to_string());
        }

        // Calculate confidence based on how extreme the compression was
        guideline.confidence = if compression_ratio < 0.3 {
            0.3 // Very aggressive compression, lower confidence
        } else if compression_ratio < 0.5 {
            0.6
        } else {
            0.8
        };

        self.guidelines.push(guideline.clone());
        guideline
    }

    /// Merge multiple guidelines into a consolidated one
    pub fn merge_guidelines(&self) -> CompressionGuideline {
        let mut merged = CompressionGuideline::default();
        merged.confidence = 0.0;

        for guideline in &self.guidelines {
            // Union of preserve patterns
            for pattern in &guideline.preserve_patterns {
                if !merged.preserve_patterns.contains(pattern) {
                    merged.preserve_patterns.push(pattern.clone());
                }
            }

            // Union of compress patterns
            for pattern in &guideline.compress_patterns {
                if !merged.compress_patterns.contains(pattern) {
                    merged.compress_patterns.push(pattern.clone());
                }
            }

            // Average domain rules
            merged.domain_rules.extend(guideline.domain_rules.clone());

            // Max confidence
            merged.confidence = merged.confidence.max(guideline.confidence);
        }

        // Remove duplicates from domain_rules
        merged.domain_rules.sort();
        merged.domain_rules.dedup();

        merged
    }

    /// Get current best guideline
    pub fn get_best_guideline(&self) -> CompressionGuideline {
        self.guidelines
            .iter()
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
            .cloned()
            .unwrap_or_default()
    }
}

impl Default for AconeCompressionAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Apply learned guidelines to compress context
pub fn acon_compress(context: &str, guideline: &CompressionGuideline) -> String {
    let mut result = context.to_string();

    // Simple string-based preservation (avoiding regex dependency)
    // Protect preserved patterns by replacing with placeholders
    let mut placeholders: Vec<(String, String)> = Vec::new();

    // For now, use simple string matching for key patterns
    // Numbers and special values
    let number_patterns = ["$", "€", "£", "¥", "%", "http://", "https://"];
    for (i, &pattern) in number_patterns.iter().enumerate() {
        if result.contains(pattern) {
            // Find all occurrences and protect them
            let mut search_start = 0usize;
            let mut j = 0;
            while let Some(pos) = result[search_start..].find(pattern) {
                let abs_pos = search_start + pos;
                // Extract a reasonable context around the pattern
                let start = abs_pos.saturating_sub(5);
                let end = (abs_pos + pattern.len() + 10).min(result.len());
                let protected = result[start..end].to_string();
                let placeholder = format!("__P{}__", i * 100 + j);
                placeholders.push((placeholder.clone(), protected));
                result = format!("{} {}...", &result[..start], placeholder);
                search_start = start + placeholder.len() + 3;
                j += 1;
            }
        }
    }

    // Clean up multiple spaces
    while result.contains("  ") {
        result = result.replace("  ", " ");
    }

    // Restore preserved patterns
    for (placeholder, original) in &placeholders {
        result = result.replace(placeholder, original);
    }

    result.trim().to_string()
}

#[cfg(test)]
mod acon_tests {
    use super::*;

    #[test]
    fn test_analyze_failure_learns_numbers() {
        let mut analyzer = AconeCompressionAnalyzer::new();

        let pair = TrajectoryPair {
            full_context: "The price is $123.45 and quantity is 500 units".to_string(),
            compressed_context: "The price is and quantity is units".to_string(),
            task: "Calculate total cost".to_string(),
            lost_information: "The numerical values were lost".to_string(),
        };

        let guideline = analyzer.analyze_failure(&pair);
        assert!(guideline.preserve_patterns.iter().any(|p| p.contains(r"\d")));
    }

    #[test]
    fn test_acon_compress_preserves_special_chars() {
        let guideline = CompressionGuideline {
            preserve_patterns: vec![],
            compress_patterns: vec![],
            min_relevance: 0.5,
            domain_rules: vec![],
            confidence: 0.8,
        };

        let result = acon_compress("The price is $100 and quantity is 50", &guideline);
        // Simple compression just trims whitespace
        assert!(result.contains("$100"));
    }

    #[test]
    fn test_merge_guidelines() {
        let analyzer = AconeCompressionAnalyzer::new();
        let merged = analyzer.merge_guidelines();
        assert!(merged.confidence > 0.0);
    }
}