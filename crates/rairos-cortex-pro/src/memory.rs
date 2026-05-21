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
use std::collections::VecDeque;
use std::fmt::Debug;
use std::sync::RwLock;
use chrono::{DateTime, Utc};

/// Maximum entries in each memory store
const MAX_IDEATION_ENTRIES: usize = 1000;
const MAX_EXPERIMENTATION_ENTRIES: usize = 500;

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

    /// Get all ideation entries
    pub fn get_ideation(&self) -> Vec<IdeationEntry> {
        self.ideation.read().unwrap().iter().cloned().collect()
    }

    /// Get all experimentation entries
    pub fn get_experimentation(&self) -> Vec<ExperimentationEntry> {
        self.experimentation.read().unwrap().iter().cloned().collect()
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
