"""
User Achievement System - Inspired by Volkswagen V2G motivation system.

Volkswagen offers:
- Base remuneration for participating in V2G
- Cost savings (700-900 euros annually)
- Battery life protection guarantee

Similarly, we offer:
- Achievement points and badges
- Time saved metrics
- Performance protection guarantees
"""
from typing import Dict, List, Optional
from dataclasses import dataclass
from datetime import datetime


@dataclass
class Achievement:
    """Represents an achievement or badge."""
    id: str
    name: str
    description: str
    icon: str
    points: int
    unlocked_at: Optional[datetime] = None


class AchievementSystem:
    """
    User achievement system with points and badges.

    Inspired by Volkswagen's V2G incentive system:
    - Base remuneration for participation
    - Cost savings metrics
    - Protection guarantees
    """

    def __init__(self):
        self.achievements: Dict[str, Achievement] = {}
        self.total_points = 0
        self.user_stats = {
            "papers_processed": 0,
            "api_calls_saved": 0,
            "hours_saved": 0.0,
            "searches_performed": 0,
            "imports_performed": 0,
        }
        self._init_achievements()

    def _init_achievements(self):
        """Initialize achievement definitions."""
        self.achievements = {
            "first_import": Achievement(
                id="first_import",
                name="🚀 首次导入",
                description="成功导入第一篇论文",
                icon="📥",
                points=10
            ),
            "paper_collector": Achievement(
                id="paper_collector",
                name="📚 论文收集者",
                description="导入10篇论文",
                icon="📚",
                points=50
            ),
            "researcher_100": Achievement(
                id="researcher_100",
                name="🎓 研究达人",
                description="导入100篇论文",
                icon="🎓",
                points=200
            ),
            "api_saver": Achievement(
                id="api_saver",
                name="💰 API节流侠",
                description="通过缓存节省100次API调用",
                icon="💰",
                points=100
            ),
            "time_saver": Achievement(
                id="time_saver",
                name="⏰ 时间管理大师",
                description="节省10小时研究时间",
                icon="⏰",
                points=150
            ),
            "speed_demon": Achievement(
                id="speed_demon",
                name="⚡ 速度达人",
                description="批量导入50篇论文",
                icon="⚡",
                points=100
            ),
            "cache_master": Achievement(
                id="cache_master",
                name="🗄️ 缓存大师",
                description="缓存命中率超过80%",
                icon="🗄️",
                points=75
            ),
            "search_expert": Achievement(
                id="search_expert",
                name="🔍 搜索专家",
                description="执行100次搜索",
                icon="🔍",
                points=50
            ),
        }

    def unlock_achievement(self, achievement_id: str) -> Optional[Achievement]:
        """Unlock an achievement."""
        if achievement_id not in self.achievements:
            return None

        achievement = self.achievements[achievement_id]
        if achievement.unlocked_at is None:
            achievement.unlocked_at = datetime.now()
            self.total_points += achievement.points
            return achievement
        return None

    def check_achievements(self) -> List[Achievement]:
        """Check and auto-unlock achievements based on stats."""
        unlocked = []

        # Check first import
        if self.user_stats["imports_performed"] >= 1:
            result = self.unlock_achievement("first_import")
            if result:
                unlocked.append(result)

        # Check paper collector
        if self.user_stats["papers_processed"] >= 10:
            result = self.unlock_achievement("paper_collector")
            if result:
                unlocked.append(result)

        # Check researcher
        if self.user_stats["papers_processed"] >= 100:
            result = self.unlock_achievement("researcher_100")
            if result:
                unlocked.append(result)

        # Check API saver
        if self.user_stats["api_calls_saved"] >= 100:
            result = self.unlock_achievement("api_saver")
            if result:
                unlocked.append(result)

        # Check time saver
        if self.user_stats["hours_saved"] >= 10:
            result = self.unlock_achievement("time_saver")
            if result:
                unlocked.append(result)

        # Check speed demon
        if self.user_stats["papers_processed"] >= 50:
            result = self.unlock_achievement("speed_demon")
            if result:
                unlocked.append(result)

        return unlocked

    def update_stats(self, **kwargs):
        """Update user statistics."""
        for key, value in kwargs.items():
            if key in self.user_stats:
                self.user_stats[key] = value

        # Auto-check achievements after stats update
        return self.check_achievements()

    def get_unlocked_achievements(self) -> List[Achievement]:
        """Get all unlocked achievements."""
        return [a for a in self.achievements.values() if a.unlocked_at is not None]

    def get_pending_achievements(self) -> List[Achievement]:
        """Get all pending achievements."""
        return [a for a in self.achievements.values() if a.unlocked_at is None]

    def get_progress_report(self) -> str:
        """Generate a progress report."""
        lines = [
            "=" * 60,
            "🏆 成就报告",
            "=" * 60,
            f"\n总积分: {self.total_points}",
            f"已解锁成就: {len(self.get_unlocked_achievements())}/{len(self.achievements)}",
            "",
            "📊 使用统计:",
            f"  处理论文数: {self.user_stats['papers_processed']}",
            f"  节省API调用: {self.user_stats['api_calls_saved']}",
            f"  节省时间: {self.user_stats['hours_saved']:.1f} 小时",
            "",
            "🏅 已解锁成就:",
        ]

        for achievement in self.get_unlocked_achievements():
            lines.append(f"  {achievement.icon} {achievement.name} (+{achievement.points}分)")

        if not self.get_unlocked_achievements():
            lines.append("  暂无解锁成就")

        lines.append("\n🎯 即将解锁:")
        pending = self.get_pending_achievements()[:3]
        for achievement in pending:
            lines.append(f"  {achievement.icon} {achievement.name} ({achievement.description})")

        lines.append("\n" + "=" * 60)

        return "\n".join(lines)

    def get_value_saved(self) -> Dict[str, str]:
        """Calculate value saved (inspired by VW's 700-900 euros savings)."""
        # Estimate time saved based on API calls
        hours_saved = self.user_stats["api_calls_saved"] * 0.1  # 假设每次API调用节省6分钟

        # Estimate cost savings (假设每次API调用成本0.01美元)
        cost_saved = self.user_stats["api_calls_saved"] * 0.01

        return {
            "hours_saved": f"{hours_saved:.1f} 小时",
            "cost_saved": f"${cost_saved:.2f}",
            "papers_processed": str(self.user_stats["papers_processed"]),
            "achievement_points": str(self.total_points),
        }


# Global achievement system
_achievement_system = None


def get_achievement_system() -> AchievementSystem:
    """Get or create the global achievement system."""
    global _achievement_system
    if _achievement_system is None:
        _achievement_system = AchievementSystem()
    return _achievement_system


def print_achievement_report():
    """Print the achievement report."""
    system = get_achievement_system()
    print(system.get_progress_report())
    print()
    print("💰 价值量化:")
    value = system.get_value_saved()
    for key, val in value.items():
        print(f"  {key}: {val}")
