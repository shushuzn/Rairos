"""
Research Story Weaver: Generate narrative understanding from research papers.

研究故事编织器：从论文中生成叙事性理解。

核心算法：
1. 叙事提取：识别核心贡献、转折点、矛盾
2. 关系图构建：论文间的逻辑关系
3. 故事生成：时间线编排 + LLM 叙事
4. 对比模式：两条故事线的分歧与共识
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from enum import Enum
from typing import Optional, List, Dict, Any, Tuple
from collections import defaultdict

# Optional LLM import
try:
    from llm.chat import call_llm_chat_completions
    LLM_AVAILABLE = True
except ImportError:
    LLM_AVAILABLE = False


class NarrativeRole(Enum):
    """Role a paper plays in the narrative."""
    PROTAGONIST = "protagonist"   # 主角 - 主流方法
    ANTAGONIST = "antagonist"     # 反派 - 待解决的问题
    TURNING_POINT = "turning_point"  # 转折点 - 突破性工作
    DIVERGENCE = "divergence"    # 分叉 - 产生新方向
    SYNTHESIS = "synthesis"      # 综合 - 融合多种方法


class RelationshipType(Enum):
    """Relationship between papers."""
    INHERITS = "inherits"        # 继承
    EXTENDS = "extends"          # 扩展
    CONTRASTS = "contrasts"     # 对比
    CONTRADICTS = "contradicts" # 矛盾
    SYNTHESIZES = "synthesizes" # 综合
    CITES = "cites"             # 引用


@dataclass
class PaperNarrative:
    """Narrative element extracted from a paper."""
    paper_id: str
    title: str
    year: int
    role: NarrativeRole
    core_contribution: str  # 核心贡献
    key_insight: str  # 关键洞察
    turning_point_type: str = ""  # 转折类型
    conflicts_with: List[str] = field(default_factory=list)  # 与哪些论文冲突


@dataclass
class Chapter:
    """A chapter in the research story."""
    title: str
    time_range: Tuple[int, int]
    papers: List[PaperNarrative]
    summary: str = ""
    theme: str = ""  # 章节主题


@dataclass
class Relationship:
    """Relationship between two papers."""
    from_paper: str
    to_paper: str
    relationship: RelationshipType
    description: str = ""


@dataclass
class StoryResult:
    """Complete story weaving result."""
    topic: str
    chapters: List[Chapter] = field(default_factory=list)
    relationships: List[Relationship] = field(default_factory=list)
    protagonist_arc: str = ""  # 主角发展弧线
    contradictions: List[Tuple[str, str]] = field(default_factory=list)  # 矛盾对
    themes: List[str] = field(default_factory=list)  # 核心主题
    summary: str = ""  # 叙事总结


class StoryWeaver:
    """Generate narrative understanding from research papers."""

    # Key narrative patterns
    TURNING_POINT_PATTERNS = [
        r'breakthrough|revolution|paradigm shift|game changer|state-of-the-art',
        r'outperforms?|surpasses?|exceeds? previous',
        r'first to|for the first time|introduces? a new',
        r'despite|however|but|nevertheless|contradict',
    ]

    DIVERGENCE_PATTERNS = [
        r'alternative|instead|rather|unlike|contrast',
        r'different approach|different from|diverges',
        r'on the other hand|meanwhile|conversely',
    ]

    def __init__(self, db=None):
        self.db = db

    def weave(
        self,
        topic: str,
        use_llm: bool = True,
        api_key: Optional[str] = None,
        base_url: Optional[str] = None,
        model: Optional[str] = None,
        max_papers: int = 20,
    ) -> StoryResult:
        """
        Weave a research story from papers.

        Args:
            topic: Research topic/keyword
            use_llm: Whether to use LLM for narrative generation
            api_key: LLM API key
            base_url: LLM API base URL
            model: Model name
            max_papers: Maximum papers to analyze

        Returns:
            StoryResult with narrative structure
        """
        # 1. Collect papers
        papers = self._collect_papers(topic, max_papers)
        if not papers:
            return self._empty_result(topic)

        # 2. Extract narrative elements
        narratives = self._extract_narratives(papers)

        # 3. Build relationship graph
        relationships = self._build_relationships(narratives)

        # 4. Identify chapters by time period
        chapters = self._organize_chapters(narratives)

        # 5. Find contradictions
        contradictions = self._find_contradictions(narratives)

        # 6. Identify core themes
        themes = self._identify_themes(narratives)

        result = StoryResult(
            topic=topic,
            chapters=chapters,
            relationships=relationships,
            contradictions=contradictions,
            themes=themes,
        )

        # 7. Generate narrative summaries
        if use_llm and LLM_AVAILABLE:
            result = self._enhance_with_llm(result, papers, api_key, base_url, model)

        # 8. Generate overall summary
        result.summary = self._generate_summary(result)

        return result

    def compare(
        self,
        topic_a: str,
        topic_b: str,
        use_llm: bool = True,
    ) -> str:
        """
        Compare two research storylines.

        Args:
            topic_a: First research topic
            topic_b: Second research topic
            use_llm: Whether to use LLM

        Returns:
            Comparison narrative
        """
        story_a = self.weave(topic_a, use_llm=use_llm)
        story_b = self.weave(topic_b, use_llm=use_llm)

        return self._generate_comparison(story_a, story_b)

    def _collect_papers(self, topic: str, max_papers: int) -> List[Dict]:
        """Collect papers from DB."""
        if not self.db:
            return []

        papers = []
        try:
            rows, _ = self.db.search_papers(topic, limit=max_papers)
            for row in rows:
                paper = {
                    "id": getattr(row, 'id', ''),
                    "title": getattr(row, 'title', '') or '',
                    "abstract": getattr(row, 'abstract', '') or '',
                    "year": getattr(row, 'year', 0) or 0,
                    "citations": getattr(row, 'citations', 0) or 0,
                }
                if paper["year"] > 2000:
                    papers.append(paper)
        except Exception:
            pass

        # Sort by citations for importance
        papers.sort(key=lambda x: x.get("citations", 0), reverse=True)
        return papers[:max_papers]

    def _extract_narratives(self, papers: List[Dict]) -> List[PaperNarrative]:
        """Extract narrative elements from papers."""
        narratives = []

        for paper in papers:
            text = f"{paper.get('title', '')} {paper.get('abstract', '')}".lower()
            year = paper.get('year', 0)
            title = paper.get('title', '')[:60]

            # Determine role
            role = self._determine_role(text, year)

            # Extract core contribution
            contribution = self._extract_contribution(paper)

            # Extract key insight
            insight = self._extract_insight(text)

            # Check for turning point
            turning_type = self._detect_turning_point(text)

            narratives.append(PaperNarrative(
                paper_id=paper.get('id', ''),
                title=title,
                year=year,
                role=role,
                core_contribution=contribution,
                key_insight=insight,
                turning_point_type=turning_type,
            ))

        return narratives

    def _determine_role(self, text: str, year: int) -> NarrativeRole:
        """Determine narrative role of a paper."""
        # High impact early papers are often protagonists
        if year <= 2018:
            if any(p in text for p in ['attention is all you need', 'bert', 'gpt']):
                return NarrativeRole.PROTAGONIST

        # Check for turning points
        if any(re.search(p, text) for p in self.TURNING_POINT_PATTERNS):
            return NarrativeRole.TURNING_POINT

        # Check for divergences
        if any(re.search(p, text) for p in self.DIVERGENCE_PATTERNS):
            return NarrativeRole.DIVERGENCE

        return NarrativeRole.PROTAGONIST

    def _extract_contribution(self, paper: Dict) -> str:
        """Extract core contribution from paper."""
        title = paper.get('title', '')
        abstract = paper.get('abstract', '')[:200]

        # Try to extract from abstract
        contribution_patterns = [
            r'we (?:propose|present|introduce|develop) (.+?)\.',
            r'this paper (.+?)\.',
            r'we show that (.+?)\.',
            r'(?:propose|present|introduce) (.+?)(?:\.|$)',
        ]

        for pattern in contribution_patterns:
            match = re.search(pattern, abstract.lower())
            if match:
                return match.group(1).strip()[:100]

        return title[:60] if title else "Unknown contribution"

    def _extract_insight(self, text: str) -> str:
        """Extract key insight from text."""
        insight_patterns = [
            r'(?:key|central|core) insight:?\s*(.+?)(?:\.|$)',
            r'we find that (.+?)(?:\.|$)',
            r'discover(?:y|ed) that (.+?)(?:\.|$)',
            r'((?:the|this) .+? is(?: all| the) .+?)(?:\.|$)',
        ]

        for pattern in insight_patterns:
            match = re.search(pattern, text.lower())
            if match:
                return match.group(1).strip()[:80]

        return "Provides new approach to the problem"

    def _detect_turning_point(self, text: str) -> str:
        """Detect type of turning point."""
        if 'breakthrough' in text or 'revolution' in text:
            return "颠覆性突破"
        if 'paradigm shift' in text:
            return "范式转变"
        if 'state-of-the-art' in text or 'sota' in text:
            return "性能突破"
        if 'first' in text and 'time' in text:
            return "首次实现"
        return ""

    def _build_relationships(
        self,
        narratives: List[PaperNarrative],
    ) -> List[Relationship]:
        """Build relationship graph between papers."""
        relationships = []
        texts = {n.paper_id: n.title.lower() for n in narratives}

        for i, narrative in enumerate(narratives):
            for j, other in enumerate(narratives[i + 1:], i + 1):
                rel_type, desc = self._infer_relationship(narrative, other)
                if rel_type:
                    relationships.append(Relationship(
                        from_paper=narrative.paper_id,
                        to_paper=other.paper_id,
                        relationship=rel_type,
                        description=desc,
                    ))

        return relationships

    def _infer_relationship(
        self,
        a: PaperNarrative,
        b: PaperNarrative,
    ) -> Tuple[Optional[RelationshipType], str]:
        """Infer relationship between two papers."""
        # Time-based inheritance
        if b.year > a.year:
            if 'extends' in a.title.lower() or 'building' in b.title.lower():
                return RelationshipType.EXTENDS, f"{b.year} work extends {a.year} work"

        # Inherits from foundational work
        if a.year <= 2017:
            if b.year > 2019:
                return RelationshipType.INHERITS, f"Based on foundational work from {a.year}"

        # Divergence
        if any(p in b.title.lower() for p in ['instead', 'alternative', 'rather', 'unlike']):
            return RelationshipType.DIVERGENCE, f"Proposes alternative to {a.title[:30]}..."

        # Contrast
        if any(p in a.title.lower() + b.title.lower() for p in ['vs', 'versus', '对比', '比较']):
            return RelationshipType.CONTRASTS, f"Contrasts with {a.title[:30]}..."

        return None, ""

    def _organize_chapters(
        self,
        narratives: List[PaperNarrative],
    ) -> List[Chapter]:
        """Organize papers into chapters by time period."""
        # Group by approximate time periods
        periods = defaultdict(list)
        for n in narratives:
            year = n.year
            if year < 2015:
                period = (2008, 2014)
            elif year < 2018:
                period = (2015, 2017)
            elif year < 2020:
                period = (2018, 2019)
            elif year < 2022:
                period = (2020, 2021)
            elif year < 2024:
                period = (2022, 2023)
            else:
                period = (2024, 2026)
            periods[period].append(n)

        # Create chapters
        chapters = []
        for period in sorted(periods.keys()):
            papers = periods[period]
            papers.sort(key=lambda x: x.year)

            # Chapter title based on theme
            titles = {
                (2008, 2014): "萌芽期 - Attention 机制的发现",
                (2015, 2017): "突破期 - Attention Is All You Need",
                (2018, 2019): "扩散期 - BERT 与预训练革命",
                (2020, 2021): "规模化初期 - GPT-3 的里程碑",
                (2022, 2023): "百模大战 - 开源与闭源的对抗",
                (2024, 2026): "AGI 探索 - 超越 Transformer?",
            }

            chapter = Chapter(
                title=titles.get(period, f"时期 ({period[0]}-{period[1]})"),
                time_range=period,
                papers=papers,
            )
            chapters.append(chapter)

        return chapters

    def _find_contradictions(
        self,
        narratives: List[PaperNarrative],
    ) -> List[Tuple[str, str]]:
        """Find contradictory claims in papers."""
        contradictions = []

        # Simple keyword-based contradiction detection
        efficiency_keywords = ['efficient', 'fast', 'lightweight', 'small', 'distill']
        scale_keywords = ['large', 'massive', 'scale', 'billions', 'parameters']

        for i, a in enumerate(narratives):
            for b in narratives[i + 1:]:
                a_text = a.title.lower()
                b_text = b.title.lower()

                # Efficiency vs Scale contradiction
                a_efficient = any(k in a_text for k in efficiency_keywords)
                b_scale = any(k in b_text for k in scale_keywords)
                if a_efficient and b_scale:
                    contradictions.append((a.title, b.title))

                a_scale = any(k in a_text for k in scale_keywords)
                b_efficient = any(k in b_text for k in efficiency_keywords)
                if a_scale and b_efficient:
                    contradictions.append((a.title, b.title))

        return contradictions[:5]

    def _identify_themes(self, narratives: List[PaperNarrative]) -> List[str]:
        """Identify core themes in the research story."""
        themes = []
        theme_keywords = {
            'Attention 机制': ['attention', 'self-attention', 'multi-head'],
            '预训练范式': ['pre-train', 'fine-tun', 'mask'],
            '规模化': ['scale', 'large', 'billions', 'parameters'],
            '效率优化': ['efficient', 'fast', 'distill', 'prune', 'quantize'],
            '多模态': ['multimodal', 'vision', 'image', 'text'],
            '推理能力': ['reason', 'chain-of-thought', 'cot'],
            '对齐与安全': ['align', 'rlhf', 'safety', 'value'],
        }

        all_text = ' '.join(n.title.lower() for n in narratives)
        for theme, keywords in theme_keywords.items():
            if any(k in all_text for k in keywords):
                themes.append(theme)

        return themes[:5]

    def _enhance_with_llm(
        self,
        result: StoryResult,
        papers: List[Dict],
        api_key: Optional[str],
        base_url: Optional[str],
        model: Optional[str],
    ) -> StoryResult:
        """Use LLM to enhance narrative generation."""
        import os

        api_key = api_key or os.getenv("OPENAI_API_KEY", "")
        if not api_key:
            return result

        # Generate chapter summaries
        for chapter in result.chapters:
            if not chapter.papers:
                continue

            paper_texts = '\n'.join([
                f"- {p.title} ({p.year}): {p.core_contribution}"
                for p in chapter.papers[:5]
            ])

            system_prompt = """为科研故事的一个章节生成简短的总结。
格式：主题 + 2-3句关键内容概括"""

            user_prompt = f"""章节: {chapter.title}
论文:
{paper_texts}

请生成章节总结："""

            try:
                response = call_llm_chat_completions(
                    base_url=base_url or os.getenv("OPENAI_BASE_URL", "https://api.openai.com/v1"),
                    api_key=api_key,
                    model=model or os.getenv("DEFAULT_LLM_MODEL", "gpt-4o-mini"),
                    system_prompt=system_prompt,
                    user_prompt=user_prompt,
                )
                chapter.summary = response.strip()
            except Exception:
                pass

        return result

    def _generate_summary(self, result: StoryResult) -> str:
        """Generate overall story summary."""
        if not result.chapters:
            return "暂无足够数据生成故事"

        # Build summary from themes and protagonist arc
        themes = ', '.join(result.themes[:3]) if result.themes else '技术演进'

        summary = f"""《{result.topic}》的演进是一场关于{themes}的探索。
从 {result.chapters[0].time_range[0]} 年的开创性工作，到 {result.chapters[-1].time_range[-1]} 年的最新突破，
领域经历了从理论验证到工程化应用，从单一模型到多元化生态的转变。
"""

        if result.contradictions:
            summary += f"\n核心张力: 发现 {len(result.contradictions)} 个主要矛盾点，"
            summary += "体现了领域内不同技术路线的竞争与融合。"

        return summary

    def _generate_comparison(self, story_a: StoryResult, story_b: StoryResult) -> str:
        """Generate comparison between two stories."""
        lines = [
            f"📖 故事线对比: {story_a.topic} vs {story_b.topic}",
            "",
        ]

        # Compare themes
        shared_themes = set(story_a.themes) & set(story_b.themes)
        if shared_themes:
            lines.append(f"🔗 共同主题: {', '.join(shared_themes)}")

        # Compare time spans
        lines.append("")
        lines.append(f"📅 {story_a.topic}: {story_a.chapters[0].time_range[0]}-{story_a.chapters[-1].time_range[-1]}")
        lines.append(f"📅 {story_b.topic}: {story_b.chapters[0].time_range[0]}-{story_b.chapters[-1].time_range[-1]}")

        # Compare protagonists
        lines.append("")
        lines.append("🎭 主角发展弧线:")
        lines.append(f"  • {story_a.topic}: {story_a.protagonist_arc[:80] if story_a.protagonist_arc else '传统方法演进'}")
        lines.append(f"  • {story_b.topic}: {story_b.protagonist_arc[:80] if story_b.protagonist_arc else '新方法探索'}")

        return '\n'.join(lines)

    def _empty_result(self, topic: str) -> StoryResult:
        """Return empty result."""
        return StoryResult(topic=topic)

    def render_result(self, result: StoryResult) -> str:
        """Render story as formatted text."""
        lines = [
            f"📖 研究故事: {result.topic}",
            "",
        ]

        # Chapters
        for i, chapter in enumerate(result.chapters, 1):
            lines.append(f"第{i}章: {chapter.title}")
            lines.append(f"   时间: {chapter.time_range[0]}-{chapter.time_range[1]}")

            if chapter.summary:
                lines.append(f"   {chapter.summary}")
            else:
                # Auto-generate summary from papers
                contributions = [p.core_contribution[:50] for p in chapter.papers[:3]]
                lines.append(f"   关键贡献: {' | '.join(contributions)}")

            lines.append("")

            for paper in chapter.papers[:3]:
                role_icon = {
                    NarrativeRole.PROTAGONIST: "├─",
                    NarrativeRole.TURNING_POINT: "└─",
                    NarrativeRole.DIVERGENCE: "├─",
                    NarrativeRole.ANTAGONIST: "├─",
                    NarrativeRole.SYNTHESIS: "└─",
                }.get(paper.role, "├─")

                lines.append(f"   {role_icon} {paper.title} ({paper.year})")
                lines.append(f"   │  └─ 💡 {paper.key_insight[:60]}")

                if paper.turning_point_type:
                    lines.append(f"   │     🔥 {paper.turning_point_type}")

            lines.append("")

        # Contradictions
        if result.contradictions:
            lines.append("⚡ 核心矛盾:")
            for a, b in result.contradictions[:3]:
                lines.append(f"   • {a[:40]}...")
                lines.append(f"     ↔ {b[:40]}...")
            lines.append("")

        # Themes
        if result.themes:
            lines.append(f"🧭 核心主题: {', '.join(result.themes[:4])}")
            lines.append("")

        # Summary
        if result.summary:
            lines.append(f"📝 {result.summary}")

        return '\n'.join(lines)

    def render_mermaid(self, result: StoryResult) -> str:
        """Render story as Mermaid flowchart."""
        lines = [
            "```mermaid",
            "flowchart TD",
            f'    title["📖 {result.topic}"]',
            "",
        ]

        # Add nodes for key papers
        for i, chapter in enumerate(result.chapters):
            for paper in chapter.papers[:2]:
                node_id = f"P{paper.year}{i}"
                role_class = {
                    NarrativeRole.TURNING_POINT: "fill:#ff6b6b",
                    NarrativeRole.DIVERGENCE: "fill:#4ecdc4",
                }.get(paper.role, "fill:#ddd")

                lines.append(f'    {node_id}["{paper.title[:30]}..."]:::{paper.role.value}')
                lines.append(f'    classDef {paper.role.value} {role_class}')

        lines.append("```")
        return '\n'.join(lines)
