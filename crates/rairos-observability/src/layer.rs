//! Tracing Layer for rairos-observability
//!
//! This module implements a `tracing::Layer` that bridges `tracing` events
//! with the existing `rairos-observability` infrastructure.

use std::path::PathBuf;
use std::sync::Arc;

use tracing::{Event, Subscriber};
use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter, Registry};
use tracing_appender::non_blocking::WorkerGuard;

use crate::{EventEmitter, LogRecord, MetricsCollector, get_trace_id, new_span_id};

#[derive(Clone)]
pub struct ObservabilityLayer {
    emitter: Option<Arc<EventEmitter>>,
    metrics: Option<Arc<MetricsCollector>>,
}

impl ObservabilityLayer {
    pub fn new() -> Self {
        Self {
            emitter: None,
            metrics: None,
        }
    }

    pub fn with_emitter(mut self, emitter: Arc<EventEmitter>) -> Self {
        self.emitter = Some(emitter);
        self
    }

    pub fn with_metrics(mut self, metrics: Arc<MetricsCollector>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    fn event_to_log_record(&self, event: &Event) -> Option<LogRecord> {
        let metadata = event.metadata();

        let trace_id = get_trace_id();
        let span_id = Some(new_span_id());

        let target = metadata.target().to_string();
        let level = metadata.level().to_string();

        let message = format!("{:?}", event);

        Some(LogRecord {
            timestamp: crate::now_iso(),
            level,
            logger: target,
            message,
            module: String::new(),
            function: String::new(),
            line: 0,
            trace_id,
            span_id,
            exception: None,
            extra: None,
        })
    }
}

impl Default for ObservabilityLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Subscriber> tracing_subscriber::layer::Layer<S> for ObservabilityLayer {
    fn on_event(&self, event: &Event, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let _ = ctx;
        if let Some(log_record) = self.event_to_log_record(event) {
            if let Some(emitter) = &self.emitter {
                emitter.emit(&log_record.message, log_record.extra.clone());
            }
        }
    }
}

pub fn log_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ai_research_os/logs")
}

pub fn init_logging() -> WorkerGuard {
    use tracing_appender::rolling::{RollingFileAppender, Rotation};

    let log_dir = log_dir();
    std::fs::create_dir_all(&log_dir).ok();

    let file_appender = RollingFileAppender::new(
        Rotation::DAILY,
        &log_dir,
        "rairos.log",
    );

    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let subscriber = Registry::default()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_writer(std::io::stdout)
                .with_ansi(true)
        )
        .with(
            fmt::layer()
                .with_writer(file_writer)
                .with_ansi(false)
        );

    tracing::subscriber::set_global_default(subscriber)
        .expect("failed to set tracing subscriber - another subscriber may already be set");

    guard
}

pub fn cleanup_old_logs(log_dir: &PathBuf, days: u32) -> std::io::Result<usize> {
    use std::time::{SystemTime, UNIX_EPOCH};

    if !log_dir.exists() {
        return Ok(0);
    }

    let cutoff = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64 - (days as i64 * 24 * 60 * 60);

    let mut removed = 0;
    for entry in std::fs::read_dir(log_dir)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if let Ok(modified) = metadata.modified() {
            if modified
                .duration_since(UNIX_EPOCH)
                .map(|d| (d.as_secs() as i64) < cutoff)
                .unwrap_or(false)
            {
                std::fs::remove_file(entry.path())?;
                removed += 1;
            }
        }
    }
    Ok(removed)
}

#[cfg(feature = "otlp")]
pub fn init_otlp_tracing(
    endpoint: &str,
) -> Result<tracing_opentelemetry::OpenTelemetryLayer<tracing_subscriber::Registry, opentelemetry_sdk::trace::Tracer>, Box<dyn std::error::Error>> {
    use opentelemetry::trace::TracerProvider;
    use opentelemetry_otlp::WithExportConfig;
    use opentelemetry_sdk::runtime;
    use tracing_opentelemetry::OpenTelemetryLayer;

    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(endpoint);

    let tracer_provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(opentelemetry_sdk::trace::Config::default())
        .install_batch(runtime::Tokio)?;

    let tracer = tracer_provider.tracer("rairos");
    let otel_layer = OpenTelemetryLayer::new(tracer);
    Ok(otel_layer)
}

#[cfg(not(feature = "otlp"))]
pub fn init_otlp_tracing(_endpoint: &str) -> Result<(), Box<dyn std::error::Error>> {
    Err("OTLP feature not enabled. Add `otlp = [\"rairos-observability/otlp\"]` to enable.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_dir() {
        let dir = log_dir();
        assert!(dir.to_string_lossy().contains(".ai_research_os"));
    }

    #[test]
    fn test_observability_layer_default() {
        let layer = ObservabilityLayer::new();
        assert!(layer.emitter.is_none());
        assert!(layer.metrics.is_none());
    }
}
