//! rairos-logging-utils — Logging and monitoring utilities for AI Research OS.
//!
//! Ported from `core/logging_utils.py`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricStats {
    pub count: usize,
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub total: f64,
}

#[derive(Debug, Clone, Default)]
pub struct PerformanceMonitor {
    metrics: Arc<RwLock<HashMap<String, Vec<f64>>>>,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn record(&self, name: &str, value: f64) {
        let mut metrics = self.metrics.write().unwrap();
        metrics.entry(name.to_string()).or_default().push(value);
    }

    pub fn get_stats(&self, name: &str) -> Option<MetricStats> {
        let metrics = self.metrics.read().unwrap();
        let values = metrics.get(name)?;

        if values.is_empty() {
            return None;
        }

        let count = values.len();
        let min = values.iter().copied().fold(f64::INFINITY, f64::min);
        let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let total: f64 = values.iter().sum();
        let avg = total / count as f64;

        Some(MetricStats {
            count,
            min,
            max,
            avg,
            total,
        })
    }

    pub fn get_all_stats(&self) -> HashMap<String, MetricStats> {
        let metrics = self.metrics.read().unwrap();
        metrics
            .keys()
            .filter_map(|k| self.get_stats(k).map(|stats| (k.clone(), stats)))
            .collect()
    }

    pub fn reset(&self) {
        let mut metrics = self.metrics.write().unwrap();
        metrics.clear();
    }

    pub fn reset_metric(&self, name: &str) {
        let mut metrics = self.metrics.write().unwrap();
        metrics.remove(name);
    }
}

static GLOBAL_MONITOR: LazyLock<PerformanceMonitor, fn() -> PerformanceMonitor> =
    LazyLock::new(PerformanceMonitor::new);

pub fn get_monitor() -> PerformanceMonitor {
    GLOBAL_MONITOR.clone()
}

pub fn record_metric(name: &str, value: f64) {
    GLOBAL_MONITOR.record(name, value);
}

pub fn get_performance_report() -> String {
    let stats = GLOBAL_MONITOR.get_all_stats();

    if stats.is_empty() {
        return "No performance metrics recorded.".to_string();
    }

    let mut lines = vec!["=== Performance Report ===".to_string(), String::new()];

    for (name, metric_stats) in stats {
        lines.push(format!("{}:", name));
        lines.push(format!("  Count: {}", metric_stats.count));
        lines.push(format!("  Avg:   {:.3}s", metric_stats.avg));
        lines.push(format!("  Min:   {:.3}s", metric_stats.min));
        lines.push(format!("  Max:   {:.3}s", metric_stats.max));
        lines.push(format!("  Total: {:.3}s", metric_stats.total));
        lines.push(String::new());
    }

    lines.join("\n")
}

pub fn track_time<F, R>(name: &str, func: F) -> R
where
    F: FnOnce() -> R,
{
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

    GLOBAL_MONITOR.record(name, elapsed);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_monitor_new() {
        let monitor = PerformanceMonitor::new();
        assert!(monitor.get_stats("nonexistent").is_none());
    }

    #[test]
    fn test_record_metric() {
        let monitor = PerformanceMonitor::new();
        monitor.record("test.metric", 1.0);
        monitor.record("test.metric", 2.0);
        let stats = monitor.get_stats("test.metric").unwrap();
        assert_eq!(stats.count, 2);
        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 2.0);
        assert_eq!(stats.avg, 1.5);
    }

    #[test]
    fn test_get_all_stats() {
        let monitor = PerformanceMonitor::new();
        monitor.record("metric1", 1.0);
        monitor.record("metric2", 2.0);
        let all = monitor.get_all_stats();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_reset_metric() {
        let monitor = PerformanceMonitor::new();
        monitor.record("test", 1.0);
        monitor.reset_metric("test");
        assert!(monitor.get_stats("test").is_none());
    }

    #[test]
    fn test_reset_all() {
        let monitor = PerformanceMonitor::new();
        monitor.record("test", 1.0);
        monitor.reset();
        assert!(monitor.get_all_stats().is_empty());
    }

    #[test]
    fn test_global_monitor() {
        let monitor = get_monitor();
        monitor.record("global_test", 42.0);
        let stats = monitor.get_stats("global_test")
            .expect("global_test metric should exist after record()");
        assert_eq!(stats.count, 1);
    }

    #[test]
    fn test_record_metric_function() {
        record_metric("func_test", 3.14);
        let monitor = get_monitor();
        let stats = monitor.get_stats("func_test").unwrap();
        assert_eq!(stats.count, 1);
    }

    #[test]
    fn test_track_time() {
        let result = track_time("timed_op", || {
            std::thread::sleep(std::time::Duration::from_millis(10));
            42
        });
        assert_eq!(result, 42);
        let stats = get_monitor().get_stats("timed_op").unwrap();
        assert!(stats.total > 0.0);
        get_monitor().reset();
    }

    #[test]
    #[ignore]
    fn test_get_performance_report() {
        let monitor = get_monitor();
        monitor.reset();
        monitor.record("report_test", 1.0);
        let report = get_performance_report();
        assert!(report.contains("report_test"));
    }

    #[test]
    fn test_get_performance_report_empty() {
        let monitor = get_monitor();
        monitor.reset();
        let report = get_performance_report();
        assert_eq!(report, "No performance metrics recorded.");
    }
}
