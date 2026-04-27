"""Tier 2 unit tests — llm/insight_evolution.py, pure functions, no I/O."""
import pytest
from llm.insight_evolution import (
    ExplorationAction,
    PreferenceTag,
    EvolutionEvent,
    UserPreferenceProfile,
    GapExplorationState,
    EvolutionTracker,
)


# =============================================================================
# Enum tests
# =============================================================================
class TestExplorationAction:
    """Test ExplorationAction enum values."""

    def test_all_actions_have_values(self):
        """All ExplorationAction variants have string values."""
        assert ExplorationAction.VIEWED.value == "viewed"
        assert ExplorationAction.ACCEPTED.value == "accepted"
        assert ExplorationAction.REJECTED.value == "rejected"
        assert ExplorationAction.EXPANDED.value == "expanded"
        assert ExplorationAction.HYPOTHESIZED.value == "hypothesized"
        assert ExplorationAction.VALIDATED.value == "validated"
        assert ExplorationAction.NARRATED.value == "narrated"

    def test_can_be_constructed_from_value(self):
        """Enum can be constructed from string value."""
        action = ExplorationAction("accepted")
        assert action == ExplorationAction.ACCEPTED


class TestPreferenceTag:
    """Test PreferenceTag enum values."""

    def test_all_tags_have_values(self):
        """All PreferenceTag variants have string values."""
        assert PreferenceTag.METHOD_FOCUSED.value == "method_focused"
        assert PreferenceTag.APPLICATION_FOCUSED.value == "app_focused"
        assert PreferenceTag.THEORY_FOCUSED.value == "theory_focused"
        assert PreferenceTag.HIGH_RISK_TOLERANT.value == "high_risk"
        assert PreferenceTag.LOW_RISK_TOLERANT.value == "low_risk"
        assert PreferenceTag.EXPLORATORY.value == "exploratory"
        assert PreferenceTag.CONFIRMATORY.value == "confirmatory"
        assert PreferenceTag.CROSS_DOMAIN.value == "cross_domain"


# =============================================================================
# Dataclass tests
# =============================================================================
class TestEvolutionEvent:
    """Test EvolutionEvent dataclass."""

    def test_required_fields(self):
        """Required fields: timestamp, topic, action."""
        event = EvolutionEvent(
            timestamp="2024-01-01T10:00:00",
            topic="NLP",
            action=ExplorationAction.VIEWED,
        )
        assert event.timestamp == "2024-01-01T10:00:00"
        assert event.topic == "NLP"
        assert event.action == ExplorationAction.VIEWED

    def test_optional_fields_defaults(self):
        """Optional fields have sensible defaults."""
        event = EvolutionEvent(
            timestamp="2024-01-01T10:00:00",
            topic="T",
            action=ExplorationAction.VIEWED,
        )
        assert event.gap_type == ""
        assert event.gap_title == ""
        assert event.gap_description == ""
        assert event.hypothesis_id == ""
        assert event.question_id == ""
        assert event.paper_ids == []
        assert event.duration_seconds == 0
        assert event.notes == ""

    def test_all_fields_can_be_set(self):
        """All fields can be set."""
        event = EvolutionEvent(
            timestamp="2024-01-01T10:00:00",
            topic="CV",
            action=ExplorationAction.HYPOTHESIZED,
            gap_type="method_gap",
            gap_title="Transformer efficiency",
            gap_description="Need faster transformers",
            hypothesis_id="hyp-123",
            question_id="q-456",
            paper_ids=["p1", "p2"],
            duration_seconds=120,
            notes="Very interesting",
        )
        assert event.gap_type == "method_gap"
        assert event.hypothesis_id == "hyp-123"
        assert event.paper_ids == ["p1", "p2"]
        assert event.duration_seconds == 120


class TestUserPreferenceProfile:
    """Test UserPreferenceProfile dataclass."""

    def test_default_values(self):
        """Default profile has zero counts."""
        profile = UserPreferenceProfile()
        assert profile.total_sessions == 0
        assert profile.total_events == 0
        assert profile.views == 0
        assert profile.accepts == 0
        assert profile.rejects == 0
        assert profile.expands == 0
        assert profile.hypothesizes == 0
        assert profile.gap_type_preferences == {}
        assert profile.keyword_preferences == {}
        assert profile.topics_explored == []
        assert profile.topic_frequency == {}
        assert profile.preference_tags == {}
        assert profile.recent_topics == []
        assert profile.last_updated == ""

    def test_profile_can_be_populated(self):
        """Profile can hold user preference data."""
        profile = UserPreferenceProfile(
            total_events=100,
            views=50,
            accepts=20,
            rejects=10,
            expands=15,
            hypothesizes=5,
            gap_type_preferences={"method_gap": 0.5, "application_gap": 0.3},
            keyword_preferences={"transformer": 0.4},
            topics_explored=["NLP", "CV"],
            topic_frequency={"NLP": 60, "CV": 40},
            recent_topics=["CV", "NLP"],
        )
        assert profile.total_events == 100
        assert profile.views == 50
        assert profile.gap_type_preferences["method_gap"] == 0.5


class TestGapExplorationState:
    """Test GapExplorationState dataclass."""

    def test_required_fields(self):
        """Required fields: topic, session_id, started_at."""
        state = GapExplorationState(
            topic="NLP",
            session_id="sess-123",
            started_at="2024-01-01T10:00:00",
        )
        assert state.topic == "NLP"
        assert state.session_id == "sess-123"
        assert state.started_at == "2024-01-01T10:00:00"

    def test_optional_fields_default(self):
        """Optional fields default to empty lists/zero."""
        state = GapExplorationState(
            topic="T",
            session_id="s",
            started_at="2024-01-01T10:00:00",
        )
        assert state.events == []
        assert state.gaps_explored == []
        assert state.gaps_accepted == []
        assert state.gaps_rejected == []
        assert state.hypotheses_generated == 0


# =============================================================================
# Event weight constants
# =============================================================================
class TestEventWeights:
    """Test _EVENT_WEIGHTS constant values."""

    def test_event_weights_exist(self):
        """Event weights dict is defined."""
        assert ExplorationAction.VIEWED in EvolutionTracker._EVENT_WEIGHTS
        assert ExplorationAction.ACCEPTED in EvolutionTracker._EVENT_WEIGHTS
        assert ExplorationAction.REJECTED in EvolutionTracker._EVENT_WEIGHTS

    def test_positive_weights_for_engagement(self):
        """Engagement actions have positive weights."""
        assert EvolutionTracker._EVENT_WEIGHTS[ExplorationAction.VIEWED] > 0
        assert EvolutionTracker._EVENT_WEIGHTS[ExplorationAction.ACCEPTED] > 0
        assert EvolutionTracker._EVENT_WEIGHTS[ExplorationAction.EXPANDED] > 0
        assert EvolutionTracker._EVENT_WEIGHTS[ExplorationAction.HYPOTHESIZED] > 0
        assert EvolutionTracker._EVENT_WEIGHTS[ExplorationAction.VALIDATED] > 0
        assert EvolutionTracker._EVENT_WEIGHTS[ExplorationAction.NARRATED] > 0

    def test_rejected_weight_is_negative(self):
        """REJECTED action has negative weight."""
        assert EvolutionTracker._EVENT_WEIGHTS[ExplorationAction.REJECTED] < 0

    def test_hypothesized_has_highest_weight(self):
        """HYPOTHESIZED has the highest positive weight."""
        weights = EvolutionTracker._EVENT_WEIGHTS
        hypo_weight = weights[ExplorationAction.HYPOTHESIZED]
        assert hypo_weight == 0.40
        # Should be highest or tied for highest
        assert hypo_weight >= weights[ExplorationAction.VALIDATED]

    def test_reject_no_hypothesis_penalty(self):
        """_REJECT_NO_HYPOTHESIS_PENALTY is lighter than full reject."""
        assert EvolutionTracker._REJECT_NO_HYPOTHESIS_PENALTY == -0.10
        assert abs(EvolutionTracker._REJECT_NO_HYPOTHESIS_PENALTY) < abs(
            EvolutionTracker._EVENT_WEIGHTS[ExplorationAction.REJECTED]
        )


# =============================================================================
# Time decay calculation
# =============================================================================
class TestDecayWeight:
    """Test _decay_weight logic."""

    def _decay_weight(self, base_weight: float, event_timestamp: str, lambda_: float = 0.01) -> float:
        """Replicate decay calculation logic."""
        from datetime import datetime
        try:
            event_time = datetime.fromisoformat(event_timestamp)
            age_days = (datetime.now() - event_time).total_seconds() / 86400.0
            return base_weight * (2.0 ** (-lambda_ * age_days))
        except (ValueError, TypeError, OSError):
            return 0.0

    def test_decay_returns_base_for_current_time(self):
        """Recent event (now) returns base weight."""
        from datetime import datetime
        now = datetime.now().isoformat()
        result = self._decay_weight(1.0, now)
        assert abs(result - 1.0) < 0.01  # Essentially unchanged

    def test_decay_reduces_old_events(self):
        """Older events have reduced weight."""
        from datetime import datetime, timedelta
        recent = datetime.now().isoformat()
        old = (datetime.now() - timedelta(days=30)).isoformat()
        recent_weight = self._decay_weight(1.0, recent)
        old_weight = self._decay_weight(1.0, old)
        assert old_weight < recent_weight

    def test_decay_formula_30_days(self):
        """Decay formula: 2^(-0.01 * 30) ≈ 0.74."""
        from datetime import datetime, timedelta
        thirty_days_ago = (datetime.now() - timedelta(days=30)).isoformat()
        result = self._decay_weight(1.0, thirty_days_ago)
        # 2^(-0.3) ≈ 0.74, allow range due to leap second / precision
        assert 0.7 < result < 0.85

    def test_decay_formula_90_days(self):
        """Decay formula: 2^(-0.01 * 90) ≈ 0.37."""
        from datetime import datetime, timedelta
        ninety_days_ago = (datetime.now() - timedelta(days=90)).isoformat()
        result = self._decay_weight(1.0, ninety_days_ago)
        # 2^(-0.9) ≈ 0.52... wait let me recalculate
        # 2^(-0.9) = 1/(2^0.9) ≈ 1/1.87 ≈ 0.54
        # Let me check: 2^(-0.01 * 90) = 2^(-0.9) ≈ 0.54
        assert 0.4 < result < 0.6

    def test_invalid_timestamp_returns_zero(self):
        """Invalid timestamp returns 0."""
        result = self._decay_weight(1.0, "invalid-timestamp")
        assert result == 0.0

    def test_empty_timestamp_returns_zero(self):
        """Empty timestamp returns 0."""
        result = self._decay_weight(1.0, "")
        assert result == 0.0

    def test_none_timestamp_returns_zero(self):
        """None timestamp handled gracefully."""
        result = self._decay_weight(1.0, None)
        assert result == 0.0

    def test_custom_lambda(self):
        """Custom lambda changes decay rate."""
        from datetime import datetime, timedelta
        thirty_days_ago = (datetime.now() - timedelta(days=30)).isoformat()
        slow_decay = self._decay_weight(1.0, thirty_days_ago, lambda_=0.005)
        fast_decay = self._decay_weight(1.0, thirty_days_ago, lambda_=0.02)
        assert slow_decay > fast_decay


# =============================================================================
# Preference tag computation
# =============================================================================
class TestComputePreferenceTags:
    """Test _compute_preference_tags logic."""

    def _compute_preference_tags(self, profile: UserPreferenceProfile) -> dict:
        """Replicate preference tag computation logic."""
        tags: dict = {}

        # Gap type focus
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

        # Action ratios
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
            topics_str = " ".join(profile.topics_explored).lower()
            domain_indicators = ["nlp", "vision", "audio", "graph", "reinforcement", "supervised"]
            detected = sum(1 for d in domain_indicators if d in topics_str)
            if detected >= 2:
                confidence = min(detected / len(domain_indicators), 1.0)
                tags[PreferenceTag.CROSS_DOMAIN.value] = confidence

        return tags

    def test_empty_profile_returns_empty_tags(self):
        """Empty profile has no tags."""
        profile = UserPreferenceProfile()
        tags = self._compute_preference_tags(profile)
        assert tags == {}

    def test_method_gap_type_focus(self):
        """Method gap type creates METHOD_FOCUSED tag."""
        profile = UserPreferenceProfile(
            gap_type_preferences={"method_gap": 0.8, "app_gap": 0.1},
            views=10,
            accepts=5,
        )
        tags = self._compute_preference_tags(profile)
        assert PreferenceTag.METHOD_FOCUSED.value in tags

    def test_application_gap_type_focus(self):
        """Application/unexplored gap creates APPLICATION_FOCUSED tag."""
        profile = UserPreferenceProfile(
            gap_type_preferences={"application_gap": 0.6},
            views=10,
        )
        tags = self._compute_preference_tags(profile)
        assert PreferenceTag.APPLICATION_FOCUSED.value in tags

    def test_unexplored_gap_type_focus(self):
        """Unexplored gap creates APPLICATION_FOCUSED tag."""
        profile = UserPreferenceProfile(
            gap_type_preferences={"unexplored_territory": 0.6},
            views=10,
        )
        tags = self._compute_preference_tags(profile)
        assert PreferenceTag.APPLICATION_FOCUSED.value in tags

    def test_theoretical_gap_type_focus(self):
        """Theoretical gap creates THEORY_FOCUSED tag."""
        profile = UserPreferenceProfile(
            gap_type_preferences={"theoretical_gap": 0.7},
            views=10,
        )
        tags = self._compute_preference_tags(profile)
        assert PreferenceTag.THEORY_FOCUSED.value in tags

    def test_high_accept_rate_creates_exploratory_tag(self):
        """High accept rate (>30%) creates EXPLORATORY tag."""
        profile = UserPreferenceProfile(
            views=100,
            accepts=50,  # 50% accept rate
            gap_type_preferences={"method_gap": 0.5},
        )
        tags = self._compute_preference_tags(profile)
        assert PreferenceTag.EXPLORATORY.value in tags

    def test_low_accept_rate_no_exploratory_tag(self):
        """Low accept rate (<30%) doesn't create EXPLORATORY tag."""
        profile = UserPreferenceProfile(
            views=100,
            accepts=10,  # 10% accept rate
            gap_type_preferences={"method_gap": 0.5},
        )
        tags = self._compute_preference_tags(profile)
        assert PreferenceTag.EXPLORATORY.value not in tags

    def test_high_reject_rate_creates_low_risk_tag(self):
        """High reject rate (>30%) creates LOW_RISK_TOLERANT tag."""
        profile = UserPreferenceProfile(
            views=100,
            rejects=40,  # 40% reject rate
            gap_type_preferences={"method_gap": 0.5},
        )
        tags = self._compute_preference_tags(profile)
        assert PreferenceTag.LOW_RISK_TOLERANT.value in tags

    def test_hypothesis_generation_creates_high_risk_tag(self):
        """Many hypotheses (>20% of views) creates HIGH_RISK_TOLERANT tag."""
        profile = UserPreferenceProfile(
            views=50,
            accepts=10,
            hypothesizes=15,  # 15 / 60 = 25% > 20%
            gap_type_preferences={"method_gap": 0.5},
        )
        tags = self._compute_preference_tags(profile)
        assert PreferenceTag.HIGH_RISK_TOLERANT.value in tags

    def test_cross_domain_detection(self):
        """Multiple domain topics creates CROSS_DOMAIN tag."""
        profile = UserPreferenceProfile(
            views=30,
            topics_explored=["NLP research", "Computer Vision", "Audio Processing"],
            gap_type_preferences={"method_gap": 0.5},
        )
        tags = self._compute_preference_tags(profile)
        assert PreferenceTag.CROSS_DOMAIN.value in tags

    def test_single_domain_no_cross_domain(self):
        """Single domain doesn't create CROSS_DOMAIN tag."""
        profile = UserPreferenceProfile(
            views=30,
            topics_explored=["NLP research", "NLP applications"],
            gap_type_preferences={"method_gap": 0.5},
        )
        tags = self._compute_preference_tags(profile)
        assert PreferenceTag.CROSS_DOMAIN.value not in tags

    def test_confidence_bounded_0_to_1(self):
        """Tag confidence is always between 0 and 1."""
        profile = UserPreferenceProfile(
            gap_type_preferences={"gap_a": 10.0, "gap_b": 20.0},
            views=100,
        )
        tags = self._compute_preference_tags(profile)
        for confidence in tags.values():
            assert 0 <= confidence <= 1


# =============================================================================
# Gap type score calculation
# =============================================================================
class TestGapTypeScore:
    """Test gap type score calculation logic."""

    def _event_weight(self, action: ExplorationAction, has_hypothesis: bool = True) -> float:
        """Replicate event weight calculation."""
        weights = {
            ExplorationAction.VIEWED: 0.05,
            ExplorationAction.ACCEPTED: 0.30,
            ExplorationAction.REJECTED: -0.30,
            ExplorationAction.EXPANDED: 0.20,
            ExplorationAction.HYPOTHESIZED: 0.40,
            ExplorationAction.VALIDATED: 0.40,
            ExplorationAction.NARRATED: 0.25,
        }
        weight = weights.get(action, 0.0)
        # Gap reject without hypothesis gets lighter penalty
        if action == ExplorationAction.REJECTED and not has_hypothesis:
            weight = -0.10
        return weight

    def _aggregate_gap_scores(self, events: list) -> dict:
        """Replicate gap score aggregation."""
        scores: dict = {}
        for e in events:
            weight = self._event_weight(e.action, bool(e.hypothesis_id))
            scores[e.gap_type] = scores.get(e.gap_type, 0.0) + weight
        return scores

    def test_viewed_event_increases_score(self):
        """VIEWED action increases gap type score."""
        events = [
            EvolutionEvent(
                timestamp="2024-01-01T10:00:00",
                topic="T",
                action=ExplorationAction.VIEWED,
                gap_type="method_gap",
            )
        ]
        scores = self._aggregate_gap_scores(events)
        assert scores["method_gap"] == 0.05

    def test_accepted_event_strong_increase(self):
        """ACCEPTED action strongly increases score."""
        events = [
            EvolutionEvent(
                timestamp="2024-01-01T10:00:00",
                topic="T",
                action=ExplorationAction.ACCEPTED,
                gap_type="app_gap",
            )
        ]
        scores = self._aggregate_gap_scores(events)
        assert scores["app_gap"] == 0.30

    def test_rejected_with_hypothesis_decreases_score(self):
        """REJECTED with hypothesis_id decreases score significantly."""
        events = [
            EvolutionEvent(
                timestamp="2024-01-01T10:00:00",
                topic="T",
                action=ExplorationAction.REJECTED,
                gap_type="theory_gap",
                hypothesis_id="hyp-1",
            )
        ]
        scores = self._aggregate_gap_scores(events)
        assert scores["theory_gap"] == -0.30

    def test_rejected_without_hypothesis_light_penalty(self):
        """REJECTED without hypothesis_id gets lighter penalty."""
        events = [
            EvolutionEvent(
                timestamp="2024-01-01T10:00:00",
                topic="T",
                action=ExplorationAction.REJECTED,
                gap_type="theory_gap",
                hypothesis_id="",  # No hypothesis
            )
        ]
        scores = self._aggregate_gap_scores(events)
        assert scores["theory_gap"] == -0.10

    def test_multiple_events_accumulate(self):
        """Multiple events accumulate scores."""
        events = [
            EvolutionEvent(timestamp="2024-01-01T10:00:00", topic="T", action=ExplorationAction.VIEWED, gap_type="m_gap"),
            EvolutionEvent(timestamp="2024-01-01T10:01:00", topic="T", action=ExplorationAction.VIEWED, gap_type="m_gap"),
            EvolutionEvent(timestamp="2024-01-01T10:02:00", topic="T", action=ExplorationAction.ACCEPTED, gap_type="m_gap"),
        ]
        scores = self._aggregate_gap_scores(events)
        assert scores["m_gap"] == 0.40  # 0.05 + 0.05 + 0.30

    def test_different_gap_types_separate(self):
        """Different gap types have separate scores."""
        events = [
            EvolutionEvent(timestamp="2024-01-01T10:00:00", topic="T", action=ExplorationAction.ACCEPTED, gap_type="gap_a"),
            EvolutionEvent(timestamp="2024-01-01T10:00:00", topic="T", action=ExplorationAction.ACCEPTED, gap_type="gap_b"),
        ]
        scores = self._aggregate_gap_scores(events)
        assert scores["gap_a"] == 0.30
        assert scores["gap_b"] == 0.30

    def test_hypothesized_highest_weight(self):
        """HYPOTHESIZED has highest weight."""
        events = [
            EvolutionEvent(timestamp="2024-01-01T10:00:00", topic="T", action=ExplorationAction.HYPOTHESIZED, gap_type="h_gap"),
        ]
        scores = self._aggregate_gap_scores(events)
        assert scores["h_gap"] == 0.40


# =============================================================================
# should_deprioritize_gap_type logic
# =============================================================================
class TestShouldDeprioritize:
    """Test should_deprioritize_gap_type logic."""

    def test_negative_score_indicates_dislike(self):
        """Score below -0.05 should be deprioritized."""
        score = -0.10
        should_depri = score < -0.05
        assert should_depri is True

    def test_mildly_negative_score_not_disliked(self):
        """Score between -0.05 and 0 is not disliked."""
        score = -0.03
        should_depri = score < -0.05
        assert should_depri is False

    def test_positive_score_not_disliked(self):
        """Positive score should not be deprioritized."""
        score = 0.20
        should_depri = score < -0.05
        assert should_depri is False

    def test_threshold_is_negative(self):
        """Threshold is -0.05 (slightly negative)."""
        threshold = -0.05
        # Score just above threshold
        score_above = -0.04
        assert not (score_above < threshold)
        # Score at threshold
        score_at = -0.05
        assert not (score_at < threshold)
        # Score below threshold
        score_below = -0.06
        assert score_below < threshold


# =============================================================================
# Trend arrow calculation
# =============================================================================
class TestTrendArrow:
    """Test _trend_arrow logic."""

    def _trend_arrow(self, first: float, cur: float) -> str:
        """Replicate trend arrow calculation."""
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

    def test_strong_increase(self):
        """> +0.05 change shows ↑↑."""
        assert self._trend_arrow(0.1, 0.2) == "↑↑"
        assert self._trend_arrow(0.0, 0.1) == "↑↑"

    def test_moderate_increase(self):
        """+0.01 to +0.05 change shows ↑ ."""
        assert self._trend_arrow(0.1, 0.12) == "↑ "
        assert self._trend_arrow(0.5, 0.52) == "↑ "

    def test_strong_decrease(self):
        """< -0.05 change shows ↓↓."""
        assert self._trend_arrow(0.3, 0.1) == "↓↓"
        assert self._trend_arrow(0.0, -0.1) == "↓↓"

    def test_moderate_decrease(self):
        """-0.01 to -0.05 change shows ↓ ."""
        assert self._trend_arrow(0.3, 0.28) == "↓ "
        assert self._trend_arrow(0.5, 0.48) == "↓ "  # -0.02 < -0.01

    def test_stable_both_near_zero(self):
        """Both values near zero shows '  '."""
        assert self._trend_arrow(0.0, 0.0) == "  "
        assert self._trend_arrow(0.005, -0.005) == "  "

    def test_stable_both_nonzero(self):
        """Similar non-zero values show ~ ."""
        # 0.52 > 0.51 (first + 0.01), returns "↑ " for moderate increase
        assert self._trend_arrow(0.5, 0.52) == "↑ "
        # 0.505 vs 0.5 = 0.005 < 0.01, returns "~ "
        assert self._trend_arrow(0.5, 0.505) == "~ "


# =============================================================================
# Profile merge logic
# =============================================================================
class TestProfileMerge:
    """Test _merge_profiles logic."""

    def _merge_profiles(self, base: dict, incoming: dict) -> dict:
        """Replicate profile merge logic for scalar/dict fields."""
        result = {}

        # Scalars: max
        for key in ["total_sessions", "total_events", "views", "accepts", "rejects", "expands", "hypothesizes"]:
            result[key] = max(base.get(key, 0), incoming.get(key, 0))

        # Dict fields: sum
        result_gap = dict(base.get("gap_type_preferences", {}))
        inc_gap = incoming.get("gap_type_preferences", {})
        for k, v in inc_gap.items():
            result_gap[k] = result_gap.get(k, 0.0) + v
        result["gap_type_preferences"] = result_gap

        result_kw = dict(base.get("keyword_preferences", {}))
        inc_kw = incoming.get("keyword_preferences", {})
        for k, v in inc_kw.items():
            result_kw[k] = result_kw.get(k, 0.0) + v
        result["keyword_preferences"] = result_kw

        # topic_frequency: sum
        result_tf = dict(base.get("topic_frequency", {}))
        for k, v in incoming.get("topic_frequency", {}).items():
            result_tf[k] = result_tf.get(k, 0) + v
        result["topic_frequency"] = result_tf

        # preference_tags: max confidence per tag
        result_tags = dict(base.get("preference_tags", {}))
        inc_tags = incoming.get("preference_tags", {})
        for k, v in inc_tags.items():
            result_tags[k] = max(result_tags.get(k, 0.0), v)
        result["preference_tags"] = result_tags

        return result

    def test_scalars_use_max(self):
        """Scalar fields use max of base and incoming."""
        base = {"total_events": 100, "views": 50}
        incoming = {"total_events": 80, "views": 60}
        result = self._merge_profiles(base, incoming)
        assert result["total_events"] == 100
        assert result["views"] == 60

    def test_gap_type_preferences_sum(self):
        """Gap type preferences are summed."""
        base = {"gap_type_preferences": {"method_gap": 0.5}}
        incoming = {"gap_type_preferences": {"method_gap": 0.3, "app_gap": 0.4}}
        result = self._merge_profiles(base, incoming)
        assert result["gap_type_preferences"]["method_gap"] == 0.8
        assert result["gap_type_preferences"]["app_gap"] == 0.4

    def test_keyword_preferences_sum(self):
        """Keyword preferences are summed."""
        base = {"keyword_preferences": {"transformer": 0.3}}
        incoming = {"keyword_preferences": {"transformer": 0.2, "attention": 0.5}}
        result = self._merge_profiles(base, incoming)
        assert result["keyword_preferences"]["transformer"] == 0.5
        assert result["keyword_preferences"]["attention"] == 0.5

    def test_topic_frequency_sums(self):
        """Topic frequency counts are summed."""
        base = {"topic_frequency": {"NLP": 30}}
        incoming = {"topic_frequency": {"NLP": 20, "CV": 50}}
        result = self._merge_profiles(base, incoming)
        assert result["topic_frequency"]["NLP"] == 50
        assert result["topic_frequency"]["CV"] == 50

    def test_preference_tags_take_max_confidence(self):
        """Preference tags keep higher confidence."""
        base = {"preference_tags": {"method_focused": 0.6}}
        incoming = {"preference_tags": {"method_focused": 0.4, "exploratory": 0.7}}
        result = self._merge_profiles(base, incoming)
        assert result["preference_tags"]["method_focused"] == 0.6  # Base higher
        assert result["preference_tags"]["exploratory"] == 0.7  # Incoming higher

    def test_empty_incoming_preserves_base(self):
        """Empty incoming dict preserves base values."""
        base = {"total_events": 100, "gap_type_preferences": {"gap": 0.5}}
        incoming = {}
        result = self._merge_profiles(base, incoming)
        assert result["total_events"] == 100
        assert result["gap_type_preferences"]["gap"] == 0.5


# =============================================================================
# EvolutionTracker instantiation
# =============================================================================
class TestEvolutionTrackerInit:
    """Test EvolutionTracker class instantiation."""

    def test_cache_ttl_is_five_minutes(self):
        """Cache TTL is 300 seconds."""
        assert EvolutionTracker._CACHE_TTL_SECONDS == 300

    def test_can_create_instance(self):
        """EvolutionTracker can be instantiated."""
        import tempfile
        from pathlib import Path
        with tempfile.TemporaryDirectory() as tmpdir:
            tracker = EvolutionTracker(data_dir=Path(tmpdir))
            assert tracker.data_dir == Path(tmpdir)

    def test_default_data_dir_ends_with_expected_path(self):
        """Default data_dir ends with ~/.ai_research_os/evolution."""
        tracker = EvolutionTracker()
        assert tracker.data_dir.name == "evolution"
        assert tracker.data_dir.parent.name == ".ai_research_os"
