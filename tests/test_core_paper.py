"""Tests for the Paper dataclass in core/__init__.py."""

import pytest
from dataclasses import fields, is_dataclass, asdict, replace
from core import Paper


def make_minimal_paper(**overrides):
    base = dict(
        source="arxiv",
        uid="2301.00001",
        title="A Simple Title",
        authors=["Alice Smith", "Bob Jones"],
        abstract="This is a test abstract.",
        published="2024-01-15",
        updated="2024-01-20",
        abs_url="https://arxiv.org/abs/2301.00001",
        pdf_url="https://arxiv.org/pdf/2301.00001.pdf",
    )
    base.update(overrides)
    return Paper(**base)


class TestPaperIsDataclass:
    def test_paper_is_dataclass(self):
        assert is_dataclass(Paper) is True

    def test_paper_field_count(self):
        assert len(fields(Paper)) == 19

    def test_field_names(self):
        expected = [
            "source",
            "uid",
            "title",
            "authors",
            "abstract",
            "published",
            "updated",
            "abs_url",
            "pdf_url",
            "primary_category",
            "journal",
            "volume",
            "issue",
            "page",
            "doi",
            "comment",
            "journal_ref",
            "categories",
            "reference_count",
        ]
        actual = [f.name for f in fields(Paper)]
        assert actual == expected


class TestRequiredFields:
    @pytest.mark.parametrize(
        "field",
        [
            "source",
            "uid",
            "title",
            "authors",
            "abstract",
            "published",
            "updated",
            "abs_url",
            "pdf_url",
        ],
    )
    def test_required_field_presence(self, field):
        p = make_minimal_paper()
        assert hasattr(p, field)
        assert getattr(p, field) is not None

    def test_authors_is_list(self):
        assert isinstance(make_minimal_paper().authors, list)

    def test_source_values(self):
        assert make_minimal_paper(source="arxiv").source == "arxiv"
        assert make_minimal_paper(source="doi").source == "doi"

    def test_uid_not_empty(self):
        assert len(make_minimal_paper(uid="10.1234/test").uid) > 0

    def test_date_format_yyyy_mm_dd(self):
        p = make_minimal_paper(published="2024-06-30", updated="2024-07-01")
        assert len(p.published) == 10
        assert p.published[4] == "-"
        assert p.published[7] == "-"

    def test_urls_are_strings(self):
        p = make_minimal_paper()
        assert isinstance(p.abs_url, str)
        assert isinstance(p.pdf_url, str)


class TestOptionalFieldDefaults:
    def test_primary_category_default(self):
        assert make_minimal_paper().primary_category == ""

    def test_journal_default(self):
        assert make_minimal_paper().journal == ""

    def test_volume_default(self):
        assert make_minimal_paper().volume == ""

    def test_issue_default(self):
        assert make_minimal_paper().issue == ""

    def test_page_default(self):
        assert make_minimal_paper().page == ""

    def test_doi_default(self):
        assert make_minimal_paper().doi == ""

    def test_comment_default(self):
        assert make_minimal_paper().comment == ""

    def test_journal_ref_default(self):
        assert make_minimal_paper().journal_ref == ""

    def test_categories_default(self):
        assert make_minimal_paper().categories == ""

    def test_reference_count_default(self):
        assert make_minimal_paper().reference_count == 0


class TestOptionalFieldExplicitValues:
    def test_primary_category(self):
        assert make_minimal_paper(primary_category="cs.AI").primary_category == "cs.AI"

    def test_journal(self):
        assert make_minimal_paper(journal="Nature").journal == "Nature"

    def test_volume_issue_page(self):
        p = make_minimal_paper(volume="42", issue="3", page="100-120")
        assert p.volume == "42"
        assert p.issue == "3"
        assert p.page == "100-120"

    def test_doi(self):
        assert make_minimal_paper(doi="10.1234/example").doi == "10.1234/example"

    def test_comment(self):
        c = "32 pages, 5 figures, v2"
        assert make_minimal_paper(comment=c).comment == c

    def test_journal_ref(self):
        r = "Science 370:302-307 (2020)"
        assert make_minimal_paper(journal_ref=r).journal_ref == r

    def test_categories_comma_separated(self):
        cats = "cs.AI,cs.LG,stat.ML"
        assert make_minimal_paper(categories=cats).categories == cats

    def test_reference_count_positive(self):
        assert make_minimal_paper(reference_count=99).reference_count == 99

    def test_reference_count_zero(self):
        assert make_minimal_paper(reference_count=0).reference_count == 0

    def test_reference_count_large(self):
        assert make_minimal_paper(reference_count=1_000_000).reference_count == 1_000_000


class TestAuthorsField:
    def test_single_author(self):
        assert len(make_minimal_paper(authors=["Alice"]).authors) == 1

    def test_multiple_authors(self):
        authors = ["Alice Smith", "Bob Jones", "Carol White"]
        assert make_minimal_paper(authors=authors).authors == authors

    def test_empty_authors_list(self):
        assert make_minimal_paper(authors=[]).authors == []


class TestSourceField:
    def test_source_arxiv(self):
        assert make_minimal_paper(source="arxiv").source == "arxiv"

    def test_source_doi(self):
        assert make_minimal_paper(source="doi").source == "doi"

    def test_source_arbitrary(self):
        assert make_minimal_paper(source="crossref").source == "crossref"


class TestDataclassBehaviours:
    def test_repr_contains_title(self):
        assert "My Paper Title" in repr(make_minimal_paper(title="My Paper Title"))

    def test_repr_contains_uid(self):
        assert "2301.00001" in repr(make_minimal_paper(uid="2301.00001"))

    def test_equality_same_fields(self):
        p1 = make_minimal_paper()
        p2 = make_minimal_paper()
        assert p1 == p2

    def test_equality_different_uid(self):
        p1 = make_minimal_paper(uid="2301.00001")
        p2 = make_minimal_paper(uid="2301.00002")
        assert p1 != p2

    def test_equality_different_authors(self):
        p1 = make_minimal_paper(authors=["Alice"])
        p2 = make_minimal_paper(authors=["Bob"])
        assert p1 != p2

    def test_equality_different_source(self):
        p1 = make_minimal_paper(source="arxiv")
        p2 = make_minimal_paper(source="doi")
        assert p1 != p2

    def test_replace_preserves_unchanged(self):
        p = make_minimal_paper(primary_category="cs.AI")
        p2 = replace(p, primary_category="cs.LG")
        assert p.primary_category == "cs.AI"
        assert p2.primary_category == "cs.LG"


class TestFieldTypes:
    def test_source_is_str(self):
        assert type(make_minimal_paper().source) is str

    def test_uid_is_str(self):
        assert type(make_minimal_paper().uid) is str

    def test_title_is_str(self):
        assert type(make_minimal_paper().title) is str

    def test_abstract_is_str(self):
        assert type(make_minimal_paper().abstract) is str

    def test_published_is_str(self):
        assert type(make_minimal_paper().published) is str

    def test_updated_is_str(self):
        assert type(make_minimal_paper().updated) is str

    def test_abs_url_is_str(self):
        assert type(make_minimal_paper().abs_url) is str

    def test_pdf_url_is_str(self):
        assert type(make_minimal_paper().pdf_url) is str

    def test_primary_category_is_str(self):
        assert type(make_minimal_paper().primary_category) is str

    def test_journal_is_str(self):
        assert type(make_minimal_paper().journal) is str

    def test_volume_is_str(self):
        assert type(make_minimal_paper().volume) is str

    def test_issue_is_str(self):
        assert type(make_minimal_paper().issue) is str

    def test_page_is_str(self):
        assert type(make_minimal_paper().page) is str

    def test_doi_is_str(self):
        assert type(make_minimal_paper().doi) is str

    def test_comment_is_str(self):
        assert type(make_minimal_paper().comment) is str

    def test_journal_ref_is_str(self):
        assert type(make_minimal_paper().journal_ref) is str

    def test_categories_is_str(self):
        assert type(make_minimal_paper().categories) is str

    def test_reference_count_is_int(self):
        assert type(make_minimal_paper().reference_count) is int


class TestOptionalFieldsAcceptEmptyStrings:
    @pytest.mark.parametrize(
        "field",
        [
            "primary_category",
            "journal",
            "volume",
            "issue",
            "page",
            "doi",
            "comment",
            "journal_ref",
            "categories",
        ],
    )
    def test_optional_field_empty_string(self, field):
        p = make_minimal_paper(**{field: ""})
        assert getattr(p, field) == ""

    def test_reference_count_zero(self):
        assert make_minimal_paper(reference_count=0).reference_count == 0


class TestDictRoundTrip:
    def test_asdict_complete(self):
        p = make_minimal_paper(
            source="doi",
            uid="10.1234/test",
            title="Dict Test",
            authors=["X", "Y"],
            abstract="A",
            published="2020-01-01",
            updated="2020-01-02",
            abs_url="https://doi.org/10.1234/test",
            pdf_url="",
            primary_category="eess",
            journal="J",
            volume="1",
            issue="1",
            page="1",
            doi="10.1234/test",
            comment="note",
            journal_ref="J 1:1 (2020)",
            categories="eess",
            reference_count=5,
        )
        d = asdict(p)
        assert d["source"] == "doi"
        assert d["uid"] == "10.1234/test"
        assert d["reference_count"] == 5
        assert isinstance(d["authors"], list)

    def test_replace_changes_one_field(self):
        p = make_minimal_paper(source="arxiv")
        p2 = replace(p, source="doi")
        assert p.source == "arxiv"
        assert p2.source == "doi"
