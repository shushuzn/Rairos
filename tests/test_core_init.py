"""Tests for core/__init__.py — Paper dataclass and today_iso."""

from core import Paper, today_iso
from core._constants import ARXIV_API, CROSSREF_WORKS


class TestPaper:
    def test_minimal_paper(self):
        p = Paper(
            source="arxiv",
            uid="2301.00001",
            title="Test Paper",
            authors=["A. Smith"],
            abstract="Abstract text",
            published="2024-01-01",
            updated="2024-01-02",
            abs_url="https://arxiv.org/abs/2301.00001",
            pdf_url="https://arxiv.org/pdf/2301.00001.pdf",
        )
        assert p.source == "arxiv"
        assert p.uid == "2301.00001"
        assert p.title == "Test Paper"
        assert p.authors == ["A. Smith"]
        assert p.published == "2024-01-01"
        assert p.updated == "2024-01-02"
        assert p.primary_category == ""

    def test_full_paper(self):
        p = Paper(
            source="doi",
            uid="10.1234/example",
            title="Full Paper",
            authors=["A. Smith", "B. Jones"],
            abstract="Full abstract",
            published="2023-06-15",
            updated="2023-06-20",
            abs_url="https://doi.org/10.1234/example",
            pdf_url="",
            primary_category="cs.AI",
            journal="Nature",
            volume="1",
            issue="2",
            page="100-110",
            doi="10.1234/example",
            comment="v2: fixed typos",
            journal_ref="Nature 1:100-110 (2023)",
            categories="cs.AI,cs.LG",
            reference_count=42,
        )
        assert p.journal == "Nature"
        assert p.volume == "1"
        assert p.issue == "2"
        assert p.doi == "10.1234/example"
        assert p.reference_count == 42

    def test_paper_defaults(self):
        p = Paper(
            source="arxiv",
            uid="x",
            title="T",
            authors=[],
            abstract="",
            published="",
            updated="",
            abs_url="",
            pdf_url="",
        )
        assert p.primary_category == ""
        assert p.journal == ""
        assert p.doi == ""
        assert p.reference_count == 0


class TestTodayIso:
    def test_returns_iso_date_string(self):
        import datetime as dt

        result = today_iso()
        assert result == dt.date.today().isoformat()

    def test_format_is_yyyy_mm_dd(self):
        result = today_iso()
        assert len(result) == 10
        assert result[4] == "-"
        assert result[7] == "-"


class TestConstants:
    def test_arxiv_api_is_url(self):
        assert ARXIV_API.startswith("http")

    def test_crossref_works_is_url(self):
        assert "crossref" in CROSSREF_WORKS.lower()
