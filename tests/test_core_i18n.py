"""Tests for internationalization (i18n) functionality."""
import pytest
from core.i18n import set_lang, get_lang, _, _MSGS


def test_get_lang_default():
    """Test default language is zh."""
    original_lang = get_lang()
    set_lang("zh")
    assert get_lang() == "zh"
    # Restore original
    set_lang(original_lang)


def test_set_lang_zh():
    """Test setting language to Chinese."""
    set_lang("zh")
    assert get_lang() == "zh"


def test_set_lang_en():
    """Test setting language to English."""
    set_lang("en")
    assert get_lang() == "en"


def test_set_lang_short_e():
    """Test setting language with short code 'e'."""
    set_lang("e")
    assert get_lang() == "en"


def test_set_lang_short_z():
    """Test setting language with short code 'z'."""
    set_lang("z")
    assert get_lang() == "zh"


def test_set_lang_invalid():
    """Test setting invalid language defaults to zh."""
    set_lang("invalid")
    assert get_lang() == "zh"


def test_underscore_zh():
    """Test translation function with Chinese."""
    set_lang("zh")
    msg = _("research_searching", query="LLM")
    assert "LLM" in msg
    assert "搜索" in msg


def test_underscore_en():
    """Test translation function with English."""
    set_lang("en")
    msg = _("research_searching", query="LLM")
    assert "LLM" in msg
    assert "Searching" in msg


def test_underscore_no_kwargs():
    """Test translation function without kwargs."""
    set_lang("zh")
    msg = _("research_no_papers")
    assert "未找到" in msg


def test_underscore_fallback():
    """Test translation function falls back to key for unknown key."""
    set_lang("zh")
    msg = _("unknown_key")
    assert msg == "unknown_key"


def test_all_zh_keys_exist():
    """Test all keys exist in Chinese messages."""
    set_lang("zh")
    zh_keys = _MSGS["zh"].keys()
    assert "research_searching" in zh_keys
    assert "research_no_papers" in zh_keys
    assert "research_found" in zh_keys


def test_all_en_keys_exist():
    """Test all keys exist in English messages."""
    set_lang("en")
    en_keys = _MSGS["en"].keys()
    assert "research_searching" in en_keys
    assert "research_no_papers" in en_keys
    assert "research_found" in en_keys
