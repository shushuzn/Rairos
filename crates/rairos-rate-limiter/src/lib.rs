//! rairos-rate-limiter — API Rate Limiter and Request Manager.
//!
//! Ported from `core/rate_limiter.py`.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, LazyLock, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_second: f64,
    pub requests_per_minute: f64,
    pub requests_per_hour: f64,
    pub burst_size: usize,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 10.0,
            requests_per_minute: 100.0,
            requests_per_hour: 1000.0,
            burst_size: 5,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RateLimiter {
    config: RateLimitConfig,
    tokens: Arc<RwLock<f64>>,
    last_update: Arc<RwLock<f64>>,
    second_history: Arc<RwLock<VecDeque<f64>>>,
    minute_history: Arc<RwLock<VecDeque<f64>>>,
    hour_history: Arc<RwLock<VecDeque<f64>>>,
    total_requests: Arc<RwLock<usize>>,
    total_wait_time: Arc<RwLock<f64>>,
    total_rejected: Arc<RwLock<usize>>,
}

impl RateLimiter {
    pub fn new(config: Option<RateLimitConfig>) -> Self {
        let config = config.unwrap_or_default();
        let burst_size = config.burst_size as f64;
        let now = Self::now_secs();
        Self {
            config,
            tokens: Arc::new(RwLock::new(burst_size)),
            last_update: Arc::new(RwLock::new(now)),
            second_history: Arc::new(RwLock::new(VecDeque::new())),
            minute_history: Arc::new(RwLock::new(VecDeque::new())),
            hour_history: Arc::new(RwLock::new(VecDeque::new())),
            total_requests: Arc::new(RwLock::new(0)),
            total_wait_time: Arc::new(RwLock::new(0.0)),
            total_rejected: Arc::new(RwLock::new(0)),
        }
    }

    fn now_secs() -> f64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
    }

    fn refill_tokens(&self) {
        let now = Self::now_secs();
        let last = *self.last_update.read().unwrap();
        let elapsed = now - last;
        let refill = elapsed * self.config.requests_per_second;
        let mut tokens = self.tokens.write().unwrap();
        *tokens = (self.config.burst_size as f64).min(*tokens + refill);
        *self.last_update.write().unwrap() = now;
    }

    fn clean_history(&self) {
        let now = Self::now_secs();

        let mut second = self.second_history.write().unwrap();
        while let Some(&front) = second.front() {
            if now - front > 1.0 {
                second.pop_front();
            } else {
                break;
            }
        }

        let mut minute = self.minute_history.write().unwrap();
        while let Some(&front) = minute.front() {
            if now - front > 60.0 {
                minute.pop_front();
            } else {
                break;
            }
        }

        let mut hour = self.hour_history.write().unwrap();
        while let Some(&front) = hour.front() {
            if now - front > 3600.0 {
                hour.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn can_make_request(&self) -> bool {
        self.clean_history();

        let second = self.second_history.read().unwrap();
        let minute = self.minute_history.read().unwrap();
        let hour = self.hour_history.read().unwrap();

        if second.len() >= self.config.requests_per_second as usize {
            return false;
        }
        if minute.len() >= self.config.requests_per_minute as usize {
            return false;
        }
        if hour.len() >= self.config.requests_per_hour as usize {
            return false;
        }

        true
    }

    pub fn acquire(&self, blocking: bool, timeout_secs: Option<f64>) -> bool {
        let start_time = Instant::now();

        loop {
            self.refill_tokens();
            self.clean_history();

            let now = Self::now_secs();

            if self.can_make_request() {
                let mut second = self.second_history.write().unwrap();
                let mut minute = self.minute_history.write().unwrap();
                let mut hour = self.hour_history.write().unwrap();

                second.push_back(now);
                minute.push_back(now);
                hour.push_back(now);
                *self.tokens.write().unwrap() -= 1.0;

                let mut total = self.total_requests.write().unwrap();
                *total += 1;
                return true;
            }

            let wait_time = {
                let second = self.second_history.read().unwrap();
                let minute = self.minute_history.read().unwrap();

                if !second.is_empty() {
                    1.0 - (now - second[0])
                } else if !minute.is_empty() {
                    60.0 - (now - minute[0])
                } else {
                    1.0 / self.config.requests_per_second
                }
            };

            let wait_time = wait_time.clamp(0.01, 10.0);

            if !blocking {
                let mut rejected = self.total_rejected.write().unwrap();
                *rejected += 1;
                return false;
            }

            if let Some(timeout) = timeout_secs {
                if start_time.elapsed().as_secs_f64() >= timeout {
                    let mut rejected = self.total_rejected.write().unwrap();
                    *rejected += 1;
                    return false;
                }
            }

            std::thread::sleep(Duration::from_secs_f64(wait_time));

            let mut total_wait = self.total_wait_time.write().unwrap();
            *total_wait += wait_time;
        }
    }

    pub fn wait_if_needed(&self) -> f64 {
        let start = Instant::now();
        self.acquire(true, None);
        start.elapsed().as_secs_f64()
    }

    pub fn get_stats(&self) -> RateLimiterStats {
        self.clean_history();

        RateLimiterStats {
            total_requests: *self.total_requests.read().unwrap(),
            total_wait_time: *self.total_wait_time.read().unwrap(),
            total_rejected: *self.total_rejected.read().unwrap(),
            current_second_requests: self.second_history.read().unwrap().len(),
            current_minute_requests: self.minute_history.read().unwrap().len(),
            current_hour_requests: self.hour_history.read().unwrap().len(),
            tokens_available: *self.tokens.read().unwrap(),
            limits: RateLimitLimits {
                per_second: self.config.requests_per_second,
                per_minute: self.config.requests_per_minute,
                per_hour: self.config.requests_per_hour,
                burst_size: self.config.burst_size,
            },
        }
    }

    pub fn reset_stats(&self) {
        let mut total = self.total_requests.write().unwrap();
        *total = 0;
        let mut wait = self.total_wait_time.write().unwrap();
        *wait = 0.0;
        let mut rejected = self.total_rejected.write().unwrap();
        *rejected = 0;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimiterStats {
    pub total_requests: usize,
    pub total_wait_time: f64,
    pub total_rejected: usize,
    pub current_second_requests: usize,
    pub current_minute_requests: usize,
    pub current_hour_requests: usize,
    pub tokens_available: f64,
    pub limits: RateLimitLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitLimits {
    pub per_second: f64,
    pub per_minute: f64,
    pub per_hour: f64,
    pub burst_size: usize,
}

#[derive(Debug, Clone, Default)]
pub struct APIRateLimitManager {
    limiters: Arc<RwLock<HashMap<String, RateLimiter>>>,
}

impl APIRateLimitManager {
    pub fn new() -> Self {
        Self {
            limiters: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get_limiter(&self, endpoint: &str, config: Option<RateLimitConfig>) -> RateLimiter {
        let mut limiters = self.limiters.write().unwrap();
        if !limiters.contains_key(endpoint) {
            limiters.insert(endpoint.to_string(), RateLimiter::new(config));
        }
        limiters.get(endpoint).unwrap().clone()
    }

    pub fn wait_for_endpoint(&self, endpoint: &str, config: Option<RateLimitConfig>) -> f64 {
        let limiter = self.get_limiter(endpoint, config);
        limiter.wait_if_needed()
    }

    pub fn can_call_endpoint(&self, endpoint: &str) -> bool {
        let limiters = self.limiters.read().unwrap();
        match limiters.get(endpoint) {
            Some(limiter) => limiter.can_make_request(),
            None => true,
        }
    }

    pub fn get_all_stats(&self) -> HashMap<String, RateLimiterStats> {
        let limiters = self.limiters.read().unwrap();
        limiters
            .iter()
            .map(|(k, v)| (k.clone(), v.get_stats()))
            .collect()
    }
}

use std::collections::HashMap;

static RATE_LIMIT_MANAGER: LazyLock<APIRateLimitManager, fn() -> APIRateLimitManager> =
    LazyLock::new(APIRateLimitManager::new);

pub fn get_rate_limit_manager() -> APIRateLimitManager {
    RATE_LIMIT_MANAGER.clone()
}

pub fn create_limiter(
    requests_per_second: f64,
    requests_per_minute: f64,
    requests_per_hour: f64,
    burst_size: usize,
) -> RateLimiter {
    let config = RateLimitConfig {
        requests_per_second,
        requests_per_minute,
        requests_per_hour,
        burst_size,
    };
    RateLimiter::new(Some(config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_new() {
        let limiter = RateLimiter::new(None);
        assert!(limiter.can_make_request());
    }

    #[test]
    fn test_rate_limiter_acquire() {
        let limiter = RateLimiter::new(Some(RateLimitConfig {
            requests_per_second: 10.0,
            requests_per_minute: 100.0,
            requests_per_hour: 1000.0,
            burst_size: 3,
        }));

        assert!(limiter.acquire(false, None));
        assert!(limiter.acquire(false, None));
        assert!(limiter.acquire(false, None));
    }

    #[test]
    fn test_rate_limiter_stats() {
        let limiter = RateLimiter::new(None);
        limiter.acquire(false, None);
        let stats = limiter.get_stats();
        assert_eq!(stats.total_requests, 1);
        assert_eq!(stats.total_rejected, 0);
    }

    #[test]
    fn test_rate_limiter_reset_stats() {
        let limiter = RateLimiter::new(None);
        limiter.acquire(false, None);
        limiter.reset_stats();
        let stats = limiter.get_stats();
        assert_eq!(stats.total_requests, 0);
    }

    #[test]
    fn test_rate_limit_manager_new() {
        let manager = APIRateLimitManager::new();
        assert!(manager.can_call_endpoint("nonexistent"));
    }

    #[test]
    fn test_rate_limit_manager_get_limiter() {
        let manager = APIRateLimitManager::new();
        let limiter = manager.get_limiter("test_api", None);
        assert!(limiter.can_make_request());
    }

    #[test]
    fn test_rate_limit_manager_wait_for_endpoint() {
        let manager = APIRateLimitManager::new();
        let wait_time = manager.wait_for_endpoint("test", None);
        assert!(wait_time >= 0.0);
    }

    #[test]
    fn test_create_limiter() {
        let limiter = create_limiter(5.0, 50.0, 500.0, 2);
        assert!(limiter.can_make_request());
    }

    #[test]
    fn test_get_global_manager() {
        let manager = get_rate_limit_manager();
        manager.get_limiter("global_test", None);
        assert!(manager.can_call_endpoint("global_test"));
    }

    #[test]
    fn test_rate_limiter_wait_if_needed() {
        let limiter = create_limiter(100.0, 1000.0, 10000.0, 10);
        let wait = limiter.wait_if_needed();
        assert!(wait >= 0.0);
    }

    #[test]
    fn test_rate_limiter_blocking_acquire() {
        let limiter = create_limiter(100.0, 1000.0, 10000.0, 5);
        let result = limiter.acquire(true, Some(1.0));
        assert!(result);
    }

    #[test]
    fn test_rate_limiter_timeout() {
        let limiter = create_limiter(0.1, 1.0, 10.0, 1);
        limiter.acquire(false, None);
        let result = limiter.acquire(false, Some(0.1));
        assert!(!result);
    }
}
