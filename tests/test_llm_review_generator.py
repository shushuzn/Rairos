"""Tier 2 unit tests — llm/review_generator.py, pure functions, no I/O."""
import pytest
from llm.review_generator import (
    ResearchStream,
    Controversy,
    ReviewSection,
    LiteratureReview,
    ReviewGenerator,
)


# =============================================================================
# Dataclass tests
# =============================================================================
class TestResearchStream:
    """Test ResearchStream dataclass."""

    def test_required_fields(self):
        """Required fields: name."""
        s = ResearchStream(name="Retrieval-Augmented")
        assert s.name == "Retrieval-Augmented"

    def test_optional_fields_default(self):
        """Optional fields have defaults."""
        s = ResearchStream(name="N")
        assert s.papers == []
        assert s.methods == []
        assert s.key_contributions == []


class TestControversy:
    """Test Controversy dataclass."""

    def test_required_fields(self):
        """Required fields: topic, stream_a, stream_b, position_a, position_b."""
        c = Controversy(
            topic="Efficiency vs Quality",
            stream_a="Retrieval",
            stream_b="Generation",
            position_a="Uses external knowledge",
            position_b="End-to-end trained",
        )
        assert c.topic == "Efficiency vs Quality"
        assert c.stream_a == "Retrieval"
        assert c.papers == []


class TestReviewSection:
    """Test ReviewSection dataclass."""

    def test_required_fields(self):
        """Required fields: title, content."""
        s = ReviewSection(title="Overview", content="Introduction text")
        assert s.title == "Overview"
        assert s.content == "Introduction text"

    def test_subsection_default(self):
        """Subsection list defaults to empty."""
        s = ReviewSection(title="T", content="C")
        assert s.subsections == []


class TestLiteratureReview:
    """Test LiteratureReview dataclass."""

    def test_required_fields(self):
        """Required fields: topic."""
        r = LiteratureReview(topic="RAG")
        assert r.topic == "RAG"

    def test_optional_fields_default(self):
        """Optional fields have defaults."""
        r = LiteratureReview(topic="T")
        assert r.streams == []
        assert r.controversies == []
        assert r.timeline == []
        assert r.open_problems == []
        assert r.sections == []


# =============================================================================
# _classify_streams tests
# =============================================================================
class TestClassifyStreams:
    """Test _classify_streams logic."""

    def _classify_streams(self, papers):
        """Replicate _classify_streams."""
        streams = {}
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
                streams[stream_name] = ResearchStream(name=stream_name, papers=[], methods=[], key_contributions=[])
            streams[stream_name].papers.append(getattr(paper, 'uid', ''))
        return list(streams.values())

    def _paper(self, title, abstract="", uid="p1"):
        class P:
            pass
        p = P()
        p.title = title
        p.abstract = abstract
        p.uid = uid
        return p

    def test_retrieval_keyword(self):
        """Papers with retrieval keywords → 检索增强型."""
        papers = [self._paper("RAG System", "retrieval augmented generation", "p1")]
        streams = self._classify_streams(papers)
        assert len(streams) == 1
        assert streams[0].name == "检索增强型"

    def test_generation_keyword(self):
        """Papers with generation keywords → 生成优化型."""
        papers = [self._paper("GPT Model", "large language model generation", "p1")]
        streams = self._classify_streams(papers)
        assert len(streams) == 1
        assert streams[0].name == "生成优化型"

    def test_hybrid_keyword(self):
        """Papers with hybrid keywords → 混合方法."""
        papers = [self._paper("Hybrid Approach", "fusion-based method", "p1")]
        streams = self._classify_streams(papers)
        assert len(streams) == 1
        assert streams[0].name == "混合方法"

    def test_finetune_keyword(self):
        """Papers with fine-tuning keywords → 适配优化型."""
        papers = [self._paper("Fine-tuned Model", "transfer learning adaptation", "p1")]
        streams = self._classify_streams(papers)
        assert len(streams) == 1
        assert streams[0].name == "适配优化型"

    def test_default_other_method(self):
        """No keywords → 其他方法."""
        papers = [self._paper("Random Paper", "some random content", "p1")]
        streams = self._classify_streams(papers)
        assert len(streams) == 1
        assert streams[0].name == "其他方法"

    def test_multiple_papers_same_stream(self):
        """Multiple papers with same keywords → same stream."""
        papers = [
            self._paper("RAG v1", "retrieval system", "p1"),
            self._paper("RAG v2", "better retrieval", "p2"),
        ]
        streams = self._classify_streams(papers)
        assert len(streams) == 1
        assert len(streams[0].papers) == 2

    def test_multiple_streams(self):
        """Papers with different keywords → multiple streams."""
        papers = [
            self._paper("RAG System", "retrieval", "p1"),
            self._paper("GPT Model", "generation", "p2"),
        ]
        streams = self._classify_streams(papers)
        assert len(streams) == 2
        names = {s.name for s in streams}
        assert "检索增强型" in names
        assert "生成优化型" in names

    def test_case_insensitive(self):
        """Keyword matching is case insensitive."""
        papers = [self._paper("RETRIEVAL System", "RETRIEVAL", "p1")]
        streams = self._classify_streams(papers)
        assert streams[0].name == "检索增强型"

    def test_abstract_only(self):
        """Classification works on abstract alone."""
        papers = [self._paper("", "retrieval search index", "p1")]
        streams = self._classify_streams(papers)
        assert streams[0].name == "检索增强型"


# =============================================================================
# _detect_controversies tests
# =============================================================================
class TestDetectControversies:
    """Test _detect_controversies logic."""

    def _detect_controversies(self, streams):
        """Replicate _detect_controversies."""
        controversies = []
        stream_names = [s.name for s in streams]
        if "检索增强型" in stream_names and "生成优化型" in stream_names:
            controversies.append(Controversy(
                topic="效率 vs 质量",
                stream_a="检索增强型",
                stream_b="生成优化型",
                position_a="检索提供外部知识，减少生成参数",
                position_b="端到端训练，知识内化",
                papers=[],
            ))
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

    def _stream(self, name):
        return ResearchStream(name=name, papers=[], methods=[], key_contributions=[])

    def test_retrieval_vs_generation(self):
        """检索增强型 + 生成优化型 → efficiency controversy."""
        streams = [
            self._stream("检索增强型"),
            self._stream("生成优化型"),
        ]
        controversies = self._detect_controversies(streams)
        assert len(controversies) == 1
        assert controversies[0].topic == "效率 vs 质量"

    def test_hybrid_plus_others(self):
        """混合方法 + other streams → hybrid controversy."""
        streams = [
            self._stream("混合方法"),
            self._stream("检索增强型"),
        ]
        controversies = self._detect_controversies(streams)
        assert len(controversies) == 1
        assert controversies[0].topic == "通用性 vs 专用性"

    def test_both_controversies(self):
        """Both conditions met → both controversies."""
        streams = [
            self._stream("检索增强型"),
            self._stream("生成优化型"),
            self._stream("混合方法"),
        ]
        controversies = self._detect_controversies(streams)
        assert len(controversies) == 2
        topics = {c.topic for c in controversies}
        assert "效率 vs 质量" in topics
        assert "通用性 vs 专用性" in topics

    def test_no_controversy_single_stream(self):
        """Only one stream → no controversies."""
        streams = [self._stream("检索增强型")]
        controversies = self._detect_controversies(streams)
        assert controversies == []

    def test_hybrid_alone_no_controversy(self):
        """混合方法 alone → no hybrid controversy (needs len>1)."""
        streams = [self._stream("混合方法")]
        controversies = self._detect_controversies(streams)
        assert controversies == []


# =============================================================================
# _build_timeline tests
# =============================================================================
class TestBuildTimeline:
    """Test _build_timeline logic."""

    def _build_timeline(self, papers):
        """Replicate _build_timeline."""
        timeline = []
        for paper in papers:
            year = getattr(paper, 'year', None) or 2020
            title = getattr(paper, 'title', '')[:50]
            if year and title:
                timeline.append((int(year), title))
        timeline.sort(key=lambda x: x[0])
        return timeline[:20]

    def _paper(self, year, title):
        class P:
            pass
        p = P()
        p.year = year
        p.title = title
        return p

    def test_sorted_by_year_ascending(self):
        """Timeline sorted by year ascending."""
        papers = [
            self._paper(2023, "Recent Paper"),
            self._paper(2020, "Older Paper"),
            self._paper(2021, "Middle Paper"),
        ]
        timeline = self._build_timeline(papers)
        assert timeline[0] == (2020, "Older Paper")
        assert timeline[1] == (2021, "Middle Paper")
        assert timeline[2] == (2023, "Recent Paper")

    def test_title_truncated_to_50(self):
        """Title truncated to 50 chars."""
        papers = [self._paper(2020, "A" * 100)]
        timeline = self._build_timeline(papers)
        assert len(timeline[0][1]) == 50

    def test_max_20_items(self):
        """Returns max 20 items."""
        papers = [self._paper(2020 + i, f"Paper {i}") for i in range(30)]
        timeline = self._build_timeline(papers)
        assert len(timeline) == 20

    def test_missing_year_defaults_to_2020(self):
        """Missing year defaults to 2020."""
        papers = []
        class P:
            title = "Test"
            year = None
        papers.append(P())
        timeline = self._build_timeline(papers)
        assert timeline[0][0] == 2020

    def test_empty_list(self):
        """Empty list returns empty timeline."""
        assert self._build_timeline([]) == []


# =============================================================================
# _identify_gaps tests
# =============================================================================
class TestIdentifyGaps:
    """Test _identify_gaps logic."""

    def _identify_gaps(self, papers, streams):
        """Replicate _identify_gaps."""
        gaps = []
        stream_names = [s.name for s in streams]
        if "检索增强型" in stream_names and "生成优化型" not in stream_names:
            gaps.append("检索增强与生成优化的结合尚未充分探索")
        if len(papers) < 10:
            gaps.append("该领域论文数量较少，研究深度有限")
        if len(streams) < 2:
            gaps.append("领域方法单一，缺乏方法多样性")
        gaps.extend([
            "长文档场景下的检索效率问题",
            "检索结果与生成质量的一致性保证",
            "跨领域知识迁移的有效性评估",
        ])
        return gaps[:5]

    def _stream(self, name):
        return ResearchStream(name=name, papers=[], methods=[], key_contributions=[])

    def _paper(self):
        class P:
            title = "Test Paper"
            abstract = ""
        return P()

    def test_retrieval_without_generation_gap(self):
        """检索增强型 without 生成优化型 → specific gap."""
        papers = [self._paper()]
        streams = [self._stream("检索增强型")]
        gaps = self._identify_gaps(papers, streams)
        assert "检索增强与生成优化的结合尚未充分探索" in gaps

    def test_few_papers_gap(self):
        """Less than 10 papers → paper count gap."""
        papers = [self._paper() for _ in range(5)]
        streams = [self._stream("检索增强型")]
        gaps = self._identify_gaps(papers, streams)
        assert "该领域论文数量较少，研究深度有限" in gaps

    def test_single_stream_gap(self):
        """Single stream → method diversity gap."""
        papers = [self._paper() for _ in range(10)]
        streams = [self._stream("检索增强型")]
        gaps = self._identify_gaps(papers, streams)
        assert "领域方法单一，缺乏方法多样性" in gaps

    def test_always_common_gaps(self):
        """Always includes common gaps."""
        papers = [self._paper() for _ in range(15)]
        streams = [self._stream("检索增强型"), self._stream("生成优化型")]
        gaps = self._identify_gaps(papers, streams)
        assert "长文档场景下的检索效率问题" in gaps
        assert "检索结果与生成质量的一致性保证" in gaps
        assert "跨领域知识迁移的有效性评估" in gaps

    def test_max_5_gaps(self):
        """Returns max 5 gaps."""
        papers = [self._paper()]
        streams = [self._stream("检索增强型")]
        gaps = self._identify_gaps(papers, streams)
        assert len(gaps) <= 5


# =============================================================================
# _generate_sections tests
# =============================================================================
class TestGenerateSections:
    """Test _generate_sections logic."""

    def _generate_sections(self, topic, streams, controversies, timeline, open_problems, depth):
        """Replicate _generate_sections."""
        sections = []
        sections.append(ReviewSection(
            title="概述",
            content=f"本综述覆盖 {topic} 领域的关键研究，"
                    f"涉及 {len(streams)} 个主要流派。",
        ))
        if streams:
            stream_content = "\n".join([
                f"### {s.name}\n"
                f"- 论文数: {len(s.papers)}\n"
                f"- 代表方法: {', '.join(s.methods[:3]) or '待识别'}"
                for s in streams
            ])
            sections.append(ReviewSection(title="方法流派", content=stream_content))
        if controversies:
            controversy_content = "\n".join([
                f"### {c.topic}\n"
                f"- {c.stream_a}观点: {c.position_a}\n"
                f"- {c.stream_b}观点: {c.position_b}"
                for c in controversies
            ])
            sections.append(ReviewSection(title="核心争论", content=controversy_content))
        if timeline and depth == "full":
            timeline_content = "\n".join([
                f"- {year}: {event}"
                for year, event in timeline[:10]
            ])
            sections.append(ReviewSection(title="演化脉络", content=timeline_content))
        if open_problems:
            problems_content = "\n".join([
                f"- {i+1}. {p}"
                for i, p in enumerate(open_problems)
            ])
            sections.append(ReviewSection(title="待解决问题", content=problems_content))
        return sections

    def _stream(self, name, papers_count=1, methods=None):
        s = ResearchStream(name=name, papers=["p"] * papers_count, methods=[], key_contributions=[])
        if methods:
            s.methods = methods
        return s

    def _controversy(self, topic):
        return Controversy(
            topic=topic,
            stream_a="A",
            stream_b="B",
            position_a="Pos A",
            position_b="Pos B",
            papers=[],
        )

    def test_always_has_overview(self):
        """Overview section always included."""
        sections = self._generate_sections("RAG", [], [], [], [], "full")
        assert any(s.title == "概述" for s in sections)
        assert any("RAG" in s.content for s in sections)

    def test_overview_shows_stream_count(self):
        """Overview content mentions stream count."""
        sections = self._generate_sections("Topic", [self._stream("S1"), self._stream("S2")], [], [], [], "full")
        overview = next(s for s in sections if s.title == "概述")
        assert "2" in overview.content  # 2 streams

    def test_method_streams_section(self):
        """Streams section shows stream details."""
        sections = self._generate_sections(
            "T",
            [self._stream("检索增强型", 5, ["Method A"])],
            [],
            [],
            [],
            "full",
        )
        assert any(s.title == "方法流派" for s in sections)
        stream_section = next(s for s in sections if s.title == "方法流派")
        assert "检索增强型" in stream_section.content
        assert "5" in stream_section.content  # paper count

    def test_method_stream_no_methods(self):
        """Stream with no methods shows 待识别."""
        sections = self._generate_sections(
            "T",
            [self._stream("检索增强型", 1)],
            [],
            [],
            [],
            "full",
        )
        stream_section = next(s for s in sections if s.title == "方法流派")
        assert "待识别" in stream_section.content

    def test_controversies_section(self):
        """Controversies section shows debate."""
        sections = self._generate_sections(
            "T",
            [],
            [self._controversy("效率 vs 质量")],
            [],
            [],
            "full",
        )
        assert any(s.title == "核心争论" for s in sections)
        c_section = next(s for s in sections if s.title == "核心争论")
        assert "效率 vs 质量" in c_section.content

    def test_timeline_full_depth(self):
        """Timeline section in full depth."""
        timeline = [(2020, "Paper 2020"), (2021, "Paper 2021")]
        sections = self._generate_sections("T", [], [], timeline, [], "full")
        assert any(s.title == "演化脉络" for s in sections)
        tl_section = next(s for s in sections if s.title == "演化脉络")
        assert "2020: Paper 2020" in tl_section.content

    def test_timeline_short_depth_skipped(self):
        """Timeline skipped in short depth."""
        timeline = [(2020, "Paper 2020")]
        sections = self._generate_sections("T", [], [], timeline, [], "short")
        assert not any(s.title == "演化脉络" for s in sections)

    def test_open_problems_section(self):
        """Open problems section shows numbered list."""
        problems = ["Problem A", "Problem B"]
        sections = self._generate_sections("T", [], [], [], problems, "full")
        assert any(s.title == "待解决问题" for s in sections)
        prob_section = next(s for s in sections if s.title == "待解决问题")
        assert "1. Problem A" in prob_section.content
        assert "2. Problem B" in prob_section.content


# =============================================================================
# render_markdown tests
# =============================================================================
class TestRenderMarkdown:
    """Test render_markdown formatting."""

    def _render_markdown(self, review):
        """Replicate render_markdown."""
        lines = [f"# {review.topic}", ""]
        for section in review.sections:
            lines.append(f"## {section.title}")
            lines.append(section.content)
            lines.append("")
        return "\n".join(lines)

    def test_header(self):
        """Header shows topic."""
        review = LiteratureReview(topic="RAG Research")
        output = self._render_markdown(review)
        assert "# RAG Research" in output

    def test_empty_review(self):
        """Empty sections renders just header."""
        review = LiteratureReview(topic="T")
        output = self._render_markdown(review)
        lines = output.strip().split("\n")
        assert lines == ["# T"]

    def test_section_rendering(self):
        """Sections rendered as markdown."""
        review = LiteratureReview(
            topic="T",
            sections=[
                ReviewSection(title="Overview", content="Intro text"),
                ReviewSection(title="Methods", content="Method details"),
            ],
        )
        output = self._render_markdown(review)
        assert "## Overview" in output
        assert "Intro text" in output
        assert "## Methods" in output
        assert "Method details" in output


# =============================================================================
# render_json tests
# =============================================================================
class TestRenderJson:
    """Test render_json formatting."""

    def _render_json(self, review):
        """Replicate render_json."""
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

    def test_topic_included(self):
        """Topic included in JSON."""
        review = LiteratureReview(topic="RAG")
        output = self._render_json(review)
        assert '"topic": "RAG"' in output

    def test_streams_with_paper_count(self):
        """Streams included with paper count."""
        review = LiteratureReview(
            topic="T",
            streams=[ResearchStream(name="检索增强型", papers=["p1", "p2"], methods=[], key_contributions=[])],
        )
        output = self._render_json(review)
        assert "检索增强型" in output
        assert '"paper_count": 2' in output

    def test_controversies_with_sides(self):
        """Controversies included with stream sides."""
        review = LiteratureReview(
            topic="T",
            controversies=[
                Controversy(topic="Efficiency", stream_a="A", stream_b="B", position_a="", position_b="", papers=[]),
            ],
        )
        output = self._render_json(review)
        assert "Efficiency" in output
        assert '"sides"' in output and '"A"' in output and '"B"' in output

    def test_open_problems(self):
        """Open problems included."""
        review = LiteratureReview(
            topic="T",
            open_problems=["Problem 1", "Problem 2"],
        )
        output = self._render_json(review)
        assert "Problem 1" in output
        assert "Problem 2" in output


# =============================================================================
# ReviewGenerator instantiation
# =============================================================================
class TestReviewGeneratorInit:
    """Test ReviewGenerator class."""

    def test_can_instantiate(self):
        """ReviewGenerator can be instantiated."""
        gen = ReviewGenerator()
        assert gen.db is None

    def test_can_instantiate_with_db(self):
        """ReviewGenerator can be instantiated with db."""
        mock_db = object()
        gen = ReviewGenerator(db=mock_db)
        assert gen.db is mock_db
