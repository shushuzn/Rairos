//! Circuit breaker + rate limiter for LLM API calls.
//!
//! Circuit breaker: tracks consecutive failures, opens circuit after threshold,
//! allows retry after cooldown period.
//!
//! Rate limiter: token bucket algorithm with configurable QPS.

use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    Closed,    // Normal operation
    Open,      // Failing, no requests allowed
    HalfOpen,  // Testing if service recovered
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,      // Consecutive failures to open circuit
    pub cooldown_secs: u64,          // Seconds before half-open retry
    pub half_open_max_requests: u32, // Requests allowed in half-open state
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            cooldown_secs: 30,
            half_open_max_requests: 1,
        }
    }
}

/// Circuit breaker for LLM API calls
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: std::sync::RwLock<CircuitState>,
    failure_count: AtomicU32,
    last_failure_time: std::sync::RwLock<Instant>,
    half_open_requests: AtomicU32,
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new(CircuitBreakerConfig::default())
    }
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: std::sync::RwLock::new(CircuitState::Closed),
            failure_count: AtomicU32::new(0),
            last_failure_time: std::sync::RwLock::new(Instant::now()),
            half_open_requests: AtomicU32::new(0),
        }
    }

    /// Check if a request is allowed through the circuit breaker
    pub fn is_allowed(&self) -> bool {
        let state = *self.state.read().unwrap();
        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                let cooldown = Duration::from_secs(self.config.cooldown_secs);
                if self.last_failure_time.read().unwrap().elapsed() >= cooldown {
                    // Transition to half-open
                    *self.state.write().unwrap() = CircuitState::HalfOpen;
                    self.half_open_requests.store(0, Ordering::Relaxed);
                    true
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => {
                let count = self.half_open_requests.fetch_add(1, Ordering::Relaxed);
                count < self.config.half_open_max_requests
            }
        }
    }

    /// Record a successful call
    pub fn record_success(&self) {
        *self.state.write().unwrap() = CircuitState::Closed;
        self.failure_count.store(0, Ordering::Relaxed);
        self.half_open_requests.store(0, Ordering::Relaxed);
    }

    /// Record a failed call
    pub fn record_failure(&self) {
        *self.last_failure_time.write().unwrap() = Instant::now();
        let count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        if count >= self.config.failure_threshold {
            *self.state.write().unwrap() = CircuitState::Open;
        }
    }

    /// Get current state
    pub fn state(&self) -> CircuitState {
        *self.state.read().unwrap()
    }

    /// Reset to closed state
    pub fn reset(&self) {
        *self.state.write().unwrap() = CircuitState::Closed;
        self.failure_count.store(0, Ordering::Relaxed);
        self.half_open_requests.store(0, Ordering::Relaxed);
    }
}

/// Token bucket rate limiter
pub struct RateLimiter {
    capacity: u32,
    tokens: AtomicU32,
    refill_rate: f64,         // tokens per second
    last_refill: std::sync::RwLock<Instant>,
}

impl RateLimiter {
    pub fn new(qps: f64) -> Self {
        Self {
            capacity: qps.ceil() as u32,
            tokens: AtomicU32::new(qps.ceil() as u32),
            refill_rate: qps,
            last_refill: std::sync::RwLock::new(Instant::now()),
        }
    }

    /// Try to acquire a token. Returns true if allowed.
    pub fn try_acquire(&self) -> bool {
        self.refill();
        let mut current = self.tokens.load(Ordering::Relaxed);
        loop {
            if current == 0 {
                return false;
            }
            match self.tokens.compare_exchange(current, current - 1, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => return true,
                Err(actual) => current = actual,
            }
        }
    }

    fn refill(&self) {
        let mut last = self.last_refill.write().unwrap();
        let elapsed = last.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            let new_tokens = (elapsed * self.refill_rate) as u32;
            if new_tokens > 0 {
                let current = self.tokens.load(Ordering::Relaxed);
                let new_count = (current + new_tokens).min(self.capacity);
                self.tokens.store(new_count, Ordering::Relaxed);
                *last = Instant::now();
            }
        }
    }

    pub fn capacity(&self) -> u32 {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_closed_by_default() {
        let cb = CircuitBreaker::default();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.is_allowed());
    }

    #[test]
    fn test_circuit_breaker_opens_after_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            cooldown_secs: 60,
            half_open_max_requests: 1,
        };
        let cb = CircuitBreaker::new(config);
        for _ in 0..3 {
            cb.record_failure();
        }
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.is_allowed(), "should not allow when open");
    }

    #[test]
    fn test_circuit_breaker_records_success() {
        let cb = CircuitBreaker::default();
        cb.record_failure();
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.is_allowed());
    }

    #[test]
    fn test_rate_limiter_allows_burst() {
        let rl = RateLimiter::new(10.0);
        for _ in 0..10 {
            assert!(rl.try_acquire(), "should allow up to capacity");
        }
        assert!(!rl.try_acquire(), "should deny after capacity");
    }

    #[test]
    fn test_rate_limiter_refills() {
        let rl = RateLimiter::new(100.0);
        for _ in 0..100 {
            rl.try_acquire();
        }
        assert!(!rl.try_acquire(), "should be empty");
        // Wait for refill
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert!(rl.try_acquire(), "should refill after time");
    }
}
