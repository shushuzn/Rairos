//! Insight system — gene pool evolution, capsule storage, credibility scoring, and cards.
//!
//! Inlined from the `rairos-insight-*` crate family.

pub mod cards;
pub mod credibility;
pub mod crossover;
pub mod evolution;
pub mod storage;
pub mod tracker;
pub mod types;

// Convenience re-exports for the most commonly used types.
pub use cards::InsightManager;
pub use credibility::CredibilityScorer;
pub use crossover::CapsuleGene;
pub use evolution::EvolutionEngine;
pub use storage::CapsuleStorage;
pub use tracker::EvolutionTracker;
pub use types::{EvolutionEvent, ExplorationAction, UserPreferenceProfile};
