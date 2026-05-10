"""Tests for db/database.py bulk operations."""

import pytest
import time
from pathlib import Path

from db.database import Database


@pytest.fixture
def bulk_db(tmp_path: Path) -> Database:
    db = Database(db_path=str(tmp_path / "bulk.db"))
    db.init()
    return db


def test_upsert_papers_bulk_insert(bulk_db: Database) -> None:
    """Test bulk insert of new papers."""
    papers = [
        {
            "paper_id": "2201.00001",
            "title": "Paper One",
            "authors": ["Alice"],
            "abstract": "Abstract one",
            "published": "2022-01-01",
            "abs_url": "https://arxiv.org/abs/2201.00001",
            "pdf_url": "",
            "primary_category": "cs.AI",
            "doi": "",
        },
        {
            "paper_id": "2201.00002",
            "title": "Paper Two",
            "authors": ["Bob", "Carol"],
            "abstract": "Abstract two",
            "published": "2022-01-02",
            "abs_url": "https://arxiv.org/abs/2201.00002",
            "pdf_url": "",
            "primary_category": "cs.LG",
            "doi": "",
        },
    ]
    inserted, updated = bulk_db.upsert_papers_bulk(papers, "test")
    assert inserted == 2
    assert updated == 0

    # Verify records
    result = bulk_db.get_papers_bulk(["2201.00001", "2201.00002"])
    assert len(result) == 2
    assert result["2201.00001"].title == "Paper One"
    assert result["2201.00002"].authors == ["Bob", "Carol"]


def test_upsert_papers_bulk_update(bulk_db: Database) -> None:
    """Test bulk update of existing papers."""
    # Insert first
    papers = [
        {
            "paper_id": "2201.00003",
            "title": "Original Title",
            "authors": ["Alice"],
            "abstract": "Original abstract",
            "published": "2022-01-01",
            "abs_url": "https://arxiv.org/abs/2201.00003",
            "pdf_url": "",
            "primary_category": "cs.AI",
            "doi": "",
        },
    ]
    inserted, updated = bulk_db.upsert_papers_bulk(papers, "test")
    assert inserted == 1
    assert updated == 0

    # Update with new data
    updated_papers = [
        {
            "paper_id": "2201.00003",
            "title": "Updated Title",
            "authors": ["Alice", "Bob"],
            "abstract": "Updated abstract",
            "published": "2022-01-01",
            "abs_url": "https://arxiv.org/abs/2201.00003",
            "pdf_url": "",
            "primary_category": "cs.AI",
            "doi": "",
        },
    ]
    inserted2, updated2 = bulk_db.upsert_papers_bulk(updated_papers, "test")
    assert inserted2 == 0
    assert updated2 == 1

    result = bulk_db.get_paper("2201.00003")
    assert result is not None
    assert result.title == "Updated Title"
    assert result.authors == ["Alice", "Bob"]


def test_upsert_papers_bulk_mixed(bulk_db: Database) -> None:
    """Test bulk with mix of insert and update."""
    # Insert 2
    papers = [
        {"paper_id": "2201.00004", "title": "A", "authors": [], "abstract": "", "published": "", "abs_url": "", "pdf_url": "", "primary_category": "", "doi": ""},
        {"paper_id": "2201.00005", "title": "B", "authors": [], "abstract": "", "published": "", "abs_url": "", "pdf_url": "", "primary_category": "", "doi": ""},
    ]
    i, u = bulk_db.upsert_papers_bulk(papers, "test")
    assert i == 2 and u == 0

    # Insert 1, update 1
    mixed = [
        {"paper_id": "2201.00004", "title": "A-updated", "authors": [], "abstract": "", "published": "", "abs_url": "", "pdf_url": "", "primary_category": "", "doi": ""},
        {"paper_id": "2201.00006", "title": "C", "authors": [], "abstract": "", "published": "", "abs_url": "", "pdf_url": "", "primary_category": "", "doi": ""},
    ]
    i2, u2 = bulk_db.upsert_papers_bulk(mixed, "test")
    assert i2 == 1 and u2 == 1

    result = bulk_db.get_papers_bulk(["2201.00004", "2201.00005", "2201.00006"])
    assert len(result) == 3
    assert result["2201.00004"].title == "A-updated"


def test_upsert_papers_bulk_empty(bulk_db: Database) -> None:
    """Test empty list."""
    i, u = bulk_db.upsert_papers_bulk([], "test")
    assert i == 0 and u == 0


def test_upsert_papers_bulk_idempotent(bulk_db: Database) -> None:
    """Test running bulk upsert twice with same data is idempotent."""
    papers = [
        {"paper_id": "2201.00007", "title": "X", "authors": [], "abstract": "", "published": "", "abs_url": "", "pdf_url": "", "primary_category": "", "doi": ""},
    ]
    bulk_db.upsert_papers_bulk(papers, "test")
    bulk_db.upsert_papers_bulk(papers, "test")
    result = bulk_db.get_papers_bulk(["2201.00007"])
    assert len(result) == 1
