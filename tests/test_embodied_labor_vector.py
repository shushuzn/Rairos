"""Tests for embodied_planning, labor_displacement_tracker, vector_store."""

import pytest
from llm.research.embodied_planning import (
    track_embodied_evolution,
    render_embodied_planning_dashboard,
    render_embodied_planning_graph,
    render_evolution_timeline,
    render_confidence_calibration,
)
from llm.labor_displacement_tracker import (
    is_labor_related,
    get_labor_papers,
    render_labor_tracker_html,
)
from core.vector_store import SearchResult, ZillizStore, is_zilliz_configured


class TestEmbodiedPlanning:
    def test_track_embodied_evolution(self):
        result = track_embodied_evolution(
            paper_id="p1",
            title="Test",
            representation_type="symbolic",
            confidence=0.9,
            gap_title="Gap",
        )
        assert result is None or isinstance(result, dict)

    def test_render_dashboard(self):
        result = render_embodied_planning_dashboard(type_counts={}, papers=[])
        assert isinstance(result, str)
        assert isinstance(result, str)

    def test_render_graph(self):
        result = render_embodied_planning_graph(type_counts={})
        assert isinstance(result, str)

    def test_render_timeline(self):
        result = render_evolution_timeline()
        assert isinstance(result, str)

    def test_render_confidence(self):
        result = render_confidence_calibration()
        assert isinstance(result, str)


class TestLaborDisplacementTracker:
    def test_is_labor_related(self):
        paper = {"title": "AI and Jobs", "abstract": "Automation displaces workers"}
        result = is_labor_related(paper)
        assert isinstance(result, bool)

    def test_get_labor_papers(self):
        result = get_labor_papers()
        assert isinstance(result, list)

    def test_render_html(self):
        result = render_labor_tracker_html()
        assert isinstance(result, str)
        assert isinstance(result, str)


class TestVectorStore:
    def test_search_result_fields(self):
        r = SearchResult(id="s1", score=0.95, content="test content", file="a.py", line=10)
        assert r.score == 0.95
        assert r.content == "test content"

    def test_is_zilliz_configured(self):
        result = is_zilliz_configured()
        assert isinstance(result, bool)
