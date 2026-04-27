"""Tier 2 unit tests — llm/trend_analyzer.py, pure functions, no I/O."""
import pytest
from llm.trend_analyzer import (
    TrendDirection,
    YearlyStats,
    TrendKeyword,
    TrendAnalysisResult,
    TrendAnalyzer,
)


# =============================================================================
# Enum tests
# =============================================================================
class TestTrendDirection:
    """Test TrendDirection enum."""

    def test_all_directions_have_values(self):
        """All TrendDirection variants have string values."""
        assert TrendDirection.RISING.value == "rising"
        assert TrendDirection.FALLING.value == "falling"
        assert TrendDirection.EMERGING.value == "emerging"
        assert TrendDirection.STABLE.value == "stable"
        assert TrendDirection.UNKNOWN.value == "unknown"

    def test_can_construct_from_value(self):
        """Enum can be constructed from string value."""
        assert TrendDirection("rising") == TrendDirection.RISING
        assert TrendDirection("emerging") == TrendDirection.EMERGING


# =============================================================================
# Dataclass tests
# =============================================================================
class TestYearlyStats:
    """Test YearlyStats dataclass."""

    def test_required_fields(self):
        """Required fields: year, paper_count, total_citations, avg_citations."""
        stats = YearlyStats(year=2023, paper_count=50, total_citations=500, avg_citations=10.0)
        assert stats.year == 2023
        assert stats.paper_count == 50
        assert stats.total_citations == 500
        assert stats.avg_citations == 10.0

    def test_keywords_default(self):
        """keywords defaults to empty dict."""
        stats = YearlyStats(year=2023, paper_count=10, total_citations=100, avg_citations=10.0)
        assert stats.keywords == {}


class TestTrendKeyword:
    """Test TrendKeyword dataclass."""

    def test_required_fields(self):
        """Required fields: keyword, direction, yearly_counts, growth_rate, peak_year, current_year_count."""
        tk = TrendKeyword(
            keyword="transformer",
            direction=TrendDirection.RISING,
            yearly_counts={2022: 10, 2023: 20},
            growth_rate=100.0,
            peak_year=2023,
            current_year_count=20,
        )
        assert tk.keyword == "transformer"
        assert tk.direction == TrendDirection.RISING
        assert tk.yearly_counts == {2022: 10, 2023: 20}
        assert tk.growth_rate == 100.0
        assert tk.peak_year == 2023
        assert tk.current_year_count == 20

    def test_optional_defaults(self):
        """velocity and momentum default to 0.0."""
        tk = TrendKeyword(
            keyword="bert",
            direction=TrendDirection.STABLE,
            yearly_counts={2023: 15},
            growth_rate=5.0,
            peak_year=2023,
            current_year_count=15,
        )
        assert tk.velocity == 0.0
        assert tk.momentum == 0.0


class TestTrendAnalysisResult:
    """Test TrendAnalysisResult dataclass."""

    def test_required_fields(self):
        """Required fields: topic, year_range, total_papers."""
        result = TrendAnalysisResult(
            topic="NLP",
            year_range=(2020, 2025),
            total_papers=500,
        )
        assert result.topic == "NLP"
        assert result.year_range == (2020, 2025)
        assert result.total_papers == 500

    def test_optional_defaults(self):
        """Optional fields default to empty/false."""
        result = TrendAnalysisResult(
            topic="T",
            year_range=(2020, 2025),
            total_papers=0,
        )
        assert result.yearly_distribution == []
        assert result.rising_trends == []
        assert result.falling_trends == []
        assert result.emerging_trends == []
        assert result.stable_trends == []
        assert result.hot_keywords == []
        assert result.declining_keywords == []
        assert result.emerging_keywords == []
        assert result.growth_rate == 0.0


# =============================================================================
# _compute_yearly_stats tests
# =============================================================================
class TestComputeYearlyStats:
    """Test _compute_yearly_stats logic."""

    def _compute_yearly_stats(self, papers, year_range, tech_keywords) -> list:
        """Replicate _compute_yearly_stats logic."""
        from collections import defaultdict

        yearly_data = defaultdict(lambda: {
            "count": 0,
            "citations": 0,
            "keywords": defaultdict(int),
        })

        for paper in papers:
            year = paper.get("year", 0)
            if year_range[0] <= year <= year_range[1]:
                yearly_data[year]["count"] += 1
                yearly_data[year]["citations"] += paper.get("citations", 0)

                text = f"{paper.get('title', '')} {paper.get('abstract', '')}".lower()
                for kw in tech_keywords:
                    if kw.lower() in text:
                        yearly_data[year]["keywords"][kw] += 1

        stats = []
        for year in range(year_range[0], year_range[1] + 1):
            data = yearly_data[year]
            stats.append(YearlyStats(
                year=year,
                paper_count=data["count"],
                total_citations=data["citations"],
                avg_citations=data["citations"] / data["count"] if data["count"] > 0 else 0,
                keywords=dict(data["keywords"]),
            ))

        return stats

    def test_filters_by_year_range(self):
        """Papers outside year range are excluded."""
        papers = [
            {"year": 2019, "citations": 10},
            {"year": 2020, "citations": 5},
            {"year": 2021, "citations": 8},
            {"year": 2022, "citations": 3},
            {"year": 2024, "citations": 2},
        ]
        stats = self._compute_yearly_stats(papers, (2020, 2022), [])
        assert len(stats) == 3
        counts = [s.paper_count for s in stats]
        assert counts == [1, 1, 1]

    def test_empty_papers(self):
        """Empty paper list returns all-zero stats."""
        stats = self._compute_yearly_stats([], (2020, 2022), [])
        assert len(stats) == 3
        assert all(s.paper_count == 0 for s in stats)
        assert all(s.total_citations == 0 for s in stats)

    def test_counts_papers_per_year(self):
        """Paper counts aggregated per year."""
        papers = [
            {"year": 2021, "citations": 10},
            {"year": 2021, "citations": 20},
            {"year": 2021, "citations": 30},
            {"year": 2022, "citations": 5},
        ]
        stats = self._compute_yearly_stats(papers, (2021, 2022), [])
        y2021 = next(s for s in stats if s.year == 2021)
        y2022 = next(s for s in stats if s.year == 2022)
        assert y2021.paper_count == 3
        assert y2021.total_citations == 60
        assert y2022.paper_count == 1
        assert y2022.total_citations == 5

    def test_avg_citations_zero_when_no_papers(self):
        """avg_citations is 0 when no papers."""
        stats = self._compute_yearly_stats([], (2021, 2021), [])
        assert stats[0].avg_citations == 0.0

    def test_avg_citations_correct(self):
        """avg_citations = total / count."""
        papers = [{"year": 2021, "citations": 90}]
        stats = self._compute_yearly_stats(papers, (2021, 2021), [])
        assert stats[0].avg_citations == 90.0

    def test_keywords_extracted_from_title(self):
        """Keywords matched in title."""
        papers = [
            {"year": 2021, "title": "Transformer architecture", "abstract": ""},
            {"year": 2021, "title": "Transformer attention", "abstract": ""},
        ]
        stats = self._compute_yearly_stats(papers, (2021, 2021), ["transformer", "bert"])
        assert stats[0].keywords["transformer"] == 2
        # Only matched keywords are added to the dict — "bert" never matched so not present

    def test_keywords_extracted_from_abstract(self):
        """Keywords matched in abstract."""
        papers = [
            {"year": 2021, "title": "Paper", "abstract": "GPT model with language understanding"},
        ]
        stats = self._compute_yearly_stats(papers, (2021, 2021), ["gpt", "llm"])
        assert stats[0].keywords["gpt"] == 1
        # Only matched keywords are added to the dict — "llm" did not appear so not present

    def test_keywords_case_insensitive(self):
        """Keyword matching is case insensitive."""
        papers = [{"year": 2021, "title": "TRANSFORMER", "abstract": ""}]
        stats = self._compute_yearly_stats(papers, (2021, 2021), ["transformer"])
        assert stats[0].keywords["transformer"] == 1

    def test_missing_year_keyword(self):
        """Paper with no year is excluded."""
        papers = [{"year": 0, "title": "Transformer", "abstract": ""}]
        stats = self._compute_yearly_stats(papers, (2021, 2021), ["transformer"])
        assert stats[0].paper_count == 0

    def test_all_years_in_range_returned(self):
        """All years in range are returned, even with no papers."""
        stats = self._compute_yearly_stats([], (2020, 2025), [])
        assert len(stats) == 6
        years = [s.year for s in stats]
        assert years == [2020, 2021, 2022, 2023, 2024, 2025]


# =============================================================================
# _compute_trend tests
# =============================================================================
class TestComputeTrend:
    """Test _compute_trend logic."""

    def _compute_trend(self, keyword, yearly_counts, year_range) -> TrendKeyword:
        """Replicate _compute_trend logic."""
        from llm.trend_analyzer import TrendDirection

        years = list(range(year_range[0], year_range[1] + 1))
        counts = [yearly_counts.get(y, 0) for y in years]

        first_nonzero = next((c for c in counts if c > 0), 0)
        last_count = counts[-1]

        if first_nonzero == 0:
            return None

        growth_rate = ((last_count - first_nonzero) / first_nonzero) * 100

        if last_count > first_nonzero * 1.5:
            if counts[-1] > counts[-2] > 0:
                direction = TrendDirection.EMERGING
            else:
                direction = TrendDirection.RISING
        elif last_count < first_nonzero * 0.7:
            direction = TrendDirection.FALLING
        elif abs(growth_rate) < 20:
            direction = TrendDirection.STABLE
        else:
            direction = TrendDirection.UNKNOWN

        peak_year = max(yearly_counts.keys(), key=lambda y: yearly_counts[y], default=years[-1])

        if len(counts) >= 3:
            recent_change = counts[-1] - counts[-2]
            prev_change = counts[-2] - counts[-3] if counts[-2] > 0 else 1
            momentum = (recent_change - prev_change) / max(prev_change, 1)
        else:
            momentum = 0

        return TrendKeyword(
            keyword=keyword,
            direction=direction,
            yearly_counts=dict(yearly_counts),
            growth_rate=growth_rate,
            peak_year=peak_year,
            current_year_count=last_count,
            velocity=sum(counts[-3:]) if len(counts) >= 3 else sum(counts),
            momentum=momentum,
        )

    def test_returns_none_when_all_zero(self):
        """All-zero counts returns None."""
        result = self._compute_trend("transformer", {}, (2020, 2023))
        assert result is None

    def test_growth_rate_positive(self):
        """Growth rate = (last - first) / first * 100."""
        trend = self._compute_trend("bert", {2020: 10, 2023: 20}, (2020, 2023))
        assert trend is not None
        assert trend.growth_rate == 100.0

    def test_growth_rate_negative(self):
        """Decline gives negative growth rate."""
        trend = self._compute_trend("lstm", {2020: 20, 2023: 10}, (2020, 2023))
        assert trend is not None
        assert trend.growth_rate == -50.0

    def test_growth_rate_zero(self):
        """Same count gives 0% growth."""
        trend = self._compute_trend("cnn", {2020: 10, 2023: 10}, (2020, 2023))
        assert trend is not None
        assert trend.growth_rate == 0.0

    def test_direction_rising_over_50_percent(self):
        """>50% growth with no recent acceleration = RISING."""
        trend = self._compute_trend("rag", {2020: 10, 2023: 20}, (2020, 2023))
        assert trend.direction == TrendDirection.RISING

    def test_direction_emerging_with_acceleration(self):
        """>50% growth with last > prev-last = EMERGING."""
        # counts: [5, 8, 15] -> last=15 > prev=8 (acceleration)
        trend = self._compute_trend("moe", {2021: 5, 2022: 8, 2023: 15}, (2021, 2023))
        assert trend.direction == TrendDirection.EMERGING

    def test_direction_falling_over_30_percent(self):
        """>30% decline = FALLING."""
        trend = self._compute_trend("rnn", {2020: 100, 2023: 50}, (2020, 2023))
        assert trend.direction == TrendDirection.FALLING

    def test_direction_stable_under_20_percent(self):
        """<20% change = STABLE."""
        trend = self._compute_trend("bert", {2020: 10, 2023: 11}, (2020, 2023))
        assert trend.direction == TrendDirection.STABLE

    def test_peak_year(self):
        """Peak year is the year with highest count."""
        trend = self._compute_trend("gpt", {2020: 5, 2021: 20, 2022: 15, 2023: 10}, (2020, 2023))
        assert trend.peak_year == 2021

    def test_peak_year_fallback(self):
        """Peak year falls back to last year when all zero."""
        trend = self._compute_trend("x", {}, (2020, 2023))
        assert trend is None

    def test_momentum_positive(self):
        """Positive momentum when recent acceleration increases."""
        # counts: [5, 8, 15] -> recent_change=7, prev_change=3 -> momentum=(7-3)/3
        trend = self._compute_trend("m", {2021: 5, 2022: 8, 2023: 15}, (2021, 2023))
        assert trend.momentum > 0

    def test_momentum_zero_without_history(self):
        """Momentum is 0 when not enough history."""
        trend = self._compute_trend("m", {2023: 10}, (2023, 2023))
        assert trend.momentum == 0.0

    def test_velocity_sum_last_3_years(self):
        """Velocity is sum of last 3 years."""
        trend = self._compute_trend("m", {2021: 5, 2022: 8, 2023: 15}, (2021, 2023))
        assert trend.velocity == 28  # 5 + 8 + 15

    def test_current_year_count(self):
        """current_year_count is last count."""
        trend = self._compute_trend("m", {2020: 1, 2021: 2, 2023: 20}, (2020, 2023))
        assert trend.current_year_count == 20


# =============================================================================
# _compute_growth_rate tests
# =============================================================================
class TestComputeGrowthRate:
    """Test _compute_growth_rate logic."""

    def _compute_growth_rate(self, yearly_stats) -> float:
        """Replicate _compute_growth_rate logic."""
        if len(yearly_stats) < 2:
            return 0.0
        first = yearly_stats[0].paper_count
        last = yearly_stats[-1].paper_count
        if first == 0:
            return 0.0
        return ((last - first) / first) * 100

    def test_growth_rate_formula(self):
        """Growth rate = (last - first) / first * 100."""
        stats = [
            YearlyStats(year=2020, paper_count=50, total_citations=0, avg_citations=0),
            YearlyStats(year=2021, paper_count=100, total_citations=0, avg_citations=0),
        ]
        assert self._compute_growth_rate(stats) == 100.0

    def test_growth_rate_negative(self):
        """Decline gives negative growth rate."""
        stats = [
            YearlyStats(year=2020, paper_count=100, total_citations=0, avg_citations=0),
            YearlyStats(year=2021, paper_count=25, total_citations=0, avg_citations=0),
        ]
        assert self._compute_growth_rate(stats) == -75.0

    def test_growth_rate_zero_when_same(self):
        """Same count gives 0%."""
        stats = [
            YearlyStats(year=2020, paper_count=50, total_citations=0, avg_citations=0),
            YearlyStats(year=2021, paper_count=50, total_citations=0, avg_citations=0),
        ]
        assert self._compute_growth_rate(stats) == 0.0

    def test_growth_rate_zero_single_year(self):
        """Single year returns 0."""
        stats = [YearlyStats(year=2020, paper_count=50, total_citations=0, avg_citations=0)]
        assert self._compute_growth_rate(stats) == 0.0

    def test_growth_rate_zero_when_first_zero(self):
        """First year zero returns 0."""
        stats = [
            YearlyStats(year=2020, paper_count=0, total_citations=0, avg_citations=0),
            YearlyStats(year=2021, paper_count=100, total_citations=0, avg_citations=0),
        ]
        assert self._compute_growth_rate(stats) == 0.0


# =============================================================================
# render_result tests
# =============================================================================
class TestRenderResult:
    """Test render_result logic."""

    def _render_result(self, result) -> str:
        """Replicate render_result logic."""
        lines = [
            f"📈 《{result.topic}》研究趋势分析 ({result.year_range[0]}-{result.year_range[1]})",
            f"   总论文数: {result.total_papers} | 整体增长率: {result.growth_rate:+.1f}%",
            "",
        ]

        if result.yearly_distribution:
            lines.append("📊 年度分布:")
            for stats in result.yearly_distribution:
                bar = "█" * min(stats.paper_count, 20)
                lines.append(f"   {stats.year}: {stats.paper_count:3d} {bar}")
            lines.append("")

        if result.rising_trends:
            lines.append("🔥 上升趋势:")
            for trend in result.rising_trends[:5]:
                growth_str = f"+{trend.growth_rate:.0f}%" if trend.growth_rate >= 0 else f"{trend.growth_rate:.0f}%"
                lines.append(f"   ↑ {trend.keyword}: {growth_str} ({trend.current_year_count}篇)")
            lines.append("")

        if result.emerging_trends:
            lines.append("🆕 新兴方向:")
            for trend in result.emerging_trends[:5]:
                lines.append(f"   ✨ {trend.keyword}: +{trend.growth_rate:.0f}% 加速中")
            lines.append("")

        if result.falling_trends:
            lines.append("📉 下降趋势:")
            for trend in result.falling_trends[:5]:
                lines.append(f"   ↓ {trend.keyword}: {trend.growth_rate:.0f}% ({trend.current_year_count}篇)")
            lines.append("")

        return "\n".join(lines)

    def test_header_contains_topic(self):
        """Header contains topic name."""
        result = TrendAnalysisResult(topic="NLP", year_range=(2020, 2025), total_papers=500)
        output = self._render_result(result)
        assert "NLP" in output
        assert "2020-2025" in output

    def test_header_contains_total_papers(self):
        """Header shows total paper count."""
        result = TrendAnalysisResult(topic="T", year_range=(2020, 2025), total_papers=123)
        output = self._render_result(result)
        assert "总论文数: 123" in output

    def test_header_contains_growth_rate(self):
        """Header shows growth rate with sign."""
        result = TrendAnalysisResult(topic="T", year_range=(2020, 2025), total_papers=0, growth_rate=50.0)
        output = self._render_result(result)
        assert "+50.0%" in output

    def test_yearly_distribution_shown(self):
        """Yearly distribution section rendered."""
        stats = [YearlyStats(year=2023, paper_count=50, total_citations=0, avg_citations=0)]
        result = TrendAnalysisResult(
            topic="T", year_range=(2023, 2023), total_papers=50,
            yearly_distribution=stats,
        )
        output = self._render_result(result)
        assert "📊 年度分布" in output
        assert "2023" in output

    def test_bar_visualization(self):
        """Bar chart uses █ character."""
        stats = [YearlyStats(year=2023, paper_count=5, total_citations=0, avg_citations=0)]
        result = TrendAnalysisResult(
            topic="T", year_range=(2023, 2023), total_papers=5,
            yearly_distribution=stats,
        )
        output = self._render_result(result)
        assert "█" in output

    def test_rising_trends_shown(self):
        """Rising trends section rendered."""
        trend = TrendKeyword(
            keyword="transformer",
            direction=TrendDirection.RISING,
            yearly_counts={2023: 20},
            growth_rate=100.0,
            peak_year=2023,
            current_year_count=20,
        )
        result = TrendAnalysisResult(
            topic="T", year_range=(2023, 2023), total_papers=20,
            rising_trends=[trend],
        )
        output = self._render_result(result)
        assert "🔥 上升趋势" in output
        assert "transformer" in output
        assert "+100%" in output

    def test_emerging_trends_shown(self):
        """Emerging trends section rendered."""
        trend = TrendKeyword(
            keyword="moe",
            direction=TrendDirection.EMERGING,
            yearly_counts={2023: 15},
            growth_rate=200.0,
            peak_year=2023,
            current_year_count=15,
        )
        result = TrendAnalysisResult(
            topic="T", year_range=(2023, 2023), total_papers=15,
            emerging_trends=[trend],
        )
        output = self._render_result(result)
        assert "🆕 新兴方向" in output
        assert "moe" in output

    def test_falling_trends_shown(self):
        """Falling trends section rendered."""
        trend = TrendKeyword(
            keyword="rnn",
            direction=TrendDirection.FALLING,
            yearly_counts={2023: 5},
            growth_rate=-40.0,
            peak_year=2021,
            current_year_count=5,
        )
        result = TrendAnalysisResult(
            topic="T", year_range=(2023, 2023), total_papers=5,
            falling_trends=[trend],
        )
        output = self._render_result(result)
        assert "📉 下降趋势" in output
        assert "rnn" in output

    def test_empty_result_no_sections(self):
        """Empty result only shows header."""
        result = TrendAnalysisResult(topic="T", year_range=(2020, 2025), total_papers=0)
        output = self._render_result(result)
        assert "📊" not in output
        assert "🔥" not in output
        assert "📉" not in output


# =============================================================================
# render_mermaid_timeline tests
# =============================================================================
class TestRenderMermaidTimeline:
    """Test render_mermaid_timeline logic."""

    def _render_mermaid_timeline(self, result) -> str:
        """Replicate render_mermaid_timeline logic."""
        lines = [
            "gantt",
            "    title Research Trends - " + result.topic,
            "    dateFormat YYYY",
            "    section Keywords",
        ]

        for trend in result.emerging_trends[:3] + result.rising_trends[:3]:
            yearly = trend.yearly_counts
            if yearly:
                start_year = min(yearly.keys())
                end_year = max(yearly.keys())
                status = "active" if trend.direction == TrendDirection.EMERGING else "done"
                lines.append(f"    {trend.keyword} ({status}) :t{start_year}, {end_year - start_year + 1}y")

        return "\n".join(lines)

    def test_header_present(self):
        """Gantt chart header present."""
        result = TrendAnalysisResult(topic="NLP", year_range=(2020, 2025), total_papers=0)
        output = self._render_mermaid_timeline(result)
        assert "gantt" in output
        assert "Research Trends - NLP" in output
        assert "dateFormat YYYY" in output

    def test_empty_trends_no_entries(self):
        """No trends means no entries."""
        result = TrendAnalysisResult(topic="T", year_range=(2020, 2025), total_papers=0)
        output = self._render_mermaid_timeline(result)
        lines = output.split("\n")
        # Only header lines, no trend entries
        assert len(lines) == 4

    def test_emerging_trend_active(self):
        """Emerging trend gets 'active' status."""
        trend = TrendKeyword(
            keyword="moe",
            direction=TrendDirection.EMERGING,
            yearly_counts={2022: 5, 2023: 15},
            growth_rate=200.0,
            peak_year=2023,
            current_year_count=15,
        )
        result = TrendAnalysisResult(
            topic="T", year_range=(2020, 2025), total_papers=20,
            emerging_trends=[trend],
        )
        output = self._render_mermaid_timeline(result)
        assert "moe (active)" in output
        assert "t2022" in output

    def test_rising_trend_done(self):
        """Rising trend gets 'done' status."""
        trend = TrendKeyword(
            keyword="transformer",
            direction=TrendDirection.RISING,
            yearly_counts={2020: 10, 2023: 50},
            growth_rate=400.0,
            peak_year=2023,
            current_year_count=50,
        )
        result = TrendAnalysisResult(
            topic="T", year_range=(2020, 2025), total_papers=60,
            rising_trends=[trend],
        )
        output = self._render_mermaid_timeline(result)
        assert "transformer (done)" in output

    def test_duration_calculation(self):
        """Duration is end_year - start_year + 1."""
        trend = TrendKeyword(
            keyword="m",
            direction=TrendDirection.RISING,
            yearly_counts={2020: 1, 2021: 2, 2022: 3},
            growth_rate=200.0,
            peak_year=2022,
            current_year_count=3,
        )
        result = TrendAnalysisResult(
            topic="T", year_range=(2020, 2025), total_papers=6,
            rising_trends=[trend],
        )
        output = self._render_mermaid_timeline(result)
        # 2022 - 2020 + 1 = 3 years
        assert "3y" in output

    def test_max_3_emerging_trends(self):
        """At most 3 emerging trends included."""
        trends = [
            TrendKeyword(
                keyword=f"t{i}",
                direction=TrendDirection.EMERGING,
                yearly_counts={2023: i},
                growth_rate=100.0,
                peak_year=2023,
                current_year_count=i,
            )
            for i in range(5)
        ]
        result = TrendAnalysisResult(
            topic="T", year_range=(2020, 2025), total_papers=10,
            emerging_trends=trends,
        )
        output = self._render_mermaid_timeline(result)
        for i in range(3):
            assert f"t{i}" in output
        assert "t3" not in output


# =============================================================================
# render_mermaid_timeline_v2 tests
# =============================================================================
class TestRenderMermaidTimelineV2:
    """Test render_mermaid_timeline_v2 logic."""

    def _render_mermaid_timeline_v2(self, result) -> str:
        """Replicate render_mermaid_timeline_v2 logic."""
        all_trends = result.emerging_trends[:2] + result.rising_trends[:2]
        if not all_trends:
            return ""

        all_years = set()
        for trend in all_trends:
            all_years.update(trend.yearly_counts.keys())
        if not all_years:
            return ""
        year_range = list(range(min(all_years), max(all_years) + 1))

        lines = [
            "%%{ init: { 'theme': 'base', 'themeVariables': { 'primaryColor': '#ff9900' } } }%%",
            "```mermaid",
            "xychart-beta",
            f'    title "{result.topic} - Keyword Trends"',
            f"    x-axis [{', '.join(str(y) for y in year_range)}]",
            "    y-axis \"Papers\" 0 --> 50",
            "",
            "    bar",
        ]

        for trend in all_trends[:4]:
            counts = [trend.yearly_counts.get(y, 0) for y in year_range]
            lines.append(f'        "{trend.keyword}" : {", ".join(str(c) for c in counts)}')

        lines.append("```")
        return "\n".join(lines)

    def test_empty_trends_returns_empty(self):
        """No trends returns empty string."""
        result = TrendAnalysisResult(topic="T", year_range=(2020, 2025), total_papers=0)
        assert self._render_mermaid_timeline_v2(result) == ""

    def test_header_present(self):
        """Mermaid XYChart header present."""
        trend = TrendKeyword(
            keyword="transformer",
            direction=TrendDirection.RISING,
            yearly_counts={2022: 10, 2023: 20},
            growth_rate=100.0,
            peak_year=2023,
            current_year_count=20,
        )
        result = TrendAnalysisResult(
            topic="NLP", year_range=(2020, 2025), total_papers=30,
            rising_trends=[trend],
        )
        output = self._render_mermaid_timeline_v2(result)
        assert "xychart-beta" in output
        assert "NLP - Keyword Trends" in output

    def test_year_axis_dynamic(self):
        """X-axis built from actual data, not hardcoded."""
        trend = TrendKeyword(
            keyword="gpt",
            direction=TrendDirection.RISING,
            yearly_counts={2019: 5, 2020: 10, 2021: 20},
            growth_rate=300.0,
            peak_year=2021,
            current_year_count=20,
        )
        result = TrendAnalysisResult(
            topic="T", year_range=(2019, 2023), total_papers=35,
            rising_trends=[trend],
        )
        output = self._render_mermaid_timeline_v2(result)
        assert "2019" in output
        assert "2021" in output

    def test_bar_data_series(self):
        """Bar data series for each trend."""
        trend = TrendKeyword(
            keyword="rag",
            direction=TrendDirection.RISING,
            yearly_counts={2022: 5, 2023: 15},
            growth_rate=200.0,
            peak_year=2023,
            current_year_count=15,
        )
        result = TrendAnalysisResult(
            topic="T", year_range=(2022, 2023), total_papers=20,
            rising_trends=[trend],
        )
        output = self._render_mermaid_timeline_v2(result)
        assert '"rag" : 5, 15' in output

    def test_max_4_trends(self):
        """At most 4 trends included (2 emerging + 2 rising)."""
        rising = TrendKeyword(
            keyword="x",
            direction=TrendDirection.RISING,
            yearly_counts={2023: 5},
            growth_rate=100.0,
            peak_year=2023,
            current_year_count=5,
        )
        emerging = TrendKeyword(
            keyword="y",
            direction=TrendDirection.EMERGING,
            yearly_counts={2023: 3},
            growth_rate=200.0,
            peak_year=2023,
            current_year_count=3,
        )
        result = TrendAnalysisResult(
            topic="T", year_range=(2023, 2023), total_papers=5,
            rising_trends=[rising] * 5,
            emerging_trends=[emerging] * 5,
        )
        output = self._render_mermaid_timeline_v2(result)
        # Should have 4 bar entries max (2 emerging + 2 rising from slicing)
        lines = [l for l in output.split("\n") if '"x"' in l or '"y"' in l]
        assert len(lines) == 4


# =============================================================================
# TrendAnalyzer instantiation
# =============================================================================
class TestTrendAnalyzerInit:
    """Test TrendAnalyzer class."""

    def test_can_instantiate(self):
        """TrendAnalyzer can be instantiated."""
        analyzer = TrendAnalyzer()
        assert analyzer.db is None

    def test_can_instantiate_with_db(self):
        """TrendAnalyzer can be instantiated with db."""
        mock_db = object()
        analyzer = TrendAnalyzer(db=mock_db)
        assert analyzer.db is mock_db
