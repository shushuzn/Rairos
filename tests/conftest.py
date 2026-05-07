"""Shared fixtures for tests."""

from __future__ import annotations

import importlib
import json
import sqlite3
import sys
import tempfile
import types as types_
from pathlib import Path
from unittest.mock import MagicMock

import pytest


def _is_fake_stub(mod) -> bool:
    """Return True if mod is a hand-crafted stub without a real module loader."""
    if not isinstance(mod, types_.ModuleType):
        return False
    return not hasattr(mod, "__loader__")


def _cleanup_fake_stubs() -> None:
    """Remove any fake stub modules from sys.modules that would pollute later tests.

    Also removes the 'research_loop' package itself when its 'core' submodule is a
    fake stub, ensuring the package gets re-imported cleanly by subsequent tests.
    """
    stubs_to_remove = []
    for name in list(sys.modules.keys()):
        mod = sys.modules[name]
        if mod is not None and _is_fake_stub(mod):
            stubs_to_remove.append(name)
    for name in stubs_to_remove:
        sys.modules.pop(name, None)

    # If research_loop.core was a fake stub, also remove the research_loop
    # package so it gets re-imported cleanly
    if "research_loop.core" in stubs_to_remove:
        # Remove all research_loop submodules
        for name in list(sys.modules.keys()):
            if name.startswith("research_loop."):
                sys.modules.pop(name, None)
        # Remove the package itself
        sys.modules.pop("research_loop", None)


# ─────────────────────────────────────────────────────────────────────────────
# Lazy-load mocking: intercept fitz/pymupdf before tests import them.
# This avoids loading the heavy PyMuPDF/pdfminer libraries during test collection.
# ─────────────────────────────────────────────────────────────────────────────


def pytest_configure(config: pytest.Config) -> None:
    """Pre-import hooks: swap heavy libs with lightweight mocks at collect time."""
    import threading

    config.addinivalue_line("markers", "lean: tests that require Lean 4 installed")
    config.addinivalue_line("markers", "ollama: tests that require Ollama embedding service")
    # Increase thread stack size to prevent stack overflow on Windows with many tests
    try:
        threading.stack_size(2 * 1024 * 1024)  # 2MB instead of default ~1MB
    except Exception:
        pass  # May fail on some platforms
    # Pre-import sklearn early to avoid C stack overflow when many modules are loaded
    # This must happen before any mock registration
    try:
        import sklearn.utils._array_api  # type: ignore
    except Exception:
        pass
    # Create mock fitz/pymupdf that satisfy basic type checks
    mock_fitz = MagicMock(name="fitz")
    mock_fitz.open.return_value = MagicMock(
        __enter__=MagicMock(return_value=mock_fitz),
        __exit__=MagicMock(return_value=False),
        page_count=0,
        tobytes=MemoryError,
    )

    # Register mocks BEFORE any test code runs
    sys.modules["fitz"] = mock_fitz
    sys.modules["pymupdf"] = mock_fitz

    # Register pdfminer extraction mocks
    if "pdfminer" not in sys.modules:
        sys.modules["pdfminer"] = MagicMock(name="pdfminer")
    if "pdfminer.extractor" not in sys.modules:
        sys.modules["pdfminer.extractor"] = MagicMock(name="pdfminer.extractor")
    if "pdfminer.layout" not in sys.modules:
        sys.modules["pdfminer.layout"] = MagicMock(name="pdfminer.layout")

    # Clean up any leftover fake stubs from previous pytest runs.
    # test_deep_research.py creates hand-crafted stub modules without __spec__
    # that pollute subsequent test modules.
    _cleanup_fake_stubs()


def pytest_collect_file(file_path: Path, parent) -> None:
    """Prevent test_deep_research.py from being collected.

    This file creates fake stub modules (via types.ModuleType) at import time.
    These pollute sys.modules and break patching in other test modules that
    import research_loop.core. Returning None (not pytest.skip()) prevents the
    file from being collected without skipping the entire test run.
    """
    if file_path.name == "test_deep_research.py":
        return None


def pytest_collection_modifyitems(config: pytest.Config, items: list) -> None:
    """Skip @pytest.mark.lean tests when Lean 4 is not installed, and clean up fake stubs."""
    try:
        from llm.lean_verifier import check_lean_installed

        status, _ = check_lean_installed()
        lean_available = status.value == "available"
    except Exception:
        lean_available = False

    if not lean_available:
        skip_lean = pytest.mark.skip(reason="Lean 4 not installed")
        for item in items:
            if item.get_closest_marker("lean") is not None:
                item.add_marker(skip_lean)

    # Skip ollama tests when Ollama is not available
    try:
        import urllib.request

        req = urllib.request.Request(
            "http://localhost:11434/api/tags",
            method="GET",
        )
        with urllib.request.urlopen(req, timeout=2) as resp:
            ollama_available = resp.status == 200
    except Exception:
        ollama_available = False

    if not ollama_available:
        skip_ollama = pytest.mark.skip(reason="Ollama not running on localhost:11434")
        for item in items:
            if item.get_closest_marker("ollama") is not None:
                item.add_marker(skip_ollama)

    # Skip test_deep_research.py — its module-level stubs pollute sys.modules
    # during collection. Using pytest_collection_modifyitems is too late because
    # the stubs are already registered by the time collection runs. A
    # pytest_collect_file hook that returns None prevents the file from being
    # imported at all. Note: returning pytest.skip() from pytest_collect_file
    # skips the ENTIRE test run, so we must return None.
    _deep_research_collector = None  # populated in pytest_collect_file


def pytest_sessionfinish(session, exitstatus) -> None:
    """Clean up any fake stub modules after all tests finish."""
    _cleanup_fake_stubs()


# Module cache cleanup - reset pdf.extract fitz/tesseract caches between tests
@pytest.fixture(autouse=True)
def _reset_module_caches():
    yield
    try:
        import pdf.extract

        pdf.extract._fitz_pdf = None
        pdf.extract._tesseract = None
        sys.modules.pop("fitz", None)
        sys.modules.pop("pymupdf", None)
    except (AttributeError, ImportError):
        pass


# Frozen date constants used across Tier 4 tests
FROZEN_DATE = "2024-06-15"
FROZEN_DATE_ISO = "2024-06-15"
FROZEN_YEAR = "2024"


@pytest.fixture
def frozen_date_iso() -> str:
    """Return the frozen date ISO string '2024-06-15'."""
    return FROZEN_DATE_ISO


@pytest.fixture
def frozen_year() -> str:
    """Return the frozen year constant '2024'."""
    return FROZEN_YEAR


# ---------------------------------------------------------------------------
# freezegun integration
# ---------------------------------------------------------------------------


@pytest.fixture(autouse=True)
def freeze_time_fixture(request: pytest.FixtureRequest):
    """Automatically freeze time to 2024-06-15 for every test in the session.

    Tests sensitive to wall-clock time (perf_counter, sleep) can opt out by
    decorating with @pytest.mark.no_freeze.
    """
    # Check if this test/class is marked to skip freezing
    if request.node.get_closest_marker("no_freeze") is not None:
        yield  # run without freezing
        return

    from freezegun import freeze_time as _freeze_time

    with _freeze_time(FROZEN_DATE):
        yield


def freeze_time(datetime_str: str = FROZEN_DATE):
    """Return a freezegun freeze_time context manager pinned to the given datetime string.

    Usage in tests (alternative to autouse fixture):
        @pytest.mark.freeze_time("2024-06-15")
        def test_something():
            ...
    """
    from freezegun import freeze_time as _freeze_time

    return _freeze_time(datetime_str)


# ---------------------------------------------------------------------------
# Shared fixtures
# ---------------------------------------------------------------------------


@pytest.fixture
def mock_research_root():
    """Create a temporary research directory tree via airo.ensure_research_tree."""
    import ai_research_os as airo

    with tempfile.TemporaryDirectory() as d:
        root = Path(d)
        airo.ensure_research_tree(root)
        yield root


@pytest.fixture
def sample_pdf(tmp_path):
    """Create a minimal valid PDF for testing."""
    pdf_path = tmp_path / "sample.pdf"
    # Minimal PDF structure
    pdf_path.write_bytes(
        b"%PDF-1.4\n"
        b"1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n"
        b"2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n"
        b"3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]"
        b"/Contents 4 0 R/Resources<</Font<</F1 5 0 R>>>>>>endobj\n"
        b"4 0 obj<</Length 44>>stream\n"
        b"BT /F1 12 Tf 100 700 Td (Hello World) Tj ET\n"
        b"endstream endobj\n"
        b"5 0 obj<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>endobj\n"
        b"xref\n0 6\n"
        b"0000000000 65535 f\n"
        b"0000000009 00000 n\n"
        b"0000000058 00000 n\n"
        b"0000000115 00000 n\n"
        b"0000000270 00000 n\n"
        b"0000000350 00000 n\n"
        b"trailer<</Size 6/Root 1 0 R>>\n"
        b"startxref\n"
        b"427\n"
        b"%%EOF"
    )
    return pdf_path


@pytest.fixture
def tmp_db(tmp_path):
    """Create an in-memory SQLite database for parser cache tests."""
    db_path = tmp_path / "research.db"
    conn = sqlite3.connect(str(db_path))
    conn.execute(
        "CREATE TABLE IF NOT EXISTS papers ("
        "  id TEXT PRIMARY KEY,"
        "  source TEXT NOT NULL,"
        "  title TEXT DEFAULT '',"
        "  authors TEXT DEFAULT '[]',"
        "  abstract TEXT DEFAULT '',"
        "  published TEXT DEFAULT '',"
        "  updated TEXT DEFAULT '',"
        "  abs_url TEXT DEFAULT '',"
        "  pdf_url TEXT DEFAULT '',"
        "  primary_category TEXT DEFAULT '',"
        "  journal TEXT DEFAULT '',"
        "  volume TEXT DEFAULT '',"
        "  issue TEXT DEFAULT '',"
        "  page TEXT DEFAULT '',"
        "  doi TEXT DEFAULT '',"
        "  categories TEXT DEFAULT '',"
        "  reference_count INTEGER DEFAULT 0,"
        "  added_at TEXT NOT NULL,"
        "  updated_at TEXT NOT NULL,"
        "  pdf_path TEXT DEFAULT '',"
        "  pdf_hash TEXT DEFAULT '',"
        "  parse_status TEXT DEFAULT 'pending',"
        "  parse_error TEXT DEFAULT '',"
        "  parse_version INTEGER DEFAULT 0,"
        "  plain_text TEXT DEFAULT '',"
        "  latex_blocks TEXT DEFAULT '[]',"
        "  table_count INTEGER DEFAULT 0,"
        "  figure_count INTEGER DEFAULT 0,"
        "  word_count INTEGER DEFAULT 0,"
        "  page_count INTEGER DEFAULT 0,"
        "  pnote_path TEXT DEFAULT '',"
        "  cnote_path TEXT DEFAULT '',"
        "  mnote_path TEXT DEFAULT '',"
        "  embed_vector BLOB DEFAULT NULL"
        ")"
    )
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute("PRAGMA foreign_keys=ON")
    conn.commit()

    def get_embedding(paper_id):
        cur = conn.cursor()
        cur.execute("SELECT embed_vector FROM papers WHERE id = ?", (paper_id,))
        row = cur.fetchone()
        if row is None or row["embed_vector"] is None:
            return None
        import struct

        blob = row["embed_vector"]
        count = len(blob) // 4
        return list(struct.unpack(f"{count}f", blob))

    def get_paper(paper_id):
        cur = conn.cursor()
        cur.execute("SELECT * FROM papers WHERE id = ?", (paper_id,))
        row = cur.fetchone()
        if row is None:
            return None
        columns = [desc[0] for desc in cur.description]
        return dict(zip(columns, row))

    def update_parse_status(
        paper_id,
        status,
        error="",
        plain_text="",
        latex_blocks=None,
        table_count=0,
        figure_count=0,
        word_count=0,
        page_count=0,
    ):
        latex_json = (
            json.dumps(latex_blocks or []) if not isinstance(latex_blocks, str) else latex_blocks
        )
        cur = conn.cursor()
        cur.execute(
            "SELECT parse_version FROM papers WHERE id = ?",
            (paper_id,),
        )
        row = cur.fetchone()
        version = (row["parse_version"] if row else 0) + 1
        cur.execute(
            "UPDATE papers SET parse_status=?, parse_error=?, plain_text=?, "
            "latex_blocks=?, table_count=?, figure_count=?, word_count=?, "
            "page_count=?, parse_version=? WHERE id=?",
            (
                status,
                error,
                plain_text,
                latex_json,
                table_count,
                figure_count,
                word_count,
                page_count,
                version,
                paper_id,
            ),
        )
        conn.commit()

    class _FakeDB:
        pass

    _FakeDB.get_embedding = staticmethod(get_embedding)
    _FakeDB.get_paper = staticmethod(get_paper)
    _FakeDB.update_parse_status = staticmethod(update_parse_status)
    return _FakeDB()
