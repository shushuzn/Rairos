"""Tests for exception handling functionality."""
from core.exceptions import (
    AIResearchOSError,
    PDFParseError,
    ValidationError,
    InvalidInputError,
    MissingDependencyError,
    format_error_message,
)


def test_base_exception_get_error_info():
    """Test base exception get_error_info method."""
    error = AIResearchOSError("Test error message")
    info = error.get_error_info()
    
    assert info["error_type"] == "AIResearchOSError"
    assert info["message"] == "Test error message"
    assert info["has_cause"] is False


def test_base_exception_with_cause():
    """Test base exception with cause."""
    cause = ValueError("Original error")
    error = AIResearchOSError("Wrapped error", cause=cause)
    info = error.get_error_info()
    
    assert info["has_cause"] is True
    assert "Original error" in info["cause"]


def test_pdf_parse_error():
    """Test PDFParseError exception."""
    error = PDFParseError("PDF parsing failed")
    assert str(error) == "PDF parsing failed"


def test_validation_error():
    """Test ValidationError exception."""
    error = ValidationError("Invalid input")
    assert str(error) == "Invalid input"


def test_invalid_input_error():
    """Test InvalidInputError with field and value."""
    error = InvalidInputError(
        message="Invalid email format",
        field="email",
        value="not-an-email"
    )
    info = error.get_error_info()
    
    assert info["error_type"] == "InvalidInputError"
    assert info["field"] == "email"
    assert info["value_type"] == "str"


def test_missing_dependency_error():
    """Test MissingDependencyError."""
    error = MissingDependencyError(
        message="Required package not found",
        dependency="pymupdf"
    )
    info = error.get_error_info()
    
    assert info["error_type"] == "MissingDependencyError"
    assert error.dependency == "pymupdf"


def test_format_error_message_with_airesearch_error():
    """Test formatting AIResearchOSError."""
    error = AIResearchOSError("Test error")
    formatted = format_error_message(error)
    
    assert "AIResearchOSError" in formatted
    assert "Test error" in formatted


def test_format_error_message_with_cause():
    """Test formatting error with cause."""
    cause = ValueError("Original")
    error = AIResearchOSError("Wrapped", cause=cause)
    formatted = format_error_message(error)
    
    assert "Wrapped" in formatted
    assert "Original" in formatted


def test_format_error_message_with_invalid_input():
    """Test formatting InvalidInputError."""
    error = InvalidInputError(
        message="Invalid format",
        field="title",
        value=123
    )
    formatted = format_error_message(error)
    
    assert "InvalidInputError" in formatted
    assert "Invalid format" in formatted
    # Field info is in the error info dict, not in formatted message
    info = error.get_error_info()
    assert info["field"] == "title"


def test_exception_inheritance():
    """Test that custom exceptions inherit from base exception."""
    error = PDFParseError("PDF Error")
    assert isinstance(error, AIResearchOSError)
    assert isinstance(error, PDFParseError)
