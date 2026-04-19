"""Tests for db/database.py — reflects actual API."""
from __future__ import annotations


import pytest

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
        results, total = db.search_papers("attention")
        assert total >= 1
        assert any("attention" in r.title.lower() for r in results)
        assert all(isinstance(r.paper_id, str) for r in results)


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




class TestUpdateParseStatus:
    def test_update_parse_status_success(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="Test Paper")
        db.update_parse_status("2301.00001", status="success", plain_text="Full text", latex_blocks=["b1","b2"], table_count=3, figure_count=5, word_count=1000, page_count=10)
        paper = db.get_paper("2301.00001")
        assert paper.parse_status == "success"
        assert paper.plain_text == "Full text"
        assert paper.latex_blocks == ["b1","b2"]
        assert paper.table_count == 3
        assert paper.figure_count == 5
        assert paper.word_count == 1000
        assert paper.page_count == 10
        assert paper.parse_version == 1

    def test_update_parse_status_increments_version(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="Test Paper")
        db.update_parse_status("2301.00001", status="success", plain_text="v1")
        db.update_parse_status("2301.00001", status="success", plain_text="v2")
        paper = db.get_paper("2301.00001")
        assert paper.parse_version == 2
        assert paper.plain_text == "v2"

    def test_update_parse_status_failure(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="Test Paper")
        db.update_parse_status("2301.00001", status="failed", error="PDF unreadable")
        paper = db.get_paper("2301.00001")
        assert paper.parse_status == "failed"
        assert paper.parse_error == "PDF unreadable"

    def test_update_parse_status_nonexistent_paper(self, db):
        db.update_parse_status("does-not-exist", status="success")

    def test_update_parse_status_with_empty_latex(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="Test Paper")
        db.update_parse_status("2301.00001", status="success", latex_blocks=[])
        paper = db.get_paper("2301.00001")
        assert paper.latex_blocks == []


class TestMergePapers:
    def test_merge_papers_basic(self, db):
        db.upsert_paper("target-001", "arxiv", title="Target Paper", doi="10.1234/test")
        db.upsert_paper("dup-001", "arxiv", title="Duplicate Paper", doi="10.1234/test")
        db.add_tag("dup-001", "vision")
        db.enqueue_job("dup-001", "parse")
        result = db.merge_papers("target-001", "dup-001")
        assert result is True
        assert db.get_paper("target-001") is not None
        assert db.get_paper("dup-001") is None
        tags = db.get_tags("target-001")
        assert "vision" in tags
        assert db.queue_depth("queued") == 1

    def test_merge_papers_fills_empty_parse_fields(self, db):
        db.upsert_paper("target-001", "arxiv", title="Target Paper")
        db.upsert_paper("dup-001", "arxiv", title="Dup Paper")
        db.update_parse_status("dup-001", status="success", plain_text="Full text", word_count=500)
        db.merge_papers("target-001", "dup-001")
        paper = db.get_paper("target-001")
        assert paper.plain_text == "Full text"
        assert paper.word_count == 500

    def test_merge_papers_does_not_overwrite_filled_fields(self, db):
        db.upsert_paper("target-001", "arxiv", title="Target Paper")
        db.update_parse_status("target-001", status="success", plain_text="Original text")
        db.upsert_paper("dup-001", "arxiv", title="Dup Paper")
        db.update_parse_status("dup-001", status="success", plain_text="New text")
        db.merge_papers("target-001", "dup-001")
        paper = db.get_paper("target-001")
        assert paper.plain_text == "Original text"

    def test_merge_papers_nonexistent_duplicate(self, db):
        db.upsert_paper("target-001", "arxiv", title="Target Paper")
        result = db.merge_papers("target-001", "nonexistent-dup")
        assert result is False

    def test_merge_papers_preserves_target_tags(self, db):
        db.upsert_paper("target-001", "arxiv", title="Target Paper")
        db.upsert_paper("dup-001", "arxiv", title="Dup Paper")
        db.add_tag("target-001", "nlp")
        db.add_tag("dup-001", "vision")
        db.merge_papers("target-001", "dup-001")
        tags = db.get_tags("target-001")
        assert "nlp" in tags
        assert "vision" in tags


class TestGetPapersBulk:
    def test_get_papers_bulk_empty(self, db):
        result = db.get_papers_bulk([])
        assert result == {}

    def test_get_papers_bulk_single(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="Paper One")
        result = db.get_papers_bulk(["2301.00001"])
        assert len(result) == 1
        assert result["2301.00001"].title == "Paper One"

    def test_get_papers_bulk_multiple(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="Paper One")
        db.upsert_paper("2301.00002", "arxiv", title="Paper Two")
        db.upsert_paper("2301.00003", "arxiv", title="Paper Three")
        result = db.get_papers_bulk(["2301.00001", "2301.00003"])
        assert len(result) == 2
        assert "2301.00001" in result
        assert "2301.00003" in result
        assert "2301.00002" not in result

    def test_get_papers_bulk_missing_ids(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="Paper One")
        result = db.get_papers_bulk(["2301.00001", "nonexistent"])
        assert len(result) == 1
        assert "2301.00001" in result
        assert "nonexistent" not in result


class TestListPapers:
    def test_list_papers_default(self, db):
        for i in range(5):
            db.upsert_paper("2301.{i:05d}".format(i=i), "arxiv", title="Paper {i}".format(i=i))
        papers, total = db.list_papers()
        assert total == 5
        assert len(papers) == 5

    def test_list_papers_limit_offset(self, db):
        for i in range(5):
            db.upsert_paper("2301.{i:05d}".format(i=i), "arxiv", title="Paper {i}".format(i=i))
        papers, total = db.list_papers(limit=2, offset=1)
        assert total == 5
        assert len(papers) == 2

    def test_list_papers_filter_by_source(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="ArXiv Paper")
        db.upsert_paper("2301.00002", "semantic", title="Semantic Paper")
        papers, total = db.list_papers(source="arxiv")
        assert total == 1
        assert papers[0].source == "arxiv"

    def test_list_papers_filter_by_parse_status(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="Paper 1")
        db.upsert_paper("2301.00002", "arxiv", title="Paper 2")
        db.update_parse_status("2301.00001", status="success", plain_text="done")
        papers, total = db.list_papers(parse_status="success")
        assert total == 1
        assert papers[0].id == "2301.00001"

    def test_list_papers_filter_by_category(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="Paper 1", primary_category="cs.AI")
        db.upsert_paper("2301.00002", "arxiv", title="Paper 2", primary_category="cs.LG")
        papers, total = db.list_papers(category="cs.AI")
        assert total == 1
        assert papers[0].primary_category == "cs.AI"

    def test_list_papers_sort_order(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="Alpha")
        db.upsert_paper("2301.00002", "arxiv", title="Beta")
        papers_asc, _ = db.list_papers(sort_by="title", sort_order="asc")
        papers_desc, _ = db.list_papers(sort_by="title", sort_order="desc")
        assert papers_asc[0].title == "Alpha"
        assert papers_desc[0].title == "Beta"

    def test_list_papers_empty(self, db):
        papers, total = db.list_papers()
        assert total == 0
        assert papers == []


class TestGetStats:
    def test_get_stats_empty(self, db):
        stats = db.get_stats()
        assert stats["total_papers"] == 0
        assert stats["by_source"] == {}
        assert stats["by_status"] == {}

    def test_get_stats_basic(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="Paper 1")
        db.upsert_paper("2301.00002", "semantic", title="Paper 2")
        db.update_parse_status("2301.00001", status="success", plain_text="text")
        stats = db.get_stats()
        assert stats["total_papers"] == 2
        assert stats["by_source"]["arxiv"] == 1
        assert stats["by_source"]["semantic"] == 1
        assert stats["by_status"]["success"] == 1
        assert stats["by_status"]["pending"] == 1

    def test_get_stats_queue(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="P1")
        db.upsert_paper("2301.00002", "arxiv", title="P2")
        db.enqueue_job("2301.00001", "parse")
        db.enqueue_job("2301.00002", "parse")
        stats = db.get_stats()
        assert stats["queue_queued"] == 2
        assert stats["queue_running"] == 0

    def test_get_stats_cache_and_dedup(self, db):
        db.set_cached_paper("uid1", {"data": "test"})
        db.upsert_paper("target", "arxiv", title="T")
        stats = db.get_stats()
        assert stats["cache_entries"] == 1
        assert stats["dedup_records"] == 0


class TestPaperCount:
    def test_paper_count_with_status(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="Paper 1")
        db.upsert_paper("2301.00002", "arxiv", title="Paper 2")
        db.update_parse_status("2301.00001", status="success", plain_text="text")
        assert db.paper_count() == 2
        assert db.paper_count(status="success") == 1
        assert db.paper_count(status="pending") == 1
        assert db.paper_count(status="failed") == 0

    def test_paper_count_nonexistent_status(self, db):
        assert db.paper_count(status="nonexistent_status") == 0


class TestSearchEdgeCases:
    def test_search_papers_empty_results(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="Attention Is All You Need")
        results, total = db.search_papers("xyznonexistentquery123")
        assert results == []
        assert total == 0

    def test_search_papers_filter_by_source(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="ArXiv Paper")
        db.upsert_paper("2301.00002", "semantic", title="Semantic Paper")
        db.upsert_paper("2301.00003", "arxiv", title="Another ArXiv Paper")
        results, total = db.search_papers("paper", source="arxiv")
        assert total == 2
        assert all(r.source == "arxiv" for r in results)

    def test_search_papers_filter_by_category(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="AI Paper", primary_category="cs.AI")
        db.upsert_paper("2301.00002", "arxiv", title="LG Paper", primary_category="cs.LG")
        results, total = db.search_papers("paper", category="cs.AI")
        assert total == 1
        assert results[0].primary_category == "cs.AI"

    def test_search_papers_filter_by_parse_status(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="Parsed Paper")
        db.upsert_paper("2301.00002", "arxiv", title="Unparsed Paper")
        db.update_parse_status("2301.00001", status="success", plain_text="done")
        results, total = db.search_papers("paper", parse_status="success")
        assert total == 1
        assert results[0].paper_id == "2301.00001"

    def test_search_papers_date_range(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="Paper 2023", published="2023-01-01")
        db.upsert_paper("2301.00002", "arxiv", title="Paper 2024", published="2024-01-01")
        db.upsert_paper("2301.00003", "arxiv", title="Paper 2025", published="2025-01-01")
        results, total = db.search_papers("paper", date_from="2024-01-01", date_to="2024-12-31")
        assert total == 1
        assert results[0].paper_id == "2301.00002"

    def test_search_papers_date_from_only(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="Old Paper", published="2023-01-01")
        db.upsert_paper("2301.00002", "arxiv", title="New Paper", published="2025-01-01")
        results, total = db.search_papers("paper", date_from="2024-01-01")
        assert total == 1
        assert results[0].paper_id == "2301.00002"

    def test_search_papers_date_to_only(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="Old Paper", published="2023-01-01")
        db.upsert_paper("2301.00002", "arxiv", title="New Paper", published="2025-01-01")
        results, total = db.search_papers("paper", date_to="2024-01-01")
        assert total == 1
        assert results[0].paper_id == "2301.00001"


class TestListPapersEdgeCases:
    def test_list_papers_filter_by_source(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="ArXiv Paper")
        db.upsert_paper("2301.00002", "semantic", title="Semantic Paper")
        papers, total = db.list_papers(source="arxiv")
        assert total == 1
        assert papers[0].source == "arxiv"

    def test_list_papers_filter_by_parse_status(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="Parsed")
        db.upsert_paper("2301.00002", "arxiv", title="Unparsed")
        db.update_parse_status("2301.00001", status="success", plain_text="done")
        papers, total = db.list_papers(parse_status="success")
        assert total == 1
        assert papers[0].id == "2301.00001"

    def test_list_papers_filter_by_category(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="AI Paper", primary_category="cs.AI")
        db.upsert_paper("2301.00002", "arxiv", title="LG Paper", primary_category="cs.LG")
        papers, total = db.list_papers(category="cs.AI")
        assert total == 1
        assert papers[0].primary_category == "cs.AI"


class TestJobQueueEdgeCases:
    def test_dequeue_job_no_queue(self, db):
        assert db.dequeue_job() is None

    def test_dequeue_job_leaves_pending(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="P1")
        db.upsert_paper("2301.00002", "arxiv", title="P2")
        db.enqueue_job("2301.00001", "parse")
        db.enqueue_job("2301.00002", "parse")
        db.dequeue_job()
        assert db.queue_depth() == 1

    def test_complete_job_nonexistent(self, db):
        db.complete_job(99999, "done")
        # Should not raise
