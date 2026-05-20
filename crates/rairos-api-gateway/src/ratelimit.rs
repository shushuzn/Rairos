//! Rate Limiting
//!
//! Supports both in-memory (single instance) and Redis (distributed) rate limiting.

use std::sync::Arc;
use tokio::sync::RwLock;
use rustc_hash::FxHashMap;

use crate::models::Tier;

const DAILY_WINDOW_SECS: i64 = 86400;
const MINUTE_WINDOW_SECS: i64 = 60;

#[derive(Clone)]
pub struct RateLimiter {
    inner: Arc<RateLimiterInner>,
}

enum RateLimiterInner {
    Redis {
        client: redis::Client,
    },
    InMemory {
        daily: Arc<RwLock<FxHashMap<String, u32>>>,
        minute: Arc<RwLock<FxHashMap<String, u32>>>,
    },
}

impl RateLimiter {
    pub fn new(redis_client: Option<redis::Client>) -> Self {
        let inner = match redis_client {
            Some(client) => RateLimiterInner::Redis { client },
            None => RateLimiterInner::InMemory {
                daily: Arc::new(RwLock::new(FxHashMap::default())),
                minute: Arc::new(RwLock::new(FxHashMap::default())),
            },
        };

        Self {
            inner: Arc::new(inner),
        }
    }

    pub async fn check_rate_limit(
        &self,
        key_hash: &str,
        tier: Tier,
    ) -> Option<(u32, chrono::DateTime<chrono::Utc>)> {
        let limits = get_tier_limits(tier);

        match self.inner.as_ref() {
            RateLimiterInner::Redis { client } => {
                Self::check_redis_rate_limit(client, key_hash, limits).await
            }
            RateLimiterInner::InMemory { daily, minute } => {
                Self::check_in_memory_rate_limit(daily, minute, key_hash, limits).await
            }
        }
    }

    async fn check_redis_rate_limit(
        client: &redis::Client,
        key_hash: &str,
        limits: TierLimits,
    ) -> Option<(u32, chrono::DateTime<chrono::Utc>)> {
        let mut conn = match client.get_multiplexed_async_connection().await {
            Ok(conn) => conn,
            Err(_) => return None,
        };

        let daily_key = format!("ratelimit:daily:{}", key_hash);
        let minute_key = format!("ratelimit:minute:{}", key_hash);

        let daily_count: Option<u32> = redis::cmd("GET")
            .arg(&daily_key)
            .query_async(&mut conn)
            .await
            .ok();

        if let Some(count) = daily_count {
            if count >= limits.daily {
                return Some((limits.daily, chrono::Utc::now() + chrono::Duration::days(1)));
            }
        }

        let minute_count: Option<u32> = redis::cmd("GET")
            .arg(&minute_key)
            .query_async(&mut conn)
            .await
            .ok();

        if let Some(count) = minute_count {
            if count >= limits.minute {
                return Some((limits.minute, chrono::Utc::now() + chrono::Duration::minutes(1)));
            }
        }

        None
    }

    async fn check_in_memory_rate_limit(
        daily: &Arc<RwLock<FxHashMap<String, u32>>>,
        minute: &Arc<RwLock<FxHashMap<String, u32>>>,
        key_hash: &str,
        limits: TierLimits,
    ) -> Option<(u32, chrono::DateTime<chrono::Utc>)> {
        let daily_reader = daily.read().await;
        if let Some(count) = daily_reader.get(key_hash) {
            if *count >= limits.daily {
                return Some((limits.daily, chrono::Utc::now() + chrono::Duration::days(1)));
            }
        }
        drop(daily_reader);

        let minute_reader = minute.read().await;
        if let Some(count) = minute_reader.get(key_hash) {
            if *count >= limits.minute {
                return Some((limits.minute, chrono::Utc::now() + chrono::Duration::minutes(1)));
            }
        }

        None
    }

    pub async fn record_request(&self, key_hash: &str) {
        match self.inner.as_ref() {
            RateLimiterInner::Redis { client } => {
                if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
                    let daily_key = format!("ratelimit:daily:{}", key_hash);
                    let minute_key = format!("ratelimit:minute:{}", key_hash);

                    let _: Result<(), _> = redis::pipe()
                        .incr(&daily_key, 1)
                        .expire(&daily_key, DAILY_WINDOW_SECS)
                        .incr(&minute_key, 1)
                        .expire(&minute_key, MINUTE_WINDOW_SECS)
                        .query_async(&mut conn)
                        .await;
                }
            }
            RateLimiterInner::InMemory { daily, minute } => {
                let mut daily_writer = daily.write().await;
                *daily_writer.entry(key_hash.to_string()).or_default() += 1;

                let mut minute_writer = minute.write().await;
                *minute_writer.entry(key_hash.to_string()).or_default() += 1;
            }
        }
    }
}

struct TierLimits {
    daily: u32,
    minute: u32,
}

fn get_tier_limits(tier: Tier) -> TierLimits {
    TierLimits {
        daily: tier.requests_limit() as u32,
        minute: tier.rate_limit_per_minute(),
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

    #[test]
    fn test_tier_limits_free() {
        let limits = get_tier_limits(Tier::Free);
        assert_eq!(limits.daily, 100);
        assert_eq!(limits.minute, 10);
    }

    #[test]
    fn test_tier_limits_team() {
        let limits = get_tier_limits(Tier::Team);
        assert_eq!(limits.daily, 100_000);
        assert_eq!(limits.minute, 10_000);
    }

    #[test]
    fn test_rate_limiter_construction_in_memory() {
        let limiter = RateLimiter::new(None);
        assert!(matches!(*limiter.inner, RateLimiterInner::InMemory { .. }));
    }
}
