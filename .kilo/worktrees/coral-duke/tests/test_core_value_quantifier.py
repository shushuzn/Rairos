"""Tests for core/value_quantifier.py."""

import pytest


class TestValueMetric:
    """Tests for ValueMetric dataclass."""

    def test_dataclass_fields(self):
        """ValueMetric stores name, value, unit, and description."""
        from core.value_quantifier import ValueMetric
        metric = ValueMetric(
            name="API Calls Saved",
            value=100.0,
            unit="次",
            description="Through caching",
        )
        assert metric.name == "API Calls Saved"
        assert metric.value == 100.0
        assert metric.unit == "次"
        assert metric.description == "Through caching"


class TestValueQuantifier:
    """Tests for ValueQuantifier class."""

    def test_init_metrics_are_zeroed(self):
        """All metrics start at zero."""
        from core.value_quantifier import ValueQuantifier
        q = ValueQuantifier()
        assert q.metrics["api_calls_saved"] == 0
        assert q.metrics["papers_processed"] == 0
        assert q.metrics["searches_performed"] == 0
        assert q.metrics["hours_saved"] == 0.0
        assert q.metrics["cost_saved_usd"] == 0.0
        assert q.metrics["efficiency_gain_percent"] == 0.0

    def test_update_changes_metric(self):
        """update() changes the specified metric."""
        from core.value_quantifier import ValueQuantifier
        q = ValueQuantifier()
        q.update("api_calls_saved", 50.0)
        assert q.metrics["api_calls_saved"] == 50.0

    def test_update_ignores_unknown_metric(self):
        """update() silently ignores unknown metric keys."""
        from core.value_quantifier import ValueQuantifier
        q = ValueQuantifier()
        q.update("unknown_metric", 999.0)
        assert "unknown_metric" not in q.metrics

    def test_calculate_value_returns_all_metrics(self):
        """calculate_value returns 4 ValueMetric entries."""
        from core.value_quantifier import ValueQuantifier
        q = ValueQuantifier()
        values = q.calculate_value()
        assert len(values) == 4
        assert "api_calls_saved" in values
        assert "hours_saved" in values
        assert "cost_saved" in values
        assert "papers_processed" in values

    def test_calculate_value_derived_from_api_calls(self):
        """hours_saved = api_calls_saved * 0.1."""
        from core.value_quantifier import ValueQuantifier
        q = ValueQuantifier()
        q.update("api_calls_saved", 100)
        values = q.calculate_value()
        assert values["hours_saved"].value == 10.0

    def test_calculate_value_cost_includes_time_value(self):
        """cost_saved = api_calls * 0.01 + hours_saved * 50."""
        from core.value_quantifier import ValueQuantifier
        q = ValueQuantifier()
        q.update("api_calls_saved", 100)
        values = q.calculate_value()
        # api_cost = 100 * 0.01 = 1.0
        # research_time = 10.0 * 50 = 500.0
        # total = 501.0
        assert values["cost_saved"].value == pytest.approx(501.0)

    def test_get_value_report_contains_vw_reference(self):
        """Report references Volkswagen's 700-900 euros."""
        from core.value_quantifier import ValueQuantifier
        q = ValueQuantifier()
        q.update("api_calls_saved", 100)
        report = q.get_value_report()
        assert "Volkswagen" in report

    def test_get_value_report_contains_value_sections(self):
        """Report contains expected sections."""
        from core.value_quantifier import ValueQuantifier
        q = ValueQuantifier()
        q.update("api_calls_saved", 100)
        report = q.get_value_report()
        assert "价值量化报告" in report
        assert "API调用节省" in report
        assert "总价值" in report

    def test_get_value_report_empty_when_no_metrics(self):
        """Report shows no value when all metrics are zero."""
        from core.value_quantifier import ValueQuantifier
        q = ValueQuantifier()
        report = q.get_value_report()
        assert "总价值" not in report

    def test_get_vw_comparison_contains_vw_text(self):
        """VW comparison string references Volkswagen."""
        from core.value_quantifier import ValueQuantifier
        q = ValueQuantifier()
        q.update("api_calls_saved", 100)
        comparison = q.get_vw_comparison()
        assert "Volkswagen" in comparison

    def test_get_vw_comparison_annualizes(self):
        """VW comparison annualizes current savings."""
        from core.value_quantifier import ValueQuantifier
        q = ValueQuantifier()
        q.update("api_calls_saved", 100)
        comparison = q.get_vw_comparison()
        # 100 api calls * 0.1 hours * $50/hour = $500 + $1 (cost) = $501 * 12 ≈ $6012
        # actual: (100 * 0.1 * 50 + 100 * 0.01) * 12 = 501 * 12 = 6012
        assert "年化" in comparison

    def test_get_vw_comparison_mentions_ai_research_os(self):
        """Comparison mentions AI Research OS."""
        from core.value_quantifier import ValueQuantifier
        q = ValueQuantifier()
        comparison = q.get_vw_comparison()
        assert "AI Research OS" in comparison


class TestGlobalValueQuantifier:
    """Tests for global singleton."""

    def test_get_value_quantifier_returns_instance(self):
        """get_value_quantifier returns a ValueQuantifier instance."""
        from core.value_quantifier import get_value_quantifier
        q = get_value_quantifier()
        assert q is not None

    def test_get_value_quantifier_singleton(self):
        """get_value_quantifier returns the same instance on repeated calls."""
        from core.value_quantifier import get_value_quantifier
        q1 = get_value_quantifier()
        q2 = get_value_quantifier()
        assert q1 is q2
