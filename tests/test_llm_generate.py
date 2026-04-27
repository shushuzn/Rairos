"""Tier 2 unit tests — llm/generate.py, pure functions, no I/O."""
import pytest
from llm.generate import (
    estimate_tokens,
    get_model_price,
    estimate_cost,
)


# =============================================================================
# estimate_tokens — token estimation
# =============================================================================
class TestEstimateTokens:
    """Test token estimation logic."""

    def test_empty_string_returns_zero(self):
        """Empty input returns 0 tokens."""
        assert estimate_tokens("") == 0
        assert estimate_tokens(None) == 0  # type check

    def test_short_text_returns_minimum_one(self):
        """Very short text returns at least 1 token."""
        assert estimate_tokens("a") == 1
        assert estimate_tokens("hi") == 1

    def test_divides_by_four(self):
        """Token estimate = chars / 4."""
        assert estimate_tokens("abcd") == 1
        assert estimate_tokens("a" * 8) == 2
        assert estimate_tokens("a" * 12) == 3

    def test_chinese_text_estimation(self):
        """Chinese characters: ~1 token per char (4 chars = 1 token)."""
        result = estimate_tokens("深度学习")
        assert result == 1  # 4 chars / 4

    def test_mixed_chinese_english(self):
        """Mixed content: total chars / 4."""
        result = estimate_tokens("Attention is all you need")
        # 27 chars / 4 = 6
        assert result == 6


# =============================================================================
# get_model_price — model pricing lookup
# =============================================================================
class TestGetModelPrice:
    """Test model price lookup logic."""

    def test_returns_tuple(self):
        """Returns (input_per_1M, output_per_1M) tuple."""
        price = get_model_price("gpt-4o")
        assert isinstance(price, tuple)
        assert len(price) == 2

    def test_gpt4o_pricing(self):
        """GPT-4o pricing lookup."""
        inp, out = get_model_price("gpt-4o")
        assert inp > 0
        assert out > 0
        assert inp < out  # Output typically more expensive

    def test_gpt35_turbo_pricing(self):
        """GPT-3.5-turbo pricing lookup."""
        inp, out = get_model_price("gpt-3.5-turbo")
        assert inp >= 0
        assert out >= 0

    def test_case_insensitive(self):
        """Model lookup is case-insensitive."""
        price_lower = get_model_price("gpt-4o")
        price_upper = get_model_price("GPT-4O")
        price_mixed = get_model_price("Gpt-4o")
        assert price_lower == price_upper == price_mixed

    def test_partial_match(self):
        """Partial model name matches."""
        # Should match 'gpt' in 'gpt-4o-mini'
        price1 = get_model_price("gpt-4o-mini")
        price2 = get_model_price("gpt-4o")
        # Both should return gpt prices
        assert price1[0] > 0

    def test_unknown_model_defaults(self):
        """Unknown model returns default pricing."""
        price = get_model_price("completely-unknown-model-xyz")
        default = get_model_price("default")
        assert price == default


# =============================================================================
# estimate_cost — cost calculation
# =============================================================================
class TestEstimateCost:
    """Test cost estimation logic."""

    def test_returns_dict_with_all_keys(self):
        """Returns dict with all expected keys."""
        result = estimate_cost("gpt-4o", "input text", "output text")
        assert "input_tokens" in result
        assert "output_tokens" in result
        assert "total_tokens" in result
        assert "input_cost_usd" in result
        assert "output_cost_usd" in result
        assert "total_cost_usd" in result

    def test_total_equals_sum_of_parts(self):
        """Total cost = input + output cost."""
        result = estimate_cost("gpt-4o", "hello world", "response")
        assert result["total_cost_usd"] == pytest.approx(
            result["input_cost_usd"] + result["output_cost_usd"]
        )

    def test_total_tokens_equals_sum(self):
        """Total tokens = input + output tokens."""
        result = estimate_cost("gpt-4o", "hello", "world")
        assert result["total_tokens"] == result["input_tokens"] + result["output_tokens"]

    def test_empty_input(self):
        """Empty input text handled correctly."""
        result = estimate_cost("gpt-4o", "", "output")
        assert result["input_tokens"] == 0
        assert result["input_cost_usd"] == 0.0

    def test_empty_output(self):
        """Empty output text handled correctly."""
        result = estimate_cost("gpt-4o", "input", "")
        assert result["output_tokens"] == 0
        assert result["output_cost_usd"] == 0.0

    def test_cost_proportional_to_tokens(self):
        """Cost increases with token count."""
        short = estimate_cost("gpt-4o", "a", "b")
        long = estimate_cost("gpt-4o", "a" * 100, "b" * 100)
        assert long["total_cost_usd"] > short["total_cost_usd"]

    def test_cost_precision(self):
        """Cost is rounded to 6 decimal places."""
        result = estimate_cost("gpt-4o", "test", "result")
        # Check precision is reasonable (not raw float)
        cost_str = str(result["total_cost_usd"])
        decimal_places = len(cost_str.split(".")[-1]) if "." in cost_str else 0
        assert decimal_places <= 6

    def test_different_models_different_costs(self):
        """Different models have different pricing."""
        cheap_model = "gpt-3.5-turbo"
        expensive_model = "gpt-4o"
        text = "a" * 1000

        cost_cheap = estimate_cost(cheap_model, text, text)
        cost_expensive = estimate_cost(expensive_model, text, text)

        # Expensive model should cost more
        assert cost_expensive["total_cost_usd"] > cost_cheap["total_cost_usd"]

    def test_cost_reasonable_magnitude(self):
        """Typical costs are in reasonable range (not astronomical)."""
        result = estimate_cost("gpt-4o", "a" * 1000, "b" * 1000)
        # 500 tokens * $5/1M input + $15/1M output ≈ $0.01
        assert result["total_cost_usd"] < 1.0  # Should be under $1 for this size
        assert result["total_cost_usd"] > 0


# =============================================================================
# Template format validation
# =============================================================================
class TestPnoteTemplate:
    """Test P-Note template structure."""

    def test_pnote_user_template_has_required_sections(self):
        """P-Note template includes all required sections."""
        from llm.generate import _PNOTE_USER_PROMPT_TEMPLATE

        required_sections = [
            "paper_title",
            "paper_authors",
            "paper_abstract",
            "paper_body",
            "## 1. 背景",
            "## 2. 核心问题",
            "## 3. 方法结构",
            "## 4. 关键创新",
            "## 5. 实验分析",
            "## 6. 对抗式审稿",
            "## 7. 优势",
            "## 8. 局限",
            "## 14. 评分量表",
        ]

        for section in required_sections:
            assert section in _PNOTE_USER_PROMPT_TEMPLATE, f"Missing: {section}"

    def test_pnote_user_template_has_rubric_format(self):
        """P-Note template includes rubric scoring format."""
        from llm.generate import _PNOTE_USER_PROMPT_TEMPLATE

        assert "Novelty (1-5)" in _PNOTE_USER_PROMPT_TEMPLATE
        assert "Leverage" in _PNOTE_USER_PROMPT_TEMPLATE
        assert "Evidence" in _PNOTE_USER_PROMPT_TEMPLATE
        assert "评分量表" in _PNOTE_USER_PROMPT_TEMPLATE

    def test_pnote_system_prompt_has_hard_rules(self):
        """System prompt includes hard rules."""
        from llm.generate import _PNOTE_SYSTEM_PROMPT

        assert "[违规]" in _PNOTE_SYSTEM_PROMPT
        assert "Claims" in _PNOTE_SYSTEM_PROMPT
        assert "Markdown" in _PNOTE_SYSTEM_PROMPT


class TestCnoteTemplate:
    """Test C-Note template structure."""

    def test_cnote_user_template_has_required_sections(self):
        """C-Note template includes all required sections."""
        from llm.generate import _CNOTE_USER_PROMPT_TEMPLATE

        required_sections = [
            "## 核心定义",
            "## 产生背景",
            "## 技术本质",
            "## 常见实现路径",
            "## 优势",
            "## 局限",
            "## 代表论文",
            "## 演化时间线",
            "## 未来趋势",
        ]

        for section in required_sections:
            assert section in _CNOTE_USER_PROMPT_TEMPLATE, f"Missing: {section}"

    def test_cnote_system_prompt_has_hard_rules(self):
        """System prompt includes hard rules."""
        from llm.generate import _CNOTE_SYSTEM_PROMPT

        assert "[违规]" in _CNOTE_SYSTEM_PROMPT
        assert "Claims" in _CNOTE_SYSTEM_PROMPT


class TestRecommendationTemplate:
    """Test reading recommendation template structure."""

    def test_recommendation_template_has_required_sections(self):
        """Recommendation template includes required sections."""
        from llm.generate import _READ_QUEUE_EXPLANATION_USER_PROMPT_TEMPLATE

        required_sections = [
            "## 推荐理由",
            "## 与已读论文的关联",
            "## 适合阅读的场景",
            "paper_title",
            "score",
            "semantic_score",
            "citation_score",
            "tag_score",
            "recency_score",
        ]

        for section in required_sections:
            assert section in _READ_QUEUE_EXPLANATION_USER_PROMPT_TEMPLATE, f"Missing: {section}"

    def test_recommendation_system_prompt_rules(self):
        """System prompt has output rules."""
        from llm.generate import _READ_QUEUE_EXPLANATION_SYSTEM_PROMPT

        assert "Markdown" in _READ_QUEUE_EXPLANATION_SYSTEM_PROMPT
        assert "150" in _READ_QUEUE_EXPLANATION_SYSTEM_PROMPT  # 150 chars limit


# =============================================================================
# Template formatting integration
# =============================================================================
class TestPnoteFormatting:
    """Test P-Note template formatting."""

    def test_pnote_template_format_replaces_all_placeholders(self):
        """Template format replaces all placeholders."""
        from llm.generate import _PNOTE_USER_PROMPT_TEMPLATE

        formatted = _PNOTE_USER_PROMPT_TEMPLATE.format(
            paper_title="Test Paper",
            paper_authors="Author One, Author Two",
            paper_source="arxiv",
            paper_uid="2301.00001",
            paper_published="2023",
            paper_tags="AI, ML",
            paper_abstract="This is a test abstract.",
            paper_body="## Test Content\n\nTest body text.",
        )

        # Check placeholders are replaced
        assert "{paper_title}" not in formatted
        assert "{paper_authors}" not in formatted
        assert "{paper_abstract}" not in formatted

        # Check content is present
        assert "Test Paper" in formatted
        assert "This is a test abstract." in formatted


class TestCnoteFormatting:
    """Test C-Note template formatting."""

    def test_cnote_template_format_replaces_placeholders(self):
        """Template format replaces concept and paper placeholders."""
        from llm.generate import _CNOTE_USER_PROMPT_TEMPLATE

        formatted = _CNOTE_USER_PROMPT_TEMPLATE.format(
            concept="Attention Mechanism",
            pnotes_text="论文1：标题",
            num_papers=5,
        )

        assert "Attention Mechanism" in formatted
        assert "5" in formatted  # num_papers
        assert "{concept}" not in formatted


class TestRecommendationFormatting:
    """Test recommendation template formatting."""

    def test_recommendation_template_format_complete(self):
        """Template format replaces all placeholders."""
        from llm.generate import _READ_QUEUE_EXPLANATION_USER_PROMPT_TEMPLATE

        formatted = _READ_QUEUE_EXPLANATION_USER_PROMPT_TEMPLATE.format(
            paper_title="Transformer Paper",
            paper_authors="Vaswani et al",
            paper_year="2017",
            paper_category="NLP",
            score=0.85,
            semantic_score=0.9,
            citation_score=0.8,
            tag_score=0.7,
            recency_score=0.6,
            top_signal="语义相似度",
            top_value=0.9,
            read_papers_str="论文A, 论文B",
        )

        assert "Transformer Paper" in formatted
        assert "0.85" in formatted
        assert "0.9" in formatted
        assert "语义相似度" in formatted
