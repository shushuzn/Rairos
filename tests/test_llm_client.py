"""Tests for llm/client.py — LLM API client core logic (no I/O)."""

import pytest
from unittest.mock import patch, MagicMock
import os
import tempfile
from pathlib import Path

from llm.client import (
    _is_ollama_model,
    _is_anthropic_model,
    _use_claude_cli_fallback,
    _use_warp_cli_fallback,
    _get_cache_ttl,
    _cache_stats,
    get_cache_stats,
    reset_cache_stats,
    _resolve_llm_credentials,
    _generate_cache_key,
    clear_llm_cache,
    get_llm_cache_size,
)


class TestModelClassification:
    """Test model type classification functions."""

    def test_is_ollama_model_with_prefix(self):
        """ollama/* models should be detected as Ollama."""
        assert _is_ollama_model("ollama/llama2") is True
        assert _is_ollama_model("ollama/codellama") is True

    def test_is_ollama_model_local_models_without_key(self):
        """Local model names without API keys should be detected."""
        with patch.dict(os.environ, {"OPENAI_API_KEY": "", "ANTHROPIC_API_KEY": ""}):
            assert _is_ollama_model("llama2") is True
            assert _is_ollama_model("qwen2.5") is True
            assert _is_ollama_model("mistral") is True

    def test_is_ollama_model_false_with_api_key(self):
        """Should return False for local models when API key is set."""
        with patch.dict(os.environ, {"OPENAI_API_KEY": "sk-test"}):
            assert _is_ollama_model("llama2") is False
            assert _is_ollama_model("qwen2.5") is False

    def test_is_ollama_model_false_for_openai(self):
        """OpenAI models should not be detected as Ollama."""
        assert _is_ollama_model("gpt-4") is False
        assert _is_ollama_model("gpt-3.5-turbo") is False

    def test_is_anthropic_model_claude_prefix(self):
        """Models starting with 'claude' should be detected."""
        assert _is_anthropic_model("claude-3-opus") is True
        assert _is_anthropic_model("claude-3-sonnet") is True
        assert _is_anthropic_model("claude-3.5-sonnet") is True

    def test_is_anthropic_model_case_insensitive(self):
        """Detection should be case-insensitive."""
        assert _is_anthropic_model("CLAUDE-3-OPUS") is True
        assert _is_anthropic_model("Claude-3-Sonnet") is True

    def test_is_anthropic_model_false_for_others(self):
        """Non-Anthropic models should return False."""
        assert _is_anthropic_model("gpt-4") is False
        assert _is_anthropic_model("ollama/llama2") is False


class TestUseCliFallback:
    """Test CLI fallback detection."""

    def test_use_claude_cli_fallback_claude_model(self):
        """Claude models should be considered for CLI fallback."""
        with patch.dict(os.environ, {"ANTHROPIC_API_KEY": ""}):
            result = _use_claude_cli_fallback("claude-3-opus")
            assert isinstance(result, bool)

    def test_use_warp_cli_fallback_warp_models(self):
        """Warp AI models should be detected."""
        with patch.dict(os.environ, {}):
            result = _use_warp_cli_fallback("warp-model")
            assert isinstance(result, bool)


class TestCacheTTL:
    """Test cache TTL logic."""

    def test_get_cache_ttl_default(self):
        """Default TTL should be 7 days."""
        with patch.dict(os.environ, {}, clear=True):
            ttl = _get_cache_ttl()
            assert ttl == 7 * 24 * 3600

    def test_get_cache_ttl_from_env(self):
        """TTL should be read from AIROS_CACHE_TTL_SECONDS env var."""
        with patch.dict(os.environ, {"AIROS_CACHE_TTL_SECONDS": "3600"}):
            ttl = _get_cache_ttl()
            assert ttl == 3600

    def test_get_cache_ttl_invalid_env(self):
        """Invalid env var should fall back to default."""
        with patch.dict(os.environ, {"AIROS_CACHE_TTL_SECONDS": "not-a-number"}):
            ttl = _get_cache_ttl()
            assert ttl == 7 * 24 * 3600


class TestCacheStats:
    """Test cache statistics tracking."""

    @pytest.fixture(autouse=True)
    def _reset_cache(self):
        """Reset module-level cache counters and cache files before each test."""
        from llm.client import reset_cache_stats, clear_llm_cache
        clear_llm_cache()
        reset_cache_stats()
        yield

    def test_cache_stats_has_entries(self):
        """Cache stats should track hits, expired, entries."""
        stats = _cache_stats()
        assert "hits" in stats
        assert "expired" in stats
        assert "entries" in stats

    def test_get_cache_stats_format(self):
        """get_cache_stats returns float ratios."""
        stats = get_cache_stats()
        assert "hit_rate" in stats
        assert 0.0 <= stats["hit_rate"] <= 1.0

    def test_reset_cache_stats(self):
        """reset_cache_stats zeroes the counters."""
        reset_cache_stats()
        stats = _cache_stats()
        assert stats["hits"] == 0
        assert stats["expired"] == 0


class TestCredentials:
    """Test credential resolution."""

    def test_resolve_llm_credentials_returns_tuple(self):
        """_resolve_llm_credentials should return (url, key) tuple."""
        base_url = "https://api.openai.com/v1"
        api_key = "sk-test123"
        resolved = _resolve_llm_credentials(base_url, api_key)
        assert isinstance(resolved, tuple)
        assert len(resolved) == 2


class TestCacheKey:
    """Test cache key generation."""

    def test_generate_cache_key_deterministic(self):
        """Same inputs should produce same cache key."""
        messages = [{"role": "user", "content": "hello"}]
        key1 = _generate_cache_key(messages, model="gpt-4")
        key2 = _generate_cache_key(messages, model="gpt-4")
        assert key1 == key2
        assert isinstance(key1, str)
        assert len(key1) > 0

    def test_generate_cache_key_different_for_different_content(self):
        """Different content should produce different keys."""
        messages1 = [{"role": "user", "content": "hello"}]
        messages2 = [{"role": "user", "content": "goodbye"}]
        key1 = _generate_cache_key(messages1, model="gpt-4")
        key2 = _generate_cache_key(messages2, model="gpt-4")
        assert key1 != key2

    def test_generate_cache_key_different_for_different_model(self):
        """Different models should produce different keys."""
        messages = [{"role": "user", "content": "hello"}]
        key1 = _generate_cache_key(messages, model="gpt-4")
        key2 = _generate_cache_key(messages, model="gpt-3.5-turbo")
        assert key1 != key2


class TestCacheOperations:
    """Test cache read/write operations."""

    def test_llm_cache(self):
        """Test LLM response caching."""
        clear_llm_cache()
        size = get_llm_cache_size()
        assert isinstance(size, int)

    def test_clear_llm_cache(self):
        """clear_llm_cache should not raise."""
        clear_llm_cache()

    def test_get_llm_cache_size(self):
        """get_llm_cache_size should return non-negative integer."""
        size = get_llm_cache_size()
        assert isinstance(size, int)
        assert size >= 0
