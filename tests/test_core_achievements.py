"""Tests for core/achievements.py."""

import pytest
from datetime import datetime


class TestAchievementSystem:
    """Tests for AchievementSystem."""

    def test_init(self):
        """System initializes with zero points and empty unlocked list."""
        from core.achievements import AchievementSystem
        system = AchievementSystem()
        assert system.total_points == 0
        assert system.user_stats["papers_processed"] == 0
        assert system.user_stats["api_calls_saved"] == 0
        assert system.user_stats["hours_saved"] == 0.0

    def test_init_achievements_defines_all_badges(self):
        """All 8 achievements are registered on init."""
        from core.achievements import AchievementSystem
        system = AchievementSystem()
        assert len(system.achievements) == 8
        assert "first_import" in system.achievements
        assert "paper_collector" in system.achievements
        assert "researcher_100" in system.achievements
        assert "api_saver" in system.achievements
        assert "time_saver" in system.achievements
        assert "speed_demon" in system.achievements
        assert "cache_master" in system.achievements
        assert "search_expert" in system.achievements

    def test_unlock_achievement_returns_achievement(self):
        """unlock_achievement returns the achievement when newly unlocked."""
        from core.achievements import AchievementSystem
        system = AchievementSystem()
        result = system.unlock_achievement("first_import")
        assert result is not None
        assert result.id == "first_import"
        assert result.points == 10

    def test_unlock_achievement_increments_total_points(self):
        """Unlocking an achievement adds its points to total_points."""
        from core.achievements import AchievementSystem
        system = AchievementSystem()
        assert system.total_points == 0
        system.unlock_achievement("first_import")
        assert system.total_points == 10

    def test_unlock_achievement_sets_unlocked_at(self):
        """Unlocking sets the unlocked_at timestamp."""
        from core.achievements import AchievementSystem
        system = AchievementSystem()
        assert system.achievements["first_import"].unlocked_at is None
        system.unlock_achievement("first_import")
        assert system.achievements["first_import"].unlocked_at is not None

    def test_unlock_achievement_idempotent(self):
        """Unlocking the same achievement twice returns None and does not double-count points."""
        from core.achievements import AchievementSystem
        system = AchievementSystem()
        system.unlock_achievement("first_import")
        assert system.total_points == 10
        result = system.unlock_achievement("first_import")
        assert result is None
        assert system.total_points == 10

    def test_unlock_unknown_achievement_returns_none(self):
        """Unknown achievement IDs return None."""
        from core.achievements import AchievementSystem
        system = AchievementSystem()
        result = system.unlock_achievement("nonexistent")
        assert result is None

    def test_check_achievements_unlocks_first_import(self):
        """check_achievements auto-unlocks first_import after 1 import."""
        from core.achievements import AchievementSystem
        system = AchievementSystem()
        unlocked = system.check_achievements()
        assert len(unlocked) == 0
        unlocked = system.update_stats(imports_performed=1)
        assert any(a.id == "first_import" for a in unlocked)

    def test_check_achievements_unlocks_paper_collector(self):
        """check_achievements auto-unlocks paper_collector at 10 papers."""
        from core.achievements import AchievementSystem
        system = AchievementSystem()
        unlocked = system.update_stats(papers_processed=10)
        assert any(a.id == "paper_collector" for a in unlocked)

    def test_check_achievements_unlocks_api_saver(self):
        """check_achievements auto-unlocks api_saver at 100 api_calls_saved."""
        from core.achievements import AchievementSystem
        system = AchievementSystem()
        unlocked = system.update_stats(api_calls_saved=100)
        assert any(a.id == "api_saver" for a in unlocked)

    def test_check_achievements_unlocks_time_saver(self):
        """check_achievements auto-unlocks time_saver at 10 hours_saved."""
        from core.achievements import AchievementSystem
        system = AchievementSystem()
        unlocked = system.update_stats(hours_saved=10.0)
        assert any(a.id == "time_saver" for a in unlocked)

    def test_check_achievements_unlocks_speed_demon(self):
        """check_achievements auto-unlocks speed_demon at 50 papers_processed."""
        from core.achievements import AchievementSystem
        system = AchievementSystem()
        unlocked = system.update_stats(papers_processed=50)
        assert any(a.id == "speed_demon" for a in unlocked)

    def test_check_achievements_unlocks_researcher_100(self):
        """check_achievements auto-unlocks researcher_100 at 100 papers."""
        from core.achievements import AchievementSystem
        system = AchievementSystem()
        unlocked = system.update_stats(papers_processed=100)
        assert any(a.id == "researcher_100" for a in unlocked)

    def test_get_unlocked_achievements(self):
        """get_unlocked_achievements returns only achievements with unlocked_at set."""
        from core.achievements import AchievementSystem
        system = AchievementSystem()
        system.unlock_achievement("first_import")
        unlocked = system.get_unlocked_achievements()
        assert len(unlocked) == 1
        assert unlocked[0].id == "first_import"

    def test_get_pending_achievements(self):
        """get_pending_achievements returns achievements not yet unlocked."""
        from core.achievements import AchievementSystem
        system = AchievementSystem()
        pending = system.get_pending_achievements()
        assert len(pending) == 8
        system.unlock_achievement("first_import")
        pending = system.get_pending_achievements()
        assert len(pending) == 7

    def test_update_stats_returns_check_achievements(self):
        """update_stats returns list of newly unlocked achievements."""
        from core.achievements import AchievementSystem
        system = AchievementSystem()
        result = system.update_stats(imports_performed=1)
        assert any(a.id == "first_import" for a in result)

    def test_get_progress_report_contains_sections(self):
        """Progress report contains key sections."""
        from core.achievements import AchievementSystem
        system = AchievementSystem()
        report = system.get_progress_report()
        assert "总积分" in report
        assert "已解锁成就" in report
        assert "使用统计" in report
        assert "即将解锁" in report

    def test_get_value_saved_keys(self):
        """get_value_saved returns expected keys."""
        from core.achievements import AchievementSystem
        system = AchievementSystem()
        system.update_stats(api_calls_saved=50)
        saved = system.get_value_saved()
        assert "hours_saved" in saved
        assert "cost_saved" in saved
        assert "papers_processed" in saved
        assert "achievement_points" in saved

    def test_get_value_saved_hours_calculation(self):
        """hours_saved = api_calls_saved * 0.1."""
        from core.achievements import AchievementSystem
        system = AchievementSystem()
        system.update_stats(api_calls_saved=100)
        saved = system.get_value_saved()
        assert "10.0" in saved["hours_saved"]

    def test_get_value_saved_cost_calculation(self):
        """cost_saved = api_calls_saved * 0.01."""
        from core.achievements import AchievementSystem
        system = AchievementSystem()
        system.update_stats(api_calls_saved=100)
        saved = system.get_value_saved()
        assert "$1.00" in saved["cost_saved"]


class TestGlobalAchievementSystem:
    """Tests for global singleton."""

    def test_get_achievement_system_returns_instance(self):
        """get_achievement_system returns an AchievementSystem instance."""
        from core.achievements import get_achievement_system
        system = get_achievement_system()
        assert system is not None

    def test_get_achievement_system_singleton(self):
        """get_achievement_system returns the same instance on repeated calls."""
        from core.achievements import get_achievement_system
        system1 = get_achievement_system()
        system2 = get_achievement_system()
        assert system1 is system2
