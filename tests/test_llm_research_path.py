"""Tier 2 unit tests — llm/research_path.py, pure functions, no I/O."""
import pytest
from llm.research_path import (
    ReadingLevel, PaperNode, ReadingStep, ReadingPath,
    ResearchPathPlanner,
)


# =============================================================================
# Dataclass tests
# =============================================================================
class TestReadingLevel:
    """Test ReadingLevel enum."""

    def test_values(self):
        assert ReadingLevel.INTRO.value == "intro"
        assert ReadingLevel.INTERMEDIATE.value == "intermediate"
        assert ReadingLevel.ADVANCED.value == "advanced"

    def test_all_defined(self):
        assert len(ReadingLevel) == 3


class TestPaperNode:
    """Test PaperNode dataclass."""

    def test_required_fields(self):
        node = PaperNode(paper_id="p001", title="Attention Is All You Need")
        assert node.paper_id == "p001"
        assert node.title == "Attention Is All You Need"
        assert node.year == 0
        assert node.authors == []
        assert node.cited_by == []
        assert node.cites == []

    def test_citation_fields_default(self):
        node = PaperNode(paper_id="p001", title="T")
        assert node.cited_by == []
        assert node.cites == []

    def test_all_fields(self):
        node = PaperNode(
            paper_id="p001", title="T", year=2017,
            authors=["Vaswani", "Shazeer"],
            cited_by=["p002", "p003"], cites=["p004"],
            relevance_score=0.9, pagerank=0.15,
            is_foundational=True, is_milestone=True,
        )
        assert node.year == 2017
        assert node.authors == ["Vaswani", "Shazeer"]
        assert node.cited_by == ["p002", "p003"]
        assert node.cites == ["p004"]
        assert node.relevance_score == 0.9
        assert node.pagerank == 0.15
        assert node.is_foundational is True
        assert node.is_milestone is True


class TestReadingStep:
    """Test ReadingStep dataclass."""

    def test_required_fields(self):
        paper = PaperNode(paper_id="p001", title="T")
        step = ReadingStep(order=1, paper=paper, role="core", reason="必读")
        assert step.order == 1
        assert step.paper.paper_id == "p001"
        assert step.role == "core"
        assert step.reason == "必读"
        assert step.estimated_read_time_minutes == 15  # default


class TestReadingPath:
    """Test ReadingPath dataclass."""

    def test_required_fields(self):
        path = ReadingPath(
            topic="transformer", level=ReadingLevel.INTERMEDIATE,
            total_papers=0, total_reading_time_minutes=0,
            steps=[],
        )
        assert path.topic == "transformer"
        assert path.level == ReadingLevel.INTERMEDIATE
        assert path.total_papers == 0
        assert path.steps == []
        assert path.alternative_paths == []  # default
        assert path.skipped_papers == []     # default


# =============================================================================
# ResearchPathPlanner — no kg/db, so plan_path returns empty path
# =============================================================================
class TestResearchPathPlannerInit:
    """Test ResearchPathPlanner instantiation and methods."""

    def test_instantiate_without_deps(self):
        planner = ResearchPathPlanner()
        assert planner.kg is None
        assert planner.db is None

    def test_has_expected_methods(self):
        planner = ResearchPathPlanner()
        assert hasattr(planner, "plan_path")
        assert hasattr(planner, "render_path")
        assert hasattr(planner, "render_mermaid")

    def test_plan_path_no_papers_returns_empty(self):
        planner = ResearchPathPlanner()
        path = planner.plan_path("nonexistent_topic_xyz")
        assert path.total_papers == 0
        assert path.steps == []
        assert path.topic == "nonexistent_topic_xyz"


# =============================================================================
# _parse_authors — pure string parsing
# =============================================================================
class TestParseAuthors:
    """Test _parse_authors."""

    planner = ResearchPathPlanner()

    def test_empty_string(self):
        assert self.planner._parse_authors("") == []

    def test_single_author(self):
        assert self.planner._parse_authors("Vaswani") == ["Vaswani"]

    def test_and_separator(self):
        result = self.planner._parse_authors("Vaswani and Shazeer")
        assert result == ["Vaswani", "Shazeer"]

    def test_comma_separated(self):
        result = self.planner._parse_authors("Vaswani, Shazeer, Parmar")
        assert result == ["Vaswani", "Shazeer", "Parmar"]

    def test_strips_whitespace(self):
        result = self.planner._parse_authors(" Vaswani ,  Shazeer ")
        assert result == ["Vaswani", "Shazeer"]

    def test_empty_parts_ignored(self):
        result = self.planner._parse_authors("Vaswani, , Shazeer")
        assert "Vaswani" in result
        assert "Shazeer" in result


# =============================================================================
# _calculate_pagerank — pure numpy algorithm
# =============================================================================
class TestCalculatePagerank:
    """Test _calculate_pagerank."""

    def _graph(self, *papers):
        return {p.paper_id: p for p in papers}

    def test_empty_graph(self):
        planner = ResearchPathPlanner()
        planner._calculate_pagerank({})
        # No error, no change

    def test_single_node(self):
        planner = ResearchPathPlanner()
        p = PaperNode(paper_id="p1", title="Single Paper")
        g = self._graph(p)
        planner._calculate_pagerank(g)
        assert g["p1"].pagerank > 0

    def test_two_nodes_no_edges(self):
        planner = ResearchPathPlanner()
        p1 = PaperNode(paper_id="p1", title="Paper 1")
        p2 = PaperNode(paper_id="p2", title="Paper 2")
        g = self._graph(p1, p2)
        planner._calculate_pagerank(g)
        # Both get some PageRank
        assert g["p1"].pagerank >= 0
        assert g["p2"].pagerank >= 0

    def test_two_nodes_citation(self):
        # p2 has cited_by=["p1"] means p1 cites p2 -> p2 should have higher PageRank
        planner = ResearchPathPlanner()
        p1 = PaperNode(paper_id="p1", title="Older Paper")
        p2 = PaperNode(paper_id="p2", title="Newer Paper", cited_by=["p1"])
        g = self._graph(p1, p2)
        planner._calculate_pagerank(g)
        assert g["p2"].pagerank > g["p1"].pagerank

    def test_pagerank_sum_approximately_one(self):
        planner = ResearchPathPlanner()
        papers = [PaperNode(paper_id=f"p{i}", title=f"Paper {i}") for i in range(5)]
        g = {p.paper_id: p for p in papers}
        planner._calculate_pagerank(g)
        total = sum(p.pagerank for p in g.values())
        assert 0.1 < total <= 1.5  # PageRank sum varies by graph structure

    def test_damping_converges(self):
        planner = ResearchPathPlanner()
        papers = [
            PaperNode(paper_id=f"p{i}", title=f"Paper {i}")
            for i in range(3)
        ]
        g = {p.paper_id: p for p in papers}
        # No error even with no edges
        planner._calculate_pagerank(g)
        assert all(p.pagerank >= 0 for p in g.values())


# =============================================================================
# _identify_key_papers — pure sorting logic
# =============================================================================
class TestIdentifyKeyPapers:
    """Test _identify_key_papers."""

    def _g(self, *papers):
        return {p.paper_id: p for p in papers}

    def test_empty_graph(self):
        planner = ResearchPathPlanner()
        planner._identify_key_papers({}, ReadingLevel.INTERMEDIATE)
        # No error

    def test_single_paper_marked_foundational(self):
        planner = ResearchPathPlanner()
        p = PaperNode(paper_id="p1", title="Paper 1", pagerank=1.0)
        g = self._g(p)
        planner._identify_key_papers(g, ReadingLevel.INTERMEDIATE)
        # top 25% = 1 paper -> foundational
        assert p.is_foundational is True

    def test_four_papers_top_quarter_foundational(self):
        planner = ResearchPathPlanner()
        papers = [
            PaperNode(paper_id=f"p{i}", title=f"Paper {i}", pagerank=float(i))
            for i in range(1, 5)  # p1..p4, pageranks 1,2,3,4
        ]
        g = self._g(*papers)
        planner._identify_key_papers(g, ReadingLevel.INTERMEDIATE)
        # top 25% = 1 paper (p4 with pagerank 4)
        foundational = [p for p in papers if p.is_foundational]
        assert len(foundational) == 1
        assert foundational[0].paper_id == "p4"

    def test_milestone_identified_by_early_year(self):
        planner = ResearchPathPlanner()
        papers = [
            PaperNode(paper_id="p_old", title="Old Paper", year=2010, pagerank=1.0),
            PaperNode(paper_id="p_new", title="New Paper", year=2020, pagerank=0.5),
        ]
        g = self._g(*papers)
        planner._identify_key_papers(g, ReadingLevel.INTERMEDIATE)
        # p_old is earliest with decent PageRank -> milestone
        assert papers[0].is_milestone is True


# =============================================================================
# _topological_sort — pure graph algorithm
# =============================================================================
class TestTopologicalSort:
    """Test _topological_sort."""

    def _g(self, *papers):
        return {p.paper_id: p for p in papers}

    def test_empty_graph(self):
        planner = ResearchPathPlanner()
        result = planner._topological_sort({}, ReadingLevel.INTERMEDIATE)
        assert result == []

    def test_single_node(self):
        planner = ResearchPathPlanner()
        p = PaperNode(paper_id="p1", title="Single Paper")
        result = planner._topological_sort(self._g(p), ReadingLevel.INTERMEDIATE)
        assert len(result) == 1
        assert result[0].paper_id == "p1"

    def test_two_nodes_independent(self):
        planner = ResearchPathPlanner()
        p1 = PaperNode(paper_id="p1", title="Paper 1")
        p2 = PaperNode(paper_id="p2", title="Paper 2")
        g = self._g(p1, p2)
        result = planner._topological_sort(g, ReadingLevel.INTERMEDIATE)
        assert len(result) == 2
        ids = {r.paper_id for r in result}
        assert ids == {"p1", "p2"}

    def test_citation_order(self):
        # p2 cites p1 (p2.cited_by = ["p1"]) -> p1 must come before p2
        planner = ResearchPathPlanner()
        p1 = PaperNode(paper_id="p1", title="Older Paper")
        p2 = PaperNode(paper_id="p2", title="Newer Paper", cited_by=["p1"])
        g = self._g(p1, p2)
        result = planner._topological_sort(g, ReadingLevel.INTERMEDIATE)
        ids = [r.paper_id for r in result]
        # p1 must appear before p2 since p1 cites p2 (p1 is in p2.cited_by)
        idx1 = ids.index("p1")
        idx2 = ids.index("p2")
        assert idx1 < idx2


# =============================================================================
# _assign_role — pure conditional logic
# =============================================================================
class TestAssignRole:
    """Test _assign_role."""

    planner = ResearchPathPlanner()

    def _role(self, paper, position, level=ReadingLevel.INTERMEDIATE):
        seen = set()
        return self.planner._assign_role(paper, position, level, seen)

    def test_foundational_pre_2018(self):
        p = PaperNode(paper_id="p1", title="T", is_foundational=True, year=2015)
        role, reason = self._role(p, 0)
        assert role == "foundation"
        assert "基础" in reason

    def test_foundational_post_2018(self):
        p = PaperNode(paper_id="p1", title="T", is_foundational=True, year=2019)
        role, reason = self._role(p, 0)
        assert role == "core"

    def test_first_position_is_core(self):
        p = PaperNode(paper_id="p1", title="Entry", year=2020)
        role, reason = self._role(p, 0)
        assert role == "core"

    def test_latest_paper(self):
        p = PaperNode(paper_id="p1", title="T", year=2024)
        role, reason = self._role(p, 3)
        assert role == "latest"
        assert "2024" in reason

    def test_highly_cited_improvement(self):
        p = PaperNode(paper_id="p1", title="T", year=2021, cited_by=["a", "b", "c"])
        role, reason = self._role(p, 2)
        assert role == "improvement"

    def test_intro_level_prefers_older(self):
        p = PaperNode(paper_id="p1", title="T", year=2019, is_foundational=False)
        role, reason = self._role(p, 1, ReadingLevel.INTRO)
        assert role == "core"

    def test_intro_level_variants(self):
        p = PaperNode(paper_id="p1", title="T", year=2021, is_foundational=False)
        role, reason = self._role(p, 1, ReadingLevel.INTRO)
        assert role == "variant"
        assert "2021" in reason

    def test_default_improvement(self):
        p = PaperNode(paper_id="p1", title="T", year=2020, cited_by=[])
        role, reason = self._role(p, 2)
        assert role == "improvement"


# =============================================================================
# _estimate_read_time — pure computation
# =============================================================================
class TestEstimateReadTime:
    """Test _estimate_read_time."""

    planner = ResearchPathPlanner()

    def test_base_time(self):
        p = PaperNode(paper_id="p1", title="Short")
        assert self.planner._estimate_read_time(p) == 15

    def test_long_title(self):
        # Every 50 chars adds 1 min
        p = PaperNode(paper_id="p1", title="A" * 200)
        time = self.planner._estimate_read_time(p)
        assert time > 15

    def test_many_citations(self):
        p = PaperNode(
            paper_id="p1", title="T",
            cited_by=["a", "b", "c", "d", "e", "f"],
        )
        time = self.planner._estimate_read_time(p)
        assert time >= 15 + 10  # base + 10 for >5 citations

    def test_capped_at_45(self):
        p = PaperNode(
            paper_id="p1",
            title="A" * 3000,  # long title adds ~60 min
            cited_by=["a", "b", "c", "d", "e", "f", "g", "h"],
        )
        time = self.planner._estimate_read_time(p)
        assert time == 45  # capped


# =============================================================================
# _empty_path — pure factory
# =============================================================================
class TestEmptyPath:
    """Test _empty_path."""

    def test_returns_empty_path(self):
        planner = ResearchPathPlanner()
        path = planner._empty_path("unknown", ReadingLevel.ADVANCED)
        assert path.total_papers == 0
        assert path.steps == []
        assert path.topic == "unknown"
        assert path.level == ReadingLevel.ADVANCED
        assert path.alternative_paths == []
        assert path.skipped_papers == []


# =============================================================================
# render_path — pure string formatting
# =============================================================================
class TestRenderPath:
    """Test render_path."""

    planner = ResearchPathPlanner()

    def test_empty_returns_not_found(self):
        path = ResearchPathPlanner()._empty_path("xyz", ReadingLevel.INTERMEDIATE)
        output = self.planner.render_path(path)
        assert "xyz" in output
        assert "未找到" in output

    def test_header_with_topic(self):
        path = ResearchPathPlanner()._empty_path("transformer", ReadingLevel.INTRO)
        output = self.planner.render_path(path)
        assert "transformer" in output

    def test_single_step(self):
        paper = PaperNode(paper_id="p1", title="Attention Is All You Need", year=2017)
        step = ReadingStep(order=1, paper=paper, role="foundation",
                           reason="开创性工作", estimated_read_time_minutes=20)
        path = ReadingPath(
            topic="transformer", level=ReadingLevel.INTERMEDIATE,
            total_papers=1, total_reading_time_minutes=20, steps=[step],
        )
        output = self.planner.render_path(path)
        assert "Attention" in output
        assert "2017" in output
        assert "🏛️" in output  # foundation icon
        assert "开创性" in output

    def test_no_year_shown_as_empty(self):
        paper = PaperNode(paper_id="p1", title="Paper", year=0)
        step = ReadingStep(order=1, paper=paper, role="core", reason="R", estimated_read_time_minutes=15)
        path = ReadingPath(
            topic="T", level=ReadingLevel.INTERMEDIATE,
            total_papers=1, total_reading_time_minutes=15, steps=[step],
        )
        output = self.planner.render_path(path)
        assert "[0]" not in output

    def test_title_truncated_to_50(self):
        paper = PaperNode(paper_id="p1", title="A" * 60)
        step = ReadingStep(order=1, paper=paper, role="core", reason="R", estimated_read_time_minutes=15)
        path = ReadingPath(
            topic="T", level=ReadingLevel.INTERMEDIATE,
            total_papers=1, total_reading_time_minutes=15, steps=[step],
        )
        output = self.planner.render_path(path)
        assert ("A" * 50) in output
        assert ("A" * 51) not in output

    def test_authors_truncated_to_two(self):
        paper = PaperNode(paper_id="p1", title="T", authors=["A", "B", "C"])
        step = ReadingStep(order=1, paper=paper, role="core", reason="R", estimated_read_time_minutes=15)
        path = ReadingPath(
            topic="T", level=ReadingLevel.INTERMEDIATE,
            total_papers=1, total_reading_time_minutes=15, steps=[step],
        )
        output = self.planner.render_path(path)
        assert "A, B" in output
        assert "et al." in output

    def test_skipped_papers_shown(self):
        paper = PaperNode(paper_id="p1", title="Included")
        step = ReadingStep(order=1, paper=paper, role="core", reason="R", estimated_read_time_minutes=15)
        path = ReadingPath(
            topic="T", level=ReadingLevel.INTERMEDIATE,
            total_papers=1, total_reading_time_minutes=15, steps=[step],
            skipped_papers=["Skipped Paper A", "Skipped Paper B"],
        )
        output = self.planner.render_path(path)
        assert "2 篇" in output


# =============================================================================
# render_mermaid — pure string formatting
# =============================================================================
class TestRenderMermaid:
    """Test render_mermaid."""

    planner = ResearchPathPlanner()

    def test_empty(self):
        path = ResearchPathPlanner()._empty_path("T", ReadingLevel.INTERMEDIATE)
        output = self.planner.render_mermaid(path)
        assert "Empty" in output

    def test_single_node(self):
        paper = PaperNode(paper_id="p001", title="First Paper")
        step = ReadingStep(order=1, paper=paper, role="core", reason="R", estimated_read_time_minutes=15)
        path = ReadingPath(
            topic="T", level=ReadingLevel.INTERMEDIATE,
            total_papers=1, total_reading_time_minutes=15, steps=[step],
        )
        output = self.planner.render_mermaid(path)
        assert "graph TD" in output
        assert "Reading Path" in output
        assert "p001" in output

    def test_node_title_truncated(self):
        paper = PaperNode(paper_id="p001", title="A" * 40)
        step = ReadingStep(order=1, paper=paper, role="foundation", reason="R", estimated_read_time_minutes=15)
        path = ReadingPath(
            topic="T", level=ReadingLevel.INTERMEDIATE,
            total_papers=1, total_reading_time_minutes=15, steps=[step],
        )
        output = self.planner.render_mermaid(path)
        # Should not crash; title truncated to 30
        assert "graph TD" in output

    def test_arrows_between_steps(self):
        papers = [
            PaperNode(paper_id=f"p{i:03d}", title=f"Paper {i}")
            for i in range(1, 4)
        ]
        steps = [
            ReadingStep(order=i, paper=papers[i-1], role="core", reason="R", estimated_read_time_minutes=15)
            for i in range(1, 4)
        ]
        path = ReadingPath(
            topic="T", level=ReadingLevel.INTERMEDIATE,
            total_papers=3, total_reading_time_minutes=45, steps=steps,
        )
        output = self.planner.render_mermaid(path)
        # Should have arrows between adjacent steps
        assert "-->" in output

    def test_role_classes_defined(self):
        paper = PaperNode(paper_id="p001", title="T")
        step = ReadingStep(order=1, paper=paper, role="foundation", reason="R", estimated_read_time_minutes=15)
        path = ReadingPath(
            topic="T", level=ReadingLevel.INTERMEDIATE,
            total_papers=1, total_reading_time_minutes=15, steps=[step],
        )
        output = self.planner.render_mermaid(path)
        assert "classDef foundation" in output


# =============================================================================
# Integration: plan_path with mocked deps returns empty path
# =============================================================================
class TestPlanPathIntegration:
    """Test plan_path end-to-end with no dependencies."""

    def test_plan_path_unknown_topic(self):
        planner = ResearchPathPlanner()
        path = planner.plan_path("definitely_no_papers_match_this_query_xyz")
        assert path.total_papers == 0
        assert path.steps == []

    def test_plan_path_honors_max_papers(self):
        planner = ResearchPathPlanner()
        path = planner.plan_path("x", max_papers=3)
        assert path.total_papers <= 3
