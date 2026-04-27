"""Tier 2 unit tests — llm/text_utils.py, pure functions, no I/O."""
import pytest
from llm.text_utils import extract_keywords


class TestExtractKeywords:
    """Test extract_keywords function."""

    def test_empty_string(self):
        """Empty input returns empty list."""
        assert extract_keywords("") == []

    def test_none_raises_error(self):
        """None input raises AttributeError (not handled gracefully)."""
        with pytest.raises(AttributeError):
            extract_keywords(None)

    def test_short_words_filtered(self):
        """Words shorter than min_len are filtered."""
        result = extract_keywords("a an the is at be to of", min_len=3)
        assert result == []

    def test_stopwords_filtered(self):
        """Common stopwords are filtered."""
        result = extract_keywords("the and for are but not you all can had")
        assert result == []

    def test_gap_problem_stopwords(self):
        """Research stopwords are filtered."""
        result = extract_keywords("method approach gap issue problem limitation study work paper")
        assert result == []

    def test_returns_lowercase(self):
        """Keywords are returned lowercase."""
        result = extract_keywords("TRANSFORMER ATTENTION MECHANISM")
        assert result == ["transformer", "attention", "mechanism"]

    def test_mixed_case(self):
        """Mixed case is normalized."""
        result = extract_keywords("Transformer attention Mechanism")
        assert result == ["transformer", "attention", "mechanism"]

    def test_numbers_included(self):
        """Numbers are included as keywords, hyphens split them."""
        result = extract_keywords("BERT GPT-3 2024 v2")
        assert "bert" in result
        assert "gpt" in result  # hyphen splits: "gpt-3" → "gpt", "3"
        assert "2024" in result
        assert "v2" not in result  # "v2" has only 2 chars, filtered by min_len=3

    def test_min_len_default(self):
        """Default min_len is 3."""
        result = extract_keywords("ab abc abcd")
        assert "ab" not in result
        assert "abc" in result
        assert "abcd" in result

    def test_min_len_custom(self):
        """Custom min_len works."""
        result = extract_keywords("ab abc abcde", min_len=4)
        assert "ab" not in result
        assert "abc" not in result
        assert "abcde" in result

    def test_real_abstract(self):
        """Real abstract content extracts meaningful keywords."""
        abstract = (
            "We propose a novel Transformer architecture with self-attention mechanism "
            "for natural language understanding tasks. Our model achieves state-of-the-art "
            "results on benchmark datasets including SQuAD and GLUE."
        )
        result = extract_keywords(abstract)
        assert "transformer" in result
        assert "self" in result  # hyphen splits: "self-attention" → "self", "attention"
        assert "attention" in result
        assert "architecture" in result
        assert "natural" in result
        assert "language" in result
        assert "model" in result
        assert "results" in result
        assert "benchmark" in result
        assert "datasets" in result

    def test_chinese_not_extracted(self):
        """Chinese characters are not matched by regex."""
        result = extract_keywords("深度学习 transformer")
        assert "深度学习" not in result
        assert "transformer" in result

    def test_underscore_split(self):
        """Underscores are NOT captured by [A-Za-z0-9]+ regex."""
        result = extract_keywords("self_attention cross_attention")
        assert "self" in result  # underscore splits
        assert "attention" in result
        assert "cross" in result
        assert "self_attention" not in result

    def test_hyphen_split(self):
        """Hyphens are NOT captured by [A-Za-z0-9]+ regex."""
        result = extract_keywords("state-of-the-art pre-training")
        assert "state" in result
        assert "art" in result
        assert "pre" in result
        assert "training" in result
        assert "state-of-the-art" not in result
        assert "pre-training" not in result
