"""Tests for core/achievements.py Achievement dataclass."""
import pytest
from datetime import datetime


class TestAchievement:
    """Test Achievement dataclass from core/achievements.py."""

    def _achievement(self):
        from core.achievements import Achievement

        return Achievement

    def test_required_fields(self):
        ACH = self._achievement()
        ach = ACH(id="paper-hunter", name="Paper Hunter", description="Import 10 papers", icon="📚", points=100)
        assert ach.id == "paper-hunter"
        assert ach.name == "Paper Hunter"
        assert ach.description == "Import 10 papers"
        assert ach.icon == "📚"
        assert ach.points == 100

    def test_optional_unlocked_at(self):
        ACH = self._achievement()
        now = datetime(2026, 5, 12, 10, 0, 0)
        ach = ACH(id="early-bird", name="Early Bird", description="First paper", icon="🐦", points=50, unlocked_at=now)
        assert ach.unlocked_at == now

    def test_unlocked_at_default_none(self):
        ACH = self._achievement()
        ach = ACH(id="test", name="Test", description="Test", icon="🧪", points=0)
        assert ach.unlocked_at is None

    def test_points_non_negative(self):
        ACH = self._achievement()
        ach = ACH(id="free", name="Free", description="Free badge", icon="🎁", points=0)
        assert ach.points == 0

    def test_achievement_str_representation(self):
        ACH = self._achievement()
        ach = ACH(id="speed-demon", name="Speed Demon", description="Fast parser", icon="⚡", points=200)
        s = str(ach)
        assert "speed-demon" in s
        assert "Speed Demon" in s
        assert "200" in s
