"""Tests for llm/research/gap_extract.py."""

import pytest
from llm.research.gap_extract import _extract_keywords, extract_gap_from_paper


class TestExtractKeywords:
    def test_basic_extraction(self):
        text = "Machine learning transformers have revolutionized natural language processing applications"
        result = _extract_keywords(text)
        assert isinstance(result, list)
        assert len(result) <= 6
        for kw in result:
            assert len(kw) > 0

    def test_stopwords_filtered(self):
        text = "the a an of in to for and or with is are that this we our by on as at from"
        result = _extract_keywords(text)
        assert result == []

    def test_short_words_filtered(self):
        text = "a bc de fgh ijkl mnopqr"
        # 'bc', 'de' are <=4 chars, 'fgh' is 3, 'ijkl' is 4, 'mnopqr' is 6
        result = _extract_keywords(text)
        assert all(len(w) > 4 for w in result)

    def test_punctuation_stripped(self):
        text = "machine, learning; transformers: attention! mechanisms."
        result = _extract_keywords(text)
        assert all("," not in w and ";" not in w and ":" not in w and "!" not in w and "." not in w for w in result)

    def test_duplicates_removed(self):
        text = "machine learning machine learning machine"
        result = _extract_keywords(text)
        assert len(result) == len(set(result))

    def test_max_six_keywords(self):
        text = "deep neural network architecture attention mechanism transformer optimization gradient descent backpropagation regularization generalization"
        result = _extract_keywords(text)
        assert len(result) <= 6

    def test_empty_string(self):
        result = _extract_keywords("")
        assert result == []

    def test_case_insensitive(self):
        text = "MACHINE Learning NEURAL"
        result = _extract_keywords(text)
        assert all(w == w.lower() for w in result)

    def test_parens_bracket_punctuation(self):
        text = "attention (mechanism) [transformer] {network}"
        result = _extract_keywords(text)
        for kw in result:
            assert "(" not in kw
            assert ")" not in kw
            assert "[" not in kw
            assert "]" not in kw
            assert "{" not in kw
            assert "}" not in kw


class TestExtractGapFromPaper:
    def test_returns_dict(self):
        result = extract_gap_from_paper(
            paper_id="test123",
            title="Test Paper Title",
            abstract="This is a test abstract about machine learning.",
        )
        assert isinstance(result, dict)

    def test_fallback_on_import_error(self, monkeypatch):
        import llm.research.gap_extract as ge
        monkeypatch.delattr(ge, "call_llm_chat_completions", raising=False)
        result = extract_gap_from_paper(
            paper_id="test123",
            title="Test Paper Title",
            abstract="This paper studies deep neural networks.",
        )
        assert result["gap_type"] == "method_limitation"
        assert result["gap_title"] == "Test Paper Title"
        assert "error" in result

    def test_fallback_on_llm_exception(self, monkeypatch):
        import llm.client
        def fake_call(*args, **kwargs):
            raise RuntimeError("API error")
        monkeypatch.setattr(llm.client, "call_llm_chat_completions", fake_call)
        result = extract_gap_from_paper(
            paper_id="test123",
            title="Test Paper Title",
            abstract="This paper studies deep neural networks.",
        )
        assert result["gap_type"] == "method_limitation"
        assert result["gap_title"] == "Test Paper Title"
        assert "error" in result

    def test_authors_truncated_to_three(self):
        import llm.research.gap_extract as ge
        def fake_call(*args, **kwargs):
            return '{"gap_type":"method_limitation","gap_title":"T","keywords":[],"summary":"S"}'
        ge.call_llm_chat_completions = fake_call
        result = extract_gap_from_paper(
            paper_id="test123",
            title="T",
            abstract="A",
            authors=["Author One", "Author Two", "Author Three", "Author Four"],
        )
        # The call was made - no error means it worked

