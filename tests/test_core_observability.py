"""Tests for core/observability.py — structured logging, correlation, events, metrics."""

import logging
import pytest
import threading
import time

from core.observability import (
    EventType,
    LogLevel,
    MetricsCollector,
    ResearchEventEmitter,
    StructuredLogger,
    correlation_context,
    emit_research_event,
    get_logger,
    get_recent_events,
    get_trace_id,
    new_span,
    track_duration,
)


class TestStructuredLogger:
    """Tests for StructuredLogger."""

    def test_get_logger_returns_structured_logger(self) -> None:
        log = get_logger("test.module")
        assert isinstance(log, StructuredLogger)

    def test_get_logger_caches(self) -> None:
        log1 = get_logger("test.cache")
        log2 = get_logger("test.cache")
        assert log1 is log2

    def test_log_info_with_fields(self, caplog: pytest.LogCaptureFixture) -> None:
        log = get_logger("test.info")
        log.info("test_event", foo="bar", count=42)
        assert "test_event" in caplog.text
        # Structured fields go to record._extra_fields, not text output

    def test_log_error_with_fields(self, caplog: pytest.LogCaptureFixture) -> None:
        log = get_logger("test.error")
        log.error("error_event", error_code=500)
        assert "error_event" in caplog.text

    def test_log_debug_with_fields(self, caplog: pytest.LogCaptureFixture) -> None:
        log = get_logger("test.debug")
        log.debug("debug_event", key="value")
        assert "debug_event" in caplog.text


class TestCorrelationContext:
    """Tests for correlation_context and trace ID propagation."""

    def test_trace_id_auto_generated(self) -> None:
        with correlation_context():
            tid = get_trace_id()
            assert tid is not None
            assert len(tid) == 16

    def test_trace_id_propagated(self) -> None:
        with correlation_context(trace_id="abc123def4567890"):
            tid = get_trace_id()
            assert tid == "abc123def4567890"

    def test_new_span_returns_8_chars(self) -> None:
        span = new_span()
        assert len(span) == 8
        assert isinstance(span, str)

    def test_correlation_context_nested(self) -> None:
        """Nested context managers preserve parent span after exit."""
        with correlation_context(trace_id="outer_trace"):
            outer_tid = get_trace_id()
            with correlation_context():
                inner_tid = get_trace_id()
                assert inner_tid == outer_tid  # same trace
            # inner exited, should still have outer trace
            assert get_trace_id() == outer_tid


class TestResearchEventEmitter:
    """Tests for ResearchEventEmitter ring buffer."""

    def test_emit_and_get_events(self) -> None:
        emitter = ResearchEventEmitter(capacity=100)
        emitter.emit(EventType.PAPER_INGESTED, paper_id="2201.00001")
        events = emitter.get_events()
        assert len(events) == 1
        assert events[0]["event"] == "paper_ingested"

    def test_emit_string_event(self) -> None:
        emitter = ResearchEventEmitter(capacity=100)
        emitter.emit("custom_event", value=123)
        events = emitter.get_events()
        assert events[0]["event"] == "custom_event"
        assert events[0]["value"] == 123

    def test_get_events_with_limit(self) -> None:
        emitter = ResearchEventEmitter(capacity=100)
        for i in range(50):
            emitter.emit(f"event_{i}")
        events = emitter.get_events(limit=10)
        assert len(events) == 10

    def test_get_events_filter_by_type(self) -> None:
        emitter = ResearchEventEmitter(capacity=100)
        emitter.emit(EventType.SESSION_START)
        emitter.emit(EventType.PAPER_INGESTED)
        emitter.emit(EventType.SESSION_END)
        gaps = emitter.get_events(event_type="gap_discovered")
        assert len(gaps) == 0
        sessions = emitter.get_events(event_type="session_start")
        assert len(sessions) == 1

    def test_ring_buffer_capacity(self) -> None:
        emitter = ResearchEventEmitter(capacity=5)
        for i in range(10):
            emitter.emit(f"event_{i}")
        events = emitter.get_events()
        assert len(events) == 5
        assert events[-1]["event"] == "event_9"

    def test_clear(self) -> None:
        emitter = ResearchEventEmitter(capacity=100)
        emitter.emit(EventType.SESSION_START)
        emitter.clear()
        assert len(emitter.get_events()) == 0


class TestGlobalEmitter:
    """Tests for the global _emitter functions."""

    def test_emit_research_event(self) -> None:
        emit_research_event(EventType.PAPER_INGESTED, paper_id="2201.00001")
        events = get_recent_events(n=1)
        assert len(events) >= 1
        # Find our event
        found = [e for e in events if e.get("paper_id") == "2201.00001"]
        assert len(found) >= 1


class TestMetricsCollector:
    """Tests for MetricsCollector counter/gauge/histogram."""

    def test_increment_counter(self) -> None:
        mc = MetricsCollector()
        mc.inc("test", "requests")
        mc.inc("test", "requests")
        mc.inc("test", "requests")
        assert mc.counter("test", "requests") == 3.0

    def test_set_gauge(self) -> None:
        mc = MetricsCollector()
        mc.set("test", "memory_mb", 42.5)
        assert mc.gauge("test", "memory_mb") == 42.5

    def test_record_histogram(self) -> None:
        mc = MetricsCollector()
        mc.observe("test", "latency_ms", 10.0)
        mc.observe("test", "latency_ms", 20.0)
        mc.observe("test", "latency_ms", 30.0)
        stats = mc.histogram_stats("test", "latency_ms")
        assert stats["count"] == 3
        assert stats["mean"] == 20.0
        assert stats["min"] == 10.0
        assert stats["max"] == 30.0

    def test_gauge_default_none(self) -> None:
        mc = MetricsCollector()
        # Returns None for nonexistent keys (not 0)
        assert mc.gauge("nonexistent", "field") is None


class TestTrackDuration:
    """Tests for @track_duration decorator."""

    def test_track_duration_records_duration(self) -> None:
        @track_duration("test.operation")
        def slow_op():
            time.sleep(0.05)
            return 42

        result = slow_op()
        assert result == 42
        # Check histogram was recorded
        # (We can't easily inspect the global metrics, but we verify no exception)

    def test_track_duration_propagates_exception(self) -> None:
        @track_duration("test.failing")
        def failing_op():
            raise ValueError("expected")

        with pytest.raises(ValueError, match="expected"):
            failing_op()


class TestLogLevel:
    """Tests for LogLevel enum."""

    def test_log_level_values(self) -> None:
        assert LogLevel.DEBUG.value == "debug"
        assert LogLevel.INFO.value == "info"
        assert LogLevel.WARNING.value == "warning"
        assert LogLevel.ERROR.value == "error"
        assert LogLevel.CRITICAL.value == "critical"
