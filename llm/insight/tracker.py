"""EvolutionTracker — core insight evolution engine."""

from __future__ import annotations

import json
import uuid
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional, cast

from llm.insight.preferences import ExplorationAction, PreferenceTag, EvolutionEvent
from llm.insight.storage import CapsuleStorageMixin
from llm.text_utils import extract_keywords
from llm.insight.profile import UserPreferenceProfile, GapExplorationState


class EvolutionTracker(CapsuleStorageMixin):
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
        ExplorationAction.REJECTED: -0.30,  # gap reject (no hypothesis_id)
        ExplorationAction.EXPANDED: 0.20,
        ExplorationAction.HYPOTHESIZED: 0.40,
        ExplorationAction.VALIDATED: 0.40,
        ExplorationAction.NARRATED: 0.25,
        ExplorationAction.INSIGHT_RATED: 0.20,
    }

    # Gap reject (no hypothesis) gets a lighter penalty in the profile accumulator.

    _REJECT_NO_HYPOTHESIS_PENALTY: float = -0.10

    # ─── Capsule Lifecycle Events ────────────────────────────────────────────────

    def record_capsule_lifecycle_event(
        self,
        capsule_id: str,
        action: str,
        gap_title: str,
        gap_type: str,
        details: str = "",
    ) -> None:
        """Append a capsule lifecycle event to lifecycle_events.jsonl."""
        event = {
            "timestamp": self._get_timestamp(),
            "capsule_id": capsule_id,
            "action": action,
            "gap_title": gap_title,
            "gap_type": gap_type,
            "details": details,
        }
        with open(self.lifecycle_events_file, "a", encoding="utf-8") as f:
            f.write(json.dumps(event, ensure_ascii=False) + "\n")

    def get_evolution_log(self, limit: int = 100) -> List[Dict[str, Any]]:
        """Read newest lifecycle events first (newest at top of file)."""
        if not self.lifecycle_events_file.exists():
            return []
        with open(self.lifecycle_events_file, "r", encoding="utf-8") as f:
            lines = f.readlines()
        events = [json.loads(line) for line in lines if line.strip()]
        events.reverse()
        return events[:limit]

    def __init__(self, data_dir: Optional[Path] = None):

        self.data_dir = data_dir or Path.home() / ".ai_research_os" / "evolution"

        self.data_dir.mkdir(parents=True, exist_ok=True)

        self.events_file = self.data_dir / "events.jsonl"

        self.lifecycle_events_file = self.data_dir / "lifecycle_events.jsonl"

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
        insight_card_id: str = "",
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
            insight_card_id=insight_card_id,
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
        event = self.record_event(
            topic=topic,
            action=ExplorationAction.ACCEPTED,
            gap_type=gap_type,
            gap_title=gap_title,
            gap_description=gap_description,
        )
        # Feedback loop: if capsule already exists, update its score instead of duplicating.
        existing = self.get_capsule_by_title(gap_title, topic)
        if existing:
            profile = self._load_profile()
            total = max(profile.accepts + profile.rejects + profile.views, 1)
            new_score = profile.accepts / total
            existing.feedback_count += 1
            existing.outcome_success_score = (
                existing.outcome_success_score * 0.7 + new_score * 0.3
            )
            try:
                self._update_credibility(existing)
            except Exception:
                pass
            self.update_capsule(existing)
            self.record_capsule_lifecycle_event(
                capsule_id=existing.capsule_id,
                action="consumed",
                gap_title=existing.action_gap_title,
                gap_type=existing.action_gap_type,
                details=f"Re-accepted (feedback_count={existing.feedback_count})",
            )
        else:
            profile = self._load_profile()
            total = max(profile.accepts + profile.rejects + profile.views, 1)
            success_score = profile.accepts / total
            self.encode_capsule(
                topic=topic,
                gap_type=gap_type,
                gap_title=gap_title,
                gap_description=gap_description,
                success_score=success_score,
            )
        return event

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

    def record_insight_feedback(
        self,
        topic: str,
        insight_card_id: str,
        rating: int,
        paper_title: str = "",
    ) -> EvolutionEvent:
        """Record user rating an insight card. Bridges insight quality to evolution profile."""

        event = self.record_event(
            topic=topic,
            action=ExplorationAction.INSIGHT_RATED,
            notes=f"insight:{insight_card_id} rating:{rating}",
            gap_description=paper_title,
            insight_card_id=insight_card_id,
        )

        # Immediately recompute credibility so capsule scores reflect the new feedback
        try:
            self.recompute_credibility_all()
        except Exception:
            pass

        return event

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

            profile.recent_topics = list(dict.fromkeys([event.topic] + profile.recent_topics))[:10]

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

    # ─── Research Archetype ───────────────────────────────────────────────────────

    # Map gap types to archetype dimensions

    _GAP_TYPE_TO_ARCHETYPE = {
        "method_limitation": ["method_focused"],
        "contradiction": ["method_focused"],
        "evaluation_gap": ["method_focused", "theory_focused"],
        "scalability_issue": ["method_focused"],
        "unexplored_application": ["app_focused"],
        "theoretical_gap": ["theory_focused"],
        "dataset_gap": ["theory_focused", "method_focused"],
        "generalization_gap": ["theory_focused"],
    }

    ARCHETYPE_DIMENSIONS = [
        ("method_focused", "Method-Focused", "Focuses on methodology & theory"),
        ("app_focused", "App-Focused", "Prioritizes real-world applications"),
        ("theory_focused", "Theory-Focused", "Pursues rigorous foundations"),
        ("high_risk", "High-Risk", "Tackles high-uncertainty problems"),
        ("low_risk", "Low-Risk", "Prefers robust, reproducible work"),
        ("exploratory", "Exploratory", "Loves discovering new questions"),
        ("confirmatory", "Confirmatory", "Focuses on validation & replication"),
        ("cross_domain", "Cross-Domain", "Interested in interdisciplinary work"),
    ]

    def get_archetype(self):

        profile = self._load_profile()

        event_count = profile.total_events

        gap_prefs = profile.gap_type_preferences or {}

        dimension_scores = {d: 0.0 for d, *_ in self.ARCHETYPE_DIMENSIONS}

        for gap_type, score in gap_prefs.items():
            if score <= 0:
                continue

            for ad in self._GAP_TYPE_TO_ARCHETYPE.get(gap_type, []):
                dimension_scores[ad] += score

        accepts = profile.accepts or 0

        hypothesizes = profile.hypothesizes or 0

        total = max(event_count, 1)

        if hypothesizes / total > 0.2:
            dimension_scores["exploratory"] += hypothesizes * 0.5

        elif accepts / total > 0.4:
            dimension_scores["confirmatory"] += accepts * 0.3

        topics = profile.topics_explored or []

        if len(set(topics)) > 5:
            dimension_scores["cross_domain"] = min(len(set(topics)) * 0.3, 3.0)

        method_score = gap_prefs.get("method_limitation", 0)

        if method_score > 0.5:
            dimension_scores["high_risk"] = method_score * 2

        elif method_score < -0.2:
            dimension_scores["low_risk"] = abs(method_score) * 2

        max_raw = max(dimension_scores.values()) if any(dimension_scores.values()) else 1.0

        max_raw = max(max_raw, 0.01)

        normalized = {}

        for dim, label, desc in self.ARCHETYPE_DIMENSIONS:
            raw = dimension_scores[dim]

            norm = min(raw / max_raw, 1.0)

            normalized[dim] = (round(raw, 3), round(norm, 2), label, desc)

        dominant = (
            max(dimension_scores, key=dimension_scores.get)
            if any(dimension_scores.values())
            else "method_focused"
        )

        labels = {
            "method_focused": "Method Hunter",
            "app_focused": "Application Pioneer",
            "theory_focused": "Theory Builder",
            "high_risk": "Risk Taker",
            "low_risk": "Steady Researcher",
            "exploratory": "Explorer",
            "confirmatory": "Verifier",
            "cross_domain": "Bridge Builder",
        }

        confidence = min(event_count / 20, 1.0)

        return {
            "dimensions": normalized,
            "dominant": dominant,
            "archetype_label": labels.get(dominant, "Undefined"),
            "confidence": round(confidence, 2),
            "event_count": event_count,
        }

    def render_archetype_radar(self):

        arch = self.get_archetype()

        if arch["event_count"] == 0:
            return "No exploration data yet. Run airos gap <topic> to discover your archetype."

        dims = arch["dimensions"]

        blk = chr(0x2588)

        dim_sym = chr(0x2591)

        lines = []

        lines.append("")

        conf = "{:.0%}".format(arch["confidence"])

        title = "Research Archetype Radar  (confidence {}, {} events)".format(
            conf, arch["event_count"]
        )

        lines.append("  " + chr(0x256D) + chr(0x2500) * 51 + chr(0x256E))

        lines.append(
            "  " + chr(0x2502) + "  " + title + " " * max(0, 51 - len(title)) + "  " + chr(0x2502)
        )

        label = arch["archetype_label"]

        lines.append(
            "  " + chr(0x2502) + "  " + label + " " * max(0, 51 - len(label)) + "  " + chr(0x2502)
        )

        lines.append("  " + chr(0x2570) + chr(0x2500) * 51 + chr(0x256F))

        lines.append("")

        sorted_dims = sorted(dims.items(), key=lambda x: x[1][0], reverse=True)[:4]

        max_label = 14

        for _dim_name, (_raw, norm, label, desc) in sorted_dims:
            bar_len = int(norm * 20)

            bar = blk * bar_len + dim_sym * (20 - bar_len)

            lines.append(
                "  {label:<{w}} {bar} {norm}  ({desc})".format(
                    label=label, w=max_label, bar=bar, norm=norm, desc=desc
                )
            )

        lines.append("")

        lines.append("  All dimensions:")

        for _dim_name, (_raw, norm, label, desc) in dims.items():
            lines.append("  - {}: {} [{}]".format(label, desc, norm))

        lines.append("")

        return "\n".join(lines)

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

        return cast(Dict[str, float], self._get_all_scores_cached()["gap_types"])

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
            action_counts: Dict[str, int] = {}

            for e in recent:
                action_counts[e.action.value] = action_counts.get(e.action.value, 0) + 1

            stats["recent_action_breakdown"] = action_counts

        return stats

    def render_stats(self) -> str:
        """Render exploration statistics overview (WarpBlocks Rich)."""

        from rich.console import Console

        from cli.warp import WarpBlocks

        c = Console()

        stats = self.get_exploration_stats()

        overview_rows = [
            ["Total Events", f"[#A5D5FE]{stats['total_events']}[/]"],
            ["Topics", f"[#A5D5FE]{stats['total_topics']}[/]"],
        ]

        rows_action, rows_gap, rows_topic, rows_tag = [], [], [], []

        for key, _title, _sort_items, limit in [
            ("recent_action_breakdown", "⚡ Actions", False, None),
            ("top_gap_types", "📈 Top Gap Types", False, 5),
            ("topic_frequency", "🔑 Topics", False, 5),
            ("preference_tags", "🏷️ Tags", False, 5),
        ]:
            raw = stats.get(key)

            if not raw:
                continue

            if isinstance(raw, list):
                items = list(raw)[:limit] if limit else list(raw)

            else:
                items = sorted(
                    raw.items(), key=lambda x: -x[1] if isinstance(x[1], (int, float)) else 0
                )[:limit]

            for item in items:
                if isinstance(item, tuple):
                    k, v = item

                    if key == "preference_tags":
                        level = (
                            "[#B4FA72]●[/]"
                            if v >= 0.6
                            else "[#FEFDC2]●[/]"
                            if v >= 0.3
                            else "[#8E8E8E]●[/]"
                        )

                        rows_tag.append([level, k, f"[#A5D5FE]{v:.0%}[/]"])

                    elif key == "topic_frequency":
                        rows_topic.append([k, f"[#A5D5FE]{v}×[/]"])

                    elif key == "recent_action_breakdown":
                        rows_action.append([k, f"[#A5D5FE]{v}[/]"])

                else:
                    rows_gap.append([str(item)[:30]])

        parts = [
            WarpBlocks.panel(
                "[#FF8272]📊 Exploration Statistics[/]",
                f"[#A5D5FE]{stats['total_events']} events[/] · [#A5D5FE]{stats['total_topics']} topics[/]",
                width=65,
            ),
            "",
        ]

        c.print(WarpBlocks.table(["Metric", "Value"], overview_rows))

        c.print()

        if rows_action:
            c.print(WarpBlocks.table(["Action", "Count"], rows_action, title="Recent Actions"))

            c.print()

        if rows_gap:
            c.print(WarpBlocks.table(["Gap Type"], rows_gap, title="Top Gap Types"))

            c.print()

        if rows_topic:
            c.print(WarpBlocks.table(["Topic", "Count"], rows_topic, title="Top Topics"))

            c.print()

        if rows_tag:
            c.print(WarpBlocks.table(["", "Tag", "Score"], rows_tag, title="Preference Tags"))

        return "\n".join(parts)

    def render_profile(self) -> str:
        """Render user preference profile (WarpBlocks Rich)."""

        from rich.console import Console

        from cli.warp import WarpBlocks

        c = Console()

        profile = self._load_profile()

        stats = self.get_exploration_stats()

        # preference_tags

        tag_rows = []

        raw = stats.get("preference_tags")

        if raw:
            if isinstance(raw, list):
                items = list(raw)

            else:
                items = sorted(
                    raw.items(), key=lambda x: -x[1] if isinstance(x[1], (int, float)) else 0
                )

            for item in items:
                if isinstance(item, tuple):
                    k, v = item

                    level = (
                        "[#B4FA72]●[/]"
                        if v >= 0.6
                        else "[#FEFDC2]●[/]"
                        if v >= 0.3
                        else "[#8E8E8E]●[/]"
                    )

                    tag_rows.append([level, k, f"[#A5D5FE]{v:.0%}[/]"])

        # top_gap_types

        gap_rows = []

        raw = stats.get("top_gap_types")

        if raw:
            items = list(raw) if isinstance(raw, list) else list(raw.items())[:5]

            for item in items:
                score = profile.gap_type_preferences.get(
                    str(item) if isinstance(item, str) else item[0], 0
                )

                gap_rows.append(
                    [
                        str(item)[:30] if isinstance(item, str) else item[0][:30],
                        f"[#A5D5FE]{score:.2f}[/]",
                    ]
                )

        # topic_frequency

        topic_rows = []

        raw = stats.get("topic_frequency")

        if raw:
            items = (
                sorted(raw.items(), key=lambda x: -x[1])[:5]
                if isinstance(raw, dict)
                else list(raw)[:5]
            )

            for item in items:
                topic_rows.append([str(item[0])[:35], f"[#A5D5FE]{item[1]}×[/]"])

        parts = [
            WarpBlocks.panel(
                "[#FF8272]🧠 Research Preference Profile[/]",
                f"[#A5D5FE]{stats['total_events']} events[/] · [#A5D5FE]{stats['total_topics']} topics[/]",
                width=65,
            ),
            "",
        ]

        # Capture Rich output using Console.capture()

        with c.capture() as capture:
            if tag_rows:
                c.print(WarpBlocks.table(["", "Tag", "Score"], tag_rows, title="Preference Tags"))

                c.print()

            if gap_rows:
                c.print(WarpBlocks.table(["Gap Type", "Score"], gap_rows, title="Top Gap Types"))

                c.print()

            if topic_rows:
                c.print(WarpBlocks.table(["Topic", "Count"], topic_rows, title="Top Topics"))

        if capture.get():
            parts.append(capture.get().rstrip("\n"))

        return "\n".join(parts)

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

    def _decay_weight(
        self, base_weight: float, event_timestamp: str, lambda_: float = 0.01
    ) -> float:
        """Apply exponential time decay to an event weight.


        Recent events decay slowly; older events contribute less.


        With lambda=0.01: ~73% at 30 days, ~37% at 90 days, ~9% at 1 year.


        """

        try:
            event_time = datetime.fromisoformat(event_timestamp)

            age_days = (datetime.now() - event_time).total_seconds() / 86400.0

            decay = 2.0 ** (-lambda_ * age_days)

            return cast(float, base_weight * decay)

        except (ValueError, TypeError, OSError):
            return 0.0

    def get_gap_type_score(self, gap_type: str) -> float:
        """Get the numeric preference score for a gap type with time decay."""

        scores = cast(Dict[str, float], self._get_all_scores_cached()["gap_types"])

        return scores.get(gap_type, 0.0)

    def get_keyword_score(self, keyword: str) -> float:
        """Get the numeric preference score for a keyword with time decay."""

        kw_scores = cast(Dict[str, float], self._get_all_scores_cached()["keywords"])

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

        events = self._load_events()

        if not events:
            return "暂无探索事件记录"

        running, first_nonzero, first_ts, last_ts = self._compute_running_preferences(events)

        all_gap_types = set(running.keys())

        if not all_gap_types:
            return "暂无 gap_type 偏好记录"

        return self._render_preferences_table(events, running, first_nonzero, first_ts, last_ts)

    def _load_events(self) -> List[EvolutionEvent]:
        """Load and parse all events from the JSONL event log."""

        if not self.events_file.exists():
            return []

        events = []

        with open(self.events_file, encoding="utf-8") as f:
            for line in f:
                line = line.strip()

                if not line:
                    continue

                try:
                    data = json.loads(line)

                    try:
                        action = ExplorationAction(data.get("action", ""))

                    except ValueError:
                        continue  # Skip unknown action types

                    events.append(
                        EvolutionEvent(
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
                        )
                    )

                except (json.JSONDecodeError, KeyError):
                    continue

        return events

    def _compute_running_preferences(self, events: List[EvolutionEvent]):
        """Single-pass computation of running preference scores per gap_type.


        Returns (running, first_nonzero, first_ts, last_ts).


        """

        running: Dict[str, float] = {}

        first_nonzero: Dict[str, float] = {}

        for event in events:
            if event.gap_type:
                w = self._event_weight(event)

                decayed = self._decay_weight(w, event.timestamp)

                new_val = running.get(event.gap_type, 0.0) + decayed

                running[event.gap_type] = new_val

                if new_val != 0.0 and event.gap_type not in first_nonzero:
                    first_nonzero[event.gap_type] = new_val

        first_ts = events[0].timestamp[:10]

        last_ts = events[-1].timestamp[:10]

        return running, first_nonzero, first_ts, last_ts

    def _trend_arrow(self, first: float, cur: float) -> str:
        """Compute trend arrow between initial and current preference values."""

        if cur > first + 0.05:
            return "↑↑"

        elif cur > first + 0.01:
            return "↑ "

        elif cur < first - 0.05:
            return "↓↓"

        elif cur < first - 0.01:
            return "↓ "

        elif abs(cur) < 0.01 and abs(first) < 0.01:
            return "  "

        else:
            return "~ "

    def _render_preferences_table(
        self,
        events: List[EvolutionEvent],
        running: Dict[str, float],
        first_nonzero: Dict[str, float],
        first_ts: str,
        last_ts: str,
    ) -> str:
        """Render the preferences evolution as a WarpBlocks table."""

        from rich.console import Console

        from cli.warp import WarpBlocks

        c = Console()

        active_types = [gt for gt in running if running.get(gt, 0.0) != 0.0]

        if not active_types:
            active_types = sorted(running.keys())

        total_events = len(events)

        rows = []

        for gt in sorted(active_types, key=lambda g: running.get(g, 0.0), reverse=True):
            first_v = first_nonzero.get(gt, 0.0)

            cur_v = running.get(gt, 0.0)

            arrow = self._trend_arrow(first_v, cur_v)

            bar = (
                "[#B4FA72]●[/]"
                if cur_v > 0.1
                else "[#FF5555]●[/]"
                if cur_v < -0.05
                else "[#8E8E8E]●[/]"
            )

            val_str = f"{first_v:>+6.2f}" if first_v != 0.0 else "   —  "

            cur_str = f"{cur_v:>+6.2f}" if cur_v != 0.0 else "   —  "

            rows.append(
                [
                    bar,
                    gt[:25],
                    f"[#A5D5FE]{val_str.strip()}[/]",
                    f"[#A5D5FE]{cur_str.strip()}[/]",
                    arrow,
                ]
            )

        header = WarpBlocks.panel(
            "[#FF8272]📈 Gap Type Preference Evolution[/]",
            f"[#A5D5FE]{total_events} events[/] · [#A5D5FE]{first_ts}[/] → [#A5D5FE]{last_ts}[/]",
            width=70,
        )

        c.print(header)

        c.print()

        c.print(
            WarpBlocks.table(
                ["", "Gap Type", "Initial", "Current", "Trend"],
                rows,
                title=f"Preference Timeline ({len(active_types)} types)",
            )
        )

        c.print()

        c.print(
            WarpBlocks.section(
                "Legend",
                "[#B4FA72]●[/]  positive (preferred)    [#FF5555]●[/]  negative (avoided)    [#8E8E8E]●[/]  neutral",
                "↑↑/↑  growing    ↓↓/↓  declining    ~  stable",
                width=65,
            )
        )

        return ""

    def _event_weight(self, event: EvolutionEvent) -> float:
        """Compute preference weight for a single event for the time-decay cache."""

        weight = self._EVENT_WEIGHTS.get(event.action, 0.0)

        # Gap reject (no hypothesis) gets lighter penalty in cache score too

        if event.action == ExplorationAction.REJECTED and not event.hypothesis_id:
            weight = self._REJECT_NO_HYPOTHESIS_PENALTY

        return weight

    def render_topic_history(self, topic: str) -> str:
        """Render exploration history for a topic (WarpBlocks Rich)."""

        from rich.console import Console

        from cli.warp import WarpBlocks

        c = Console()

        events = self.get_topic_history(topic)

        if not events:
            return WarpBlocks.panel("No History", f"[#8E8E8E]暂无 '{topic}' 的探索记录[/]")

        action_colors = {
            ExplorationAction.VIEWED: "[#A5D5FE]",
            ExplorationAction.ACCEPTED: "[#B4FA72]",
            ExplorationAction.REJECTED: "[#FF5555]",
            ExplorationAction.EXPANDED: "[#FEFDC2]",
            ExplorationAction.HYPOTHESIZED: "[#D0D1FE]",
        }

        action_icons = {
            ExplorationAction.VIEWED: "👁️",
            ExplorationAction.ACCEPTED: "✅",
            ExplorationAction.REJECTED: "❌",
            ExplorationAction.EXPANDED: "📋",
            ExplorationAction.HYPOTHESIZED: "🎯",
        }

        rows = []

        for event in events[-30:]:
            color = action_colors.get(event.action, "")

            icon = action_icons.get(event.action, "•")

            time = event.timestamp[11:16]

            gap_short = event.gap_title[:35] if event.gap_title else "[#8E8E8E]N/A[/]"

            rows.append(
                [
                    f"[#8E8E8E]{time}[/]",
                    f"{color}{icon}[/]",
                    f"{color}{event.action.value}[/]",
                    gap_short,
                ]
            )

        parts = [
            WarpBlocks.panel(
                f"[#FF8272]📖 '{topic}'[/] — Exploration History",
                f"[#A5D5FE]{len(events)} events[/]",
                width=65,
            ),
            "",
        ]

        c.print(
            WarpBlocks.table(
                ["Time", "", "Action", "Gap"], rows, title=f"Last {min(30, len(events))} Events"
            )
        )

        return "\n".join(parts)

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

        # Also export events.jsonl so scores can be rebuilt from it on import.

        # Read-then-write instead of shutil.copy2 to avoid WinError 32 (file locked

        # in append mode).

        if self.events_file.exists():
            try:
                with open(self.events_file, "r", encoding="utf-8") as src:
                    content = src.read()

                dst_events = path.parent / "events.jsonl"

                with open(dst_events, "w", encoding="utf-8") as dst:
                    dst.write(content)

            except Exception:
                # Events export is best-effort — don't fail the profile export.

                pass

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

            # Restore events.jsonl so _get_all_scores_cached() has real data to compute from.

            import shutil

            src_events = path.parent / "events.jsonl"

            if src_events.exists():
                shutil.copy2(src_events, self.events_file)

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
        return sorted(
            self.data_dir.glob("profile_backup_*.json"),
            key=lambda p: p.stat().st_mtime,
            reverse=True,
        )

def get_evolution_tracker() -> EvolutionTracker:
    """Factory: get a shared EvolutionTracker instance."""
    return EvolutionTracker()
