"""Tests for core/user_helpers.py."""
import json
import sys
from io import StringIO
from pathlib import Path
import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))
from core.user_helpers import (
    UserError, DatabaseError, APIError, ParseError,
    format_error, print_error, ProgressIndicator,
    print_table, print_json, print_banner,
)


class TestUserErrorHierarchy:
    def test_user_error_basic(self):
        err = UserError("test message", suggestion="try this")
        assert str(err) == "test message"
        assert err.suggestion == "try this"

    def test_user_error_get_helpful_message(self):
        err = UserError("error msg", suggestion="fix it")
        msg = err.get_helpful_message()
        assert "error msg" in msg
        assert "fix it" in msg

    def test_database_error_basic(self):
        err = DatabaseError("db error")
        assert str(err) == "db error"
        assert isinstance(err, UserError)

    def test_database_error_not_found(self):
        err = DatabaseError.not_found("paper", "abc123")
        assert "abc123" in str(err)
        assert err.suggestion is not None

    def test_database_error_connection_failed(self):
        err = DatabaseError.connection_failed()
        assert "连接失败" in str(err)
        assert err.suggestion is not None

    def test_api_error_basic(self):
        err = APIError("api error")
        assert isinstance(err, UserError)

    def test_api_error_rate_limit(self):
        err = APIError.rate_limit("/search", 60)
        assert "API请求过于频繁" in str(err)
        assert err.suggestion is not None

    def test_api_error_network_failed(self):
        err = APIError.network_failed()
        assert "网络" in str(err)

    def test_api_error_auth_failed(self):
        err = APIError.auth_failed()
        assert "认证" in str(err)

    def test_parse_error_basic(self):
        err = ParseError("parse fail")
        assert isinstance(err, UserError)

    def test_parse_error_pdf_failed(self):
        err = ParseError.pdf_failed("paper-x")
        assert "paper-x" in str(err)


class TestFormatError:
    def test_format_user_error(self):
        err = UserError("msg", "sug")
        formatted = format_error(err)
        assert "msg" in formatted

    def test_format_generic_error(self):
        formatted = format_error(ValueError("bad value"))
        assert "bad value" in formatted


class TestPrintError:
    def test_print_error_writes_to_stderr(self, capsys):
        err = UserError("oops")
        print_error(err)
        captured = capsys.readouterr()
        assert "oops" in captured.err


class TestProgressIndicator:
    def test_update_increments_current(self):
        p = ProgressIndicator(10, "Test")
        p.update(3)
        assert p.current == 3

    def test_update_default_increment(self):
        p = ProgressIndicator(10)
        p.update()
        assert p.current == 1

    def test_context_manager(self):
        with ProgressIndicator(5, "Test") as p:
            p.update(2)
            assert p.current == 2
        assert p.current == 2

    def test_zero_total_no_divide_by_zero(self):
        p = ProgressIndicator(0)
        p.update()
        assert p.current == 1


class TestPrintTable:
    def test_print_table_with_data(self, capsys):
        print_table(["Name", "Age"], [["Alice", "30"], ["Bob", "25"]])
        captured = capsys.readouterr().out
        assert "Name" in captured
        assert "Alice" in captured


class TestPrintJson:
    def test_print_json_formats_data(self, capsys):
        print_json({"key": "value"})
        captured = capsys.readouterr().out
        assert "key" in captured
        assert "value" in captured


class TestPrintBanner:
    def test_print_banner_prints_text(self, capsys):
        print_banner("HELLO")
        captured = capsys.readouterr().out
        assert "HELLO" in captured
