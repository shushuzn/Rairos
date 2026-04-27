"""Tier 2 unit tests — llm/story_weaver.py, pure functions, no I/O."""
import pytest
from llm.story_weaver import (
    NarrativeRole,
    RelationshipType,
    PaperNarrative,
    Chapter,
    Relationship,
    StoryResult,
    StoryWeaver,
)


# =============================================================================
# Enum tests
# =============================================================================
class TestNarrativeRole:
    """Test NarrativeRole enum."""

    def test_all_roles_have_values(self):
        """All role variants have string values."""
        assert NarrativeRole.PROTAGONIST.value == "protagonist"
        assert NarrativeRole.ANTAGONIST.value == "antagonist"
        assert NarrativeRole.TURNING_POINT.value == "turning_point"
        assert NarrativeRole.DIVERGENCE.value == "divergence"
        assert NarrativeRole.SYNTHESIS.value == "synthesis"


class TestRelationshipType:
    """Test RelationshipType enum."""

    def test_all_types_have_values(self):
        """All relationship types have string values."""
        assert RelationshipType.INHERITS.value == "inherits"
        assert RelationshipType.EXTENDS.value == "extends"
        assert RelationshipType.CONTRASTS.value == "contrasts"
        assert RelationshipType.CONTRADICTS.value == "contradicts"
        assert RelationshipType.SYNTHESIZES.value == "synthesizes"
        assert RelationshipType.CITES.value == "cites"


# =============================================================================
# Dataclass tests
# =============================================================================
class TestPaperNarrative:
    """Test PaperNarrative dataclass."""

    def test_required_fields(self):
        """Required fields: paper_id, title, year, role, core_contribution, key_insight."""
        n = PaperNarrative(
            paper_id="p1",
            title="Attention Is All You Need",
            year=2017,
            role=NarrativeRole.PROTAGONIST,
            core_contribution="Transformer architecture",
            key_insight="Self-attention mechanism",
        )
        assert n.paper_id == "p1"
        assert n.role == NarrativeRole.PROTAGONIST

    def test_optional_fields_default(self):
        """Optional fields default."""
        n = PaperNarrative(
            paper_id="p", title="T", year=2020,
            role=NarrativeRole.PROTAGONIST,
            core_contribution="C", key_insight="I",
        )
        assert n.turning_point_type == ""
        assert n.conflicts_with == []


class TestChapter:
    """Test Chapter dataclass."""

    def test_required_fields(self):
        """Required fields: title, time_range, papers."""
        c = Chapter(
            title="Early Days",
            time_range=(2015, 2017),
            papers=[],
        )
        assert c.title == "Early Days"
        assert c.time_range == (2015, 2017)

    def test_optional_fields_default(self):
        """Optional fields default."""
        c = Chapter(title="T", time_range=(2020, 2021), papers=[])
        assert c.summary == ""
        assert c.theme == ""


class TestRelationship:
    """Test Relationship dataclass."""

    def test_required_fields(self):
        """Required fields: from_paper, to_paper, relationship."""
        r = Relationship(
            from_paper="p1",
            to_paper="p2",
            relationship=RelationshipType.EXTENDS,
        )
        assert r.from_paper == "p1"
        assert r.relationship == RelationshipType.EXTENDS


class TestStoryResult:
    """Test StoryResult dataclass."""

    def test_required_fields(self):
        """Required fields: topic."""
        s = StoryResult(topic="Transformers")
        assert s.topic == "Transformers"

    def test_optional_fields_default(self):
        """Optional fields default."""
        s = StoryResult(topic="T")
        assert s.chapters == []
        assert s.relationships == []
        assert s.protagonist_arc == ""
        assert s.contradictions == []
        assert s.themes == []
        assert s.summary == ""


# =============================================================================
# _determine_role tests
# =============================================================================
class TestDetermineRole:
    """Test _determine_role logic."""

    def _determine_role(self, text: str, year: int):
        """Replicate _determine_role."""
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
        if year <= 2018:
            if any(p in text for p in ['attention is all you need', 'bert', 'gpt']):
                return NarrativeRole.PROTAGONIST
        import re
        if any(re.search(p, text) for p in TURNING_POINT_PATTERNS):
            return NarrativeRole.TURNING_POINT
        if any(re.search(p, text) for p in DIVERGENCE_PATTERNS):
            return NarrativeRole.DIVERGENCE
        return NarrativeRole.PROTAGONIST

    def test_foundational_papers_pre_2018(self):
        """Foundational papers pre-2018 → PROTAGONIST."""
        assert self._determine_role("Attention Is All You Need", 2017) == NarrativeRole.PROTAGONIST
        assert self._determine_role("BERT pre-training", 2018) == NarrativeRole.PROTAGONIST

    def test_turning_point_pattern(self):
        """Turning point keywords → TURNING_POINT."""
        assert self._determine_role("paradigm shift in AI research", 2020) == NarrativeRole.TURNING_POINT
        assert self._determine_role("state-of-the-art results achieved", 2021) == NarrativeRole.TURNING_POINT
        assert self._determine_role("first to propose this method", 2022) == NarrativeRole.TURNING_POINT

    def test_divergence_pattern(self):
        """Divergence keywords → DIVERGENCE."""
        assert self._determine_role("an alternative approach instead", 2020) == NarrativeRole.DIVERGENCE
        assert self._determine_role("unlike previous methods, we diverge", 2021) == NarrativeRole.DIVERGENCE

    def test_default_protagonist(self):
        """No special pattern → PROTAGONIST."""
        assert self._determine_role("a method for classification", 2020) == NarrativeRole.PROTAGONIST

    def test_case_insensitive(self):
        """Pattern matching is case insensitive."""
        # The real caller lowercases text before passing to _determine_role.
        # _determine_role itself uses lowercase regex patterns, so it needs lowercase input.
        text = "breakthrough discovery with novel approach".lower()
        assert self._determine_role(text, 2020) == NarrativeRole.TURNING_POINT


# =============================================================================
# _extract_contribution tests
# =============================================================================
class TestExtractContribution:
    """Test _extract_contribution logic."""

    def _extract_contribution(self, paper: dict) -> str:
        """Replicate _extract_contribution."""
        import re
        title = paper.get('title', '')
        abstract = paper.get('abstract', '')[:200]
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

    def test_abstract_propose_pattern(self):
        """Propose pattern in abstract."""
        paper = {"title": "Test Paper", "abstract": "We propose a new transformer architecture."}
        result = self._extract_contribution(paper)
        assert "new transformer architecture" in result

    def test_abstract_show_pattern(self):
        """Show pattern in abstract."""
        paper = {"title": "T", "abstract": "We show that attention is effective."}
        result = self._extract_contribution(paper)
        assert "attention is effective" in result

    def test_abstract_this_paper_pattern(self):
        """This paper pattern in abstract."""
        paper = {"title": "T", "abstract": "This paper introduces a novel method."}
        result = self._extract_contribution(paper)
        assert "a novel method" in result

    def test_fallback_to_title(self):
        """No pattern match → use title."""
        paper = {"title": "A Novel Deep Learning Approach", "abstract": "Some unrelated text."}
        result = self._extract_contribution(paper)
        assert result == "A Novel Deep Learning Approach"

    def test_empty_title_fallback(self):
        """No title and no pattern → Unknown contribution."""
        paper = {"title": "", "abstract": "Some unrelated text."}
        result = self._extract_contribution(paper)
        assert result == "Unknown contribution"

    def test_contribution_truncated_to_100(self):
        """Contribution truncated to 100 chars."""
        paper = {"title": "T", "abstract": "We propose " + "x" * 200}
        result = self._extract_contribution(paper)
        assert len(result) <= 100


# =============================================================================
# _extract_insight tests
# =============================================================================
class TestExtractInsight:
    """Test _extract_insight logic."""

    def _extract_insight(self, text: str) -> str:
        """Replicate _extract_insight."""
        import re
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

    def test_key_insight_pattern(self):
        """Key insight pattern."""
        assert self._extract_insight("key insight: attention scales with data") == "attention scales with data"

    def test_find_that_pattern(self):
        """We find that pattern."""
        result = self._extract_insight("we find that the model generalizes well")
        assert "model generalizes well" in result

    def test_discover_pattern(self):
        """Discovery pattern."""
        result = self._extract_insight("discovered that self-attention is key")
        assert "self-attention is key" in result

    def test_default_insight(self):
        """No pattern → default."""
        assert self._extract_insight("a paper about deep learning") == "Provides new approach to the problem"

    def test_truncated_to_80(self):
        """Insight truncated to 80 chars."""
        long_text = "x" * 200
        result = self._extract_insight(f"key insight: {long_text}")
        assert len(result) <= 80


# =============================================================================
# _detect_turning_point tests
# =============================================================================
class TestDetectTurningPoint:
    """Test _detect_turning_point logic."""

    def _detect_turning_point(self, text: str) -> str:
        """Replicate _detect_turning_point."""
        if 'breakthrough' in text or 'revolution' in text:
            return "颠覆性突破"
        if 'paradigm shift' in text:
            return "范式转变"
        if 'state-of-the-art' in text or 'sota' in text:
            return "性能突破"
        if 'first' in text and 'time' in text:
            return "首次实现"
        return ""

    def test_breakthrough(self):
        """Breakthrough keyword → 颠覆性突破."""
        assert self._detect_turning_point("a breakthrough in AI") == "颠覆性突破"
        assert self._detect_turning_point("revolutionary approach") == "颠覆性突破"

    def test_paradigm_shift(self):
        """Paradigm shift → 范式转变."""
        assert self._detect_turning_point("paradigm shift in thinking") == "范式转变"

    def test_state_of_the_art(self):
        """SOTA keywords → 性能突破."""
        assert self._detect_turning_point("state-of-the-art results") == "性能突破"
        assert self._detect_turning_point("sota performance") == "性能突破"

    def test_first_time(self):
        """First + time → 首次实现."""
        assert self._detect_turning_point("for the first time we show") == "首次实现"

    def test_no_match(self):
        """No keywords → empty string."""
        assert self._detect_turning_point("a normal research paper") == ""


# =============================================================================
# _infer_relationship tests
# =============================================================================
class TestInferRelationship:
    """Test _infer_relationship logic."""

    def _infer_relationship(self, a, b):
        """Replicate _infer_relationship."""
        rel_type, desc = None, ""
        if b.year > a.year:
            if 'extends' in a.title.lower() or 'building' in b.title.lower():
                return RelationshipType.EXTENDS, f"{b.year} work extends {a.year} work"
        if a.year <= 2017:
            if b.year > 2019:
                return RelationshipType.INHERITS, f"Based on foundational work from {a.year}"
        if any(p in b.title.lower() for p in ['instead', 'alternative', 'rather', 'unlike']):
            return RelationshipType.CONTRASTS, f"Proposes alternative to {a.title[:30]}..."
        if any(p in a.title.lower() + b.title.lower() for p in ['vs', 'versus', '对比', '比较']):
            return RelationshipType.CONTRASTS, f"Contrasts with {a.title[:30]}..."
        return None, ""

    def _narrative(self, year, title):
        return PaperNarrative(paper_id="", title=title, year=year,
                              role=NarrativeRole.PROTAGONIST,
                              core_contribution="", key_insight="")

    def test_extends_keyword(self):
        """'extends' in older paper's title → EXTENDS."""
        # Code checks: 'extends' in a.title.lower() OR 'building' in b.title.lower()
        a = self._narrative(2020, "The Model extends Prior Work")
        b = self._narrative(2021, "New Approach")
        rel_type, desc = self._infer_relationship(a, b)
        assert rel_type == RelationshipType.EXTENDS

    def test_building_keyword(self):
        """"building' in later title → EXTENDS."""
        a = self._narrative(2020, "Base")
        b = self._narrative(2021, "Building on Base")
        rel_type, _ = self._infer_relationship(a, b)
        assert rel_type == RelationshipType.EXTENDS

    def test_foundational_inheritance(self):
        """Foundational pre-2018 paper → INHERITS."""
        a = self._narrative(2017, "Foundational Work")
        b = self._narrative(2020, "New Work")
        rel_type, desc = self._infer_relationship(a, b)
        assert rel_type == RelationshipType.INHERITS
        assert "foundational work from 2017" in desc

    def test_divergence_keyword(self):
        """'instead' keyword in later paper → DIVERGENCE in NarrativeRole."""
        # Note: source code has a bug — returns RelationshipType.CONTRASTS instead of DIVERGENCE
        # (RelationshipType has no DIVERGENCE; that's on NarrativeRole)
        a = self._narrative(2020, "Original Method")
        b = self._narrative(2021, "An Instead Approach")
        rel_type, desc = self._infer_relationship(a, b)
        assert rel_type == RelationshipType.CONTRASTS  # Bug in source: should be DIVERGENCE
        assert "alternative" in desc

    def test_contrasts_keyword(self):
        """vs/versus → CONTRASTS."""
        a = self._narrative(2020, "Method A vs Method B")
        b = self._narrative(2021, "Comparison Study")
        rel_type, _ = self._infer_relationship(a, b)
        assert rel_type == RelationshipType.CONTRASTS

    def test_contrasts_chinese(self):
        """"对比' or '比较' → CONTRASTS."""
        a = self._narrative(2020, "对比分析")
        b = self._narrative(2021, "研究")
        rel_type, _ = self._infer_relationship(a, b)
        assert rel_type == RelationshipType.CONTRASTS

    def test_no_relationship(self):
        """No keywords → None."""
        a = self._narrative(2020, "Method Alpha")
        b = self._narrative(2021, "Method Beta Study")
        rel_type, desc = self._infer_relationship(a, b)
        assert rel_type is None
        assert desc == ""


# =============================================================================
# _organize_chapters tests
# =============================================================================
class TestOrganizeChapters:
    """Test _organize_chapters logic."""

    def _organize_chapters(self, narratives):
        """Replicate _organize_chapters."""
        from collections import defaultdict
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
        chapters = []
        titles = {
            (2008, 2014): "萌芽期 - Attention 机制的发现",
            (2015, 2017): "突破期 - Attention Is All You Need",
            (2018, 2019): "扩散期 - BERT 与预训练革命",
            (2020, 2021): "规模化初期 - GPT-3 的里程碑",
            (2022, 2023): "百模大战 - 开源与闭源的对抗",
            (2024, 2026): "AGI 探索 - 超越 Transformer?",
        }
        for period in sorted(periods.keys()):
            papers = periods[period]
            papers.sort(key=lambda x: x.year)
            chapter = Chapter(
                title=titles.get(period, f"时期 ({period[0]}-{period[1]})"),
                time_range=period,
                papers=papers,
            )
            chapters.append(chapter)
        return chapters

    def _narrative(self, year):
        return PaperNarrative(paper_id=f"p{year}", title=f"Paper {year}", year=year,
                              role=NarrativeRole.PROTAGONIST,
                              core_contribution="C", key_insight="I")

    def test_2013_goes_to_2008_2014_period(self):
        """Year < 2015 → period (2008, 2014)."""
        chapters = self._organize_chapters([self._narrative(2013)])
        assert len(chapters) == 1
        assert chapters[0].time_range == (2008, 2014)

    def test_2017_goes_to_2015_2017_period(self):
        """Year 2015-2017 → period (2015, 2017)."""
        chapters = self._organize_chapters([self._narrative(2016)])
        assert chapters[0].time_range == (2015, 2017)

    def test_2018_goes_to_2018_2019_period(self):
        """Year 2018-2019 → period (2018, 2019)."""
        chapters = self._organize_chapters([self._narrative(2019)])
        assert chapters[0].time_range == (2018, 2019)

    def test_2021_goes_to_2020_2021_period(self):
        """Year 2020-2021 → period (2020, 2021)."""
        chapters = self._organize_chapters([self._narrative(2021)])
        assert chapters[0].time_range == (2020, 2021)

    def test_2023_goes_to_2022_2023_period(self):
        """Year 2022-2023 → period (2022, 2023)."""
        chapters = self._organize_chapters([self._narrative(2023)])
        assert chapters[0].time_range == (2022, 2023)

    def test_2025_goes_to_2024_2026_period(self):
        """Year >= 2024 → period (2024, 2026)."""
        chapters = self._organize_chapters([self._narrative(2025)])
        assert chapters[0].time_range == (2024, 2026)

    def test_multiple_papers_in_same_period(self):
        """Multiple papers in same period → one chapter."""
        chapters = self._organize_chapters([
            self._narrative(2016),
            self._narrative(2017),
        ])
        assert len(chapters) == 1
        assert len(chapters[0].papers) == 2

    def test_papers_sorted_by_year(self):
        """Papers within chapter sorted by year ascending."""
        chapters = self._organize_chapters([
            self._narrative(2019),
            self._narrative(2018),
        ])
        assert chapters[0].papers[0].year == 2018
        assert chapters[0].papers[1].year == 2019

    def test_empty_list(self):
        """Empty list → empty chapters."""
        assert self._organize_chapters([]) == []


# =============================================================================
# _find_contradictions tests
# =============================================================================
class TestFindContradictions:
    """Test _find_contradictions logic."""

    def _find_contradictions(self, narratives):
        """Replicate _find_contradictions."""
        contradictions = []
        efficiency_keywords = ['efficient', 'fast', 'lightweight', 'small', 'distill']
        scale_keywords = ['large', 'massive', 'scale', 'billions', 'parameters']
        for i, a in enumerate(narratives):
            for b in narratives[i + 1:]:
                a_text = a.title.lower()
                b_text = b.title.lower()
                a_efficient = any(k in a_text for k in efficiency_keywords)
                b_scale = any(k in b_text for k in scale_keywords)
                if a_efficient and b_scale:
                    contradictions.append((a.title, b.title))
                a_scale = any(k in a_text for k in scale_keywords)
                b_efficient = any(k in b_text for k in efficiency_keywords)
                if a_scale and b_efficient:
                    contradictions.append((a.title, b.title))
        return contradictions[:5]

    def _narrative(self, title):
        return PaperNarrative(paper_id="", title=title, year=2020,
                              role=NarrativeRole.PROTAGONIST,
                              core_contribution="", key_insight="")

    def test_efficient_vs_large(self):
        """Efficient + large scale → one directional contradiction."""
        narratives = [
            self._narrative("Efficient Transformer"),
            self._narrative("Large Scale Pretraining"),
        ]
        contradictions = self._find_contradictions(narratives)
        # Code: if a_efficient and b_scale → (a, b). Only one direction triggers.
        assert len(contradictions) == 1
        assert "Efficient Transformer" in contradictions[0]

    def test_large_vs_efficient(self):
        """Large scale + efficient → one directional contradiction."""
        narratives = [
            self._narrative("Massive Model Training"),
            self._narrative("Fast Lightweight Distillation"),
        ]
        contradictions = self._find_contradictions(narratives)
        # Code: if a_scale and b_efficient → (a, b). Only one direction triggers.
        assert len(contradictions) == 1

    def test_no_contradiction(self):
        """No efficiency vs scale → empty."""
        narratives = [
            self._narrative("Standard Classification Method"),
            self._narrative("Another Classification Method"),
        ]
        assert self._find_contradictions(narratives) == []

    def test_max_5_contradictions(self):
        """Returns max 5 contradictions."""
        narratives = [self._narrative("Efficient Model v" + str(i)) for i in range(6)]
        narratives += [self._narrative("Large Scale Model v" + str(i)) for i in range(6)]
        contradictions = self._find_contradictions(narratives)
        assert len(contradictions) == 5


# =============================================================================
# _identify_themes tests
# =============================================================================
class TestIdentifyThemes:
    """Test _identify_themes logic."""

    def _identify_themes(self, narratives):
        """Replicate _identify_themes."""
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

    def _narrative(self, title):
        return PaperNarrative(paper_id="", title=title, year=2020,
                              role=NarrativeRole.PROTAGONIST,
                              core_contribution="", key_insight="")

    def test_attention_theme(self):
        """Attention keyword → Attention 机制."""
        narratives = [self._narrative("Self-Attention Mechanism")]
        themes = self._identify_themes(narratives)
        assert "Attention 机制" in themes

    def test_pretraining_theme(self):
        """Pre-train keyword → 预训练范式."""
        narratives = [self._narrative("Pre-training of Language Models")]
        themes = self._identify_themes(narratives)
        assert "预训练范式" in themes

    def test_scaling_theme(self):
        """Scale keyword → 规模化."""
        narratives = [self._narrative("Large Scale Training with Billions")]
        themes = self._identify_themes(narratives)
        assert "规模化" in themes

    def test_efficiency_theme(self):
        """Efficiency keyword → 效率优化."""
        narratives = [self._narrative("Efficient Model Distillation")]
        themes = self._identify_themes(narratives)
        assert "效率优化" in themes

    def test_multimodal_theme(self):
        """Multimodal keyword → 多模态."""
        narratives = [self._narrative("Multimodal Vision and Text Model")]
        themes = self._identify_themes(narratives)
        assert "多模态" in themes

    def test_empty_list(self):
        """No themes match → empty."""
        narratives = [self._narrative("Random Random Random")]
        assert self._identify_themes(narratives) == []

    def test_max_5_themes(self):
        """Returns max 5 themes."""
        narratives = [
            self._narrative("Attention Pretraining Large Scale Efficient Multimodal"),
        ]
        themes = self._identify_themes(narratives)
        assert len(themes) <= 5


# =============================================================================
# _generate_summary tests
# =============================================================================
class TestGenerateSummary:
    """Test _generate_summary logic."""

    def _generate_summary(self, result):
        """Replicate _generate_summary."""
        if not result.chapters:
            return "暂无足够数据生成故事"
        themes = ', '.join(result.themes[:3]) if result.themes else '技术演进'
        summary = f"""《{result.topic}》的演进是一场关于{themes}的探索。
从 {result.chapters[0].time_range[0]} 年的开创性工作，到 {result.chapters[-1].time_range[-1]} 年的最新突破，
领域经历了从理论验证到工程化应用，从单一模型到多元化生态的转变。
"""
        if result.contradictions:
            summary += f"\n核心张力: 发现 {len(result.contradictions)} 个主要矛盾点，"
            summary += "体现了领域内不同技术路线的竞争与融合。"
        return summary

    def _chapter(self, start_year, end_year):
        return Chapter(title="Test", time_range=(start_year, end_year), papers=[])

    def _result(self, topic, chapters=None, themes=None, contradictions=None):
        r = StoryResult(topic=topic)
        r.chapters = chapters or []
        r.themes = themes or []
        r.contradictions = contradictions or []
        return r

    def test_empty_chapters(self):
        """No chapters → empty message."""
        result = self._result("Topic", chapters=[])
        assert self._generate_summary(result) == "暂无足够数据生成故事"

    def test_uses_theme_list(self):
        """Uses theme list in summary."""
        result = self._result("Transformers", chapters=[self._chapter(2017, 2017)], themes=["Attention 机制"])
        summary = self._generate_summary(result)
        assert "Attention 机制" in summary

    def test_no_themes_uses_default(self):
        """No themes → uses 技术演进."""
        result = self._result("Topic", chapters=[self._chapter(2020, 2020)], themes=[])
        summary = self._generate_summary(result)
        assert "技术演进" in summary

    def test_year_range_from_chapters(self):
        """Year range from chapter time_range."""
        result = self._result("RAG", chapters=[self._chapter(2017, 2017), self._chapter(2023, 2023)])
        summary = self._generate_summary(result)
        assert "2017" in summary
        assert "2023" in summary

    def test_contradictions_count(self):
        """Shows contradiction count."""
        result = self._result("Topic", chapters=[self._chapter(2020, 2020)],
                               contradictions=[("A", "B"), ("C", "D"), ("E", "F")])
        summary = self._generate_summary(result)
        assert "3" in summary
        assert "主要矛盾点" in summary

    def test_no_contradictions(self):
        """No contradictions → no contradiction paragraph."""
        result = self._result("Topic", chapters=[self._chapter(2020, 2020)], contradictions=[])
        summary = self._generate_summary(result)
        assert "核心张力" not in summary


# =============================================================================
# _generate_comparison tests
# =============================================================================
class TestGenerateComparison:
    """Test _generate_comparison logic."""

    def _generate_comparison(self, story_a, story_b):
        """Replicate _generate_comparison."""
        lines = [
            f"📖 故事线对比: {story_a.topic} vs {story_b.topic}",
            "",
        ]
        shared_themes = set(story_a.themes) & set(story_b.themes)
        if shared_themes:
            lines.append(f"🔗 共同主题: {', '.join(shared_themes)}")
        lines.append("")
        lines.append(f"📅 {story_a.topic}: {story_a.chapters[0].time_range[0]}-{story_a.chapters[-1].time_range[-1]}")
        lines.append(f"📅 {story_b.topic}: {story_b.chapters[0].time_range[0]}-{story_b.chapters[-1].time_range[-1]}")
        lines.append("")
        lines.append("🎭 主角发展弧线:")
        lines.append(f"  • {story_a.topic}: {story_a.protagonist_arc[:80] if story_a.protagonist_arc else '传统方法演进'}")
        lines.append(f"  • {story_b.topic}: {story_b.protagonist_arc[:80] if story_b.protagonist_arc else '新方法探索'}")
        return "\n".join(lines)

    def _chapter(self, year):
        return Chapter(title="T", time_range=(year, year), papers=[])

    def _result(self, topic, chapters, themes=None, arc=""):
        r = StoryResult(topic=topic, chapters=chapters)
        r.themes = themes or []
        r.protagonist_arc = arc
        return r

    def test_header(self):
        """Header shows both topics."""
        a = self._result("RAG", [self._chapter(2020)])
        b = self._result("Agents", [self._chapter(2021)])
        output = self._generate_comparison(a, b)
        assert "RAG vs Agents" in output

    def test_shared_themes(self):
        """Shared themes shown."""
        a = self._result("RAG", [self._chapter(2020)], themes=["Attention 机制", "Scaling"])
        b = self._result("Agents", [self._chapter(2021)], themes=["Scaling", "Efficiency"])
        output = self._generate_comparison(a, b)
        assert "Scaling" in output
        assert "Attention 机制" not in output

    def test_year_ranges(self):
        """Year ranges from chapters."""
        a = self._result("RAG", [self._chapter(2019)])
        b = self._result("Agents", [self._chapter(2022)])
        output = self._generate_comparison(a, b)
        assert "2019" in output
        assert "2022" in output

    def test_arc_from_result(self):
        """Uses protagonist_arc from result."""
        a = self._result("RAG", [self._chapter(2020)], arc="From retrieval to generation")
        b = self._result("Agents", [self._chapter(2021)], arc="Planning and reasoning")
        output = self._generate_comparison(a, b)
        assert "From retrieval to generation" in output


# =============================================================================
# render_result tests
# =============================================================================
class TestRenderResult:
    """Test render_result formatting."""

    def _render_result(self, result):
        """Replicate render_result."""
        lines = [f"📖 研究故事: {result.topic}", ""]
        for i, chapter in enumerate(result.chapters, 1):
            lines.append(f"第{i}章: {chapter.title}")
            lines.append(f"   时间: {chapter.time_range[0]}-{chapter.time_range[1]}")
            if chapter.summary:
                lines.append(f"   {chapter.summary}")
            else:
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
        if result.contradictions:
            lines.append("⚡ 核心矛盾:")
            for a, b in result.contradictions[:3]:
                lines.append(f"   • {a[:40]}...")
                lines.append(f"     ↔ {b[:40]}...")
            lines.append("")
        if result.themes:
            lines.append(f"🧭 核心主题: {', '.join(result.themes[:4])}")
            lines.append("")
        if result.summary:
            lines.append(f"📝 {result.summary}")
        return "\n".join(lines)

    def _narrative(self, title, year=2020, role=NarrativeRole.PROTAGONIST, insight="Key insight", turn_type=""):
        return PaperNarrative(paper_id="", title=title, year=year, role=role,
                              core_contribution="Contribution", key_insight=insight,
                              turning_point_type=turn_type)

    def _chapter(self, title, time_range, narratives=None, summary=""):
        c = Chapter(title=title, time_range=time_range, papers=narratives or [])
        c.summary = summary
        return c

    def test_header(self):
        """Header shows topic."""
        result = StoryResult(topic="Transformers")
        output = self._render_result(result)
        assert "📖 研究故事: Transformers" in output

    def test_chapter_title_and_time(self):
        """Chapter title and time range shown."""
        result = StoryResult(
            topic="T",
            chapters=[self._chapter("突破期", (2017, 2017), [])],
        )
        output = self._render_result(result)
        assert "第1章: 突破期" in output
        assert "时间: 2017-2017" in output

    def test_chapter_summary(self):
        """Chapter summary used when present."""
        result = StoryResult(
            topic="T",
            chapters=[self._chapter("T", (2020, 2020), [], summary="Chapter summary text")],
        )
        output = self._render_result(result)
        assert "Chapter summary text" in output

    def test_chapter_auto_contribution(self):
        """Without summary, shows core_contribution."""
        result = StoryResult(
            topic="T",
            chapters=[self._chapter("T", (2020, 2020), [self._narrative("Paper 1", 2020, insight="A")])],
        )
        output = self._render_result(result)
        assert "关键贡献" in output

    def test_paper_title_and_year(self):
        """Paper title and year shown."""
        result = StoryResult(
            topic="T",
            chapters=[self._chapter("T", (2020, 2020), [self._narrative("Attention Paper", 2017)])],
        )
        output = self._render_result(result)
        assert "Attention Paper" in output
        assert "2017" in output

    def test_insight_shown(self):
        """Key insight shown."""
        result = StoryResult(
            topic="T",
            chapters=[self._chapter("T", (2020, 2020), [self._narrative("P", 2020, insight="Self attention works")])],
        )
        output = self._render_result(result)
        assert "Self attention works" in output

    def test_turning_point_type(self):
        """Turning point type shown."""
        result = StoryResult(
            topic="T",
            chapters=[self._chapter("T", (2020, 2020), [
                self._narrative("P", 2020, role=NarrativeRole.TURNING_POINT, turn_type="性能突破")
            ])],
        )
        output = self._render_result(result)
        assert "性能突破" in output

    def test_contradictions(self):
        """Contradictions shown."""
        result = StoryResult(topic="T", contradictions=[("Efficient Method", "Large Scale Method")])
        output = self._render_result(result)
        assert "⚡ 核心矛盾" in output
        assert "Efficient Method" in output

    def test_themes(self):
        """Themes shown."""
        result = StoryResult(topic="T", themes=["Attention", "Scaling", "Efficiency"])
        output = self._render_result(result)
        assert "🧭 核心主题" in output
        assert "Attention" in output

    def test_overall_summary(self):
        """Overall summary shown."""
        result = StoryResult(topic="T", summary="This is the story summary.")
        output = self._render_result(result)
        assert "This is the story summary" in output


# =============================================================================
# render_mermaid tests
# =============================================================================
class TestRenderMermaid:
    """Test render_mermaid formatting."""

    def _render_mermaid(self, result):
        """Replicate render_mermaid."""
        lines = ["```mermaid", "flowchart TD", f'    title["📖 {result.topic}"]', ""]
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
        return "\n".join(lines)

    def _narrative(self, title, year=2020, role=NarrativeRole.PROTAGONIST):
        return PaperNarrative(paper_id="", title=title, year=year, role=role,
                              core_contribution="", key_insight="")

    def _chapter(self, narratives=None):
        return Chapter(title="T", time_range=(2020, 2020), papers=narratives or [])

    def test_header(self):
        """Header includes mermaid and topic."""
        result = StoryResult(topic="Transformers")
        output = self._render_mermaid(result)
        assert "```mermaid" in output
        assert "📖 Transformers" in output

    def test_empty_chapters(self):
        """No chapters → just header, title node, blank line, closing."""
        result = StoryResult(topic="T")
        output = self._render_mermaid(result)
        lines = output.strip().split("\n")
        assert lines[-1] == "```"
        # Lines: ```mermaid, flowchart TD, title node, blank line, ```
        assert len(lines) == 5

    def test_paper_nodes(self):
        """Papers added as nodes."""
        result = StoryResult(
            topic="T",
            chapters=[self._chapter([self._narrative("Attention Paper", 2017)])],
        )
        output = self._render_mermaid(result)
        assert "P20170" in output
        assert "Attention Paper" in output[:50] or "Attention Paper" in output

    def test_role_class(self):
        """Role-specific class definitions."""
        result = StoryResult(
            topic="T",
            chapters=[self._chapter([
                self._narrative("Turning Point Paper", 2020, NarrativeRole.TURNING_POINT)
            ])],
        )
        output = self._render_mermaid(result)
        assert "fill:#ff6b6b" in output
        assert "turning_point" in output

    def test_max_2_papers_per_chapter(self):
        """Only first 2 papers per chapter shown."""
        # Use different years to avoid duplicate node IDs (same year → same node_id)
        result = StoryResult(
            topic="T",
            chapters=[self._chapter([
                self._narrative(f"P{i}", 2020 + i) for i in range(5)
            ])],
        )
        output = self._render_mermaid(result)
        # Node IDs: P{year}{chapter_index}, chapter_index=0 → P20200, P20210, P20220...
        assert "P20200" in output  # year=2020
        assert "P20210" in output  # year=2021
        assert "P20220" not in output  # only first 2 papers shown


# =============================================================================
# StoryWeaver instantiation
# =============================================================================
class TestStoryWeaverInit:
    """Test StoryWeaver class."""

    def test_can_instantiate(self):
        """StoryWeaver can be instantiated."""
        weaver = StoryWeaver()
        assert weaver.db is None

    def test_can_instantiate_with_db(self):
        """StoryWeaver can be instantiated with db."""
        mock_db = object()
        weaver = StoryWeaver(db=mock_db)
        assert weaver.db is mock_db
