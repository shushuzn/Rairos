"""
Research Trend Analyzer: Analyze research trends over time.

研究趋势分析器：分析论文的时间分布、关键词趋势、引用速度。

核心算法：
1. 时间序列分析：论文数量按年/月的分布
2. 关键词趋势：特定方法/概念的出现频率变化
3. 引用速度：新论文被引用的速度
4. 趋势分类：上升、下降、新兴、平稳
"""

from __future__ import annotations

import re
from collections import defaultdict
from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum
from typing import Optional, List, Dict, Any, Tuple

from llm.constants import AI_RESEARCH_KEYWORDS

import numpy as np


class TrendDirection(Enum):
    """Trend direction classification."""
    RISING = "rising"      # 上升趋势
    FALLING = "falling"    # 下降趋势
    EMERGING = "emerging"  # 新兴
    STABLE = "stable"      # 平稳
    UNKNOWN = "unknown"    # 未知


@dataclass
class YearlyStats:
    """Statistics for a single year."""
    year: int
    paper_count: int
    total_citations: int
    avg_citations: float
    keywords: Dict[str, int] = field(default_factory=dict)


@dataclass
class TrendKeyword:
    """A keyword/method with trend data."""
    keyword: str
    direction: TrendDirection
    yearly_counts: Dict[int, int]  # year -> count
    growth_rate: float  # percentage growth
    peak_year: int
    current_year_count: int
    velocity: float = 0.0  # citation velocity
    momentum: float = 0.0  # acceleration


@dataclass
class TrendAnalysisResult:
    """Complete trend analysis result."""
    topic: str
    year_range: Tuple[int, int]
    total_papers: int
    yearly_distribution: List[YearlyStats] = field(default_factory=list)
    rising_trends: List[TrendKeyword] = field(default_factory=list)
    falling_trends: List[TrendKeyword] = field(default_factory=list)
    emerging_trends: List[TrendKeyword] = field(default_factory=list)
    stable_trends: List[TrendKeyword] = field(default_factory=list)
    hot_keywords: List[str] = field(default_factory=list)
    declining_keywords: List[str] = field(default_factory=list)
    emerging_keywords: List[str] = field(default_factory=list)
    growth_rate: float = 0.0  # Overall field growth


class TrendAnalyzer:
    """Analyze research trends from paper corpus."""

    # Common tech keywords to track
    TECH_KEYWORDS = AI_RESEARCH_KEYWORDS

    def __init__(self, db=None):
        self.db = db

    def analyze(
        self,
        topic: str,
        year_range: Optional[Tuple[int, int]] = None,
        min_papers: int = 10,
    ) -> TrendAnalysisResult:
        """
        Analyze research trends for a topic.

        Args:
            topic: Research topic/keyword
            year_range: (start_year, end_year) to analyze
            min_papers: Minimum papers needed

        Returns:
            TrendAnalysisResult with trend data
        """
        # Default year range: last 6 years
        current_year = datetime.now().year
        if year_range is None:
            year_range = (current_year - 5, current_year)

        # Collect papers
        papers = self._collect_papers(topic)
        if len(papers) < min_papers:
            return self._empty_result(topic, year_range)

        # Extract yearly statistics
        yearly_stats = self._compute_yearly_stats(papers, year_range)

        # Detect keyword trends
        trends = self._detect_keyword_trends(papers, year_range)

        # Compute overall growth rate
        growth = self._compute_growth_rate(yearly_stats)

        # Build result
        result = TrendAnalysisResult(
            topic=topic,
            year_range=year_range,
            total_papers=len(papers),
            yearly_distribution=yearly_stats,
            rising_trends=[t for t in trends if t.direction == TrendDirection.RISING],
            falling_trends=[t for t in trends if t.direction == TrendDirection.FALLING],
            emerging_trends=[t for t in trends if t.direction == TrendDirection.EMERGING],
            stable_trends=[t for t in trends if t.direction == TrendDirection.STABLE],
            hot_keywords=[t.keyword for t in trends if t.direction == TrendDirection.RISING][:5],
            declining_keywords=[t.keyword for t in trends if t.direction == TrendDirection.FALLING][:5],
            emerging_keywords=[t.keyword for t in trends if t.direction == TrendDirection.EMERGING][:5],
            growth_rate=growth,
        )

        return result

    def _collect_papers(self, topic: str) -> List[Dict[str, Any]]:
        """Collect papers from DB, with correct forward citation counts from citations table.

        Falls back to reference_count if citations table is empty.
        """
        if not self.db:
            return []

        papers = []
        try:
            rows, _ = self.db.search_papers(topic, limit=100)
            paper_ids = [row.paper_id for row in rows if row.paper_id]

            if not paper_ids:
                return []

            # Build forward citation count from citations table
            placeholders = ",".join("?" * len(paper_ids))
            cite_rows = self.db.conn.execute(
                f"""
                SELECT target_id, COUNT(*) AS forward_cites
                FROM citations
                WHERE target_id IN ({placeholders})
                GROUP BY target_id
                """,
                paper_ids,
            ).fetchall()
            cite_map = {row[0]: row[1] for row in cite_rows}

            for row in rows:
                pid = row.paper_id
                pub = row.published or ""
                try:
                    year_val = int(pub[:4]) if pub else 0
                except (ValueError, TypeError):
                    year_val = 0
                # Forward citations: from citations table
                forward_cites = cite_map.get(pid, 0)
                # Fallback: reference_count column (backward citations / references)
                ref_count = getattr(row, 'reference_count', 0) or 0

                paper = {
                    "id": pid,
                    "title": row.title or '',
                    "abstract": getattr(row, 'abstract', '') or '',
                    "year": year_val,
                    "citations": forward_cites,  # forward = papers that cite this one
                    "reference_count": ref_count,  # backward = papers this one cites
                    "authors": getattr(row, 'authors', '') or '',
                }
                if year_val > 2000:
                    papers.append(paper)
        except Exception:
            # Paper enrichment failed — return partial results without crashing.
            pass

        return papers

    def _compute_yearly_stats(
        self,
        papers: List[Dict],
        year_range: Tuple[int, int],
    ) -> List[YearlyStats]:
        """Compute statistics per year."""
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

                # Extract keywords from title/abstract
                text = f"{paper.get('title', '')} {paper.get('abstract', '')}".lower()
                for kw in self.TECH_KEYWORDS:
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

    def _detect_keyword_trends(
        self,
        papers: List[Dict],
        year_range: Tuple[int, int],
    ) -> List[TrendKeyword]:
        """Detect trends for common keywords."""
        # Count keyword occurrences per year
        keyword_yearly = defaultdict(lambda: defaultdict(int))

        for paper in papers:
            year = paper.get("year", 0)
            if year_range[0] <= year <= year_range[1]:
                text = f"{paper.get('title', '')} {paper.get('abstract', '')}".lower()
                for kw in self.TECH_KEYWORDS:
                    if kw.lower() in text:
                        keyword_yearly[kw][year] += 1

        # Compute trends
        trends = []
        for keyword, yearly_counts in keyword_yearly.items():
            if sum(yearly_counts.values()) < 3:  # Min occurrences
                continue

            trend = self._compute_trend(keyword, yearly_counts, year_range)
            if trend:
                trends.append(trend)

        # Sort by growth rate and return top trends
        trends.sort(key=lambda t: t.growth_rate, reverse=True)
        return trends[:20]

    def _compute_trend(
        self,
        keyword: str,
        yearly_counts: Dict[int, int],
        year_range: Tuple[int, int],
    ) -> Optional[TrendKeyword]:
        """Compute trend for a single keyword."""
        years = list(range(year_range[0], year_range[1] + 1))
        counts = [yearly_counts.get(y, 0) for y in years]

        # Growth rate: percentage change from first to last non-zero year
        first_nonzero = next((c for c in counts if c > 0), 0)
        last_count = counts[-1]

        if first_nonzero == 0:
            return None

        growth_rate = ((last_count - first_nonzero) / first_nonzero) * 100

        # Determine direction
        if last_count > first_nonzero * 1.5:  # >50% growth
            if counts[-1] > counts[-2] > 0:  # Recent acceleration
                direction = TrendDirection.EMERGING
            else:
                direction = TrendDirection.RISING
        elif last_count < first_nonzero * 0.7:  # >30% decline
            direction = TrendDirection.FALLING
        elif abs(growth_rate) < 20:  # <20% change
            direction = TrendDirection.STABLE
        else:
            direction = TrendDirection.UNKNOWN

        # Peak year
        peak_year = max(yearly_counts.keys(), key=lambda y: yearly_counts[y], default=years[-1])

        # Momentum: acceleration of growth
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

    def _compute_growth_rate(self, yearly_stats: List[YearlyStats]) -> float:
        """Compute overall field growth rate."""
        if len(yearly_stats) < 2:
            return 0.0

        first = yearly_stats[0].paper_count
        last = yearly_stats[-1].paper_count

        if first == 0:
            return 0.0

        return ((last - first) / first) * 100

    def _empty_result(self, topic: str, year_range: Tuple[int, int]) -> TrendAnalysisResult:
        """Return empty result."""
        return TrendAnalysisResult(
            topic=topic,
            year_range=year_range,
            total_papers=0,
        )

    def render_result(self, result: TrendAnalysisResult) -> str:
        """Render analysis as formatted text."""
        lines = [
            f"📈 《{result.topic}》研究趋势分析 ({result.year_range[0]}-{result.year_range[1]})",
            f"   总论文数: {result.total_papers} | 整体增长率: {result.growth_rate:+.1f}%",
            "",
        ]

        # Yearly distribution
        if result.yearly_distribution:
            lines.append("📊 年度分布:")
            for stats in result.yearly_distribution:
                bar = "█" * min(stats.paper_count, 20)
                lines.append(f"   {stats.year}: {stats.paper_count:3d} {bar}")
            lines.append("")

        # Rising trends
        if result.rising_trends:
            lines.append("🔥 上升趋势:")
            for trend in result.rising_trends[:5]:
                growth_str = f"+{trend.growth_rate:.0f}%" if trend.growth_rate >= 0 else f"{trend.growth_rate:.0f}%"
                lines.append(f"   ↑ {trend.keyword}: {growth_str} ({trend.current_year_count}篇)")
            lines.append("")

        # Emerging trends
        if result.emerging_trends:
            lines.append("🆕 新兴方向:")
            for trend in result.emerging_trends[:5]:
                lines.append(f"   ✨ {trend.keyword}: +{trend.growth_rate:.0f}% 加速中")
            lines.append("")

        # Falling trends
        if result.falling_trends:
            lines.append("📉 下降趋势:")
            for trend in result.falling_trends[:5]:
                lines.append(f"   ↓ {trend.keyword}: {trend.growth_rate:.0f}% ({trend.current_year_count}篇)")
            lines.append("")

        return "\n".join(lines)

    def render_mermaid_timeline(self, result: TrendAnalysisResult) -> str:
        """Render as Mermaid timeline."""
        lines = [
            "gantt",
            "    title Research Trends - " + result.topic,
            "    dateFormat YYYY",
            "    section Keywords",
        ]

        # Add top rising and emerging trends
        for trend in result.emerging_trends[:3] + result.rising_trends[:3]:
            yearly = trend.yearly_counts
            if yearly:
                start_year = min(yearly.keys())
                end_year = max(yearly.keys())
                status = "active" if trend.direction == TrendDirection.EMERGING else "done"
                lines.append(f"    {trend.keyword} ({status}) :t{start_year}, {end_year - start_year + 1}y")

        return "\n".join(lines)

    def render_mermaid_timeline_v2(self, result: TrendAnalysisResult) -> str:
        """Render as Mermaid XYChart (if supported)."""
        lines = [
            "%%{ init: { 'theme': 'base', 'themeVariables': { 'primaryColor': '#ff9900' } } }%%",
            "```mermaid",
            "xychart-beta",
            f'    title "{result.topic} - Keyword Trends"',
            "    x-axis [2019, 2020, 2021, 2022, 2023, 2024, 2025]",
            "    y-axis \"Papers\" 0 --> 50",
            "",
            '    bar',
        ]

        # Add data series for top keywords
        for trend in (result.emerging_trends[:2] + result.rising_trends[:2])[:4]:
            years = list(range(2019, 2026))
            counts = [trend.yearly_counts.get(y, 0) for y in years]
            data_str = ", ".join(str(c) for c in counts)
            lines.append(f'        "{trend.keyword}" : {data_str}')

        lines.append("```")
        return "\n".join(lines)
