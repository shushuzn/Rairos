"""Tests for db/database.py — reflects actual API."""
from __future__ import annotations

import sqlite3
from pathlib import Path

import pytest

from core.exceptions import DatabaseError
from db import Database


@pytest.fixture
def db(tmp_path):
    d = Database(tmp_path / "research.db")
    d.init()
    return d


class TestPapers:
    def test_upsert_and_get_paper(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="Attention Is All You Need")
        paper = db.get_paper("2301.00001")
        assert paper is not None
        assert paper.id == "2301.00001"
        assert paper.title == "Attention Is All You Need"

    def test_upsert_paper_updates_existing(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="Original")
        db.upsert_paper("2301.00001", "arxiv", title="Updated")
        paper = db.get_paper("2301.00001")
        assert paper.title == "Updated"

    def test_get_paper_returns_none_for_missing(self, db):
        assert db.get_paper("does-not-exist") is None

    def test_paper_count(self, db):
        assert db.paper_count() == 0
        db.upsert_paper("2301.00001", "arxiv")
        assert db.paper_count() == 1
        db.upsert_paper("2301.00002", "arxiv")
        assert db.paper_count() == 2


class TestTags:
    def test_add_and_get_tags(self, db):
        db.upsert_paper("2301.00001", "arxiv")
        db.add_tag("2301.00001", "nlp")
        db.add_tag("2301.00001", "transformer")
        tags = db.get_tags("2301.00001")
        assert set(tags) == {"nlp", "transformer"}

    def test_remove_tag(self, db):
        db.upsert_paper("2301.00001", "arxiv")
        db.add_tag("2301.00001", "nlp")
        db.add_tag("2301.00001", "transformer")
        db.remove_tag("2301.00001", "nlp")
        tags = db.get_tags("2301.00001")
        assert "nlp" not in tags
        assert "transformer" in tags

    def test_list_all_tags(self, db):
        db.upsert_paper("2301.00001", "arxiv")
        db.upsert_paper("2301.00002", "arxiv")
        db.add_tag("2301.00001", "nlp")
        db.add_tag("2301.00002", "nlp")
        db.add_tag("2301.00002", "vision")
        all_t = db.list_all_tags()
        assert set(all_t) == {"nlp", "vision"}

    def test_papers_by_tag(self, db):
        db.upsert_paper("2301.00001", "arxiv")
        db.upsert_paper("2301.00002", "arxiv")
        db.add_tag("2301.00001", "nlp")
        db.add_tag("2301.00002", "nlp")
        papers = db.papers_by_tag("nlp")
        assert len(papers) == 2


class TestJobQueue:
    def test_enqueue_job(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="T")
        job_id = db.enqueue_job("2301.00001", "parse")
        assert job_id > 0

    def test_dequeue_job(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="T")
        db.enqueue_job("2301.00001", "parse")
        row = db.dequeue_job()
        assert row is not None
        assert row["paper_id"] == "2301.00001"

    def test_dequeue_empty_queue(self, db):
        assert db.dequeue_job() is None

    def test_complete_job(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="T")
        job_id = db.enqueue_job("2301.00001", "parse")
        db.complete_job(job_id, "done")
        cur = db.conn.cursor()
        cur.execute("SELECT status, completed_at FROM job_queue WHERE id = ?", (job_id,))
        row = cur.fetchone()
        assert row["status"] == "done"
        assert row["completed_at"] != ""

    def test_queue_depth(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="P1")
        db.upsert_paper("2301.00002", "arxiv", title="P2")
        db.enqueue_job("2301.00001", "parse")
        db.enqueue_job("2301.00002", "parse")
        assert db.queue_depth() == 2


class TestSearch:
    def test_search_papers(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="Attention Is All You Need")
        db.upsert_paper("2301.00002", "arxiv", title="BERT Pre-training")
        results = db.search_papers("attention")
        assert len(results) >= 1
        assert any(r.id == "2301.00001" for r in results)


class TestSettings:
    def test_set_and_get_setting(self, db):
        db.set_setting("api_key", "sk-test")
        assert db.get_setting("api_key") == "sk-test"
        assert db.get_setting("missing", "default") == "default"


class TestParseHistory:
    def test_record_and_get_parse_history(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="T")
        db.record_parse_attempt("2301.00001", duration_sec=1.5, status="success", pdf_hash="abc123", file_size=1024)
        history = db.get_parse_history("2301.00001")
        assert len(history) == 1
        assert history[0]["duration_sec"] == 1.5
        assert history[0]["status"] == "success"



