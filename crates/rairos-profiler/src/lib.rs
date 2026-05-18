//! rairos-profiler — Performance Profiler and Analysis Tools.
//!
//! Ported from `core/profiler.py`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionProfile {
    pub name: String,
    pub call_count: u64,
    pub total_time: f64,
    pub min_time: f64,
    pub max_time: f64,
    pub avg_time: f64,
    pub last_called: f64,
}

impl FunctionProfile {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            call_count: 0,
            total_time: 0.0,
            min_time: f64::INFINITY,
            max_time: 0.0,
            avg_time: 0.0,
            last_called: 0.0,
        }
    }

    pub fn update(&mut self, elapsed: f64) {
        self.call_count += 1;
        self.total_time += elapsed;
        self.min_time = self.min_time.min(elapsed);
        self.max_time = self.max_time.max(elapsed);
        self.avg_time = self.total_time / self.call_count as f64;
        self.last_called = now_secs();
    }
}

fn now_secs() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

#[derive(Debug, Default)]
pub struct PerformanceProfiler {
    profiles: RwLock<HashMap<String, FunctionProfile>>,
    enabled: RwLock<bool>,
    start_time: f64,
}

impl PerformanceProfiler {
    pub fn new() -> Self {
        Self {
            profiles: RwLock::new(HashMap::new()),
            enabled: RwLock::new(true),
            start_time: now_secs(),
        }
    }

    pub fn enable(&self) {
        *self.enabled.write().expect("profiler lock poisoned") = true;
    }

    pub fn disable(&self) {
        *self.enabled.write().expect("profiler lock poisoned") = false;
    }

    pub fn is_enabled(&self) -> bool {
        *self.enabled.read().expect("profiler lock poisoned")
    }

    fn record_call(&self, name: &str, elapsed: f64) {
        let mut profiles = self.profiles.write().expect("profiler lock poisoned");
        let profile = profiles
            .entry(name.to_string())
            .or_insert_with(|| FunctionProfile::new(name));
        profile.update(elapsed);
    }

    pub fn get_profile(&self, name: &str) -> Option<FunctionProfile> {
        let profiles = self.profiles.read().expect("profiler lock poisoned");
        profiles.get(name).cloned()
    }

    pub fn get_all_profiles(&self) -> Vec<FunctionProfile> {
        let profiles = self.profiles.read().expect("profiler lock poisoned");
        let mut result: Vec<_> = profiles.values().cloned().collect();
        result.sort_by(|a, b| {
            b.total_time
                .partial_cmp(&a.total_time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        result
    }

    pub fn get_slowest_functions(&self, count: usize) -> Vec<FunctionProfile> {
        let all = self.get_all_profiles();
        all.into_iter().take(count).collect()
    }

    pub fn get_most_called(&self, count: usize) -> Vec<FunctionProfile> {
        let profiles = self.profiles.read().expect("profiler lock poisoned");
        let mut result: Vec<_> = profiles.values().cloned().collect();
        result.sort_by_key(|b| std::cmp::Reverse(b.call_count));
        result.into_iter().take(count).collect()
    }

    pub fn get_report(&self) -> String {
        let mut lines = vec![
            "=".repeat(70),
            "PERFORMANCE PROFILE REPORT".to_string(),
            "=".repeat(70),
            String::new(),
            format!("Total profiling time: {:.2}s", now_secs() - self.start_time),
            format!(
                "Total functions tracked: {}",
                self.profiles.read().expect("profiler lock poisoned").len()
            ),
            String::new(),
            "TOP 10 SLOWEST FUNCTIONS (by total time):".to_string(),
            "-".repeat(70),
        ];

        let slowest = self.get_slowest_functions(10);
        for (i, profile) in slowest.iter().enumerate() {
            lines.push(format!("{:2}. {}", i + 1, profile.name));
            lines.push(format!(
                "    Total: {:.3}s | Calls: {} | Avg: {:.2}ms | Min: {:.2}ms | Max: {:.2}ms",
                profile.total_time,
                profile.call_count,
                profile.avg_time * 1000.0,
                profile.min_time * 1000.0,
                profile.max_time * 1000.0
            ));
        }

        lines.push(String::new());
        lines.push("TOP 10 MOST CALLED FUNCTIONS:".to_string());
        lines.push("-".repeat(70));

        let most_called = self.get_most_called(10);
        for (i, profile) in most_called.iter().enumerate() {
            lines.push(format!(
                "{:2}. {} - {} calls ({:.3}s total)",
                i + 1,
                profile.name,
                profile.call_count,
                profile.total_time
            ));
        }

        lines.push(String::new());
        lines.push("=".repeat(70));

        lines.join("\n")
    }

    pub fn reset(&self) {
        self.profiles.write().expect("profiler lock poisoned").clear();
    }

    pub fn get_stats_dict(&self) -> serde_json::Value {
        let slowest: Vec<_> = self
            .get_slowest_functions(10)
            .into_iter()
            .map(|p| {
                serde_json::json!({
                    "name": p.name,
                    "total_time": p.total_time,
                    "call_count": p.call_count,
                    "avg_time": p.avg_time,
                    "min_time": p.min_time,
                    "max_time": p.max_time,
                })
            })
            .collect();

        let most_called: Vec<_> = self
            .get_most_called(10)
            .into_iter()
            .map(|p| {
                serde_json::json!({
                    "name": p.name,
                    "call_count": p.call_count,
                    "total_time": p.total_time,
                })
            })
            .collect();

        serde_json::json!({
            "total_functions": self.profiles.read().expect("profiler lock poisoned").len(),
            "total_time": now_secs() - self.start_time,
            "slowest": slowest,
            "most_called": most_called,
        })
    }
}

static GLOBAL_PROFILER: LazyLock<Arc<PerformanceProfiler>> =
    LazyLock::new(|| Arc::new(PerformanceProfiler::new()));

pub fn get_profiler() -> Arc<PerformanceProfiler> {
    GLOBAL_PROFILER.clone()
}

pub fn profile_function<F, R>(name: Option<&str>, func: F) -> R
where
    F: FnOnce() -> R,
{
    let profiler = get_profiler();
    if !profiler.is_enabled() {
        return func();
    }

    let start = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();

    let result = func();

    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
        - start;

    let profile_name = name.unwrap_or("anonymous");
    profiler.record_call(profile_name, elapsed);

    result
}

pub fn profile_block<F, R>(name: &str, func: F) -> R
where
    F: FnOnce() -> R,
{
    let profiler = get_profiler();
    if !profiler.is_enabled() {
        return func();
    }

    let start = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();

    let result = func();

    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
        - start;

    profiler.record_call(name, elapsed);

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_profile_new() {
        let profile = FunctionProfile::new("test_func");
        assert_eq!(profile.name, "test_func");
        assert_eq!(profile.call_count, 0);
    }

    #[test]
    fn test_function_profile_update() {
        let mut profile = FunctionProfile::new("test");
        profile.update(0.1);
        assert_eq!(profile.call_count, 1);
        assert!((profile.total_time - 0.1).abs() < 1e-10);
        assert!((profile.min_time - 0.1).abs() < 1e-10);
        assert!((profile.max_time - 0.1).abs() < 1e-10);

        profile.update(0.2);
        assert_eq!(profile.call_count, 2);
        assert!((profile.total_time - 0.3).abs() < 1e-10);
        assert!((profile.avg_time - 0.15).abs() < 1e-10);
        assert!((profile.min_time - 0.1).abs() < 1e-10);
        assert!((profile.max_time - 0.2).abs() < 1e-10);
    }

    #[test]
    fn test_profiler_new() {
        let profiler = PerformanceProfiler::new();
        assert!(profiler.is_enabled());
    }

    #[test]
    fn test_profiler_enable_disable() {
        let profiler = PerformanceProfiler::new();
        profiler.disable();
        assert!(!profiler.is_enabled());
        profiler.enable();
        assert!(profiler.is_enabled());
    }

    #[test]
    fn test_profiler_record_call() {
        let profiler = PerformanceProfiler::new();
        profiler.record_call("test_func", 0.05);
        profiler.record_call("test_func", 0.10);

        let profile = profiler.get_profile("test_func").unwrap();
        assert_eq!(profile.call_count, 2);
        assert!((profile.total_time - 0.15).abs() < 1e-10);
    }

    #[test]
    fn test_profiler_get_slowest() {
        let profiler = PerformanceProfiler::new();
        profiler.record_call("slow", 1.0);
        profiler.record_call("fast", 0.01);

        let slowest = profiler.get_slowest_functions(1);
        assert_eq!(slowest[0].name, "slow");
    }

    #[test]
    fn test_profiler_get_most_called() {
        let profiler = PerformanceProfiler::new();
        for _ in 0..10 {
            profiler.record_call("frequent", 0.001);
        }
        profiler.record_call("rare", 1.0);

        let most_called = profiler.get_most_called(1);
        assert_eq!(most_called[0].name, "frequent");
    }

    #[test]
    fn test_profiler_reset() {
        let profiler = PerformanceProfiler::new();
        profiler.record_call("test", 0.1);
        profiler.reset();
        assert!(profiler.get_profile("test").is_none());
    }

    #[test]
    #[ignore = "flaky in CI: race condition on global state"]
    fn test_global_profiler() {
        let profiler = get_profiler();
        profiler.reset();
        profiler.record_call("global_test", 0.5);
        let profile = profiler.get_profile("global_test");
        assert!(profile.is_some());
    }

    #[test]
    fn test_profile_function() {
        let profiler = get_profiler();
        profiler.reset();

        let result = profile_function(Some("my_func"), || {
            std::thread::sleep(std::time::Duration::from_millis(10));
            42
        });

        assert_eq!(result, 42);
        let profile = profiler.get_profile("my_func");
        assert!(profile.is_some());
    }

    #[test]
    fn test_profile_block() {
        let profiler = get_profiler();
        profiler.reset();

        let result = profile_block("my_block", || {
            std::thread::sleep(std::time::Duration::from_millis(10));
            100
        });

        assert_eq!(result, 100);
        let profile = profiler.get_profile("my_block");
        assert!(profile.is_some());
    }

    #[test]
    fn test_profiler_get_report() {
        let profiler = PerformanceProfiler::new();
        profiler.record_call("func1", 0.1);
        profiler.record_call("func2", 0.2);

        let report = profiler.get_report();
        assert!(report.contains("PERFORMANCE PROFILE REPORT"));
        assert!(report.contains("func1"));
        assert!(report.contains("func2"));
    }

    #[test]
    fn test_profiler_stats_dict() {
        let profiler = PerformanceProfiler::new();
        profiler.record_call("test_stats", 0.5);

        let stats = profiler.get_stats_dict();
        assert!(stats.get("total_functions").is_some());
        assert!(stats.get("slowest").is_some());
        assert!(stats.get("most_called").is_some());
    }
}
