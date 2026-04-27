"""
Insight Evolution Tracker: Track user research exploration patterns.

Insight Evolution 追踪 — 系统记录用户探索了哪些 gaps/hypotheses，
忽略哪些。形成用户研究偏好的学习。

这才是 'Self-Evolving Research Partner' 的核心：
- 追踪用户对研究空白的探索行为
- 学习用户的研究偏好（方法论、领域、风险承受度）
- 基于历史行为优化推荐
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum
from pathlib import Path
from typing import Any, Dict, List, Optional


class ExplorationAction(Enum):
    """User actions during research exploration."""
    VIEWED = "viewed"           # 查看详情
    ACCEPTED = "accepted"       # 采纳/喜欢
    REJECTED = "rejected"       # 忽略/跳过
    EXPANDED = "expanded"       # 展开子问题
    HYPOTHESIZED = "hypothesized"  # 生成了假说
    VALIDATED = "validated"     # 验证了问题
    NARRATED = "narrated"      # 编织了故事


class PreferenceTag(Enum):
    """Research preference tags learned from behavior."""
    METHOD_FOCUSED = "method_focused"      # 方法论导向
    APPLICATION_FOCUSED = "app_focused"    # 应用导向
    THEORY_FOCUSED = "theory_focused"      # 理论导向
    HIGH_RISK_TOLERANT = "high_risk"      # 高风险容忍
    LOW_RISK_TOLERANT = "low_risk"         # 低风险容忍
    EXPLORATORY = "exploratory"           # 探索型
    CONFIRMATORY = "confirmatory"          # 验证型
    CROSS_DOMAIN = "cross_domain"          # 跨领域兴趣


@dataclass
class EvolutionEvent:
    """A single exploration event."""
    timestamp: str
    topic: str
    action: ExplorationAction
    gap_type: str = ""          # GapType enum value
    gap_title: str = ""
    gap_description: str = ""
    hypothesis_id: str = ""
    question_id: str = ""
    paper_ids: List[str] = field(default_factory=list)
    duration_seconds: int = 0   # Time spent on this item
    notes: str = ""             # User's optional notes


@dataclass
class UserPreferenceProfile:
    """Learned user research preferences."""
    total_sessions: int = 0
    total_events: int = 0

    # Action counts
    views: int = 0
    accepts: int = 0
    rejects: int = 0
    expands: int = 0
    hypothesizes: int = 0

    # Gap type preferences (which types user engages with)
    gap_type_preferences: Dict[str, float] = field(default_factory=dict)

    # Topics explored
    topics_explored: List[str] = field(default_factory=list)
    topic_frequency: Dict[str, int] = field(default_factory=dict)

    # Preference tags (computed)
    preference_tags: List[str] = field(default_factory=list)

    # Recent topics (last 10)
    recent_topics: List[str] = field(default_factory=list)

    # Last updated
    last_updated: str = ""


@dataclass
class GapExplorationState:
    """Current state of a gap exploration session."""
    topic: str
    session_id: str
    started_at: str
    events: List[EvolutionEvent] = field(default_factory=list)
    gaps_explored: List[str] = field(default_factory=list)  # gap titles
    gaps_accepted: List[str] = field(default_factory=list)   # accepted gap titles
    gaps_rejected: List[str] = field(default_factory=list)   # rejected gap titles
    hypotheses_generated: int = 0


class EvolutionTracker:
    """
    Track and learn from user research exploration patterns.

    核心功能：
    1. 记录用户的探索行为事件
    2. 构建用户偏好画像
    3. 基于历史优化推荐
    """

    def __init__(self, data_dir: Optional[Path] = None):
        self.data_dir = data_dir or Path.home() / ".ai_research_os" / "evolution"
        self.data_dir.mkdir(parents=True, exist_ok=True)
        self.events_file = self.data_dir / "events.jsonl"
        self.profile_file = self.data_dir / "preference_profile.json"
        self.sessions_dir = self.data_dir / "sessions"
        self.sessions_dir.mkdir(exist_ok=True)

    def _get_timestamp(self) -> str:
        return datetime.now().isoformat()

    # ─── Event Recording ─────────────────────────────────────────────────────────

    def record_event(
        self,
        topic: str,
        action: ExplorationAction,
        gap_type: str = "",
        gap_title: str = "",
        gap_description: str = "",
        hypothesis_id: str = "",
        question_id: str = "",
        paper_ids: Optional[List[str]] = None,
        duration_seconds: int = 0,
        notes: str = "",
    ) -> EvolutionEvent:
        """Record a single exploration event."""
        event = EvolutionEvent(
            timestamp=self._get_timestamp(),
            topic=topic,
            action=action,
            gap_type=gap_type,
            gap_title=gap_title,
            gap_description=gap_description,
            hypothesis_id=hypothesis_id,
            question_id=question_id,
            paper_ids=paper_ids or [],
            duration_seconds=duration_seconds,
            notes=notes,
        )

        # Append to events log (serialize enum as value)
        event_data = event.__dict__.copy()
        event_data["action"] = event.action.value
        with open(self.events_file, "a", encoding="utf-8") as f:
            f.write(json.dumps(event_data, ensure_ascii=False) + "\n")

        # Update profile
        self._update_profile(event)

        return event

    def record_gap_view(
        self,
        topic: str,
        gap_type: str,
        gap_title: str,
        gap_description: str = "",
        duration_seconds: int = 0,
    ) -> EvolutionEvent:
        """Record viewing a research gap."""
        return self.record_event(
            topic=topic,
            action=ExplorationAction.VIEWED,
            gap_type=gap_type,
            gap_title=gap_title,
            gap_description=gap_description,
            duration_seconds=duration_seconds,
        )

    def record_gap_accept(
        self,
        topic: str,
        gap_type: str,
        gap_title: str,
        gap_description: str = "",
    ) -> EvolutionEvent:
        """Record accepting/choosing a gap for further exploration."""
        return self.record_event(
            topic=topic,
            action=ExplorationAction.ACCEPTED,
            gap_type=gap_type,
            gap_title=gap_title,
            gap_description=gap_description,
        )

    def record_gap_reject(
        self,
        topic: str,
        gap_type: str,
        gap_title: str,
        reason: str = "",
    ) -> EvolutionEvent:
        """Record rejecting/ignoring a gap."""
        return self.record_event(
            topic=topic,
            action=ExplorationAction.REJECTED,
            gap_type=gap_type,
            gap_title=gap_title,
            notes=reason,
        )

    def record_expand(
        self,
        topic: str,
        gap_type: str,
        gap_title: str,
        sub_questions: List[str],
    ) -> EvolutionEvent:
        """Record expanding a gap into sub-questions."""
        return self.record_event(
            topic=topic,
            action=ExplorationAction.EXPANDED,
            gap_type=gap_type,
            gap_title=gap_title,
            notes="; ".join(sub_questions[:3]),
        )

    def record_hypothesis_generated(
        self,
        topic: str,
        gap_type: str,
        gap_title: str,
        hypothesis_id: str,
    ) -> EvolutionEvent:
        """Record generating a hypothesis from a gap."""
        return self.record_event(
            topic=topic,
            action=ExplorationAction.HYPOTHESIZED,
            gap_type=gap_type,
            gap_title=gap_title,
            hypothesis_id=hypothesis_id,
        )

    # ─── Profile Learning ───────────────────────────────────────────────────────

    def _update_profile(self, event: EvolutionEvent) -> None:
        """Update user preference profile based on event."""
        profile = self._load_profile()

        profile.total_events += 1
        profile.last_updated = self._get_timestamp()

        # Update action counts
        action_counts = {
            ExplorationAction.VIEWED: "views",
            ExplorationAction.ACCEPTED: "accepts",
            ExplorationAction.REJECTED: "rejects",
            ExplorationAction.EXPANDED: "expands",
            ExplorationAction.HYPOTHESIZED: "hypothesizes",
        }
        attr = action_counts.get(event.action)
        if attr:
            setattr(profile, attr, getattr(profile, attr, 0) + 1)

        # Update topic frequency
        if event.topic:
            profile.topic_frequency[event.topic] = profile.topic_frequency.get(event.topic, 0) + 1
            if event.topic not in profile.topics_explored:
                profile.topics_explored.append(event.topic)
            profile.recent_topics = list(dict.fromkeys(
                [event.topic] + profile.recent_topics
            ))[:10]

        # Update gap type preferences
        if event.gap_type:
            current = profile.gap_type_preferences.get(event.gap_type, 0.0)
            # Weight accept/reject more than view
            weight = 0.1
            if event.action == ExplorationAction.ACCEPTED:
                weight = 0.3
            elif event.action == ExplorationAction.REJECTED:
                # Distinguish: gap reject (no hypothesis_id) vs experiment reject (has hypothesis_id)
                weight = -0.3 if event.hypothesis_id else -0.1
            elif event.action == ExplorationAction.EXPANDED:
                weight = 0.2
            elif event.action == ExplorationAction.HYPOTHESIZED:
                weight = 0.25
            elif event.action == ExplorationAction.VALIDATED:
                # Experiment success: strong positive signal for this gap type
                weight = 0.4
            elif event.action == ExplorationAction.NARRATED:
                # Building arguments: positive signal for this gap type
                weight = 0.25

            profile.gap_type_preferences[event.gap_type] = current + weight

        # Compute preference tags
        profile.preference_tags = self._compute_preference_tags(profile)

        self._save_profile(profile)

    def _compute_preference_tags(self, profile: UserPreferenceProfile) -> List[str]:
        """Compute preference tags from profile stats."""
        tags = []

        # Gap type preferences
        if profile.gap_type_preferences:
            top_type = max(profile.gap_type_preferences.items(), key=lambda x: x[1])[0]

            if "method" in top_type.lower():
                tags.append(PreferenceTag.METHOD_FOCUSED.value)
            elif "application" in top_type.lower() or "unexplored" in top_type.lower():
                tags.append(PreferenceTag.APPLICATION_FOCUSED.value)
            elif "theoretical" in top_type.lower():
                tags.append(PreferenceTag.THEORY_FOCUSED.value)

        # Action ratios
        total = max(profile.views, 1)
        accept_rate = profile.accepts / total
        reject_rate = profile.rejects / total

        if accept_rate > 0.3:
            tags.append(PreferenceTag.EXPLORATORY.value)
        if reject_rate > 0.3:
            tags.append(PreferenceTag.LOW_RISK_TOLERANT.value)

        if profile.hypothesizes > profile.views * 0.2:
            tags.append(PreferenceTag.HIGH_RISK_TOLERANT.value)

        # Cross-domain detection
        if len(profile.topics_explored) >= 3:
            # Check if topics are diverse (simple heuristic)
            topics_str = " ".join(profile.topics_explored).lower()
            domain_indicators = ["nlp", "vision", "audio", "graph", "reinforcement", "supervised"]
            detected = sum(1 for d in domain_indicators if d in topics_str)
            if detected >= 2:
                tags.append(PreferenceTag.CROSS_DOMAIN.value)

        return list(set(tags))  # Deduplicate

    def _load_profile(self) -> UserPreferenceProfile:
        """Load user preference profile."""
        if self.profile_file.exists():
            try:
                with open(self.profile_file, "r", encoding="utf-8") as f:
                    data = json.load(f)
                    return UserPreferenceProfile(**data)
            except Exception:
                pass
        return UserPreferenceProfile()

    def _save_profile(self, profile: UserPreferenceProfile) -> None:
        """Save user preference profile."""
        with open(self.profile_file, "w", encoding="utf-8") as f:
            json.dump(profile.__dict__, f, ensure_ascii=False, indent=2)

    def get_profile(self) -> UserPreferenceProfile:
        """Get current user preference profile."""
        return self._load_profile()

    # ─── Query Methods ───────────────────────────────────────────────────────────

    def get_recent_events(
        self,
        topic: Optional[str] = None,
        limit: int = 50,
    ) -> List[EvolutionEvent]:
        """Get recent exploration events."""
        if not self.events_file.exists():
            return []

        events = []
        try:
            with open(self.events_file, "r", encoding="utf-8") as f:
                for line in f:
                    if line.strip():
                        try:
                            data = json.loads(line)
                            # Deserialize action from string
                            if "action" in data and isinstance(data["action"], str):
                                data["action"] = ExplorationAction(data["action"])
                            event = EvolutionEvent(**data)
                            if topic is None or event.topic == topic:
                                events.append(event)
                        except Exception:
                            continue
        except Exception:
            pass

        return events[-limit:]

    def get_topic_history(self, topic: str) -> List[EvolutionEvent]:
        """Get all events for a specific topic."""
        return self.get_recent_events(topic=topic, limit=1000)

    def get_hypothesis_events(self, hypothesis_id: str) -> List[EvolutionEvent]:
        """Get all events for a specific hypothesis_id."""
        if not self.events_file.exists():
            return []
        events = []
        try:
            with open(self.events_file, "r", encoding="utf-8") as f:
                for line in f:
                    if not line.strip():
                        continue
                    try:
                        data = json.loads(line)
                        if isinstance(data.get("action"), str):
                            data["action"] = ExplorationAction(data["action"])
                        event = EvolutionEvent(**data)
                        if event.hypothesis_id == hypothesis_id:
                            events.append(event)
                    except Exception:
                        continue
        except Exception:
            pass
        return events

    def get_preferred_gap_types(self, limit: int = 3) -> List[str]:
        """Get most preferred gap types based on history."""
        profile = self._load_profile()
        if not profile.gap_type_preferences:
            return []

        sorted_types = sorted(
            profile.gap_type_preferences.items(),
            key=lambda x: x[1],
            reverse=True,
        )
        return [gt for gt, score in sorted_types[:limit] if score > 0]

    def get_disliked_gap_types(self, limit: int = 2) -> List[str]:
        """Get gap types user tends to reject."""
        profile = self._load_profile()
        # This would require analyzing reject events specifically
        # For now, return gap types with negative scores
        return [
            gt for gt, score in profile.gap_type_preferences.items()
            if score < 0
        ][:limit]

    def get_exploration_stats(self) -> Dict[str, Any]:
        """Get overall exploration statistics."""
        profile = self._load_profile()
        recent = self.get_recent_events(limit=100)

        stats = {
            "total_events": profile.total_events,
            "total_sessions": profile.total_sessions,
            "total_topics": len(profile.topics_explored),
            "recent_events": len(recent),
            "preference_tags": profile.preference_tags,
            "top_gap_types": self.get_preferred_gap_types(5),
            "topic_frequency": dict(
                sorted(
                    profile.topic_frequency.items(),
                    key=lambda x: x[1],
                    reverse=True,
                )[:5]
            ),
        }

        # Action breakdown
        if recent:
            action_counts = {}
            for e in recent:
                action_counts[e.action.value] = action_counts.get(e.action.value, 0) + 1
            stats["recent_action_breakdown"] = action_counts

        return stats

    def render_stats(self) -> str:
        """Render exploration statistics overview as text."""
        stats = self.get_exploration_stats()
        profile = self._load_profile()

        lines = [
            "═" * 60,
            "📊 探索统计概览",
            "═" * 60,
            "",
            f"总事件: {stats['total_events']}  |  探索主题: {stats['total_topics']}",
            "",
        ]

        if stats.get('recent_action_breakdown'):
            lines.append("⚡ 最近行为分布:")
            for action, count in sorted(stats['recent_action_breakdown'].items()):
                lines.append(f"   {action}: {count}")
            lines.append("")

        if stats.get('top_gap_types'):
            lines.append("📈 偏好的 Gap 类型 (Top 5):")
            for i, gt in enumerate(stats['top_gap_types'], 1):
                score = profile.gap_type_preferences.get(gt, 0)
                lines.append(f"   {i}. {gt} ({score:+.2f})")
            lines.append("")

        if stats.get('topic_frequency'):
            lines.append("🔑 热门研究主题:")
            for topic, count in list(stats['topic_frequency'].items())[:5]:
                lines.append(f"   • {topic} ({count}次)")
            lines.append("")

        if stats.get('preference_tags'):
            lines.append("🏷️ 偏好标签:")
            for tag in stats['preference_tags']:
                lines.append(f"   • {tag}")

        lines.append("")
        lines.append("═" * 60)
        return "\n".join(lines)

    # ─── Recommendation Helper ───────────────────────────────────────────────────

    def should_prioritize_gap_type(self, gap_type: str) -> bool:
        """Check if a gap type should be prioritized for this user."""
        profile = self._load_profile()
        score = profile.gap_type_preferences.get(gap_type, 0.0)
        return score > 0.1  # Threshold for positive preference

    def should_deprioritize_gap_type(self, gap_type: str) -> bool:
        """Check if a gap type should be deprioritized."""
        profile = self._load_profile()
        score = profile.gap_type_preferences.get(gap_type, 0.0)
        return score < -0.05  # Threshold for negative preference

    def get_recommended_gap_order(
        self,
        gaps: List[Dict[str, str]],
    ) -> List[Dict[str, str]]:
        """
        Reorder gaps based on user preferences.

        Args:
            gaps: List of dicts with 'type' and 'title' keys

        Returns:
            Reordered list with preferred types first
        """
        profile = self._load_profile()

        def gap_score(gap: Dict[str, str]) -> float:
            gap_type = gap.get("type", "")
            return profile.gap_type_preferences.get(gap_type, 0.0)

        return sorted(gaps, key=gap_score, reverse=True)

    # ─── Rendering ──────────────────────────────────────────────────────────────

    def render_profile(self) -> str:
        """Render user preference profile as text."""
        profile = self._load_profile()
        stats = self.get_exploration_stats()

        lines = [
            "═" * 60,
            "🧠 研究偏好画像",
            "═" * 60,
            "",
            f"总探索事件: {stats['total_events']}",
            f"探索主题数: {stats['total_topics']}",
            "",
        ]

        if stats.get('preference_tags'):
            lines.append("🏷️ 偏好标签:")
            for tag in stats['preference_tags']:
                lines.append(f"   • {tag}")
            lines.append("")

        if stats.get('top_gap_types'):
            lines.append("📊 偏好的空白类型 (Top 5):")
            for i, gap_type in enumerate(stats['top_gap_types'], 1):
                score = profile.gap_type_preferences.get(gap_type, 0)
                lines.append(f"   {i}. {gap_type}: {score:.2f}")
            lines.append("")

        if stats.get('topic_frequency'):
            lines.append("📚 热门研究主题:")
            for topic, count in stats['topic_frequency'].items():
                lines.append(f"   • {topic} ({count}次)")
            lines.append("")

        if stats.get('recent_action_breakdown'):
            lines.append("⚡ 最近行为分布:")
            for action, count in stats['recent_action_breakdown'].items():
                lines.append(f"   • {action}: {count}")
            lines.append("")

        lines.append("═" * 60)
        return "\n".join(lines)

    def render_gap_type_preferences_history(self) -> str:
        """Render the timeline of how gap_type_preferences evolved.

        Replays all events from the JSONL log to reconstruct preference values
        at key points in time, showing how the user's research tastes evolved.
        """
        # Load all events from disk (sorted by time)
        if not self.events_file.exists():
            return "暂无探索事件记录"

        events = []
        with open(self.events_file, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    data = json.loads(line)
                    # Reconstruct ExplorationAction from string
                    action_str = data.get("action", "")
                    try:
                        action = ExplorationAction(action_str)
                    except ValueError:
                        continue  # Skip unknown action types
                    events.append(EvolutionEvent(
                        timestamp=data.get("timestamp", ""),
                        topic=data.get("topic", ""),
                        action=action,
                        gap_type=data.get("gap_type", ""),
                        gap_title=data.get("gap_title", ""),
                        gap_description=data.get("gap_description", ""),
                        hypothesis_id=data.get("hypothesis_id", ""),
                        question_id=data.get("question_id", ""),
                        paper_ids=data.get("paper_ids", []),
                        duration_seconds=data.get("duration_seconds", 0),
                        notes=data.get("notes", ""),
                    ))
                except (json.JSONDecodeError, KeyError):
                    continue

        if not events:
            return "暂无探索事件记录"

        events.sort(key=lambda e: e.timestamp)

        # Compute running preferences
        running: Dict[str, float] = {}
        all_gap_types = set()

        # Capture checkpoints: first event, every significant change, last event
        checkpoints = []
        last_snapshot: Dict[str, float] = {}

        def _make_snapshot(ts: str) -> tuple:
            return (ts, dict(running))

        # First event
        first_ts = events[0].timestamp[:10]

        for event in events:
            if event.gap_type:
                all_gap_types.add(event.gap_type)
                current = running.get(event.gap_type, 0.0)
                weight = self._event_weight(event)
                running[event.gap_type] = current + weight

        last_ts = events[-1].timestamp[:10]

        if not all_gap_types:
            return "暂无 gap_type 偏好记录"

        # Collect all gap types that ever had non-zero score
        active_types = [gt for gt in all_gap_types if running.get(gt, 0.0) != 0.0]

        if not active_types:
            # All ended at zero — still show them
            active_types = sorted(all_gap_types)

        # Build columns: gap_type | first_val | ... | current_val | trend
        header_gap = "gap_type"
        val_width = 8
        header_width = max(len("gap_type"), max(len(gt) for gt in active_types))

        # Simple format: show first (earliest non-zero) and last values + trend
        # Find first non-zero for each type
        first_nonzero: Dict[str, float] = {}
        for gt in active_types:
            running2: Dict[str, float] = {}
            for e in events:
                if e.gap_type == gt:
                    current2 = running2.get(gt, 0.0)
                    running2[gt] = current2 + self._event_weight(e)
            # Find first non-zero
            running3: Dict[str, float] = {}
            first_nonzero[gt] = 0.0
            for e in events:
                if e.gap_type == gt:
                    current3 = running3.get(gt, 0.0)
                    new_val = current3 + self._event_weight(e)
                    running3[gt] = new_val
                    if new_val != 0.0 and first_nonzero[gt] == 0.0:
                        first_nonzero[gt] = new_val

        current_vals = running

        def trend_arrow(gt: str) -> str:
            first = first_nonzero.get(gt, 0.0)
            cur = current_vals.get(gt, 0.0)
            if cur > first + 0.05:
                return "↑↑"
            elif cur > first + 0.01:
                return "↑ "
            elif cur < first - 0.05:
                return "↓↓"
            elif cur < first - 0.01:
                return "↓ "
            elif abs(cur) < 0.01 and abs(first) < 0.01:
                return "  "  # both near zero
            else:
                return "~ "

        def fmt_val(v: float) -> str:
            if v == 0.0:
                return "  .00"
            return f"{v:>+6.2f}"

        # Header
        total_events = len(events)
        lines = [
            "═" * 70,
            "📈 Gap Type 偏好演化时间轴",
            "═" * 70,
            "",
            f"  事件总数: {total_events}  |  周期: {first_ts} ~ {last_ts}",
            "",
        ]

        # Table header
        col_gap = "gap_type"
        col_first = "初始"
        col_curr = "当前"
        col_trend = "趋势"
        lines.append(f"  {col_gap:<{header_width}}  {col_first:>{val_width}}  {col_curr:>{val_width}}  {col_trend}")
        lines.append(f"  {'─' * header_width}  {'─' * val_width}  {'─' * val_width}  {'─' * 4}")

        # Sort by current value descending
        sorted_types = sorted(active_types, key=lambda gt: current_vals.get(gt, 0.0), reverse=True)

        for gt in sorted_types:
            first_v = first_nonzero.get(gt, 0.0)
            cur_v = current_vals.get(gt, 0.0)
            arrow = trend_arrow(gt)
            # Color by sentiment
            if cur_v > 0.1:
                bar = "🟢"
            elif cur_v < -0.05:
                bar = "🔴"
            else:
                bar = "⚪"
            lines.append(f"  {bar} {gt:<{header_width - 2}}  {fmt_val(first_v)}  {fmt_val(cur_v)}  {arrow}")

        lines.append("")
        lines.append("  解释: 🟢 = 正偏好(优先)  🔴 = 负偏好(规避)  ⚪ = 中性")
        lines.append("  趋势: ↑↑/↑ 偏好增强   ↓↓/↓ 偏好减弱   ~  稳定")
        lines.append("")
        lines.append("═" * 70)
        return "\n".join(lines)

    def _event_weight(self, event: EvolutionEvent) -> float:
        """Compute preference weight for a single event (matches _update_profile)."""
        weight = 0.1
        if event.action == ExplorationAction.ACCEPTED:
            weight = 0.3
        elif event.action == ExplorationAction.REJECTED:
            weight = -0.15
        elif event.action == ExplorationAction.EXPANDED:
            weight = 0.2
        elif event.action == ExplorationAction.HYPOTHESIZED:
            weight = 0.4
        elif event.action == ExplorationAction.VALIDATED:
            weight = 0.4
        elif event.action == ExplorationAction.NARRATED:
            weight = 0.25
        elif event.action == ExplorationAction.VIEWED:
            weight = 0.05
        return weight

    def render_topic_history(self, topic: str) -> str:
        """Render exploration history for a topic."""
        events = self.get_topic_history(topic)

        if not events:
            return f"暂无 '{topic}' 的探索记录"

        lines = [
            "═" * 60,
            f"📖 '{topic}' 探索历史",
            "═" * 60,
            "",
        ]

        current_date = ""
        for event in events[-30:]:  # Last 30 events
            date = event.timestamp[:10]
            if date != current_date:
                current_date = date
                lines.append(f"\n📅 {date}")
                lines.append("-" * 40)

            action_icon = {
                ExplorationAction.VIEWED: "👁️",
                ExplorationAction.ACCEPTED: "✅",
                ExplorationAction.REJECTED: "❌",
                ExplorationAction.EXPANDED: "📋",
                ExplorationAction.HYPOTHESIZED: "🎯",
            }.get(event.action, "•")

            time = event.timestamp[11:16]
            gap_short = event.gap_title[:30] if event.gap_title else "N/A"

            lines.append(
                f"  {time} {action_icon} [{event.action.value}] {gap_short}"
            )

        lines.append("")
        lines.append("═" * 60)
        return "\n".join(lines)
