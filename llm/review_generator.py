"""
Literature Review Generator: Generate structured literature reviews.

Generates comprehensive reviews with:
- Research stream classification
- Controversy detection between streams
- Evolution timeline construction
- Open problem identification
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import List, Optional, Tuple, Any


@dataclass
class ResearchStream:
    """A research stream/school in the literature."""
    name: str
    papers: List[str] = field(default_factory=list)
    methods: List[str] = field(default_factory=list)
    key_contributions: List[str] = field(default_factory=list)


@dataclass
class Controversy:
    """A controversy between research streams."""
    topic: str
    stream_a: str
    stream_b: str
    position_a: str
    position_b: str
    papers: List[str] = field(default_factory=list)


@dataclass
class ReviewSection:
    """A section of the review."""
    title: str
    content: str
    subsections: List['ReviewSection'] = field(default_factory=list)


@dataclass
class LiteratureReview:
    """Generated literature review."""
    topic: str
    streams: List[ResearchStream] = field(default_factory=list)
    controversies: List[Controversy] = field(default_factory=list)
    timeline: List[Tuple[int, str]] = field(default_factory=list)
    open_problems: List[str] = field(default_factory=list)
    sections: List[ReviewSection] = field(default_factory=list)


class ReviewGenerator:
    """Generate structured literature reviews."""

    def __init__(self, db=None):
        self.db = db

    def generate(
        self,
        topic: str,
        max_papers: int = 50,
        depth: str = "full",
        sections: List[str] = None,
    ) -> LiteratureReview:
        """
        Generate a literature review for the topic.

        Args:
            topic: Research topic
            max_papers: Maximum papers to analyze
            depth: "short" or "full"
            sections: Specific sections to generate

        Returns:
            LiteratureReview with structured content
        """
        # 1. Collect papers
        papers = self._collect_papers(topic, max_papers)

        # 2. Classify into streams
        streams = self._classify_streams(papers)

        # 3. Detect controversies
        controversies = self._detect_controversies(streams)

        # 4. Build timeline
        timeline = self._build_timeline(papers)

        # 5. Identify open problems
        open_problems = self._identify_gaps(papers, streams)

        # 6. Generate structured sections
        review_sections = self._generate_sections(
            topic, streams, controversies, timeline, open_problems, depth
        )

        return LiteratureReview(
            topic=topic,
            streams=streams,
            controversies=controversies,
            timeline=timeline,
            open_problems=open_problems,
            sections=review_sections,
        )

    def _collect_papers(self, topic: str, max_papers: int) -> List[Any]:
        """Collect relevant papers from database."""
        if not self.db:
            return []
        rows, _ = self.db.search_papers(topic, limit=max_papers)
        return rows

    def _classify_streams(self, papers: List[Any]) -> List[ResearchStream]:
        """Classify papers into research streams."""
        streams: dict = {}

        for paper in papers:
            text = (getattr(paper, 'title', '') + " " +
                    getattr(paper, 'abstract', '')).lower()

            if any(k in text for k in ['retrieval', 'retriever', 'search', 'index']):
                stream_name = "检索增强型"
            elif any(k in text for k in ['generation', 'generator', 'decoder', 'llm', 'gpt']):
                stream_name = "生成优化型"
            elif any(k in text for k in ['hybrid', 'fusion', 'combine', 'ensemble']):
                stream_name = "混合方法"
            elif any(k in text for k in ['fine-tun', 'tuning', 'adaptation', 'transfer']):
                stream_name = "适配优化型"
            else:
                stream_name = "其他方法"

            if stream_name not in streams:
                streams[stream_name] = ResearchStream(
                    name=stream_name,
                    papers=[],
                    methods=[],
                    key_contributions=[],
                )
            streams[stream_name].papers.append(getattr(paper, 'uid', ''))

        return list(streams.values())

    def _detect_controversies(self, streams: List[ResearchStream]) -> List[Controversy]:
        """Detect controversies between streams."""
        controversies = []

        stream_names = [s.name for s in streams]

        # Efficiency vs quality trade-off
        if "检索增强型" in stream_names and "生成优化型" in stream_names:
            controversies.append(Controversy(
                topic="效率 vs 质量",
                stream_a="检索增强型",
                stream_b="生成优化型",
                position_a="检索提供外部知识，减少生成参数",
                position_b="端到端训练，知识内化",
                papers=[],
            ))

        # Hybrid vs specialized
        if "混合方法" in stream_names and len(streams) > 1:
            controversies.append(Controversy(
                topic="通用性 vs 专用性",
                stream_a="混合方法",
                stream_b="专用方法",
                position_a="融合多种技术，追求通用性",
                position_b="针对特定场景优化，追求性能",
                papers=[],
            ))

        return controversies

    def _build_timeline(self, papers: List[Any]) -> List[Tuple[int, str]]:
        """Build evolution timeline."""
        timeline: List[Tuple[int, str]] = []

        for paper in papers:
            year = getattr(paper, 'year', None) or 2020
            title = getattr(paper, 'title', '')[:50]
            if year and title:
                timeline.append((int(year), title))

        timeline.sort(key=lambda x: x[0])
        return timeline[:20]

    def _identify_gaps(self, papers: List[Any], streams: List[ResearchStream]) -> List[str]:
        """Identify open problems and gaps."""
        gaps = []
        stream_names = [s.name for s in streams]

        # Check for underexplored combinations
        if "检索增强型" in stream_names and "生成优化型" not in stream_names:
            gaps.append("检索增强与生成优化的结合尚未充分探索")

        # Generic gaps based on paper count
        if len(papers) < 10:
            gaps.append("该领域论文数量较少，研究深度有限")
        if len(streams) < 2:
            gaps.append("领域方法单一，缺乏方法多样性")

        # Common open problems
        gaps.extend([
            "长文档场景下的检索效率问题",
            "检索结果与生成质量的一致性保证",
            "跨领域知识迁移的有效性评估",
        ])

        return gaps[:5]

    def _generate_sections(
        self,
        topic: str,
        streams: List[ResearchStream],
        controversies: List[Controversy],
        timeline: List[Tuple[int, str]],
        open_problems: List[str],
        depth: str,
    ) -> List[ReviewSection]:
        """Generate structured review sections."""
        sections = []

        # Overview
        sections.append(ReviewSection(
            title="概述",
            content=f"本综述覆盖 {topic} 领域的关键研究，"
                    f"涉及 {len(streams)} 个主要流派。",
        ))

        # Method Streams
        if streams:
            stream_content = "\n".join([
                f"### {s.name}\n"
                f"- 论文数: {len(s.papers)}\n"
                f"- 代表方法: {', '.join(s.methods[:3]) or '待识别'}"
                for s in streams
            ])
            sections.append(ReviewSection(
                title="方法流派",
                content=stream_content,
            ))

        # Controversies
        if controversies:
            controversy_content = "\n".join([
                f"### {c.topic}\n"
                f"- {c.stream_a}观点: {c.position_a}\n"
                f"- {c.stream_b}观点: {c.position_b}"
                for c in controversies
            ])
            sections.append(ReviewSection(
                title="核心争论",
                content=controversy_content,
            ))

        # Timeline (short mode skips this)
        if timeline and depth == "full":
            timeline_content = "\n".join([
                f"- {year}: {event}"
                for year, event in timeline[:10]
            ])
            sections.append(ReviewSection(
                title="演化脉络",
                content=timeline_content,
            ))

        # Open Problems
        if open_problems:
            problems_content = "\n".join([
                f"- {i+1}. {p}"
                for i, p in enumerate(open_problems)
            ])
            sections.append(ReviewSection(
                title="待解决问题",
                content=problems_content,
            ))

        return sections

    def render_markdown(self, review: LiteratureReview) -> str:
        """Render review as Markdown."""
        lines = [
            f"# {review.topic} 文献综述",
            "",
        ]

        for section in review.sections:
            lines.append(f"## {section.title}")
            lines.append(section.content)
            lines.append("")

        return '\n'.join(lines)

    def render_json(self, review: LiteratureReview) -> str:
        """Render review as JSON."""
        import json
        return json.dumps({
            "topic": review.topic,
            "streams": [
                {"name": s.name, "paper_count": len(s.papers)}
                for s in review.streams
            ],
            "controversies": [
                {"topic": c.topic, "sides": [c.stream_a, c.stream_b]}
                for c in review.controversies
            ],
            "open_problems": review.open_problems,
        }, ensure_ascii=False, indent=2)
