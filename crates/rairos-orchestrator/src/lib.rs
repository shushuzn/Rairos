//! rairos-orchestrator

pub mod error;
pub mod persistence;
pub mod state;
pub mod orchestrator;

#[cfg(feature = "cortex-integration")]
pub mod cortex_bridge;

pub use error::{OrchestratorError, Result};
pub use orchestrator::AutonomousOrchestrator;
pub use state::{
    DeepResearchResult, FilterStats, GenePoolStats, OrchestratorConfig, OrchestratorState,
    PaperInfo, ResearchAlert, ScoredGap,
};
pub use persistence::{get_state_path, load_state, save_state};

#[cfg(feature = "cortex-integration")]
pub use cortex_bridge::{CortexBridge, CortexBridgeConfig};