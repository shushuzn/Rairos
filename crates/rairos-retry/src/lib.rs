//! rairos-retry — Retry utilities with exponential backoff and circuit breaker.
//!
//! Ported from `core/retry.py`.

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, RwLock};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RetryStatsData {
    pub total_attempts: i32,
    pub total_failures: i32,
    pub total_retries: i32,
    pub total_success: i32,
    pub errors: HashMap<String, i32>,
}

#[derive(Debug, Clone, Default)]
pub struct RetryStats {
    stats: Arc<RwLock<HashMap<String, RetryStatsData>>>,
}

impl RetryStats {
    pub fn new() -> Self {
        Self {
            stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn record_attempt(
        &self,
        func_name: &str,
        attempt: i32,
        success: bool,
        error: Option<&str>,
    ) {
        let mut guard = self.stats.write().unwrap();
        let entry = guard.entry(func_name.to_string()).or_default();
        entry.total_attempts += 1;
        if !success {
            entry.total_failures += 1;
            entry.total_retries += attempt;
            if let Some(err_str) = error {
                *entry.errors.entry(err_str.to_string()).or_insert(0) += 1;
            }
        } else {
            entry.total_success += 1;
        }
    }

    pub fn get_stats(&self, func_name: Option<&str>) -> HashMap<String, RetryStatsData> {
        let guard = self.stats.read().unwrap();
        match func_name {
            Some(name) => guard
                .get(name)
                .map(|v| HashMap::from([(name.to_string(), v.clone())]))
                .unwrap_or_default(),
            None => guard.clone(),
        }
    }

    pub fn reset(&self, func_name: Option<&str>) {
        let mut guard = self.stats.write().unwrap();
        match func_name {
            Some(name) => {
                guard.remove(name);
            }
            None => guard.clear(),
        }
    }
}

static RETRY_STATS: LazyLock<RetryStats, fn() -> RetryStats> =
    LazyLock::new(RetryStats::new);

pub fn get_retry_stats() -> RetryStats {
    RETRY_STATS.clone()
}

pub fn retry_with_backoff<F, T, E>(
    max_attempts: i32,
    base_delay_secs: f64,
    max_delay_secs: f64,
    func: F,
) -> Result<T, E>
where
    F: Fn() -> Result<T, E>,
{
    let mut last_err = None;
    let mut rng = rand::thread_rng();

    for attempt in 1..=max_attempts {
        match func() {
            Ok(result) => {
                RETRY_STATS.record_attempt("anonymous", attempt, true, None);
                return Ok(result);
            }
            Err(err) => {
                last_err = Some(err);
                RETRY_STATS.record_attempt("anonymous", attempt, false, None);
                if attempt < max_attempts {
                    let delay = (base_delay_secs * 2_f64.powi(attempt - 1)).min(max_delay_secs);
                    let jitter = delay * 0.1 * rng.gen::<f64>();
                    thread::sleep(Duration::from_secs_f64(delay + jitter));
                }
            }
        }
    }

        Err(last_err.unwrap())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum CircuitState {
    #[default]
    Closed,
    Open,
    HalfOpen,
}


#[derive(Debug)]
pub struct CircuitBreaker {
    failure_threshold: i32,
    recovery_timeout_secs: f64,
    state: RwLock<CircuitState>,
    failure_count: RwLock<i32>,
    last_failure_time: RwLock<Option<f64>>,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: i32, recovery_timeout_secs: f64) -> Self {
        Self {
            failure_threshold,
            recovery_timeout_secs,
            state: RwLock::new(CircuitState::Closed),
            failure_count: RwLock::new(0),
            last_failure_time: RwLock::new(None),
        }
    }

    pub fn state(&self) -> CircuitState {
        let state = self.state.read().unwrap();
        if *state == CircuitState::Open {
            let last_failure = self.last_failure_time.read().unwrap();
            if let Some(last_time) = *last_failure {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs_f64();
                if now - last_time >= self.recovery_timeout_secs {
                    drop(last_failure);
                    *self.state.write().unwrap() = CircuitState::HalfOpen;
                    return CircuitState::HalfOpen;
                }
            }
        }
        *state
    }

    pub fn record_success(&self) {
        let mut state = self.state.write().unwrap();
        *state = CircuitState::Closed;
        let mut count = self.failure_count.write().unwrap();
        *count = 0;
    }

    pub fn record_failure(&self) {
        let mut count = self.failure_count.write().unwrap();
        *count += 1;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();
        *self.last_failure_time.write().unwrap() = Some(now);

        if *count >= self.failure_threshold {
            *self.state.write().unwrap() = CircuitState::Open;
        }
    }

    pub fn call<F, T, E>(&self, func: F) -> Result<T, E>
    where
        F: Fn() -> Result<T, E>,
        E: From<CircuitOpenError>,
    {
        match self.state() {
            CircuitState::Open => Err(CircuitOpenError {
                message: format!("Circuit breaker is OPEN. Retry after {:.0}s", self.recovery_timeout_secs),
            }.into()),
            CircuitState::HalfOpen | CircuitState::Closed => {
                match func() {
                    Ok(result) => {
                        self.record_success();
                        Ok(result)
                    }
                    Err(err) => {
                        self.record_failure();
                        Err(err)
                    }
                }
            }
        }
    }
}

impl Clone for CircuitBreaker {
    fn clone(&self) -> Self {
        Self {
            failure_threshold: self.failure_threshold,
            recovery_timeout_secs: self.recovery_timeout_secs,
            state: RwLock::new(*self.state.read().unwrap()),
            failure_count: RwLock::new(*self.failure_count.read().unwrap()),
            last_failure_time: RwLock::new(*self.last_failure_time.read().unwrap()),
        }
    }
}

#[derive(Debug)]
pub struct CircuitOpenError {
    pub message: String,
}

impl fmt::Display for CircuitOpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CircuitOpen: {}", self.message)
    }
}

impl std::error::Error for CircuitOpenError {}

impl fmt::Display for CircuitState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "closed"),
            CircuitState::Open => write!(f, "open"),
            CircuitState::HalfOpen => write!(f, "half-open"),
        }
    }
}

use std::fmt;

static CB_BREAKERS: LazyLock<RwLock<HashMap<String, CircuitBreaker>>, fn() -> RwLock<HashMap<String, CircuitBreaker>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub fn get_circuit_breaker(name: &str) -> Option<CircuitBreaker> {
    let guard = CB_BREAKERS.read().unwrap();
    guard.get(name).cloned()
}

pub fn register_circuit_breaker(name: &str, breaker: CircuitBreaker) {
    let mut guard = CB_BREAKERS.write().unwrap();
    guard.insert(name.to_string(), breaker);
}

pub fn circuit_breaker_call<F, T, E>(name: &str, func: F) -> Result<T, E>
where
    F: Fn() -> Result<T, E>,
    E: From<CircuitOpenError>,
{
    let breaker = {
        let guard = CB_BREAKERS.read().unwrap();
        guard.get(name).cloned()
    };

    match breaker {
        Some(b) => b.call(func),
        None => func(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_stats_new() {
        let stats = RetryStats::new();
        let result = stats.get_stats(None);
        assert!(result.is_empty());
    }

    #[test]
    fn test_retry_stats_record_attempt() {
        let stats = RetryStats::new();
        stats.record_attempt("test_func", 1, true, None);
        let result = stats.get_stats(Some("test_func"));
        assert_eq!(result.get("test_func").unwrap().total_attempts, 1);
        assert_eq!(result.get("test_func").unwrap().total_success, 1);
    }

    #[test]
    fn test_retry_stats_record_failure() {
        let stats = RetryStats::new();
        stats.record_attempt("test_func", 1, false, Some("TimeoutError"));
        let result = stats.get_stats(Some("test_func"));
        assert_eq!(result.get("test_func").unwrap().total_failures, 1);
        assert_eq!(result.get("test_func").unwrap().errors.get("TimeoutError"), Some(&1));
    }

    #[test]
    fn test_retry_stats_reset() {
        let stats = RetryStats::new();
        stats.record_attempt("func1", 1, true, None);
        stats.reset(Some("func1"));
        let result = stats.get_stats(Some("func1"));
        assert!(result.is_empty());
    }

    #[test]
    fn test_retry_with_backoff_success() {
        let result = retry_with_backoff(3, 0.01, 1.0, || Ok::<i32, ()>(42));
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_retry_stats_global() {
        let stats = get_retry_stats();
        stats.reset(None);
        stats.record_attempt("global_test", 1, true, None);
        let result = stats.get_stats(Some("global_test"));
        assert_eq!(result.get("global_test").unwrap().total_attempts, 1);
    }

    #[test]
    fn test_circuit_breaker_new() {
        let cb = CircuitBreaker::new(5, 60.0);
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_record_success() {
        let cb = CircuitBreaker::new(5, 60.0);
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(*cb.failure_count.read().unwrap(), 0);
    }

    #[test]
    fn test_circuit_breaker_opens_after_threshold() {
        let cb = CircuitBreaker::new(3, 60.0);
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_circuit_breaker_call_success() {
        let cb = CircuitBreaker::new(3, 60.0);
        let result: Result<i32, CircuitOpenError> = cb.call(|| Ok(42));
        assert_eq!(result.unwrap(), 42);
        assert_eq!(*cb.failure_count.read().unwrap(), 0);
    }

    #[test]
    fn test_circuit_breaker_call_failure() {
        let cb = CircuitBreaker::new(3, 60.0);
        let result: Result<i32, CircuitOpenError> = cb.call(|| Err(CircuitOpenError { message: "fail".to_string() }));
        assert!(result.is_err());
        assert_eq!(*cb.failure_count.read().unwrap(), 1);
    }

    #[test]
    fn test_circuit_breaker_open_rejects() {
        let cb = CircuitBreaker::new(1, 60.0);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        let result: Result<i32, CircuitOpenError> = cb.call(|| Ok(42));
        assert!(result.is_err());
    }

    #[test]
    fn test_get_circuit_breaker_not_found() {
        let result = get_circuit_breaker("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_register_and_get_circuit_breaker() {
        let cb = CircuitBreaker::new(5, 30.0);
        register_circuit_breaker("test_cb", cb);
        let retrieved = get_circuit_breaker("test_cb");
        assert!(retrieved.is_some());
    }
}
