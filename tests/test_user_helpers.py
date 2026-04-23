"""Tests for user-friendly error messages and helpers."""
import pytest
from core.user_helpers import (
    UserError,
    DatabaseError,
    APIError,
    ParseError,
    format_error,
    print_table,
    ProgressIndicator,
)


def test_user_error_creation():
    """Test UserError creation."""
    error = UserError("Test error", "Try this")
    assert str(error) == "Test error"
    assert error.suggestion == "Try this"
    assert "Test error" in error.get_helpful_message()
    assert "Try this" in error.get_helpful_message()


def test_database_error_not_found():
    """Test DatabaseError.not_found."""
    error = DatabaseError.not_found("论文", "2301.001")
    assert "未找到" in str(error)
    assert "2301.001" in str(error)


def test_database_error_connection():
    """Test DatabaseError.connection_failed."""
    error = DatabaseError.connection_failed()
    assert "连接失败" in str(error)


def test_api_error_rate_limit():
    """Test APIError.rate_limit."""
    error = APIError.rate_limit("arxiv", 30)
    # Check error message contains relevant info
    msg = str(error)
    assert "arxiv" in msg


def test_api_error_network():
    """Test APIError.network_failed."""
    error = APIError.network_failed()
    assert "网络" in str(error)


def test_api_error_auth():
    """Test APIError.auth_failed."""
    error = APIError.auth_failed()
    assert "认证" in str(error)


def test_parse_error_pdf():
    """Test ParseError.pdf_failed."""
    error = ParseError.pdf_failed("2301.001")
    assert "解析" in str(error)
    assert "2301.001" in str(error)


def test_format_error_user_error():
    """Test format_error with UserError."""
    error = UserError("Test", "Suggestion")
    formatted = format_error(error)
    assert "Test" in formatted
    assert "Suggestion" in formatted


def test_format_error_regular_error():
    """Test format_error with regular Exception."""
    error = ValueError("Regular error")
    formatted = format_error(error)
    assert "Regular error" in formatted


def test_print_table():
    """Test table printing."""
    headers = ["名称", "数量"]
    rows = [
        ["项目A", "10"],
        ["项目B", "20"]
    ]
    
    # Should not raise exception
    print_table(headers, rows)


def test_progress_indicator():
    """Test progress indicator."""
    with ProgressIndicator(10, "测试") as progress:
        for _ in range(10):
            progress.update()
    
    # Should complete without error
    assert progress.current == 10


def test_progress_indicator_zero_total():
    """Test progress indicator with zero total."""
    with ProgressIndicator(0, "测试") as progress:
        pass
    
    # Should complete without error
    assert progress.current == 0
