//! Orphan utility crates inlined into rairos-llm.
//!
//! Each module corresponds to a former standalone crate that had no external consumers.
//! Inlined to reduce workspace fragmentation while preserving the original API surface.

pub mod achievements;
pub mod briefing_dist;
pub mod briefing_gen;
pub mod chat;
pub mod client;
pub mod climate;
pub mod contradiction;
pub mod credibility;
pub mod credibility_scorer;
pub mod cross_referencer;
pub mod distributor;
pub mod eval_gap_monitor;
pub mod game_mode;
pub mod gene_pool_decay;
pub mod impact_metrics;
pub mod impact_scorer;
pub mod litreview_base;
pub mod litreview_generator;
pub mod paradigm;
pub mod pool;
pub mod query;
pub mod query_types;
pub mod rankers;
pub mod rankers_base;
pub mod rankers_cosine;
pub mod rankers_score;
pub mod replication_check;
pub mod rigor;
pub mod scoring_momentum;
pub mod trend_analyzer;
