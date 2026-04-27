"""Tests for Insight Evolution Tracker."""
from __future__ import annotations

import json
import tempfile
from pathlib import Path

import pytest

from llm.insight_evolution import (
    EvolutionTracker,
    ExplorationAction,
    EvolutionEvent,
    UserPreferenceProfile,
)


@pytest.fixture
def temp_tracker():
    """Create a tracker with temporary storage."""
    with tempfile.TemporaryDirectory() as tmpdir:
        tracker = EvolutionTracker(data_dir=Path(tmpdir))
        yield tracker


class TestEvolutionTracker:
    """Test EvolutionTracker functionality."""

    def test_record_event(self, temp_tracker):
        """Test recording a basic event."""
        event = temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.VIEWED,
            gap_type="method_limitation",
            gap_title="Scalability issues in RAG",
        )

        assert event.topic == "RAG"
        assert event.action == ExplorationAction.VIEWED
        assert event.gap_type == "method_limitation"
        assert event.timestamp

    def test_record_gap_view(self, temp_tracker):
        """Test recording gap view with convenience method."""
        event = temp_tracker.record_gap_view(
            topic="RAG",
            gap_type="method_limitation",
            gap_title="Scalability issues in RAG",
            duration_seconds=30,
        )

        assert event.action == ExplorationAction.VIEWED
        assert event.duration_seconds == 30

    def test_record_gap_accept(self, temp_tracker):
        """Test recording gap acceptance."""
        event = temp_tracker.record_gap_accept(
            topic="RAG",
            gap_type="method_limitation",
            gap_title="Scalability issues in RAG",
        )

        assert event.action == ExplorationAction.ACCEPTED

    def test_record_gap_reject(self, temp_tracker):
        """Test recording gap rejection."""
        event = temp_tracker.record_gap_reject(
            topic="RAG",
            gap_type="method_limitation",
            gap_title="Scalability issues in RAG",
            reason="Not relevant to my research",
        )

        assert event.action == ExplorationAction.REJECTED
        assert event.notes == "Not relevant to my research"

    def test_record_hypothesis_generated(self, temp_tracker):
        """Test recording hypothesis generation."""
        event = temp_tracker.record_hypothesis_generated(
            topic="RAG",
            gap_type="method_limitation",
            gap_title="Scalability issues in RAG",
            hypothesis_id="hyp_001",
        )

        assert event.action == ExplorationAction.HYPOTHESIZED
        assert event.hypothesis_id == "hyp_001"

    def test_profile_updates_on_event(self, temp_tracker):
        """Test that profile is updated when events are recorded."""
        temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.VIEWED,
            gap_type="method_limitation",
        )
        temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.ACCEPTED,
            gap_type="method_limitation",
        )

        profile = temp_tracker.get_profile()
        assert profile.total_events == 2
        assert profile.views == 1
        assert profile.accepts == 1
        assert "RAG" in profile.topics_explored

    def test_gap_type_preference_scoring(self, temp_tracker):
        """Test that gap type preferences are tracked."""
        # Record multiple events for same gap type
        for _ in range(3):
            temp_tracker.record_event(
                topic="RAG",
                action=ExplorationAction.VIEWED,
                gap_type="method_limitation",
            )
        temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.ACCEPTED,
            gap_type="method_limitation",
        )

        profile = temp_tracker.get_profile()
        # Accept should have higher weight
        assert profile.gap_type_preferences["method_limitation"] > 0

    def test_preference_tags_computation(self, temp_tracker):
        """Test that preference tags are computed from events."""
        # Record exploratory behavior
        for _ in range(5):
            temp_tracker.record_event(
                topic="RAG",
                action=ExplorationAction.ACCEPTED,
                gap_type="unexplored_application",
            )
        temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.HYPOTHESIZED,
            gap_type="unexplored_application",
        )

        profile = temp_tracker.get_profile()
        # Should have exploratory tag due to high accept rate
        assert "exploratory" in profile.preference_tags or \
               "high_risk" in profile.preference_tags or \
               "app_focused" in profile.preference_tags

    def test_get_recent_events(self, temp_tracker):
        """Test retrieving recent events."""
        for i in range(5):
            temp_tracker.record_event(
                topic=f"Topic{i}",
                action=ExplorationAction.VIEWED,
            )

        events = temp_tracker.get_recent_events(limit=3)
        assert len(events) == 3

    def test_get_topic_history(self, temp_tracker):
        """Test retrieving events for a specific topic."""
        temp_tracker.record_event(topic="RAG", action=ExplorationAction.VIEWED)
        temp_tracker.record_event(topic="RAG", action=ExplorationAction.ACCEPTED)
        temp_tracker.record_event(topic="Other", action=ExplorationAction.VIEWED)

        rag_events = temp_tracker.get_topic_history("RAG")
        assert len(rag_events) == 2

    def test_get_preferred_gap_types(self, temp_tracker):
        """Test getting preferred gap types."""
        temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.ACCEPTED,
            gap_type="method_limitation",
        )
        temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.VIEWED,
            gap_type="theoretical_gap",
        )

        preferred = temp_tracker.get_preferred_gap_types(limit=2)
        assert "method_limitation" in preferred

    def test_should_prioritize_gap_type(self, temp_tracker):
        """Test gap type prioritization check."""
        # No history yet
        assert not temp_tracker.should_prioritize_gap_type("method_limitation")

        # Add positive history
        for _ in range(3):
            temp_tracker.record_event(
                topic="RAG",
                action=ExplorationAction.ACCEPTED,
                gap_type="method_limitation",
            )

        assert temp_tracker.should_prioritize_gap_type("method_limitation")

    def test_get_recommended_gap_order(self, temp_tracker):
        """Test reordering gaps based on preferences."""
        # Build some preference
        for _ in range(5):
            temp_tracker.record_event(
                topic="RAG",
                action=ExplorationAction.ACCEPTED,
                gap_type="method_limitation",
            )

        gaps = [
            {"type": "theoretical_gap", "title": "Theory Gap"},
            {"type": "method_limitation", "title": "Method Gap"},
        ]

        reordered = temp_tracker.get_recommended_gap_order(gaps)
        # Method limitation should be first due to preference
        assert reordered[0]["type"] == "method_limitation"

    def test_render_profile(self, temp_tracker):
        """Test profile rendering."""
        temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.VIEWED,
            gap_type="method_limitation",
        )

        profile_text = temp_tracker.render_profile()
        assert "研究偏好画像" in profile_text
        assert "total_events" in profile_text.lower() or "总探索事件" in profile_text

    def test_render_topic_history(self, temp_tracker):
        """Test topic history rendering."""
        temp_tracker.record_event(topic="RAG", action=ExplorationAction.VIEWED)

        history_text = temp_tracker.render_topic_history("RAG")
        assert "RAG" in history_text

    def test_render_topic_history_empty(self, temp_tracker):
        """Test rendering history for unvisited topic."""
        result = temp_tracker.render_topic_history("NeverVisited")
        assert "暂无" in result or "No" in result

    def test_get_exploration_stats(self, temp_tracker):
        """Test getting exploration statistics."""
        temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.VIEWED,
            gap_type="method_limitation",
        )

        stats = temp_tracker.get_exploration_stats()
        assert "total_events" in stats
        assert "recent_events" in stats
        assert stats["total_events"] == 1


class TestExplorationAction:
    """Test ExplorationAction enum."""

    def test_all_actions_defined(self):
        """Test all exploration actions are defined."""
        assert ExplorationAction.VIEWED.value == "viewed"
        assert ExplorationAction.ACCEPTED.value == "accepted"
        assert ExplorationAction.REJECTED.value == "rejected"
        assert ExplorationAction.EXPANDED.value == "expanded"
        assert ExplorationAction.HYPOTHESIZED.value == "hypothesized"


class TestKeywordPreferences:
    """Test keyword preference learning."""

    def test_extract_keywords_basic(self, temp_tracker):
        """Test basic keyword extraction."""
        keywords = temp_tracker._extract_keywords("Scalability issues in retrieval systems")
        assert "scalability" in keywords
        assert "retrieval" in keywords
        assert "systems" in keywords
        # Common stopwords excluded
        assert "the" not in keywords
        assert "in" not in keywords

    def test_extract_keywords_filters_short(self, temp_tracker):
        """Test that keywords < 3 chars are excluded."""
        keywords = temp_tracker._extract_keywords("RAG is great for NLP tasks")
        assert "rag" in keywords  # 3 chars ok
        assert "nlp" in keywords  # 3 chars ok
        assert "is" not in keywords  # 2 chars excluded

    def test_keyword_preferences_updated_on_accept(self, temp_tracker):
        """Test that keyword preferences are updated when gap with matching title is accepted."""
        temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.ACCEPTED,
            gap_type="method_limitation",
            gap_title="Scalability issues in retrieval",
        )

        profile = temp_tracker.get_profile()
        assert "scalability" in profile.keyword_preferences
        assert "retrieval" in profile.keyword_preferences
        # ACCEPTED has weight 0.3, multiplied by 0.5 for keyword
        assert profile.keyword_preferences["scalability"] > 0

    def test_keyword_preferences_updated_on_reject(self, temp_tracker):
        """Test that keyword preferences decrease on reject."""
        temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.REJECTED,
            gap_type="dataset_gap",
            gap_title="Missing domain datasets",
        )

        profile = temp_tracker.get_profile()
        assert "datasets" in profile.keyword_preferences or "domain" in profile.keyword_preferences
        # REJECTED has negative weight
        kw_vals = list(profile.keyword_preferences.values())
        assert any(v < 0 for v in kw_vals)

    def test_get_keyword_score(self, temp_tracker):
        """Test get_keyword_score returns correct value."""
        temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.ACCEPTED,
            gap_type="method_limitation",
            gap_title="Robustness in transformer models",
        )

        score = temp_tracker.get_keyword_score("robustness")
        assert score > 0

        # Unknown keyword returns 0
        unknown = temp_tracker.get_keyword_score("unknownwordxyz")
        assert unknown == 0.0

    def test_get_top_keywords(self, temp_tracker):
        """Test get_top_keywords returns highest-scoring keywords."""
        # Record events with different keywords
        temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.ACCEPTED,
            gap_type="method_limitation",
            gap_title="Scalability analysis",
        )
        temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.ACCEPTED,
            gap_type="unexplored_application",
            gap_title="Scalability study",  # repeat same keyword
        )
        temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.VIEWED,
            gap_type="theoretical_gap",
            gap_title="Theoretical foundations",
        )

        top = temp_tracker.get_top_keywords(limit=3)
        # Scalability should be top (appears twice with ACCEPTED weight)
        assert "scalability" in top

    def test_keyword_preferences_accumulates(self, temp_tracker):
        """Test that keyword preferences accumulate over multiple events."""
        for _ in range(3):
            temp_tracker.record_event(
                topic="RAG",
                action=ExplorationAction.VIEWED,
                gap_type="method_limitation",
                gap_title="Robustness in models",
            )

        profile = temp_tracker.get_profile()
        score = profile.keyword_preferences.get("robustness", 0.0)
        # VIEWED has weight 0.1 * 0.5 = 0.05 per event, 3 events = 0.15
        assert score > 0.1
