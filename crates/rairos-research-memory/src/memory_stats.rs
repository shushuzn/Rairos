//! Memory statistics types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Overall memory statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_stances: usize,
    pub stance_breakdown: HashMap<String, usize>,
    pub total_anomalies: usize,
    pub recent_anomalies: usize,
}
