"""Tests for citation chain."""
import pytest
from llm.citation_chain import (
    CitationChainBuilder,
    CitationChain,
    CitationNode,
)


class TestCitationChainBuilder:
    """Test CitationChainBuilder."""

    @pytest.fixture
    def builder(self):
        return CitationChainBuilder()

    def test_add_paper(self, builder):
        """Test adding a paper."""
        node = builder.add_paper("p1", "Paper One", year=2023)
        assert node.paper_id == "p1"
        assert node.title == "Paper One"
        assert node.year == 2023

    def test_link_citations(self, builder):
        """Test linking citations."""
        builder.add_paper("p1", "Paper One")
        builder.add_paper("p2", "Paper Two")
        builder.link_citations("p1", "p2")

        assert "p2" in builder.nodes["p1"].citations
        assert "p1" in builder.nodes["p2"].cited_by

    def test_find_path(self, builder):
        """Test finding citation path."""
        builder.add_paper("p1", "Paper One")
        builder.add_paper("p2", "Paper Two")
        builder.add_paper("p3", "Paper Three")
        builder.link_citations("p1", "p2")
        builder.link_citations("p2", "p3")

        path = builder.find_path("p1", "p3")
        assert path == ["p1", "p2", "p3"]

    def test_find_path_no_path(self, builder):
        """Test no path exists."""
        builder.add_paper("p1", "Paper One")
        builder.add_paper("p2", "Paper Two")

        path = builder.find_path("p1", "p2")
        assert path is None

    def test_find_influencers(self, builder):
        """Test finding influencers."""
        builder.add_paper("p1", "Paper One")
        builder.add_paper("p2", "Paper Two")
        builder.add_paper("p3", "Paper Three")
        builder.link_citations("p1", "p2")
        builder.link_citations("p2", "p3")

        influencers = builder.find_influencers("p3")
        assert len(influencers) == 2
        ids = [i.paper_id for i in influencers]
        assert "p1" in ids
        assert "p2" in ids

    def test_find_impact(self, builder):
        """Test finding impact."""
        builder.add_paper("p1", "Paper One")
        builder.add_paper("p2", "Paper Two")
        builder.add_paper("p3", "Paper Three")
        builder.link_citations("p1", "p2")
        builder.link_citations("p2", "p3")

        impact = builder.find_impact("p1")
        assert len(impact) == 2
        ids = [i.paper_id for i in impact]
        assert "p2" in ids
        assert "p3" in ids

    def test_render_text(self, builder):
        """Test text rendering."""
        builder.add_paper("p1", "Paper One", year=2023)
        builder.add_paper("p2", "Paper Two", year=2022)
        chain = CitationChain(nodes=list(builder.nodes.values()), edges=[])
        output = builder.render_text(chain)

        assert "Citation Chain" in output
        assert "Paper One" in output

    def test_render_graphviz(self, builder):
        """Test Graphviz rendering."""
        builder.add_paper("p1", "Paper One")
        builder.link_citations("p1", "p2")
        builder.add_paper("p2", "Paper Two")
        chain = CitationChain(nodes=list(builder.nodes.values()), edges=[("p1", "p2")])
        output = builder.render_graphviz(chain)

        assert "digraph citations" in output
        assert "p1" in output
        assert 'p1" -> "p2"' in output

    def test_render_mermaid(self, builder):
        """Test Mermaid rendering."""
        builder.add_paper("p1", "Paper One")
        builder.add_paper("p2", "Paper Two")
        chain = CitationChain(nodes=list(builder.nodes.values()), edges=[("p1", "p2")])
        output = builder.render_mermaid(chain)

        assert "```mermaid" in output
        assert "flowchart LR" in output


class TestCitationNode:
    """Test CitationNode."""

    def test_creation(self):
        """Test creating a node."""
        node = CitationNode(paper_id="p1", title="Test Paper")
        assert node.paper_id == "p1"
        assert len(node.citations) == 0
        assert len(node.cited_by) == 0


class TestCitationChain:
    """Test CitationChain."""

    def test_creation(self):
        """Test creating a chain."""
        chain = CitationChain()
        assert len(chain.nodes) == 0
        assert len(chain.edges) == 0
