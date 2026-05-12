//! rairos-observability — Structured logging, correlation IDs, event tracking, and metrics.
//!
//! Ported from `core/observability.py`.

use chrono::Utc;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn now_secs_f64() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

impl LogLevel {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "debug" => LogLevel::Debug,
            "info" => LogLevel::Info,
            "warning" => LogLevel::Warning,
            "error" => LogLevel::Error,
            "critical" => LogLevel::Critical,
            _ => LogLevel::Info,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRecord {
    pub timestamp: String,
    pub level: String,
    pub logger: String,
    pub message: String,
    pub module: String,
    pub function: String,
    pub line: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exception: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<HashMap<String, serde_json::Value>>,
}

impl LogRecord {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

#[derive(Debug, Clone)]
pub struct CorrelationContext {
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub parent_span_id: Option<String>,
}

thread_local! {
    static CORRELATION: RwLock<CorrelationContext> = const { RwLock::new(CorrelationContext {
        trace_id: None,
        span_id: None,
        parent_span_id: None,
    }) };
}

pub fn get_trace_id() -> Option<String> {
    CORRELATION.with(|c| c.read().unwrap().trace_id.clone())
}

pub fn new_span_id() -> String {
    let id: String = rand::thread_rng()
        .sample_iter(rand::distributions::Alphanumeric)
        .take(8)
        .map(char::from)
        .collect();
    CORRELATION.with(|c| {
        let mut ctx = c.write().unwrap();
        ctx.span_id = Some(id.clone());
    });
    id
}

pub fn set_trace_id(trace_id: Option<String>) {
    CORRELATION.with(|c| {
        let mut ctx = c.write().unwrap();
        ctx.trace_id = trace_id;
    });
}

pub fn correlation_context<F, R>(trace_id: Option<String>, span_id: Option<String>, f: F) -> R
where
    F: FnOnce() -> R,
{
    let (prev_trace, prev_span, prev_parent) = CORRELATION.with(|c| {
        let mut ctx = c.write().unwrap();
        let prev_trace = ctx.trace_id.clone();
        let prev_span = ctx.span_id.clone();
        let prev_parent = ctx.parent_span_id.clone();

        if let Some(tid) = trace_id {
            ctx.trace_id = Some(tid);
        } else if ctx.trace_id.is_none() {
            ctx.trace_id = Some(
                rand::thread_rng()
                    .sample_iter(rand::distributions::Alphanumeric)
                    .take(16)
                    .map(char::from)
                    .collect(),
            );
        }

        if let Some(sid) = span_id {
            ctx.span_id = Some(sid);
        } else if ctx.span_id.is_none() {
            ctx.span_id = Some(
                rand::thread_rng()
                    .sample_iter(rand::distributions::Alphanumeric)
                    .take(8)
                    .map(char::from)
                    .collect(),
            );
        }

        ctx.parent_span_id = ctx.span_id.clone();

        (prev_trace, prev_span, prev_parent)
    });

    let result = f();

    CORRELATION.with(|c| {
        let mut ctx = c.write().unwrap();
        ctx.trace_id = prev_trace;
        ctx.span_id = prev_span;
        ctx.parent_span_id = prev_parent;
    });

    result
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub event: String,
    pub timestamp: String,
    pub trace_id: String,
    pub span_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Default)]
pub struct EventEmitter {
    buffer: Arc<RwLock<Vec<Event>>>,
    capacity: usize,
}

impl EventEmitter {
    pub fn new() -> Self {
        Self::with_capacity(10000)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Arc::new(RwLock::new(Vec::new())),
            capacity,
        }
    }

    pub fn emit(&self, event_name: &str, extra: Option<HashMap<String, serde_json::Value>>) {
        let trace_id = get_trace_id().unwrap_or_default();
        let span_id = CORRELATION
            .with(|c| c.read().unwrap().span_id.clone())
            .unwrap_or_default();

        let event = Event {
            event: event_name.to_string(),
            timestamp: now_iso(),
            trace_id,
            span_id,
            extra,
        };

        let mut buffer = self.buffer.write().unwrap();
        buffer.push(event);
        if buffer.len() > self.capacity {
            buffer.remove(0);
        }
    }

    pub fn get_events(
        &self,
        event_type: Option<&str>,
        trace_id: Option<&str>,
        limit: usize,
    ) -> Vec<Event> {
        let buffer = self.buffer.read().unwrap();
        let mut events: Vec<Event> = buffer.clone();

        if let Some(et) = event_type {
            events.retain(|e| e.event == et);
        }
        if let Some(tid) = trace_id {
            events.retain(|e| e.trace_id == tid);
        }

        let start = events.len().saturating_sub(limit);
        events[start..].to_vec()
    }

    pub fn get_recent(&self, n: usize) -> Vec<Event> {
        self.get_events(None, None, n)
    }

    pub fn clear(&self) {
        self.buffer.write().unwrap().clear();
    }
}

static GLOBAL_EMITTER: LazyLock<EventEmitter, fn() -> EventEmitter> =
    LazyLock::new(|| EventEmitter::with_capacity(10000));

pub fn emit_research_event(event: &str, extra: Option<HashMap<String, serde_json::Value>>) {
    GLOBAL_EMITTER.emit(event, extra);
}

pub fn get_recent_events(n: usize) -> Vec<Event> {
    GLOBAL_EMITTER.get_recent(n)
}

#[derive(Debug, Clone, Default)]
pub struct MetricsCollector {
    counters: Arc<RwLock<HashMap<String, f64>>>,
    gauges: Arc<RwLock<HashMap<String, f64>>>,
    #[allow(dead_code)]
    hist_maxlen: usize,
    histograms: Arc<RwLock<HashMap<String, Vec<f64>>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            counters: Arc::new(RwLock::new(HashMap::new())),
            gauges: Arc::new(RwLock::new(HashMap::new())),
            histograms: Arc::new(RwLock::new(HashMap::new())),
            hist_maxlen: 1000,
        }
    }

    pub fn inc(&self, subsystem: &str, name: &str, value: f64) {
        let key = format!("{}.{}", subsystem, name);
        let mut counters = self.counters.write().unwrap();
        *counters.entry(key).or_insert(0.0) += value;
    }

    pub fn counter(&self, subsystem: &str, name: &str) -> f64 {
        let key = format!("{}.{}", subsystem, name);
        self.counters
            .read()
            .unwrap()
            .get(&key)
            .copied()
            .unwrap_or(0.0)
    }

    pub fn set(&self, subsystem: &str, name: &str, value: f64) {
        let key = format!("{}.{}", subsystem, name);
        self.gauges.write().unwrap().insert(key, value);
    }

    pub fn gauge(&self, subsystem: &str, name: &str) -> Option<f64> {
        let key = format!("{}.{}", subsystem, name);
        self.gauges.read().unwrap().get(&key).copied()
    }

    pub fn observe(&self, subsystem: &str, name: &str, value: f64) {
        let key = format!("{}.{}", subsystem, name);
        let mut histograms = self.histograms.write().unwrap();
        let hist = histograms.entry(key).or_default();
        hist.push(value);
        if hist.len() > 1000 {
            hist.remove(0);
        }
    }

    pub fn histogram_stats(&self, subsystem: &str, name: &str) -> HashMap<String, f64> {
        let key = format!("{}.{}", subsystem, name);
        let histograms = self.histograms.read().unwrap();
        let values: Vec<f64> = histograms.get(&key).cloned().unwrap_or_default();

        if values.is_empty() {
            return HashMap::new();
        }

        let mut sorted = values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n = sorted.len();

        let mut stats = HashMap::new();
        stats.insert("count".to_string(), n as f64);
        stats.insert("min".to_string(), sorted[0]);
        stats.insert("max".to_string(), sorted[n - 1]);
        stats.insert("mean".to_string(), sorted.iter().sum::<f64>() / n as f64);
        stats.insert("p50".to_string(), sorted[n / 2]);
        stats.insert(
            "p95".to_string(),
            sorted[(n as f64 * 0.95) as usize].min(sorted[n - 1]),
        );
        stats.insert(
            "p99".to_string(),
            sorted[(n as f64 * 0.99) as usize].min(sorted[n - 1]),
        );

        stats
    }

    pub fn export_prometheus(&self) -> String {
        let ts = now_secs_f64();
        let mut lines = Vec::new();

        let counters = self.counters.read().unwrap();
        for (key, value) in counters.iter() {
            lines.push(format!("# TYPE {} counter", key));
            lines.push(format!("{} {} {}", key, value, (ts * 1000.0) as i64));
        }

        let gauges = self.gauges.read().unwrap();
        for (key, value) in gauges.iter() {
            lines.push(format!("# TYPE {} gauge", key));
            lines.push(format!("{} {} {}", key, value, (ts * 1000.0) as i64));
        }

        let histograms = self.histograms.read().unwrap();
        for (key, values) in histograms.iter() {
            if values.is_empty() {
                continue;
            }
            lines.push(format!("# TYPE {} histogram", key));
            let mut sorted = values.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let n = sorted.len();
            for boundary in [0.5, 0.95, 0.99] {
                let idx = ((n as f64 * boundary) as usize).min(n - 1);
                lines.push(format!(
                    "{}_bucket{} {} {}",
                    key,
                    boundary,
                    sorted[idx],
                    (ts * 1000.0) as i64
                ));
            }
            lines.push(format!(
                "{}_sum {} {}",
                key,
                sorted.iter().sum::<f64>(),
                (ts * 1000.0) as i64
            ));
            lines.push(format!("{}_count {} {}", key, n, (ts * 1000.0) as i64));
        }

        lines.join("\n")
    }

    pub fn summary(&self) -> HashMap<String, serde_json::Value> {
        let mut result = HashMap::new();

        let counters = self.counters.read().unwrap();
        result.insert(
            "counters".to_string(),
            serde_json::to_value(&*counters).unwrap_or(serde_json::Value::Null),
        );

        let gauges = self.gauges.read().unwrap();
        result.insert(
            "gauges".to_string(),
            serde_json::to_value(&*gauges).unwrap_or(serde_json::Value::Null),
        );

        let histograms = self.histograms.read().unwrap();
        let hist_summary: HashMap<String, HashMap<String, f64>> = histograms
            .keys()
            .map(|k| {
                let stats = Self::new().histogram_stats(
                    k.split('.').next().unwrap_or(""),
                    k.split('.').nth(1).unwrap_or(""),
                );
                (k.clone(), stats)
            })
            .collect();
        result.insert(
            "histograms".to_string(),
            serde_json::to_value(hist_summary).unwrap_or(serde_json::Value::Null),
        );

        result
    }
}

static GLOBAL_METRICS: LazyLock<MetricsCollector, fn() -> MetricsCollector> =
    LazyLock::new(MetricsCollector::new);

pub fn get_metrics() -> MetricsCollector {
    GLOBAL_METRICS.clone()
}

pub fn track_duration<F, R>(subsystem: &str, name: &str, func: F) -> R
where
    F: FnOnce() -> R,
{
    let start = std::time::Instant::now();
    let result = func();
    let duration = start.elapsed().as_secs_f64();
    get_metrics().observe(subsystem, &format!("{}.duration", name), duration);
    result
}

#[derive(Debug, Clone)]
pub struct LogSampler {
    pub rate: f64,
}

impl LogSampler {
    pub fn new(rate: f64) -> Self {
        Self { rate }
    }

    pub fn should_emit(&self, _logger_name: &str, level: &str) -> bool {
        if level == "error" || level == "critical" {
            return true;
        }
        rand::thread_rng().gen::<f64>() < self.rate
    }
}

pub fn setup_observability(level: &str, json_logs: bool, log_file: Option<&str>) {
    let _log_level = LogLevel::from_str(level);
    let _json = json_logs;
    let _file = log_file;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_correlation_context() {
        let result = correlation_context(
            Some("trace123".to_string()),
            Some("span456".to_string()),
            || {
                assert_eq!(get_trace_id(), Some("trace123".to_string()));
                "done"
            },
        );
        assert_eq!(result, "done");
    }

    #[test]
    fn test_new_span_id() {
        let id1 = new_span_id();
        assert_eq!(id1.len(), 8);
        let id2 = new_span_id();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_event_emitter_new() {
        let emitter = EventEmitter::with_capacity(100);
        assert_eq!(emitter.get_recent(10).len(), 0);
    }

    #[test]
    fn test_event_emitter_emit() {
        let emitter = EventEmitter::with_capacity(100);
        emitter.emit("test_event", None);
        let events = emitter.get_recent(10);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event, "test_event");
    }

    #[test]
    fn test_event_emitter_capacity() {
        let emitter = EventEmitter::with_capacity(3);
        emitter.emit("e1", None);
        emitter.emit("e2", None);
        emitter.emit("e3", None);
        emitter.emit("e4", None);
        assert_eq!(emitter.get_recent(10).len(), 3);
    }

    #[test]
    fn test_event_emitter_filter() {
        let emitter = EventEmitter::with_capacity(100);
        emitter.emit("event_a", None);
        emitter.emit("event_b", None);
        let filtered = emitter.get_events(Some("event_a"), None, 10);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_global_emitter() {
        emit_research_event("global_test", None);
        let events = get_recent_events(10);
        assert!(events.iter().any(|e| e.event == "global_test"));
    }

    #[test]
    fn test_metrics_collector_new() {
        let m = MetricsCollector::new();
        assert_eq!(m.counter("test", "count"), 0.0);
    }

    #[test]
    fn test_metrics_inc() {
        let m = MetricsCollector::new();
        m.inc("test", "requests", 1.0);
        m.inc("test", "requests", 2.0);
        assert_eq!(m.counter("test", "requests"), 3.0);
    }

    #[test]
    fn test_metrics_gauge() {
        let m = MetricsCollector::new();
        m.set("test", "temperature", 98.6);
        assert_eq!(m.gauge("test", "temperature"), Some(98.6));
    }

    #[test]
    fn test_metrics_histogram() {
        let m = MetricsCollector::new();
        m.observe("test", "latency", 1.0);
        m.observe("test", "latency", 2.0);
        m.observe("test", "latency", 3.0);
        let stats = m.histogram_stats("test", "latency");
        assert_eq!(stats.get("count"), Some(&3.0));
        assert_eq!(stats.get("min"), Some(&1.0));
        assert_eq!(stats.get("max"), Some(&3.0));
    }

    #[test]
    fn test_metrics_export_prometheus() {
        let m = MetricsCollector::new();
        m.inc("http", "requests_total", 5.0);
        m.set("system", "cpu_usage", 0.75);
        let output = m.export_prometheus();
        assert!(output.contains("http.requests_total"));
        assert!(output.contains("system.cpu_usage"));
    }

    #[test]
    fn test_log_sampler() {
        let sampler = LogSampler::new(0.5);
        assert!(sampler.should_emit("mylogger", "error"));
        assert!(sampler.should_emit("mylogger", "critical"));
    }

    #[test]
    fn test_get_metrics_returns_global() {
        let m = get_metrics();
        m.inc("global", "test", 1.0);
        assert_eq!(m.counter("global", "test"), 1.0);
    }

    #[test]
    fn test_track_duration() {
        let result = track_duration("test", "my_op", || {
            thread::sleep(std::time::Duration::from_millis(10));
            42
        });
        assert_eq!(result, 42);
        let stats = get_metrics().histogram_stats("test", "my_op.duration");
        assert!(stats.contains_key("count"));
    }
}
