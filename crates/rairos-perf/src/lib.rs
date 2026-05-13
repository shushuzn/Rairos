//! rairos-perf — Performance monitoring and timing utilities.

#![allow(dead_code)]
//!
//! Ported from `core/logging_utils.py`.
//!
//! Features:
//! - PerformanceMonitor with metric recording and statistics
//! - ScopedTimer RAII guard for automatic timing via Drop
//! - Global performance monitor with LazyLock

use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::time::Instant;

/// Statistics for a named metric.
#[derive(Debug, Clone)]
pub struct MetricStats {
    pub count: usize,
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub total: f64,
}

/// Thread-safe performance monitor tracking named metrics.
#[derive(Debug, Default)]
pub struct PerformanceMonitor {
    metrics: Mutex<HashMap<String, Vec<f64>>>,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a value for a named metric.
    pub fn record(&self, name: &str, value: f64) {
        let mut guard = self.metrics.lock().unwrap();
        guard.entry(name.to_string()).or_default().push(value);
    }

    /// Get statistics for a named metric.
    pub fn get_stats(&self, name: &str) -> Option<MetricStats> {
        let guard = self.metrics.lock().unwrap();
        guard.get(name).map(|values| {
            let count = values.len();
            let total = values.iter().sum::<f64>();
            let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let avg = if count > 0 { total / count as f64 } else { 0.0 };
            MetricStats {
                count,
                min,
                max,
                avg,
                total,
            }
        })
    }

    /// Get statistics for all metrics.
    pub fn get_all_stats(&self) -> HashMap<String, MetricStats> {
        let guard = self.metrics.lock().unwrap();
        guard
            .iter()
            .filter_map(|(name, values)| {
                if values.is_empty() {
                    return None;
                }
                let count = values.len();
                let total = values.iter().sum::<f64>();
                let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
                let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let avg = total / count as f64;
                Some((
                    name.clone(),
                    MetricStats {
                        count,
                        min,
                        max,
                        avg,
                        total,
                    },
                ))
            })
            .collect()
    }

    /// Reset all metrics.
    pub fn reset(&self) {
        let mut guard = self.metrics.lock().unwrap();
        guard.clear();
    }

    /// Reset a specific metric.
    pub fn reset_metric(&self, name: &str) {
        let mut guard = self.metrics.lock().unwrap();
        guard.remove(name);
    }

    /// Get a performance report as a formatted string.
    pub fn get_report(&self) -> String {
        let stats = self.get_all_stats();
        if stats.is_empty() {
            return "No performance metrics recorded.".to_string();
        }

        let mut lines = vec!["=== Performance Report ===".to_string(), String::new()];
        for (name, s) in stats {
            lines.push(format!("{}:", name));
            lines.push(format!("  Count: {}", s.count));
            lines.push(format!("  Avg:   {:.3}s", s.avg));
            lines.push(format!("  Min:   {:.3}s", s.min));
            lines.push(format!("  Max:   {:.3}s", s.max));
            lines.push(format!("  Total: {:.3}s", s.total));
            lines.push(String::new());
        }
        lines.join("\n")
    }
}

// ─── Global monitor ─────────────────────────────────────────────────────────

static GLOBAL_MONITOR: LazyLock<PerformanceMonitor> = LazyLock::new(PerformanceMonitor::new);

/// Get the global performance monitor.
pub fn get_monitor() -> &'static PerformanceMonitor {
    &GLOBAL_MONITOR
}

// ─── Scoped Timer ───────────────────────────────────────────────────────────

/// RAII guard that records elapsed time when dropped.
pub struct ScopedTimer<'a> {
    name: &'a str,
    start: Instant,
    monitor: &'a PerformanceMonitor,
}

impl<'a> ScopedTimer<'a> {
    /// Start timing a named scope. Records time when dropped.
    pub fn new(name: &'a str, monitor: &'a PerformanceMonitor) -> Self {
        Self {
            name,
            start: Instant::now(),
            monitor,
        }
    }
}

impl Drop for ScopedTimer<'_> {
    fn drop(&mut self) {
        let duration = self.start.elapsed().as_secs_f64();
        self.monitor.record(self.name, duration);
    }
}

/// RAII guard that records elapsed time using the global monitor.
pub struct GlobalTimer<'a> {
    timer: ScopedTimer<'a>,
}

impl<'a> GlobalTimer<'a> {
    /// Start timing with the global monitor.
    pub fn new(name: &'a str) -> Self {
        Self {
            timer: ScopedTimer::new(name, get_monitor()),
        }
    }
}

/// Time a closure using the global monitor.
pub fn time_scope<F>(name: &str, f: F)
where
    F: FnOnce(),
{
    let _timer = GlobalTimer::new(name);
    f();
}

/// Time a closure and return its result.
pub fn time_scope_with_result<F, T>(name: &str, f: F) -> T
where
    F: FnOnce() -> T,
{
    let _timer = GlobalTimer::new(name);
    f()
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_get_stats() {
        let m = PerformanceMonitor::new();
        m.record("test", 1.0);
        m.record("test", 2.0);
        m.record("test", 3.0);

        let stats = m.get_stats("test").unwrap();
        assert_eq!(stats.count, 3);
        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 3.0);
        assert_eq!(stats.avg, 2.0);
        assert!((stats.total - 6.0).abs() < 0.001);
    }

    #[test]
    fn test_get_stats_unknown_metric() {
        let m = PerformanceMonitor::new();
        assert!(m.get_stats("unknown").is_none());
    }

    #[test]
    fn test_reset_metric() {
        let m = PerformanceMonitor::new();
        m.record("x", 1.0);
        m.reset_metric("x");
        assert!(m.get_stats("x").is_none());
    }

    #[test]
    fn test_get_all_stats() {
        let m = PerformanceMonitor::new();
        m.record("a", 1.0);
        m.record("b", 2.0);
        let all = m.get_all_stats();
        assert_eq!(all.len(), 2);
        assert!(all.contains_key("a"));
        assert!(all.contains_key("b"));
    }

    #[test]
    fn test_scoped_timer_records() {
        let m = PerformanceMonitor::new();
        {
            let _t = ScopedTimer::new("timer_test", &m);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let stats = m.get_stats("timer_test").unwrap();
        assert!(stats.count >= 1);
        assert!(stats.total >= 0.01);
    }

    #[test]
    fn test_global_monitor() {
        let m = get_monitor();
        m.reset();
        m.record("global_test", 5.0);
        let stats = m.get_stats("global_test").unwrap();
        assert_eq!(stats.count, 1);
        assert_eq!(stats.total, 5.0);
    }

    #[test]
    fn test_performance_report() {
        let m = PerformanceMonitor::new();
        m.record("op", 1.5);
        let report = m.get_report();
        assert!(report.contains("Performance Report"));
        assert!(report.contains("op"));
    }

    #[test]
    fn test_performance_report_empty() {
        let m = PerformanceMonitor::new();
        let report = m.get_report();
        assert_eq!(report, "No performance metrics recorded.");
    }

    #[test]
    fn test_time_scope() {
        let m = get_monitor();
        m.reset();
        time_scope("scoped_op", || {
            std::thread::sleep(std::time::Duration::from_millis(5));
        });
        let stats = m.get_stats("scoped_op").unwrap();
        assert!(stats.count >= 1);
    }
}
