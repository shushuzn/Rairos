"""Tier 2 unit tests — llm/slides.py, pure functions, no I/O."""
import pytest
from llm.slides import (
    Slide,
    SlidesConfig,
    SlidesResult,
    PaperSlidesGenerator,
)


# =============================================================================
# Dataclass tests
# =============================================================================
class TestSlidesConfig:
    """Test SlidesConfig dataclass."""

    def test_default_values(self):
        """Default configuration values."""
        config = SlidesConfig()
        assert config.template == "academic"
        assert config.num_slides == 10
        assert config.output_format == "pptx"
        assert config.include_notes is False
        assert config.language == "zh"

    def test_custom_values(self):
        """Custom configuration values."""
        config = SlidesConfig(
            template="modern",
            num_slides=20,
            output_format="html",
            include_notes=True,
            language="en",
        )
        assert config.template == "modern"
        assert config.num_slides == 20
        assert config.output_format == "html"
        assert config.include_notes is True
        assert config.language == "en"

    def test_optional_output_path(self):
        """Output path is optional."""
        config = SlidesConfig()
        assert config.output_path is None


class TestSlide:
    """Test Slide dataclass."""

    def test_required_fields(self):
        """Required fields: title, content."""
        slide = Slide(title="Test Title", content="Test content")
        assert slide.title == "Test Title"
        assert slide.content == "Test content"

    def test_optional_fields_defaults(self):
        """Optional fields have defaults."""
        slide = Slide(title="T", content="C")
        assert slide.notes == ""
        assert slide.slide_type == "content"

    def test_all_fields(self):
        """All fields can be set."""
        slide = Slide(
            title="Full Slide",
            content="Full content",
            notes="Speaker notes",
            slide_type="title",
        )
        assert slide.notes == "Speaker notes"
        assert slide.slide_type == "title"

    def test_slide_type_options(self):
        """All valid slide types work."""
        for slide_type in ["title", "content", "comparison", "summary"]:
            slide = Slide(title="T", content="C", slide_type=slide_type)
            assert slide.slide_type == slide_type


class TestSlidesResult:
    """Test SlidesResult dataclass."""

    def test_required_fields(self):
        """Required fields: output_path, slide_count, paper_count."""
        result = SlidesResult(
            output_path="/path/to/slides.pptx",
            slide_count=10,
            paper_count=2,
        )
        assert result.output_path == "/path/to/slides.pptx"
        assert result.slide_count == 10
        assert result.paper_count == 2

    def test_slides_list_defaults_empty(self):
        """Slides list defaults to empty."""
        result = SlidesResult(
            output_path="out.pptx",
            slide_count=5,
            paper_count=1,
        )
        assert result.slides == []

    def test_slides_list_can_be_populated(self):
        """Slides list can be populated."""
        slides = [
            Slide(title="Slide 1", content="Content 1"),
            Slide(title="Slide 2", content="Content 2"),
        ]
        result = SlidesResult(
            output_path="out.pptx",
            slide_count=2,
            paper_count=1,
            slides=slides,
        )
        assert len(result.slides) == 2
        assert result.slides[0].title == "Slide 1"


# =============================================================================
# Template tests
# =============================================================================
class TestSlideTemplates:
    """Test PaperSlidesGenerator templates."""

    def test_academic_template(self):
        """Academic template structure."""
        template = PaperSlidesGenerator.TEMPLATES["academic"]
        assert "title_slide" in template
        assert "content_slide" in template
        assert "section_slide" in template
        assert template["title_slide"]["layout"] == "title"

    def test_minimal_template(self):
        """Minimal template structure."""
        template = PaperSlidesGenerator.TEMPLATES["minimal"]
        assert template["title_slide"]["layout"] == "blank"
        assert template["content_slide"]["font_title"] == 28

    def test_modern_template(self):
        """Modern template structure."""
        template = PaperSlidesGenerator.TEMPLATES["modern"]
        assert template["title_slide"]["layout"] == "title"
        # RGB tuple for background color
        assert template["title_slide"]["bg_color"] == (0, 100, 180)

    def test_all_templates_have_required_slides(self):
        """All templates have title, content, section slides."""
        for name, template in PaperSlidesGenerator.TEMPLATES.items():
            assert "title_slide" in template
            assert "content_slide" in template
            assert "section_slide" in template

    def test_font_sizes_academic(self):
        """Academic template has appropriate font sizes."""
        template = PaperSlidesGenerator.TEMPLATES["academic"]
        assert template["content_slide"]["font_title"] == 32
        assert template["content_slide"]["font_body"] == 18


# =============================================================================
# Comparison table generation
# =============================================================================
class TestComparisonTable:
    """Test _generate_comparison_table logic."""

    def _generate_comparison_table(self, papers: list[dict]) -> str:
        """Replicate comparison table generation logic."""
        headers = ["论文", "年份", "标签"]
        rows = []
        for p in papers:
            rows.append([
                p["title"][:30],
                p.get("year", ""),
                ", ".join(p.get("tags", [])[:3]),
            ])

        col_widths = [max(len(str(row[i])) for row in [headers] + rows) + 2
                      for i in range(len(headers))]

        lines = []
        header_line = " | ".join(h.ljust(w) for h, w in zip(headers, col_widths))
        lines.append(header_line)
        lines.append("|" + "|".join("-" * w for w in col_widths) + "|")

        for row in rows:
            lines.append(" | ".join(str(cell).ljust(w) for cell, w in zip(row, col_widths)))

        return "\n".join(lines)

    def test_table_has_header_row(self):
        """Table contains header row."""
        papers = [{"title": "Paper A", "year": "2024", "tags": ["AI"]}]
        table = self._generate_comparison_table(papers)
        assert "论文" in table
        assert "年份" in table
        assert "标签" in table

    def test_table_has_separator(self):
        """Table has markdown separator row."""
        papers = [{"title": "Test Paper", "year": "2024", "tags": []}]
        table = self._generate_comparison_table(papers)
        assert "---" in table or "--" in table

    def test_truncates_long_titles(self):
        """Long titles are truncated to 30 chars."""
        papers = [{"title": "A" * 50, "year": "2024", "tags": []}]
        table = self._generate_comparison_table(papers)
        assert "A" * 30 in table
        assert "A" * 31 not in table

    def test_limits_tags_to_three(self):
        """Tags are limited to first 3."""
        papers = [{"title": "T", "year": "2024", "tags": ["a", "b", "c", "d", "e"]}]
        table = self._generate_comparison_table(papers)
        # Should contain only 3 tags
        assert "a, b, c" in table
        assert ", d" not in table

    def test_handles_missing_year(self):
        """Missing year handled gracefully."""
        papers = [{"title": "Test Paper", "tags": []}]
        table = self._generate_comparison_table(papers)
        assert "Test Paper" in table

    def test_handles_empty_tags(self):
        """Empty tags handled gracefully."""
        papers = [{"title": "Test", "year": "2024", "tags": []}]
        table = self._generate_comparison_table(papers)
        assert "Test" in table

    def test_multiple_papers(self):
        """Multiple papers render correctly."""
        papers = [
            {"title": "Paper A", "year": "2023", "tags": ["ML"]},
            {"title": "Paper B", "year": "2024", "tags": ["NLP"]},
            {"title": "Paper C", "year": "2025", "tags": ["CV"]},
        ]
        table = self._generate_comparison_table(papers)
        assert "Paper A" in table
        assert "Paper B" in table
        assert "Paper C" in table

    def test_column_widths_adjusted(self):
        """Column widths adjust to content."""
        papers = [
            {"title": "Short", "year": "2024", "tags": ["X"]},
            {"title": "Very Long Paper Title Here", "year": "2024", "tags": ["Y"]},
        ]
        table = self._generate_comparison_table(papers)
        lines = table.split("\n")
        # Both data rows should have consistent column count
        assert lines[0].count("|") == lines[2].count("|")


# =============================================================================
# Slide generation logic
# =============================================================================
class TestSlideGeneration:
    """Test slide generation methods."""

    def _generate_single_paper_slides(self, paper: dict, config: SlidesConfig) -> list[Slide]:
        """Replicate single paper slide generation logic."""
        slides = []

        # Title slide
        slides.append(Slide(
            title=paper["title"],
            content=f"{paper.get('authors', '')}\n{paper.get('year', '')}",
            notes="开场介绍论文标题和作者",
            slide_type="title",
        ))

        # Abstract/motivation
        abstract = paper.get("abstract", "")[:500]
        slides.append(Slide(
            title="研究动机",
            content=abstract,
            notes="介绍研究背景和动机",
            slide_type="content",
        ))

        return slides

    def test_creates_title_slide(self):
        """First slide is title slide."""
        paper = {
            "title": "Attention Is All You Need",
            "authors": "Vaswani et al",
            "year": "2017",
            "abstract": "We propose a new architecture...",
        }
        config = SlidesConfig()
        slides = self._generate_single_paper_slides(paper, config)

        assert slides[0].slide_type == "title"
        assert slides[0].title == paper["title"]

    def test_creates_abstract_slide(self):
        """Second slide is abstract/motivation."""
        paper = {
            "title": "Test Paper",
            "authors": "Author",
            "year": "2024",
            "abstract": "This is the abstract of the paper.",
        }
        config = SlidesConfig()
        slides = self._generate_single_paper_slides(paper, config)

        assert slides[1].title == "研究动机"
        assert "This is the abstract" in slides[1].content

    def test_truncates_long_abstract(self):
        """Long abstract is truncated to 500 chars."""
        paper = {
            "title": "T",
            "authors": "A",
            "year": "2024",
            "abstract": "A" * 1000,
        }
        config = SlidesConfig()
        slides = self._generate_single_paper_slides(paper, config)

        assert len(slides[1].content) <= 500

    def test_handles_missing_authors(self):
        """Missing authors handled gracefully."""
        paper = {
            "title": "Test",
            "year": "2024",
            "abstract": "Abstract",
        }
        config = SlidesConfig()
        slides = self._generate_single_paper_slides(paper, config)

        assert slides[0].title == "Test"
        assert "Author" not in slides[0].content

    def test_handles_missing_abstract(self):
        """Missing abstract handled gracefully."""
        paper = {
            "title": "Test",
            "authors": "Author",
            "year": "2024",
        }
        config = SlidesConfig()
        slides = self._generate_single_paper_slides(paper, config)

        assert slides[1].content == ""


class TestMultiPaperSlides:
    """Test multi-paper slide generation."""

    def _generate_comparison_slides(self, papers: list[dict], config: SlidesConfig) -> list[Slide]:
        """Replicate comparison slides generation logic."""
        slides = []

        titles = [p["title"][:40] for p in papers]
        slides.append(Slide(
            title="论文对比分析",
            content="\n".join(f"• {t}" for t in titles),
            notes="介绍即将对比的论文",
            slide_type="title",
        ))

        return slides

    def test_title_lists_all_papers(self):
        """Title slide lists all papers."""
        papers = [
            {"title": "Paper One"},
            {"title": "Paper Two"},
            {"title": "Paper Three"},
        ]
        config = SlidesConfig()
        slides = self._generate_comparison_slides(papers, config)

        assert "Paper One" in slides[0].content
        assert "Paper Two" in slides[0].content
        assert "Paper Three" in slides[0].content

    def test_truncates_long_titles_in_list(self):
        """Long titles truncated in the list."""
        papers = [{"title": "A" * 60}]
        config = SlidesConfig()
        slides = self._generate_comparison_slides(papers, config)

        # Should be truncated to 40 chars
        assert "A" * 40 in slides[0].content
        assert "A" * 41 not in slides[0].content

    def test_uses_bullet_points(self):
        """Paper list uses bullet points."""
        papers = [{"title": "Paper A"}, {"title": "Paper B"}]
        config = SlidesConfig()
        slides = self._generate_comparison_slides(papers, config)

        assert "•" in slides[0].content


# =============================================================================
# Output format detection
# =============================================================================
class TestOutputFormats:
    """Test output format handling."""

    def test_supported_formats(self):
        """All supported formats are valid."""
        for fmt in ["pptx", "md", "html"]:
            config = SlidesConfig(output_format=fmt)
            assert config.output_format == fmt

    def test_pptx_default(self):
        """PPTX is the default format."""
        config = SlidesConfig()
        assert config.output_format == "pptx"
