//! Rairos Research Memory — Research stance tracking and anomaly detection
#![allow(dead_code)]
#![allow(clippy::too_many_arguments)]
//!
//! Translates: llm/research_memory.py (410 LOC)
//!
//! Tracks research decisions over time and detects contradictions when new papers appear.

pub use crate::alert::AnomalyAlert;
pub use crate::memory::ResearchMemory;
pub use crate::memory_stats::MemoryStats;
pub use crate::stance::{AnomalySeverity, ResearchStance, StanceType};

mod alert;
mod memory;
pub mod memory_stats;
mod stance;

