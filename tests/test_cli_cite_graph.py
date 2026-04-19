"""Unit tests for _run_cite_graph."""
import argparse
import json
from io import StringIO
from unittest.mock import MagicMock, patch

import pytest

from cli import _run_cite_graph


# ─────────────────────────────────────────────────────────────────────────────
# Helpers
# ─────────────────────────────────────────────────────────────────────────────

def make_args(**kwargs):
    defaults = dict(
        subcmd="cite-graph",
        paper="arXiv:1234.5678",
        depth=2,
        max_nodes=30,
        format="text",
        plain_text=None,
        fetch_metadata=False,
    )
    defaults.update(kwargs)
    ns = argparse.Namespace()
    for k, v in defaults.items():
        setattr(ns, k, v)
    return ns


# ─────────────────────────────────────────────────────────────────────────────
# Plain-text mode
# ─────────────────────────────────────────────────────────────────────────────

class TestCiteGraphPlainTextMode:
    """Test plain-text reference extraction mode."""

    @pytest.fixture(autouse=True)
    def setup(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")

    def test_plain_text_extracts_arxiv_ids(self):
        args = make_args(
            paper="arXiv:1703.12345",
            plain_text=(
                "This work builds on [2103.00001] and another paper arXiv:2104.11111. "
                "Also cited: arXiv:2201.00100."
            ),
        )
        with patch("cli._extract_references_from_text") as mock_extract:
            mock_extract.return_value = {
                "arxiv_ids": ["2103.00001", "2104.11111", "2201.00100"],
                "dois": [],
                "pmids": [],
                "isbns": [],
            }
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_cite_graph(args)
        assert rc == 0
        output = captured.getvalue()
        assert "2103.00001" in output
        assert "2104.11111" in output

    def test_plain_text_extracts_dois(self):
        args = make_args(
            paper="arXiv:1703.12345",
            plain_text="Cited 10.1038/nature12373 and 10.1126/science.abc1234.",
        )
        with patch("cli._extract_references_from_text") as mock_extract:
            mock_extract.return_value = {
                "arxiv_ids": [],
                "dois": ["10.1038/nature12373", "10.1126/science.abc1234"],
                "pmids": [],
                "isbns": [],
            }
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_cite_graph(args)
        assert rc == 0
        output = captured.getvalue()
        assert "10.1038/nature12373" in output

    def test_plain_text_extracts_pmids(self):
        args = make_args(
            paper="arXiv:1703.12345",
            plain_text="Prior work includes PMID:12345678 and PMID:87654321.",
        )
        with patch("cli._extract_references_from_text") as mock_extract:
            mock_extract.return_value = {
                "arxiv_ids": [],
                "dois": [],
                "pmids": ["12345678", "87654321"],
                "isbns": [],
            }
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_cite_graph(args)
        assert rc == 0
        output = captured.getvalue()
        assert "12345678" in output

    def test_plain_text_extracts_isbns(self):
        args = make_args(
            paper="arXiv:1703.12345",
            plain_text="Refer to ISBN:978-0-12-345678-9 and ISBN:978-3-16-148410-0.",
        )
        with patch("cli._extract_references_from_text") as mock_extract:
            mock_extract.return_value = {
                "arxiv_ids": [],
                "dois": [],
                "pmids": [],
                "isbns": ["978-0-12-345678-9", "978-3-16-148410-0"],
            }
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_cite_graph(args)
        assert rc == 0
        output = captured.getvalue()
        assert "978-0-12-345678-9" in output

    def test_plain_text_mixed_references(self):
        args = make_args(
            paper="arXiv:1703.12345",
            plain_text="Mixed: arXiv:2103.00001, doi:10.1038/nature12373, PMID:12345678.",
        )
        with patch("cli._extract_references_from_text") as mock_extract:
            mock_extract.return_value = {
                "arxiv_ids": ["2103.00001"],
                "dois": ["10.1038/nature12373"],
                "pmids": ["12345678"],
                "isbns": [],
            }
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_cite_graph(args)
        assert rc == 0
        output = captured.getvalue()
        assert "2103.00001" in output
        assert "nature12373" in output
        assert "12345678" in output

    def test_plain_text_no_references_found(self):
        args = make_args(
            paper="arXiv:1703.12345",
            plain_text="This paper has no citations at all.",
        )
        with patch("cli._extract_references_from_text") as mock_extract:
            mock_extract.return_value = {
                "arxiv_ids": [], "dois": [], "pmids": [], "isbns": []
            }
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_cite_graph(args)
        assert rc == 0
        output = captured.getvalue()
        assert "No references found" in output

    def test_plain_text_json_format(self):
        args = make_args(
            paper="arXiv:1703.12345",
            plain_text="arXiv:2103.00001 is cited here.",
            format="json",
        )
        with patch("cli._extract_references_from_text") as mock_extract:
            mock_extract.return_value = {
                "arxiv_ids": ["2103.00001"],
                "dois": [],
                "pmids": [],
                "isbns": [],
            }
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_cite_graph(args)
        assert rc == 0
        data = json.loads(captured.getvalue())
        assert data["root"] == "arXiv:1703.12345"
        assert data["mode"] == "plain-text"
        assert data["stats"]["arxiv_count"] == 1

    def test_plain_text_mermaid_format(self):
        args = make_args(
            paper="arXiv:1703.12345",
            plain_text="arXiv:2103.00001 is cited here.",
            format="mermaid",
        )
        with patch("cli._extract_references_from_text") as mock_extract:
            mock_extract.return_value = {
                "arxiv_ids": ["2103.00001"],
                "dois": [],
                "pmids": [],
                "isbns": [],
            }
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_cite_graph(args)
        assert rc == 0
        output = captured.getvalue()
        assert "```mermaid" in output
        assert "graph TD" in output


# ─────────────────────────────────────────────────────────────────────────────
# Plain-text + metadata fetch mode
# ─────────────────────────────────────────────────────────────────────────────

class TestCiteGraphFetchMetadata:
    """Test --fetch-metadata in plain-text mode."""

    @pytest.fixture(autouse=True)
    def setup(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")

    def test_fetch_metadata_arxiv_and_doi(self):
        args = make_args(
            paper="arXiv:1703.12345",
            plain_text="arXiv:2103.00001 and doi:10.1038/nature12373 are referenced.",
            fetch_metadata=True,
            format="json",
        )
        with patch("cli._extract_references_from_text") as mock_extract:
            mock_extract.return_value = {
                "arxiv_ids": ["2103.00001"],
                "dois": ["10.1038/nature12373"],
                "pmids": [],
                "isbns": [],
            }
            with patch("cli._fetch_arxiv_title", return_value="Example Paper"):
                with patch("cli._fetch_doi_title", return_value="DOI Paper"):
                    with patch("cli.time.sleep"):
                        captured = StringIO()
                        with patch("sys.stdout", captured):
                            rc = _run_cite_graph(args)
        assert rc == 0
        data = json.loads(captured.getvalue())
        assert data["metadata_fetched"] is True
        assert data["stats"]["titles_fetched"] == 2

    def test_fetch_metadata_requires_plain_text(self):
        """--fetch-metadata without --plain-text should error."""
        args = make_args(
            paper="arXiv:1703.12345",
            plain_text=None,
            fetch_metadata=True,
        )
        captured_err = StringIO()
        with patch("sys.stderr", captured_err):
            rc = _run_cite_graph(args)
        assert rc == 1
        assert "--plain-text" in captured_err.getvalue()

    def test_fetch_metadata_pmid_and_isbn(self):
        args = make_args(
            paper="arXiv:1703.12345",
            plain_text="PMID:12345678 and ISBN:978-0-12-345678-9 are referenced.",
            fetch_metadata=True,
            format="json",
        )
        with patch("cli._extract_references_from_text") as mock_extract:
            mock_extract.return_value = {
                "arxiv_ids": [],
                "dois": [],
                "pmids": ["12345678"],
                "isbns": ["978-0-12-345678-9"],
            }
            with patch("cli._fetch_pmid_title", return_value="PubMed Article"):
                with patch("cli._fetch_isbn_title", return_value="Book Title"):
                    with patch("cli.time.sleep"):
                        captured = StringIO()
                        with patch("sys.stdout", captured):
                            rc = _run_cite_graph(args)
        assert rc == 0
        data = json.loads(captured.getvalue())
        assert data["stats"]["pmid_count"] == 1
        assert data["stats"]["isbn_count"] == 1
        assert data["stats"]["titles_fetched"] == 2


# ─────────────────────────────────────────────────────────────────────────────
# DB mode
# ─────────────────────────────────────────────────────────────────────────────

class TestCiteGraphDBMode:
    """Test DB-backed citation graph mode."""

    @pytest.fixture(autouse=True)
    def setup(self, monkeypatch):
        monkeypatch.setenv("PYTHONHOME", "C:/Users/adm/AppData/Local/Programs/Python/Python312")
        monkeypatch.setenv("PYTHONPATH", "")

    def test_db_mode_paper_not_found(self):
        mock_db = MagicMock()
        mock_db.paper_exists.return_value = False
        with patch("cli.Database", return_value=mock_db):
            captured_err = StringIO()
            with patch("sys.stderr", captured_err):
                rc = _run_cite_graph(make_args(plain_text=None))
        assert rc == 1
        assert "not found" in captured_err.getvalue()

    def test_db_mode_depth_1(self):
        mock_db = MagicMock()
        mock_db.paper_exists.return_value = True
        mock_db.get_paper_title.return_value = "Root Paper"
        mock_db.get_citations.side_effect = lambda pid, d: (
            [MagicMock(source_id="cited:by:root", target_id="root:cites")]
            if d == "from" else
            [MagicMock(source_id="cites:root", target_id="root")]
        )
        mock_db.get_papers_bulk.return_value = {}
        with patch("cli.Database", return_value=mock_db):
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_cite_graph(make_args(plain_text=None, depth=1, max_nodes=10))
        assert rc == 0
        output = captured.getvalue()
        assert "Root Paper" in output

    def test_db_mode_depth_2(self):
        mock_db = MagicMock()
        mock_db.paper_exists.return_value = True
        mock_db.get_paper_title.return_value = "Root Paper"
        mock_db.get_citations.return_value = []
        mock_db.get_papers_bulk.return_value = {}
        with patch("cli.Database", return_value=mock_db):
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_cite_graph(make_args(plain_text=None, depth=2, max_nodes=10))
        assert rc == 0
        output = captured.getvalue()
        assert "Root Paper" in output

    def test_db_mode_json_format(self):
        mock_db = MagicMock()
        mock_db.paper_exists.return_value = True
        mock_db.get_paper_title.return_value = "Root Paper"
        mock_db.get_citations.side_effect = lambda pid, d: (
            [MagicMock(source_id="cites:root", target_id="root")] if d == "from" else []
        )
        mock_db.get_papers_bulk.return_value = {}
        with patch("cli.Database", return_value=mock_db):
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_cite_graph(make_args(plain_text=None, format="json"))
        assert rc == 0
        data = json.loads(captured.getvalue())
        assert "nodes" in data
        assert "edges" in data
        assert "root" in data
        assert data["root"] == "arXiv:1234.5678"

    def test_db_mode_mermaid_format(self):
        mock_db = MagicMock()
        mock_db.paper_exists.return_value = True
        mock_db.get_paper_title.return_value = "Root Paper"
        mock_db.get_citations.side_effect = lambda pid, d: (
            [MagicMock(source_id="cites:root", target_id="root")] if d == "from" else []
        )
        mock_db.get_papers_bulk.return_value = {}
        with patch("cli.Database", return_value=mock_db):
            captured = StringIO()
            with patch("sys.stdout", captured):
                rc = _run_cite_graph(make_args(plain_text=None, format="mermaid"))
        assert rc == 0
        output = captured.getvalue()
        assert "```mermaid" in output
        assert "graph TD" in output
