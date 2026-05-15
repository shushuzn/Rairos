//! `rairos_core::rate_limiter` — re-export of `rairos_rate_limiter`.
//!
//! This module provides a unified path through `rairos_core` for consumers
//! that prefer not to depend on the internal crate directly.

#[doc(inline)]
pub use rairos_rate_limiter::*;
