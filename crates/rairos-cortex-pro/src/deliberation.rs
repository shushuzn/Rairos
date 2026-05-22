//! Deliberation-First Orchestration Module
//!
//! Based on DOVA (arXiv:2603.13327) - Deliberation-First Multi-Agent Orchestration
//!
//! ## Key Innovations
//!
//! 1. **Deliberation Layer**: Meta-reasoning BEFORE tool invocation
//! 2. **Hybrid Collaborative Reasoning**: Ensemble → Blackboard → Iterative Refinement
//! 3. **Adaptive Multi-Tiered Thinking**: 6-tier token budget allocation
//!
//! ## Architecture
//!
//! ```text
//! Query → Deliberation Layer
//!              │
//!              ├── Trigger Check (needs external info?)
//!              ├── Tool Necessity Assessment
//!              └── Budget Allocation
//!              │
//!              ▼
//!         [If tools needed]
//!              │
//!              ├── Ensemble Phase (diverse perspectives)
//!              ├── Blackboard Phase (shared evidence)
//!              └── Iterative Refinement (critique循环)
//! ```
//!
//! ## Token Budget Tiers
//!
//! | Tier | Tokens | Use Case |
//! |-------|--------|----------|
//! | 1 | 50 | Simple factual queries |
//! | 2 | 150 | Opinion/analysis |
//! | 3 | 500 | Light research |
//! | 4 | 1000 | Standard research |
//! | 5 | 2000 | Deep research |
//! | 6 | 4000+ | Complex investigation |

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::Hash;

/// Deliberation trigger types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeliberationTrigger {
    /// Always invoke tools
    ForceTools,
    /// Never invoke tools (internal reasoning only)
    InternalOnly,
    /// Deliberate first
    Deliberate,
}

/// Result of deliberation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliberationResult {
    /// Whether external tools are needed
    pub needs_tools: bool,
    /// Confidence in the decision
    pub confidence: f32,
    /// Reasoning for the decision
    pub reasoning: String,
    /// Recommended thinking tier
    pub thinking_tier: ThinkingTier,
    /// Estimated token budget
    pub estimated_tokens: u32,
}

/// Thinking budget tiers (DOVA's 6-tier system)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ThinkingTier {
    /// Tier 1: Simple factual queries (~50 tokens)
    Tier1_Factual = 1,
    /// Tier 2: Opinion/analysis (~150 tokens)
    Tier2_Opinion = 2,
    /// Tier 3: Light research (~500 tokens)
    Tier3_Light = 3,
    /// Tier 4: Standard research (~1000 tokens)
    Tier4_Standard = 4,
    /// Tier 5: Deep research (~2000 tokens)
    Tier5_Deep = 5,
    /// Tier 6: Complex investigation (~4000+ tokens)
    Tier6_Complex = 6,
}

impl ThinkingTier {
    /// Get token budget for this tier
    pub fn token_budget(&self) -> u32 {
        match self {
            ThinkingTier::Tier1_Factual => 50,
            ThinkingTier::Tier2_Opinion => 150,
            ThinkingTier::Tier3_Light => 500,
            ThinkingTier::Tier4_Standard => 1000,
            ThinkingTier::Tier5_Deep => 2000,
            ThinkingTier::Tier6_Complex => 4000,
        }
    }

    /// Get description of this tier
    pub fn description(&self) -> &'static str {
        match self {
            ThinkingTier::Tier1_Factual => "Simple factual query",
            ThinkingTier::Tier2_Opinion => "Opinion or light analysis",
            ThinkingTier::Tier3_Light => "Light research with minimal tools",
            ThinkingTier::Tier4_Standard => "Standard research task",
            ThinkingTier::Tier5_Deep => "Deep investigation",
            ThinkingTier::Tier6_Complex => "Complex multi-faceted investigation",
        }
    }
}

/// Query analysis for deliberation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryAnalysis {
    /// Original query
    pub query: String,
    /// Detected temporal keywords
    pub temporal_keywords: Vec<String>,
    /// Specificity markers
    pub specificity_markers: Vec<String>,
    /// Domain complexity estimate
    pub complexity: f32,
    /// Whether it requires real-time data
    pub needs_realtime: bool,
    /// Whether it requires external tools
    pub needs_external: bool,
}

/// Deliberation engine
pub struct DeliberationEngine {
    config: DeliberationConfig,
    /// History of deliberation decisions (using VecDeque for O(1) front removal)
    history: VecDeque<DeliberationRecord>,
    /// Trigger keywords for tool necessity
    tool_triggers: HashSet<String>,
    /// Temporal keywords
    temporal_keywords: HashSet<String>,
    /// Specificity markers
    specificity_markers: HashSet<String>,
}

/// Configuration for deliberation
#[derive(Debug, Clone)]
pub struct DeliberationConfig {
    /// Enable deliberation layer
    pub enabled: bool,
    /// Enable adaptive thinking tiers
    pub adaptive_tiers: bool,
    /// Minimum confidence for tool invocation
    pub min_tool_confidence: f32,
    /// Enable hybrid reasoning phases
    pub hybrid_reasoning: bool,
}

impl Default for DeliberationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            adaptive_tiers: true,
            min_tool_confidence: 0.7,
            hybrid_reasoning: true,
        }
    }
}

/// Record of deliberation decision
#[derive(Debug, Clone)]
pub struct DeliberationRecord {
    pub query_hash: u64,
    pub needs_tools: bool,
    pub confidence: f32,
    pub thinking_tier: ThinkingTier,
    pub actual_tokens_used: u32,
    pub was_correct: bool,
}

impl DeliberationEngine {
    /// Create new deliberation engine
    pub fn new(config: DeliberationConfig) -> Self {
        let mut tool_triggers = HashSet::new();
        tool_triggers.insert("search".to_string());
        tool_triggers.insert("find".to_string());
        tool_triggers.insert("look up".to_string());
        tool_triggers.insert("get".to_string());
        tool_triggers.insert("fetch".to_string());
        tool_triggers.insert("retrieve".to_string());
        tool_triggers.insert("download".to_string());
        tool_triggers.insert("access".to_string());
        tool_triggers.insert("query".to_string());
        tool_triggers.insert("latest".to_string());
        tool_triggers.insert("recent".to_string());
        tool_triggers.insert("current".to_string());
        tool_triggers.insert("new".to_string());
        tool_triggers.insert("updated".to_string());

        let mut temporal_keywords = HashSet::new();
        temporal_keywords.insert("latest".to_string());
        temporal_keywords.insert("recent".to_string());
        temporal_keywords.insert("current".to_string());
        temporal_keywords.insert("new".to_string());
        temporal_keywords.insert("2024".to_string());
        temporal_keywords.insert("2025".to_string());
        temporal_keywords.insert("2026".to_string());
        temporal_keywords.insert("today".to_string());
        temporal_keywords.insert("yesterday".to_string());
        temporal_keywords.insert("this week".to_string());
        temporal_keywords.insert("this month".to_string());
        temporal_keywords.insert("last year".to_string());

        let mut specificity_markers = HashSet::new();
        specificity_markers.insert("specific".to_string());
        specificity_markers.insert("exact".to_string());
        specificity_markers.insert("particular".to_string());
        specificity_markers.insert("concrete".to_string());
        specificity_markers.insert("detailed".to_string());
        specificity_markers.insert("comprehensive".to_string());
        specificity_markers.insert("thorough".to_string());
        specificity_markers.insert("in-depth".to_string());
        specificity_markers.insert("specific papers".to_string());
        specificity_markers.insert("particular study".to_string());

        Self {
            config,
            history: VecDeque::new(),
            tool_triggers,
            temporal_keywords,
            specificity_markers,
        }
    }

    /// Deliberate on a query - should we use tools?
    pub fn deliberate(&self, query: &str) -> DeliberationResult {
        if !self.config.enabled {
            return DeliberationResult {
                needs_tools: true,
                confidence: 1.0,
                reasoning: "Deliberation disabled".to_string(),
                thinking_tier: ThinkingTier::Tier4_Standard,
                estimated_tokens: ThinkingTier::Tier4_Standard.token_budget(),
            };
        }

        let analysis = self.analyze_query(query);

        // Check for mandatory tool triggers
        if self.has_mandatory_tool_trigger(&analysis) {
            return DeliberationResult {
                needs_tools: true,
                confidence: 0.95,
                reasoning: "Mandatory tool trigger detected".to_string(),
                thinking_tier: self.estimate_thinking_tier(&analysis),
                estimated_tokens: self.estimate_tokens(&analysis),
            };
        }

        // Deliberate on tool necessity
        let (needs_tools, reasoning, confidence) = self.assess_tool_necessity(&analysis);

        DeliberationResult {
            needs_tools,
            confidence,
            reasoning,
            thinking_tier: self.estimate_thinking_tier(&analysis),
            estimated_tokens: self.estimate_tokens(&analysis),
        }
    }

    /// Analyze a query for deliberation
    fn analyze_query(&self, query: &str) -> QueryAnalysis {
        let query_lower = query.to_lowercase();
        let words: Vec<&str> = query_lower.split_whitespace().collect();

        // Single pass: collect both temporal_keywords and specificity_markers
        let mut temporal_keywords = Vec::new();
        let mut specificity_markers = Vec::new();
        for w in &words {
            if self.temporal_keywords.contains(&w.to_string()) {
                temporal_keywords.push(w.to_string());
            }
            if self.specificity_markers.contains(&w.to_string()) {
                specificity_markers.push(w.to_string());
            }
        }

        // Detect real-time needs
        let needs_realtime = !temporal_keywords.is_empty();

        // Detect external tool needs
        let needs_external = words
            .iter()
            .any(|w| self.tool_triggers.contains(*w));

        // Estimate complexity based on query length and markers
        let complexity = self.estimate_complexity(query, &specificity_markers);

        QueryAnalysis {
            query: query.to_string(),
            temporal_keywords,
            specificity_markers,
            complexity,
            needs_realtime,
            needs_external,
        }
    }

    /// Check for mandatory tool triggers
    fn has_mandatory_tool_trigger(&self, analysis: &QueryAnalysis) -> bool {
        // Temporal keywords always require tools
        if !analysis.temporal_keywords.is_empty() {
            return true;
        }

        // High specificity often requires external data
        if analysis.specificity_markers.len() >= 2 && analysis.complexity > 0.5 {
            return true;
        }

        false
    }

    /// Assess whether tools are necessary
    fn assess_tool_necessity(&self, analysis: &QueryAnalysis) -> (bool, String, f32) {
        let mut reasons = Vec::new();
        let mut confidence = 0.5f32;

        // If real-time data is needed, use tools
        if analysis.needs_realtime {
            reasons.push("query requires real-time information");
            confidence = confidence.max(0.9);
        }

        // If external data is needed, use tools
        if analysis.needs_external {
            reasons.push("query explicitly requests external data");
            confidence = confidence.max(0.85);
        }

        // High complexity suggests tools might help
        if analysis.complexity > 0.7 {
            reasons.push("high complexity benefits from external knowledge");
            confidence = confidence.max(0.75);
        }

        // If multiple specificity markers, likely needs research
        if analysis.specificity_markers.len() >= 2 {
            reasons.push("multiple specificity markers indicate research need");
            confidence = confidence.max(0.8);
        }

        let needs_tools = confidence >= self.config.min_tool_confidence;
        let reasoning = if reasons.is_empty() {
            "Query appears to be answerable from internal knowledge".to_string()
        } else {
            format!("Tool recommendation: {}", reasons.join("; "))
        };

        (needs_tools, reasoning, confidence)
    }

    /// Estimate thinking tier based on analysis
    fn estimate_thinking_tier(&self, analysis: &QueryAnalysis) -> ThinkingTier {
        if !self.config.adaptive_tiers {
            return ThinkingTier::Tier4_Standard;
        }

        // Simple factual queries
        if analysis.query.len() < 30 && !analysis.needs_external && !analysis.needs_realtime {
            return ThinkingTier::Tier1_Factual;
        }

        // Light analysis without external needs
        if !analysis.needs_external && !analysis.needs_realtime && analysis.complexity < 0.4 {
            return ThinkingTier::Tier2_Opinion;
        }

        // Light research
        if (analysis.needs_external || analysis.needs_realtime) && analysis.complexity < 0.5 {
            return ThinkingTier::Tier3_Light;
        }

        // Standard research
        if analysis.complexity < 0.7 {
            return ThinkingTier::Tier4_Standard;
        }

        // Deep research
        if analysis.complexity < 0.85 {
            return ThinkingTier::Tier5_Deep;
        }

        // Complex investigation
        ThinkingTier::Tier6_Complex
    }

    /// Estimate token budget
    fn estimate_tokens(&self, analysis: &QueryAnalysis) -> u32 {
        self.estimate_thinking_tier(analysis).token_budget()
    }

    /// Estimate query complexity
    fn estimate_complexity(&self, query: &str, specificity_markers: &[String]) -> f32 {
        let mut complexity = 0.3f32;

        // Length factor
        let word_count = query.split_whitespace().count();
        if word_count > 50 {
            complexity += 0.2;
        } else if word_count > 20 {
            complexity += 0.1;
        }

        // Specificity factor
        complexity += (specificity_markers.len() as f32 * 0.1).min(0.3);

        // Question marks and conditional words suggest complexity
        if query.contains("?") {
            complexity += 0.1;
        }
        if query.contains(" if ") || query.contains(" when ") || query.contains(" how ") {
            complexity += 0.1;
        }

        complexity.min(1.0)
    }

    /// Record deliberation outcome for learning
    pub fn record_outcome(&mut self, query: &str, result: &DeliberationResult, was_correct: bool) {
        let query_hash = self.hash_query(query);
        self.history.push_back(DeliberationRecord {
            query_hash,
            needs_tools: result.needs_tools,
            confidence: result.confidence,
            thinking_tier: result.thinking_tier,
            actual_tokens_used: result.estimated_tokens,
            was_correct,
        });

        // Trim history (VecDeque pop_front is O(1), vs Vec remove(0) which is O(n))
        if self.history.len() > 1000 {
            self.history.pop_front();
        }
    }

    /// Hash query for history lookup
    fn hash_query(&self, query: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        query.hash(&mut hasher);
        hasher.finish()
    }

    /// Get deliberation statistics
    pub fn stats(&self) -> DeliberationStats {
        let total = self.history.len();
        if total == 0 {
            return DeliberationStats {
                total_deliberations: 0,
                tool_invocation_rate: 0.0,
                average_confidence: 0.0,
                tier_distribution: HashMap::new(),
            };
        }

        let tool_invocations = self.history.iter().filter(|r| r.needs_tools).count();
        let avg_confidence: f32 = self.history.iter().map(|r| r.confidence).sum::<f32>() / total as f32;

        let mut tier_distribution = HashMap::new();
        for record in &self.history {
            *tier_distribution.entry(record.thinking_tier).or_insert(0) += 1;
        }

        DeliberationStats {
            total_deliberations: total,
            tool_invocation_rate: tool_invocations as f32 / total as f32,
            average_confidence: avg_confidence,
            tier_distribution,
        }
    }
}

/// Deliberation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliberationStats {
    pub total_deliberations: usize,
    pub tool_invocation_rate: f32,
    pub average_confidence: f32,
    pub tier_distribution: HashMap<ThinkingTier, usize>,
}

// =============================================================================
// Hybrid Collaborative Reasoning (DOVA's 3-Phase Pipeline)
// =============================================================================

/// Phase in hybrid reasoning
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningPhase {
    /// Phase 1: Ensemble - diverse perspectives
    Ensemble,
    /// Phase 2: Blackboard - shared evidence
    Blackboard,
    /// Phase 3: Iterative Refinement - critique循环
    IterativeRefinement,
}

/// A perspective in the ensemble phase
#[derive(Debug, Clone)]
pub struct Perspective {
    pub agent_id: String,
    pub viewpoint: String,
    pub confidence: f32,
    pub evidence: Vec<String>,
}

/// Shared blackboard entry
#[derive(Debug, Clone)]
pub struct BlackboardEntry {
    pub source: String,
    pub content: String,
    pub relevance_score: f32,
    pub timestamp: u64,
}

/// Critique in refinement phase
#[derive(Debug, Clone)]
pub struct Critique {
    pub source: String,
    pub target: String,
    pub issue: String,
    pub severity: f32,
    pub suggestion: Option<String>,
}

/// Hybrid reasoning engine
pub struct HybridReasoning {
    config: HybridReasoningConfig,
}

#[derive(Debug, Clone)]
pub struct HybridReasoningConfig {
    pub max_perspectives: usize,
    pub max_iterations: usize,
    pub convergence_threshold: f32,
}

impl Default for HybridReasoningConfig {
    fn default() -> Self {
        Self {
            max_perspectives: 5,
            max_iterations: 3,
            convergence_threshold: 0.8,
        }
    }
}

impl HybridReasoning {
    pub fn new(config: HybridReasoningConfig) -> Self {
        Self { config }
    }

    /// Run ensemble phase - generate diverse perspectives
    pub fn ensemble_phase(&self, query: &str, agent_ids: &[String]) -> Vec<Perspective> {
        let mut perspectives = Vec::new();

        for agent_id in agent_ids.iter().take(self.config.max_perspectives) {
            // In a real implementation, each agent would generate a perspective
            // Here we create a placeholder structure
            perspectives.push(Perspective {
                agent_id: agent_id.clone(),
                viewpoint: format!("Perspective from {}", agent_id),
                confidence: 0.7,
                evidence: Vec::new(),
            });
        }

        perspectives
    }

    /// Run blackboard phase - consolidate evidence
    pub fn blackboard_phase(&self, perspectives: &[Perspective]) -> Vec<BlackboardEntry> {
        let mut entries = Vec::new();
        let mut seen_content = HashSet::new();

        for perspective in perspectives {
            for evidence in &perspective.evidence {
                let content_hash = self.hash_content(evidence);
                if !seen_content.contains(&content_hash) {
                    seen_content.insert(content_hash);
                    entries.push(BlackboardEntry {
                        source: perspective.agent_id.clone(),
                        content: evidence.clone(),
                        relevance_score: perspective.confidence,
                        timestamp: 0,
                    });
                }
            }
        }

        // Sort by relevance
        entries.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
        entries
    }

    /// Run iterative refinement phase
    pub fn refinement_phase(
        &self,
        initial_answer: &str,
        critiques: Vec<Critique>,
    ) -> RefinementResult {
        let mut current_answer = initial_answer.to_string();
        let mut iterations = 0;

        while iterations < self.config.max_iterations {
            let mut has_critiques = false;

            for critique in &critiques {
                if critique.severity > 0.5 {
                    has_critiques = true;
                    if let Some(suggestion) = &critique.suggestion {
                        current_answer = format!("{} [Refined: {}]", current_answer, suggestion);
                    }
                }
            }

            iterations += 1;

            if !has_critiques || iterations >= self.config.max_iterations {
                break;
            }
        }

        RefinementResult {
            answer: current_answer,
            iterations,
            converged: iterations < self.config.max_iterations,
        }
    }

    fn hash_content(&self, content: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }
}

/// Result of refinement
#[derive(Debug, Clone)]
pub struct RefinementResult {
    pub answer: String,
    pub iterations: usize,
    pub converged: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deliberation_factual() {
        let engine = DeliberationEngine::new(DeliberationConfig::default());
        let result = engine.deliberate("What is rust?");
        assert!(!result.needs_tools);
        assert_eq!(result.thinking_tier, ThinkingTier::Tier1_Factual);
    }

    #[test]
    fn test_deliberation_realtime() {
        let engine = DeliberationEngine::new(DeliberationConfig::default());
        let result = engine.deliberate("What is the latest research on AI agents?");
        assert!(result.needs_tools);
    }

    #[test]
    fn test_deliberation_complex() {
        let engine = DeliberationEngine::new(DeliberationConfig::default());
        let result = engine.deliberate(
            "Provide a comprehensive in-depth analysis of the specific papers \
             on transformer architecture, including detailed comparisons of \
             recent developments and particular studies from this year",
        );
        assert!(result.needs_tools);
        assert!(result.thinking_tier >= ThinkingTier::Tier5_Deep);
    }

    #[test]
    fn test_thinking_tier_token_budget() {
        assert_eq!(ThinkingTier::Tier1_Factual.token_budget(), 50);
        assert_eq!(ThinkingTier::Tier4_Standard.token_budget(), 1000);
        assert_eq!(ThinkingTier::Tier6_Complex.token_budget(), 4000);
    }

    #[test]
    fn test_hybrid_reasoning() {
        let reasoning = HybridReasoning::new(HybridReasoningConfig::default());
        let agents = vec!["agent1".to_string(), "agent2".to_string()];
        let perspectives = reasoning.ensemble_phase("test query", &agents);
        assert_eq!(perspectives.len(), 2);
    }
}
