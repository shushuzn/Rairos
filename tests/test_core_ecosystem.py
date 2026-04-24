"""Tests for core/ecosystem.py."""

import pytest


class TestEcosystemComponent:
    """Tests for EcosystemComponent dataclass."""

    def test_dataclass_fields(self):
        """Component stores all fields."""
        from core.ecosystem import EcosystemComponent
        comp = EcosystemComponent(
            name="Test",
            description="A test component",
            icon="🧪",
            status="ready",
            url="http://test",
        )
        assert comp.name == "Test"
        assert comp.description == "A test component"
        assert comp.icon == "🧪"
        assert comp.status == "ready"
        assert comp.url == "http://test"


class TestEcosystem:
    """Tests for Ecosystem class."""

    def test_init_registers_all_components(self):
        """Ecosystem initializes all 10 components."""
        from core.ecosystem import Ecosystem
        eco = Ecosystem()
        assert len(eco.components) == 10
        assert "cli" in eco.components
        assert "simple_cli" in eco.components
        assert "api" in eco.components
        assert "achievements" in eco.components
        assert "performance" in eco.components
        assert "value" in eco.components
        assert "setup_wizard" in eco.components
        assert "gui" in eco.components
        assert "plugins" in eco.components
        assert "marketplace" in eco.components

    def test_init_sets_correct_status_values(self):
        """Each component has a valid status string."""
        from core.ecosystem import Ecosystem
        eco = Ecosystem()
        valid_statuses = {"ready", "planned", "coming_soon"}
        for comp in eco.components.values():
            assert comp.status in valid_statuses

    def test_ready_components_have_url(self):
        """All 'ready' components have a non-empty URL."""
        from core.ecosystem import Ecosystem
        eco = Ecosystem()
        for comp in eco.components.values():
            if comp.status == "ready":
                assert comp.url is not None

    def test_get_ecosystem_report_contains_vw_reference(self):
        """Report references Volkswagen."""
        from core.ecosystem import Ecosystem
        eco = Ecosystem()
        report = eco.get_ecosystem_report()
        assert "Volkswagen" in report

    def test_get_ecosystem_report_contains_ready_section(self):
        """Report contains the ready components section."""
        from core.ecosystem import Ecosystem
        eco = Ecosystem()
        report = eco.get_ecosystem_report()
        assert "已就绪" in report

    def test_get_ecosystem_report_contains_planned_section(self):
        """Report contains the planned components section."""
        from core.ecosystem import Ecosystem
        eco = Ecosystem()
        report = eco.get_ecosystem_report()
        assert "规划中" in report

    def test_get_ecosystem_report_contains_coming_soon_section(self):
        """Report contains the coming soon section."""
        from core.ecosystem import Ecosystem
        eco = Ecosystem()
        report = eco.get_ecosystem_report()
        assert "即将推出" in report

    def test_get_ecosystem_report_lists_all_ready_components(self):
        """Each ready component appears in the report."""
        from core.ecosystem import Ecosystem
        eco = Ecosystem()
        report = eco.get_ecosystem_report()
        ready = [c.name for c in eco.components.values() if c.status == "ready"]
        for comp in ready:
            assert comp in report

    def test_get_ecosystem_report_includes_cli_component(self):
        """CLI component is listed in report."""
        from core.ecosystem import Ecosystem
        eco = Ecosystem()
        report = eco.get_ecosystem_report()
        assert "命令行工具" in report


class TestGlobalEcosystem:
    """Tests for global ecosystem singleton."""

    def test_get_ecosystem_returns_instance(self):
        """get_ecosystem returns an Ecosystem instance."""
        from core.ecosystem import get_ecosystem
        eco = get_ecosystem()
        assert eco is not None

    def test_get_ecosystem_singleton(self):
        """get_ecosystem returns the same instance on repeated calls."""
        from core.ecosystem import get_ecosystem
        eco1 = get_ecosystem()
        eco2 = get_ecosystem()
        assert eco1 is eco2
