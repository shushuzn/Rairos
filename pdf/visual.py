"""
Visual Understanding Enhancement for PDFs.

Extracts and renders:
- Figures/images as PNG files
- LaTeX formulas as rendered images
- Tables as structured markdown/CSV

Usage:
    from pdf.visual import VisualExtractor
    extractor = VisualExtractor(output_dir="output/figures")
    result = extractor.extract_visual_content("paper.pdf")
"""
from __future__ import annotations

import hashlib
import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import List, Optional, Tuple

# Optional dependencies
_fitz = None
_Image = None


def _ensure_deps():
    global _fitz, _Image
    if _fitz is None:
        try:
            import fitz
            from PIL import Image
            _fitz = fitz
            _Image = Image
        except ImportError as e:
            raise RuntimeError("Install dependencies: pip install pymupdf pillow") from e


@dataclass
class RenderedFormula:
    """A rendered LaTeX formula."""
    latex: str
    is_display: bool
    page: int
    image_path: Optional[str] = None
    mathml: Optional[str] = None


@dataclass
class ExtractedFigure:
    """An extracted figure from the PDF."""
    page: int
    image_path: Optional[str] = None
    caption: str = ""
    bbox: Tuple[float, float, float, float] = (0, 0, 0, 0)


@dataclass
class TableAsMarkdown:
    """A table formatted as markdown."""
    headers: List[str]
    rows: List[List[str]]
    page: int
    caption: str = ""
    markdown: str = ""


@dataclass
class VisualContent:
    """All visual content extracted from a PDF."""
    paper_id: str
    figures: List[ExtractedFigure] = field(default_factory=list)
    rendered_formulas: List[RenderedFormula] = field(default_factory=list)
    tables_markdown: List[TableAsMarkdown] = field(default_factory=list)
    output_dir: Optional[Path] = None


class VisualExtractor:
    """Extract visual content from PDFs: figures, formulas, tables."""

    def __init__(self, output_dir: Optional[str] = None, dpi: int = 150):
        """
        Args:
            output_dir: Directory to save extracted images. If None, images are not saved.
            dpi: Resolution for rendered formulas (default 150).
        """
        _ensure_deps()
        self.output_dir = Path(output_dir) if output_dir else None
        self.dpi = dpi
        self._formula_count = 0

        if self.output_dir:
            self.output_dir.mkdir(parents=True, exist_ok=True)

    def extract_visual_content(self, pdf_path: str, paper_id: str = "") -> VisualContent:
        """
        Extract all visual content from a PDF.

        Args:
            pdf_path: Path to PDF file.
            paper_id: Optional identifier for naming output files.

        Returns:
            VisualContent with all extracted elements.
        """
        _ensure_deps()
        import fitz

        pdf_path = Path(pdf_path)
        paper_id = paper_id or pdf_path.stem
        doc = fitz.open(str(pdf_path))

        visual = VisualContent(paper_id=paper_id, output_dir=self.output_dir)

        for page_idx in range(doc.page_count):
            page = doc.load_page(page_idx)

            # Extract figures
            visual.figures.extend(self._extract_figures(page, page_idx, paper_id))

            # Extract and render LaTeX formulas
            visual.rendered_formulas.extend(self._extract_formulas(page, page_idx, paper_id))

            # Extract tables as markdown
            visual.tables_markdown.extend(self._extract_tables_markdown(page, page_idx))

        return visual

    def _extract_figures(
        self, page, page_idx: int, paper_id: str
    ) -> List[ExtractedFigure]:
        """Extract embedded images as figures."""
        figures = []
        image_list = page.get_images(full=True)

        for img_idx, img in enumerate(image_list):
            try:
                xref = img[0]
                base_image = page.parent.extract_image(xref)

                if not base_image:
                    continue

                image_bytes = base_image.get("image")
                if not image_bytes:
                    continue

                # Get image metadata
                img_width = base_image.get("width", 0)
                img_height = base_image.get("height", 0)
                ext = base_image.get("ext", "png")

                # Save image if output_dir is set
                image_path = None
                if self.output_dir:
                    filename = f"{paper_id}_p{page_idx + 1}_fig{img_idx + 1}.{ext}"
                    image_path = self.output_dir / filename
                    with open(image_path, "wb") as f:
                        f.write(image_bytes)

                # Find caption near image
                bbox = img.get("bbox", (0, 0, 0, 0))
                caption = self._find_caption_near(page, bbox)

                figures.append(ExtractedFigure(
                    page=page_idx,
                    image_path=str(image_path) if image_path else None,
                    caption=caption,
                    bbox=bbox,
                ))
            except Exception:
                continue

        return figures

    def _extract_formulas(
        self, page, page_idx: int, paper_id: str
    ) -> List[RenderedFormula]:
        """Extract LaTeX formulas and optionally render as images."""
        formulas = []
        self._formula_count = 0

        # Pattern for inline math: $...$
        inline_pattern = re.compile(r'\$([^\$]+)\$')

        # Pattern for display math: $$...$$
        display_pattern = re.compile(r'\$\$([^\$]+)\$\$')

        # Extract text with bbox info
        page_dict = page.get_text("dict", flags=_fitz.TEXTFLAGS_BLOCKS)

        for block in page_dict.get("blocks", []):
            if block.get("type") != 0:  # text block only
                continue

            for line in block.get("lines", []):
                line_text = ""
                for span in line.get("spans", []):
                    line_text += span.get("text", "")

                # Check for display math ($$...$$)
                for match in display_pattern.finditer(line_text):
                    formula = RenderedFormula(
                        latex=match.group(1),
                        is_display=True,
                        page=page_idx,
                    )
                    if self.output_dir and self._Image:
                        formula.image_path = self._render_latex_image(
                            formula.latex, page_idx, paper_id, is_display=True
                        )
                    formulas.append(formula)

                # Check for inline math ($...$)
                for match in inline_pattern.finditer(line_text):
                    # Skip if this is actually display math
                    if f"${match.group(1)}$" in line_text.replace("$$", ""):
                        formula = RenderedFormula(
                            latex=match.group(1),
                            is_display=False,
                            page=page_idx,
                        )
                        formulas.append(formula)

        return formulas

    def _render_latex_image(
        self, latex: str, page_idx: int, paper_id: str, is_display: bool
    ) -> Optional[str]:
        """Render LaTeX formula as PNG image."""
        try:
            self._formula_count += 1
            filename = f"{paper_id}_p{page_idx + 1}_eq{self._formula_count}.png"
            image_path = self.output_dir / filename if self.output_dir else None

            # Try to use matplotlib for rendering
            try:
                import matplotlib.pyplot as plt
                import matplotlib.mathtext as mathtext

                fig = plt.figure(figsize=(4, 0.8) if not is_display else (6, 1))
                ax = fig.add_axes([0, 0, 1, 1])
                ax.text(0.5, 0.5, f"${latex}$", fontsize=12,
                       ha='center', va='center', transform=ax.transAxes)
                ax.axis('off')

                if image_path:
                    fig.savefig(image_path, dpi=self.dpi, transparent=True,
                               bbox_inches='tight', pad_inches=0.1)
                plt.close(fig)
                return str(image_path) if image_path else None

            except ImportError:
                # matplotlib not available, skip rendering
                return None

        except Exception:
            return None

    def _extract_tables_markdown(
        self, page, page_idx: int
    ) -> List[TableAsMarkdown]:
        """Extract tables and format as markdown."""
        tables = []

        try:
            table_browser = page.find_tables()
            for table in table_browser:
                try:
                    # Get table data
                    data = table.extract()

                    if not data or len(data) < 2:
                        continue

                    headers = [str(h).strip() if h else "" for h in data[0]]
                    rows = [
                        [str(cell).strip() if cell else "" for cell in row]
                        for row in data[1:]
                    ]

                    # Build markdown
                    md = self._build_markdown_table(headers, rows)

                    # Find caption
                    bbox = table.bbox
                    caption = self._find_caption_near(page, bbox)

                    tables.append(TableAsMarkdown(
                        headers=headers,
                        rows=rows,
                        page=page_idx,
                        caption=caption,
                        markdown=md,
                    ))
                except Exception:
                    continue

        except Exception:
            pass

        return tables

    def _build_markdown_table(
        self, headers: List[str], rows: List[List[str]]
    ) -> str:
        """Build markdown table from headers and rows."""
        lines = []

        # Header row
        lines.append("| " + " | ".join(headers) + " |")

        # Separator row
        lines.append("| " + " | ".join(["---"] * len(headers)) + " |")

        # Data rows
        for row in rows:
            # Pad row to match header count
            while len(row) < len(headers):
                row.append("")
            lines.append("| " + " | ".join(row[:len(headers)]) + " |")

        return "\n".join(lines)

    def _find_caption_near(self, page, bbox: tuple, max_distance: float = 50) -> str:
        """Find caption text near a figure or table."""
        try:
            page_dict = page.get_text("dict", flags=fitz.TEXTFLAGS_BLOCKS)

            x_center = (bbox[0] + bbox[2]) / 2
            y_bottom = bbox[3]

            captions = []
            for block in page_dict.get("blocks", []):
                if block.get("type") != 0:
                    continue

                block_bbox = block.get("bbox", (0, 0, 0, 0))
                block_text = ""
                for line in block.get("lines", []):
                    for span in line.get("spans", []):
                        block_text += span.get("text", "")

                block_text = block_text.strip()

                # Check if text is below and close to the figure
                if block_bbox[1] > y_bottom and block_bbox[1] - y_bottom < max_distance:
                    # Check if it looks like a caption (starts with Fig, Table, etc.)
                    if re.match(r'^(Figure|Fig\.|Table|表|图)', block_text, re.I):
                        captions.append((block_bbox[1], block_text))

            if captions:
                # Return the closest caption
                captions.sort(key=lambda x: x[0])
                return captions[0][1][:200]  # Limit length

        except Exception:
            pass

        return ""
