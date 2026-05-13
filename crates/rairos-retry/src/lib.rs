//! rairos-retry — Retry utilities with exponential backoff and circuit breaker.

#![allow(clippy::type_repetition_in_bounds)]
//!
//! Ported from `core/retry.py`.
//!
//! Features:
//! - Exponential backoff with optional jitter
//! - Configurable exception filtering
//! - Thread-safe retry statistics tracking
//! - Circuit breaker pattern (CLOSED → OPEN → HALF_OPEN)

use std::collections::HashMap;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

// ─── Error types ────────────────────────────────────────────────────────────────

/// Error raised when a circuit breaker is open.
#[derive(Debug, Clone)]
pub struct CircuitOpen {
    pub message: String,
}

impl std::fmt::Display for CircuitOpen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CircuitOpen {}

// ─── Retry Statistics ─────────────────────────────────────────────────────────

/// Thread-safe retry statistics tracker.
#[derive(Debug, Default)]
pub struct RetryStats {
    stats: Mutex<HashMap<String, FuncStats>>,
}

#[derive(Debug, Clone, Default)]
struct FuncStats {
    total_attempts: u64,
    total_failures: u64,
    total_retries: u64,
    total_success: u64,
    errors: HashMap<String, u64>,
}

impl RetryStats {
    /// Create a new stats tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a successful or failed attempt.
    pub fn record_attempt(
        &self,
        func_name: &str,
        attempt: u32,
        success: bool,
        error_type: Option<&str>,
    ) {
        let mut guard = self.stats.lock().unwrap();
        let entry = guard.entry(func_name.to_string()).or_default();
        entry.total_attempts += 1;
        if success {
            entry.total_success += 1;
        } else {
            entry.total_failures += 1;
            entry.total_retries += attempt as u64;
            if let Some(e) = error_type {
                *entry.errors.entry(e.to_string()).or_insert(0) += 1;
            }
        }
    }

    /// Get stats for a specific function or all functions.
    pub fn get_stats(&self, func_name: Option<&str>) -> HashMap<String, FuncStats> {
        let guard = self.stats.lock().unwrap();
        match func_name {
            Some(name) => guard
                .get(name)
                .map(|s| HashMap::from([(name.to_string(), s.clone())]))
                .unwrap_or_default(),
            None => guard.clone(),
        }
    }

    /// Reset stats for a function or all functions.
    pub fn reset(&self, func_name: Option<&str>) {
        let mut guard = self.stats.lock().unwrap();
        match func_name {
            Some(name) => {
                guard.remove(name);
            }
            None => guard.clear(),
        }
    }
}

// ─── Retry ─────────────────────────────────────────────────────────────────

/// Configuration for retry behavior.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub base_delay_secs: f64,
    pub max_delay_secs: f64,
    pub jitter: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay_secs: 1.0,
            max_delay_secs: 30.0,
            jitter: 0.0,
        }
    }
}

/// Execute a fallible operation with retry and exponential backoff.
pub fn retry_with_backoff<C, F, T, E>(config: C, mut op: F) -> Result<T, E>
where
    C: Into<Option<RetryConfig>>,
    F: FnMut() -> Result<T, E>,
    E: std::fmt::Debug,
{
    let config = config.into().unwrap_or_default();
    let mut last_err: Option<E> = None;
    for attempt in 1..=config.max_attempts {
        match op() {
            Ok(val) => return Ok(val),
            Err(e) => {
                last_err = Some(e);
                if attempt == config.max_attempts {
                    break;
                }
                let delay = (config.base_delay_secs * 2.0_f64.powf(attempt as f64 - 1.0))
                    .min(config.max_delay_secs);
                let delay = if config.jitter > 0.0 {
                    delay + rand_simple() * delay * config.jitter
                } else {
                    delay
                };
                thread::sleep(Duration::from_secs_f64(delay));
            }
        }
    }
    Err(last_err.unwrap())
}

/// Simple pseudo-random 0.0-1.0 (no external deps).
fn rand_simple() -> f64 {
    static mut SEED: u64 = 0;
    unsafe {
        if SEED == 0 {
            SEED = std::time::Instant::now().elapsed().as_nanos() as u64;
        }
        SEED = SEED.wrapping_mul(6364136223846793005).wrapping_add(1);
        (SEED >> 33) as f64 / (u32::MAX >> 1) as f64
    }
}

// ─── Circuit Breaker ──────────────────────────────────────────────────────────

/// Circuit breaker state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker that prevents repeated calls to a failing service.
pub struct CircuitBreaker {
    failure_threshold: u32,
    recovery_timeout_secs: f64,
    state: Mutex<CircuitState>,
    failure_count: Mutex<u32>,
    last_failure_time: Mutex<Option<f64>>,
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new(5, 60.0)
    }
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, recovery_timeout_secs: f64) -> Self {
        Self {
            failure_threshold,
            recovery_timeout_secs,
            state: Mutex::new(CircuitState::Closed),
            failure_count: Mutex::new(0),
            last_failure_time: Mutex::new(None),
        }
    }

    /// Get current state.
    pub fn state(&self) -> CircuitState {
        let state = *self.state.lock().unwrap();
        if state == CircuitState::Open {
            let last_fail = *self.last_failure_time.lock().unwrap();
            if let Some(t) = last_fail {
                let now = now_secs();
                if now - t >= self.recovery_timeout_secs {
                    *self.state.lock().unwrap() = CircuitState::HalfOpen;
                    return CircuitState::HalfOpen;
                }
            }
        }
        state
    }

    /// Record a successful call (resets failure count).
    pub fn record_success(&self) {
        let mut count = self.failure_count.lock().unwrap();
        *count = 0;
        *self.state.lock().unwrap() = CircuitState::Closed;
    }

    /// Record a failed call.
    pub fn record_failure(&self) {
        let mut count = self.failure_count.lock().unwrap();
        *count += 1;
        *self.last_failure_time.lock().unwrap() = Some(now_secs());
        if *count >= self.failure_threshold {
            *self.state.lock().unwrap() = CircuitState::Open;
        }
    }

    /// Execute a function through the circuit breaker.
    pub fn call<F, T, E>(&self, f: F) -> Result<T, CircuitOpen>
    where
        F: FnOnce() -> Result<T, E>,
        E: std::fmt::Debug,
    {
        let current_state = self.state();
        if current_state == CircuitState::Open {
            return Err(CircuitOpen {
                message: format!(
                    "Circuit breaker is OPEN. Retry after {:.0}s",
                    self.recovery_timeout_secs
                ),
            });
        }

        match f() {
            Ok(val) => {
                self.record_success();
                Ok(val)
            }
            Err(_) => {
                self.record_failure();
                Err(CircuitOpen {
                    message: format!(
                        "Circuit breaker is {:?}. Retry after {:.0}s",
                        self.state(),
                        self.recovery_timeout_secs
                    ),
                })
            }
        }
    }

    /// Returns true if circuit is currently open.
    pub fn is_open(&self) -> bool {
        self.state() == CircuitState::Open
    }

    /// Manually reset to closed state.
    pub fn reset(&self) {
        *self.state.lock().unwrap() = CircuitState::Closed;
        *self.failure_count.lock().unwrap() = 0;
        *self.last_failure_time.lock().unwrap() = None;
    }

    /// Get failure count.
    pub fn failure_count(&self) -> u32 {
        *self.failure_count.lock().unwrap()
    }
}

fn now_secs() -> f64 {
    std::time::Instant::now().elapsed().as_secs_f64()
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_stats_new_is_empty() {
        let stats = RetryStats::new();
        assert!(stats.get_stats(None).is_empty());
    }

    #[test]
    fn test_retry_stats_record_and_get() {
        let stats = RetryStats::new();
        stats.record_attempt("my_func", 1, true, None);
        stats.record_attempt("my_func", 2, false, Some("IOError"));

        let all = stats.get_stats(None);
        let func = all.get("my_func").unwrap();
        assert_eq!(func.total_attempts, 2);
        assert_eq!(func.total_success, 1);
        assert_eq!(func.total_failures, 1);
        assert_eq!(func.errors.get("IOError"), Some(&1));
    }

    #[test]
    fn test_retry_success_first_attempt() {
        let result = retry_with_backoff(None, || Ok::<i32, ()>(42));
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_retry_fails_after_max_attempts() {
        let config = RetryConfig {
            max_attempts: 2,
            base_delay_secs: 0.001,
            max_delay_secs: 1.0,
            jitter: 0.0,
        };
        let result = retry_with_backoff(Some(config), || Err::<i32, _>("fail"));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "fail");
    }

    #[test]
    fn test_retry_succeeds_on_second_attempt() {
        let config = RetryConfig {
            max_attempts: 3,
            base_delay_secs: 0.001,
            max_delay_secs: 1.0,
            jitter: 0.0,
        };
        let mut count = 0;
        let result = retry_with_backoff(Some(config), || {
            count += 1;
            if count < 2 {
                Err("try again")
            } else {
                Ok(99)
            }
        });
        assert_eq!(result.unwrap(), 99);
        assert_eq!(count, 2);
    }

    #[test]
    fn test_circuit_breaker_initial_closed() {
        let cb = CircuitBreaker::new(3, 60.0);
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(!cb.is_open());
    }

    #[test]
    fn test_circuit_breaker_opens_after_threshold() {
        let cb = CircuitBreaker::new(3, 60.0);
        for _ in 0..3 {
            let _ = cb.call::<_, (), _>(|| Err("error"));
        }
        assert!(cb.is_open());
    }

    #[test]
    fn test_circuit_breaker_success_resets() {
        let cb = CircuitBreaker::new(2, 60.0);
        let _ = cb.call::<_, (), _>(|| Err(()));
        assert_eq!(cb.failure_count(), 1);
        let _ = cb.call::<_, i32, ()>(|| Ok(1));
        assert_eq!(cb.failure_count(), 0);
        assert!(!cb.is_open());
    }

    #[test]
    fn test_circuit_breaker_call_returns_value() {
        let cb = CircuitBreaker::new(5, 60.0);
        let result = cb.call::<_, i32, ()>(|| Ok(100));
        assert_eq!(result.unwrap(), 100);
    }

    #[test]
    fn test_circuit_breaker_open_blocks_call() {
        let cb = CircuitBreaker::new(1, 60.0);
        let _ = cb.call::<_, (), _>(|| Err(()));
        assert!(cb.is_open());
        let result = cb.call::<_, i32, ()>(|| Ok(1));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CircuitOpen { .. }));
    }

    #[test]
    fn test_circuit_breaker_reset() {
        let cb = CircuitBreaker::new(1, 60.0);
        let _ = cb.call::<_, (), _>(|| Err(()));
        assert!(cb.is_open());
        cb.reset();
        assert!(!cb.is_open());
        let result = cb.call::<_, i32, ()>(|| Ok(1));
        assert_eq!(result.unwrap(), 1);
    }
}
