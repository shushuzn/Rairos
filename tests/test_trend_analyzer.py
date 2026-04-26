"""Tests for research trend analyzer."""
import pytest

from llm.trend_analyzer import (
    TrendAnalyzer,
    TrendDirection,
    YearlyStats,
    TrendKeyword,
    TrendAnalysisResult,
)


class TestTrendAnalyzer:
    """Test TrendAnalyzer."""

    def test_empty_result_when_no_db(self):
        """Test empty result when no DB available."""
        analyzer = TrendAnalyzer(db=None)
        result = analyzer.analyze("nonexistent_topic_xyz_123")

        assert isinstance(result, TrendAnalysisResult)
        assert result.topic == "nonexistent_topic_xyz_123"
        assert result.total_papers == 0

    def test_trend_direction_enum(self):
        """Test TrendDirection enum values."""
        assert TrendDirection.RISING.value == "rising"
        assert TrendDirection.FALLING.value == "falling"
        assert TrendDirection.EMERGING.value == "emerging"
        assert TrendDirection.STABLE.value == "stable"
        assert TrendDirection.UNKNOWN.value == "unknown"

    def test_yearly_stats_creation(self):
        """Test YearlyStats dataclass."""
        stats = YearlyStats(
            year=2024,
            paper_count=50,
            total_citations=500,
            avg_citations=10.0,
            keywords={"transformer": 20, "attention": 15},
        )

        assert stats.year == 2024
        assert stats.paper_count == 50
        assert stats.avg_citations == 10.0
        assert stats.keywords["transformer"] == 20

    def test_trend_keyword_creation(self):
        """Test TrendKeyword dataclass."""
        keyword = TrendKeyword(
            keyword="transformer",
            direction=TrendDirection.RISING,
            yearly_counts={2022: 10, 2023: 25, 2024: 50},
            growth_rate=400.0,
            peak_year=2024,
            current_year_count=50,
            velocity=125.0,
            momentum=1.5,
        )

        assert keyword.keyword == "transformer"
        assert keyword.direction == TrendDirection.RISING
        assert keyword.growth_rate == 400.0
        assert keyword.peak_year == 2024

    def test_compute_trend_rising(self):
        """Test rising trend detection."""
        analyzer = TrendAnalyzer()
        yearly_counts = {2022: 5, 2023: 10, 2024: 20}

        trend = analyzer._compute_trend("test_keyword", yearly_counts, (2022, 2024))

        assert trend is not None
        assert trend.direction in (TrendDirection.RISING, TrendDirection.EMERGING)
        assert trend.growth_rate > 0
        assert trend.peak_year == 2024

    def test_compute_trend_falling(self):
        """Test falling trend detection."""
        analyzer = TrendAnalyzer()
        yearly_counts = {2022: 50, 2023: 30, 2024: 10}

        trend = analyzer._compute_trend("test_keyword", yearly_counts, (2022, 2024))

        assert trend is not None
        assert trend.direction == TrendDirection.FALLING
        assert trend.growth_rate < 0

    def test_compute_trend_stable(self):
        """Test stable trend detection."""
        analyzer = TrendAnalyzer()
        yearly_counts = {2022: 10, 2023: 11, 2024: 10}

        trend = analyzer._compute_trend("test_keyword", yearly_counts, (2022, 2024))

        assert trend is not None
        assert trend.direction == TrendDirection.STABLE

    def test_compute_trend_all_zeros(self):
        """Test trend with all zeros returns None."""
        analyzer = TrendAnalyzer()
        yearly_counts = {2022: 0, 2023: 0, 2024: 0}  # All zero

        trend = analyzer._compute_trend("test_keyword", yearly_counts, (2022, 2024))

        assert trend is None  # Should return None for all zeros

    def test_compute_growth_rate(self):
        """Test growth rate calculation."""
        analyzer = TrendAnalyzer()
        stats = [
            YearlyStats(year=2022, paper_count=10, total_citations=100, avg_citations=10.0),
            YearlyStats(year=2023, paper_count=20, total_citations=200, avg_citations=10.0),
            YearlyStats(year=2024, paper_count=40, total_citations=400, avg_citations=10.0),
        ]

        growth = analyzer._compute_growth_rate(stats)

        assert growth == 300.0  # (40-10)/10 * 100

    def test_compute_growth_rate_empty(self):
        """Test growth rate with empty stats."""
        analyzer = TrendAnalyzer()
        growth = analyzer._compute_growth_rate([])
        assert growth == 0.0

    def test_compute_growth_rate_single_year(self):
        """Test growth rate with single year."""
        analyzer = TrendAnalyzer()
        stats = [
            YearlyStats(year=2024, paper_count=10, total_citations=100, avg_citations=10.0),
        ]
        growth = analyzer._compute_growth_rate(stats)
        assert growth == 0.0

    def test_render_result(self):
        """Test result rendering."""
        analyzer = TrendAnalyzer()
        result = TrendAnalysisResult(
            topic="Transformer",
            year_range=(2022, 2024),
            total_papers=100,
            yearly_distribution=[
                YearlyStats(year=2022, paper_count=20, total_citations=200, avg_citations=10.0),
                YearlyStats(year=2023, paper_count=35, total_citations=350, avg_citations=10.0),
                YearlyStats(year=2024, paper_count=45, total_citations=450, avg_citations=10.0),
            ],
            rising_trends=[
                TrendKeyword(
                    keyword="attention",
                    direction=TrendDirection.RISING,
                    yearly_counts={2022: 10, 2023: 20, 2024: 30},
                    growth_rate=200.0,
                    peak_year=2024,
                    current_year_count=30,
                ),
            ],
            growth_rate=125.0,
        )

        output = analyzer.render_result(result)

        assert "Transformer" in output
        assert "100" in output
        assert "125" in output or "125.0" in output
        assert "attention" in output

    def test_render_mermaid_timeline(self):
        """Test Mermaid timeline rendering."""
        analyzer = TrendAnalyzer()
        result = TrendAnalysisResult(
            topic="LLM",
            year_range=(2022, 2024),
            total_papers=50,
            emerging_trends=[
                TrendKeyword(
                    keyword="scaling",
                    direction=TrendDirection.EMERGING,
                    yearly_counts={2022: 5, 2023: 15, 2024: 40},
                    growth_rate=700.0,
                    peak_year=2024,
                    current_year_count=40,
                ),
            ],
            rising_trends=[
                TrendKeyword(
                    keyword="rlhf",
                    direction=TrendDirection.RISING,
                    yearly_counts={2022: 8, 2023: 12, 2024: 18},
                    growth_rate=125.0,
                    peak_year=2024,
                    current_year_count=18,
                ),
            ],
        )

        output = analyzer.render_mermaid_timeline(result)

        assert "gantt" in output
        assert "LLM" in output
        assert "scaling" in output
        assert "rlhf" in output

    def test_render_mermaid_timeline_v2(self):
        """Test Mermaid XYChart rendering."""
        analyzer = TrendAnalyzer()
        result = TrendAnalysisResult(
            topic="Vision",
            year_range=(2022, 2024),
            total_papers=30,
            rising_trends=[
                TrendKeyword(
                    keyword="vit",
                    direction=TrendDirection.RISING,
                    yearly_counts={2022: 10, 2023: 20, 2024: 35},
                    growth_rate=250.0,
                    peak_year=2024,
                    current_year_count=35,
                ),
            ],
        )

        output = analyzer.render_mermaid_timeline_v2(result)

        assert "xychart-beta" in output
        assert "Vision" in output
        assert "vit" in output

    def test_empty_result(self):
        """Test empty result handling."""
        analyzer = TrendAnalyzer()
        result = analyzer._empty_result("Test Topic", (2020, 2024))

        assert result.topic == "Test Topic"
        assert result.year_range == (2020, 2024)
        assert result.total_papers == 0
        assert len(result.rising_trends) == 0

    def test_tech_keywords_list(self):
        """Test that TECH_KEYWORDS contains expected terms."""
        analyzer = TrendAnalyzer()

        expected_keywords = [
            "transformer", "attention", "bert", "gpt", "llm",
            "neural", "embedding", "fine-tuning", "rlhf", "rag",
            "diffusion", "gan", "clip", "vit",
            "multimodal", "pre-training", "reasoning",
        ]

        for kw in expected_keywords:
            assert kw in analyzer.TECH_KEYWORDS, f"{kw} not in TECH_KEYWORDS"
