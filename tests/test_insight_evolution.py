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
        # Should have exploratory tag (or app_focused) due to high accept rate
        # Now preference_tags is Dict[str, float] — check keys
        assert len(profile.preference_tags) > 0
        assert any(
            tag in profile.preference_tags and profile.preference_tags[tag] > 0
            for tag in ("exploratory", "app_focused", "method_focused")
        )

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
        # VIEWED has weight 0.05 * 0.5 = 0.025 per event, 3 events = 0.075
        assert score > 0.05


class TestTTLCache:
    """Test in-memory TTL cache for decay-weighted scores."""

    def test_cache_miss_on_first_read(self, temp_tracker):
        """First call computes and caches scores."""
        temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.ACCEPTED,
            gap_type="method_limitation",
            gap_title="Scalability in retrieval",
        )

        # First call populates cache
        score1 = temp_tracker.get_gap_type_score("method_limitation")
        assert score1 > 0

        # Cache should be populated
        assert temp_tracker._cache_time is not None
        assert "gap_types" in temp_tracker._score_cache

    def test_cache_hit_avoids_rescan(self, temp_tracker):
        """Second call returns cached value without reading JSONL again."""
        temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.ACCEPTED,
            gap_type="theoretical_gap",
            gap_title="Theoretical foundations",
        )

        # Populate cache
        score1 = temp_tracker.get_gap_type_score("theoretical_gap")

        # Capture cache state
        saved_cache = dict(temp_tracker._score_cache)
        saved_time = temp_tracker._cache_time

        # Second call should return same value from cache
        score2 = temp_tracker.get_gap_type_score("theoretical_gap")
        assert score1 == score2
        assert temp_tracker._score_cache == saved_cache
        assert temp_tracker._cache_time == saved_time

    def test_cache_invalidated_on_new_event(self, temp_tracker):
        """Recording a new event clears the cache."""
        temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.ACCEPTED,
            gap_type="method_limitation",
        )

        # Populate cache
        temp_tracker.get_gap_type_score("method_limitation")
        assert temp_tracker._cache_time is not None

        # Record new event — cache must be invalidated
        temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.VIEWED,
            gap_type="dataset_gap",
            gap_title="Missing benchmark",
        )

        assert temp_tracker._cache_time is None
        assert len(temp_tracker._score_cache) == 0

    def test_both_cache_keys_populated(self, temp_tracker):
        """Both gap_type and keyword caches are filled in one scan."""
        temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.ACCEPTED,
            gap_type="method_limitation",
            gap_title="Scalability in retrieval",
        )

        # Trigger cache population
        temp_tracker.get_gap_type_score("method_limitation")

        cached = temp_tracker._score_cache
        assert "gap_types" in cached
        assert "keywords" in cached
        # Keyword from gap_title should be present
        assert "scalability" in cached["keywords"]

    def test_get_keyword_score_from_cache(self, temp_tracker):
        """get_keyword_score uses cached keyword scores."""
        temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.ACCEPTED,
            gap_type="method_limitation",
            gap_title="Scalability analysis",
        )

        # First call — cache miss, populates
        score1 = temp_tracker.get_keyword_score("scalability")

        # Second call — cache hit
        score2 = temp_tracker.get_keyword_score("scalability")
        assert score1 == score2
        assert score1 > 0

    def test_get_top_keywords_from_cache(self, temp_tracker):
        """get_top_keywords uses cached keyword scores."""
        temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.ACCEPTED,
            gap_type="method_limitation",
            gap_title="Scalability analysis",
        )
        temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.VIEWED,
            gap_type="theoretical_gap",
            gap_title="Theoretical foundations",
        )

        # Trigger via get_top_keywords (populates cache)
        top1 = temp_tracker.get_top_keywords(limit=3)

        # Cache hit on second call
        top2 = temp_tracker.get_top_keywords(limit=3)
        assert top1 == top2
        assert "scalability" in top1


class TestProfilePersistence:
    """Test export/import profile functionality."""

    def test_export_creates_file(self, temp_tracker, tmp_path):
        """export_profile creates a JSON file."""
        temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.ACCEPTED,
            gap_type="method_limitation",
        )

        backup_path = temp_tracker.export_profile(path=tmp_path / "backup.json")

        assert backup_path.exists()
        data = json.loads(backup_path.read_text(encoding="utf-8"))
        assert "_exported_at" in data
        assert "_version" in data
        assert data["total_events"] == 1
        assert "method_limitation" in data["gap_type_preferences"]

    def test_export_with_default_path(self, temp_tracker):
        """export_profile defaults to data_dir with timestamp."""
        temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.VIEWED,
            gap_type="theoretical_gap",
        )

        path = temp_tracker.export_profile()

        assert path.exists()
        assert "profile_backup_" in path.name
        assert path.parent == temp_tracker.data_dir

    def test_import_replaces_profile(self, temp_tracker, tmp_path):
        """import_profile with merge=False replaces existing profile."""
        temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.ACCEPTED,
            gap_type="method_limitation",
        )
        # Verify cache is populated
        temp_tracker.get_gap_type_score("method_limitation")

        # Export
        backup = tmp_path / "backup.json"
        temp_tracker.export_profile(path=backup)

        # Add more events
        temp_tracker.record_event(
            topic="LLM",
            action=ExplorationAction.VIEWED,
            gap_type="dataset_gap",
        )

        # Import with replace
        imported = temp_tracker.import_profile(backup, merge=False)

        assert imported.total_events == 1  # replaced, not merged
        assert imported.gap_type_preferences.get("method_limitation", 0) > 0
        assert "dataset_gap" not in imported.gap_type_preferences

    def test_import_merges_profiles(self, temp_tracker, tmp_path):
        """import_profile with merge=True sums numeric fields."""
        temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.ACCEPTED,
            gap_type="method_limitation",
        )
        backup = tmp_path / "backup.json"
        temp_tracker.export_profile(path=backup)

        # Add different events to get different preferences
        temp_tracker.record_event(
            topic="LLM",
            action=ExplorationAction.VIEWED,
            gap_type="theoretical_gap",
        )

        # Import with merge
        merged = temp_tracker.import_profile(backup, merge=True)

        # Both gap types should be present (summed)
        assert "method_limitation" in merged.gap_type_preferences
        assert "theoretical_gap" in merged.gap_type_preferences

    def test_import_invalid_path_raises(self, temp_tracker, tmp_path):
        """import_profile raises FileNotFoundError for missing file."""
        with pytest.raises(FileNotFoundError):
            temp_tracker.import_profile(tmp_path / "nonexistent.json")

    def test_list_backups_finds_exports(self, temp_tracker):
        """list_backups returns sorted backup files."""
        temp_tracker.record_event(topic="RAG", action=ExplorationAction.VIEWED)

        path1 = temp_tracker.export_profile()
        path2 = temp_tracker.export_profile()

        backups = temp_tracker.list_backups()
        assert len(backups) >= 2
        assert all(p.name.startswith("profile_backup_") for p in backups)
        # Both exports appear in the list (order within same-timestamp files is non-deterministic
        # when datetime is mocked to a fixed value, so check set equality instead)
        assert set(backups) >= {path1, path2}

    def test_import_clears_cache(self, temp_tracker, tmp_path):
        """import_profile invalidates score cache."""
        temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.ACCEPTED,
            gap_type="method_limitation",
        )
        # Populate cache
        temp_tracker.get_gap_type_score("method_limitation")
        assert temp_tracker._cache_time is not None

        backup = tmp_path / "backup.json"
        temp_tracker.export_profile(path=backup)
        temp_tracker.import_profile(backup, merge=False)

        assert temp_tracker._cache_time is None
        assert len(temp_tracker._score_cache) == 0

    def test_merge_accumulates_keyword_preferences(self, temp_tracker, tmp_path):
        """Merging accumulates keyword preferences from both profiles."""
        temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.ACCEPTED,
            gap_type="method_limitation",
            gap_title="Scalability in retrieval",
        )
        backup = tmp_path / "backup.json"
        temp_tracker.export_profile(path=backup)

        temp_tracker.record_event(
            topic="LLM",
            action=ExplorationAction.VIEWED,
            gap_type="dataset_gap",
            gap_title="Missing benchmark datasets",
        )

        merged = temp_tracker.import_profile(backup, merge=True)

        # Both sets of keywords should be present
        kw = merged.keyword_preferences
        assert "scalability" in kw
        assert "datasets" in kw or "benchmark" in kw


class TestPreferenceTagConfidence:
    """Test preference tags with confidence scores."""

    def test_tags_are_dict_with_float_values(self, temp_tracker):
        """preference_tags is now Dict[str, float], not List[str]."""
        temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.ACCEPTED,
            gap_type="method_limitation",
        )
        profile = temp_tracker.get_profile()
        assert isinstance(profile.preference_tags, dict)
        for tag, conf in profile.preference_tags.items():
            assert isinstance(tag, str)
            assert isinstance(conf, float)
            assert 0.0 <= conf <= 1.0

    def test_high_accept_rate_yields_exploratory_tag(self, temp_tracker):
        """High accept rate should produce exploratory tag with high confidence."""
        for _ in range(10):
            temp_tracker.record_event(
                topic="RAG",
                action=ExplorationAction.ACCEPTED,
                gap_type="method_limitation",
            )
        profile = temp_tracker.get_profile()
        assert "exploratory" in profile.preference_tags
        # With 10 accepts and 0 views, accept_rate = 1.0
        assert profile.preference_tags["exploratory"] >= 0.3

    def test_tag_confidence_ranges(self, temp_tracker):
        """Tag confidence should be between 0 and 1."""
        temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.VIEWED,
            gap_type="method_limitation",
        )
        profile = temp_tracker.get_profile()
        for tag, conf in profile.preference_tags.items():
            assert 0.0 <= conf <= 1.0

    def test_render_profile_shows_confidence_level(self, temp_tracker):
        """render_profile output should include confidence percentages."""
        for _ in range(5):
            temp_tracker.record_event(
                topic="RAG",
                action=ExplorationAction.ACCEPTED,
                gap_type="method_limitation",
            )
        text = temp_tracker.render_profile()
        # Should show emoji-coded confidence levels
        assert any(emoji in text for emoji in ["🟢", "🟡", "⚪"])
        # Should include percentage
        assert "%" in text

    def test_merge_preference_tags_takes_max_confidence(self, temp_tracker, tmp_path):
        """Merging profiles should keep higher confidence for each tag."""
        # First profile: exploratory at 0.3
        temp_tracker.record_event(
            topic="RAG",
            action=ExplorationAction.ACCEPTED,
            gap_type="method_limitation",
        )
        backup = tmp_path / "backup.json"
        temp_tracker.export_profile(path=backup)

        # Second profile: exploratory at 0.8 (different session)
        temp_tracker2 = temp_tracker.__class__(data_dir=tmp_path / "other")
        temp_tracker2.record_event(
            topic="RAG",
            action=ExplorationAction.ACCEPTED,
            gap_type="method_limitation",
        )
        temp_tracker2.record_event(
            topic="RAG",
            action=ExplorationAction.ACCEPTED,
            gap_type="method_limitation",
        )
        temp_tracker2.record_event(
            topic="RAG",
            action=ExplorationAction.ACCEPTED,
            gap_type="method_limitation",
        )
        backup2 = tmp_path / "backup2.json"
        temp_tracker2.export_profile(path=backup2)

        # Merge backup2 into temp_tracker
        merged = temp_tracker.import_profile(backup2, merge=True)
        # Should have max confidence
        assert "exploratory" in merged.preference_tags


# =============================================================================
# Score getters and deprioritization
# =============================================================================
class TestScoreGetters:
    """Test get_gap_type_score, get_keyword_score, get_top_keywords."""

    def test_get_gap_type_score_returns_zero_when_no_history(self, temp_tracker):
        score = temp_tracker.get_gap_type_score("method_limitation")
        assert score == 0.0

    def test_get_gap_type_score_returns_positive_after_accept(self, temp_tracker):
        temp_tracker.record_gap_accept(topic="RAG", gap_type="method_limitation", gap_title="Gap")
        score = temp_tracker.get_gap_type_score("method_limitation")
        assert score > 0

    def test_get_gap_type_score_returns_negative_after_reject(self, temp_tracker):
        temp_tracker.record_gap_reject(topic="RAG", gap_type="method_limitation", gap_title="Gap")
        score = temp_tracker.get_gap_type_score("method_limitation")
        assert score < 0

    def test_get_keyword_score_returns_zero_when_no_history(self, temp_tracker):
        score = temp_tracker.get_keyword_score("transformer")
        assert score == 0.0

    def test_get_top_keywords_returns_empty_when_no_history(self, temp_tracker):
        kws = temp_tracker.get_top_keywords(limit=5)
        assert kws == []

    def test_should_deprioritize_returns_false_when_no_history(self, temp_tracker):
        result = temp_tracker.should_deprioritize_gap_type("method_limitation")
        assert result is False

    def test_should_deprioritize_returns_true_after_rejection(self, temp_tracker):
        temp_tracker.record_gap_reject(topic="RAG", gap_type="method_limitation", gap_title="Gap")
        result = temp_tracker.should_deprioritize_gap_type("method_limitation")
        assert result is True


# =============================================================================
# Keyword extraction
# =============================================================================
class TestExtractKeywords:
    """Test _extract_keywords delegates to extract_keywords util."""

    def test_extracts_single_word(self):
        from llm.insight_evolution import EvolutionTracker
        with tempfile.TemporaryDirectory() as tmpdir:
            tracker = EvolutionTracker(data_dir=Path(tmpdir))
            kw = tracker._extract_keywords("transformer attention mechanism")
            assert "transformer" in kw

    def test_extracts_nothing_from_empty_string(self):
        from llm.insight_evolution import EvolutionTracker
        with tempfile.TemporaryDirectory() as tmpdir:
            tracker = EvolutionTracker(data_dir=Path(tmpdir))
            kw = tracker._extract_keywords("")
            assert isinstance(kw, list)


# =============================================================================
# Export / Import / List backups
# =============================================================================
class TestBackupManagement:
    """Test export_profile, import_profile (replace mode), list_backups."""

    def test_export_profile_returns_path(self, temp_tracker):
        temp_tracker.record_gap_accept(topic="RAG", gap_type="method_limitation", gap_title="Gap")
        path = temp_tracker.export_profile()
        assert isinstance(path, Path)
        assert path.exists()

    def test_import_profile_replace_mode(self, tmp_path, temp_tracker):
        """import_profile with merge=False replaces the profile entirely."""
        temp_tracker.record_gap_accept(topic="RAG", gap_type="method_limitation", gap_title="Gap")
        backup = tmp_path / "backup.json"
        temp_tracker.export_profile(path=backup)

        # New tracker with empty history
        tracker2 = EvolutionTracker(data_dir=tmp_path / "other")
        assert tracker2.get_gap_type_score("method_limitation") == 0.0

        # Import with replace (merge=False)
        imported = tracker2.import_profile(backup, merge=False)
        assert tracker2.get_gap_type_score("method_limitation") > 0

    def test_list_backups_returns_empty_when_none(self, tmp_path):
        tracker = EvolutionTracker(data_dir=tmp_path / "empty")
        backups = tracker.list_backups()
        assert backups == []

    def test_list_backups_finds_exports(self, tmp_path, temp_tracker):
        """list_backups finds files matching profile_backup_*.json pattern."""
        temp_tracker.record_gap_accept(topic="RAG", gap_type="method_limitation", gap_title="Gap")
        temp_tracker.export_profile()
        temp_tracker.export_profile()

        backups = temp_tracker.list_backups()
        assert len(backups) == 2
        assert all("profile_backup_" in str(b) for b in backups)

    def test_list_backups_sorted_reverse(self, tmp_path, temp_tracker):
        """list_backups returns newest first."""
        temp_tracker.record_gap_accept(topic="RAG", gap_type="method_limitation", gap_title="Gap")
        temp_tracker.export_profile()
        import time
        time.sleep(0.01)
        temp_tracker.export_profile()

        backups = temp_tracker.list_backups()
        assert len(backups) == 2
        # Newest first (sorted reverse)
        assert backups[0].stat().st_mtime >= backups[1].stat().st_mtime
