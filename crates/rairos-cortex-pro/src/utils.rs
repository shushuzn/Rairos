//! Common utilities module for shared functionality across the crate.
//!
//! This module provides common utility functions that are used across multiple
//! modules to avoid code duplication.
//!
//! ## Contents
//!
//! - `uuid_simple()` - Generate simple UUID v4 strings
//! - `current_timestamp()` - Get current Unix timestamp in seconds
//! - `generate_id()` - Generate a unique ID with optional prefix

use std::time::{SystemTime, UNIX_EPOCH};

/// Generate a simple UUID v4 string.
/// Alias for `uuid::Uuid::new_v4().to_string()`.
pub fn uuid_simple() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Get current Unix timestamp in seconds.
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Get current Unix timestamp in milliseconds.
pub fn current_timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

/// Generate a unique ID with optional prefix.
///
/// # Examples
///
/// ```
/// let id = generate_id("user");
/// // Returns something like: "user-a1b2c3d4-e5f6-7890-abcd-ef1234567890"
/// ```
pub fn generate_id(prefix: &str) -> String {
    if prefix.is_empty() {
        uuid_simple()
    } else {
        format!("{}-{}", prefix, uuid_simple())
    }
}

/// Format a timestamp as an RFC3339 string.
pub fn format_timestamp(time: u64) -> String {
    use chrono::{DateTime, Utc};
    let dt = DateTime::<Utc>::from_timestamp(time as i64, 0)
        .unwrap_or_default();
    dt.to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_simple() {
        let id1 = uuid_simple();
        let id2 = uuid_simple();
        assert_ne!(id1, id2);
        assert_eq!(id1.len(), 36); // UUID v4 format
    }

    #[test]
    fn test_current_timestamp() {
        let ts = current_timestamp();
        assert!(ts > 0);
    }

    #[test]
    fn test_generate_id_with_prefix() {
        let id = generate_id("agent");
        assert!(id.starts_with("agent-"));
        assert_eq!(id.len(), 7 + 36); // "agent-" + UUID
    }

    #[test]
    fn test_generate_id_without_prefix() {
        let id = generate_id("");
        assert_eq!(id.len(), 36);
    }
}
