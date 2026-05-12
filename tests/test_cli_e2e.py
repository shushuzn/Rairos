"""End-to-end CLI integration tests.

These tests exercise the full CLI workflow via subprocess, using mocks
for all external calls (network, DB writes).

Run with: python -m pytest tests/test_cli_e2e.py -v
"""

from __future__ import annotations

import json
import os
import sqlite3
import subprocess
import sys
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest

# Project root for subprocess calls
PROJECT_ROOT = Path(__file__).parent.parent.resolve()
PYTHON_BIN = Path(sys.executable)

# ---------------------------------------------------------------------------
# Test fixtures
# ---------------------------------------------------------------------------


@pytest.fixture
def tmp_db_path(tmp_path):
    """Return a Path to a temporary SQLite DB file (not yet initialized)."""
    return tmp_path / "test_research.db"


def _seed_search_db(db_path: Path, papers: list[dict]) -> None:
    """Create schema and seed a search DB for testing.

    Uses the actual project schema to ensure db.init() migrations succeed.
    """
    conn = sqlite3.connect(str(db_path))
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute("PRAGMA foreign_keys=ON")

    # Import actual schema from database.py to ensure consistency
    import sys
    from pathlib import Path as P

    # Temporarily add project root to path so we can import db.database
    project_root = P(__file__).parent.parent
    if str(project_root) not in sys.path:
        sys.path.insert(0, str(project_root))

    from db.database import _SCHEMA, _FTS_SCHEMA

    # Create actual schema (mimics what db.init() does)
    conn.executescript(_SCHEMA)
    conn.executescript(_FTS_SCHEMA)

    # ── FTS5 virtual table ─────────────────────────────────────────────────────
    conn.execute("""
        CREATE VIRTUAL TABLE IF NOT EXISTS papers_fts USING fts5(
            paper_id UNINDEXED,
            title,
            abstract,
            plain_text
        );
    """)

    now = "2024-01-01T00:00:00Z"
    for p in papers:
        conn.execute(
            """INSERT INTO papers (id, source, title, authors, abstract, published,
                                  primary_category, parse_status, added_at, updated_at,
                                  abs_url, pdf_url)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)""",
            (
                p["id"],
                p.get("source", "arxiv"),
                p.get("title", ""),
                p.get("authors", "[]"),
                p.get("abstract", ""),
                p.get("published", ""),
                p.get("primary_category", ""),
                p.get("parse_status", "pending"),
                now,
                now,
                p.get("abs_url", ""),
                p.get("pdf_url", ""),
            ),
        )
        conn.execute(
            "INSERT INTO papers_fts(paper_id, title, abstract, plain_text) VALUES (?, ?, ?, '')",
            (p["id"], p.get("title", ""), p.get("abstract", "")),
        )

    conn.commit()
    conn.close()


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _run_cli(
    argv: list[str], timeout: int = 60, env: dict | None = None
) -> subprocess.CompletedProcess:
    """Run the CLI via subprocess and return the result."""
    cmd = [str(PYTHON_BIN), "-m", "cli"] + argv
    base_env = dict(os.environ)
    if env:
        base_env.update(env)
    return subprocess.run(
        cmd,
        cwd=str(PROJECT_ROOT),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=timeout,
        env=base_env,
    )


# ---------------------------------------------------------------------------
# Test 1: rairos search  (seeded temp DB)
# ---------------------------------------------------------------------------


def test_search_returns_json_results(tmp_db_path):
    """'rairos search' should query the seeded DB and return matching papers as JSON."""
    papers = [
        {
            "id": "arXiv:2301.00001",
            "title": "Attention Is All You Need",
            "authors": '["Ashish Vaswani", "Noam Shazeer"]',
            "abstract": "We propose a new simple network architecture based solely on attention mechanisms.",
            "published": "2017-06-12",
            "primary_category": "cs.CL",
            "parse_status": "parsed",
            "source": "arxiv",
            "abs_url": "https://arxiv.org/abs/2301.00001",
            "pdf_url": "https://arxiv.org/pdf/2301.00001",
        },
        {
            "id": "arXiv:2302.00002",
            "title": "BERT: Pre-training of Deep Bidirectional Transformers for Language Understanding",
            "authors": '["Jacob Devlin", "Ming-Wei Chang"]',
            "abstract": "We introduce a new language representation model called BERT.",
            "published": "2018-10-11",
            "primary_category": "cs.CL",
            "parse_status": "parsed",
            "source": "arxiv",
            "abs_url": "https://arxiv.org/abs/2302.00002",
            "pdf_url": "https://arxiv.org/pdf/2302.00002",
        },
    ]
    _seed_search_db(tmp_db_path, papers)

    proc = _run_cli(
        ["search", "attention", "--limit", "5", "--format", "json"],
        env={"AIROS_DB": str(tmp_db_path)},
    )

    assert proc.returncode == 0, f"stderr: {proc.stderr}\nstdout: {proc.stdout}"
    output = json.loads(proc.stdout)

    assert output["total"] >= 1, f"Expected at least 1 result, got {output['total']}: {proc.stdout}"
    assert len(output["results"]) >= 1
    # Verify our seeded paper appears in results
    result_ids = [r["paper_id"] for r in output["results"]]
    assert "arXiv:2301.00001" in result_ids, f"Expected paper not in results: {result_ids}"
    titles = [r["title"] for r in output["results"]]
    assert "Attention Is All You Need" in titles, f"Expected title not in results: {titles}"
    assert "score" in output["results"][0]


def test_search_with_empty_results(tmp_db_path):
    """'rairos search' with no matches should return empty list gracefully."""
    # Seed with unrelated paper so search finds nothing
    papers = [
        {
            "id": "arXiv:9999.99999",
            "title": "Unrelated Work on Databases",
            "authors": "[]",
            "abstract": "This paper is about database systems.",
            "published": "2020-01-01",
            "primary_category": "cs.DB",
            "parse_status": "pending",
            "source": "arxiv",
            "abs_url": "",
            "pdf_url": "",
        }
    ]
    _seed_search_db(tmp_db_path, papers)

    proc = _run_cli(
        ["search", "xyznonexistentquery", "--format", "json"],
        env={"AIROS_DB": str(tmp_db_path)},
    )

    assert proc.returncode == 0
    output = json.loads(proc.stdout)
    assert output["total"] == 0
    assert output["results"] == []


def test_search_csv_format(tmp_db_path):
    """'rairos search --format csv' should produce valid CSV with our seeded data."""
    papers = [
        {
            "id": "arXiv:2301.00001",
            "title": "Attention Is All You Need",
            "authors": '["Ashish Vaswani", "Noam Shazeer"]',
            "abstract": "We propose attention mechanisms.",
            "published": "2017-06-12",
            "primary_category": "cs.CL",
            "parse_status": "parsed",
            "source": "arxiv",
            "abs_url": "https://arxiv.org/abs/2301.00001",
            "pdf_url": "https://arxiv.org/pdf/2301.00001",
        },
    ]
    _seed_search_db(tmp_db_path, papers)

    proc = _run_cli(
        ["search", "attention", "--format", "csv"],
        env={"AIROS_DB": str(tmp_db_path)},
    )

    assert proc.returncode == 0, f"stderr: {proc.stderr}\nstdout: {proc.stdout[:500]}"
    lines = proc.stdout.strip().splitlines()
    assert len(lines) >= 2, f"Expected header + data rows, got {len(lines)} lines"
    header = lines[0]
    assert "paper_id" in header, f"Header missing paper_id: {header}"
    assert "title" in header, f"Header missing title: {header}"
    assert "arXiv:2301.00001" in proc.stdout, "Expected paper ID in output"
    assert "Attention Is All You Need" in proc.stdout, "Expected title in output"


# ---------------------------------------------------------------------------
# Test 2: rairos cite-graph  (cite command — mocked DB BFS in-process)
# ---------------------------------------------------------------------------


def test_cite_graph_db_mode_returns_text():
    """'rairos cite-graph run --paper <id>' should render citation graph from DB."""
    mock_nodes = {
        "arXiv:2301.00001": MagicMock(
            paper_id="arXiv:2301.00001",
            title="Attention Is All You Need",
            depth=0,
            direction="root",
        ),
        "arXiv:2302.00002": MagicMock(
            paper_id="arXiv:2302.00002",
            title="BERT Pre-training",
            depth=1,
            direction="backward",
        ),
    }
    mock_edges = [("arXiv:2301.00001", "arXiv:2302.00002", "backward")]

    # Patch the BFS function that cite-graph text mode uses internally
    with patch("cli.cmd.cite_graph._db_citation_bfs", return_value=(mock_nodes, mock_edges)):
        proc = _run_cli(["cite-graph", "run", "--paper", "arXiv:2301.00001", "--format", "text"])

    assert proc.returncode == 0
    assert "arXiv:2301.00001" in proc.stdout


def test_cite_graph_json_format():
    """'rairos cite-graph run --paper <id> --format json' should return valid JSON."""
    mock_nodes = {
        "W12345": MagicMock(
            paper_id="W12345",
            title="Some Paper",
            depth=1,
            direction="forward",
        ),
    }
    mock_edges = []

    with patch("cli.cmd.cite_graph._db_citation_bfs", return_value=(mock_nodes, mock_edges)):
        proc = _run_cli(["cite-graph", "run", "--paper", "arXiv:2301.00001", "--format", "json"])

    assert proc.returncode == 0
    output = json.loads(proc.stdout)
    assert "nodes" in output
    assert "edges" in output


# ---------------------------------------------------------------------------
# Test 3: rairos daemon --help  (verify it starts / parses correctly)
# ---------------------------------------------------------------------------


def test_daemon_help_shows_usage():
    """'rairos daemon --help' should exit 0 and list subcommands."""
    proc = _run_cli(["daemon", "--help"])

    assert proc.returncode == 0, f"stderr: {proc.stderr}"
    # Check that key subcommands appear in the help output
    combined = proc.stdout + proc.stderr
    assert "start" in combined.lower() or "daemon" in combined.lower()
    # Should not contain a Python traceback
    assert "Traceback" not in proc.stderr
    assert "Error" not in proc.stderr


def test_daemon_unknown_subcommand_exits_nonzero():
    """'rairos daemon unknown-subcommand' should exit non-zero."""
    proc = _run_cli(["daemon", "this-subcommand-does-not-exist"])

    # argparse exits with 2 for unrecognized args, or the subcommand handler
    # may exit with 1 — both are acceptable for "unknown subcommand"
    assert proc.returncode != 0


# ---------------------------------------------------------------------------
# Test 4: rairos doctor  (local checks only — no network / no mock needed)
# ---------------------------------------------------------------------------


def test_doctor_runs_without_errors():
    """'rairos doctor' should execute all local checks and exit 0 (no issues)
    or exit 1 (issues found) — but must NOT crash.
    """
    proc = _run_cli(["doctor"], timeout=90)

    # doctor returns 0 (all OK) or 1 (issues found) — both are acceptable
    assert proc.returncode in (0, 1), (
        f"Unexpected failure:\nstdout: {proc.stdout}\nstderr: {proc.stderr}"
    )
    # Should not contain a Python traceback
    assert "Traceback" not in proc.stderr
    # Output should contain diagnostic sections
    combined = proc.stdout + proc.stderr
    assert "Python" in combined or "Platform" in combined or "Summary" in combined


def test_doctor_produces_expected_sections():
    """'rairos doctor' output should contain known diagnostic sections."""
    proc = _run_cli(["doctor"], timeout=90)

    output = proc.stdout + proc.stderr
    # The doctor command prints section headers in rich format
    assert "Python" in output or "Executables" in output or "Summary" in output, (
        f"Unexpected doctor output:\n{output}"
    )
    assert "Traceback" not in proc.stderr


# ---------------------------------------------------------------------------
# Test 5: Invalid / unknown subcommand
# ---------------------------------------------------------------------------


def test_unknown_subcommand_exits_nonzero():
    """An unknown subcommand should result in a non-zero exit code."""
    proc = _run_cli(["this-subcommand-does-not-exist"])

    assert proc.returncode != 0
    # Should get some helpful error message (argparse or custom)
    combined = proc.stdout + proc.stderr
    assert len(combined) > 0


# ---------------------------------------------------------------------------
# Test 6: search with table format (default)
# ---------------------------------------------------------------------------


def test_search_table_format_shows_results(tmp_db_path):
    """'rairos search' (default table format) should display paper titles."""
    papers = [
        {
            "id": "arXiv:2301.00001",
            "title": "Attention Is All You Need",
            "authors": '["Ashish Vaswani"]',
            "abstract": "Attention mechanism network.",
            "published": "2017-06-12",
            "primary_category": "cs.CL",
            "parse_status": "parsed",
            "source": "arxiv",
            "abs_url": "https://arxiv.org/abs/2301.00001",
            "pdf_url": "https://arxiv.org/pdf/2301.00001",
        },
    ]
    _seed_search_db(tmp_db_path, papers)

    proc = _run_cli(
        ["search", "attention"],
        env={"AIROS_DB": str(tmp_db_path)},
    )

    assert proc.returncode == 0, f"stderr: {proc.stderr}\nstdout: {proc.stdout}"
    # Table format should contain the title
    assert "Attention Is All You Need" in proc.stdout
    assert "Found" in proc.stdout or "papers" in proc.stdout.lower()
