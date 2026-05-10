"""Test LLM cost tracking via MetricsCollector."""

from __future__ import annotations


from core.observability import get_metrics
from llm.generate import estimate_cost, get_model_price


class TestCostTracking:
    """Verify cost data flows into MetricsCollector correctly."""

    def setup_method(self):
        # Reset metrics before each test
        self._collector = get_metrics()
        self._collector._counters.clear()
        self._collector._gauges.clear()
        self._collector._histograms.clear()

    def test_get_model_price_returns_tuple(self):
        inp, out = get_model_price("gpt-4o-mini")
        assert isinstance(inp, float)
        assert isinstance(out, float)
        assert inp >= 0
        assert out >= 0

    def test_estimate_cost_returns_required_keys(self):
        result = estimate_cost("gpt-4o-mini", "hello world", "response text")
        assert "input_tokens" in result
        assert "output_tokens" in result
        assert "total_tokens" in result
        assert "input_cost_usd" in result
        assert "output_cost_usd" in result
        assert "total_cost_usd" in result

    def test_estimate_cost_nonnegative(self):
        result = estimate_cost("gpt-4o-mini", "a" * 1000, "b" * 500)
        assert result["input_cost_usd"] >= 0
        assert result["output_cost_usd"] >= 0
        assert result["total_cost_usd"] >= 0
        assert result["total_tokens"] >= 0

    def test_estimate_cost_proportional_to_input_length(self):
        short = estimate_cost("gpt-4o-mini", "hi", "there")
        long = estimate_cost("gpt-4o-mini", "hi" * 100, "there")
        assert long["input_cost_usd"] > short["input_cost_usd"]

    def test_metrics_collects_cost_increment(self):
        # Simulate a cost tracking call similar to research_loop/core.py
        cost_info = estimate_cost("gpt-4o-mini", "test input", "test output")
        metrics = get_metrics()
        metrics.inc("llm", "calls")
        metrics.inc("llm", "total_cost_usd", cost_info["total_cost_usd"])
        metrics.inc("llm", "total_tokens", cost_info["total_tokens"])

        assert metrics.counter("llm", "calls") == 1.0
        assert metrics.counter("llm", "total_cost_usd") == cost_info["total_cost_usd"]
        assert metrics.counter("llm", "total_tokens") == cost_info["total_tokens"]

    def test_metrics_accumulates_multiple_calls(self):
        metrics = get_metrics()
        # Simulate 3 LLM calls
        for _ in range(3):
            cost_info = estimate_cost("gpt-4o-mini", "input", "output")
            metrics.inc("llm", "calls")
            metrics.inc("llm", "total_cost_usd", cost_info["total_cost_usd"])
            metrics.inc("llm", "total_tokens", cost_info["total_tokens"])

        assert metrics.counter("llm", "calls") == 3.0
        assert metrics.counter("llm", "total_cost_usd") > 0
        assert metrics.counter("llm", "total_tokens") > 0

    def test_cost_tracking_with_expensive_model(self):
        """Verify cost accumulation works with a more expensive model."""
        cost_info = estimate_cost("gpt-4o", "long input text " * 50, "long output text " * 50)
        metrics = get_metrics()
        metrics.inc("llm", "calls")
        metrics.inc("llm", "total_cost_usd", cost_info["total_cost_usd"])
        metrics.inc("llm", "total_tokens", cost_info["total_tokens"])

        assert cost_info["total_cost_usd"] > 0
        assert cost_info["total_tokens"] > 0
        assert metrics.counter("llm", "total_cost_usd") == cost_info["total_cost_usd"]

    def test_cost_tracking_with_free_model(self):
        """Ollama/local models should have zero cost."""
        cost_info = estimate_cost("ollama/llama3.2", "input", "output")
        metrics = get_metrics()
        metrics.inc("llm", "calls")
        metrics.inc("llm", "total_cost_usd", cost_info["total_cost_usd"])

        assert cost_info["total_cost_usd"] == 0.0
        assert cost_info["input_cost_usd"] == 0.0
        assert cost_info["output_cost_usd"] == 0.0
        assert metrics.counter("llm", "total_cost_usd") == 0.0
