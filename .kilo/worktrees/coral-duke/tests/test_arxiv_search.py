"""Tier 2 unit tests — parsers/arxiv_search.py, no I/O, no network."""
from unittest.mock import Mock, patch

import pytest

from parsers.arxiv_search import _entry_to_paper, search_arxiv


# =============================================================================
# _entry_to_paper — pure conversion
# =============================================================================


class TestEntryToPaper:
    def test_basic_fields(self):
        e = Mock()
        e.id = "http://arxiv.org/abs/2301.12345v1"
        e.title = "Test Paper Title\n"
        e.summary = "This is the abstract.\nWith line breaks."
        e.authors = []
        for n in ["John Doe", "Jane Smith"]:
            m = Mock()
            m.name = n
            e.authors.append(m)
        e.published = "2023-01-15T00:00:00Z"
        e.updated = "2023-02-20T00:00:00Z"
        e.link = "https://arxiv.org/abs/2301.12345v1"
        e.links = []
        e.arxiv_primary_category = {"term": "cs.AI"}
        e.tags = [{"term": "cs.AI"}, {"term": "cs.LG"}]
        e.arxiv_comment = "15 pages, 8 figures"
        e.journal_ref = "Nature 2023"
        e.arxiv_doi = "10.1234/test"

        p = _entry_to_paper(e)

        assert p.source == "arxiv"
        assert p.uid == "2301.12345v1"
        assert p.title == "Test Paper Title"
        assert p.abstract == "This is the abstract. With line breaks."
        assert p.authors == ["John Doe", "Jane Smith"]
        assert p.published == "2023-01-15"
        assert p.updated == "2023-02-20"
        assert p.primary_category == "cs.AI"
        assert p.categories == "cs.AI, cs.LG"
        assert p.comment == "15 pages, 8 figures"
        assert p.journal_ref == "Nature 2023"
        assert p.doi == "10.1234/test"

    def test_falls_back_to_pdf_url_from_id(self):
        e = Mock()
        e.id = "http://arxiv.org/abs/2301.12345"
        e.title = "T"
        e.summary = "A"
        e.authors = []
        e.published = ""
        e.updated = ""
        e.link = ""
        e.links = []  # no PDF link
        e.arxiv_primary_category = {}
        e.tags = []
        e.arxiv_comment = ""
        e.journal_ref = ""
        e.arxiv_doi = ""

        p = _entry_to_paper(e)

        assert p.pdf_url == "https://arxiv.org/pdf/2301.12345.pdf"
        assert p.abs_url == "https://arxiv.org/abs/2301.12345"

    def test_extracts_pdf_link_from_links(self):
        e = Mock()
        e.id = "http://arxiv.org/abs/2301.99999"
        e.title = "T"
        e.summary = "A"
        e.authors = []
        e.published = ""
        e.updated = ""
        e.link = ""
        pdf_link = Mock()
        pdf_link.type = "application/pdf"
        pdf_link.href = "https://example.com/custom/pdf/path.pdf"
        e.links = [pdf_link]
        e.arxiv_primary_category = {}
        e.tags = []
        e.arxiv_comment = ""
        e.journal_ref = ""
        e.arxiv_doi = ""

        p = _entry_to_paper(e)

        assert p.pdf_url == "https://example.com/custom/pdf/path.pdf"

    def test_handles_missing_optional_fields(self):
        e = Mock()
        e.id = "http://arxiv.org/abs/2301.00000"
        e.title = ""  # empty strings — handled gracefully
        e.summary = ""
        e.authors = []  # empty list
        e.published = ""
        e.updated = ""
        e.link = ""
        e.links = []
        e.arxiv_primary_category = {}
        e.tags = []
        e.arxiv_comment = ""
        e.journal_ref = ""
        e.arxiv_doi = ""

        p = _entry_to_paper(e)

        assert p.title == ""
        assert p.abstract == ""
        assert p.authors == []
        assert p.categories == ""

    def test_truncates_published_to_date_only(self):
        e = Mock()
        e.id = "http://arxiv.org/abs/2301.00001"
        e.title = "T"
        e.summary = "A"
        e.authors = []
        e.published = "2023-06-01T12:34:56Z"
        e.updated = "2023-07-15T09:00:00Z"
        e.link = ""
        e.links = []
        e.arxiv_primary_category = {}
        e.tags = []
        e.arxiv_comment = ""
        e.journal_ref = ""
        e.arxiv_doi = ""

        p = _entry_to_paper(e)

        assert p.published == "2023-06-01"
        assert p.updated == "2023-07-15"

    def test_handles_missing_author_names(self):
        e = Mock()
        e.id = "http://arxiv.org/abs/2301.00002"
        e.title = "T"
        e.summary = "A"
        # authors with empty name
        e.authors = []
        for n in ["", "  ", "Valid Name"]:
            m = Mock()
            m.name = n
            e.authors.append(m)
        e.published = ""
        e.updated = ""
        e.link = ""
        e.links = []
        e.arxiv_primary_category = {}
        e.tags = []
        e.arxiv_comment = ""
        e.journal_ref = ""
        e.arxiv_doi = ""

        p = _entry_to_paper(e)

        assert p.authors == ["Valid Name"]


# =============================================================================
# search_arxiv — mocked network
# =============================================================================


class TestSearchArxiv:
    @pytest.fixture
    def mock_feed(self):
        """Minimal feed entry."""
        entry = Mock()
        entry.id = "http://arxiv.org/abs/2301.12345v2"
        entry.title = "Attention Is All You Need\n"
        entry.summary = "We propose a new simple network architecture.\n"
        entry.authors = []
        for n in ["Ashish Vaswani", "Noam Shazeer"]:
            m = Mock()
            m.name = n
            entry.authors.append(m)
        entry.published = "2017-06-12T00:00:00Z"
        entry.updated = "2023-01-15T00:00:00Z"
        entry.link = "https://arxiv.org/abs/1706.03762v2"
        pdf_link = Mock()
        pdf_link.type = "application/pdf"
        pdf_link.href = "https://arxiv.org/pdf/1706.03762v2"
        entry.links = [pdf_link]
        entry.arxiv_primary_category = {"term": "cs.CL"}
        entry.tags = [{"term": "cs.CL"}, {"term": "cs.LG"}, {"term": "cs.AI"}]
        entry.arxiv_comment = "15 pages"
        entry.journal_ref = "NeurIPS 2017"
        entry.arxiv_doi = "10.48550/arXiv.1706.03762"
        return entry

    @patch("parsers.arxiv_search._http.get")
    @patch("parsers.arxiv_search.feedparser.parse")
    def test_returns_papers_sorted_by_relevance(self, mock_fp_parse, mock_get, mock_feed):
        mock_response = Mock()
        mock_response.raise_for_status = Mock()
        mock_get.return_value = mock_response

        mock_feed_obj = Mock()
        mock_feed_obj.entries = [mock_feed]
        mock_fp_parse.return_value = mock_feed_obj

        result = search_arxiv("transformer attention", max_results=5)

        assert len(result) == 1
        assert result[0].uid == "2301.12345v2"
        assert result[0].title == "Attention Is All You Need"
        assert result[0].authors == ["Ashish Vaswani", "Noam Shazeer"]

        # Verify URL construction
        mock_get.assert_called_once()
        call_url = mock_get.call_args[0][0]
        assert "search_query=all:transformer+attention" in call_url
        assert "max_results=5" in call_url
        assert "sortBy=relevance" in call_url

    @patch("parsers.arxiv_search._http.get")
    @patch("parsers.arxiv_search.feedparser.parse")
    def test_returns_empty_list_when_no_entries(self, mock_fp_parse, mock_get):
        mock_response = Mock()
        mock_response.raise_for_status = Mock()
        mock_get.return_value = mock_response

        mock_feed_obj = Mock()
        mock_feed_obj.entries = []
        mock_fp_parse.return_value = mock_feed_obj

        result = search_arxiv("nonexistent query xyz 123")

        assert result == []

    @patch("parsers.arxiv_search._http.get")
    def test_raises_runtime_error_on_http_failure(self, mock_get):
        import requests

        mock_response = Mock()
        mock_response.raise_for_status = Mock(
            side_effect=requests.HTTPError("404 Not Found")
        )
        mock_get.return_value = mock_response

        with pytest.raises(RuntimeError, match="arXiv search failed"):
            search_arxiv("test query")

    @patch("parsers.arxiv_search._http.get")
    def test_raises_runtime_error_on_timeout(self, mock_get):
        mock_get.side_effect = TimeoutError("Connection timed out")

        with pytest.raises(RuntimeError, match="arXiv search failed"):
            search_arxiv("test query")

    @patch("parsers.arxiv_search._http.get")
    @patch("parsers.arxiv_search.feedparser.parse")
    def test_passes_timeout_to_requests(self, mock_fp_parse, mock_get):
        mock_response = Mock()
        mock_response.raise_for_status = Mock()
        mock_get.return_value = mock_response
        mock_fp_parse.return_value = Mock(entries=[])

        search_arxiv("test", timeout=60)

        mock_get.assert_called_with(
            "https://export.arxiv.org/api/query?"
            "search_query=all:test&start=0&max_results=5&"
            "sortBy=relevance&sortOrder=descending",
            timeout=60,
        )
