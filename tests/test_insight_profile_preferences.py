"""Tests for llm/insight/preferences.py and llm/insight/profile.py."""

import pytest
from llm.insight.preferences import ExplorationAction, PreferenceTag, EvolutionEvent
from llm.insight.profile import UserPreferenceProfile, GapExplorationState


class TestExplorationAction:
    def test_all_actions_have_string_values(self):
        for action in ExplorationAction:
            assert isinstance(action.value, str)
            assert len(action.value) > 0

    def test_action_values_are_snake_case(self):
        for action in ExplorationAction:
            assert action.value == action.value.lower()
            assert "_" not in action.value or action.value.count("_") == action.value.count("_")

    def test_action_count(self):
        assert len(ExplorationAction) == 10

    def test_specific_actions(self):
        assert ExplorationAction.VIEWED.value == "viewed"
        assert ExplorationAction.ACCEPTED.value == "accepted"
        assert ExplorationAction.REJECTED.value == "rejected"
        assert ExplorationAction.EXPANDED.value == "expanded"
        assert ExplorationAction.HYPOTHESIZED.value == "hypothesized"
        assert ExplorationAction.VALIDATED.value == "validated"
        assert ExplorationAction.NARRATED.value == "narrated"
        assert ExplorationAction.INSIGHT_RATED.value == "insight_rated"
        assert ExplorationAction.IMPLEMENTATION_PASS.value == "implementation_pass"
        assert ExplorationAction.IMPLEMENTATION_FAIL.value == "implementation_fail"


class TestPreferenceTag:
    def test_all_tags_have_string_values(self):
        for tag in PreferenceTag:
            assert isinstance(tag.value, str)
            assert len(tag.value) > 0

    def test_tag_count(self):
        assert len(PreferenceTag) == 8

    def test_specific_tags(self):
        assert PreferenceTag.METHOD_FOCUSED.value == "method_focused"
        assert PreferenceTag.APPLICATION_FOCUSED.value == "app_focused"
        assert PreferenceTag.THEORY_FOCUSED.value == "theory_focused"
        assert PreferenceTag.HIGH_RISK_TOLERANT.value == "high_risk"
        assert PreferenceTag.LOW_RISK_TOLERANT.value == "low_risk"
        assert PreferenceTag.EXPLORATORY.value == "exploratory"
        assert PreferenceTag.CONFIRMATORY.value == "confirmatory"
        assert PreferenceTag.CROSS_DOMAIN.value == "cross_domain"


class TestEvolutionEvent:
    def test_minimal_event(self):
        event = EvolutionEvent(timestamp="2024-01-01T00:00:00", topic="test", action=ExplorationAction.VIEWED)
        assert event.timestamp == "2024-01-01T00:00:00"
        assert event.topic == "test"
        assert event.action == ExplorationAction.VIEWED
        assert event.gap_type == ""
        assert event.gap_title == ""
        assert event.gap_description == ""
        assert event.hypothesis_id == ""
        assert event.question_id == ""
        assert event.paper_ids == []
        assert event.duration_seconds == 0
        assert event.notes == ""
        assert event.insight_card_id == ""

    def test_full_event(self):
        event = EvolutionEvent(
            timestamp="2024-01-01T00:00:00",
            topic="machine learning",
            action=ExplorationAction.ACCEPTED,
            gap_type="methodology",
            gap_title="Gap in optimization",
            gap_description="Missing comparison",
            hypothesis_id="hyp_123",
            question_id="q_456",
            paper_ids=["arxiv:1234.5678", "doi:10.1234/foo"],
            duration_seconds=120,
            notes="Interesting finding",
            insight_card_id="card_789",
        )
        assert event.gap_type == "methodology"
        assert event.gap_title == "Gap in optimization"
        assert event.gap_description == "Missing comparison"
        assert event.hypothesis_id == "hyp_123"
        assert event.question_id == "q_456"
        assert len(event.paper_ids) == 2
        assert event.duration_seconds == 120
        assert event.notes == "Interesting finding"
        assert event.insight_card_id == "card_789"



class TestUserPreferenceProfile:
    def test_default_profile(self):
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

    def test_profile_with_counts(self):
        profile = UserPreferenceProfile(
            total_sessions=5,
            total_events=42,
            views=20,
            accepts=10,
            rejects=8,
            expands=3,
            hypothesizes=1,
        )
        assert profile.total_sessions == 5
        assert profile.total_events == 42
        assert profile.views == 20
        assert profile.accepts == 10
        assert profile.rejects == 8
        assert profile.expands == 3
        assert profile.hypothesizes == 1

    def test_profile_with_preferences(self):
        profile = UserPreferenceProfile(
            gap_type_preferences={"methodology": 0.7, "application": 0.3},
            keyword_preferences={"neural": 0.8, "transformer": 0.6},
            topics_explored=["ML", "NLP", "CV"],
            topic_frequency={"ML": 10, "NLP": 5, "CV": 3},
            preference_tags={"method_focused": 0.8, "exploratory": 0.4},
            recent_topics=["ML", "NLP"],
            last_updated="2024-06-01T12:00:00",
        )
        assert profile.gap_type_preferences["methodology"] == 0.7
        assert profile.keyword_preferences["neural"] == 0.8
        assert len(profile.topics_explored) == 3
        assert profile.topic_frequency["ML"] == 10
        assert profile.preference_tags["method_focused"] == 0.8
        assert len(profile.recent_topics) == 2
        assert profile.last_updated == "2024-06-01T12:00:00"


class TestGapExplorationState:
    def test_state_creation(self):
        state = GapExplorationState(
            topic="deep learning",
            session_id="sess_abc123",
            started_at="2024-01-15T10:00:00",
        )
        assert state.topic == "deep learning"
        assert state.session_id == "sess_abc123"
        assert state.started_at == "2024-01-15T10:00:00"
        assert state.events == []
        assert state.gaps_explored == []
        assert state.gaps_accepted == []
        assert state.gaps_rejected == []
        assert state.hypotheses_generated == 0

    def test_state_with_events_and_gaps(self):
        events = [
            EvolutionEvent(
                timestamp="2024-01-15T10:05:00",
                topic="deep learning",
                action=ExplorationAction.VIEWED,
                gap_title="Missing baselines",
            ),
            EvolutionEvent(
                timestamp="2024-01-15T10:10:00",
                topic="deep learning",
                action=ExplorationAction.ACCEPTED,
                gap_title="Missing baselines",
            ),
        ]
        state = GapExplorationState(
            topic="deep learning",
            session_id="sess_abc123",
            started_at="2024-01-15T10:00:00",
            events=events,
            gaps_explored=["Missing baselines", "No ablation study"],
            gaps_accepted=["Missing baselines"],
            gaps_rejected=["No ablation study"],
            hypotheses_generated=3,
        )
        assert len(state.events) == 2
        assert len(state.gaps_explored) == 2
        assert len(state.gaps_accepted) == 1
        assert len(state.gaps_rejected) == 1
        assert state.hypotheses_generated == 3
        assert state.events[0].action == ExplorationAction.VIEWED
        assert state.events[1].action == ExplorationAction.ACCEPTED

    def test_state_requires_topic_and_session(self):
        state = GapExplorationState(topic="AI", session_id="sess_1", started_at="2024-01-01T00:00:00")
        assert state.topic == "AI"
        assert state.session_id == "sess_1"
