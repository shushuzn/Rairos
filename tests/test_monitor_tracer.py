"""Tests for subscription_monitor, climate_ai_monitor, policy_impact_tracer."""
import pytest
from llm.subscription_monitor import SubscriptionMonitor, search_arxiv
from llm.climate_ai_monitor import is_climate_related, get_climate_papers, get_watch_stats, render_climate_monitor_html
from llm.policy_impact_tracer import check_policy_impact, get_impacted_capsules, render_policy_tracer_html


class TestSubscriptionMonitor:
    def test_init(self):
        m = SubscriptionMonitor(db=None, scorer=None)
        assert m is not None


class TestSearchArxiv:
    def test_signature(self):
        import inspect
        sig = inspect.signature(search_arxiv)
        assert "query" in sig.parameters


class TestClimateAiMonitor:
    def test_is_climate_related(self):
        paper = {"title": "Climate Change and AI", "abstract": "We study carbon emissions"}
        result = is_climate_related(paper)
        assert isinstance(result, bool)

    def test_get_climate_papers(self):
        result = get_climate_papers()
        assert isinstance(result, list)

    def test_get_watch_stats(self):
        result = get_watch_stats()
        assert isinstance(result, dict)

    def test_render_html(self):
        result = render_climate_monitor_html(stats={})
        assert isinstance(result, str)
        assert "<" in result


class TestPolicyImpactTracer:
    def test_check_policy_impact(self):
        paper = {"title": "AI Regulation Policy", "abstract": "Government AI rules"}
        result = check_policy_impact(paper)
        assert isinstance(result, list)

    def test_get_impacted_capsules(self):
        result = get_impacted_capsules()
        assert isinstance(result, list)

    def test_render_tracer_html(self):
        result = render_policy_tracer_html()
        assert isinstance(result, str)
        assert "<" in result
