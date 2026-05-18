//! rairos-orchestrator

pub mod error;
pub mod persistence;
pub mod state;
pub mod orchestrator;

pub use error::{OrchestratorError, Result};
pub use orchestrator::AutonomousOrchestrator;
pub use state::{
    DeepResearchResult, FilterStats, GenePoolStats, OrchestratorConfig, OrchestratorState,
    PaperInfo, ResearchAlert, ScoredGap,
};
pub use persistence::{get_state_path, load_state, save_state};