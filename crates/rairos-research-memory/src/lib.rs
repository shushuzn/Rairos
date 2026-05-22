//! Rairos Research Memory — Research stance tracking and anomaly detection
#![allow(dead_code)]
#![allow(clippy::too_many_arguments)]
//!
//! Translates: llm/research_memory.py (410 LOC)
//!
//! Tracks research decisions over time and detects contradictions when new papers appear.
//!
//! ## Modules
//!
//! - [`belief_network`] - Structured belief tracking (Hindsight-inspired)
//! - [`stance`] - Research stance types
//! - [`alert`] - Anomaly alerts for contradictions

pub use crate::alert::AnomalyAlert;
pub use crate::memory::{BeliefStats, ResearchMemory};
pub use crate::memory_stats::MemoryStats;
pub use crate::stance::{AnomalySeverity, ResearchStance, StanceType};

// Belief network exports
pub use belief_network::{
    Belief, BeliefState, EntitySummary, EntityType, Reflection, ReflectionType,
};

mod alert;
pub mod belief_network;
mod memory;
pub mod memory_stats;
mod stance;

