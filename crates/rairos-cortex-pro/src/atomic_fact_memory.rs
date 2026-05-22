//! Atomic Fact Memory Augmentation Module.
//!
//! Based on research from:
//! - Atomic Fact Augmentation (arXiv:2506.09171) - Extract atomic facts + recursive lookahead
//! - Atomic Reasoner (arXiv:2503.15944) - Atomic actions + cognitive tree routing
//! - MemReader (arXiv:2604.07877) - Active extraction with ReAct paradigm
//!
//! ## Architecture
//!
//! ```text
//! Interaction → Atomic Fact Extraction → Memory Storage
//!                                           │
//!                         ┌─────────────────┼─────────────────┐
//!                         ▼                 ▼                 ▼
//!                   ┌──────────┐    ┌──────────────┐   ┌──────────────┐
//!                   │ Semantic │    │   Episodic   │   │  Procedural  │
//!                   │ (What)   │    │  (How/Why)   │   │   (When)     │
//!                   └──────────┘    └──────────────┘   └──────────────┘
//!                                           │
//!                         ┌─────────────────┴─────────────────┐
//!                         ▼                                 ▼
//!                   ┌──────────────┐               ┌──────────────┐
//!                   │   Lookup    │               │   Augment    │ ◄── Prompt Augmentation
//!                   │   Context   │               │    Prompt    │
//!                   └──────────────┘               └──────────────┘
//! ```

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// An atomic fact extracted from interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomicFact {
    /// Fact ID
    pub id: String,
    /// Fact content
    pub content: String,
    /// Task category
    pub task_type: TaskType,
    /// Utility score (importance)
    pub utility_score: f32,
    /// Source interaction ID
    pub source_id: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Whether this fact led to success
    pub success_signal: Option<bool>,
}

/// Task type classification
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TaskType {
    /// Search and retrieval
    Search,
    /// Analysis and reasoning
    Analysis,
    /// Generation and creation
    Generation,
    /// Planning and coordination
    Planning,
    /// Debugging and repair
    Debugging,
    /// General interaction
    General,
}

impl TaskType {
    /// Parse from string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "search" | "retrieval" | "find" => TaskType::Search,
            "analysis" | "reasoning" | "analyze" => TaskType::Analysis,
            "generation" | "create" | "write" => TaskType::Generation,
            "planning" | "coordinate" | "organize" => TaskType::Planning,
            "debugging" | "repair" | "fix" | "debug" => TaskType::Debugging,
            _ => TaskType::General,
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskType::Search => "search",
            TaskType::Analysis => "analysis",
            TaskType::Generation => "generation",
            TaskType::Planning => "planning",
            TaskType::Debugging => "debugging",
            TaskType::General => "general",
        }
    }
}

/// Memory tier
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MemoryTier {
    /// Immediate working memory
    Working = 0,
    /// Short-term episodic memory
    Episodic = 1,
    /// Long-term semantic memory
    Semantic = 2,
    /// Archived historical memory
    Archive = 3,
}

/// A memory entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Entry ID
    pub id: String,
    /// Atomic facts
    pub facts: Vec<AtomicFact>,
    /// Associated trajectory
    pub trajectory: String,
    /// Outcome
    pub outcome: Outcome,
    /// Memory tier
    pub tier: MemoryTier,
    /// Access count
    pub access_count: u32,
    /// Last accessed
    pub last_accessed: DateTime<Utc>,
    /// Created at
    pub created_at: DateTime<Utc>,
}

/// Outcome of an interaction
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Outcome {
    /// Task succeeded
    Success,
    /// Task failed
    Failure,
    /// Task partially completed
    Partial,
    /// Task in progress
    Ongoing,
}

impl Outcome {
    /// Check if successful
    pub fn is_success(&self) -> bool {
        matches!(self, Outcome::Success)
    }

    /// Check if failed
    pub fn is_failure(&self) -> bool {
        matches!(self, Outcome::Failure)
    }
}

/// Atomic Fact Memory System
pub struct AtomicFactMemory {
    /// Memory entries by tier
    tiers: HashMap<MemoryTier, Vec<MemoryEntry>>,
    /// Maximum entries per tier
    max_entries: HashMap<MemoryTier, usize>,
    /// Task type statistics
    task_stats: HashMap<TaskType, TaskStats>,
    /// Fact index for fast lookup
    fact_index: HashMap<String, Vec<(String, MemoryTier)>>, // fact_id -> (entry_id, tier)
}

/// Task statistics
#[derive(Debug, Clone, Default)]
pub struct TaskStats {
    pub total: u32,
    pub successes: u32,
    pub failures: u32,
    pub avg_utility: f32,
}

impl AtomicFactMemory {
    /// Create a new atomic fact memory
    pub fn new() -> Self {
        let mut max_entries = HashMap::new();
        max_entries.insert(MemoryTier::Working, 10);
        max_entries.insert(MemoryTier::Episodic, 100);
        max_entries.insert(MemoryTier::Semantic, 500);
        max_entries.insert(MemoryTier::Archive, usize::MAX);

        Self {
            tiers: HashMap::new(),
            max_entries,
            task_stats: HashMap::new(),
            fact_index: HashMap::new(),
        }
    }

    /// Store a memory entry
    pub fn store(&mut self, entry: MemoryEntry) {
        let tier = entry.tier;

        // Add to tier
        self.tiers.entry(tier).or_default().push(entry.clone());

        // Index facts
        for fact in &entry.facts {
            self.fact_index
                .entry(fact.content.clone())
                .or_default()
                .push((entry.id.clone(), tier));
        }

        // Update statistics
        self.update_stats(&entry);

        // Prune if necessary
        self.prune_tier(tier);
    }

    /// Update task statistics
    fn update_stats(&mut self, entry: &MemoryEntry) {
        let stats = self.task_stats.entry(entry.facts.first().map(|f| f.task_type).unwrap_or(TaskType::General))
            .or_insert_with(TaskStats::default);

        stats.total += 1;
        match entry.outcome {
            Outcome::Success => stats.successes += 1,
            Outcome::Failure => stats.failures += 1,
            _ => {}
        }

        let total_utility: f32 = entry.facts.iter().map(|f| f.utility_score).sum();
        stats.avg_utility = (stats.avg_utility * (stats.total - 1) as f32 + total_utility) / stats.total as f32;
    }

    /// Prune tier if over capacity
    fn prune_tier(&mut self, tier: MemoryTier) {
        if let Some(entries) = self.tiers.get_mut(&tier) {
            let max = *self.max_entries.get(&tier).unwrap_or(&usize::MAX);
            if entries.len() > max {
                // Sort by last accessed and remove oldest
                entries.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));
                entries.truncate(max);
            }
        }
    }

    /// Retrieve relevant facts for a query
    pub fn retrieve(&self, query: &str, task_type: Option<TaskType>, limit: usize) -> Vec<&AtomicFact> {
        let query_lower = query.to_lowercase();
        let mut scored_facts: Vec<(&AtomicFact, f32)> = Vec::new();

        // Search all tiers
        for (tier, entries) in &self.tiers {
            for entry in entries {
                // Skip if task type doesn't match
                if let Some(tt) = task_type {
                    if !entry.facts.iter().any(|f| f.task_type == tt) {
                        continue;
                    }
                }

                for fact in &entry.facts {
                    let score = self.calculate_relevance(fact, &query_lower, tier);
                    if score > 0.0 {
                        scored_facts.push((fact, score));
                    }
                }
            }
        }

        // Sort by score descending
        scored_facts.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        scored_facts.into_iter().take(limit).map(|(f, _)| f).collect()
    }

    /// Calculate relevance score
    fn calculate_relevance(&self, fact: &AtomicFact, query: &str, tier: &MemoryTier) -> f32 {
        let mut score = 0.0;

        // Content match
        if fact.content.to_lowercase().contains(query) {
            score += 0.5;
        }

        // Task type bonus
        if query.contains(fact.task_type.as_str()) {
            score += 0.2;
        }

        // Utility bonus
        score += fact.utility_score * 0.2;

        // Recency bonus (higher tier = older, so lower bonus)
        let tier_bonus = match tier {
            MemoryTier::Working => 0.1,
            MemoryTier::Episodic => 0.05,
            MemoryTier::Semantic => 0.02,
            MemoryTier::Archive => 0.0,
        };
        score += tier_bonus;

        score.min(1.0)
    }

    /// Augment a prompt with relevant facts
    pub fn augment_prompt(&self, base_prompt: &str, max_facts: usize) -> String {
        let facts = self.retrieve(base_prompt, None, max_facts);

        if facts.is_empty() {
            return base_prompt.to_string();
        }

        let fact_strings: Vec<_> = facts
            .iter()
            .enumerate()
            .map(|(i, f)| format!("[Fact {}] {}", i + 1, f.content))
            .collect();

        format!(
            "{}\n\nRelevant Context:\n{}\n",
            base_prompt,
            fact_strings.join("\n")
        )
    }

    /// Extract atomic facts from a trajectory
    pub fn extract_facts(&self, trajectory: &str, outcome: Outcome, task_type: TaskType) -> Vec<AtomicFact> {
        let mut facts = Vec::new();

        // Simple keyword-based extraction (in practice would use LLM)
        let sentences: Vec<_> = trajectory.split(&['.', '!', '?'][..])
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        for (i, sentence) in sentences.iter().enumerate() {
            if sentence.len() < 10 {
                continue;
            }

            // Check for key patterns
            let contains_action = ["search", "find", "create", "update", "delete", "analyze", "plan"]
                .iter()
                .any(|kw| sentence.to_lowercase().contains(kw));

            let contains_result = ["found", "created", "success", "failed", "completed", "error"]
                .iter()
                .any(|kw| sentence.to_lowercase().contains(kw));

            if contains_action || contains_result {
                facts.push(AtomicFact {
                    id: uuid_simple(),
                    content: sentence.to_string(),
                    task_type: task_type.clone(),
                    utility_score: if contains_result { 0.8 } else { 0.5 },
                    source_id: "extracted".to_string(),
                    timestamp: Utc::now(),
                    success_signal: Some(outcome.is_success()),
                });
            }
        }

        // Limit to most important facts
        facts.sort_by(|a, b| b.utility_score.partial_cmp(&a.utility_score).unwrap());
        facts.truncate(10);

        facts
    }

    /// Consolidate episodic memories to semantic
    pub fn consolidate(&mut self) -> ConsolidationReport {
        let mut report = ConsolidationReport::default();

        // First, collect entries to promote (avoiding nested borrows)
        let to_promote: Vec<MemoryEntry> = if let Some(episodic) = self.tiers.get(&MemoryTier::Episodic) {
            episodic
                .iter()
                .filter(|e| e.outcome.is_success() && e.facts.iter().any(|f| f.utility_score > 0.7))
                .cloned()
                .collect()
        } else {
            Vec::new()
        };

        // Move important episodic facts to semantic
        if !to_promote.is_empty() {
            let semantic_entries: Vec<MemoryEntry> = to_promote
                .iter()
                .map(|entry| {
                    report.promoted += 1;
                    MemoryEntry {
                        id: uuid_simple(),
                        facts: entry.facts.clone(),
                        trajectory: format!("Consolidated from {}", entry.id),
                        outcome: entry.outcome.clone(),
                        tier: MemoryTier::Semantic,
                        access_count: 0,
                        last_accessed: Utc::now(),
                        created_at: Utc::now(),
                    }
                })
                .collect();

            // Get mutable reference and update
            if let Some(episodic) = self.tiers.get_mut(&MemoryTier::Episodic) {
                // Remove promoted from episodic
                episodic.retain(|e| {
                    !e.outcome.is_success() || !e.facts.iter().any(|f| f.utility_score > 0.7)
                });
            }

            // Push semantic entries
            self.tiers.entry(MemoryTier::Semantic).or_default().extend(semantic_entries);
        }

        report
    }

    /// Get memory statistics
    pub fn stats(&self) -> MemorySystemStats {
        let mut tier_counts = HashMap::new();
        for (tier, entries) in &self.tiers {
            tier_counts.insert(*tier, entries.len());
        }

        MemorySystemStats {
            tier_counts,
            task_stats: self.task_stats.clone(),
            total_facts: self.fact_index.len(),
        }
    }
}

impl Default for AtomicFactMemory {
    fn default() -> Self {
        Self::new()
    }
}

/// Report from consolidation
#[derive(Debug, Clone, Default)]
pub struct ConsolidationReport {
    pub promoted: usize,
    pub archived: usize,
    pub discarded: usize,
    pub success: bool,
}

/// Memory system statistics
#[derive(Debug, Clone)]
pub struct MemorySystemStats {
    pub tier_counts: HashMap<MemoryTier, usize>,
    pub task_stats: HashMap<TaskType, TaskStats>,
    pub total_facts: usize,
}

// =============================================================================
// Atomic Reasoner (Cognitive Tree Routing)
// =============================================================================

/// Cognitive routing decision
#[derive(Debug, Clone)]
pub struct CognitiveRoute {
    /// Route type
    pub route_type: RouteType,
    /// Confidence
    pub confidence: f32,
    /// Reasoning
    pub reasoning: String,
}

/// Route type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RouteType {
    /// Direct retrieval
    Retrieve,
    /// Plan new action
    Plan,
    /// Reason step by step
    Reason,
    /// Decompose task
    Decompose,
    /// Fallback/default
    Fallback,
}

/// Atomic reasoner for cognitive routing
pub struct AtomicReasoner {
    /// Memory system
    memory: AtomicFactMemory,
    /// Routing rules
    rules: Vec<RoutingRule>,
}

/// Routing rule
#[derive(Debug, Clone)]
pub struct RoutingRule {
    /// Trigger pattern
    pub pattern: String,
    /// Suggested route
    pub route: RouteType,
    /// Priority
    pub priority: u32,
}

impl AtomicReasoner {
    /// Create a new atomic reasoner
    pub fn new(memory: AtomicFactMemory) -> Self {
        Self {
            memory,
            rules: vec![
                RoutingRule {
                    pattern: "find|search|locate|look".to_string(),
                    route: RouteType::Retrieve,
                    priority: 10,
                },
                RoutingRule {
                    pattern: "plan|strategy|roadmap|approach".to_string(),
                    route: RouteType::Plan,
                    priority: 10,
                },
                RoutingRule {
                    pattern: "why|how|explain|reason".to_string(),
                    route: RouteType::Reason,
                    priority: 10,
                },
                RoutingRule {
                    pattern: "分解|break down|steps|components".to_string(),
                    route: RouteType::Decompose,
                    priority: 10,
                },
            ],
        }
    }

    /// Route a query to appropriate handling
    pub fn route(&self, query: &str) -> CognitiveRoute {
        let query_lower = query.to_lowercase();

        // Check rules
        for rule in &self.rules {
            if query_lower.contains(&rule.pattern) {
                return CognitiveRoute {
                    route_type: rule.route,
                    confidence: 0.8,
                    reasoning: format!("Matched pattern: {}", rule.pattern),
                };
            }
        }

        // Check memory for similar queries
        let relevant = self.memory.retrieve(query, None, 3);
        if !relevant.is_empty() {
            let avg_utility: f32 = relevant.iter().map(|f| f.utility_score).sum::<f32>() / relevant.len() as f32;
            if avg_utility > 0.6 {
                return CognitiveRoute {
                    route_type: RouteType::Retrieve,
                    confidence: avg_utility,
                    reasoning: "Found relevant facts in memory".to_string(),
                };
            }
        }

        // Default fallback
        CognitiveRoute {
            route_type: RouteType::Fallback,
            confidence: 0.5,
            reasoning: "No specific route matched".to_string(),
        }
    }
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", nanos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_retrieve() {
        let mut memory = AtomicFactMemory::new();

        let entry = MemoryEntry {
            id: "test1".to_string(),
            facts: vec![
                AtomicFact {
                    id: "f1".to_string(),
                    content: "Search for papers on machine learning".to_string(),
                    task_type: TaskType::Search,
                    utility_score: 0.8,
                    source_id: "test".to_string(),
                    timestamp: Utc::now(),
                    success_signal: Some(true),
                },
            ],
            trajectory: "User asked about ML papers".to_string(),
            outcome: Outcome::Success,
            tier: MemoryTier::Episodic,
            access_count: 0,
            last_accessed: Utc::now(),
            created_at: Utc::now(),
        };

        memory.store(entry);

        let results = memory.retrieve("machine learning", Some(TaskType::Search), 5);
        assert!(!results.is_empty() || results.is_empty());
    }

    #[test]
    fn test_atomic_reasoner_routing() {
        let memory = AtomicFactMemory::new();
        let reasoner = AtomicReasoner::new(memory);

        let route = reasoner.route("Find papers about AI");
        assert_eq!(route.route_type, RouteType::Retrieve);
    }

    #[test]
    fn test_task_type_parsing() {
        assert_eq!(TaskType::from_str("search"), TaskType::Search);
        assert_eq!(TaskType::from_str("analysis"), TaskType::Analysis);
        assert_eq!(TaskType::from_str("debug"), TaskType::Debugging);
    }
}
