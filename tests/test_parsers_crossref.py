"""Tests for parsers/crossref.py — CrossRef API metadata fetcher."""

from parsers.crossref import (
    _best_effort_date_from_crossref,
    _authors_from_crossref,
    _title_from_crossref,
    _abstract_from_crossref,
    _try_find_arxiv_id_in_crossref,
    DOI_RESOLVER,
)


# ─── _best_effort_date_from_crossref ─────────────────────────────────────────


class TestBestEffortDateFromCrossref:
    def _call(self, item):
        return _best_effort_date_from_crossref(item)

    def test_uses_issued_year_month_day(self):
        item = {"issued": {"date-parts": [[2024, 3, 15]]}}
        assert self._call(item) == "2024-03-15"

    def test_uses_issued_year_month_only(self):
        item = {"issued": {"date-parts": [[2023, 6]]}}
        assert self._call(item) == "2023-06-01"

    def test_uses_issued_year_only(self):
        item = {"issued": {"date-parts": [[2022]]}}
        assert self._call(item) == "2022-01-01"

    def test_no_date_returns_empty(self):
        item = {"DOI": "10.1234/test"}
        assert self._call(item) == ""

    def test_returns_empty_when_no_date(self):
        item = {}
        assert self._call(item) == ""

    def test_empty_date_parts_dict_skips_to_empty(self):
        item = {
            "issued": {}
        }  # date-parts is {}, not a list, so key is skipped; no more keys -> returns ""
        assert self._call(item) == ""

    def test_prefers_published_over_issued(self):
        item = {
            "published": {"date-parts": [[2021, 3, 1]]},
            "issued": {"date-parts": [[2020, 1, 1]]},
        }
        assert self._call(item) == "2021-03-01"

    def test_only_created(self):
        item = {"created": {"date-parts": [[2020, 7, 4]]}}
        assert self._call(item) == "2020-07-04"

    def test_prefers_issued_over_created(self):
        item = {"issued": {"date-parts": [[2021, 3, 1]]}, "created": {"date-parts": [[2020, 1, 1]]}}
        assert self._call(item) == "2021-03-01"

    def test_none_item(self):
        assert _best_effort_date_from_crossref(None) == ""


# ─── _authors_from_crossref ──────────────────────────────────────────────────


class TestAuthorsFromCrossref:
    def _call(self, item):
        return _authors_from_crossref(item)

    def test_full_names(self):
        item = {
            "author": [{"given": "John", "family": "Doe"}, {"given": "Jane", "family": "Smith"}]
        }
        assert self._call(item) == ["John Doe", "Jane Smith"]

    def test_given_only(self):
        item = {"author": [{"given": "John"}, {"given": "Jane"}]}
        assert self._call(item) == ["John", "Jane"]

    def test_family_only(self):
        item = {"author": [{"family": "Doe"}, {"family": "Smith"}]}
        assert self._call(item) == ["Doe", "Smith"]

    def test_empty_author_list(self):
        item = {"author": []}
        assert self._call(item) == []

    def test_missing_author(self):
        item = {}
        assert self._call(item) == []

    def test_whitespace_trimmed(self):
        item = {"author": [{"given": "  John  ", "family": "  Doe  "}]}
        assert self._call(item) == ["John Doe"]

    def test_mixed_null_and_present(self):
        item = {"author": [{"given": "John", "family": "Doe"}, {"family": "Smith"}, {}]}
        assert self._call(item) == ["John Doe", "Smith"]

    def test_none_author(self):
        item = {"author": None}
        assert self._call(item) == []


# ─── _title_from_crossref ────────────────────────────────────────────────────


class TestTitleFromCrossref:
    def _call(self, item):
        return _title_from_crossref(item)

    def test_list_of_titles(self):
        item = {"title": ["Main Paper Title", "Subtitle"]}
        assert self._call(item) == "Main Paper Title"

    def test_string_title(self):
        item = {"title": "Direct String Title"}
        assert self._call(item) == "Direct String Title"

    def test_uses_first_of_multiple(self):
        item = {"title": ["First Title", "Second Title", "Third Title"]}
        assert self._call(item) == "First Title"

    def test_missing_title(self):
        item = {}
        assert self._call(item) == ""

    def test_empty_title(self):
        item = {"title": ""}
        assert self._call(item) == ""

    def test_none_title(self):
        item = {"title": None}
        assert self._call(item) == ""


# ─── _abstract_from_crossref ────────────────────────────────────────────────


class TestAbstractFromCrossref:
    def _call(self, item):
        return _abstract_from_crossref(item)

    def test_plain_text(self):
        item = {"abstract": "This is a plain abstract."}
        assert self._call(item) == "This is a plain abstract."

    def test_strips_html_tags(self):
        item = {"abstract": "<p>This has <strong>HTML</strong> tags.</p>"}
        assert self._call(item) == "This has HTML tags."

    def test_strips_multiple_html_tags(self):
        item = {"abstract": "<jats:p>Multiple <i>italic</i> <b>bold</b> tags.</jats:p>"}
        assert self._call(item) == "Multiple italic bold tags."

    def test_collapse_whitespace(self):
        item = {"abstract": "Word1    \n\n\n  Word2"}
        assert self._call(item) == "Word1 Word2"

    def test_missing_abstract(self):
        item = {}
        assert self._call(item) == ""

    def test_empty_abstract(self):
        item = {"abstract": ""}
        assert self._call(item) == ""


# ─── _try_find_arxiv_id_in_crossref ────────────────────────────────────────


class TestTryFindArxivIdInCrossref:
    def _call(self, item, doi):
        return _try_find_arxiv_id_in_crossref(item, doi)

    def test_finds_arxiv_doi(self):
        item = {}
        doi = "10.48550/arXiv.2301.12345"
        assert self._call(item, doi) == "2301.12345"

    def test_finds_in_doi_10_48550(self):
        item = {}
        doi = "10.48550/arXiv.2301.12345"
        assert self._call(item, doi) == "2301.12345"

    def test_no_match_regular_doi(self):
        item = {}
        doi = "10.1234/journal.article"
        assert self._call(item, doi) is None

    def test_case_insensitive_doi(self):
        item = {}
        doi = "10.48550/ARXIV.2301.12345"
        assert self._call(item, doi) == "2301.12345"

    def test_none_item_and_doi(self):
        assert self._call({}, None) is None

    def test_empty_item_and_doi(self):
        assert self._call({}, "") is None

    def test_doi_without_arxiv_prefix(self):
        item = {}
        doi = "10.1234/notarxiv"
        assert self._call(item, doi) is None


# ─── DOI_RESOLVER ────────────────────────────────────────────────────────────


class TestDOIRESOLVER:
    def test_doi_resolver_default(self):
        assert DOI_RESOLVER == "https://doi.org/"
