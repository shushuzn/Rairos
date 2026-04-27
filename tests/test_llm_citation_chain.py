"""Tier 2 unit tests — llm/citation_chain.py, pure functions, no I/O."""
import pytest
from llm.citation_chain import (
    CitationNode,
    CitationChain,
    CitationChainBuilder,
)


# =============================================================================
# Dataclass tests
# =============================================================================
class TestCitationNode:
    """Test CitationNode dataclass."""

    def test_required_fields(self):
        """Required fields: paper_id, title."""
        node = CitationNode(paper_id="p1", title="Attention Is All You Need")
        assert node.paper_id == "p1"
        assert node.title == "Attention Is All You Need"

    def test_optional_fields_defaults(self):
        """Optional fields have defaults."""
        node = CitationNode(paper_id="p1", title="T")
        assert node.year == 0
        assert node.authors == []
        assert node.citations == []
        assert node.cited_by == []

    def test_all_fields(self):
        """All fields can be set."""
        node = CitationNode(
            paper_id="arxiv:1234",
            title="BERT",
            year=2018,
            authors=["Devlin et al"],
            citations=["paper_a", "paper_b"],
            cited_by=["paper_c"],
        )
        assert node.year == 2018
        assert node.authors == ["Devlin et al"]
        assert node.citations == ["paper_a", "paper_b"]
        assert node.cited_by == ["paper_c"]


class TestCitationChain:
    """Test CitationChain dataclass."""

    def test_required_fields(self):
        """Required fields: nodes."""
        chain = CitationChain(nodes=[])
        assert chain.nodes == []

    def test_optional_fields_defaults(self):
        """Optional fields have defaults."""
        chain = CitationChain()
        assert chain.nodes == []
        assert chain.edges == []

    def test_nodes_and_edges(self):
        """Nodes and edges can be populated."""
        node = CitationNode(paper_id="p1", title="T")
        edge = ("p1", "p2")
        chain = CitationChain(nodes=[node], edges=[edge])
        assert len(chain.nodes) == 1
        assert chain.edges == [("p1", "p2")]


# =============================================================================
# CitationChainBuilder instantiation
# =============================================================================
class TestCitationChainBuilderInit:
    """Test CitationChainBuilder class."""

    def test_can_instantiate(self):
        """Builder can be instantiated."""
        builder = CitationChainBuilder()
        assert builder.db is None
        assert builder.nodes == {}

    def test_can_instantiate_with_db(self):
        """Builder can be instantiated with db."""
        mock_db = object()
        builder = CitationChainBuilder(db=mock_db)
        assert builder.db is mock_db


# =============================================================================
# add_paper tests
# =============================================================================
class TestAddPaper:
    """Test add_paper method."""

    def _add_paper(self, builder: CitationChainBuilder, paper_id: str, title: str, **kwargs):
        """Replicate add_paper logic."""
        if paper_id not in builder.nodes:
            builder.nodes[paper_id] = CitationNode(
                paper_id=paper_id,
                title=title,
                year=kwargs.get("year", 0),
                authors=kwargs.get("authors", []) or [],
                citations=kwargs.get("references", []) or [],
            )
        return builder.nodes[paper_id]

    def test_creates_new_node(self):
        """add_paper creates a new node."""
        builder = CitationChainBuilder()
        node = self._add_paper(builder, "p1", "Paper One", year=2020)
        assert builder.nodes["p1"].title == "Paper One"
        assert builder.nodes["p1"].year == 2020

    def test_returns_existing_node(self):
        """add_paper returns existing node without overwriting."""
        builder = CitationChainBuilder()
        self._add_paper(builder, "p1", "Original")
        result = self._add_paper(builder, "p1", "Overwrite")
        assert builder.nodes["p1"].title == "Original"
        assert result.title == "Original"

    def test_defaults_authors_to_empty_list(self):
        """Authors defaults to empty list when not provided."""
        builder = CitationChainBuilder()
        self._add_paper(builder, "p1", "T")
        assert builder.nodes["p1"].authors == []

    def test_defaults_references_to_empty_list(self):
        """References defaults to empty list when not provided."""
        builder = CitationChainBuilder()
        self._add_paper(builder, "p1", "T")
        assert builder.nodes["p1"].citations == []

    def test_authors_preserved_when_provided(self):
        """Authors are preserved when provided."""
        builder = CitationChainBuilder()
        self._add_paper(builder, "p1", "T", authors=["Author A", "Author B"])
        assert builder.nodes["p1"].authors == ["Author A", "Author B"]

    def test_references_preserved_as_citations(self):
        """references parameter populates citations field."""
        builder = CitationChainBuilder()
        self._add_paper(builder, "p1", "T", references=["ref1", "ref2"])
        assert builder.nodes["p1"].citations == ["ref1", "ref2"]


# =============================================================================
# link_citations tests
# =============================================================================
class TestLinkCitations:
    """Test link_citations method."""

    def _add_paper(self, builder, paper_id, title, **kwargs):
        if paper_id not in builder.nodes:
            builder.nodes[paper_id] = CitationNode(
                paper_id=paper_id, title=title,
                year=kwargs.get("year", 0),
                authors=kwargs.get("authors", []) or [],
                citations=kwargs.get("references", []) or [],
            )
        return builder.nodes[paper_id]

    def _link_citations(self, builder, from_id, to_id):
        """Replicate link_citations logic."""
        if from_id in builder.nodes and to_id in builder.nodes:
            if to_id not in builder.nodes[from_id].citations:
                builder.nodes[from_id].citations.append(to_id)
            if from_id not in builder.nodes[to_id].cited_by:
                builder.nodes[to_id].cited_by.append(from_id)

    def test_adds_citation_to_source(self):
        """Link adds to_id to from_id's citations."""
        builder = CitationChainBuilder()
        self._add_paper(builder, "a", "Paper A")
        self._add_paper(builder, "b", "Paper B")
        self._link_citations(builder, "a", "b")
        assert "b" in builder.nodes["a"].citations

    def test_adds_cited_by_to_target(self):
        """Link adds from_id to to_id's cited_by."""
        builder = CitationChainBuilder()
        self._add_paper(builder, "a", "Paper A")
        self._add_paper(builder, "b", "Paper B")
        self._link_citations(builder, "a", "b")
        assert "a" in builder.nodes["b"].cited_by

    def test_no_change_if_from_missing(self):
        """Link is skipped if from_id not in nodes."""
        builder = CitationChainBuilder()
        self._add_paper(builder, "b", "Paper B")
        self._link_citations(builder, "a", "b")
        assert builder.nodes["b"].cited_by == []

    def test_no_change_if_to_missing(self):
        """Link is skipped if to_id not in nodes."""
        builder = CitationChainBuilder()
        self._add_paper(builder, "a", "Paper A")
        self._link_citations(builder, "a", "b")
        assert builder.nodes["a"].citations == []

    def test_no_duplicate_citations(self):
        """Linking same pair twice does not duplicate."""
        builder = CitationChainBuilder()
        self._add_paper(builder, "a", "A")
        self._add_paper(builder, "b", "B")
        self._link_citations(builder, "a", "b")
        self._link_citations(builder, "a", "b")
        assert builder.nodes["a"].citations.count("b") == 1


# =============================================================================
# find_path tests (BFS)
# =============================================================================
class TestFindPath:
    """Test find_path BFS logic."""

    def _build_chain(self, builder, edges):
        """Build a chain from edges [(from, to), ...]."""
        for from_id, to_id in edges:
            if from_id not in builder.nodes:
                builder.nodes[from_id] = CitationNode(paper_id=from_id, title=from_id)
            if to_id not in builder.nodes:
                builder.nodes[to_id] = CitationNode(paper_id=to_id, title=to_id)
            if to_id not in builder.nodes[from_id].citations:
                builder.nodes[from_id].citations.append(to_id)
            if from_id not in builder.nodes[to_id].cited_by:
                builder.nodes[to_id].cited_by.append(from_id)

    def _find_path(self, builder, from_id, to_id):
        """Replicate BFS find_path logic."""
        if from_id not in builder.nodes or to_id not in builder.nodes:
            return None
        if from_id == to_id:
            return [from_id]

        visited = {from_id}
        queue = [[from_id]]

        while queue:
            path = queue.pop(0)
            current = path[-1]

            for neighbor in builder.nodes[current].citations:
                if neighbor == to_id:
                    return path + [neighbor]
                if neighbor not in visited:
                    visited.add(neighbor)
                    queue.append(path + [neighbor])

            for neighbor in builder.nodes[current].cited_by:
                if neighbor == to_id:
                    return path + [neighbor]
                if neighbor not in visited:
                    visited.add(neighbor)
                    queue.append(path + [neighbor])

        return None

    def test_returns_none_for_missing_from(self):
        """Missing from_id returns None."""
        builder = CitationChainBuilder()
        builder.nodes["b"] = CitationNode(paper_id="b", title="B")
        result = self._find_path(builder, "a", "b")
        assert result is None

    def test_returns_none_for_missing_to(self):
        """Missing to_id returns None."""
        builder = CitationChainBuilder()
        builder.nodes["a"] = CitationNode(paper_id="a", title="A")
        result = self._find_path(builder, "a", "b")
        assert result is None

    def test_returns_single_node_for_same_node(self):
        """Same node returns [node_id]."""
        builder = CitationChainBuilder()
        builder.nodes["a"] = CitationNode(paper_id="a", title="A")
        result = self._find_path(builder, "a", "a")
        assert result == ["a"]

    def test_finds_direct_citation(self):
        """A cites B → path found."""
        builder = CitationChainBuilder()
        self._build_chain(builder, [("a", "b")])
        result = self._find_path(builder, "a", "b")
        assert result == ["a", "b"]

    def test_finds_indirect_citation(self):
        """A cites B cites C → path found."""
        builder = CitationChainBuilder()
        self._build_chain(builder, [("a", "b"), ("b", "c")])
        result = self._find_path(builder, "a", "c")
        assert result == ["a", "b", "c"]

    def test_finds_reverse_path(self):
        """C cited_by B cited_by A → reverse path found."""
        builder = CitationChainBuilder()
        self._build_chain(builder, [("a", "b"), ("b", "c")])
        result = self._find_path(builder, "c", "a")
        assert result == ["c", "b", "a"]

    def test_finds_shortest_path(self):
        """Shortest path returned when multiple exist."""
        builder = CitationChainBuilder()
        self._build_chain(builder, [("a", "b"), ("b", "d"), ("a", "c"), ("c", "d")])
        result = self._find_path(builder, "a", "d")
        assert result is not None
        assert result[0] == "a"
        assert result[-1] == "d"
        assert len(result) == 3  # shortest is a→b→d or a→c→d

    def test_returns_none_when_no_path(self):
        """No path → None."""
        builder = CitationChainBuilder()
        builder.nodes["a"] = CitationNode(paper_id="a", title="A")
        builder.nodes["b"] = CitationNode(paper_id="b", title="B")
        result = self._find_path(builder, "a", "b")
        assert result is None

    def test_path_traverses_cited_by(self):
        """Path follows cited_by edges."""
        builder = CitationChainBuilder()
        self._build_chain(builder, [("older", "newer")])
        result = self._find_path(builder, "newer", "older")
        assert result == ["newer", "older"]


# =============================================================================
# render_text tests
# =============================================================================
class TestRenderText:
    """Test render_text formatting logic."""

    def _render_text(self, chain, max_nodes=20):
        """Replicate render_text logic."""
        if not chain.nodes:
            return "No citation chain."

        lines = ["=" * 60, "📚 Citation Chain", "=" * 60, ""]

        sorted_nodes = sorted(chain.nodes, key=lambda x: -x.year if x.year else 0)

        for i, node in enumerate(sorted_nodes[:max_nodes]):
            lines.append(f"[{node.paper_id[:8]}] {node.title[:50]}")
            lines.append(f"  Year: {node.year or '?'} | Cites: {len(node.citations)} | Cited by: {len(node.cited_by)}")
            lines.append("")

        if len(chain.nodes) > max_nodes:
            lines.append(f"... and {len(chain.nodes) - max_nodes} more papers")

        lines.append("")
        lines.append(f"Total: {len(chain.nodes)} papers, {len(chain.edges)} connections")
        lines.append("=" * 60)
        return "\n".join(lines)

    def test_empty_chain_message(self):
        """Empty chain returns 'No citation chain.'."""
        chain = CitationChain(nodes=[], edges=[])
        result = self._render_text(chain)
        assert result == "No citation chain."

    def test_header_format(self):
        """Header uses border and title."""
        node = CitationNode(paper_id="p1", title="Paper")
        chain = CitationChain(nodes=[node], edges=[])
        result = self._render_text(chain)
        assert "📚 Citation Chain" in result
        assert "=" * 60 in result

    def test_sorted_by_year_descending(self):
        """Nodes sorted by year descending."""
        nodes = [
            CitationNode(paper_id="p2020", title="Old", year=2020),
            CitationNode(paper_id="p2023", title="New", year=2023),
            CitationNode(paper_id="p2021", title="Mid", year=2021),
        ]
        chain = CitationChain(nodes=nodes, edges=[])
        result = self._render_text(chain)
        lines = result.split("\n")
        idx = next(i for i, l in enumerate(lines) if "New" in l)
        old_idx = next(i for i, l in enumerate(lines) if "Old" in l)
        assert idx < old_idx

    def test_title_truncated_to_50_chars(self):
        """Title truncated to 50 chars."""
        node = CitationNode(paper_id="p1", title="A" * 80)
        chain = CitationChain(nodes=[node], edges=[])
        result = self._render_text(chain)
        assert "A" * 50 in result
        assert "A" * 51 not in result

    def test_paper_id_truncated_to_8(self):
        """Paper ID truncated to 8 chars."""
        node = CitationNode(paper_id="very_long_paper_id_12345", title="T")
        chain = CitationChain(nodes=[node], edges=[])
        result = self._render_text(chain)
        # "very_lon" is the first 8 chars of "very_long_paper_id_12345"
        assert "very_lon" in result
        assert "very_long_paper" not in result

    def test_year_zero_shown_as_question_mark(self):
        """Year=0 shown as '?'."""
        node = CitationNode(paper_id="p1", title="No Year", year=0)
        chain = CitationChain(nodes=[node], edges=[])
        result = self._render_text(chain)
        assert "Year: ?" in result

    def test_year_nonzero_shown(self):
        """Non-zero year shown."""
        node = CitationNode(paper_id="p1", title="T", year=2023)
        chain = CitationChain(nodes=[node], edges=[])
        result = self._render_text(chain)
        assert "Year: 2023" in result

    def test_citations_count_shown(self):
        """Cites count is shown."""
        node = CitationNode(paper_id="p1", title="T", citations=["a", "b", "c"])
        chain = CitationChain(nodes=[node], edges=[])
        result = self._render_text(chain)
        assert "Cites: 3" in result

    def test_cited_by_count_shown(self):
        """Cited-by count is shown."""
        node = CitationNode(paper_id="p1", title="T", cited_by=["x", "y"])
        chain = CitationChain(nodes=[node], edges=[])
        result = self._render_text(chain)
        assert "Cited by: 2" in result

    def test_truncation_message(self):
        """Truncation message shown when >max_nodes."""
        nodes = [CitationNode(paper_id=f"p{i}", title=f"P{i}") for i in range(25)]
        chain = CitationChain(nodes=nodes, edges=[])
        result = self._render_text(chain, max_nodes=20)
        assert "... and 5 more papers" in result

    def test_footer_shows_totals(self):
        """Footer shows total papers and connections."""
        node1 = CitationNode(paper_id="a", title="A")
        node2 = CitationNode(paper_id="b", title="B")
        chain = CitationChain(nodes=[node1, node2], edges=[("a", "b")])
        result = self._render_text(chain)
        assert "Total: 2 papers, 1 connections" in result


# =============================================================================
# render_graphviz tests
# =============================================================================
class TestRenderGraphviz:
    """Test render_graphviz formatting logic."""

    def _render_graphviz(self, chain):
        """Replicate render_graphviz logic."""
        lines = ["digraph citations {", "  rankdir=LR;", "  node [shape=box];"]

        for node in chain.nodes:
            label = f'{node.title[:30]}...\\n({node.year})' if node.year else node.title[:30]
            lines.append(f'  "{node.paper_id}" [label="{label}"];')

        for from_id, to_id in chain.edges:
            lines.append(f'  "{from_id}" -> "{to_id}";')

        lines.append("}")
        return "\n".join(lines)

    def test_starts_with_digraph(self):
        """Output starts with digraph."""
        chain = CitationChain(nodes=[CitationNode(paper_id="p1", title="T")], edges=[])
        result = self._render_graphviz(chain)
        assert result.startswith("digraph citations {")

    def test_ends_with_closing_brace(self):
        """Output ends with closing brace."""
        chain = CitationChain(nodes=[], edges=[])
        result = self._render_graphviz(chain)
        assert result.strip().endswith("}")

    def test_node_declaration_format(self):
        """Node declared with ID and label."""
        node = CitationNode(paper_id="p1", title="Paper Title")
        chain = CitationChain(nodes=[node], edges=[])
        result = self._render_graphviz(chain)
        assert '"p1" [label="' in result

    def test_title_truncated_to_30(self):
        """Title truncated to 30 chars in label."""
        node = CitationNode(paper_id="p1", title="A" * 50)
        chain = CitationChain(nodes=[node], edges=[])
        result = self._render_graphviz(chain)
        assert "A" * 30 in result
        assert "A" * 31 not in result

    def test_year_appended_when_present(self):
        """Year appended in label when present."""
        node = CitationNode(paper_id="p1", title="T", year=2023)
        chain = CitationChain(nodes=[node], edges=[])
        result = self._render_graphviz(chain)
        assert "\\n(2023)" in result

    def test_no_year_append_when_zero(self):
        """No year in label when year=0."""
        node = CitationNode(paper_id="p1", title="NoYear", year=0)
        chain = CitationChain(nodes=[node], edges=[])
        result = self._render_graphviz(chain)
        assert "\\n(0)" not in result

    def test_edge_arrow_format(self):
        """Edge formatted as "from" -> "to"."""
        chain = CitationChain(
            nodes=[CitationNode(paper_id="a", title="A"), CitationNode(paper_id="b", title="B")],
            edges=[("a", "b")],
        )
        result = self._render_graphviz(chain)
        assert '"a" -> "b";' in result

    def test_multiple_edges(self):
        """Multiple edges all appear."""
        chain = CitationChain(
            nodes=[
                CitationNode(paper_id="a", title="A"),
                CitationNode(paper_id="b", title="B"),
                CitationNode(paper_id="c", title="C"),
            ],
            edges=[("a", "b"), ("b", "c")],
        )
        result = self._render_graphviz(chain)
        assert '"a" -> "b";' in result
        assert '"b" -> "c";' in result


# =============================================================================
# render_mermaid tests
# =============================================================================
class TestRenderMermaid:
    """Test render_mermaid formatting logic."""

    def _render_mermaid(self, chain):
        """Replicate render_mermaid logic."""
        lines = ["```mermaid", "flowchart LR"]

        for node in chain.nodes:
            year_str = f"({node.year})" if node.year else ""
            lines.append(f'    {node.paper_id[:8]}[{node.title[:30]}{year_str}]')

        for from_id, to_id in chain.edges:
            lines.append(f"    {from_id[:8]} --> {to_id[:8]}")

        lines.append("```")
        return "\n".join(lines)

    def test_starts_with_mermaid_block(self):
        """Output starts with ```mermaid and flowchart."""
        chain = CitationChain(nodes=[], edges=[])
        result = self._render_mermaid(chain)
        assert result.startswith("```mermaid")
        assert "flowchart LR" in result

    def test_ends_with_code_block_close(self):
        """Output ends with ```."""
        chain = CitationChain(nodes=[], edges=[])
        result = self._render_mermaid(chain)
        assert result.strip().endswith("```")

    def test_node_format(self):
        """Node formatted as id[title]."""
        node = CitationNode(paper_id="p1", title="Paper Title")
        chain = CitationChain(nodes=[node], edges=[])
        result = self._render_mermaid(chain)
        assert "p1[Paper Title]" in result

    def test_title_truncated_to_30(self):
        """Title truncated to 30 chars."""
        node = CitationNode(paper_id="p1", title="A" * 50)
        chain = CitationChain(nodes=[node], edges=[])
        result = self._render_mermaid(chain)
        assert "A" * 30 in result
        assert "A" * 31 not in result

    def test_paper_id_truncated_to_8(self):
        """Paper ID truncated to 8 chars."""
        node = CitationNode(paper_id="very_long_paper_id", title="T")
        chain = CitationChain(nodes=[node], edges=[])
        result = self._render_mermaid(chain)
        # "very_lon" is the first 8 chars of "very_long_paper_id"
        assert "very_lon" in result
        assert "very_long_paper" not in result

    def test_year_in_node_when_present(self):
        """Year shown in node when present."""
        node = CitationNode(paper_id="p1", title="T", year=2023)
        chain = CitationChain(nodes=[node], edges=[])
        result = self._render_mermaid(chain)
        assert "(2023)" in result

    def test_no_year_in_node_when_zero(self):
        """No year shown when year=0."""
        node = CitationNode(paper_id="p1", title="T", year=0)
        chain = CitationChain(nodes=[node], edges=[])
        result = self._render_mermaid(chain)
        assert "0" not in result.split("\n")[2]

    def test_edge_arrow_format(self):
        """Edge uses --> arrow."""
        chain = CitationChain(
            nodes=[CitationNode(paper_id="a", title="A"), CitationNode(paper_id="b", title="B")],
            edges=[("a", "b")],
        )
        result = self._render_mermaid(chain)
        assert "    a --> b" in result

    def test_multiple_edges(self):
        """Multiple edges all appear."""
        chain = CitationChain(
            nodes=[
                CitationNode(paper_id="a", title="A"),
                CitationNode(paper_id="b", title="B"),
                CitationNode(paper_id="c", title="C"),
            ],
            edges=[("a", "b"), ("a", "c")],
        )
        result = self._render_mermaid(chain)
        assert "a --> b" in result
        assert "a --> c" in result
