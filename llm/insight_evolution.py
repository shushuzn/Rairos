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
import uuid
from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum
from pathlib import Path
from typing import Any, Dict, List, Optional

from llm.text_utils import extract_keywords


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

    # Keyword preferences (learned from gap titles user engages with)
    keyword_preferences: Dict[str, float] = field(default_factory=dict)

    # Topics explored
    topics_explored: List[str] = field(default_factory=list)
    topic_frequency: Dict[str, int] = field(default_factory=dict)

    # Preference tags (computed) — tag name -> confidence [0, 1]
    # Confidence > 0.6 = stable preference, 0.3-0.6 = emerging, < 0.3 = tentative
    preference_tags: Dict[str, float] = field(default_factory=dict)

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

    # Cache TTL in seconds (5 minutes — long enough to amortize O(n) scans,
    # short enough that new events feel "live")
    _CACHE_TTL_SECONDS: int = 300

    # Single source of truth for event weights.
    # Used by _update_profile (profile persistence) and _event_weight (score cache).
    # profile.update uses these directly; cache adds time decay on top.
    _EVENT_WEIGHTS: Dict[ExplorationAction, float] = {
        ExplorationAction.VIEWED: 0.05,
        ExplorationAction.ACCEPTED: 0.30,
        ExplorationAction.REJECTED: -0.30,   # gap reject (no hypothesis_id)
        ExplorationAction.EXPANDED: 0.20,
        ExplorationAction.HYPOTHESIZED: 0.40,
        ExplorationAction.VALIDATED: 0.40,
        ExplorationAction.NARRATED: 0.25,
    }
    # Gap reject (no hypothesis) gets a lighter penalty in the profile accumulator.
    _REJECT_NO_HYPOTHESIS_PENALTY: float = -0.10

    def __init__(self, data_dir: Optional[Path] = None):
        self.data_dir = data_dir or Path.home() / ".ai_research_os" / "evolution"
        self.data_dir.mkdir(parents=True, exist_ok=True)
        self.events_file = self.data_dir / "events.jsonl"
        self.profile_file = self.data_dir / "preference_profile.json"
        self.sessions_dir = self.data_dir / "sessions"
        self.sessions_dir.mkdir(exist_ok=True)
        # In-memory TTL cache for time-decayed score reads.
        # Key -> (computed_value, timestamp_iso)
        self._score_cache: Dict[str, Any] = {}
        self._cache_time: Optional[datetime] = None

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

        # Invalidate score cache so next read recomputes with new event
        self._score_cache.clear()
        self._cache_time = None

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
            weight = self._EVENT_WEIGHTS.get(event.action, 0.0)
            # Distinguish: gap reject (no hypothesis_id) vs experiment reject (has hypothesis_id)
            if event.action == ExplorationAction.REJECTED and not event.hypothesis_id:
                weight = self._REJECT_NO_HYPOTHESIS_PENALTY
            profile.gap_type_preferences[event.gap_type] = current + weight

        # Update keyword preferences from gap_title
        if event.gap_title:
            keywords = self._extract_keywords(event.gap_title)
            for kw in keywords:
                kw_current = profile.keyword_preferences.get(kw, 0.0)
                profile.keyword_preferences[kw] = kw_current + (weight * 0.5)

        # Compute preference tags
        profile.preference_tags = self._compute_preference_tags(profile)

        self._save_profile(profile)

    def _extract_keywords(self, text: str) -> List[str]:
        """Extract research-relevant keywords from text."""
        return extract_keywords(text)

    def _compute_preference_tags(self, profile: UserPreferenceProfile) -> Dict[str, float]:
        """Compute preference tags with confidence scores [0, 1]."""
        tags: Dict[str, float] = {}

        # Gap type focus: confidence = proportion of events in top type
        if profile.gap_type_preferences:
            top_type = max(profile.gap_type_preferences.items(), key=lambda x: x[1])[0]
            total_score = sum(abs(v) for v in profile.gap_type_preferences.values())
            if total_score > 0:
                top_confidence = min(abs(profile.gap_type_preferences[top_type]) / total_score, 1.0)
            else:
                top_confidence = 0.3

            if "method" in top_type.lower():
                tags[PreferenceTag.METHOD_FOCUSED.value] = top_confidence
            elif "application" in top_type.lower() or "unexplored" in top_type.lower():
                tags[PreferenceTag.APPLICATION_FOCUSED.value] = top_confidence
            elif "theoretical" in top_type.lower():
                tags[PreferenceTag.THEORY_FOCUSED.value] = top_confidence

        # Action ratios -> behavioral tags
        total = max(profile.views, 1)
        accept_rate = profile.accepts / total
        reject_rate = profile.rejects / total

        if accept_rate > 0.3:
            tags[PreferenceTag.EXPLORATORY.value] = accept_rate
        if reject_rate > 0.3:
            tags[PreferenceTag.LOW_RISK_TOLERANT.value] = reject_rate

        if profile.hypothesizes > profile.views * 0.2:
            hypo_rate = min(profile.hypothesizes / max(profile.views + profile.accepts, 1), 1.0)
            tags[PreferenceTag.HIGH_RISK_TOLERANT.value] = hypo_rate

        # Cross-domain detection
        if len(profile.topics_explored) >= 3:
            # Check if topics are diverse (simple heuristic)
            topics_str = " ".join(profile.topics_explored).lower()
            domain_indicators = ["nlp", "vision", "audio", "graph", "reinforcement", "supervised"]
            detected = sum(1 for d in domain_indicators if d in topics_str)
            if detected >= 2:
                confidence = min(detected / len(domain_indicators), 1.0)
                tags[PreferenceTag.CROSS_DOMAIN.value] = confidence

        return tags

    def _load_profile(self) -> UserPreferenceProfile:
        """Load user preference profile."""
        if self.profile_file.exists():
            try:
                with open(self.profile_file, "r", encoding="utf-8") as f:
                    data = json.load(f)
                    return UserPreferenceProfile(**data)
            except Exception:
                # Corrupt or missing profile file — return default profile without crashing.
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
                            # Skip malformed event — continue parsing without crashing.
                            continue
        except Exception:
            # Event history loading is best-effort — return partial results without crashing.
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
                        # Skip malformed event — continue parsing without crashing.
                        continue
        except Exception:
            # Hypothesis event loading is best-effort — return partial results without crashing.
            pass
        return events

    # ─── Cached Score Reads ─────────────────────────────────────────────────────

    def _is_cache_valid(self) -> bool:
        """Check if cached scores are still within TTL window."""
        if self._cache_time is None:
            return False
        age = (datetime.now() - self._cache_time).total_seconds()
        return age < self._CACHE_TTL_SECONDS

    def _get_all_scores_cached(self) -> Dict[str, Any]:
        """Return all pre-computed scores from cache or compute + cache them.

        This is the single O(n) scan that all decay-weighted reads share.
        Returns a dict with 'gap_types' ( Dict[str,float] ) and 'keywords'
        ( Dict[str,float] ) so both score families are computed in one pass.
        """
        if self._is_cache_valid():
            return self._score_cache

        events = self.get_recent_events(limit=10000)

        gap_scores: Dict[str, float] = {}
        kw_scores: Dict[str, float] = {}
        for e in events:
            w = self._event_weight(e)
            decayed = self._decay_weight(w, e.timestamp)
            if e.gap_type:
                gap_scores[e.gap_type] = gap_scores.get(e.gap_type, 0.0) + decayed
            if e.gap_title:
                for kw in extract_keywords(e.gap_title):
                    kw_scores[kw] = kw_scores.get(kw, 0.0) + decayed * 0.5

        self._score_cache = {"gap_types": gap_scores, "keywords": kw_scores}
        self._cache_time = datetime.now()
        return self._score_cache

    def _get_all_gap_type_scores(self) -> Dict[str, float]:
        """Get time-decayed scores for all gap types (from cache or single scan)."""
        return self._get_all_scores_cached()["gap_types"]

    def get_preferred_gap_types(self, limit: int = 3) -> List[str]:
        """Get most preferred gap types based on time-decayed history."""
        scores = self._get_all_gap_type_scores()
        sorted_types = sorted(scores.items(), key=lambda x: x[1], reverse=True)
        return [gt for gt, score in sorted_types[:limit] if score > 0]

    def get_disliked_gap_types(self, limit: int = 2) -> List[str]:
        """Get gap types user tends to reject (time-decayed)."""
        scores = self._get_all_gap_type_scores()
        return [gt for gt, score in scores.items() if score < -0.05][:limit]

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
        header = [
            "═" * 60,
            "📊 探索统计概览",
            "═" * 60,
            "",
            f"总事件: {stats['total_events']}  |  探索主题: {stats['total_topics']}",
            "",
        ]
        sections = [
            ("recent_action_breakdown", "⚡ 最近行为分布:", False, None),
            ("top_gap_types", "📈 偏好的 Gap 类型 (Top 5):", False, 5),
            ("topic_frequency", "🔑 热门研究主题:", False, 5),
            ("preference_tags", "🏷️ 偏好标签:", False, None),
        ]
        return "\n".join(header + self._render_profile_sections(stats, sections))

    def render_profile(self) -> str:
        """Render user preference profile as text."""
        profile = self._load_profile()
        stats = self.get_exploration_stats()
        header = [
            "═" * 60,
            "🧠 研究偏好画像",
            "═" * 60,
            "",
            f"总探索事件: {stats['total_events']}",
            f"探索主题数: {stats['total_topics']}",
            "",
        ]
        sections = [
            ("preference_tags", "🏷️ 偏好标签:", False, None),
            ("top_gap_types", "📊 偏好的空白类型 (Top 5):", False, None),
            ("topic_frequency", "📚 热门研究主题:", False, None),
            ("recent_action_breakdown", "⚡ 最近行为分布:", False, None),
        ]
        return "\n".join(header + self._render_profile_sections(stats, sections, profile))

    def _render_profile_sections(
        self,
        stats: Dict[str, Any],
        sections: List[tuple],
        profile: Optional[UserPreferenceProfile] = None,
    ) -> List[str]:
        """Render preference sections into lines. Shared by render_stats/render_profile.

        Args:
            stats: from get_exploration_stats()
            sections: list of (key, title, sort_items, limit) tuples
                key: stats dict key
                title: section header
                sort_items: whether to sort items alphabetically (action breakdown)
                limit: max items to show, or None for unlimited
            profile: optional; used for gap_type score display
        """
        if profile is None:
            profile = self._load_profile()
        lines: List[str] = []
        for key, title, sort_items, limit in sections:
            raw = stats.get(key)
            if not raw:
                continue
            lines.append(title)

            # top_gap_types is always List[str]; handle first
            if key == "top_gap_types":
                display_list = list(raw)[:limit] if limit else list(raw)
                for i, gt in enumerate(display_list, 1):
                    score = profile.gap_type_preferences.get(gt, 0)
                    lines.append(f"   {i}. {gt}: {score:.2f}")
                lines.append("")
                continue

            # All other sections: dispatch by actual type (handles both old List[str]
            # and new Dict[str, float] preference_tags from merged PRs)
            if isinstance(raw, list):
                # Legacy List[str] format (old persisted data)
                display_list = list(raw)[:limit] if limit else list(raw)
                for item in display_list:
                    lines.append(f"   • {item}")
            else:
                # Dict[str, Any] format
                items: List[tuple] = list(raw.items())
                if sort_items:
                    items = sorted(items)
                if limit:
                    items = items[:limit]

                for k, v in items:
                    if key == "preference_tags":
                        level = "🟢" if v >= 0.6 else "🟡" if v >= 0.3 else "⚪"
                        lines.append(f"   {level} {k} ({v:.0%})")
                    elif key == "topic_frequency":
                        lines.append(f"   • {k} ({v}次)")
                    elif key == "recent_action_breakdown":
                        lines.append(f"   {k}: {v}")
            lines.append("")
        return lines

    # ─── Recommendation Helper ───────────────────────────────────────────────────

    def _decay_weight(self, base_weight: float, event_timestamp: str, lambda_: float = 0.01) -> float:
        """Apply exponential time decay to an event weight.

        Recent events decay slowly; older events contribute less.
        With lambda=0.01: ~73% at 30 days, ~37% at 90 days, ~9% at 1 year.
        """
        try:
            event_time = datetime.fromisoformat(event_timestamp)
            age_days = (datetime.now() - event_time).total_seconds() / 86400.0
            return base_weight * (2.0 ** (-lambda_ * age_days))
        except (ValueError, TypeError, OSError):
            return 0.0

    def get_gap_type_score(self, gap_type: str) -> float:
        """Get the numeric preference score for a gap type with time decay."""
        scores = self._get_all_scores_cached()["gap_types"]
        return scores.get(gap_type, 0.0)

    def get_keyword_score(self, keyword: str) -> float:
        """Get the numeric preference score for a keyword with time decay."""
        kw_scores = self._get_all_scores_cached()["keywords"]
        return kw_scores.get(keyword.lower(), 0.0)

    def get_top_keywords(self, limit: int = 5) -> List[str]:
        """Get most preferred keywords based on decay-weighted history."""
        kw_scores = self._get_all_scores_cached()["keywords"]
        sorted_kws = sorted(kw_scores.items(), key=lambda x: x[1], reverse=True)
        return [kw for kw, score in sorted_kws[:limit] if score > 0.05]

    def should_deprioritize_gap_type(self, gap_type: str) -> bool:
        """Check if a gap type should be deprioritized."""
        score = self.get_gap_type_score(gap_type)
        return score < -0.05  # Threshold for negative preference

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

        # Compute running preferences — single pass
        # Also tracks first_nonzero per gap_type (avoids O(n²) per-type re-scan)
        running: Dict[str, float] = {}
        all_gap_types: set = set()
        first_nonzero: Dict[str, float] = {}

        # First event
        first_ts = events[0].timestamp[:10]

        for event in events:
            if event.gap_type:
                all_gap_types.add(event.gap_type)
                current = running.get(event.gap_type, 0.0)
                weight = self._event_weight(event)
                new_val = current + weight
                running[event.gap_type] = new_val
                if new_val != 0.0 and event.gap_type not in first_nonzero:
                    first_nonzero[event.gap_type] = new_val

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
        """Compute preference weight for a single event for the time-decay cache."""
        weight = self._EVENT_WEIGHTS.get(event.action, 0.0)
        # Gap reject (no hypothesis) gets lighter penalty in cache score too
        if event.action == ExplorationAction.REJECTED and not event.hypothesis_id:
            weight = self._REJECT_NO_HYPOTHESIS_PENALTY
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

    # ─── Persistence ─────────────────────────────────────────────────────────────

    def export_profile(self, path: Optional[Path] = None) -> Path:
        """Export preference profile to a timestamped backup file.

        Args:
            path: Optional output path. If None, writes to
                  data_dir/profile_backup_YYYY-MM-DDTHH-MM-SS.json

        Returns:
            Path to the exported file.
        """
        profile = self._load_profile()
        data = profile.__dict__.copy()
        data["_exported_at"] = self._get_timestamp()
        data["_version"] = "1.0"

        if path is None:
            ts = self._get_timestamp().replace(":", "-").replace(".", "-")[:19]
            uid = uuid.uuid4().hex[:6]
            path = self.data_dir / f"profile_backup_{ts}_{uid}.json"

        path = Path(path)
        path.parent.mkdir(parents=True, exist_ok=True)
        with open(path, "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)

        return path

    def import_profile(
        self,
        path: Path,
        merge: bool = True,
    ) -> UserPreferenceProfile:
        """Import a preference profile from a backup file.

        Args:
            path: Path to the backup JSON file.
            merge: If True (default), merge incoming values with existing
                   profile (numeric fields are summed, lists are combined and
                   deduplicated). If False, replace existing profile entirely.

        Returns:
            The resulting merged or replaced profile.
        """
        path = Path(path)
        if not path.exists():
            raise FileNotFoundError(f"Profile backup not found: {path}")

        with open(path, "r", encoding="utf-8") as f:
            data = json.load(f)

        # Strip metadata fields from backup
        data.pop("_exported_at", None)
        data.pop("_version", None)

        if merge:
            existing = self._load_profile()
            merged = self._merge_profiles(existing, data)
            self._save_profile(merged)
            # Invalidate cache so merged values are visible immediately
            self._score_cache.clear()
            self._cache_time = None
            return merged
        else:
            profile = UserPreferenceProfile(**data)
            self._save_profile(profile)
            self._score_cache.clear()
            self._cache_time = None
            return profile

    def _merge_profiles(
        self,
        base: UserPreferenceProfile,
        incoming: Dict[str, Any],
    ) -> UserPreferenceProfile:
        """Merge incoming profile data into base profile.

        Numeric fields (preferences, counts) are summed.
        List fields (topics_explored, preference_tags) are unioned.
        Scalar fields (total_events, etc.) use the larger value.
        """
        result = UserPreferenceProfile()

        # Scalars: take max (event count, etc.)
        result.total_sessions = max(base.total_sessions, incoming.get("total_sessions", 0))
        result.total_events = max(base.total_events, incoming.get("total_events", 0))
        result.views = max(base.views, incoming.get("views", 0))
        result.accepts = max(base.accepts, incoming.get("accepts", 0))
        result.rejects = max(base.rejects, incoming.get("rejects", 0))
        result.expands = max(base.expands, incoming.get("expands", 0))
        result.hypothesizes = max(base.hypothesizes, incoming.get("hypothesizes", 0))
        result.last_updated = self._get_timestamp()

        # Dict fields: sum numeric values
        base_gap = dict(base.gap_type_preferences)
        inc_gap = incoming.get("gap_type_preferences", {})
        for k, v in inc_gap.items():
            base_gap[k] = base_gap.get(k, 0.0) + v
        result.gap_type_preferences = base_gap

        base_kw = dict(base.keyword_preferences)
        inc_kw = incoming.get("keyword_preferences", {})
        for k, v in inc_kw.items():
            base_kw[k] = base_kw.get(k, 0.0) + v
        result.keyword_preferences = base_kw

        # List fields: union + preserve order
        seen = set()
        for t in base.topics_explored:
            if t not in seen:
                seen.add(t)
                result.topics_explored.append(t)
        for t in incoming.get("topics_explored", []):
            if t not in seen:
                seen.add(t)
                result.topics_explored.append(t)

        # topic_frequency: sum
        result.topic_frequency = dict(base.topic_frequency)
        for k, v in incoming.get("topic_frequency", {}).items():
            result.topic_frequency[k] = result.topic_frequency.get(k, 0) + v

        # preference_tags: Dict[str, float] — take higher confidence for each tag
        base_tags = dict(base.preference_tags)
        inc_tags = incoming.get("preference_tags", {})
        for k, v in inc_tags.items():
            base_tags[k] = max(base_tags.get(k, 0.0), v)
        result.preference_tags = base_tags

        # recent_topics: take longer list
        base_recent = list(base.recent_topics)
        inc_recent = incoming.get("recent_topics", [])
        seen = set()
        merged = []
        for t in reversed(base_recent + inc_recent):
            if t not in seen:
                seen.add(t)
                merged.append(t)
        result.recent_topics = list(reversed(merged))[:10]

        return result

    def list_backups(self) -> List[Path]:
        """List all profile backup files in data_dir."""
        if not self.data_dir.exists():
            return []
        return sorted(self.data_dir.glob("profile_backup_*.json"), reverse=True)
