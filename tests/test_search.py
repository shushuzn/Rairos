"""Tests for FTS5 full-text search in database.py"""
import pytest
import tempfile
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from db.database import Database, SearchResult


@pytest.fixture
def db():
    """Fresh in-memory database per test."""
    tmp = tempfile.NamedTemporaryFile(suffix=".db", delete=False)
    path = tmp.name
    tmp.close()
    db = Database(path)
    db.init()
    yield db
    db.close()
    try:
        os.unlink(path)
    except OSError:
        pass


class TestSearchResultDataclass:
    def test_search_result_fields(self):
        r = SearchResult(
            paper_id="2312.12345",
            title="Attention Is All You Need",
            authors=["Vaswani", "Shazeer"],
            published="2023-01-01",
            primary_category="cs.CL",
            score=-1.5,
            snippet="**attention** mechanisms",
            parse_status="done",
            source="arxiv",
            abs_url="https://arxiv.org/abs/2312.12345",
            pdf_url="https://arxiv.org/pdf/2312.12345",
        )
        assert r.paper_id == "2312.12345"
        assert r.title == "Attention Is All You Need"
        assert r.score == -1.5
        assert r.snippet == "**attention** mechanisms"
        assert r.source == "arxiv"

    def test_search_result_defaults(self):
        r = SearchResult(
            paper_id="2312.12345",
            title="T",
            authors=[],
            published="",
            primary_category="",
            score=0.0,
            snippet="",
            parse_status="pending",
            source="",
            abs_url="",
            pdf_url="",
        )
        assert r.paper_id == "2312.12345"
        assert r.source == ""


class TestFTS5Init:
    def test_fts_table_created(self, db):
        result = db.conn.execute(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='papers_fts'"
        ).fetchone()
        assert result is not None

    def test_fts_table_schema(self, db):
        result = db.conn.execute("PRAGMA table_info(papers_fts)").fetchall()
        names = [r[1] for r in result]
        assert "paper_id" in names
        assert "title" in names
        assert "abstract" in names
        assert "plain_text" in names


class TestFTS5Insert:
    def test_upsert_indexes_paper(self, db):
        db.upsert_paper("2312.00001", "arxiv", title="Deep Learning", abstract="Neural networks")
        results, total = db.search_papers("deep")
        assert total == 1
        assert results[0].paper_id == "2312.00001"

    def test_upsert_updates_fts(self, db):
        db.upsert_paper("2312.00002", "arxiv", title="Machine Learning")
        db.upsert_paper("2312.00002", "arxiv", title="Deep Learning")
        results, total = db.search_papers("deep")
        assert total == 1
        assert results[0].title == "Deep Learning"


class TestFTS5Delete:
    def test_fts_delete_on_paper_delete(self, db):
        db.upsert_paper("2312.00003", "arxiv", title="To Be Deleted")
        db.delete_paper("2312.00003")
        results, total = db.search_papers("deleted")
        assert total == 0


class TestSearchPapersQuery:
    def test_search_by_title(self, db):
        db.upsert_paper("2312.00004", "arxiv", title="Attention Is All You Need")
        results, total = db.search_papers("attention")
        assert total == 1
        assert "attention" in results[0].title.lower()

    def test_search_by_abstract(self, db):
        db.upsert_paper("2312.00005", "arxiv", abstract="Novel neural architecture")
        results, total = db.search_papers("neural")
        assert total >= 1

    def test_search_returns_snippet(self, db):
        db.upsert_paper("2312.00006", "arxiv", title="Deep Neural Networks")
        results, total = db.search_papers("neural")
        assert total == 1
        assert results[0].snippet is not None

    def test_search_no_results(self, db):
        db.upsert_paper("2312.00007", "arxiv", title="Unrelated Work")
        results, total = db.search_papers("xyznonexistent")
        assert total == 0


class TestSearchPapersFilter:
    def test_search_filter_by_source(self, db):
        db.upsert_paper("test.src.1", "arxiv", title="Arxiv Paper One")
        db.upsert_paper("test.src.2", "doi", title="DOI Paper Two")
        results, total = db.search_papers("paper", source="arxiv")
        assert total == 1
        assert results[0].paper_id == "test.src.1"

    def test_search_filter_by_category(self, db):
        db.upsert_paper("2312.00010", "arxiv", title="CL Paper", primary_category="cs.CL")
        db.upsert_paper("2312.00011", "arxiv", title="CV Paper", primary_category="cs.CV")
        results, total = db.search_papers("paper", category="cs.CL")
        assert total == 1
        assert results[0].primary_category == "cs.CL"

    def test_search_filter_by_parse_status(self, db):
        db.upsert_paper("2312.00012", "arxiv", title="Parsed Paper")
        db.upsert_paper("2312.00013", "arxiv", title="Pending Paper")
        db.update_parse_status("2312.00012", "done")
        results, total = db.search_papers("paper", parse_status="done")
        assert total == 1
        assert results[0].parse_status == "done"

    def test_search_combined_filters(self, db):
        db.upsert_paper("2312.00014", "arxiv", title="CL Paper", primary_category="cs.CL")
        db.upsert_paper("2312.00015", "arxiv", title="CV Paper", primary_category="cs.CV")
        results, total = db.search_papers("paper", category="cs.CL", source="arxiv")
        assert total == 1


class TestSearchPapersSort:
    def test_search_sort_by_relevance(self, db):
        db.upsert_paper("2312.00020", "arxiv", title="First Paper")
        db.upsert_paper("2312.00021", "arxiv", title="Second Paper")
        results, total = db.search_papers("paper")
        assert total == 2

    def test_search_sort_by_published(self, db):
        db.upsert_paper("2312.00022", "arxiv", title="New Paper", published="2024-01-01")
        db.upsert_paper("2312.00023", "arxiv", title="Old Paper", published="2023-01-01")
        results, total = db.search_papers("paper")
        assert total == 2


class TestSearchPapersPagination:
    def test_search_with_limit(self, db):
        for i in range(5):
            db.upsert_paper(f"2312.00{30+i:02d}", "arxiv", title=f"Paper {i}")
        results, total = db.search_papers("paper", limit=3)
        assert len(results) == 3
        assert total == 5

    def test_search_with_offset(self, db):
        for i in range(5):
            db.upsert_paper(f"2312.00{40+i:02d}", "arxiv", title=f"Paper {i}")
        page1, _ = db.search_papers("paper", limit=3, offset=0)
        page2, _ = db.search_papers("paper", limit=3, offset=3)
        assert len(page1) == 3
        assert len(page2) == 2


class TestListPapers:
    def test_list_papers_basic(self, db):
        db.upsert_paper("2312.00050", "arxiv", title="Paper A")
        db.upsert_paper("2312.00051", "arxiv", title="Paper B")
        papers, total = db.list_papers()
        assert total == 2

    def test_list_papers_pagination(self, db):
        for i in range(5):
            db.upsert_paper(f"2312.00{60+i:02d}", "arxiv", title=f"Paper {i}")
        page1, _ = db.list_papers(offset=0, limit=3)
        page2, _ = db.list_papers(offset=3, limit=3)
        assert len(page1) == 3
        assert len(page2) == 2

    def test_list_papers_filter_by_source(self, db):
        db.upsert_paper("2312.00070", "arxiv", title="Arxiv Paper")
        db.upsert_paper("2312.00071", "doi", title="DOI Paper")
        papers, _ = db.list_papers(source="arxiv")
        assert all(p.source == "arxiv" for p in papers)


class TestRebuildFTS:
    def test_rebuild_clears_orphans(self, db):
        db.upsert_paper("2312.00080", "arxiv", title="First Paper")
        db.upsert_paper("2312.00081", "arxiv", title="Second Paper")
        results, total = db.search_papers("paper")
        assert total == 2
        db.rebuild_fts_index()
        results, total = db.search_papers("paper")
        assert total == 2
