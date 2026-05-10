"""Tests for research_loop/orchestrator methods."""

from research_loop.orchestrator import (
    ResearchAlert,
    OrchestratorConfig,
    AutonomousOrchestrator,
    _get_state_path,
    _load_state,
    _save_state,
)


class TestResearchAlert:
    def test_to_dict(self):
        alert = ResearchAlert(
            alert_id="a1",
            session_id="s1",
            topic="ML",
            triggered_by="2301.00001",
            trigger_title="Test Paper",
            gaps_found=3,
            top_gap_title="Gap Title",
            top_gap_type="methodology",
            severity="HIGH",
            gene_pool_score=0.8,
            preference_boost=True,
            created_at=1700000000.0,
        )
        d = alert.to_dict()
        assert d["alert_id"] == "a1"
        assert d["severity"] == "HIGH"
        assert d["top_gap_title"] == "Gap Title"

    def test_from_dict(self):
        d = {
            "session_id": "s1",
            "topic": "ML",
            "triggered_by": "2301.00001",
            "trigger_title": "Paper",
            "gaps_found": 2,
            "top_gap_title": "Gap",
            "top_gap_type": "methodology",
            "severity": "MEDIUM",
            "gene_pool_score": 0.5,
            "preference_boost": False,
            "created_at": 1700000000.0,
        }
        alert = ResearchAlert.from_dict(d)
        assert alert.alert_id is not None
        assert alert.severity == "MEDIUM"


class TestOrchestratorConfig:
    def test_default(self):
        c = OrchestratorConfig()
        assert c.min_gap_severity_for_alert == "MEDIUM"


class TestOrchestratorState:
    def test_get_state_path(self):
        p = _get_state_path()
        assert p.name == "orchestrator_state.json"

    def test_load_state_default(self):
        state = _load_state()
        assert isinstance(state, dict)

    def test_save_and_load(self, tmp_path, monkeypatch):
        # Mock _get_state_path to use tmp_path
        mock_path = tmp_path / "state.json"
        monkeypatch.setattr("research_loop.orchestrator._get_state_path", lambda: mock_path)
        _save_state({"running": True, "interval_minutes": 10})
        state = _load_state()
        assert state["running"] is True
        assert state["interval_minutes"] == 10


class TestAutonomousOrchestrator:
    def test_init_default(self):
        o = AutonomousOrchestrator()
        assert o.config is not None

    def test_get_status(self):
        o = AutonomousOrchestrator()
        status = o.get_status()
        assert isinstance(status, dict)
        assert "running" in status

    def test_get_recent_alerts(self):
        o = AutonomousOrchestrator()
        alerts = o.get_recent_alerts(limit=5)
        assert isinstance(alerts, list)
