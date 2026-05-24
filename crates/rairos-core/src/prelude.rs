//! Rairos Core Prelude — commonly used types and functions
//!
//! Import with: `use rairos_core::prelude::*;`

pub use crate::{
    Database,
    Paper,
    PaperMetadata,
    ParseStatus,
    ResearchGap,
    SearchResult,
    DbStats,
    CoreError,
    Result,
    cosine_similarity,
};
pub use crate::constants::*;
pub use crate::identifiers::*;
pub use crate::crossover::CapsuleGene;
