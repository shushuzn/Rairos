//! Rate Limiting

use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

use crate::models::Tier;

const DAILY_WINDOW_SECS: u64 = 86400;
const MINUTE_WINDOW_SECS: u64 = 60;

#[derive(Clone)]
pub struct RateLimiter {
    in_memory: Arc<RwLock<InMemoryRateLimiter>>,
}

struct InMemoryRateLimiter {
    daily: HashMap<String, u32>,
    minute: HashMap<String, u32>,
}

impl Default for InMemoryRateLimiter {
    fn default() -> Self {
        Self {
            daily: HashMap::new(),
            minute: HashMap::new(),
        }
    }
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            in_memory: Arc::new(RwLock::new(InMemoryRateLimiter::default())),
        }
    }

    #[allow(dead_code)]
    pub async fn check_rate_limit(
        &self,
        key_hash: &str,
        tier: Tier,
    ) -> Option<(u32, chrono::DateTime<chrono::Utc>)> {
        let limits = get_tier_limits(tier);
        let limiter = self.in_memory.read().await;

        if let Some(count) = limiter.daily.get(key_hash) {
            if *count >= limits.daily {
                return Some((limits.daily, chrono::Utc::now() + chrono::Duration::days(1)));
            }
        }

        if let Some(count) = limiter.minute.get(key_hash) {
            if *count >= limits.minute {
                return Some((limits.minute, chrono::Utc::now() + chrono::Duration::minutes(1)));
            }
        }

        None
    }

    #[allow(dead_code)]
    pub async fn record_request(&self, key_hash: &str) {
        let mut limiter = self.in_memory.write().await;
        *limiter.daily.entry(key_hash.to_string()).or_insert(0) += 1;
        *limiter.minute.entry(key_hash.to_string()).or_insert(0) += 1;
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

struct TierLimits {
    daily: u32,
    minute: u32,
}

fn get_tier_limits(tier: Tier) -> TierLimits {
    match tier {
        Tier::Free => TierLimits {
            daily: 100,
            minute: 10,
        },
        Tier::Pro => TierLimits {
            daily: 10_000,
            minute: 1000,
        },
        Tier::Team => TierLimits {
            daily: 100_000,
            minute: 10_000,
        },
        Tier::Enterprise => TierLimits {
            daily: u32::MAX,
            minute: u32::MAX,
        },
    }
}

pub struct RateLimitLayer;

impl RateLimitLayer {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_limits() {
        let free = get_tier_limits(Tier::Free);
        assert_eq!(free.daily, 100);
        assert_eq!(free.minute, 10);

        let pro = get_tier_limits(Tier::Pro);
        assert_eq!(pro.daily, 10_000);
        assert_eq!(pro.minute, 1_000);

        let enterprise = get_tier_limits(Tier::Enterprise);
        assert_eq!(enterprise.daily, u32::MAX);
    }
}
