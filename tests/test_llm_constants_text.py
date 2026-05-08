"""Tests for llm/constants.py and llm/text_utils.py."""

import pytest
from llm.constants import (
    LLM_BASE_URL,
    LLM_MODEL,
    OLLAMA_BASE_URL,
    OLLAMA_EMBEDDING_MODEL,
    OLLAMA_API_EMBEDDINGS_ENDPOINT,
    AI_RESEARCH_KEYWORDS,
    SMART_FOLLOWUP_BASE,
)
from llm.text_utils import extract_keywords


class TestLlmConstants:
    def test_llm_base_url_is_http(self):
        assert LLM_BASE_URL.startswith("http")

    def test_llm_model_is_string(self):
        assert isinstance(LLM_MODEL, str)
        assert len(LLM_MODEL) > 0

    def test_ollama_base_url(self):
        assert "localhost" in OLLAMA_BASE_URL or "ollama" in OLLAMA_BASE_URL

    def test_ollama_embedding_model(self):
        assert isinstance(OLLAMA_EMBEDDING_MODEL, str)
        assert len(OLLAMA_EMBEDDING_MODEL) > 0

    def test_ollama_embeddings_endpoint(self):
        assert OLLAMA_API_EMBEDDINGS_ENDPOINT.startswith("/")

    def test_ai_research_keywords_is_frozenset_or_set(self):
        assert isinstance(AI_RESEARCH_KEYWORDS, (set, frozenset))
        assert len(AI_RESEARCH_KEYWORDS) > 40

    def test_ai_research_keywords_contains_core_terms(self):
        core = {"transformer", "llm", "rlhf", "attention", "diffusion"}
        assert core.issubset(AI_RESEARCH_KEYWORDS)

    def test_smart_followup_base_is_set(self):
        assert isinstance(SMART_FOLLOWUP_BASE, set)
        assert len(SMART_FOLLOWUP_BASE) > 50

    def test_smart_followup_base_contains_nlp_terms(self):
        nlp = {"transformer", "attention", "embedding"}
        assert nlp.issubset(SMART_FOLLOWUP_BASE)


class TestExtractKeywords:
    def test_basic_extraction(self):
        result = extract_keywords("Machine learning and deep neural networks")
        assert isinstance(result, list)
        assert "machine" in result
        assert "learning" in result

    def test_stopwords_removed(self):
        result = extract_keywords("the and for are but not")
        assert len(result) == 0

    def test_min_len_filter(self):
        result = extract_keywords("ai llm gpt", min_len=3)
        assert "ai" not in result  # too short
        assert "llm" in result
        assert "gpt" in result

    def test_case_insensitive(self):
        result = extract_keywords("TRANSFORMER Attention BERT")
        assert "transformer" in result
        assert "attention" in result
        assert "bert" in result

    def test_numbers_kept(self):
        result = extract_keywords("BERT model 2024 3.5")
        assert "2024" in result

    def test_research_keywords_extracted(self):
        text = "We propose a new transformer architecture with attention mechanism"
        result = extract_keywords(text)
        assert "transformer" in result
        assert "attention" in result

    def test_gap_issue_problem_removed(self):
        text = "this gap issue problem limitation study"
        result = extract_keywords(text)
        assert "gap" not in result
        assert "issue" not in result
        assert "problem" not in result
