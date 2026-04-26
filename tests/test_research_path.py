"""Tests for research path planner."""
import pytest

from llm.research_path import (
    ResearchPathPlanner,
    ReadingLevel,
    PaperNode,
    ReadingPath,
    ReadingStep,
)


class TestResearchPathPlanner:
    """Test ResearchPathPlanner."""

    def test_empty_path_when_no_papers(self):
        """Test empty path returned when no papers found."""
        planner = ResearchPathPlanner()
        path = planner.plan_path("nonexistent_topic_xyz_123")

        assert isinstance(path, ReadingPath)
        assert path.total_papers == 0
        assert path.steps == []

    def test_paper_node_creation(self):
        """Test PaperNode dataclass."""
        paper = PaperNode(
            paper_id="test-001",
            title="Test Paper",
            year=2023,
            authors=["Author A", "Author B"],
            pagerank=0.5,
        )

        assert paper.paper_id == "test-001"
        assert paper.title == "Test Paper"
        assert paper.year == 2023
        assert len(paper.authors) == 2
        assert paper.pagerank == 0.5

    def test_reading_step_creation(self):
        """Test ReadingStep dataclass."""
        paper = PaperNode(paper_id="test-001", title="Test")
        step = ReadingStep(
            order=1,
            paper=paper,
            role="foundation",
            reason="Foundational work",
            estimated_read_time_minutes=20,
        )

        assert step.order == 1
        assert step.role == "foundation"
        assert step.reason == "Foundational work"
        assert step.estimated_read_time_minutes == 20

    def test_pagerank_calculation(self):
        """Test PageRank calculation on simple graph."""
        planner = ResearchPathPlanner()

        # Create a simple citation graph where:
        # A is cited by B (B cites A)
        # B is cited by C (C cites B)
        # So C should have highest influence (most influential)
        papers = {
            "A": PaperNode(paper_id="A", title="Paper A", cited_by=["B"], cites=[]),
            "B": PaperNode(paper_id="B", title="Paper B", cited_by=["C"], cites=["A"]),
            "C": PaperNode(paper_id="C", title="Paper C", cited_by=[], cites=["B"]),
        }

        planner._calculate_pagerank(papers)

        # All papers should have PageRank scores
        assert papers["A"].pagerank > 0
        assert papers["B"].pagerank > 0
        assert papers["C"].pagerank > 0

    def test_topological_sort(self):
        """Test topological sort for reading order."""
        planner = ResearchPathPlanner()

        # Create a dependency graph where:
        # C is cited by A and B
        # B is cited by A
        # A is not cited by any in graph
        papers = {
            "A": PaperNode(paper_id="A", title="Paper A", cited_by=[]),
            "B": PaperNode(paper_id="B", title="Paper B", cited_by=["A"], cites=["C"]),
            "C": PaperNode(paper_id="C", title="Paper C", cited_by=["B"], cites=[]),
        }

        sorted_papers = planner._topological_sort(papers, ReadingLevel.INTERMEDIATE)

        # A should come first (no incoming edges)
        assert sorted_papers[0].paper_id == "A"
        # C should come last (most incoming edges)
        assert sorted_papers[-1].paper_id == "C"

    def test_render_empty_path(self):
        """Test rendering empty path."""
        planner = ResearchPathPlanner()
        path = planner._empty_path("test_topic", ReadingLevel.INTRO)
        output = planner.render_path(path)

        assert "test_topic" in output
        assert "未找到" in output or "No papers" in output.lower()

    def test_render_path_with_steps(self):
        """Test rendering path with steps."""
        planner = ResearchPathPlanner()

        paper = PaperNode(
            paper_id="test-001",
            title="Attention Is All You Need",
            year=2017,
            authors=["Vaswani", "Shazeer"],
        )
        step = ReadingStep(
            order=1,
            paper=paper,
            role="foundation",
            reason="Groundbreaking work",
            estimated_read_time_minutes=30,
        )
        path = ReadingPath(
            topic="transformer",
            level=ReadingLevel.INTERMEDIATE,
            total_papers=1,
            total_reading_time_minutes=30,
            steps=[step],
        )

        output = planner.render_path(path)

        assert "transformer" in output
        assert "Attention" in output
        assert "2017" in output
        assert "Groundbreaking" in output

    def test_render_mermaid(self):
        """Test Mermaid diagram generation."""
        planner = ResearchPathPlanner()

        papers = [
            PaperNode(paper_id=f"paper-{i}", title=f"Paper {i}")
            for i in range(3)
        ]
        steps = [
            ReadingStep(order=i + 1, paper=p, role="core", reason="Test", estimated_read_time_minutes=15)
            for i, p in enumerate(papers)
        ]
        path = ReadingPath(
            topic="test",
            level=ReadingLevel.INTRO,
            total_papers=3,
            total_reading_time_minutes=45,
            steps=steps,
        )

        mermaid = planner.render_mermaid(path)

        assert "graph TD" in mermaid
        assert "subgraph" in mermaid
        # Should have arrows between steps
        assert "-->" in mermaid

    def test_identify_key_papers(self):
        """Test identification of foundational papers."""
        planner = ResearchPathPlanner()

        papers = {
            "p1": PaperNode(paper_id="p1", title="Old foundational", year=2015, pagerank=0.9),
            "p2": PaperNode(paper_id="p2", title="Recent paper", year=2023, pagerank=0.3),
        }

        planner._calculate_pagerank(papers)
        planner._identify_key_papers(papers, ReadingLevel.INTERMEDIATE)

        # p1 should be identified as foundational due to high PageRank
        assert papers["p1"].is_foundational

    def test_level_based_recommendations(self):
        """Test that different levels produce different roles."""
        planner = ResearchPathPlanner()

        # Paper with early year should be foundation
        papers = {
            "early": PaperNode(paper_id="early", title="Early Work", year=2015, pagerank=0.8),
            "late": PaperNode(paper_id="late", title="Recent Work", year=2023, pagerank=0.4),
        }

        planner._calculate_pagerank(papers)
        planner._identify_key_papers(papers, ReadingLevel.INTERMEDIATE)

        steps = planner._generate_steps(list(papers.values()), "test", ReadingLevel.INTERMEDIATE)

        # Should have roles assigned
        roles = {s.role for s in steps}
        assert len(roles) > 0
