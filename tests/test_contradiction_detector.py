"""Tests for llm/research/contradiction_detector.py — research gap contradiction detection."""

import pytest

from llm.research.contradiction_detector import (
    detect_field_contradiction,
    detect_polarity_contradiction,
    detect_evidence_contradiction,
    detect_contradictions,
)


class TestDetectFieldContradiction:
    """Test field-level contradiction detection."""

    def test_returns_none_when_current_is_unknown(self):
        """Unknown values should not contradict anything."""
        result = detect_field_contradiction(
            gap_type="embodied_planning",
            primary_field="representation_type",
            current_val="unknown",
        )
        assert result is None

    def test_returns_none_when_no_other_capsules(self):
        """No contradiction when no other capsules exist."""
        result = detect_field_contradiction(
            gap_type="embodied_planning",
            primary_field="representation_type",
            current_val="discrete",
            capsules=[],
        )
        assert result is None

    def test_finds_contradiction_when_value_differs(self):
        """Should find contradiction when same gap_type has different field value."""
        capsules = [
            {
                "capsule_id": "c1",
                "archetype": {
                    "source_paper_id": "paper1",
                    "representation_type": "discrete",
                },
            },
        ]
        result = detect_field_contradiction(
            gap_type="embodied_planning",
            primary_field="representation_type",
            current_val="continuous",
            capsules=capsules,
        )
        assert result is not None
        assert result["source_paper_id"] == "paper1"
        assert result["conflicting_value"] == "discrete"

    def test_returns_none_when_value_matches(self):
        """Should return None when field values are the same."""
        capsules = [
            {
                "capsule_id": "c1",
                "archetype": {
                    "source_paper_id": "paper1",
                    "representation_type": "discrete",
                },
            },
        ]
        result = detect_field_contradiction(
            gap_type="embodied_planning",
            primary_field="representation_type",
            current_val="discrete",
            capsules=capsules,
        )
        assert result is None

    def test_ignores_capsules_with_unknown_existing_value(self):
        """Capsules with 'unknown' existing value should not cause contradiction."""
        capsules = [
            {
                "capsule_id": "c1",
                "archetype": {
                    "source_paper_id": "paper1",
                    "representation_type": "unknown",
                },
            },
        ]
        result = detect_field_contradiction(
            gap_type="embodied_planning",
            primary_field="representation_type",
            current_val="discrete",
            capsules=capsules,
        )
        assert result is None

    def test_ignores_capsules_without_source_paper_id(self):
        """Capsules without source_paper_id should be skipped."""
        capsules = [
            {
                "capsule_id": "c1",
                "archetype": {},
            },
        ]
        result = detect_field_contradiction(
            gap_type="embodied_planning",
            primary_field="representation_type",
            current_val="discrete",
            capsules=capsules,
        )
        assert result is None


class TestDetectPolarityContradiction:
    """Test polarity-level contradiction detection."""

    def test_empty_list_returns_empty(self):
        """Empty capsule list returns empty contradictions."""
        result = detect_polarity_contradiction(gap_type="embodied_planning", capsules=[])
        assert result == []

    def test_same_polarity_no_contradiction(self):
        """Same polarity (positive/negative) should not contradict."""
        capsules = [
            {"capsule_id": "c1", "polarity": "positive"},
            {"capsule_id": "c2", "polarity": "positive"},
        ]
        result = detect_polarity_contradiction(gap_type="embodied_planning", capsules=capsules)
        assert result == []

    def test_opposing_polarity_contradiction(self):
        """Opposing polarities (positive vs negative) should contradict."""
        capsules = [
            {"capsule_id": "c1", "polarity": "positive", "archetype": {"source_paper_id": "p1"}},
            {"capsule_id": "c2", "polarity": "negative", "archetype": {"source_paper_id": "p2"}},
        ]
        result = detect_polarity_contradiction(gap_type="embodied_planning", capsules=capsules)
        assert len(result) == 1
        assert result[0]["type"] == "polarity"
        assert result[0]["paper_a"] == "p1"
        assert result[0]["paper_b"] == "p2"

    def test_open_polarity_does_not_conflict(self):
        """Open polarity should not conflict with any polarity."""
        capsules = [
            {"capsule_id": "c1", "polarity": "open", "archetype": {"source_paper_id": "p1"}},
            {"capsule_id": "c2", "polarity": "positive", "archetype": {"source_paper_id": "p2"}},
        ]
        result = detect_polarity_contradiction(gap_type="embodied_planning", capsules=capsules)
        assert result == []


class TestDetectEvidenceContradiction:
    """Test evidence-level contradiction detection."""

    def test_empty_list_returns_empty(self):
        """Empty capsule list returns empty contradictions."""
        result = detect_evidence_contradiction(gap_type="embodied_planning", capsules=[])
        assert result == []

    def test_same_evidence_no_contradiction(self):
        """Same evidence should not contradict."""
        capsules = [
            {"capsule_id": "c1", "archetype": {"evidence": "same evidence"}},
            {"capsule_id": "c2", "archetype": {"evidence": "same evidence"}},
        ]
        result = detect_evidence_contradiction(gap_type="embodied_planning", capsules=capsules)
        assert result == []

    def test_different_evidence_contradiction(self):
        """Different evidence should create contradiction."""
        capsules = [
            {"capsule_id": "c1", "archetype": {"evidence": "evidence A"}},
            {"capsule_id": "c2", "archetype": {"evidence": "evidence B"}},
        ]
        result = detect_evidence_contradiction(gap_type="embodied_planning", capsules=capsules)
        assert len(result) == 1
        assert result[0]["type"] == "evidence"
        assert result[0]["evidence_a"] == "evidence A"
        assert result[0]["evidence_b"] == "evidence B"

    def test_missing_evidence_ignored(self):
        """Capsules with missing evidence should be skipped."""
        capsules = [
            {"capsule_id": "c1", "archetype": {"evidence": "evidence A"}},
            {"capsule_id": "c2", "archetype": {}},
        ]
        result = detect_evidence_contradiction(gap_type="embodied_planning", capsules=capsules)
        assert result == []


class TestDetectContradictions:
    """Test the main detect_contradictions function."""

    def test_empty_list_returns_empty(self):
        """Empty capsule list returns empty."""
        result = detect_contradictions([])
        assert result == []

    def test_groups_by_gap_type(self):
        """Should group capsules by gap_type before checking."""
        capsules = [
            {
                "capsule_id": "c1",
                "trigger_gap_type": "embodied_planning",
                "polarity": "positive",
                "archetype": {"source_paper_id": "p1"},
            },
            {
                "capsule_id": "c2",
                "trigger_gap_type": "embodied_planning",
                "polarity": "negative",
                "archetype": {"source_paper_id": "p2"},
            },
            {
                "capsule_id": "c3",
                "trigger_gap_type": "rl_efficiency",
                "polarity": "positive",
                "archetype": {"source_paper_id": "p3"},
            },
        ]
        result = detect_contradictions(capsules)
        assert len(result) == 1
        assert result[0]["gap_type"] == "embodied_planning"

    def test_capsule_without_gap_type_skipped(self):
        """Capsules without trigger_gap_type should be skipped."""
        capsules = [
            {"capsule_id": "c1"},
            {"capsule_id": "c2"},
        ]
        result = detect_contradictions(capsules)
        assert result == []
