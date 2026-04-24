"""Tests for core/exceptions.py."""
import pytest
from core.exceptions import (
    AIResearchOSError, PDFParseError, APIClientError, NetworkError,
    RateLimitError, PaperNotFoundError, DatabaseError, CacheError,
    ValidationError, ParseTimeoutError, LLMCacheError, ConfigError,
    RetryExhaustedError, InvalidInputError, MissingDependencyError,
    format_error_message,
)


class TestAIResearchOSError:
    def test_base_error(self):
        err = AIResearchOSError("test error")
        assert str(err) == "test error"
        assert err.cause is None

    def test_with_cause(self):
        cause = ValueError("root cause")
        err = AIResearchOSError("wrapper", cause=cause)
        assert str(err) == "wrapper"
        assert err.cause is cause
        assert err.__cause__ is cause

    def test_get_error_info_basic(self):
        err = AIResearchOSError("details")
        info = err.get_error_info()
        assert info["error_type"] == "AIResearchOSError"
        assert info["message"] == "details"
        assert info["has_cause"] is False

    def test_get_error_info_with_cause(self):
        cause = RuntimeError("source")
        err = AIResearchOSError("msg", cause=cause)
        info = err.get_error_info()
        assert info["has_cause"] is True
        assert info["cause"] == "source"

    def test_inheritance_chain(self):
        err = AIResearchOSError()
        assert isinstance(err, Exception)


class TestSubclassHierarchy:
    def test_pdf_parse_error(self):
        err = PDFParseError("bad pdf")
        assert isinstance(err, AIResearchOSError)
        assert str(err) == "bad pdf"

    def test_network_error(self):
        err = NetworkError("no route")
        assert isinstance(err, APIClientError)
        assert isinstance(err, AIResearchOSError)

    def test_rate_limit_error(self):
        err = RateLimitError("429")
        assert isinstance(err, APIClientError)
        assert isinstance(err, AIResearchOSError)

    def test_paper_not_found_error(self):
        err = PaperNotFoundError("idontexist")
        assert isinstance(err, AIResearchOSError)

    def test_database_error(self):
        err = DatabaseError("sqlite fail")
        assert isinstance(err, AIResearchOSError)

    def test_cache_error(self):
        err = CacheError("redis down")
        assert isinstance(err, AIResearchOSError)

    def test_validation_error(self):
        err = ValidationError("bad input")
        assert isinstance(err, AIResearchOSError)

    def test_parse_timeout_error(self):
        err = ParseTimeoutError("took too long")
        assert isinstance(err, PDFParseError)
        assert isinstance(err, AIResearchOSError)

    def test_llm_cache_error(self):
        err = LLMCacheError("cache miss")
        assert isinstance(err, AIResearchOSError)

    def test_config_error(self):
        err = ConfigError("missing key")
        assert isinstance(err, AIResearchOSError)

    def test_missing_dependency_error(self):
        err = MissingDependencyError("tesseract missing", dependency="tesseract")
        assert isinstance(err, AIResearchOSError)
        assert err.dependency == "tesseract"


class TestRetryExhaustedError:
    def test_basic(self):
        err = RetryExhaustedError("gave up", retries=3)
        assert str(err) == "gave up"
        assert err.retries == 3

    def test_with_cause(self):
        cause = OSError("connection refused")
        err = RetryExhaustedError("failed", cause=cause, retries=5)
        assert err.cause is cause
        assert err.retries == 5

    def test_retries_attribute_accessible(self):
        err = RetryExhaustedError(retries=0)
        assert hasattr(err, "retries")


class TestInvalidInputError:
    def test_basic(self):
        err = InvalidInputError("invalid value", field="email", value="not-an-email")
        assert str(err) == "invalid value"
        assert err.field == "email"
        assert err.value == "not-an-email"

    def test_get_error_info_includes_field(self):
        err = InvalidInputError("bad", field="x", value=123)
        info = err.get_error_info()
        assert info["field"] == "x"
        assert info["value_type"] == "int"

    def test_get_error_info_no_field(self):
        err = InvalidInputError("bad")
        info = err.get_error_info()
        assert "field" not in info

    def test_get_error_info_no_value(self):
        err = InvalidInputError("bad", field="x")
        info = err.get_error_info()
        assert "value_type" not in info

    def test_inheritance(self):
        err = InvalidInputError()
        assert isinstance(err, ValidationError)
        assert isinstance(err, AIResearchOSError)


class TestFormatErrorMessage:
    def test_ai_research_os_error(self):
        err = AIResearchOSError("something broke", cause=ValueError("bad input"))
        msg = format_error_message(err)
        assert "[AIResearchOSError]" in msg
        assert "something broke" in msg
        assert "bad input" in msg

    def test_regular_exception(self):
        msg = format_error_message(ValueError("just wrong"))
        assert msg == "just wrong"

    def test_no_cause(self):
        err = PDFParseError("pdf is corrupted")
        msg = format_error_message(err)
        assert "[PDFParseError]" in msg
        assert "pdf is corrupted" in msg
        assert "Caused by" not in msg
