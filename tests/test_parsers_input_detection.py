"""Tests for parsers/input_detection.py — DOI and arXiv ID normalization."""

from parsers.input_detection import is_probably_doi, normalize_doi, normalize_arxiv_id


# =============================================================================
# is_probably_doi
# =============================================================================


class TestIsProbablyDoi:
    def test_bare_doi(self):
        assert is_probably_doi("10.1234/test.article")
        assert is_probably_doi("10.1002/chim.202400123")

    def test_doi_with_https_doi_org_url(self):
        assert is_probably_doi("https://doi.org/10.1234/test.article")
        assert is_probably_doi("https://dx.doi.org/10.1234/test.article")

    def test_doi_with_http_doi_org_url(self):
        assert is_probably_doi("http://doi.org/10.1234/test.article")
        assert is_probably_doi("http://dx.doi.org/10.1234/test.article")

    def test_doi_case_insensitive(self):
        assert is_probably_doi("HTTPS://DOI.ORG/10.1234/TEST")
        assert is_probably_doi("10.1234/ABCDEF")

    def test_non_doi_strings(self):
        assert not is_probably_doi("10.1234")  # too short prefix
        assert not is_probably_doi("10.123/abc")  # only 3 digits after 10
        assert not is_probably_doi("11.1234/test")  # wrong prefix
        assert not is_probably_doi("arxiv:2301.12345")
        assert not is_probably_doi("https://arxiv.org/abs/2301.12345")
        assert not is_probably_doi("just a random string")
        assert not is_probably_doi("")
        assert not is_probably_doi("10.123456789/")  # missing suffix

    def test_whitespace_stripped(self):
        assert is_probably_doi("  10.1234/test  ")
        assert not is_probably_doi("  arxiv:2301.12345  ")


# =============================================================================
# normalize_doi
# =============================================================================


class TestNormalizeDoi:
    def test_bare_doi(self):
        assert normalize_doi("10.1234/test.article") == "10.1234/test.article"

    def test_https_doi_org_url(self):
        assert normalize_doi("https://doi.org/10.1234/test.article") == "10.1234/test.article"

    def test_https_dx_doi_org_url(self):
        assert normalize_doi("https://dx.doi.org/10.1234/test.article") == "10.1234/test.article"

    def test_http_doi_org_url(self):
        assert normalize_doi("http://doi.org/10.1234/test.article") == "10.1234/test.article"

    def test_trailing_dot_stripped(self):
        assert normalize_doi("10.1234/test.article.") == "10.1234/test.article"

    def test_whitespace_stripped(self):
        assert normalize_doi("  10.1234/test  ") == "10.1234/test"
        assert normalize_doi("https://doi.org/10.1234/test  ") == "10.1234/test"

    def test_none_returns_none(self):
        assert normalize_doi(None) is None

    def test_empty_string_returns_none(self):
        assert normalize_doi("") is None

    def test_whitespace_only_string_becomes_empty_string(self):
        # whitespace-only input is stripped to "" by .strip(), not converted to None
        assert normalize_doi("   ") == ""

    def test_arxiv_doi_not_normalized(self):
        # 10.48550/arXiv IDs are not modified by normalize_doi
        result = normalize_doi("10.48550/arXiv.2301.12345")
        assert result == "10.48550/arXiv.2301.12345"


# =============================================================================
# normalize_arxiv_id
# =============================================================================


class TestNormalizeArxivId:
    # ---- New-style numeric ID (YYMM.NNNNN) ---------------------------------
    def test_bare_new_style_id(self):
        assert normalize_arxiv_id("2301.12345") == "2301.12345"

    def test_bare_new_style_id_short(self):
        assert normalize_arxiv_id("2301.1234") == "2301.1234"

    def test_new_style_id_with_version(self):
        assert normalize_arxiv_id("2301.12345v2") == "2301.12345v2"

    def test_new_style_id_with_version_single_digit(self):
        assert normalize_arxiv_id("2301.1234v1") == "2301.1234v1"

    # ---- Old-style category/NUMBER (7 digits) -----------------------------
    def test_bare_old_style_id(self):
        # Old-style IDs have exactly 7 digits after the category
        # Note: the regex [a-zA-Z\-]+ does not include dots, so dotted categories fail
        assert normalize_arxiv_id("cs/0704123") == "cs/0704123"

    def test_old_style_id_with_version(self):
        assert normalize_arxiv_id("cs/0604123v3") == "cs/0604123v3"

    def test_old_style_id_uppercase_category(self):
        # Uppercase is supported; regex is case-sensitive for category chars
        assert normalize_arxiv_id("STAT/0704123") == "STAT/0704123"

    def test_old_style_id_hyphen_category(self):
        assert normalize_arxiv_id("q-bio/0408001") == "q-bio/0408001"

    # ---- URL formats (new-style) ------------------------------------------
    def test_abs_url_new_style(self):
        assert normalize_arxiv_id("https://arxiv.org/abs/2301.12345") == "2301.12345"

    def test_abs_url_new_style_with_version(self):
        assert normalize_arxiv_id("https://arxiv.org/abs/2301.12345v2") == "2301.12345v2"

    def test_pdf_url_new_style(self):
        assert normalize_arxiv_id("https://arxiv.org/pdf/2301.12345") == "2301.12345"

    def test_pdf_url_new_style_with_version(self):
        assert normalize_arxiv_id("https://arxiv.org/pdf/2301.12345v2") == "2301.12345v2"

    def test_arxiv_url_case_insensitive(self):
        assert normalize_arxiv_id("https://ARXIV.ORG/abs/2301.12345") == "2301.12345"

    # ---- arXiv DOI format (10.48550/arXiv.NNNNNNNNN) ----------------------
    def test_arxiv_doi_format(self):
        assert normalize_arxiv_id("10.48550/arXiv.2301.12345") == "2301.12345"

    def test_arxiv_doi_format_with_version(self):
        assert normalize_arxiv_id("10.48550/arXiv.2301.12345v2") == "2301.12345v2"

    def test_arxiv_doi_format_https(self):
        assert normalize_arxiv_id("https://doi.org/10.48550/arXiv.2301.12345") == "2301.12345"

    def test_arxiv_doi_format_dx_prefix(self):
        assert normalize_arxiv_id("https://dx.doi.org/10.48550/arXiv.2301.12345") == "2301.12345"

    # ---- Edge cases -------------------------------------------------------
    def test_none_returns_none(self):
        assert normalize_arxiv_id(None) is None

    def test_empty_string_returns_none(self):
        assert normalize_arxiv_id("") is None

    def test_whitespace_stripped(self):
        assert normalize_arxiv_id("  2301.12345  ") == "2301.12345"
        assert normalize_arxiv_id("  https://arxiv.org/abs/2301.12345  ") == "2301.12345"

    def test_regular_doi_returns_none(self):
        assert normalize_arxiv_id("10.1234/test.article") is None

    def test_invalid_formats_return_none(self):
        assert normalize_arxiv_id("just some text") is None
        assert normalize_arxiv_id("10.1234") is None
        assert normalize_arxiv_id("not-a-date.NNNNN") is None
        assert normalize_arxiv_id("2301.123") is None  # too few digits after decimal
        # Old-style ID with wrong digit count (should be exactly 7 digits)
        assert normalize_arxiv_id("cs.AI/0704.1234") is None  # dots not allowed in old-style
        assert normalize_arxiv_id("cs.AI/07041234") is None  # 8 digits
        assert normalize_arxiv_id("cs.AI/07041") is None  # 5 digits

    def test_category_only_no_number_returns_none(self):
        assert normalize_arxiv_id("cs.AI/") is None
        assert normalize_arxiv_id("cs.AI") is None

    def test_old_style_url_formats_not_supported(self):
        # The URL regex uses (\d{4}\.\d{4,5}) which does not match
        # the old-style URL format like https://arxiv.org/abs/cs.AI/0704123
        assert normalize_arxiv_id("https://arxiv.org/abs/cs.AI/0704123") is None
        assert normalize_arxiv_id("https://arxiv.org/pdf/cs.AI/0704123.pdf") is None
