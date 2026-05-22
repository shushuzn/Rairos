//! Memory module for SparksCrew - inspired by EvoScientist's persistent memory architecture.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                    MemoryBank                           │
//! │  ┌─────────────────┐    ┌─────────────────────────┐   │
//! │  │ IdeationMemory  │    │ ExperimentationMemory   │   │
//! │  │                 │    │                         │   │
//! │  │ - research_dirs │    │ - data_processing       │   │
//! │  │ - failed_dirs  │    │ - model_strategies      │   │
//! │  │ - top_ideas   │    │ - code_search_traject   │   │
//! │  └─────────────────┘    └─────────────────────────┘   │
//! │                         │                              │
//! │                         ▼                              │
//! │              ┌──────────────────┐                      │
//! │              │  Memory Distiller │                      │
//! │              │  (EMA-style)     │                      │
//! │              └──────────────────┘                      │
//! └─────────────────────────────────────────────────────────┘
//! ```

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::borrow::{Borrow, Cow};
use std::fmt::Debug;
use std::sync::RwLock;
use chrono::{DateTime, Utc};

use crate::utils::uuid_simple;

/// Maximum entries in each memory store
const MAX_IDEATION_ENTRIES: usize = 1000;
const MAX_EXPERIMENTATION_ENTRIES: usize = 500;

/// Bounded cache with simple LRU eviction (FIFO - oldest entry removed first)
/// Used to prevent unbounded memory growth in HashMaps
struct BoundedCache<K, V> {
    map: HashMap<K, V>,
    order: VecDeque<K>, // Most recent at end (LRU: end = most recently used)
    capacity: usize,
}

impl<K: Eq + Hash + Clone, V> BoundedCache<K, V> {
    fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            capacity,
        }
    }

    fn insert(&mut self, k: K, v: V) {
        // If key exists, update and move to end (most recent)
        if self.map.contains_key(&k) {
            self.map.insert(k.clone(), v);
            // Remove old position and push to end
            if let Some(pos) = self.order.iter().position(|x| x == &k) {
                self.order.remove(pos);
            }
            self.order.push(k);
            return;
        }

        // Evict oldest if at capacity
        if self.map.len() >= self.capacity {
            if let Some(oldest) = self.order.first() {
                self.map.remove(oldest);
                self.order.pop_front();
            }
        }

        self.map.insert(k.clone(), v);
        self.order.push(k);
    }

    fn get<Q: ?Sized>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.map.get(k)
    }

    fn get_mut<Q: ?Sized>(&mut self, k: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.map.get_mut(k)
    }

    fn remove<Q: ?Sized>(&mut self, k: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        if let Some(v) = self.map.remove(k) {
            if let Some(pos) = self.order.iter().position(|x| x.borrow() == k) {
                self.order.remove(pos);
            }
            return Some(v);
        }
        None
    }

    fn len(&self) -> usize {
        self.map.len()
    }

    fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    fn contains_key(&self, k: &K) -> bool {
        self.map.contains_key(k)
    }

    fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
    }

    fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.map.iter()
    }
}

impl<K: Eq + Hash + Clone, V> Default for BoundedCache<K, V> {
    fn default() -> Self {
        Self::new(1000)
    }
}

/// A single ideation memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeationEntry {
    /// Research direction summary
    pub direction: String,
    /// Why this direction was promising
    pub rationale: String,
    /// Outcome status
    pub status: IdeationStatus,
    /// When this was recorded
    pub timestamp: DateTime<Utc>,
    /// Feedback from critic (if any)
    pub feedback: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IdeationStatus {
    /// Still being explored
    Active,
    /// Successfully validated
    Validated,
    /// Failed or abandoned
    Failed,
}

/// A single experimentation memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentationEntry {
    /// What was attempted
    pub strategy: String,
    /// Data processing or model training approach
    pub approach: String,
    /// Code or implementation used
    pub code_reference: Option<String>,
    /// Effectiveness score (0.0 - 1.0)
    pub effectiveness: f32,
    /// When this was recorded
    pub timestamp: DateTime<Utc>,
    /// Key learnings
    pub learnings: Vec<String>,
}

/// Memory bank holding both ideation and experimentation memories
pub struct MemoryBank {
    /// Research direction memories
    ideation: RwLock<VecDeque<IdeationEntry>>,
    /// Experimentation strategy memories
    experimentation: RwLock<VecDeque<ExperimentationEntry>>,
}

impl Clone for MemoryBank {
    fn clone(&self) -> Self {
        Self {
            ideation: RwLock::new(VecDeque::new()),
            experimentation: RwLock::new(VecDeque::new()),
        }
    }
}

impl Debug for MemoryBank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryBank")
            .field("ideation_count", &self.ideation.read().unwrap().len())
            .field("experimentation_count", &self.experimentation.read().unwrap().len())
            .finish()
    }
}

impl Default for MemoryBank {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryBank {
    /// Create a new empty memory bank
    pub fn new() -> Self {
        Self {
            ideation: RwLock::new(VecDeque::with_capacity(MAX_IDEATION_ENTRIES)),
            experimentation: RwLock::new(VecDeque::with_capacity(MAX_EXPERIMENTATION_ENTRIES)),
        }
    }

    /// Add an ideation entry
    pub fn add_ideation(&self, entry: IdeationEntry) {
        let mut ideation = self.ideation.write().unwrap();
        if ideation.len() >= MAX_IDEATION_ENTRIES {
            ideation.pop_back();
        }
        ideation.push_front(entry);
    }

    /// Add an experimentation entry
    pub fn add_experimentation(&self, entry: ExperimentationEntry) {
        let mut experimentation = self.experimentation.write().unwrap();
        if experimentation.len() >= MAX_EXPERIMENTATION_ENTRIES {
            experimentation.pop_back();
        }
        experimentation.push_front(entry);
    }

    /// Get all ideation entries (zero-copy read via Cow)
    pub fn get_ideation(&self) -> Cow<'_, [IdeationEntry]> {
        let deque = self.ideation.read().unwrap();
        let (front, back) = deque.as_slices();
        if back.is_empty() {
            Cow::Borrowed(front)
        } else {
            // VecDeque is wrapped - need to clone to get contiguous memory
            Cow::Owned(deque.iter().cloned().collect())
        }
    }

    /// Get all experimentation entries (zero-copy read via Cow)
    pub fn get_experimentation(&self) -> Cow<'_, [ExperimentationEntry]> {
        let deque = self.experimentation.read().unwrap();
        let (front, back) = deque.as_slices();
        if back.is_empty() {
            Cow::Borrowed(front)
        } else {
            // VecDeque is wrapped - need to clone to get contiguous memory
            Cow::Owned(deque.iter().cloned().collect())
        }
    }

    /// Get active ideation directions (for hypothesis generation)
    pub fn get_active_directions(&self) -> Vec<String> {
        self.ideation
            .read()
            .unwrap()
            .iter()
            .filter(|e| e.status == IdeationStatus::Active)
            .map(|e| e.direction.clone())
            .take(10)
            .collect()
    }

    /// Get top successful ideation entries
    pub fn get_successful_directions(&self) -> Vec<String> {
        self.ideation
            .read()
            .unwrap()
            .iter()
            .filter(|e| e.status == IdeationStatus::Validated)
            .map(|e| e.direction.clone())
            .take(5)
            .collect()
    }

    /// Get failed directions (to avoid repeating)
    pub fn get_failed_directions(&self) -> Vec<String> {
        self.ideation
            .read()
            .unwrap()
            .iter()
            .filter(|e| e.status == IdeationStatus::Failed)
            .map(|e| e.direction.clone())
            .collect()
    }

    /// Get top effective experimentation strategies
    pub fn get_effective_strategies(&self) -> Vec<(String, f32)> {
        let experimentation = self.experimentation.read().unwrap();
        let mut strategies: Vec<(String, f32)> = experimentation
            .iter()
            .map(|e| (e.strategy.clone(), e.effectiveness))
            .collect();
        strategies.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        strategies.into_iter().take(5).collect()
    }

    /// Retrieve relevant experiments based on a query
    pub fn retrieve_relevant_experiments(&self, query: &str) -> Vec<ExperimentationEntry> {
        let query_lower = query.to_lowercase();
        self.experimentation
            .read()
            .unwrap()
            .iter()
            .filter(|e| {
                e.strategy.to_lowercase().contains(&query_lower)
                    || e.approach.to_lowercase().contains(&query_lower)
                    || e.learnings.iter().any(|l| l.to_lowercase().contains(&query_lower))
            })
            .take(3)
            .cloned()
            .collect()
    }

    /// Update ideation entry status
    pub fn update_ideation_status(&self, direction: &str, status: IdeationStatus) {
        let mut ideation = self.ideation.write().unwrap();
        for entry in ideation.iter_mut() {
            if entry.direction == direction {
                entry.status = status;
                break;
            }
        }
    }

    /// Clear all memories
    pub fn clear(&self) {
        self.ideation.write().unwrap().clear();
        self.experimentation.write().unwrap().clear();
    }

    /// Get memory statistics
    pub fn stats(&self) -> MemoryStats {
        MemoryStats {
            ideation_count: self.ideation.read().unwrap().len(),
            experimentation_count: self.experimentation.read().unwrap().len(),
            ideation_active: self
                .ideation
                .read()
                .unwrap()
                .iter()
                .filter(|e| e.status == IdeationStatus::Active)
                .count(),
            ideation_validated: self
                .ideation
                .read()
                .unwrap()
                .iter()
                .filter(|e| e.status == IdeationStatus::Validated)
                .count(),
            ideation_failed: self
                .ideation
                .read()
                .unwrap()
                .iter()
                .filter(|e| e.status == IdeationStatus::Failed)
                .count(),
            avg_experiment_effectiveness: {
                let experimentation = self.experimentation.read().unwrap();
                if experimentation.is_empty() {
                    0.0
                } else {
                    let sum: f32 = experimentation.iter().map(|e| e.effectiveness).sum();
                    sum / experimentation.len() as f32
                }
            },
        }
    }
}

/// Memory statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub ideation_count: usize,
    pub experimentation_count: usize,
    pub ideation_active: usize,
    pub ideation_validated: usize,
    pub ideation_failed: usize,
    pub avg_experiment_effectiveness: f32,
}

/// Builder for creating memory entries
pub struct MemoryEntryBuilder {
    direction: Option<String>,
    rationale: Option<String>,
    status: IdeationStatus,
    feedback: Option<String>,
}

impl MemoryEntryBuilder {
    pub fn new() -> Self {
        Self {
            direction: None,
            rationale: None,
            status: IdeationStatus::Active,
            feedback: None,
        }
    }

    pub fn direction(mut self, direction: impl Into<String>) -> Self {
        self.direction = Some(direction.into());
        self
    }

    pub fn rationale(mut self, rationale: impl Into<String>) -> Self {
        self.rationale = Some(rationale.into());
        self
    }

    pub fn status(mut self, status: IdeationStatus) -> Self {
        self.status = status;
        self
    }

    pub fn feedback(mut self, feedback: impl Into<String>) -> Self {
        self.feedback = Some(feedback.into());
        self
    }

    pub fn build(self) -> IdeationEntry {
        IdeationEntry {
            direction: self.direction.unwrap_or_default(),
            rationale: self.rationale.unwrap_or_default(),
            status: self.status,
            timestamp: Utc::now(),
            feedback: self.feedback,
        }
    }
}

impl Default for MemoryEntryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for experimentation entries
pub struct ExperimentationEntryBuilder {
    strategy: Option<String>,
    approach: Option<String>,
    code_reference: Option<String>,
    effectiveness: f32,
    learnings: Vec<String>,
}

impl ExperimentationEntryBuilder {
    pub fn new() -> Self {
        Self {
            strategy: None,
            approach: None,
            code_reference: None,
            effectiveness: 0.5,
            learnings: Vec::new(),
        }
    }

    pub fn strategy(mut self, strategy: impl Into<String>) -> Self {
        self.strategy = Some(strategy.into());
        self
    }

    pub fn approach(mut self, approach: impl Into<String>) -> Self {
        self.approach = Some(approach.into());
        self
    }

    pub fn code_reference(mut self, code_reference: impl Into<String>) -> Self {
        self.code_reference = Some(code_reference.into());
        self
    }

    pub fn effectiveness(mut self, effectiveness: f32) -> Self {
        self.effectiveness = effectiveness;
        self
    }

    pub fn add_learning(mut self, learning: impl Into<String>) -> Self {
        self.learnings.push(learning.into());
        self
    }

    pub fn build(self) -> ExperimentationEntry {
        ExperimentationEntry {
            strategy: self.strategy.unwrap_or_default(),
            approach: self.approach.unwrap_or_default(),
            code_reference: self.code_reference,
            effectiveness: self.effectiveness,
            timestamp: Utc::now(),
            learnings: self.learnings,
        }
    }
}

impl Default for ExperimentationEntryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_bank_ideation() {
        let bank = MemoryBank::new();

        // Add ideation entries
        bank.add_ideation(
            MemoryEntryBuilder::new()
                .direction("Thermoelectric materials for waste heat recovery")
                .rationale("High ZT values achievable through nanostructuring")
                .status(IdeationStatus::Active)
                .build(),
        );

        bank.add_ideation(
            MemoryEntryBuilder::new()
                .direction("Perovskite solar cells with stable efficiency")
                .rationale("Lead-free alternatives show promise")
                .status(IdeationStatus::Failed)
                .feedback("Environmental stability issues")
                .build(),
        );

        // Check stats
        let stats = bank.stats();
        assert_eq!(stats.ideation_count, 2);
        assert_eq!(stats.ideation_active, 1);
        assert_eq!(stats.ideation_failed, 1);

        // Get active directions
        let active = bank.get_active_directions();
        assert_eq!(active.len(), 1);
        assert!(active[0].contains("Thermoelectric"));

        // Get failed directions
        let failed = bank.get_failed_directions();
        assert_eq!(failed.len(), 1);
        assert!(failed[0].contains("Perovskite"));
    }

    #[test]
    fn test_memory_bank_experimentation() {
        let bank = MemoryBank::new();

        // Add experimentation entries
        bank.add_experimentation(
            ExperimentationEntryBuilder::new()
                .strategy("CGCNN for ZT prediction")
                .approach("Graph convolution on crystal structures")
                .effectiveness(0.85)
                .add_learning("Voronoi tessellation improves accuracy")
                .build(),
        );

        bank.add_experimentation(
            ExperimentationEntryBuilder::new()
                .strategy("DFT relaxation for structure optimization")
                .approach("VASP with PBE functional")
                .effectiveness(0.92)
                .add_learning("500 eV cutoff is sufficient for convergence")
                .build(),
        );

        // Check stats
        let stats = bank.stats();
        assert_eq!(stats.experimentation_count, 2);
        assert!((stats.avg_experiment_effectiveness - 0.885).abs() < 0.01);

        // Get effective strategies
        let strategies = bank.get_effective_strategies();
        assert_eq!(strategies.len(), 2);
        assert!(strategies[0].1 > strategies[1].1);
    }

    #[test]
    fn test_retrieve_relevant_experiments() {
        let bank = MemoryBank::new();

        bank.add_experimentation(
            ExperimentationEntryBuilder::new()
                .strategy("Thermoelectric property prediction")
                .approach("Machine learning with composition features")
                .effectiveness(0.80)
                .build(),
        );

        bank.add_experimentation(
            ExperimentationEntryBuilder::new()
                .strategy("Solar cell efficiency optimization")
                .approach("Band gap engineering through alloying")
                .effectiveness(0.75)
                .build(),
        );

        // Retrieve relevant experiments
        let relevant = bank.retrieve_relevant_experiments("thermoelectric");
        assert!(!relevant.is_empty());
        assert!(relevant[0].strategy.contains("Thermoelectric"));

        // Retrieve with no match
        let empty = bank.retrieve_relevant_experiments("battery");
        assert!(empty.is_empty());
    }
}

// =============================================================================
// DeepAgent-style Memory Tiers (arXiv:2510.21618)
// =============================================================================

/// Memory tier type (inspired by DeepAgent's episodic/working/tool memories)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryTier {
    /// Short-term working memory - current context, active tasks
    Working,
    /// Episodic memory - past experiences, completed tasks
    Episodic,
    /// Tool memory - tool usage patterns, effectiveness scores
    Tool,
    /// Long-term semantic memory - domain knowledge, learned facts
    Semantic,
}

/// Entry for tiered memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TieredMemoryEntry {
    /// Unique entry ID
    pub id: String,
    /// Memory tier
    pub tier: MemoryTier,
    /// Content
    pub content: String,
    /// Importance score (0.0 - 1.0)
    pub importance: f32,
    /// Last access time
    pub last_accessed: DateTime<Utc>,
    /// Access count
    pub access_count: u32,
    /// Associated tool (if tool memory)
    pub tool_name: Option<String>,
    /// TTL in seconds (0 = no expiry)
    pub ttl_seconds: u64,
}

impl TieredMemoryEntry {
    /// Check if entry has expired
    pub fn is_expired(&self) -> bool {
        if self.ttl_seconds == 0 {
            return false;
        }
        let age = Utc::now() - self.last_accessed;
        age.num_seconds() as u64 > self.ttl_seconds
    }
}

/// DeepAgent-style tiered memory system
///
/// Based on arXiv:2510.21618 - "DeepAgent: A General Reasoning Agent with Scalable Toolsets"
/// which proposes autonomous memory folding with:
/// - Episodic memory: past experiences
/// - Working memory: current context
/// - Tool memory: tool usage patterns
pub struct TieredMemory {
    /// Working memory (short-term)
    working: RwLock<VecDeque<TieredMemoryEntry>>,
    /// Episodic memory (past experiences)
    episodic: RwLock<VecDeque<TieredMemoryEntry>>,
    /// Tool memory (tool usage patterns)
    tool: RwLock<BoundedCache<String, ToolMemoryEntry>>,
    /// Semantic memory (domain knowledge)
    semantic: RwLock<BoundedCache<String, String>>,
    /// Maximum entries per tier
    max_working: usize,
    max_episodic: usize,
}

/// Tool-specific memory for tracking effectiveness
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMemoryEntry {
    /// Tool name
    pub tool_name: String,
    /// Total usage count
    pub usage_count: u32,
    /// Successful usage count
    pub success_count: u32,
    /// Average execution time (ms)
    pub avg_exec_time_ms: f64,
    /// Last used timestamp
    pub last_used: DateTime<Utc>,
    /// Success rate
    pub success_rate: f32,
    /// Average reward from tool use
    pub avg_reward: f32,
}

/// Reflection entry storing analyzed lessons from experiences
/// (Based on ERL, R³ papers)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionEntry {
    /// Reflection ID
    pub id: String,
    /// What happened (situation)
    pub situation: String,
    /// What was tried
    pub action: String,
    /// What was the outcome
    pub outcome: String,
    /// Lesson learned
    pub lesson: String,
    /// When this was recorded
    pub timestamp: DateTime<Utc>,
    /// Confidence in this reflection (0.0 - 1.0)
    pub confidence: f32,
}

impl ToolMemoryEntry {
    pub fn new(tool_name: &str) -> Self {
        Self {
            tool_name: tool_name.to_string(),
            usage_count: 0,
            success_count: 0,
            avg_exec_time_ms: 0.0,
            last_used: Utc::now(),
            success_rate: 0.0,
            avg_reward: 0.0,
        }
    }

    /// Record a tool usage
    pub fn record_usage(&mut self, success: bool, exec_time_ms: f64, reward: f32) {
        self.usage_count += 1;
        if success {
            self.success_count += 1;
        }
        // Update running average
        self.avg_exec_time_ms = (self.avg_exec_time_ms * (self.usage_count - 1) as f64 + exec_time_ms)
            / self.usage_count as f64;
        self.avg_reward = (self.avg_reward * (self.usage_count - 1) as f32 + reward) / self.usage_count as f32;
        self.success_rate = self.success_count as f32 / self.usage_count as f32;
        self.last_used = Utc::now();
    }
}

impl Default for TieredMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl TieredMemory {
    /// Create a new tiered memory system
    pub fn new() -> Self {
        Self {
            working: RwLock::new(VecDeque::with_capacity(100)),
            episodic: RwLock::new(VecDeque::with_capacity(500)),
            tool: RwLock::new(BoundedCache::new(1000)), // Tool registry: 1000 entries max
            semantic: RwLock::new(BoundedCache::new(10000)), // Semantic memory: 10000 entries max
            max_working: 50,
            max_episodic: 200,
        }
    }

    /// Add entry to working memory
    pub fn add_working(&self, content: &str, importance: f32) {
        let entry = TieredMemoryEntry {
            id: uuid_simple(),
            tier: MemoryTier::Working,
            content: content.to_string(),
            importance,
            last_accessed: Utc::now(),
            access_count: 1,
            tool_name: None,
            ttl_seconds: 300, // 5 minutes default
        };

        let mut working = self.working.write().unwrap();
        working.push_front(entry);
        while working.len() > self.max_working {
            working.pop_back();
        }
    }

    /// Add entry to episodic memory
    pub fn add_episodic(&self, content: &str, importance: f32) {
        let entry = TieredMemoryEntry {
            id: uuid_simple(),
            tier: MemoryTier::Episodic,
            content: content.to_string(),
            importance,
            last_accessed: Utc::now(),
            access_count: 1,
            tool_name: None,
            ttl_seconds: 0, // No expiry
        };

        let mut episodic = self.episodic.write().unwrap();
        episodic.push_front(entry);
        while episodic.len() > self.max_episodic {
            episodic.pop_back();
        }
    }

    /// Record tool usage
    pub fn record_tool_usage(&self, tool_name: &str, success: bool, exec_time_ms: f64, reward: f32) {
        let mut tool_mem = self.tool.write().unwrap();
        if let Some(entry) = tool_mem.get_mut(&tool_name.to_string()) {
            entry.record_usage(success, exec_time_ms, reward);
        } else {
            let mut new_entry = ToolMemoryEntry::new(tool_name);
            new_entry.record_usage(success, exec_time_ms, reward);
            tool_mem.insert(tool_name.to_string(), new_entry);
        }
    }

    /// Store semantic knowledge
    pub fn store_knowledge(&self, key: &str, value: &str) {
        let mut semantic = self.semantic.write().unwrap();
        semantic.insert(key.to_string(), value.to_string());
    }

    /// Retrieve knowledge
    pub fn retrieve_knowledge(&self, key: &str) -> Option<String> {
        let semantic = self.semantic.read().unwrap();
        semantic.get(key).cloned()
    }

    /// Get working memory entries
    pub fn get_working(&self) -> Vec<TieredMemoryEntry> {
        let working = self.working.read().unwrap();
        working.iter().filter(|e| !e.is_expired()).cloned().collect()
    }

    /// Get episodic memory (recent experiences)
    pub fn get_episodic(&self, limit: usize) -> Vec<TieredMemoryEntry> {
        let episodic = self.episodic.read().unwrap();
        episodic.iter().take(limit).cloned().collect()
    }

    /// Get tool memory
    pub fn get_tool_memory(&self, tool_name: &str) -> Option<ToolMemoryEntry> {
        let tool_mem = self.tool.read().unwrap();
        tool_mem.get(tool_name).cloned()
    }

    /// Get all tool memories sorted by success rate
    pub fn get_effective_tools(&self, limit: usize) -> Vec<(String, f32)> {
        let tool_mem = self.tool.read().unwrap();
        let mut tools: Vec<_> = tool_mem
            .iter()
            .map(|(name, entry)| (name.clone(), entry.success_rate))
            .collect();
        tools.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        tools.into_iter().take(limit).collect()
    }

    /// Consolidate working memory to episodic (memory folding)
    pub fn consolidate(&self) {
        let working = {
            let w = self.working.read().unwrap();
            w.iter().filter(|e| e.importance > 0.5).cloned().collect::<Vec<_>>()
        };

        if !working.is_empty() {
            let mut episodic = self.episodic.write().unwrap();
            for entry in working {
                let mut e = entry;
                e.tier = MemoryTier::Episodic;
                e.ttl_seconds = 0;
                episodic.push_front(e);
            }
            while episodic.len() > self.max_episodic {
                episodic.pop_back();
            }
        }

        // Clear expired working memory
        {
            let mut working = self.working.write().unwrap();
            working.retain(|e| !e.is_expired());
        }
    }

    /// Access a memory entry (updates last_accessed)
    pub fn access(&self, entry_id: &str) -> Option<TieredMemoryEntry> {
        // Check working memory
        {
            let mut working = self.working.write().unwrap();
            if let Some(entry) = working.iter_mut().find(|e| e.id == entry_id) {
                entry.access_count += 1;
                entry.last_accessed = Utc::now();
                return Some(entry.clone());
            }
        }

        // Check episodic memory
        {
            let mut episodic = self.episodic.write().unwrap();
            if let Some(entry) = episodic.iter_mut().find(|e| e.id == entry_id) {
                entry.access_count += 1;
                entry.last_accessed = Utc::now();
                return Some(entry.clone());
            }
        }

        None
    }

    /// Get memory statistics
    pub fn tiered_stats(&self) -> TieredMemoryStats {
        TieredMemoryStats {
            working_count: self.working.read().unwrap().len(),
            episodic_count: self.episodic.read().unwrap().len(),
            tool_count: self.tool.read().unwrap().len(),
            semantic_count: self.semantic.read().unwrap().len(),
        }
    }

    // =============================================================================
    // Episodic Reflection (Based on ERL, R³ papers)
    // =============================================================================

    /// Store a reflection from experience analysis
    pub fn add_reflection(&self, situation: &str, action: &str, outcome: &str, lesson: &str, confidence: f32) {
        let reflection = ReflectionEntry {
            id: uuid_simple(),
            situation: situation.to_string(),
            action: action.to_string(),
            outcome: outcome.to_string(),
            lesson: lesson.to_string(),
            timestamp: Utc::now(),
            confidence,
        };

        // Store as episodic memory with special format
        let content = format!(
            "REFLECTION: [{}] {} → {} | Lesson: {} (conf:{:.2})",
            situation, action, outcome, lesson, confidence
        );

        let entry = TieredMemoryEntry {
            id: reflection.id,
            tier: MemoryTier::Episodic,
            content,
            importance: confidence,
            last_accessed: Utc::now(),
            access_count: 1,
            tool_name: None,
            ttl_seconds: 0,
        };

        let mut episodic = self.episodic.write().unwrap();
        episodic.push_front(entry);
        while episodic.len() > self.max_episodic {
            episodic.pop_back();
        }
    }

    /// Get reflections relevant to a query
    pub fn get_reflections(&self, query: &str, limit: usize) -> Vec<String> {
        let episodic = self.episodic.read().unwrap();
        let query_lower = query.to_lowercase();

        episodic
            .iter()
            .filter(|e| e.content.contains("REFLECTION") || e.content.to_lowercase().contains(&query_lower))
            .take(limit)
            .map(|e| e.content.clone())
            .collect()
    }

    /// Consolidate episodic memories into actionable reflections
    /// This distills multiple similar experiences into a single lesson
    pub fn consolidate_to_reflections(&self, theme: &str) -> Vec<String> {
        let episodic = self.episodic.read().unwrap();

        // Find episodic entries matching the theme
        let theme_lower = theme.to_lowercase();
        let matching: Vec<_> = episodic
            .iter()
            .filter(|e| e.content.to_lowercase().contains(&theme_lower))
            .collect();

        if matching.is_empty() {
            return vec![];
        }

        // Extract lessons (simplified - in real impl would use LLM)
        let mut lessons = Vec::new();

        // Group by outcome patterns
        let success_count = matching.iter().filter(|e| e.content.contains("Success")).count();
        let failure_count = matching.iter().filter(|e| e.content.contains("Failure")).count();

        if success_count > failure_count {
            lessons.push(format!(
                "For {}: {} successes vs {} failures - prioritize this approach",
                theme, success_count, failure_count
            ));
        } else if failure_count > success_count {
            lessons.push(format!(
                "For {}: {} failures vs {} successes - reconsider approach",
                theme, failure_count, success_count
            ));
        }

        // Extract common patterns
        if matching.len() >= 3 {
            lessons.push(format!(
                "Multiple experiences ({}) found for {} - consider as established pattern",
                matching.len(), theme
            ));
        }

        lessons
    }

    /// Generate a self-improvement suggestion based on memory history
    pub fn suggest_improvement(&self) -> Option<String> {
        let stats = self.tiered_stats();

        // Analyze tool effectiveness
        let effective_tools = self.get_effective_tools(3);

        if effective_tools.is_empty() {
            return None;
        }

        // Find tools with low success rate
        let problematic_tools: Vec<_> = effective_tools
            .iter()
            .filter(|(_, rate)| *rate < 0.5)
            .collect();

        if !problematic_tools.is_empty() {
            let tools: Vec<_> = problematic_tools.iter().map(|(name, _)| name.as_str()).collect();
            return Some(format!(
                "Consider improving or replacing tools with low success rate: {}",
                tools.join(", ")
            ));
        }

        // If most tools are effective, suggest exploration
        Some(format!(
            "Tools performing well ({} effective). Consider exploring new tool combinations.",
            effective_tools.len()
        ))
    }
}

/// Statistics for tiered memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TieredMemoryStats {
    pub working_count: usize,
    pub episodic_count: usize,
    pub tool_count: usize,
    pub semantic_count: usize,
}

#[cfg(test)]
mod tiered_memory_tests {
    use super::*;

    #[test]
    fn test_add_working_memory() {
        let mem = TieredMemory::new();
        mem.add_working("Current task: analyze Bi2Te3", 0.9);

        let working = mem.get_working();
        assert!(!working.is_empty());
        assert_eq!(working[0].content, "Current task: analyze Bi2Te3");
    }

    #[test]
    fn test_add_episodic_memory() {
        let mem = TieredMemory::new();
        mem.add_episodic("Completed DFT calculation for Bi2Se3", 0.8);

        let episodic = mem.get_episodic(10);
        assert!(!episodic.is_empty());
    }

    #[test]
    fn test_record_tool_usage() {
        let mem = TieredMemory::new();
        mem.record_tool_usage("materials_project", true, 150.0, 0.9);
        mem.record_tool_usage("materials_project", true, 200.0, 0.85);
        mem.record_tool_usage("cgcnn", false, 300.0, 0.3);

        let effective = mem.get_effective_tools(10);
        assert_eq!(effective[0].0, "materials_project");
        assert!(effective[0].1 > effective[1].1);
    }

    #[test]
    fn test_tool_memory_entry() {
        let mut entry = ToolMemoryEntry::new("test_tool");

        entry.record_usage(true, 100.0, 0.9);
        entry.record_usage(true, 200.0, 0.8);

        assert_eq!(entry.usage_count, 2);
        assert_eq!(entry.success_count, 2);
        assert!((entry.avg_exec_time_ms - 150.0).abs() < 0.1);
    }

    #[test]
    fn test_semantic_memory() {
        let mem = TieredMemory::new();
        mem.store_knowledge("thermoelectric_ZT", "Bi2Te3 has ZT ~ 1.1 at 300K");

        let knowledge = mem.retrieve_knowledge("thermoelectric_ZT");
        assert!(knowledge.is_some());
        assert!(knowledge.unwrap().contains("Bi2Te3"));
    }

    #[test]
    fn test_consolidate() {
        let mem = TieredMemory::new();
        mem.add_working("Important finding", 0.8); // High importance
        mem.add_working("Temp data", 0.2); // Low importance

        mem.consolidate();

        let episodic = mem.get_episodic(100);
        // High importance entries should be in episodic
        assert!(episodic.iter().any(|e| e.content == "Important finding"));
    }

    #[test]
    fn test_tiered_stats() {
        let mem = TieredMemory::new();
        mem.add_working("Working 1", 0.5);
        mem.add_episodic("Episodic 1", 0.5);
        mem.record_tool_usage("tool1", true, 100.0, 0.8);
        mem.store_knowledge("key1", "value1");

        let stats = mem.tiered_stats();
        assert_eq!(stats.working_count, 1);
        assert_eq!(stats.episodic_count, 1);
        assert_eq!(stats.tool_count, 1);
        assert_eq!(stats.semantic_count, 1);
    }

    #[test]
    fn test_memory_tier_enum() {
        assert_eq!(MemoryTier::Working, MemoryTier::Working);
        assert_ne!(MemoryTier::Working, MemoryTier::Episodic);
    }

    #[test]
    fn test_entry_expiry() {
        let entry = TieredMemoryEntry {
            id: "test".to_string(),
            tier: MemoryTier::Working,
            content: "Test".to_string(),
            importance: 0.5,
            last_accessed: Utc::now(),
            access_count: 1,
            tool_name: None,
            ttl_seconds: 0, // No expiry
        };

        assert!(!entry.is_expired());
    }
}
