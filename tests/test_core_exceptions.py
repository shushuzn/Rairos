"""Tests for core/exceptions.py exception hierarchy."""
import pytest


class TestAIResearchOSError:
    """Test base AIResearchOSError from core/exceptions.py."""

    def _base_error(self):
        from core.exceptions import AIResearchOSError

        return AIResearchOSError

    def test_message(self):
        from core.exceptions import AIResearchOSError

        err = AIResearchOSError("Something went wrong")
        assert str(err) == "Something went wrong"

    def test_cause(self):
        from core.exceptions import AIResearchOSError

        inner = ValueError("inner cause")
        err = AIResearchOSError("outer", cause=inner)
        assert err.cause is inner

    def test_get_error_info(self):
        from core.exceptions import AIResearchOSError

        err = AIResearchOSError("test message")
        info = err.get_error_info()
        assert info["error_type"] == "AIResearchOSError"
        assert info["message"] == "test message"
        assert info["has_cause"] is False

    def test_get_error_info_with_cause(self):
        from core.exceptions import AIResearchOSError

        inner = RuntimeError("inner")
        err = AIResearchOSError("outer", cause=inner)
        info = err.get_error_info()
        assert info["has_cause"] is True
        assert info["cause"] == "inner"


class TestPDFParseError:
    """Test PDFParseError from core/exceptions.py."""

    def test_inherits_from_base(self):
        from core.exceptions import AIResearchOSError, PDFParseError

        err = PDFParseError("PDF extraction failed")
        assert isinstance(err, AIResearchOSError)
        assert isinstance(err, Exception)

    def test_message(self):
        from core.exceptions import PDFParseError

        err = PDFParseError("Cannot parse file.pdf")
        assert "Cannot parse file.pdf" in str(err)


class TestPaperNotFoundError:
    """Test PaperNotFoundError from core/exceptions.py."""

    def test_inherits_from_base(self):
        from core.exceptions import AIResearchOSError, PaperNotFoundError

        err = PaperNotFoundError("2301.00001")
        assert isinstance(err, AIResearchOSError)

    def test_message_contains_id(self):
        from core.exceptions import PaperNotFoundError

        err = PaperNotFoundError("2301.00001")
        assert "2301.00001" in str(err)


class TestValidationError:
    """Test ValidationError from core/exceptions.py."""

    def test_inherits_from_base(self):
        from core.exceptions import AIResearchOSError, ValidationError

        err = ValidationError("Invalid field")
        assert isinstance(err, AIResearchOSError)

    def test_message(self):
        from core.exceptions import ValidationError

        err = ValidationError("field 'title' is required")
        assert "field 'title'" in str(err)
