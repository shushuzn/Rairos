//! CLI command handler implementations organized by domain.
//!
//! Each submodule contains handlers and helpers for a specific domain.

pub mod achievements;
pub use achievements::*;
pub mod game_mode;
pub use game_mode::*;
pub mod contradiction;
pub use contradiction::*;
pub mod trends;
pub use trends::*;
pub mod rigor;
pub use rigor::*;
pub mod impact;
pub use impact::*;
pub mod briefing;
pub use briefing::*;
pub mod paradigm;
pub use paradigm::*;
pub mod crossref;
pub use crossref::*;
pub mod momentum;
pub use momentum::*;
pub mod cite;
pub use cite::*;
pub mod evo;
pub use evo::*;
pub mod kg;
pub use kg::*;
pub mod llm;
pub use llm::*;
pub mod paper;
pub use paper::*;
pub mod research;
pub use research::*;
pub mod tools;
pub use tools::*;
pub mod util;
pub use util::*;
