"""Unit tests for CLI cite-graph subcommand."""
import argparse
import io
import json
from unittest.mock import MagicMock, patch

import pytest

from cli import _run_cite_graph


# ─────────────────────────────────────────────────────────────────────────────
# Helpers
# ─────────────────────────────────────────────────────────────────────────────

class FakeCitationRecord:
    """Fake CitationRecord matching db.database.CitationRecord."""
    def __init__(self, id, source_id, target_id):
        self.id = id
        self.source_id = source_id
        self.target_id = target_id


class FakePaperRecord:
    """Fake PaperRecord matching db.database.PaperRecord."""
    def __init__(self, id, title=""):
        self.id = id
        self.title = title


def make_args(**kwargs):
    defaults = dict(paper="2301.00001", depth=2, max_nodes=30, format="text")
    defaults.update(kwargs)
    ns = argparse.Namespace()
    for k, v in defaults.items():
        setattr(ns, k, v)
    return ns


# ─────────────────────────────────────────────────────────────────────────────
# Test: cite-graph --paper not found
# ─────────────────────────────────────────────────────────────────────────────

class TestCiteGraphNotFound:
    """Test _run_cite_graph when the root paper is not in the database."""

    @patch("cli.Database")
    def test_paper_not_found_returns_1(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.paper_exists.return_value = False
        mock_db_cls.return_value = mock_db

        rc = _run_cite_graph(make_args(paper="nonexistent"))
        assert rc == 1
        assert "not found" in capsys.readouterr().err


# ─────────────────────────────────────────────────────────────────────────────
# Test: cite-graph text format — root only (no citations)
# ─────────────────────────────────────────────────────────────────────────────

class TestCiteGraphTextRootOnly:
    """Test text output when root has no citations in either direction."""

    @patch("cli.Database")
    def test_empty_graph_shows_root(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.paper_exists.return_value = True
        mock_db.get_paper_title.return_value = "Attention Is All You Need"
        mock_db.get_citations.return_value = []  # no forward, no backward
        mock_db.get_papers_bulk.return_value = {}
        mock_db_cls.return_value = mock_db

        rc = _run_cite_graph(make_args(paper="2301.00001", depth=1))
        assert rc == 0
        out = capsys.readouterr().out
        assert "2301.00001" in out
        assert "ROOT" in out
        assert "Attention Is All You Need" in out
        assert "CITED BY" not in out
        assert "CITES" not in out
        assert "1 nodes" in out


# ─────────────────────────────────────────────────────────────────────────────
# Test: cite-graph text format — forward and backward citations
# ─────────────────────────────────────────────────────────────────────────────

class TestCiteGraphTextWithCitations:
    """Test text output with both forward (cited-by) and backward (references) citations."""

    @patch("cli.Database")
    def test_forward_and_backward_citations(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.paper_exists.return_value = True
        mock_db.get_paper_title.side_effect = lambda pid: {
            "2301.00001": "Attention Is All You Need",
            "2201.11111": "Paper That Cites Root",
            "2201.22222": "Another Paper Citing Root",
            "2107.12345": "BERT (cited by root)",
            "2106.09876": "GPT-3 (cited by root)",
        }.get(pid, "")

        # Root is cited by 2201.11111 and 2201.22222 (forward citations)
        # Root cites 2107.12345 and 2106.09876 (backward citations)
        mock_db.get_citations.side_effect = lambda pid, direction: {
            ("2301.00001", "to"): [
                FakeCitationRecord(id=1, source_id="2201.11111", target_id="2301.00001"),
                FakeCitationRecord(id=2, source_id="2201.22222", target_id="2301.00001"),
            ],
            ("2301.00001", "from"): [
                FakeCitationRecord(id=3, source_id="2301.00001", target_id="2107.12345"),
                FakeCitationRecord(id=4, source_id="2301.00001", target_id="2106.09876"),
            ],
        }.get((pid, direction), [])

        mock_db.get_papers_bulk.return_value = {
            "2201.11111": FakePaperRecord("2201.11111", "Paper That Cites Root"),
            "2201.22222": FakePaperRecord("2201.22222", "Another Paper Citing Root"),
            "2107.12345": FakePaperRecord("2107.12345", "BERT (cited by root)"),
            "2106.09876": FakePaperRecord("2106.09876", "GPT-3 (cited by root)"),
        }
        mock_db_cls.return_value = mock_db

        rc = _run_cite_graph(make_args(paper="2301.00001", depth=1))
        assert rc == 0
        out = capsys.readouterr().out
        assert "CITED BY" in out
        assert "CITES" in out
        assert "2201.11111" in out
        assert "2201.22222" in out
        assert "2107.12345" in out
        assert "2106.09876" in out
        assert "BERT (cited by root)" in out
        assert "nodes, 4 edges" in out


# ─────────────────────────────────────────────────────────────────────────────
# Test: cite-graph depth=2 includes 2-hop expansion
# ─────────────────────────────────────────────────────────────────────────────

class TestCiteGraphDepth2:
    """Test that depth=2 also fetches 2-hop neighbours of depth-1 nodes."""

    @patch("cli.Database")
    def test_depth_2_expands_2hop(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.paper_exists.return_value = True
        mock_db.get_paper_title.side_effect = lambda pid: {
            "2301.00001": "Attention Is All You Need",
            "2201.11111": "Paper Citing Root",
            "2107.12345": "BERT",
            "2302.33333": "Paper Citing Paper Citing Root",  # 2-hop forward
            "2108.44444": "Paper Cited By BERT",  # 2-hop backward
        }.get(pid, "")

        citation_side_effects = {
            ("2301.00001", "to"): [
                FakeCitationRecord(id=1, source_id="2201.11111", target_id="2301.00001"),
            ],
            ("2301.00001", "from"): [
                FakeCitationRecord(id=2, source_id="2301.00001", target_id="2107.12345"),
            ],
            # 2-hop: paper that cites 2201.11111
            ("2201.11111", "to"): [
                FakeCitationRecord(id=3, source_id="2302.33333", target_id="2201.11111"),
            ],
            # 2-hop: paper cited by 2107.12345
            ("2107.12345", "from"): [
                FakeCitationRecord(id=4, source_id="2107.12345", target_id="2108.44444"),
            ],
        }
        mock_db.get_citations.side_effect = lambda pid, direction: citation_side_effects.get((pid, direction), [])

        mock_db.get_papers_bulk.return_value = {
            "2201.11111": FakePaperRecord("2201.11111", "Paper Citing Root"),
            "2107.12345": FakePaperRecord("2107.12345", "BERT"),
            "2302.33333": FakePaperRecord("2302.33333", "Paper Citing Paper Citing Root"),
            "2108.44444": FakePaperRecord("2108.44444", "Paper Cited By BERT"),
        }
        mock_db_cls.return_value = mock_db

        rc = _run_cite_graph(make_args(paper="2301.00001", depth=2))
        assert rc == 0
        out = capsys.readouterr().out
        assert "2-HOP CITED BY" in out
        assert "2-HOP CITES" in out
        assert "2302.33333" in out  # 2-hop forward
        assert "2108.44444" in out  # 2-hop backward


# ─────────────────────────────────────────────────────────────────────────────
# Test: cite-graph json format
# ─────────────────────────────────────────────────────────────────────────────

class TestCiteGraphJson:
    """Test JSON output format."""

    @patch("cli.Database")
    def test_json_format(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.paper_exists.return_value = True
        mock_db.get_paper_title.side_effect = lambda pid: {
            "2301.00001": "Attention Is All You Need",
            "2201.11111": "Paper Citing Root",
        }.get(pid, "")

        mock_db.get_citations.side_effect = lambda pid, direction: {
            ("2301.00001", "to"): [
                FakeCitationRecord(id=1, source_id="2201.11111", target_id="2301.00001"),
            ],
            ("2301.00001", "from"): [],
        }.get((pid, direction), [])

        mock_db.get_papers_bulk.return_value = {
            "2201.11111": FakePaperRecord("2201.11111", "Paper Citing Root"),
        }
        mock_db_cls.return_value = mock_db

        rc = _run_cite_graph(make_args(paper="2301.00001", depth=1, format="json"))
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert data["root"] == "2301.00001"
        assert data["title"] == "Attention Is All You Need"
        node_ids = {n["id"] for n in data["nodes"]}
        assert "2301.00001" in node_ids
        assert "2201.11111" in node_ids
        edge_froms = {e["from"] for e in data["edges"]}
        assert "2201.11111" in edge_froms


# ─────────────────────────────────────────────────────────────────────────────
# Test: cite-graph mermaid format
# ─────────────────────────────────────────────────────────────────────────────

class TestCiteGraphMermaid:
    """Test Mermaid diagram output format."""

    @patch("cli.Database")
    def test_mermaid_format(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.paper_exists.return_value = True
        mock_db.get_paper_title.side_effect = lambda pid: {
            "2301.00001": "Attention Is All You Need",
            "2201.11111": "Paper Citing Root",
        }.get(pid, "")

        mock_db.get_citations.side_effect = lambda pid, direction: {
            ("2301.00001", "to"): [
                FakeCitationRecord(id=1, source_id="2201.11111", target_id="2301.00001"),
            ],
            ("2301.00001", "from"): [],
        }.get((pid, direction), [])

        mock_db.get_papers_bulk.return_value = {
            "2201.11111": FakePaperRecord("2201.11111", "Paper Citing Root"),
        }
        mock_db_cls.return_value = mock_db

        rc = _run_cite_graph(make_args(paper="2301.00001", depth=1, format="mermaid"))
        assert rc == 0
        out = capsys.readouterr().out
        assert "```mermaid" in out
        assert "graph TD" in out
        assert "-->" in out


# ─────────────────────────────────────────────────────────────────────────────
# Test: cite-graph max_nodes cap
# ─────────────────────────────────────────────────────────────────────────────

class TestCiteGraphMaxNodes:
    """Test that max_nodes limits the number of collected nodes."""

    @patch("cli.Database")
    def test_max_nodes_caps_collection(self, mock_db_cls, capsys):
        mock_db = MagicMock()
        mock_db.paper_exists.return_value = True
        mock_db.get_paper_title.return_value = "Root Paper"

        # Return 5 forward citations, 5 backward citations
        many_forward = [
            FakeCitationRecord(id=i, source_id=f"fwd{i}", target_id="2301.00001")
            for i in range(5)
        ]
        many_backward = [
            FakeCitationRecord(id=i + 10, source_id="2301.00001", target_id=f"bwd{i}")
            for i in range(5)
        ]

        def citations_side_effect(pid, direction):
            if direction == "to":
                return many_forward
            if direction == "from":
                return many_backward
            return []

        mock_db.get_citations.side_effect = citations_side_effect
        mock_db.get_papers_bulk.return_value = {}
        mock_db_cls.return_value = mock_db

        # With max_nodes=3, only 3 forward + 3 backward should appear
        rc = _run_cite_graph(make_args(paper="2301.00001", depth=1, max_nodes=3))
        assert rc == 0
        out = capsys.readouterr().out
        # Only 3 of each direction should appear
        fwd_count = sum(1 for line in out.splitlines() if "fwd" in line)
        bwd_count = sum(1 for line in out.splitlines() if "bwd" in line)
        # Each paper appears in its own line (not counting header/group label lines)
        assert fwd_count <= 3
        assert bwd_count <= 3


# ─────────────────────────────────────────────────────────────────────────────
# Test: cite-graph --plain-text mode
# ─────────────────────────────────────────────────────────────────────────────

class TestCiteGraphPlainText:
    """Test _run_cite_graph with --plain-text option."""

    def test_plain_text_arxiv_only(self, capsys):
        """Plain-text mode extracts arXiv IDs from raw text."""
        text = (
            "This paper builds on arXiv:2301.11111 and arXiv:2302.22222.\n"
            "References\n"
            "[1] Smith et al. 2023.\n"
            "[2] Jones et al. arXiv:2303.33333.\n"
        )
        args = make_args(paper="2301.00001", plain_text=text, format="text")
        rc = _run_cite_graph(args)
        assert rc == 0
        out = capsys.readouterr().out
        assert "2301.11111" in out
        assert "2302.22222" in out
        assert "2303.33333" in out
        assert "[4 nodes, 3 edges]" in out  # root + 3 refs

    def test_plain_text_doi_only(self, capsys):
        """Plain-text mode extracts DOIs from raw text."""
        text = (
            "Prior work includes DOI:10.1000/abc123 and "
            "https://doi.org/10.1001/journal.xy. References section below.\n"
        )
        args = make_args(paper="2301.00001", plain_text=text, format="text")
        rc = _run_cite_graph(args)
        assert rc == 0
        out = capsys.readouterr().out
        assert "10.1000/abc123" in out
        assert "10.1001/journal.xy" in out
        assert "arXiv: 0" in out

    def test_plain_text_mixed(self, capsys):
        """Plain-text mode handles arXiv IDs and DOIs together."""
        text = (
            "We cite arXiv:2401.99999 and DOI:10.1126/science.123456.\n"
        )
        args = make_args(paper="2401.00001", plain_text=text, format="text")
        rc = _run_cite_graph(args)
        assert rc == 0
        out = capsys.readouterr().out
        assert "arXiv:2401.99999" in out
        assert "doi:10.1126/science.123456" in out

    def test_plain_text_no_refs(self, capsys):
        """Plain-text mode returns 0 when no references found."""
        text = "This paper has no references section."
        args = make_args(paper="2301.00001", plain_text=text, format="text")
        rc = _run_cite_graph(args)
        assert rc == 0
        out = capsys.readouterr().out
        assert "No references found" in out

    def test_plain_text_json_format(self, capsys):
        """Plain-text mode outputs correct JSON structure."""
        text = "See arXiv:2401.11111 for related work."
        args = make_args(paper="2401.00001", plain_text=text, format="json")
        rc = _run_cite_graph(args)
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert data["root"] == "2401.00001"
        assert data["mode"] == "plain-text"
        assert data["stats"]["arxiv_count"] == 1
        assert len(data["nodes"]) == 2
        assert len(data["edges"]) == 1

    def test_plain_text_mermaid_format(self, capsys):
        """Plain-text mode outputs valid mermaid syntax."""
        text = "Related to arXiv:2401.22222 and arXiv:2401.33333."
        args = make_args(paper="2401.00001", plain_text=text, format="mermaid")
        rc = _run_cite_graph(args)
        assert rc == 0
        out = capsys.readouterr().out
        assert "```mermaid" in out
        assert "graph TD" in out
        assert "2401_00001" in out  # sanitized root ID
        assert "-->" in out  # edge arrow

    def test_plain_text_duplicate_refs_deduped(self, capsys):
        """Same reference appearing twice is deduplicated in graph."""
        text = (
            "We cite arXiv:2401.11111 and again arXiv:2401.11111 here.\n"
            "Also see arXiv:2401.22222.\n"
        )
        args = make_args(paper="2401.00001", plain_text=text, format="text")
        rc = _run_cite_graph(args)
        assert rc == 0
        out = capsys.readouterr().out
        # Should be 3 nodes (root + 2 unique refs), not 4
        assert "3 nodes, 2 edges" in out

