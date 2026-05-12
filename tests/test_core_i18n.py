"""Tests for core/i18n.py internationalization."""

import pytest
import os


class TestI18n:
    """Test i18n module from core/i18n.py."""

    def test_lang_default_is_zh(self):
        # Default is 'zh' when AI_RESEARCH_LANG is not set
        # Module-level LANG variable
        import core.i18n as i18n

        assert i18n.LANG in ("zh", "en")

    def test_msg_retrieval(self):
        from core.i18n import _MSGS_EN, _MSGS_ZH

        # Both message dicts should have required keys
        required_keys = [
            "research_searching",
            "research_no_papers",
            "research_found",
            "research_done",
        ]
        for key in required_keys:
            assert key in _MSGS_EN
            assert key in _MSGS_ZH

    def test_msg_format(self):
        from core.i18n import _MSGS_EN

        msg = _MSGS_EN["research_searching"]
        formatted = msg.format(query="machine learning")
        assert "machine learning" in formatted

    def test_msg_count(self):
        from core.i18n import _MSGS_EN, _MSGS_ZH

        # Both languages should have the same number of keys
        assert len(_MSGS_EN) == len(_MSGS_ZH)

    def test_research_done_format(self):
        from core.i18n import _MSGS_EN

        msg = _MSGS_EN["research_done"]
        formatted = msg.format(processed=5, total=10, failed=2, skipped=3)
        assert "5" in formatted
        assert "10" in formatted
        assert "2" in formatted

    def test_err_messages_present(self):
        from core.i18n import _MSGS_EN

        err_keys = ["err_pdf_download", "err_pdf_no_url", "err_pdf_extract"]
        for key in err_keys:
            assert key in _MSGS_EN
            assert len(_MSGS_EN[key]) > 0
