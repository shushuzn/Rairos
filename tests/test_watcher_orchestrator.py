"""Tests for gene_pool_watcher and orchestrator dataclasses."""
import pytest
from llm.gene_pool_watcher import (
    GapSubscription, WatcherState, _build_gap_subscriptions, _now_iso,
    FAMILY_ARXIV_CONFIG,
)
from research_loop.orchestrator import ResearchAlert, OrchestratorConfig


# ── gene_pool_watcher ─────────────────────────────────────────────────────────

class TestGapSubscription:
    def test_defaults(self):
        sub = GapSubscription("attention", ["transformer"], "cs.CL")
        assert sub.family == "attention"
        assert sub.keywords == ["transformer"]
        assert sub.arxiv_category == "cs.CL"
        assert sub.enabled is True
        assert sub.last_checked == ""

    def test_disabled(self):
        sub = GapSubscription("rl", ["ppo"], "cs.LG", enabled=False)
        assert sub.enabled is False


class TestWatcherState:
    def test_defaults(self):
        state = WatcherState()
        assert state.gap_subscriptions == []
        assert state.last_diversity_check == ""
        assert state.underrepresented_families == []

    def test_custom(self):
        state = WatcherState(
            gap_subscriptions={"s1": None},
            diversity_score=0.75,
        )
        assert state.diversity_score == 0.75


class TestBuildGapSubscriptions:
    def test_known_family(self):
        subs = _build_gap_subscriptions(["attention"])
        assert len(subs) == 1
        assert subs[0].family == "attention"

    def test_unknown_gets_default(self):
        """Unknown families get a generic subscription with ML keywords."""
        subs = _build_gap_subscriptions(["nonexistent_family"])
        assert len(subs) == 1
        assert subs[0].arxiv_category == "cs.LG"

    def test_multiple(self):
        subs = _build_gap_subscriptions(["attention", "reinforcement"])
        assert len(subs) == 2


class TestNowIso:
    def test_returns_iso_string(self):
        ts = _now_iso()
        assert isinstance(ts, str)
        assert "T" in ts


# ── orchestrator ──────────────────────────────────────────────────────────────

class TestResearchAlert:
    def test_construct(self):
        alert = ResearchAlert(
            alert_id="a1", session_id="s1", topic="RL", triggered_by="gap",
            trigger_title="Test Gap", gaps_found=3, top_gap_title="RL Gap",
            top_gap_type="method", severity="high", gene_pool_score=0.8,
            preference_boost=0.5, created_at="2026-01-01",
        )
        assert alert.alert_id == "a1"
        assert alert.severity == "high"
        assert alert.gaps_found == 3
        assert alert.gene_pool_score == 0.8

    def test_to_dict(self):
        alert = ResearchAlert(
            alert_id="a1", session_id="s1", topic="RL", triggered_by="gap",
            trigger_title="T", gaps_found=1, top_gap_title="G", top_gap_type="m",
            severity="low", gene_pool_score=0.5, preference_boost=0.0, created_at="now",
        )
        d = alert.to_dict()
        assert d["alert_id"] == "a1"
        assert d["severity"] == "low"


class TestOrchestratorConfig:
    def test_defaults(self):
        config = OrchestratorConfig()
        assert config.interval_minutes == 30
        assert config.min_gap_severity_for_alert == "MEDIUM"
        assert config.max_alerts_stored > 0

    def test_custom(self):
        config = OrchestratorConfig(interval_minutes=30, max_alerts_stored=50)
        assert config.interval_minutes == 30
        assert config.max_alerts_stored == 50


# ── FAMILY_ARXIV_CONFIG ──────────────────────────────────────────────────────

class TestFamilyArxivConfig:
    def test_known_families(self):
        assert "attention" in FAMILY_ARXIV_CONFIG
        assert "reinforcement" in FAMILY_ARXIV_CONFIG

    def test_each_has_keywords(self):
        for family, config in FAMILY_ARXIV_CONFIG.items():
            assert "keywords" in config
            assert len(config["keywords"]) > 0
