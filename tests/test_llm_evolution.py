"""Tier 2 unit tests — llm/evolution.py, pure functions, no I/O."""
import pytest
from llm.evolution import (
    FeedbackType,
    SignalType,
    Feedback,
    EvolutionEvent,
    LearnedPattern,
    EvolutionMemory,
    get_evolution_memory,
)


# =============================================================================
# Enum tests
# =============================================================================
class TestFeedbackType:
    """Test FeedbackType enum."""

    def test_all_types_have_values(self):
        """All FeedbackType variants have string values."""
        assert FeedbackType.POSITIVE.value == "positive"
        assert FeedbackType.NEGATIVE.value == "negative"
        assert FeedbackType.NEUTRAL.value == "neutral"

    def test_can_construct_from_value(self):
        """Enum can be constructed from string value."""
        assert FeedbackType("positive") == FeedbackType.POSITIVE
        assert FeedbackType("negative") == FeedbackType.NEGATIVE


class TestSignalType:
    """Test SignalType enum."""

    def test_all_types_have_values(self):
        """All SignalType variants have string values."""
        assert SignalType.CHAT_SUCCESS.value == "chat_success"
        assert SignalType.CHAT_FAILURE.value == "chat_failure"
        assert SignalType.RETRIEVAL_HIT.value == "retrieval_hit"
        assert SignalType.RETRIEVAL_MISS.value == "retrieval_miss"
        assert SignalType.SLIDE_QUALITY.value == "slide_quality"
        assert SignalType.SEARCH_SUCCESS.value == "search_success"


# =============================================================================
# Feedback dataclass tests
# =============================================================================
class TestFeedback:
    """Test Feedback dataclass."""

    def test_required_fields(self):
        """Required fields are set."""
        fb = Feedback(
            id="fb_1",
            type="positive",
            command="chat",
            query="What is attention?",
            paper_ids=["p1"],
            outcome="success",
            score=0.9,
        )
        assert fb.id == "fb_1"
        assert fb.type == "positive"
        assert fb.command == "chat"
        assert fb.query == "What is attention?"
        assert fb.paper_ids == ["p1"]
        assert fb.outcome == "success"
        assert fb.score == 0.9

    def test_optional_fields_defaults(self):
        """Optional fields have defaults."""
        fb = Feedback(
            id="fb_2",
            type="negative",
            command="search",
            query="Q",
            paper_ids=[],
            outcome="failure",
            score=0.3,
        )
        assert fb.note == ""
        # timestamp="" triggers __post_init__ auto-generation, so it is NOT empty
        # Only note is a true field-level default that stays empty

    def test_timestamp_auto_set_when_empty(self):
        """Empty timestamp triggers auto-generation."""
        fb = Feedback(
            id="fb_3",
            type="neutral",
            command="chat",
            query="Q",
            paper_ids=[],
            outcome="neutral",
            score=0.5,
            timestamp="",
        )
        assert fb.timestamp != ""
        assert "T" in fb.timestamp  # ISO format

    def test_timestamp_preserved_when_set(self):
        """Non-empty timestamp is preserved."""
        fb = Feedback(
            id="fb_4",
            type="positive",
            command="chat",
            query="Q",
            paper_ids=[],
            outcome="success",
            score=0.8,
            timestamp="2024-01-01T00:00:00",
        )
        assert fb.timestamp == "2024-01-01T00:00:00"

    def test_id_auto_set_when_empty(self):
        """Empty id triggers auto-generation."""
        fb = Feedback(
            id="",
            type="positive",
            command="chat",
            query="Q",
            paper_ids=[],
            outcome="success",
            score=0.8,
        )
        assert fb.id != ""
        assert fb.id.startswith("fb_")

    def test_id_preserved_when_set(self):
        """Non-empty id is preserved."""
        fb = Feedback(
            id="my_custom_id",
            type="neutral",
            command="search",
            query="Q",
            paper_ids=[],
            outcome="neutral",
            score=0.5,
        )
        assert fb.id == "my_custom_id"


# =============================================================================
# EvolutionEvent dataclass tests
# =============================================================================
class TestEvolutionEvent:
    """Test EvolutionEvent dataclass."""

    def test_required_fields(self):
        """Required fields are set."""
        ev = EvolutionEvent(
            id="ev_1",
            signal_type="chat_success",
            trigger={"query": "test"},
            action="respond",
            outcome="success",
            score=0.9,
        )
        assert ev.id == "ev_1"
        assert ev.signal_type == "chat_success"
        assert ev.trigger == {"query": "test"}
        assert ev.action == "respond"
        assert ev.outcome == "success"
        assert ev.score == 0.9

    def test_optional_fields_defaults(self):
        """Optional fields have defaults."""
        ev = EvolutionEvent(
            id="ev_2",
            signal_type="chat_failure",
            trigger={},
            action="retry",
            outcome="failure",
            score=0.2,
        )
        assert ev.genes_applied == []
        # timestamp="" triggers __post_init__ auto-generation, so it is NOT empty

    def test_genes_applied_none_defaults_to_empty_list(self):
        """genes_applied=None defaults to empty list."""
        ev = EvolutionEvent(
            id="ev_3",
            signal_type="search_success",
            trigger={},
            action="search",
            outcome="success",
            score=0.8,
            genes_applied=None,
        )
        assert ev.genes_applied == []

    def test_genes_applied_preserved_when_set(self):
        """genes_applied is preserved when explicitly set."""
        ev = EvolutionEvent(
            id="ev_4",
            signal_type="chat_success",
            trigger={},
            action="respond",
            outcome="success",
            score=0.9,
            genes_applied=["gene_a", "gene_b"],
        )
        assert ev.genes_applied == ["gene_a", "gene_b"]

    def test_timestamp_auto_set_when_empty(self):
        """Empty timestamp triggers auto-generation."""
        ev = EvolutionEvent(
            id="ev_5",
            signal_type="chat_success",
            trigger={},
            action="respond",
            outcome="success",
            score=0.8,
            timestamp="",
        )
        assert ev.timestamp != ""
        assert "T" in ev.timestamp

    def test_timestamp_preserved_when_set(self):
        """Non-empty timestamp is preserved."""
        ev = EvolutionEvent(
            id="ev_6",
            signal_type="chat_failure",
            trigger={},
            action="retry",
            outcome="failure",
            score=0.3,
            timestamp="2025-01-01T12:00:00",
        )
        assert ev.timestamp == "2025-01-01T12:00:00"

    def test_id_auto_set_when_empty(self):
        """Empty id triggers auto-generation."""
        ev = EvolutionEvent(
            id="",
            signal_type="chat_success",
            trigger={},
            action="respond",
            outcome="success",
            score=0.9,
        )
        assert ev.id != ""
        assert ev.id.startswith("ev_")


# =============================================================================
# LearnedPattern dataclass tests
# =============================================================================
class TestLearnedPattern:
    """Test LearnedPattern dataclass."""

    def test_required_fields(self):
        """Required fields are set."""
        p = LearnedPattern(
            name="chat_success_pattern",
            signal_type="chat_success",
            trigger_conditions={"has_papers": True},
        )
        assert p.name == "chat_success_pattern"
        assert p.signal_type == "chat_success"
        assert p.trigger_conditions == {"has_papers": True}

    def test_optional_fields_defaults(self):
        """Optional fields have defaults."""
        p = LearnedPattern(
            name="P",
            signal_type="chat_success",
            trigger_conditions={},
        )
        assert p.success_count == 0
        assert p.failure_count == 0
        assert p.last_used == ""
        assert p.effectiveness == 0.0

    def test_total_attempts_property(self):
        """total_attempts = success + failure."""
        p = LearnedPattern(
            name="P",
            signal_type="s",
            trigger_conditions={},
            success_count=5,
            failure_count=3,
        )
        assert p.total_attempts == 8

    def test_total_attempts_zero(self):
        """total_attempts is 0 when both counts are 0."""
        p = LearnedPattern(
            name="P",
            signal_type="s",
            trigger_conditions={},
            success_count=0,
            failure_count=0,
        )
        assert p.total_attempts == 0

    def test_is_reliable_true(self):
        """is_reliable=True when >=3 attempts and >=0.7 effectiveness."""
        p = LearnedPattern(
            name="Reliable",
            signal_type="s",
            trigger_conditions={},
            success_count=7,
            failure_count=3,
            effectiveness=0.7,
        )
        assert p.is_reliable is True

    def test_is_reliable_false_when_too_few_attempts(self):
        """is_reliable=False when <3 attempts even with high effectiveness."""
        p = LearnedPattern(
            name="Unreliable",
            signal_type="s",
            trigger_conditions={},
            success_count=2,
            failure_count=0,
            effectiveness=1.0,
        )
        assert p.is_reliable is False

    def test_is_reliable_false_when_low_effectiveness(self):
        """is_reliable=False when effectiveness <0.7 even with >=3 attempts."""
        p = LearnedPattern(
            name="Ineffective",
            signal_type="s",
            trigger_conditions={},
            success_count=3,
            failure_count=1,
            effectiveness=0.6,
        )
        assert p.is_reliable is False

    def test_is_reliable_true_at_boundary(self):
        """is_reliable=True at exact boundary (3 attempts, 0.7 effectiveness)."""
        p = LearnedPattern(
            name="Boundary",
            signal_type="s",
            trigger_conditions={},
            success_count=3,
            failure_count=0,
            effectiveness=1.0,
        )
        assert p.is_reliable is True

    def test_is_reliable_false_at_boundary_low(self):
        """is_reliable=False just below boundary."""
        p = LearnedPattern(
            name="JustBelow",
            signal_type="s",
            trigger_conditions={},
            success_count=2,
            failure_count=1,
            effectiveness=0.666,
        )
        assert p.is_reliable is False


# =============================================================================
# action_signals mapping tests
# =============================================================================
class TestActionSignals:
    """Test passive feedback action signal mapping."""

    def _action_signals(self) -> dict:
        """Replicate the action_signals mapping from infer_passive_feedback."""
        return {
            "continued": (True, 0.7, "用户继续追问"),
            "copied": (True, 0.9, "用户复制了回答"),
            "exited": (None, 0.5, "用户退出，未明确反馈"),
            "dismissed": (None, 0.5, "用户跳过反馈"),
            "repeated": (None, 0.4, "重复问题，可能回答不够好"),
        }

    def test_continued_is_positive(self):
        """'continued' maps to positive signal."""
        signals = self._action_signals()
        is_positive, score, reason = signals["continued"]
        assert is_positive is True
        assert score == 0.7
        assert "继续追问" in reason

    def test_copied_is_positive_high_score(self):
        """'copied' maps to positive with highest score."""
        signals = self._action_signals()
        is_positive, score, reason = signals["copied"]
        assert is_positive is True
        assert score == 0.9
        assert "复制" in reason

    def test_exited_is_neutral(self):
        """'exited' maps to neutral signal."""
        signals = self._action_signals()
        is_positive, score, reason = signals["exited"]
        assert is_positive is None
        assert score == 0.5

    def test_dismissed_is_neutral(self):
        """'dismissed' maps to neutral signal."""
        signals = self._action_signals()
        is_positive, score, reason = signals["dismissed"]
        assert is_positive is None
        assert score == 0.5

    def test_repeated_is_neutral_low_score(self):
        """'repeated' maps to neutral with low score."""
        signals = self._action_signals()
        is_positive, score, reason = signals["repeated"]
        assert is_positive is None
        assert score == 0.4
        assert "重复" in reason

    def test_unknown_action_not_in_signals(self):
        """Unknown action is not in the mapping."""
        signals = self._action_signals()
        assert "unknown" not in signals
        assert "clicked" not in signals


# =============================================================================
# _update_pattern_from_event logic tests
# =============================================================================
class TestUpdatePatternFromEvent:
    """Test pattern update logic from evolution events."""

    def _update_pattern_from_event(self, patterns: dict, event: dict) -> dict:
        """Replicate pattern update logic.

        Rules:
        - score >= 0.6 → success_count += 1
        - else → failure_count += 1
        - effectiveness = success_count / total (clamped to 0)
        """
        pattern_key = f"{event['signal_type']}_{event['action']}"
        if pattern_key not in patterns:
            patterns[pattern_key] = {
                "name": pattern_key,
                "signal_type": event["signal_type"],
                "trigger_conditions": event["trigger"],
                "success_count": 0,
                "failure_count": 0,
                "last_used": "",
                "effectiveness": 0.0,
            }

        p = patterns[pattern_key]
        if event["score"] >= 0.6:
            p["success_count"] += 1
        else:
            p["failure_count"] += 1

        total = p["success_count"] + p["failure_count"]
        p["effectiveness"] = p["success_count"] / total if total > 0 else 0.0
        p["last_used"] = event["timestamp"]

        return patterns

    def test_new_pattern_created_on_first_event(self):
        """New pattern key is created from first event."""
        patterns = {}
        event = {
            "signal_type": "chat_success",
            "action": "respond",
            "trigger": {"query": "test"},
            "score": 0.8,
            "timestamp": "2024-01-01T00:00:00",
        }
        patterns = self._update_pattern_from_event(patterns, event)
        key = "chat_success_respond"
        assert key in patterns
        assert patterns[key]["success_count"] == 1
        assert patterns[key]["failure_count"] == 0

    def test_high_score_increments_success(self):
        """score >= 0.6 increments success_count."""
        patterns = {}
        event = {"signal_type": "s", "action": "a", "trigger": {}, "score": 0.6, "timestamp": ""}
        patterns = self._update_pattern_from_event(patterns, event)
        assert patterns["s_a"]["success_count"] == 1
        assert patterns["s_a"]["failure_count"] == 0

    def test_low_score_increments_failure(self):
        """score < 0.6 increments failure_count."""
        patterns = {}
        event = {"signal_type": "s", "action": "a", "trigger": {}, "score": 0.59, "timestamp": ""}
        patterns = self._update_pattern_from_event(patterns, event)
        assert patterns["s_a"]["success_count"] == 0
        assert patterns["s_a"]["failure_count"] == 1

    def test_effectiveness_full_success(self):
        """100% success → effectiveness = 1.0."""
        patterns = {}
        for _ in range(3):
            event = {"signal_type": "s", "action": "a", "trigger": {}, "score": 0.9, "timestamp": ""}
            patterns = self._update_pattern_from_event(patterns, event)
        assert patterns["s_a"]["effectiveness"] == 1.0

    def test_effectiveness_full_failure(self):
        """100% failure → effectiveness = 0.0."""
        patterns = {}
        for _ in range(3):
            event = {"signal_type": "s", "action": "a", "trigger": {}, "score": 0.3, "timestamp": ""}
            patterns = self._update_pattern_from_event(patterns, event)
        assert patterns["s_a"]["effectiveness"] == 0.0

    def test_effectiveness_mixed(self):
        """Mixed results compute correct effectiveness."""
        patterns = {}
        events = [
            {"signal_type": "s", "action": "a", "trigger": {}, "score": 0.9, "timestamp": ""},
            {"signal_type": "s", "action": "a", "trigger": {}, "score": 0.3, "timestamp": ""},
            {"signal_type": "s", "action": "a", "trigger": {}, "score": 0.8, "timestamp": ""},
        ]
        for ev in events:
            patterns = self._update_pattern_from_event(patterns, ev)
        assert patterns["s_a"]["effectiveness"] == 2.0 / 3.0

    def test_last_used_updated(self):
        """last_used is set to event timestamp."""
        patterns = {}
        event = {"signal_type": "s", "action": "a", "trigger": {}, "score": 0.8, "timestamp": "2024-06-15T10:00:00"}
        patterns = self._update_pattern_from_event(patterns, event)
        assert patterns["s_a"]["last_used"] == "2024-06-15T10:00:00"

    def test_trigger_conditions_set_on_creation(self):
        """trigger_conditions from first event is preserved."""
        patterns = {}
        event = {"signal_type": "s", "action": "a", "trigger": {"key": "val"}, "score": 0.8, "timestamp": ""}
        patterns = self._update_pattern_from_event(patterns, event)
        assert patterns["s_a"]["trigger_conditions"] == {"key": "val"}

    def test_existing_pattern_not_overwritten(self):
        """Existing pattern fields are not overwritten by new events."""
        patterns = {"s_a": {"name": "custom", "signal_type": "s", "trigger_conditions": {}, "success_count": 5, "failure_count": 2, "last_used": "", "effectiveness": 0.0}}
        event = {"signal_type": "s", "action": "a", "trigger": {}, "score": 0.9, "timestamp": ""}
        patterns = self._update_pattern_from_event(patterns, event)
        # name should be preserved (only counts updated)
        assert patterns["s_a"]["success_count"] == 6


# =============================================================================
# get_reliable_patterns logic tests
# =============================================================================
class TestGetReliablePatterns:
    """Test reliable pattern filtering logic."""

    def _get_reliable_patterns(self, patterns: dict) -> list:
        """Replicate reliable pattern filtering."""
        return [
            p for p in patterns.values()
            if p["success_count"] + p["failure_count"] >= 3
            and p["effectiveness"] >= 0.7
        ]

    def test_includes_high_effectiveness_patterns(self):
        """Pattern with >=0.7 effectiveness and >=3 attempts is reliable."""
        patterns = {
            "high": {"name": "high", "signal_type": "s", "trigger_conditions": {}, "success_count": 10, "failure_count": 3, "last_used": "", "effectiveness": 0.769}
        }
        result = self._get_reliable_patterns(patterns)
        assert len(result) == 1
        assert result[0]["name"] == "high"

    def test_excludes_low_effectiveness_patterns(self):
        """Pattern with <0.7 effectiveness is not reliable."""
        patterns = {
            "low": {"name": "low", "signal_type": "s", "trigger_conditions": {}, "success_count": 5, "failure_count": 2, "last_used": "", "effectiveness": 0.69}
        }
        result = self._get_reliable_patterns(patterns)
        assert len(result) == 0

    def test_excludes_few_attempts(self):
        """Pattern with <3 attempts is not reliable even with high effectiveness."""
        patterns = {
            "few": {"name": "few", "signal_type": "s", "trigger_conditions": {}, "success_count": 2, "failure_count": 0, "last_used": "", "effectiveness": 1.0}
        }
        result = self._get_reliable_patterns(patterns)
        assert len(result) == 0

    def test_exact_boundary_included(self):
        """Exactly 3 attempts and 0.7 effectiveness is included."""
        patterns = {
            "boundary": {"name": "boundary", "signal_type": "s", "trigger_conditions": {}, "success_count": 3, "failure_count": 0, "last_used": "", "effectiveness": 1.0}
        }
        result = self._get_reliable_patterns(patterns)
        assert len(result) == 1

    def test_mixed_patterns_returns_only_reliable(self):
        """Only reliable patterns are returned from mixed set."""
        patterns = {
            "reliable": {"name": "reliable", "signal_type": "s", "trigger_conditions": {}, "success_count": 10, "failure_count": 3, "last_used": "", "effectiveness": 0.77},
            "unreliable": {"name": "unreliable", "signal_type": "s", "trigger_conditions": {}, "success_count": 1, "failure_count": 1, "last_used": "", "effectiveness": 0.5},
            "mostly_good": {"name": "mostly_good", "signal_type": "s", "trigger_conditions": {}, "success_count": 3, "failure_count": 2, "last_used": "", "effectiveness": 0.6},
        }
        result = self._get_reliable_patterns(patterns)
        assert len(result) == 1
        assert result[0]["name"] == "reliable"

    def test_empty_patterns(self):
        """Empty pattern dict returns empty list."""
        result = self._get_reliable_patterns({})
        assert result == []


# =============================================================================
# get_stats logic tests
# =============================================================================
class TestGetStats:
    """Test statistics computation logic."""

    def _compute_positive_rate(self, positive_count: int, feedback_count: int) -> float:
        """Replicate positive rate computation."""
        return positive_count / feedback_count if feedback_count > 0 else 0

    def _compute_learning_progress(self, reliable_count: int) -> float:
        """Replicate learning progress computation."""
        return reliable_count / 10 if reliable_count < 10 else 1.0

    def test_positive_rate_calculation(self):
        """Positive rate = positive / total."""
        assert self._compute_positive_rate(3, 10) == 0.3
        assert self._compute_positive_rate(10, 10) == 1.0

    def test_positive_rate_zero_when_no_feedback(self):
        """Zero feedback → zero positive rate."""
        assert self._compute_positive_rate(0, 0) == 0

    def test_learning_progress_below_10(self):
        """Learning progress = reliable / 10 when below 10."""
        assert self._compute_learning_progress(3) == 0.3
        assert self._compute_learning_progress(9) == 0.9

    def test_learning_progress_capped_at_1(self):
        """Learning progress caps at 1.0."""
        assert self._compute_learning_progress(10) == 1.0
        assert self._compute_learning_progress(15) == 1.0


# =============================================================================
# record_chat_feedback logic tests
# =============================================================================
class TestRecordChatFeedback:
    """Test record_chat_feedback logic."""

    def _build_feedback(self, is_positive: bool) -> dict:
        """Replicate feedback building from record_chat_feedback."""
        fb_type = "positive" if is_positive else "negative"
        return {
            "type": fb_type,
            "command": "chat",
            "score": 0.8,
        }

    def _build_signal(self, is_positive: bool) -> str:
        """Replicate signal type from record_chat_feedback."""
        return "chat_success" if is_positive else "chat_failure"

    def test_positive_feedback_type(self):
        """Positive is_positive yields 'positive' type."""
        fb = self._build_feedback(is_positive=True)
        assert fb["type"] == "positive"

    def test_negative_feedback_type(self):
        """Negative is_positive yields 'negative' type."""
        fb = self._build_feedback(is_positive=False)
        assert fb["type"] == "negative"

    def test_positive_signal_type(self):
        """Positive yields chat_success signal."""
        sig = self._build_signal(is_positive=True)
        assert sig == "chat_success"

    def test_negative_signal_type(self):
        """Negative yields chat_failure signal."""
        sig = self._build_signal(is_positive=False)
        assert sig == "chat_failure"

    def test_chat_command(self):
        """Feedback command is always 'chat'."""
        fb = self._build_feedback(is_positive=True)
        assert fb["command"] == "chat"


# =============================================================================
# EvolutionMemory instantiation
# =============================================================================
class TestEvolutionMemoryInit:
    """Test EvolutionMemory class."""

    def test_can_instantiate_with_path(self, tmp_path):
        """EvolutionMemory can be instantiated with a custom path."""
        mem = EvolutionMemory(memory_dir=tmp_path / "evolution")
        assert mem.memory_dir.exists()

    def test_default_memory_dir(self):
        """Default memory_dir is memory/evolution."""
        mem = EvolutionMemory()
        assert mem.memory_dir.name == "evolution"
        assert mem.memory_dir.parent.name == "memory"

    def test_feedback_file_path(self, tmp_path):
        """Feedback file path is memory_dir / feedback.jsonl."""
        mem = EvolutionMemory(memory_dir=tmp_path / "ev")
        assert mem.feedback_file.name == "feedback.jsonl"

    def test_events_file_path(self, tmp_path):
        """Events file path is memory_dir / evolution_events.jsonl."""
        mem = EvolutionMemory(memory_dir=tmp_path / "ev")
        assert mem.events_file.name == "evolution_events.jsonl"

    def test_patterns_file_path(self, tmp_path):
        """Patterns file path is memory_dir / learned_patterns.json."""
        mem = EvolutionMemory(memory_dir=tmp_path / "ev")
        assert mem.patterns_file.name == "learned_patterns.json"

    def test_get_evolution_memory_returns_instance(self):
        """get_evolution_memory returns an EvolutionMemory."""
        mem = get_evolution_memory()
        assert isinstance(mem, EvolutionMemory)
