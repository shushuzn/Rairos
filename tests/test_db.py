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


class TestPaperExists:
    def test_paper_exists_true(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="Test")
        assert db.paper_exists("2301.00001") is True

    def test_paper_exists_false(self, db):
        assert db.paper_exists("does-not-exist") is False

    def test_paper_exists_after_delete(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="Test")
        db.delete_paper("2301.00001")
        assert db.paper_exists("2301.00001") is False


class TestGetPaperTitle:
    def test_get_paper_title_found(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="Attention Is All You Need")
        assert db.get_paper_title("2301.00001") == "Attention Is All You Need"

    def test_get_paper_title_missing(self, db):
        assert db.get_paper_title("does-not-exist") == ""

    def test_get_paper_title_empty_string(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="")
        assert db.get_paper_title("2301.00001") == ""


class TestDeletePaper:
    def test_delete_paper_removes_row(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="To Delete")
        assert db.paper_exists("2301.00001") is True
        deleted = db.delete_paper("2301.00001")
        assert deleted is True
        assert db.paper_exists("2301.00001") is False

    def test_delete_paper_nonexistent(self, db):
        result = db.delete_paper("does-not-exist")
        assert result is False

    def test_delete_paper_removes_fts(self, db):
        db.upsert_paper("2301.00001", "arxiv", title="FTS Test", abstract="Abstract")
        # FTS entry exists
        results, _ = db.search_papers("FTS")
        assert any(r.paper_id == "2301.00001" for r in results)
        db.delete_paper("2301.00001")
        results, _ = db.search_papers("FTS")
        assert not any(r.paper_id == "2301.00001" for r in results)


class TestCitations:
    def test_add_citation_inserts(self, db):
        db.upsert_paper("A", "arxiv", title="Paper A")
        db.upsert_paper("B", "arxiv", title="Paper B")
        result = db.add_citation("A", "B")
        assert result is True
        cites = db.get_citations("A", "from")
        assert len(cites) == 1
        assert cites[0].source_id == "A"
        assert cites[0].target_id == "B"

    def test_add_citation_duplicate_returns_false(self, db):
        db.upsert_paper("A", "arxiv", title="Paper A")
        db.upsert_paper("B", "arxiv", title="Paper B")
        db.add_citation("A", "B")
        result = db.add_citation("A", "B")
        assert result is False

    def test_add_citations_batch(self, db):
        db.upsert_paper("A", "arxiv", title="Paper A")
        db.upsert_paper("B", "arxiv", title="Paper B")
        db.upsert_paper("C", "arxiv", title="Paper C")
        count = db.add_citations_batch("A", ["B", "C"])
        assert count == 2
        cites = db.get_citations("A", "from")
        assert len(cites) == 2

    def test_add_citations_batch_empty(self, db):
        count = db.add_citations_batch("A", [])
        assert count == 0

    def test_upsert_citations_new_and_dup(self, db):
        db.upsert_paper("A", "arxiv", title="Paper A")
        db.upsert_paper("B", "arxiv", title="Paper B")
        db.upsert_paper("C", "arxiv", title="Paper C")
        new, dup = db.upsert_citations("A", ["B", "C"])
        assert new == 2
        assert dup == 0
        new2, dup2 = db.upsert_citations("A", ["B", "C"])
        assert new2 == 0
        assert dup2 == 2

    def test_get_citations_from(self, db):
        db.upsert_paper("A", "arxiv", title="Paper A")
        db.upsert_paper("B", "arxiv", title="Paper B")
        db.add_citation("A", "B")
        cites = db.get_citations("A", "from")
        assert len(cites) == 1
        assert cites[0].target_id == "B"

    def test_get_citations_to(self, db):
        db.upsert_paper("A", "arxiv", title="Paper A")
        db.upsert_paper("B", "arxiv", title="Paper B")
        db.add_citation("A", "B")
        cites = db.get_citations("B", "to")
        assert len(cites) == 1
        assert cites[0].source_id == "A"

    def test_get_citations_both(self, db):
        db.upsert_paper("A", "arxiv", title="Paper A")
        db.upsert_paper("B", "arxiv", title="Paper B")
        db.upsert_paper("C", "arxiv", title="Paper C")
        db.add_citation("A", "B")
        db.add_citation("C", "A")
        cites = db.get_citations("A", "both")
        ids = {c.source_id if c.source_id != "A" else c.target_id for c in cites}
        assert ids == {"B", "C"}

    def test_get_citations_count(self, db):
        db.upsert_paper("A", "arxiv", title="Paper A")
        db.upsert_paper("B", "arxiv", title="Paper B")
        db.upsert_paper("C", "arxiv", title="Paper C")
        db.add_citation("A", "B")
        db.add_citation("C", "A")
        counts = db.get_citation_count("A")
        assert counts["forward"] == 1
        assert counts["backward"] == 1

    def test_get_citations_count_none(self, db):
        counts = db.get_citation_count("orphan")
        assert counts["forward"] == 0
        assert counts["backward"] == 0


class TestDedupLog:
    def test_log_dedup_and_get(self, db):
        db.upsert_paper("target", "arxiv", title="Target")
        db.upsert_paper("dup", "arxiv", title="Duplicate")
        db.log_dedup("target", "dup", "keep_newer")
        log = db.get_dedup_log()
        assert len(log) == 1
        assert log[0]["target_id"] == "target"
        assert log[0]["duplicate_id"] == "dup"
        assert log[0]["keep_policy"] == "keep_newer"


class TestClearPendingPapers:
    def test_clear_pending_papers(self, db):
        db.upsert_paper("P1", "arxiv", title="P1")
        db.upsert_paper("P2", "arxiv", title="P2")
        db.update_parse_status("P1", "pending")
        db.update_parse_status("P2", "pending")
        count = db.clear_pending_papers()
        assert count == 2
        papers, _ = db.list_papers(parse_status="pending")
        assert len(papers) == 0


class TestVacuum:
    def test_vacuum_does_not_raise(self, db):
        db.vacuum()  # Should not raise


import struct

V3 = [0.1, 0.2, 0.3]  # 3-float test vector


def make_blob(vec):
    return struct.pack(f"{len(vec)}f", *vec)


class TestEmbeddings:
    def test_set_and_get_embedding_roundtrip(self, db):
        db.upsert_paper("P1", "arxiv", title="Paper 1")
        ok = db.set_embedding("P1", V3)
        assert ok is True
        emb = db.get_embedding("P1")
        assert emb is not None
        assert len(emb) == 3
        assert emb == pytest.approx(V3)

    def test_get_embedding_missing_paper(self, db):
        assert db.get_embedding("no-such") is None

    def test_get_embedding_not_set(self, db):
        db.upsert_paper("P1", "arxiv", title="No Embed")
        assert db.get_embedding("P1") is None

    def test_set_embedding_nonexistent_paper(self, db):
        ok = db.set_embedding("ghost", V3)
        assert ok is False

    def test_get_embedding_stats(self, db):
        db.upsert_paper("P1", "arxiv", title="Has Embed")
        db.upsert_paper("P2", "arxiv", title="No Embed")
        db.upsert_paper("P3", "arxiv", title="")  # empty title, excluded
        db.set_embedding("P1", V3)
        stats = db.get_embedding_stats()
        assert stats["with_embedding"] == 1
        assert stats["total_with_text"] == 2

    def test_get_embedding_stats_empty(self, db):
        stats = db.get_embedding_stats()
        assert stats["with_embedding"] == 0
        assert stats["total_with_text"] == 0

    def test_get_papers_without_embeddings(self, db):
        db.upsert_paper("P1", "arxiv", title="Needs Embed")
        db.upsert_paper("P2", "arxiv", title="Has Embed")
        db.upsert_paper("P3", "arxiv", title="")  # empty title excluded
        db.set_embedding("P2", V3)
        papers = db.get_papers_without_embeddings(limit=10)
        assert len(papers) == 1
        assert papers[0].id == "P1"

    def test_get_papers_without_embeddings_respects_limit(self, db):
        for i in range(5):
            db.upsert_paper(f"P{i}", "arxiv", title=f"Paper {i}")
        papers = db.get_papers_without_embeddings(limit=3)
        assert len(papers) == 3

    def test_get_similarity_both_have_embedding(self, db):
        db.upsert_paper("A", "arxiv", title="Paper A")
        db.upsert_paper("B", "arxiv", title="Paper B")
        db.set_embedding("A", [1.0, 0.0, 0.0])
        db.set_embedding("B", [1.0, 0.0, 0.0])
        sim = db.get_similarity("A", "B")
        assert sim is not None
        assert 0.999 < sim <= 1.0  # ~1.0 for identical vectors

    def test_get_similarity_opposite_vectors(self, db):
        db.upsert_paper("A", "arxiv", title="Paper A")
        db.upsert_paper("B", "arxiv", title="Paper B")
        db.set_embedding("A", [1.0, 0.0, 0.0])
        db.set_embedding("B", [-1.0, 0.0, 0.0])
        sim = db.get_similarity("A", "B")
        assert sim is not None
        assert sim < -0.999  # ~-1.0 for opposite vectors

    def test_get_similarity_one_missing(self, db):
        db.upsert_paper("A", "arxiv", title="Paper A")
        db.upsert_paper("B", "arxiv", title="Paper B")
        db.set_embedding("A", V3)
        assert db.get_similarity("A", "B") is None
        assert db.get_similarity("B", "A") is None

    def test_get_similarity_nonexistent(self, db):
        db.upsert_paper("A", "arxiv", title="Paper A")
        db.set_embedding("A", V3)
        assert db.get_similarity("A", "ghost") is None
        assert db.get_similarity("ghost", "A") is None

    def test_find_similar_basic(self, db):
        db.upsert_paper("Q", "arxiv", title="Query")
        db.upsert_paper("A", "arxiv", title="Similar A")
        db.upsert_paper("B", "arxiv", title="Not Similar B")
        db.set_embedding("Q", [1.0, 0.0, 0.0])
        db.set_embedding("A", [0.99, 0.01, 0.01])
        db.set_embedding("B", [0.0, 1.0, 0.0])
        results = db.find_similar("Q", threshold=0.8, limit=5)
        ids = [r[0].id for r in results]
        assert "A" in ids
        assert "B" not in ids
        assert results[0][1] >= 0.8

    def test_find_similar_no_embedding_returns_empty(self, db):
        db.upsert_paper("Q", "arxiv", title="No Embed Query")
        assert db.find_similar("Q") == []

    def test_find_similar_threshold_excludes_all(self, db):
        db.upsert_paper("Q", "arxiv", title="Query")
        db.upsert_paper("X", "arxiv", title="X")
        db.set_embedding("Q", [1.0, 0.0, 0.0])
        db.set_embedding("X", [0.0, 1.0, 0.0])  # orthogonal, sim=0
        results = db.find_similar("Q", threshold=0.9)
        assert results == []

    def test_find_similar_respects_limit(self, db):
        db.upsert_paper("Q", "arxiv", title="Query")
        for i in range(10):
            db.upsert_paper(f"S{i}", "arxiv", title=f"Similar {i}")
            db.set_embedding(f"S{i}", [0.9, 0.0, float(i) * 0.01])
        db.set_embedding("Q", [1.0, 0.0, 0.0])
        results = db.find_similar("Q", threshold=0.0, limit=3)
        assert len(results) == 3

    def test_find_similar_sorted_by_score(self, db):
        db.upsert_paper("Q", "arxiv", title="Query")
        db.upsert_paper("S1", "arxiv", title="S1")
        db.upsert_paper("S2", "arxiv", title="S2")
        db.upsert_paper("S3", "arxiv", title="S3")
        db.set_embedding("Q", [1.0, 0.0, 0.0])
        db.set_embedding("S1", [0.5, 0.0, 0.0])   # sim ~0.5
        db.set_embedding("S2", [0.9, 0.0, 0.0])   # sim ~0.9
        db.set_embedding("S3", [0.7, 0.0, 0.0])   # sim ~0.7
        results = db.find_similar("Q", threshold=0.0)
        scores = [r[1] for r in results]
        assert scores == sorted(scores, reverse=True)


class TestGetPapers:
    def test_get_papers_default_limit(self, db):
        for i in range(25):
            db.upsert_paper(f"P{i}", "arxiv", title=f"Paper {i}")
        papers = db.get_papers()
        assert len(papers) == 25

    def test_get_papers_offset(self, db):
        for i in range(10):
            db.upsert_paper(f"P{i}", "arxiv", title=f"Paper {i}")
        papers = db.get_papers(limit=5, offset=5)
        assert len(papers) == 5

    def test_get_papers_empty(self, db):
        assert db.get_papers() == []
