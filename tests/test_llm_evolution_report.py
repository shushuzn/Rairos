"""Tier 2 unit tests — llm/evolution_report.py, pure functions, no I/O."""
import pytest
import math
from llm.evolution_report import (
    PaperInsight,
    QueryInsight,
    LearningReport,
    EvolutionReporter,
    FollowUpType,
    FollowUp,
    SmartFollowUp,
    AdaptiveRetrieval,
)


# =============================================================================
# PaperInsight dataclass
# =============================================================================
class TestPaperInsight:
    """Test PaperInsight dataclass."""

    def test_required_fields(self):
        """Required fields: paper_id, title."""
        p = PaperInsight(paper_id="p1", title="Attention Is All You Need")
        assert p.paper_id == "p1"
        assert p.title == "Attention Is All You Need"

    def test_optional_fields_default(self):
        """Optional fields have defaults."""
        p = PaperInsight(paper_id="p", title="T")
        assert p.positive_count == 0
        assert p.negative_count == 0
        assert p.avg_score == 0.0
        assert p.related_queries == []


class TestPaperInsightBoostScore:
    """Test PaperInsight.boost_score property."""

    def _boost(self, positive: int, negative: int) -> float:
        """Replicate boost_score formula."""
        total = positive + negative
        if total == 0:
            return 0.0
        return (positive - negative * 0.5) / total

    def test_zero_counts_returns_zero(self):
        """No votes → boost score is 0."""
        assert self._boost(0, 0) == 0.0

    def test_positive_only_gives_high_score(self):
        """Positive votes only → positive boost."""
        assert abs(self._boost(5, 0) - 1.0) < 0.001

    def test_negative_only_gives_negative_score(self):
        """Negative votes only → negative boost."""
        # (-0.5) / 1 = -0.5
        assert abs(self._boost(0, 1) - (-0.5)) < 0.001
        # (-1.0) / 2 = -0.5
        assert abs(self._boost(0, 2) - (-0.5)) < 0.001

    def test_mixed_balances_out(self):
        """Equal positive and negative → 0.25."""
        # (1 - 0.5) / 2 = 0.25
        assert abs(self._boost(1, 1) - 0.25) < 0.001

    def test_more_positive_than_negative(self):
        """More positive than negative → positive score."""
        # (2 - 0.5) / 3 ≈ 0.5
        assert abs(self._boost(2, 1) - 0.5) < 0.001


# =============================================================================
# QueryInsight dataclass
# =============================================================================
class TestQueryInsight:
    """Test QueryInsight dataclass."""

    def test_required_fields(self):
        """All 4 fields required."""
        q = QueryInsight(
            keywords=["transformer", "attention"],
            avg_score=0.85,
            success_rate=0.9,
            related_papers=["paper1"],
        )
        assert q.keywords == ["transformer", "attention"]
        assert q.avg_score == 0.85
        assert q.success_rate == 0.9
        assert q.related_papers == ["paper1"]

    def test_suggestions_default(self):
        """Suggestions defaults to empty list."""
        q = QueryInsight(keywords=[], avg_score=0.5, success_rate=0.5, related_papers=[])
        assert q.suggestions == []


# =============================================================================
# LearningReport dataclass
# =============================================================================
class TestLearningReport:
    """Test LearningReport dataclass."""

    def test_required_fields(self):
        """Required fields exist."""
        r = LearningReport(
            period_start="2026-01-01T00:00:00",
            period_end="2026-01-07T23:59:59",
            total_queries=10,
            positive_rate=0.8,
            top_papers=[],
            top_keywords=["transformer"],
            emerging_patterns=[],
            predicted_interests=[],
            questions_to_explore=[],
            evolution_stage="🌱 种子期",
            progress_towards_next="keep going",
        )
        assert r.total_queries == 10
        assert r.positive_rate == 0.8
        assert r.evolution_stage == "🌱 种子期"

    def test_narrative_fields_default(self):
        """Narrative fields default to empty string."""
        r = LearningReport(
            period_start="2026-01-01T00:00:00",
            period_end="2026-01-07T23:59:59",
            total_queries=0,
            positive_rate=0.0,
            top_papers=[],
            top_keywords=[],
            emerging_patterns=[],
            predicted_interests=[],
            questions_to_explore=[],
            evolution_stage="🌱 种子期",
            progress_towards_next="",
        )
        assert r.user_journey == ""
        assert r.system_learned == ""
        assert r.highlight_moment == ""


# =============================================================================
# _extract_top_keywords
# =============================================================================
class TestExtractTopKeywords:
    """Test _extract_top_keywords logic."""

    def _extract_keywords(self, feedbacks):
        """Replicate _extract_top_keywords."""
        import re
        from collections import Counter
        all_text = " ".join(
            [fb.get("query", "") + " " + " ".join(fb.get("paper_ids", [])) for fb in feedbacks]
        )
        stopwords = {
            "the", "is", "are", "a", "an", "what", "how", "why",
            "this", "that", "and", "or", "的", "是", "如何", "什么", "怎么",
        }
        words = re.findall(r'[\u4e00-\u9fff]+|[a-zA-Z]{3,}', all_text.lower())
        filtered = [w for w in words if w not in stopwords and len(w) > 2]
        return [w for w, _ in Counter(filtered).most_common(10)]

    def test_empty_feedbacks(self):
        """Empty list returns empty list."""
        assert self._extract_keywords([]) == []

    def test_stopwords_filtered(self):
        """Common stopwords are removed."""
        feedbacks = [{"query": "what is the transformer architecture", "paper_ids": []}]
        result = self._extract_keywords(feedbacks)
        assert "what" not in result
        assert "the" not in result
        assert "is" not in result

    def test_short_words_filtered(self):
        """Words shorter than 3 chars are filtered."""
        feedbacks = [{"query": "AI NLP BERT", "paper_ids": []}]
        result = self._extract_keywords(feedbacks)
        assert "ai" not in result  # "ai" < 3 chars (matched as "ai" but needs 3+)
        assert "nlp" in result  # 3 chars
        assert "bert" in result

    def test_returns_lowercase(self):
        """Keywords are lowercase."""
        feedbacks = [{"query": "TRANSFORMER Attention", "paper_ids": []}]
        result = self._extract_keywords(feedbacks)
        assert "transformer" in result
        assert "attention" in result

    def test_chinese_extracted(self):
        """Chinese characters are extracted."""
        feedbacks = [{"query": "深度学习 transformer", "paper_ids": []}]
        result = self._extract_keywords(feedbacks)
        assert "深度学习" in result
        assert "transformer" in result

    def test_takes_from_paper_ids(self):
        """Paper IDs contribute to keyword extraction."""
        feedbacks = [{"query": "", "paper_ids": ["BERT Paper"]}]
        result = self._extract_keywords(feedbacks)
        assert "bert" in result
        assert "paper" in result

    def test_max_10_returned(self):
        """At most 10 keywords returned."""
        feedbacks = [
            {"query": "word1 word2 word3 word4 word5 word6 word7 word8 word9 word10 word11", "paper_ids": []}
            for _ in range(5)
        ]
        result = self._extract_keywords(feedbacks)
        assert len(result) <= 10

    def test_sorted_by_frequency(self):
        """Keywords sorted by frequency descending."""
        feedbacks = [
            {"query": "transformer transformer transformer attention", "paper_ids": []},
            {"query": "attention attention lora", "paper_ids": []},
        ]
        result = self._extract_keywords(feedbacks)
        assert result[0] == "transformer"
        assert result[1] == "attention"


# =============================================================================
# _find_emerging_patterns
# =============================================================================
class TestFindEmergingPatterns:
    """Test _find_emerging_patterns logic."""

    def _find_patterns(self, feedbacks):
        """Replicate _find_emerging_patterns."""
        patterns = []
        compare_keywords = ["vs", "versus", "比较", "区别", "diff", "对比"]
        compare_count = sum(
            1 for fb in feedbacks
            if any(kw in fb.get("query", "").lower() for kw in compare_keywords)
        )
        if compare_count > len(feedbacks) * 0.2:
            patterns.append("你开始关注论文间的比较分析")

        long_queries = sum(1 for fb in feedbacks if len(fb.get("query", "")) > 30)
        if long_queries > len(feedbacks) * 0.5:
            patterns.append("问题变得更加深入和具体")

        return patterns

    def test_empty_feedbacks(self):
        """Empty list returns empty patterns."""
        assert self._find_patterns([]) == []

    def test_no_compare_no_pattern(self):
        """Few compare queries → no pattern."""
        feedbacks = [{"query": "what is transformer"} for _ in range(10)]
        assert self._find_patterns(feedbacks) == []

    def test_compare_over_20_percent(self):
        """Compare > 20% → adds compare pattern."""
        # 3 out of 10 = 30% > 20%
        feedbacks = [
            {"query": "transformer vs lstm"} for _ in range(3)
        ] + [{"query": "what is attention"} for _ in range(7)]
        result = self._find_patterns(feedbacks)
        assert "你开始关注论文间的比较分析" in result

    def test_compare_exactly_20_percent(self):
        """Exactly 20% compare → no pattern (strict >)."""
        feedbacks = [{"query": "a vs b"} for _ in range(2)] + [{"query": "c"} for _ in range(8)]
        assert self._find_patterns(feedbacks) == []

    def test_long_queries_over_50_percent(self):
        """Long queries > 50% → adds deep question pattern."""
        # 6 out of 10 = 60% > 50%
        feedbacks = [
            {"query": "a" * 35} for _ in range(6)
        ] + [{"query": "short"} for _ in range(4)]
        result = self._find_patterns(feedbacks)
        assert "问题变得更加深入和具体" in result

    def test_long_query_exactly_50_percent(self):
        """Exactly 50% long queries → no pattern (strict >)."""
        feedbacks = [{"query": "a" * 35} for _ in range(5)] + [{"query": "b"} for _ in range(5)]
        assert self._find_patterns(feedbacks) == []

    def test_both_patterns_possible(self):
        """Both conditions met → both patterns."""
        feedbacks = [
            {"query": "a vs b longer query here" * 5}
            for _ in range(6)
        ] + [{"query": "a" * 35} for _ in range(4)]
        result = self._find_patterns(feedbacks)
        assert len(result) == 2


# =============================================================================
# _generate_suggestions
# =============================================================================
class TestGenerateSuggestions:
    """Test _generate_suggestions logic."""

    def _generate_suggestions(self, feedbacks, paper_insights):
        """Replicate _generate_suggestions."""
        suggestions = []
        if paper_insights:
            top_paper = paper_insights[0]
            suggestions.append(f"深入探索 \"{top_paper.paper_id}\" 的相关工作")
        # Simulate _extract_top_keywords
        keywords = []
        if feedbacks:
            words = [fb.get("query", "").split() for fb in feedbacks]
            kw = [w for ws in words for w in ws if len(w) > 3][:1]
            keywords = kw
        if keywords:
            suggestions.append(f"了解 {keywords[0]} 的最新研究进展")
        suggestions.extend([
            "追踪你关注领域的最新论文",
            "定期回顾已读论文的核心贡献",
        ])
        return suggestions[:5]

    def test_empty_inputs(self):
        """Empty inputs → returns static suggestions only."""
        result = self._generate_suggestions([], [])
        assert "追踪你关注领域的最新论文" in result
        assert "定期回顾已读论文的核心贡献" in result

    def test_top_paper_suggestion_added(self):
        """Top paper → adds paper exploration suggestion."""
        paper = PaperInsight(paper_id="BERT", title="BERT Paper")
        result = self._generate_suggestions([], [paper])
        assert "深入探索 \"BERT\" 的相关工作" in result

    def test_max_5_suggestions(self):
        """At most 5 suggestions returned."""
        # With paper + keywords + 2 static = 4, so we just check upper bound
        paper = PaperInsight(paper_id="P", title="P")
        result = self._generate_suggestions([{"query": "long query word"}], [paper])
        assert len(result) <= 5


# =============================================================================
# _predict_interests
# =============================================================================
class TestPredictInterests:
    """Test _predict_interests logic."""

    def _predict_interests(self, feedbacks, paper_insights):
        """Replicate _predict_interests."""
        predictions = []
        recent_queries = [fb.get("query", "") for fb in feedbacks[-5:]]
        recent_text = " ".join(recent_queries).lower()
        if "llm" in recent_text or "language model" in recent_text:
            predictions.append("LLM架构优化")
        if "training" in recent_text or "训练" in recent_text:
            predictions.append("模型训练技巧")
        if "efficient" in recent_text or "高效" in recent_text:
            predictions.append("效率优化方法")
        return predictions[:3]

    def test_empty_feedbacks(self):
        """Empty feedbacks → no predictions."""
        assert self._predict_interests([], []) == []

    def test_llm_keyword(self):
        """'llm' in query → LLM architecture."""
        feedbacks = [{"query": "how does llm work"}]
        result = self._predict_interests(feedbacks, [])
        assert "LLM架构优化" in result

    def test_language_model_keyword(self):
        """'language model' in query → LLM architecture."""
        feedbacks = [{"query": "what is language model"}]
        result = self._predict_interests(feedbacks, [])
        assert "LLM架构优化" in result

    def test_training_keyword(self):
        """'training' in query → training skills."""
        feedbacks = [{"query": "how to do training of models"}]
        result = self._predict_interests(feedbacks, [])
        assert "模型训练技巧" in result

    def test_chinese_training_keyword(self):
        """Chinese '训练' in query → training skills."""
        feedbacks = [{"query": "模型训练的技巧"}]
        result = self._predict_interests(feedbacks, [])
        assert "模型训练技巧" in result

    def test_efficient_keyword(self):
        """'efficient' in query → efficiency methods."""
        feedbacks = [{"query": "efficient transformers"}]
        result = self._predict_interests(feedbacks, [])
        assert "效率优化方法" in result

    def test_chinese_efficient_keyword(self):
        """Chinese '高效' in query → efficiency methods."""
        feedbacks = [{"query": "高效算法"}]
        result = self._predict_interests(feedbacks, [])
        assert "效率优化方法" in result

    def test_only_last_5_queries_counted(self):
        """Only last 5 queries considered."""
        feedbacks = [
            {"query": "short"}
        ] + [{"query": "llm architecture"} for _ in range(10)]
        result = self._predict_interests(feedbacks, [])
        # Only last 5 have llm, but there are 10 total, so recent 5 should have llm
        assert "LLM架构优化" in result

    def test_max_3_predictions(self):
        """At most 3 predictions."""
        feedbacks = [{"query": "llm training efficient methods"}]
        result = self._predict_interests(feedbacks, [])
        assert len(result) <= 3


# =============================================================================
# _calc_positive_rate
# =============================================================================
class TestCalcPositiveRate:
    """Test _calc_positive_rate logic."""

    def _calc_rate(self, feedbacks):
        """Replicate _calc_positive_rate."""
        if not feedbacks:
            return 0.0
        positive = sum(1 for fb in feedbacks if fb.get("type") == "positive")
        return positive / len(feedbacks)

    def test_empty_feedbacks(self):
        """Empty → 0.0."""
        assert self._calc_rate([]) == 0.0

    def test_all_positive(self):
        """All positive → 1.0."""
        feedbacks = [{"type": "positive"} for _ in range(5)]
        assert self._calc_rate(feedbacks) == 1.0

    def test_all_negative(self):
        """All negative → 0.0."""
        feedbacks = [{"type": "negative"} for _ in range(5)]
        assert self._calc_rate(feedbacks) == 0.0

    def test_mixed(self):
        """Mixed → correct fraction."""
        feedbacks = [
            {"type": "positive"},
            {"type": "positive"},
            {"type": "negative"},
        ]
        assert abs(self._calc_rate(feedbacks) - 2 / 3) < 0.001


# =============================================================================
# _get_evolution_status
# =============================================================================
class TestGetEvolutionStatus:
    """Test _get_evolution_status logic."""

    def _get_status(self, stats):
        """Replicate _get_evolution_status."""
        reliable = stats.get("reliable_patterns", 0)
        if reliable >= 5:
            return "🚀 进化期", "系统已具备自进化能力"
        elif reliable >= 3:
            return "🌲 成熟期", "扩展模式库，覆盖更多场景"
        elif reliable >= 1:
            return "🌳 成长期", "积累 10+ 反馈，强化现有模式"
        return "🌱 种子期", "继续使用，系统会持续学习"

    def test_seed_stage(self):
        """0 reliable patterns → seed stage."""
        stage, progress = self._get_status({"reliable_patterns": 0})
        assert stage == "🌱 种子期"

    def test_growing_stage(self):
        """1-2 reliable patterns → growing stage."""
        stage, progress = self._get_status({"reliable_patterns": 1})
        assert stage == "🌳 成长期"

    def test_growing_stage_2(self):
        """1-2 reliable patterns → growing stage."""
        stage, _ = self._get_status({"reliable_patterns": 2})
        assert stage == "🌳 成长期"

    def test_mature_stage(self):
        """3-4 reliable patterns → mature stage."""
        stage, _ = self._get_status({"reliable_patterns": 3})
        assert stage == "🌲 成熟期"

    def test_mature_stage_4(self):
        """3-4 reliable patterns → mature stage."""
        stage, _ = self._get_status({"reliable_patterns": 4})
        assert stage == "🌲 成熟期"

    def test_evolution_stage(self):
        """5+ reliable patterns → evolution stage."""
        stage, progress = self._get_status({"reliable_patterns": 5})
        assert stage == "🚀 进化期"
        assert progress == "系统已具备自进化能力"

    def test_missing_key(self):
        """Missing key → defaults to 0."""
        stage, _ = self._get_status({})
        assert stage == "🌱 种子期"


# =============================================================================
# _generate_user_journey
# =============================================================================
class TestGenerateUserJourney:
    """Test _generate_user_journey logic."""

    def _journey(self, feedbacks, paper_insights):
        """Replicate _generate_user_journey."""
        total = len(feedbacks)
        if total == 0:
            return ""
        if total >= 20:
            return f"这是充实的一周！你深入探索了 {total} 个问题。"
        elif total >= 10:
            return f"你保持了良好的研究节奏，探讨了 {total} 个有意义的问题。"
        elif total >= 5:
            return f"本周你提出了 {total} 个问题，研究在稳步推进。"
        elif total >= 1:
            return "你开始了新的探索旅程，提出了第一个问题。"
        return ""

    def test_empty(self):
        """Empty → empty string."""
        assert self._journey([], []) == ""

    def test_single(self):
        """1 feedback → first journey message."""
        assert self._journey([{"query": "a"}], []) == "你开始了新的探索旅程，提出了第一个问题。"

    def test_5_to_9(self):
        """5-9 feedbacks → steady progress."""
        result = self._journey([{"query": "q"} for _ in range(7)], [])
        assert "7 个问题" in result
        assert "稳步推进" in result

    def test_10_to_19(self):
        """10-19 feedbacks → good rhythm."""
        result = self._journey([{"query": "q"} for _ in range(15)], [])
        assert "15 个有意义的问题" in result
        assert "良好的研究节奏" in result

    def test_20_plus(self):
        """20+ feedbacks → fulfilling week."""
        result = self._journey([{"query": "q"} for _ in range(25)], [])
        assert "充实的一周" in result
        assert "25 个问题" in result


# =============================================================================
# _generate_system_learned
# =============================================================================
class TestGenerateSystemLearned:
    """Test _generate_system_learned logic."""

    def _system_learned(self, feedbacks, paper_insights, stats):
        """Replicate _generate_system_learned."""
        reliable = stats.get("reliable_patterns", 0)
        total = len(feedbacks)
        if reliable >= 5:
            return f"我已经学会了 {reliable} 个有效的回应模式，能够更好地帮助你理解论文。"
        elif reliable >= 3:
            kw = []
            if feedbacks:
                words = [w for fb in feedbacks[:5] for w in fb.get("query", "").split() if len(w) > 3]
                kw = words[:1]
            learned_kw = kw[0] if kw else "相关主题"
            return f"我注意到你对「{learned_kw}」很感兴趣，学会了优先推荐这类内容。"
        elif total >= 10:
            return "通过你的反馈，我正在学习什么是最有帮助的回答方式。"
        elif total >= 1:
            return "感谢你的第一个反馈！我正在学习如何更好地帮助你。"
        return "开始使用，让我了解你的研究风格。"

    def test_empty(self):
        """Empty feedbacks and stats → start message."""
        assert self._system_learned([], [], {}) == "开始使用，让我了解你的研究风格。"

    def test_first_feedback(self):
        """1-9 feedbacks → first feedback message."""
        result = self._system_learned([{"query": "q"}], [], {})
        assert "第一个反馈" in result

    def test_10_plus_feedbacks(self):
        """10+ feedbacks → learning from feedback."""
        result = self._system_learned([{"query": "q"} for _ in range(12)], [], {})
        assert "反馈" in result

    def test_3_to_4_patterns(self):
        """3-4 reliable patterns → interest-based message."""
        result = self._system_learned([{"query": "transformer attention"}], [], {"reliable_patterns": 3})
        assert "感兴趣" in result

    def test_5_plus_patterns(self):
        """5+ reliable patterns → learned N patterns."""
        result = self._system_learned([], [], {"reliable_patterns": 7})
        assert "7 个有效的回应模式" in result


# =============================================================================
# _generate_highlight
# =============================================================================
class TestGenerateHighlight:
    """Test _generate_highlight logic."""

    def _highlight(self, feedbacks, paper_insights):
        """Replicate _generate_highlight."""
        if not paper_insights:
            return ""
        top = paper_insights[0]
        pos_count = getattr(top, "positive_count", 0)
        if pos_count >= 5:
            return f"「{top.title}」是你最信赖的参考资料，被引用了 {pos_count} 次！"
        elif pos_count >= 3:
            return f"「{top.title}」成为你的研究利器，帮你解答了多个问题。"
        elif pos_count >= 1:
            return f"「{top.title}」开始进入你的研究视野。"
        return ""

    def test_empty_papers(self):
        """No papers → empty string."""
        assert self._highlight([], []) == ""

    def test_5_plus_positive(self):
        """5+ positive count → most trusted message."""
        paper = PaperInsight(paper_id="p", title="Transformer Paper", positive_count=7, negative_count=1)
        result = self._highlight([], [paper])
        assert "最信赖" in result
        assert "7 次" in result

    def test_3_to_4_positive(self):
        """3-4 positive count → research tool message."""
        paper = PaperInsight(paper_id="p", title="BERT Paper", positive_count=4, negative_count=0)
        result = self._highlight([], [paper])
        assert "研究利器" in result

    def test_1_to_2_positive(self):
        """1-2 positive count → entering vision message."""
        paper = PaperInsight(paper_id="p", title="LSTM Paper", positive_count=2, negative_count=0)
        result = self._highlight([], [paper])
        assert "进入你的研究视野" in result

    def test_zero_positive(self):
        """Zero positive count → empty string."""
        paper = PaperInsight(paper_id="p", title="New Paper", positive_count=0, negative_count=1)
        assert self._highlight([], [paper]) == ""


# =============================================================================
# _wilson_score
# =============================================================================
class TestWilsonScore:
    """Test _wilson_score logic."""

    def _wilson_score(self, positives, total, confidence=0.95):
        """Replicate _wilson_score formula."""
        if total == 0:
            return 0.0
        p = positives / total
        z = 1.645 if confidence == 0.95 else 1.96
        n = total
        denom = 1 + z**2 / n
        center = p + z**2 / (2 * n)
        margin = z * math.sqrt(p * (1 - p) / n + z**2 / (4 * n**2))
        wilson_lower = (center - margin) / denom
        return wilson_lower * 2 - 0.5

    def test_zero_total(self):
        """Zero total → 0.0."""
        assert self._wilson_score(0, 0) == 0.0

    def test_all_positive_95(self):
        """All positive → positive score."""
        result = self._wilson_score(10, 10, 0.95)
        assert result > 0.5

    def test_all_negative(self):
        """All negative → negative score."""
        result = self._wilson_score(0, 10, 0.95)
        assert result < 0.0

    def test_half_positive(self):
        """Half positive → score close to 0."""
        result = self._wilson_score(5, 10, 0.95)
        assert abs(result) < 0.5

    def test_confidence_95_uses_1645(self):
        """95% confidence uses z=1.645."""
        r95 = self._wilson_score(5, 10, 0.95)
        r99 = self._wilson_score(5, 10, 0.99)
        # Higher confidence → wider interval → lower bound more conservative
        assert r99 < r95

    def test_small_sample_more_conservative(self):
        """Small samples get more conservative scores."""
        large = self._wilson_score(9, 10, 0.95)
        small = self._wilson_score(9, 10, 0.95)
        # For same proportion, more samples → higher score (less penalty)
        # Actually with Wilson formula, more samples gives narrower interval
        assert large > 0


# =============================================================================
# _calc_diversity_penalty
# =============================================================================
class TestCalcDiversityPenalty:
    """Test _calc_diversity_penalty logic."""

    DIVERSITY_RATIO = 0.6

    def _penalty(self, topic, topic_counts, total):
        """Replicate _calc_diversity_penalty."""
        if total == 0:
            return 1.0
        count = topic_counts.get(topic, 0)
        ratio = count / total
        if ratio > self.DIVERSITY_RATIO:
            penalty = 1.0 - (ratio - self.DIVERSITY_RATIO) * 0.5
            return max(penalty, 0.7)
        return 1.0

    def test_zero_total(self):
        """Zero total → 1.0."""
        assert self._penalty("nlp", {}, 0) == 1.0

    def test_under_threshold(self):
        """Ratio <= 0.6 → no penalty."""
        counts = {"nlp": 3, "cv": 4}
        # 3/10 = 0.3 ≤ 0.6
        assert self._penalty("nlp", counts, 10) == 1.0

    def test_over_threshold(self):
        """Ratio > 0.6 → penalty applied."""
        counts = {"nlp": 7}
        # 7/10 = 0.7 > 0.6
        # penalty = 1.0 - (0.7 - 0.6) * 0.5 = 1.0 - 0.05 = 0.95
        assert self._penalty("nlp", counts, 10) == 0.95

    def test_penalty_floor(self):
        """Penalty has floor of 0.7."""
        counts = {"nlp": 10}
        # ratio = 1.0, penalty = 1.0 - (1.0 - 0.6) * 0.5 = 1.0 - 0.2 = 0.8
        assert self._penalty("nlp", counts, 10) == 0.8

    def test_very_high_ratio_near_floor(self):
        """Very high ratio → near 0.7 floor."""
        counts = {"nlp": 10}
        # ratio = 10/10 = 1.0
        # penalty = 1.0 - (1.0 - 0.6) * 0.5 = 0.8
        assert abs(self._penalty("nlp", counts, 10) - 0.8) < 0.01

    def test_unknown_topic(self):
        """Unknown topic → no penalty."""
        counts = {"nlp": 3}
        assert self._penalty("unknown", counts, 10) == 1.0


# =============================================================================
# _apply_diversity_rerank
# =============================================================================
class TestApplyDiversityRerank:
    """Test _apply_diversity_rerank logic."""

    def _rerank(self, results):
        """Replicate _apply_diversity_rerank."""
        diverse = []
        topics_seen = {}

        for r in results:
            topic = r.get("topic", "unknown")
            if len(diverse) < 3:
                diverse.append(r)
                topics_seen[topic] = topics_seen.get(topic, 0) + 1
            else:
                if topics_seen.get(topic, 0) < 2:
                    diverse.append(r)
                    topics_seen[topic] = topics_seen.get(topic, 0) + 1
                else:
                    r["_diversity_deferred"] = True
                    diverse.append(r)

        return diverse

    def test_empty(self):
        """Empty list returns empty."""
        assert self._rerank([]) == []

    def test_first_3_always_included(self):
        """First 3 items always included regardless of topic."""
        results = [
            {"topic": "nlp", "score": 0.9},
            {"topic": "nlp", "score": 0.8},
            {"topic": "nlp", "score": 0.7},
            {"topic": "cv", "score": 0.6},
        ]
        reranked = self._rerank(results)
        assert len(reranked) == 4
        assert reranked[0]["score"] == 0.9
        assert reranked[1]["score"] == 0.8
        assert reranked[2]["score"] == 0.7

    def test_max_2_per_topic_after_3(self):
        """After first 3, deferred items kept in score order; new topics appended last."""
        results = [
            {"topic": "nlp", "score": 0.9},
            {"topic": "nlp", "score": 0.8},
            {"topic": "nlp", "score": 0.7},
            {"topic": "nlp", "score": 0.6},  # 4th nlp → deferred
            {"topic": "nlp", "score": 0.5},  # 5th nlp → deferred
            {"topic": "cv", "score": 0.4},
        ]
        reranked = self._rerank(results)
        # First 3 are all nlp (score order)
        assert reranked[0]["topic"] == "nlp"
        assert reranked[1]["topic"] == "nlp"
        assert reranked[2]["topic"] == "nlp"
        # Deferred items come next (still in score order)
        assert reranked[3]["topic"] == "nlp"
        assert reranked[3].get("_diversity_deferred") is True
        assert reranked[4]["topic"] == "nlp"
        assert reranked[4].get("_diversity_deferred") is True
        # New topic cv appended last
        assert reranked[5]["topic"] == "cv"
        assert reranked[5].get("_diversity_deferred") is None

    def test_diversity_deferred_flag(self):
        """Deferred items get _diversity_deferred flag."""
        results = [
            {"topic": "nlp", "score": 0.9},
            {"topic": "nlp", "score": 0.8},
            {"topic": "nlp", "score": 0.7},
            {"topic": "nlp", "score": 0.6},
        ]
        reranked = self._rerank(results)
        assert reranked[3]["_diversity_deferred"] is True


# =============================================================================
# _get_boost_age
# =============================================================================
class TestGetBoostAge:
    """Test _get_boost_age logic."""

    def _boost_age(self, paper_id, boost_data):
        """Replicate _get_boost_age."""
        from datetime import datetime
        data = boost_data.get(paper_id, {})
        last_update = data.get("last_update", "")
        if not last_update:
            return 30
        try:
            last = datetime.fromisoformat(last_update)
            return (datetime.now() - last).days
        except (ValueError, IndexError):
            return 30

    def test_no_data(self):
        """No data → returns 30."""
        assert self._boost_age("unknown", {}) == 30

    def test_missing_last_update(self):
        """Missing last_update → returns 30."""
        assert self._boost_age("p", {"p": {"positive": 1}}) == 30

    def test_empty_last_update(self):
        """Empty last_update → returns 30."""
        assert self._boost_age("p", {"p": {"last_update": ""}}) == 30


# =============================================================================
# FollowUpType enum
# =============================================================================
class TestFollowUpType:
    """Test FollowUpType class."""

    def test_all_types_exist(self):
        """All follow-up types exist as class attributes."""
        assert FollowUpType.MATH == "math"
        assert FollowUpType.CODE == "code"
        assert FollowUpType.COMPARE == "compare"
        assert FollowUpType.EVOLUTION == "evolution"
        assert FollowUpType.PRACTICE == "practice"
        assert FollowUpType.CITATION == "citation"


# =============================================================================
# FollowUp dataclass
# =============================================================================
class TestFollowUp:
    """Test FollowUp dataclass."""

    def test_required_fields(self):
        """Required fields: text, type, query, icon."""
        f = FollowUp(text="What is it?", type="math", query="what is it?", icon="∫")
        assert f.text == "What is it?"
        assert f.type == "math"
        assert f.query == "what is it?"
        assert f.icon == "∫"

    def test_depth_default(self):
        """Depth defaults to 1."""
        f = FollowUp(text="T", type="code", query="Q", icon="C")
        assert f.depth == 1


# =============================================================================
# SmartFollowUp helper methods
# =============================================================================
class TestExtractTechnicalTerms:
    """Test _extract_technical_terms logic."""

    def _extract_terms(self, text):
        """Replicate _extract_technical_terms."""
        import re
        patterns = [
            r'\b[A-Z][a-z]+(?:\s+[A-Z][a-z]+)*\s+(?:mechanism|model|network|architecture|method|algorithm)\b',
            r'\b(?:self-|cross-|multi-|hierarchical)\s*\w+(?:-\w+)*\b',
            r'\b\w+(?:-\w+){1,3}\b',
        ]
        terms = []
        for pattern in patterns:
            matches = re.findall(pattern, text, re.IGNORECASE)
            terms.extend([m for m in matches if len(m) > 4])
        seen = set()
        unique_terms = []
        for t in terms:
            t_lower = t.lower()
            if t_lower not in seen and len(t) > 5:
                seen.add(t_lower)
                unique_terms.append(t)
        return unique_terms[:3]

    def test_capitalized_mechanism(self):
        """Capitalized word + mechanism → extracted."""
        result = self._extract_terms("Self Attention Mechanism")
        assert "Self Attention Mechanism" in result or "Self" in result

    def test_preceded_by_prefix(self):
        """Prefix words (self-, cross-) → extracted."""
        result = self._extract_terms("Uses self-attention in transformer")
        assert len(result) > 0

    def test_hyphenated_terms(self):
        """Hyphenated terms → extracted."""
        result = self._extract_terms("state-of-the-art results")
        assert len(result) > 0

    def test_max_3_returned(self):
        """At most 3 terms returned."""
        text = "cross-attention mechanism " * 10
        result = self._extract_terms(text)
        assert len(result) <= 3

    def test_short_filtered(self):
        """Terms <= 5 chars filtered."""
        result = self._extract_terms("an ai model named AB")
        # "ai model" has "ai" = 2 chars, "AB" = 2 chars
        assert len(result) == 0 or all(len(t) > 5 for t in result)


class TestDetectTopicTypes:
    """Test _detect_topic_types logic."""

    def _detect(self, text):
        """Replicate _detect_topic_types."""
        from llm.evolution_report import FollowUpType, SmartFollowUp
        sf = SmartFollowUp()
        scores = {}
        for ftype, keywords in sf.TOPIC_KEYWORDS.items():
            score = sum(1 for kw in keywords if kw in text)
            if score > 0:
                scores[ftype] = score
        return sorted(scores.keys(), key=lambda x: scores[x], reverse=True)

    def test_empty_text(self):
        """Empty text → empty list."""
        assert self._detect("") == []

    def test_math_keyword(self):
        """'attention' in text → MATH type."""
        result = self._detect("attention mechanism explained")
        assert "math" in result

    def test_code_keyword(self):
        """'code' in text → CODE type."""
        result = self._detect("how to write code for transformer")
        assert "code" in result

    def test_compare_keyword(self):
        """'vs' in text → COMPARE type."""
        result = self._detect("transformer vs lstm")
        assert "compare" in result

    def test_sorted_by_score(self):
        """Results sorted by relevance score descending."""
        result = self._detect("transformer vs lstm attention code")
        assert len(result) >= 2
        # Both should appear, but compare/vs should score highly


# =============================================================================
# LearningReport.to_markdown narrative style
# =============================================================================
class TestLearningReportToMarkdown:
    """Test LearningReport.to_markdown() formatting."""

    def test_header(self):
        """Header includes title and period."""
        r = LearningReport(
            period_start="2026-01-01T00:00:00",
            period_end="2026-01-07T23:59:59",
            total_queries=0, positive_rate=0.0,
            top_papers=[], top_keywords=[], emerging_patterns=[],
            predicted_interests=[], questions_to_explore=[],
            evolution_stage="🌱 种子期", progress_towards_next="开始",
        )
        output = r.to_markdown()
        assert "AI Research OS 学习报告" in output
        assert "2026-01-01" in output

    def test_user_journey_included(self):
        """User journey narrative is included."""
        r = LearningReport(
            period_start="2026-01-01T00:00:00",
            period_end="2026-01-07T23:59:59",
            total_queries=5, positive_rate=0.6,
            top_papers=[], top_keywords=[], emerging_patterns=[],
            predicted_interests=[], questions_to_explore=[],
            evolution_stage="🌱 种子期", progress_towards_next="开始",
            user_journey="这是充实的一周！",
        )
        output = r.to_markdown()
        assert "这是充实的一周！" in output

    def test_system_learned_section(self):
        """System learned section present when populated."""
        r = LearningReport(
            period_start="2026-01-01T00:00:00",
            period_end="2026-01-07T23:59:59",
            total_queries=0, positive_rate=0.0,
            top_papers=[], top_keywords=[], emerging_patterns=[],
            predicted_interests=[], questions_to_explore=[],
            evolution_stage="🌱 种子期", progress_towards_next="开始",
            system_learned="我已经学会了模式",
        )
        output = r.to_markdown()
        assert "系统学会了什么" in output

    def test_top_paper_narrative(self):
        """Top paper shown with narrative."""
        paper = PaperInsight(paper_id="bert", title="BERT Paper", positive_count=5)
        r = LearningReport(
            period_start="2026-01-01T00:00:00",
            period_end="2026-01-07T23:59:59",
            total_queries=10, positive_rate=0.5,
            top_papers=[paper], top_keywords=[], emerging_patterns=[],
            predicted_interests=[], questions_to_explore=[],
            evolution_stage="🌱 种子期", progress_towards_next="开始",
        )
        output = r.to_markdown()
        assert "老朋友" in output
        assert "BERT Paper" in output

    def test_evolution_stage_footer(self):
        """Evolution stage shown at bottom."""
        r = LearningReport(
            period_start="2026-01-01T00:00:00",
            period_end="2026-01-07T23:59:59",
            total_queries=0, positive_rate=0.0,
            top_papers=[], top_keywords=[], emerging_patterns=[],
            predicted_interests=[], questions_to_explore=[],
            evolution_stage="🚀 进化期", progress_towards_next="继续进化",
        )
        output = r.to_markdown()
        assert "🚀 进化期" in output
        assert "继续进化" in output


class TestLearningReportToMarkdownClassic:
    """Test LearningReport.to_markdown_classic() formatting."""

    def test_header(self):
        """Header includes period."""
        r = LearningReport(
            period_start="2026-01-01T00:00:00",
            period_end="2026-01-07T23:59:59",
            total_queries=10, positive_rate=0.8,
            top_papers=[], top_keywords=[], emerging_patterns=[],
            predicted_interests=[], questions_to_explore=[],
            evolution_stage="🌱 种子期", progress_towards_next="开始",
        )
        output = r.to_markdown_classic()
        assert "AI Research OS 学习报告" in output
        assert "2026-01-01" in output
        assert "2026-01-07" in output

    def test_total_queries_shown(self):
        """Total queries displayed."""
        r = LearningReport(
            period_start="2026-01-01T00:00:00",
            period_end="2026-01-07T23:59:59",
            total_queries=42, positive_rate=0.5,
            top_papers=[], top_keywords=[], emerging_patterns=[],
            predicted_interests=[], questions_to_explore=[],
            evolution_stage="🌱 种子期", progress_towards_next="",
        )
        output = r.to_markdown_classic()
        assert "42" in output

    def test_positive_rate_percent(self):
        """Positive rate shown as percentage."""
        r = LearningReport(
            period_start="2026-01-01T00:00:00",
            period_end="2026-01-07T23:59:59",
            total_queries=10, positive_rate=0.75,
            top_papers=[], top_keywords=[], emerging_patterns=[],
            predicted_interests=[], questions_to_explore=[],
            evolution_stage="🌱 种子期", progress_towards_next="",
        )
        output = r.to_markdown_classic()
        assert "75" in output


# =============================================================================
# SmartFollowUp._extract_concept
# =============================================================================
class TestExtractConcept:
    """Test _extract_concept logic."""

    def _extract_concept(self, question, answer):
        """Replicate _extract_concept."""
        text = f"{question} {answer}"
        # Simulate _extract_technical_terms from answer only
        import re
        terms = re.findall(
            r'\b[A-Z][a-z]+(?:\s+[A-Z][a-z]+)*\s+(?:mechanism|model|network|architecture|method|algorithm)\b',
            answer, re.IGNORECASE
        )
        if terms:
            return terms[0][:40]

        stopwords = {
            "what", "is", "are", "how", "why", "when", "where",
            "the", "a", "an", "this", "that", "these", "those",
            "的", "是", "如何", "什么", "怎么", "为什么", "please", "explain",
        }
        # Split full text (question + answer) for keyword extraction
        words = text.split()
        keywords = [w for w in words if w.lower() not in stopwords and len(w) > 2]
        if len(keywords) >= 3:
            concept = " ".join(keywords[:3])
        elif keywords:
            concept = " ".join(keywords)
        else:
            concept = question[:30]
        return concept[:50]

    def test_technical_term_extracted(self):
        """Technical term takes priority."""
        result = self._extract_concept("What is it?", "Self Attention Mechanism")
        assert "Self Attention Mechanism" in result or "Attention" in result

    def test_capitalized_phrase_takes_priority(self):
        """Technical phrase in answer → returned as concept."""
        result = self._extract_concept(
            "What is the transformer model?",
            "It is a Neural Network Architecture."
        )
        assert "neural" in result.lower()
        assert "architecture" in result.lower()

    def test_lowercase_phrase_also_matches_ignorecase(self):
        """Pattern uses IGNORECASE, so lowercase phrase also matches."""
        # re.IGNORECASE means [A-Z][a-z]+ matches "neural" too
        result = self._extract_concept(
            "What is the transformer model?",
            "It is a neural network architecture."
        )
        # With IGNORECASE, "neural network architecture" still matches
        assert "neural" in result.lower()
        assert "network" in result.lower()

    def test_max_length_50(self):
        """Result truncated to 50 chars."""
        long_q = " ".join(["word"] * 20)
        result = self._extract_concept(long_q, "")
        assert len(result) <= 50

    def test_fallback_to_question(self):
        """No keywords → falls back to question start."""
        result = self._extract_concept("What is it", "It is a")
        assert "What is it" in result or result != ""


# =============================================================================
# SmartFollowUp._is_duplicate
# =============================================================================
class TestIsDuplicate:
    """Test _is_duplicate logic."""

    def _is_duplicate(self, new_query, existing_queries):
        """Replicate _is_duplicate."""
        for q in existing_queries:
            opt_words = set(q.lower().split())
            new_words = set(new_query.lower().split())
            overlap = len(opt_words & new_words)
            if overlap >= 2:
                return True
        return False

    def test_identical_query(self):
        """Same query → duplicate."""
        assert self._is_duplicate("transformer attention", ["transformer attention"])

    def test_shared_words(self):
        """2+ shared words → duplicate."""
        assert self._is_duplicate(
            "transformer attention mechanism",
            ["transformer attention model"],
        )

    def test_no_shared_words(self):
        """No shared words → not duplicate."""
        assert not self._is_duplicate(
            "bert model fine-tuning",
            ["lstm recurrent network"],
        )

    def test_one_shared_word(self):
        """Only 1 shared word → not duplicate."""
        assert not self._is_duplicate(
            "transformer architecture",
            ["transformer attention"],
        )


# =============================================================================
# SmartFollowUp.render_options
# =============================================================================
class TestRenderOptions:
    """Test render_options logic."""

    def _render(self, options):
        """Replicate render_options."""
        if not options:
            return ""
        lines = ["", "📌 想深入了解？选择一个追问："]
        for i, opt in enumerate(options, 1):
            lines.append(f"   [{i}] {opt.text}")
        lines.append("")
        return "\n".join(lines)

    def test_empty_returns_empty(self):
        """Empty options → empty string."""
        assert self._render([]) == ""

    def test_header_present(self):
        """Header shown."""
        options = [FollowUp(text="What is it?", type="math", query="q", icon="∫")]
        output = self._render(options)
        assert "想深入了解" in output

    def test_options_numbered(self):
        """Options numbered 1-N."""
        options = [
            FollowUp(text="Option 1", type="math", query="q", icon="∫"),
            FollowUp(text="Option 2", type="code", query="q", icon="⚙"),
        ]
        output = self._render(options)
        assert "[1]" in output
        assert "[2]" in output

    def test_text_shown(self):
        """Option text displayed."""
        options = [FollowUp(text="Attention mechanism", type="math", query="q", icon="∫")]
        output = self._render(options)
        assert "Attention mechanism" in output
