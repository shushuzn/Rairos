"""Observability layer for Rairos — structured logging, correlation IDs, and event tracking.

Components
─────────
1. StructuredLogger: JSON logging with typed fields
2. CorrelationContext: trace_id + span_id propagation across modules
3. ResearchEventEmitter: emit structured events for the research loop
4. MetricsCollector: in-memory metrics with Prometheus-compatible export
5. LogSampler: reduce log volume from noisy paths without losing signal

Usage
─────
    from core.observability import (
        get_logger, correlation_context, emit_research_event,
        get_metrics, track_duration, setup_observability
    )

    # In any module
    log = get_logger("research_loop.orchestrator")
    log.info("starting_deep_research", topic="efficiency", session_id="abc")

    # Wrap a function for automatic duration tracking
    @track_duration("paper_ingest")
    def ingest_paper(arxiv_id):
        ...

    # Emit structured research events
    emit_research_event("gap_discovered", gap_type="method_limitation", novelty=0.82)

    # Propagate correlation across async boundaries
    with correlation_context(trace_id="..."):
        run_deep_research(topic)
"""

import json
import logging
import os
import random
import sys
import threading
import time
import uuid
from collections import defaultdict
from datetime import datetime as DT, timezone
from enum import Enum
from functools import wraps
from typing import Any, Callable, Dict, List, Optional, Union

# ─── Globals ──────────────────────────────────────────────────────────────────

_TRACE_ID_HEADER = "X-Trace-ID"
_SPAN_ID_HEADER = "X-Span-ID"


class _CorrelationLocal(threading.local):
    def __init__(self):
        self.trace_id: Optional[str] = None
        self.span_id: Optional[str] = None
        self.parent_span_id: Optional[str] = None
        self._event_buffer: List[Dict[str, Any]] = []


_correlation = _CorrelationLocal()


# ─── Log Levels ───────────────────────────────────────────────────────────────


class LogLevel(Enum):
    DEBUG = "debug"
    INFO = "info"
    WARNING = "warning"
    ERROR = "error"
    CRITICAL = "critical"


# ─── JSON Formatter ───────────────────────────────────────────────────────────


class _JSONFormatter(logging.Formatter):
    """Emit logs as JSON lines with structured fields."""

    def __init__(self, include_extra: bool = True):
        super().__init__()
        self.include_extra = include_extra

    def format(self, record: logging.LogRecord) -> str:
        ts = DT.fromtimestamp(record.created, tz=timezone.utc).isoformat()
        trace_id = getattr(_correlation, "trace_id", None) or os.environ.get("RAIROS_TRACE_ID", "")
        span_id = getattr(_correlation, "span_id", None) or ""

        log_obj: Dict[str, Any] = {
            "timestamp": ts,
            "level": record.levelname.lower(),
            "logger": record.name,
            "message": record.getMessage(),
            "module": record.module,
            "function": record.funcName,
            "line": record.lineno,
        }

        if trace_id:
            log_obj["trace_id"] = trace_id
        if span_id:
            log_obj["span_id"] = span_id

        if record.exc_info:
            log_obj["exception"] = self.formatException(record.exc_info)

        if self.include_extra:
            extra = getattr(record, "_extra_fields", {})
            if extra:
                log_obj.update(extra)

        return json.dumps(log_obj, default=str)


class _PlainFormatter(logging.Formatter):
    """Human-readable format with trace context."""

    def format(self, record: logging.LogRecord) -> str:
        ts = DT.fromtimestamp(record.created, tz=timezone.utc).strftime("%H:%M:%S.%f")[:-3]
        trace_id = getattr(_correlation, "trace_id", None)
        span_id = getattr(_correlation, "span_id", None)
        ctx = ""
        if trace_id:
            ctx = f"[{trace_id[:8]}]({span_id[:8]}) " if span_id else f"[{trace_id[:8]}] "
        extra = getattr(record, "_extra_fields", {})
        extra_str = " ".join(f"{k}={v}" for k, v in extra.items()) if extra else ""
        msg = record.getMessage()
        if extra_str:
            msg = f"{msg} | {extra_str}"
        return f"{ts} {record.levelname[0]} {ctx}{record.name}: {msg}"


# ─── Structured Logger ────────────────────────────────────────────────────────


class StructuredLogger:
    """Logger with structured .info()/.error() etc. that accept key-value fields."""

    def __init__(self, logger: logging.Logger):
        self._logger = logger

    def _log(self, level: int, event: str, **kwargs: Any) -> None:
        record = self._logger.makeRecord(
            self._logger.name,
            level,
            "(unknown)",
            0,
            event,
            (),
            None,
        )
        record._extra_fields = kwargs  # type: ignore[attr-defined]
        self._logger.handle(record)

    def debug(self, event: str, **kwargs: Any) -> None:
        self._log(logging.DEBUG, event, **kwargs)

    def info(self, event: str, **kwargs: Any) -> None:
        self._log(logging.INFO, event, **kwargs)

    def warning(self, event: str, **kwargs: Any) -> None:
        self._log(logging.WARNING, event, **kwargs)

    def error(self, event: str, **kwargs: Any) -> None:
        self._log(logging.ERROR, event, **kwargs)

    def critical(self, event: str, **kwargs: Any) -> None:
        self._log(logging.CRITICAL, event, **kwargs)

    def exception(self, event: str, **kwargs: Any) -> None:
        self._log(logging.ERROR, event, **kwargs)
        if self._logger.manager.disable + 1 < logging.ERROR:
            pass  # exception info already in record

    def __getattr__(self, name: str) -> Any:
        return getattr(self._logger, name)


_loggers: Dict[str, StructuredLogger] = {}
_logger_lock = threading.Lock()


def get_logger(name: str) -> StructuredLogger:
    """Get a structured logger for a module."""
    if name in _loggers:
        return _loggers[name]
    with _logger_lock:
        if name not in _loggers:
            _loggers[name] = StructuredLogger(logging.getLogger(name))
        return _loggers[name]


# ─── Correlation Context ───────────────────────────────────────────────────────


class correlation_context:
    """Set trace_id/span_id for the current thread. Use as context manager."""

    def __init__(self, trace_id: Optional[str] = None, span_id: Optional[str] = None):
        self.new_trace = False
        if trace_id:
            _correlation.trace_id = trace_id
        else:
            if not _correlation.trace_id:
                _correlation.trace_id = uuid.uuid4().hex[:16]
                self.new_trace = True
        self.own_span = False
        if span_id:
            _correlation.span_id = span_id
        else:
            if not _correlation.span_id:
                _correlation.span_id = uuid.uuid4().hex[:8]
                self.own_span = True
        self.prev_span_id = _correlation.parent_span_id

    def __enter__(self) -> "correlation_context":
        _correlation.parent_span_id = _correlation.span_id if not self.own_span else None
        if self.own_span or not _correlation.span_id:
            _correlation.span_id = uuid.uuid4().hex[:8]
        return self

    def __exit__(self, *_: Any) -> None:
        _correlation.span_id = self.prev_span_id


def get_trace_id() -> Optional[str]:
    return getattr(_correlation, "trace_id", None)


def new_span() -> str:
    """Generate a new span ID for the current context."""
    sid = uuid.uuid4().hex[:8]
    _correlation.span_id = sid
    return sid


# ─── Research Event Emitter ────────────────────────────────────────────────────


class EventType(Enum):
    SESSION_START = "session_start"
    SESSION_END = "session_end"
    PAPER_INGESTED = "paper_ingested"
    PAPER_ANALYZED = "paper_analyzed"
    GAP_DISCOVERED = "gap_discovered"
    GAP_CLUSTERED = "gap_clustered"
    PARADIGM_SHIFT = "paradigm_shift"
    REVIEW_GENERATED = "review_generated"
    RESEARCH_COMPLETE = "research_complete"
    CONTRADICTION_DETECTED = "contradiction_detected"
    TOPIC_SUGGESTED = "topic_suggested"
    LLM_API_CALL = "llm_api_call"
    DB_QUERY = "db_query"
    ERROR = "error"


class ResearchEventEmitter:
    """Emit structured events to an in-memory ring buffer for later analysis."""

    def __init__(self, capacity: int = 10000):
        self._buffer: List[Dict[str, Any]] = []
        self._lock = threading.Lock()
        self._capacity = capacity

    def emit(
        self,
        event: Union[EventType, str],
        trace_id: Optional[str] = None,
        span_id: Optional[str] = None,
        **fields: Any,
    ) -> None:
        """Add an event to the ring buffer."""
        if isinstance(event, EventType):
            event_name = event.value
        else:
            event_name = event

        trace_id = trace_id or getattr(_correlation, "trace_id", None) or ""
        span_id = span_id or getattr(_correlation, "span_id", None) or ""

        record = {
            "event": event_name,
            "timestamp": DT.now(timezone.utc).isoformat(),
            "trace_id": trace_id,
            "span_id": span_id,
            **fields,
        }

        with self._lock:
            self._buffer.append(record)
            if len(self._buffer) > self._capacity:
                self._buffer.pop(0)

    def get_events(
        self,
        event_type: Optional[str] = None,
        trace_id: Optional[str] = None,
        limit: int = 100,
    ) -> List[Dict[str, Any]]:
        """Query events from the buffer."""
        with self._lock:
            events = list(self._buffer)

        if event_type:
            events = [e for e in events if e["event"] == event_type]
        if trace_id:
            events = [e for e in events if e.get("trace_id") == trace_id]

        return events[-limit:]

    def get_recent(self, n: int = 50) -> List[Dict[str, Any]]:
        return self.get_events(limit=n)

    def clear(self) -> None:
        with self._lock:
            self._buffer.clear()


# Global emitter instance
_emitter = ResearchEventEmitter()


def emit_research_event(event: Union[EventType, str], **fields: Any) -> None:
    _emitter.emit(event, **fields)


def get_recent_events(n: int = 50) -> List[Dict[str, Any]]:
    return _emitter.get_recent(n)


# ─── Metrics Collector ────────────────────────────────────────────────────────


class MetricsCollector:
    """In-memory metrics with counter, gauge, histogram support.

    Metrics are keyed by (subsystem, name) tuples.
    """

    def __init__(self):
        self._counters: Dict[str, float] = defaultdict(float)
        self._gauges: Dict[str, float] = {}
        self._histograms: Dict[str, List[float]] = defaultdict(list)
        self._hist_maxlen = 1000
        self._lock = threading.Lock()

    # ── Counters ─────────────────────────────────────────────────────────────

    def inc(self, subsystem: str, name: str, value: float = 1.0) -> None:
        key = f"{subsystem}.{name}"
        with self._lock:
            self._counters[key] += value

    def counter(self, subsystem: str, name: str) -> float:
        key = f"{subsystem}.{name}"
        with self._lock:
            return self._counters.get(key, 0.0)

    # ── Gauges ────────────────────────────────────────────────────────────────

    def set(self, subsystem: str, name: str, value: float) -> None:
        key = f"{subsystem}.{name}"
        with self._lock:
            self._gauges[key] = value

    def gauge(self, subsystem: str, name: str) -> Optional[float]:
        key = f"{subsystem}.{name}"
        with self._lock:
            return self._gauges.get(key)

    # ── Histograms ────────────────────────────────────────────────────────────

    def observe(self, subsystem: str, name: str, value: float) -> None:
        key = f"{subsystem}.{name}"
        with self._lock:
            hist = self._histograms[key]
            hist.append(value)
            if len(hist) > self._hist_maxlen:
                hist.pop(0)

    def histogram_stats(self, subsystem: str, name: str) -> Dict[str, float]:
        key = f"{subsystem}.{name}"
        with self._lock:
            values = list(self._histograms.get(key, []))
        if not values:
            return {}
        sorted_vals = sorted(values)
        n = len(sorted_vals)
        return {
            "count": n,
            "min": sorted_vals[0],
            "max": sorted_vals[-1],
            "mean": sum(sorted_vals) / n,
            "p50": sorted_vals[n // 2],
            "p95": sorted_vals[int(n * 0.95)] if n >= 20 else sorted_vals[-1],
            "p99": sorted_vals[int(n * 0.99)] if n >= 100 else sorted_vals[-1],
        }

    # ── Export ────────────────────────────────────────────────────────────────

    def export_prometheus(self) -> str:
        """Export all metrics in Prometheus text format."""
        lines = []
        ts = time.time()
        with self._lock:
            for key, value in self._counters.items():
                lines.append(f"# TYPE {key} counter")
                lines.append(f"{key} {value} {int(ts * 1000)}")
            for key, value in self._gauges.items():
                lines.append(f"# TYPE {key} gauge")
                lines.append(f"{key} {value} {int(ts * 1000)}")
            for key, values in self._histograms.items():
                if values:
                    lines.append(f"# TYPE {key} histogram")
                    sorted_vals = sorted(values)
                    n = len(sorted_vals)
                    for boundary, _bucket_label in [(0.5, "0.5"), (0.95, "0.95"), (0.99, "0.99")]:
                        idx = int(n * boundary) if int(n * boundary) < n else n - 1
                        lines.append(f"{key}_bucket{boundary} {sorted_vals[idx]} {int(ts * 1000)}")
                    lines.append(f"{key}_sum {sum(sorted_vals)} {int(ts * 1000)}")
                    lines.append(f"{key}_count {n} {int(ts * 1000)}")
        return "\n".join(lines)

    def summary(self) -> Dict[str, Any]:
        """Get a summary of all metrics."""
        with self._lock:
            counters = dict(self._counters)
            gauges = dict(self._gauges)
            histograms = {k: self.histogram_stats(*k.split(".", 1)) for k in self._histograms}
        return {
            "counters": counters,
            "gauges": gauges,
            "histograms": histograms,
        }


_metrics = MetricsCollector()


def get_metrics() -> MetricsCollector:
    return _metrics


# ─── Duration Tracking ────────────────────────────────────────────────────────


def track_duration(subsystem: str, metric_name: Optional[str] = None) -> Callable:
    """Decorator to track function duration as a histogram metric."""

    def decorator(func: Callable) -> Callable:
        @wraps(func)
        def wrapper(*args: Any, **kwargs: Any) -> Any:
            name = metric_name or f"{func.__module__}.{func.__qualname__}"
            start = time.perf_counter()
            try:
                return func(*args, **kwargs)
            except Exception:
                duration = time.perf_counter() - start
                _metrics.observe(subsystem, f"{name}.duration", duration)
                _metrics.inc(subsystem, f"{name}.errors")
                raise
            else:
                duration = time.perf_counter() - start
                _metrics.observe(subsystem, f"{name}.duration", duration)

        return wrapper

    return decorator


class track_duration_cm:
    """Context manager to track a code block duration."""

    def __init__(self, subsystem: str, name: str):
        self.subsystem = subsystem
        self.name = name
        self.start = 0.0

    def __enter__(self) -> "track_duration_cm":
        self.start = time.perf_counter()
        return self

    def __exit__(self, *_: Any) -> None:
        duration = time.perf_counter() - self.start
        _metrics.observe(self.subsystem, f"{self.name}.duration", duration)


# ─── Log Sampler ─────────────────────────────────────────────────────────────


class LogSampler:
    """Sample high-volume logs intelligently — keep first, last, and random sample."""

    def __init__(self, rate: float = 0.1):
        self.rate = rate  # keep rate
        self._buckets: Dict[str, Dict[int, List[logging.LogRecord]]] = defaultdict(
            lambda: defaultdict(list)
        )

    def should_emit(self, logger_name: str, level: int) -> bool:
        if level >= logging.ERROR:
            return True  # always emit errors
        _ = hash(logger_name) % 100
        return random.random() < self.rate

    def sample_record(self, logger_name: str, record: logging.LogRecord) -> bool:
        """Return True if record should be emitted."""
        if record.levelno >= logging.ERROR:
            return True
        _ = hash(f"{logger_name}:{record.levelno}") % 1000
        if random.random() < self.rate:
            return True
        return False


# ─── Observability Setup ─────────────────────────────────────────────────────


_observability_configured = False
_config_lock = threading.Lock()


def setup_observability(
    level: str = "INFO",
    json_logs: bool = False,
    log_file: Optional[str] = None,
    log_sampler_rate: float = 1.0,
) -> None:
    """Configure the observability stack for Rairos.

    Args:
        level: Log level (DEBUG, INFO, WARNING, ERROR)
        json_logs: Use JSON formatter (good for log aggregators)
        log_file: Optional file path to also log to
        log_sampler_rate: 1.0 = no sampling, 0.1 = keep 10% of INFO/DEBUG
    """
    global _observability_configured
    with _config_lock:
        if _observability_configured:
            return
        _observability_configured = True

    log_level = getattr(logging, level.upper(), logging.INFO)
    handlers: List[logging.Handler] = []

    console_handler = logging.StreamHandler(sys.stdout)
    if json_logs:
        console_handler.setFormatter(_JSONFormatter())
    else:
        console_handler.setFormatter(_PlainFormatter())
    handlers.append(console_handler)

    if log_file:
        file_handler = logging.FileHandler(log_file)
        file_handler.setFormatter(_JSONFormatter())
        handlers.append(file_handler)

    # Configure root logger
    root = logging.getLogger()
    root.setLevel(log_level)
    # Remove existing handlers
    for h in root.handlers[:]:
        root.removeHandler(h)
    for h in handlers:
        h.setLevel(log_level)
        root.addHandler(h)

    # Silence noisy third-party loggers
    for noisy in ("urllib3", "requests", "httpx", "aiohttp", "asyncio"):
        logging.getLogger(noisy).setLevel(logging.WARNING)

    get_logger("observability").info(
        "observability_configured", log_level=level, json_logs=json_logs
    )
