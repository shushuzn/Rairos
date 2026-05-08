"""Tests for research_loop agents and benchmark_runner."""

import pytest
from enum import Enum
from research_loop.agents.base import AgentStatus, MessageBus, AgentMessage, BaseAgent
from research_loop.agents.squad import SquadCoordinator, _state_path, _load_state, _save_state
from research_loop.benchmark_runner import BenchmarkResult, BenchmarkConfig, run_benchmark


class TestAgentStatus:
    def test_is_enum(self):
        assert issubclass(AgentStatus, Enum)


class TestMessageBus:
    def test_init(self):
        bus = MessageBus()
        assert bus is not None


class TestAgentMessage:
    def test_fields(self):
        msg = AgentMessage(
            id="m1",
            topic="test",
            sender="alice",
            payload={"key": "value"},
            timestamp=None,
            reply_to=None,
        )
        assert msg.id == "m1"
        assert msg.topic == "test"


class TestSquadCoordinator:
    def test_init(self):
        sc = SquadCoordinator()
        assert sc is not None


class TestSquadState:
    def test_state_path(self):
        p = _state_path()
        assert p is not None

    def test_load_state(self):
        state = _load_state()
        assert isinstance(state, dict)

    def test_save_and_load(self, tmp_path):
        _save_state(running=True, interval=5)
        state = _load_state()
        assert "running" in state
        assert "interval_minutes" in state


class TestBenchmarkRunner:
    def test_benchmark_result_init(self):
        r = BenchmarkResult(
            arxiv_id="test",
            test_dir="tests/",
            passed=10,
            failed=0,
            skipped=0,
            duration_seconds=1.5,
            passed_tests=[],
            failed_tests=[],
            error_message=None,
            gene_pool_entry=None,
            ruff_diagnostics=[],
            numerical_claims_total=0,
            numerical_claims_covered=0,
            coverage_ratio=0.8,
            covered_claims=[],
            uncovered_claims=[],
        )
        assert r.passed == 10
        assert r.coverage_ratio == 0.8

    def test_benchmark_config_init(self):
        c = BenchmarkConfig(
            arxiv_id="test",
            paper_topic="ML",
            algorithm_description=" algo",
            test_dir="tests/",
            code_path="src/",
            code_quality=0.8,
            min_pass_rate=0.8,
            algorithm_fingerprint="abc",
            generated_code="def test(): pass",
            paper_section_refs={},
            min_coverage_ratio=0.5,
            numerical_claims_total=5,
        )
        assert c.paper_topic == "ML"

    def test_run_benchmark_signature(self):
        import inspect

        sig = inspect.signature(run_benchmark)
        assert "config" in sig.parameters
