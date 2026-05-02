"""
PDF parsing with structured extraction: LaTeX, tables, figures.
Cache-aware. Retries on transient failures.
"""
from __future__ import annotations

import hashlib
import logging

import orjson
import re
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, List, Optional, Union

from core.exceptions import PDFParseError, ParseTimeoutError
from core.retry import retry
from db.database import Database

logger = logging.getLogger(__name__)

# ─── Data Classes ─────────────────────────────────────────────────────────────


@dataclass
class LaTeXBlock:
    source: str
    is_display: bool
    page: int
    bbox: tuple = (0, 0, 0, 0)


@dataclass
class TableData:
    headers: List[str]
    rows: List[List[str]]
    page: int
    bbox: tuple = (0, 0, 0, 0)
    caption: str = ""


@dataclass
class FigureData:
    caption: str
    page: int
    bbox: tuple
    alt_text: str = ""


@dataclass
class ParsedPaper:
    paper_id: str
    text: str
    latex_blocks: List[LaTeXBlock] = field(default_factory=list)
    tables: List[TableData] = field(default_factory=list)
    figures: List[FigureData] = field(default_factory=list)
    page_count: int = 0
    word_count: int = 0
    parse_version: int = 0
    pdf_hash: str = ""
    title: str = ""
    authors: List[str] = field(default_factory=list)
    abstract: str = ""
    published: str = ""
    warnings: List[str] = field(default_factory=list)
    errors: List[str] = field(default_factory=list)

    def to_cache_dict(self) -> dict:
        return {
            "paper_id": self.paper_id,
            "text": self.text,
            "latex_blocks": [
                {"source": b.source, "is_display": b.is_display, "page": b.page, "bbox": b.bbox}
                for b in self.latex_blocks
            ],
            "tables": [
                {
                    "headers": t.headers,
                    "rows": t.rows,
                    "page": t.page,
                    "bbox": t.bbox,
                    "caption": t.caption,
                }
                for t in self.tables
            ],
            "figures": [
                {"caption": f.caption, "page": f.page, "bbox": f.bbox, "alt_text": f.alt_text}
                for f in self.figures
            ],
            "page_count": self.page_count,
            "word_count": self.word_count,
            "parse_version": self.parse_version,
            "pdf_hash": self.pdf_hash,
            "title": self.title,
            "authors": self.authors,
            "abstract": self.abstract,
            "published": self.published,
            "warnings": self.warnings,
            "errors": self.errors,
        }

    @classmethod
    def from_cache_dict(cls, d: dict) -> ParsedPaper:
        latex = [
            LaTeXBlock(
                source=b["source"],
                is_display=b["is_display"],
                page=b["page"],
                bbox=tuple(b.get("bbox", (0, 0, 0, 0))),
            )
            for b in d.get("latex_blocks", [])
        ]
        tables = [
            TableData(
                headers=t["headers"],
                rows=t["rows"],
                page=t["page"],
                bbox=tuple(t.get("bbox", (0, 0, 0, 0))),
                caption=t.get("caption", ""),
            )
            for t in d.get("tables", [])
        ]
        figures = [
            FigureData(
                caption=f.get("caption", ""),
                page=f["page"],
                bbox=tuple(f.get("bbox", (0, 0, 0, 0))),
                alt_text=f.get("alt_text", ""),
            )
            for f in d.get("figures", [])
        ]
        return cls(
            paper_id=d["paper_id"],
            text=d.get("text", ""),
            latex_blocks=latex,
            tables=tables,
            figures=figures,
            page_count=d.get("page_count", 0),
            word_count=d.get("word_count", 0),
            parse_version=d.get("parse_version", 0),
            pdf_hash=d.get("pdf_hash", ""),
            title=d.get("title", ""),
            authors=d.get("authors", []),
            abstract=d.get("abstract", ""),
            published=d.get("published", ""),
            warnings=d.get("warnings", []),
            errors=d.get("errors", []),
        )


# ─── Regex Helpers ─────────────────────────────────────────────────────────────


_DISPLAY_MATH_PATTERNS = [
    re.compile(r"^\s*\$\$[\s\S]+?\$\$\s*$", re.M),
    re.compile(r"^\s*\\\[\s*[\s\S]+?\s*\\\]\s*$", re.M),
    re.compile(
        r"^\s*\\begin\{(align|align\*|gather|gather\*|eqnarray|multline)\}"
        r"[\s\S]+?"
        r"\\end\{\1\}\s*$",
        re.M,
    ),
]

_INLINE_MATH_PAT = re.compile(r"\$([^\$\n]+?)\$|\\\([^)]+\\\)")


def _is_display_math(line: str) -> bool:
    s = line.strip()
    for pat in _DISPLAY_MATH_PATTERNS:
        if pat.match(s):
            return True
    return False


# ─── Main Parser ───────────────────────────────────────────────────────────────


class PDFParser:
    """
    Structured PDF parser with caching and fallback chain.

    Fallback chain:
        1. PyMuPDF (text + tables + figures)
        2. pdfminer.six (for garbled encodings)
        3. Empty with warning (scanned/image-only)
    """

    def __init__(self, db: Optional[Database] = None, cache_dir: Union[str, Path] = "~/.cache/ai_research_os/parsed"):
        self.db = db
        self.cache_dir = Path(cache_dir).expanduser()
        self.cache_dir.mkdir(parents=True, exist_ok=True)

    # ── Public API ───────────────────────────────────────────────────────────

    def parse(
        self,
        pdf_path: Path,
        paper_id: str,
        use_cache: bool = True,
        max_parse_time: float = 120.0,
    ) -> ParsedPaper:
        """
        Parse a PDF and return structured content.

        Args:
            pdf_path: Path to the PDF file.
            paper_id: Unique paper identifier (arXiv ID or DOI).
            use_cache: If True, skip parsing if cached result exists and PDF hash matches.
            max_parse_time: Timeout in seconds for the parse operation.

        Returns:
            ParsedPaper with extracted text, LaTeX blocks, tables, and figures.
        """
        start_time = time.time()

        if not pdf_path.exists():
            raise PDFParseError(f"PDF file not found: {pdf_path}")

        pdf_hash = self._hash_file(pdf_path)

        # ── Cache check ─────────────────────────────────────────────────────
        if use_cache and self.db is not None:
            cached = self._check_db_cache(paper_id, pdf_hash)
            if cached is not None:
                logger.info("[parser] cache hit for %s (v%d)", paper_id, cached.parse_version)
                return cached

        # ── Parse ──────────────────────────────────────────────────────────
        try:
            content = self._extract_structured(pdf_path)
        except Exception as e:
            content = self._pdfminer_fallback(pdf_path)
            if not content.get("text"):
                raise PDFParseError(f"All PDF extraction methods failed: {e}") from e
            content["warnings"].append(f"PyMuPDF failed, used pdfminer fallback: {e}")

        elapsed = time.time() - start_time
        if elapsed > max_parse_time:
            raise ParseTimeoutError(
                f"PDF parsing exceeded time limit ({max_parse_time:.0f}s) for {paper_id}"
            )

        paper = ParsedPaper(
            paper_id=paper_id,
            text=content.get("text", ""),
            latex_blocks=content.get("latex_blocks", []),
            tables=content.get("tables", []),
            figures=content.get("figures", []),
            page_count=content.get("page_count", 0),
            word_count=len(content["text"].split()) if content.get("text") else 0,
            parse_version=1,
            pdf_hash=pdf_hash,
            title=content.get("title", ""),
            authors=content.get("authors", []),
            abstract=content.get("abstract", ""),
            published=content.get("published", ""),
            warnings=content.get("warnings", []),
            errors=content.get("errors", []),
        )

        # ── Save to cache ─────────────────────────────────────────────────
        if self.db is not None:
            self._save_db_cache(paper)
        else:
            self._save_file_cache(paper)

        return paper

    # ── Cache ────────────────────────────────────────────────────────────────

    def _check_db_cache(self, paper_id: str, pdf_hash: str) -> Optional[ParsedPaper]:
        """Check if a valid cached parse exists in the database."""
        try:
            paper = self.db.get_paper(paper_id)  # type: ignore[union-attr]
            if paper and paper.pdf_hash == pdf_hash and paper.parse_status == "done":
                raw = {
                    "paper_id": paper.id,
                    "text": paper.plain_text,
                    "latex_blocks": paper.latex_blocks,
                    "tables": [],
                    "figures": [],
                    "page_count": paper.page_count,
                    "word_count": paper.word_count,
                    "parse_version": paper.parse_version,
                    "pdf_hash": paper.pdf_hash,
                    "title": paper.title,
                    "authors": paper.authors,
                    "abstract": paper.abstract,
                    "published": paper.published,
                    "warnings": [],
                    "errors": [],
                }
                return ParsedPaper.from_cache_dict(raw)
        except Exception as e:
            logger.warning("[parser] cache lookup failed: %s", e)
        return None

    def _save_db_cache(self, paper: ParsedPaper) -> None:
        """Save parse result to the database."""
        try:
            self.db.update_parse_status(  # type: ignore[union-attr]
                paper_id=paper.paper_id,
                status="done",
                plain_text=paper.text,
                latex_blocks=[b.__dict__ for b in paper.latex_blocks],
                table_count=len(paper.tables),
                figure_count=len(paper.figures),
                word_count=paper.word_count,
                page_count=paper.page_count,
            )
            logger.info(
                "[parser] cached to DB: %s (v%d, %d words, %d tables, %d figures)",
                paper.paper_id,
                paper.parse_version,
                paper.word_count,
                len(paper.tables),
                len(paper.figures),
            )
        except Exception as e:
            logger.warning("[parser] failed to save to DB cache: %s", e)

    def _check_file_cache(self, paper_id: str, pdf_hash: str) -> Optional[ParsedPaper]:
        """Check file-based cache (used when no DB is available)."""
        cache_file = self.cache_dir / f"{paper_id}.json"
        if not cache_file.exists():
            return None
        try:
            d = orjson.loads(cache_file.read_bytes())
            if d.get("pdf_hash") == pdf_hash:
                return ParsedPaper.from_cache_dict(d)
        except (OSError, orjson.JSONDecodeError):
            pass
        return None

    def _save_file_cache(self, paper: ParsedPaper) -> None:
        """Save parse result to file cache."""
        try:
            cache_file = self.cache_dir / f"{paper.paper_id}.json"
            cache_file.write_bytes(
                orjson.dumps(paper.to_cache_dict(), option=orjson.OPT_INDENT_2)
            )
        except OSError as e:
            logger.warning("[parser] failed to save file cache: %s", e)

    # ─── Extraction ─────────────────────────────────────────────────────────

    @retry(max_attempts=2, exceptions=(OSError, IOError))
    def _extract_structured(self, pdf_path: Path) -> dict:
        """
        Extract structured content from PDF using PyMuPDF.
        Returns a dict with keys: text, latex_blocks, tables, figures, page_count, warnings, errors.
        """
        import fitz

        doc = fitz.open(str(pdf_path))
        page_count = doc.page_count
        text_blocks: list[dict] = []
        latex_blocks: list[LaTeXBlock] = []
        tables: list[TableData] = []
        figures: list[FigureData] = []
        warnings: list[str] = []
        errors: list[str] = []

        for page_idx in range(page_count):
            page = doc.load_page(page_idx)

            # ── Tables ───────────────────────────────────────────────────────
            try:
                table_browse = page.find_tables()
                for tbl in table_browse:
                    table_data = self._table_to_structured(tbl, page_idx)
                    if table_data:
                        tables.append(table_data)
            except Exception as e:
                warnings.append(f"Table detection failed on page {page_idx + 1}: {e}")

            # ── Figures ──────────────────────────────────────────────────────
            try:
                figure_list = page.get_images(full=True)
                for img_idx, img in enumerate(figure_list):
                    bbox = img.get("bbox", (0, 0, 0, 0))
                    caption = self._find_caption_near(page, bbox, page_idx)
                    figures.append(
                        FigureData(
                            caption=caption,
                            page=page_idx,
                            bbox=tuple(bbox) if bbox else (0, 0, 0, 0),
                            alt_text=f"image_{page_idx}_{img_idx}",
                        )
                    )
            except Exception as e:
                warnings.append(f"Figure detection failed on page {page_idx + 1}: {e}")

            # ── Text blocks ─────────────────────────────────────────────────
            try:
                page_dict = page.get_text("dict", flags=fitz.TEXTFLAGS_BLOCKS)
            except Exception:
                page_dict = {}

            for block in page_dict.get("blocks", []):
                if block.get("type") != 0:
                    continue
                for line in block.get("lines", []):
                    line_text_parts = []
                    for span in line.get("spans", []):
                        line_text_parts.append(span.get("text", ""))
                    line_text = "".join(line_text_parts).strip()
                    if not line_text or len(line_text) < 2:
                        continue

                    text_blocks.append(
                        {
                            "text": line_text,
                            "bbox": line.get("bbox", (0, 0, 0, 0)),
                            "page": page_idx,
                        }
                    )

                    # ── Inline math ─────────────────────────────────────────
                    for m in _INLINE_MATH_PAT.finditer(line_text):
                        latex_blocks.append(
                            LaTeXBlock(
                                source=m.group(0),
                                is_display=False,
                                page=page_idx,
                            )
                        )

                    # ── Display math check ─────────────────────────────────
                    if _is_display_math(line_text):
                        latex_blocks.append(
                            LaTeXBlock(
                                source=line_text,
                                is_display=True,
                                page=page_idx,
                            )
                        )

        doc.close()

        # ── Assemble text ──────────────────────────────────────────────────
        text = "\n".join(b["text"] for b in text_blocks)
        text = _clean_text(text)

        return {
            "text": text,
            "latex_blocks": latex_blocks,
            "tables": tables,
            "figures": figures,
            "page_count": page_count,
            "warnings": warnings,
            "errors": errors,
        }

    def _pdfminer_fallback(self, pdf_path: Path) -> dict:
        """
        Fallback extraction using pdfminer.six.
        Used when PyMuPDF produces garbled or very short output.
        """
        try:
            from pdfminer.high_level import extract_text

            text = (extract_text(str(pdf_path)) or "").strip()
            if text:
                return {
                    "text": _clean_text(text),
                    "latex_blocks": [],
                    "tables": [],
                    "figures": [],
                    "page_count": 0,
                    "warnings": ["Used pdfminer fallback"],
                    "errors": [],
                }
        except Exception as e:
            logger.warning("[parser] pdfminer fallback failed: %s", e)
            return {
                "text": "",
                "latex_blocks": [],
                "tables": [],
                "figures": [],
                "page_count": 0,
                "warnings": [],
                "errors": [f"All extraction methods failed: {e}"],
            }

        # pdfminer returned empty text
        return {
            "text": "",
            "latex_blocks": [],
            "tables": [],
            "figures": [],
            "page_count": 0,
            "warnings": ["Used pdfminer fallback"],
            "errors": [],
        }

    # ─── Helpers ────────────────────────────────────────────────────────────

    def _hash_file(self, path: Path) -> str:
        """SHA256 hash of a file. Streams to handle large files."""
        h = hashlib.sha256()
        with open(path, "rb") as f:
            for chunk in iter(lambda: f.read(65536), b""):
                h.update(chunk)
        return h.hexdigest()

    def _table_to_structured(self, tbl: Any, page_idx: int) -> Optional[TableData]:
        """Convert a PyMuPDF table object to TableData."""
        try:
            rows = []
            for row in tbl.rows:
                cells = []
                for cell in row:
                    cell_text = (cell.text or "").strip()
                    cells.append(cell_text)
                rows.append(cells)

            if not rows or len(rows) < 2:
                return None

            headers = rows[0] if rows else []
            data_rows = rows[1:]

            bbox = tuple(tbl.bbox) if hasattr(tbl, "bbox") and tbl.bbox else (0, 0, 0, 0)
            return TableData(
                headers=headers,
                rows=data_rows,
                page=page_idx,
                bbox=bbox,
            )
        except Exception as e:
            logger.warning("[parser] table conversion failed: %s", e)
            return None

    def _find_caption_near(
        self, page: Any, bbox: tuple, page_idx: int, search_radius: float = 50.0
    ) -> str:
        """Find a caption near an image based on spatial proximity."""
        try:
            text_dict = page.get_text("dict")
            captions = []
            img_y_center = (bbox[1] + bbox[3]) / 2

            for block in text_dict.get("blocks", []):
                if block.get("type") != 0:
                    continue
                block_bbox = block.get("bbox", (0, 0, 0, 0))
                block_y_center = (block_bbox[1] + block_bbox[3]) / 2

                if abs(block_y_center - img_y_center) < search_radius:
                    block_text = "".join(
                        span.get("text", "") for span in block.get("lines", [[]])[0].get("spans", [])
                    )
                    if re.match(r"^(Figure|Fig\.|Table|Alg\.)", block_text, re.I):
                        captions.append(block_text)

            return captions[0] if captions else ""
        except Exception:
            return ""

    def _extract_latex_blocks_from_text(self, text: str, page_idx: int) -> List[LaTeXBlock]:
        """
        Extract LaTeX blocks from plain text using regex.
        Note: This can only find LaTeX that survived text extraction.
        True LaTeX source requires pdfplumber or Marker.
        """
        blocks = []
        lines = text.splitlines()
        buffer: list[str] = []

        for line in lines:
            if _is_display_math(line):
                if buffer:
                    source = "\n".join(buffer)
                    blocks.append(LaTeXBlock(source=source, is_display=True, page=page_idx))
                    blocks.append(LaTeXBlock(source=line.strip(), is_display=True, page=page_idx))
            else:
                for m in _INLINE_MATH_PAT.finditer(line):
                    blocks.append(LaTeXBlock(source=m.group(0), is_display=False, page=page_idx))

        if buffer:
            blocks.append(LaTeXBlock(source="\n".join(buffer), is_display=True, page=page_idx))

        return blocks


def _clean_text(text: str) -> str:
    """Remove common PDF extraction artifacts."""
    text = text.replace("\r", "\n")
    text = re.sub(r"[ \t]+\n", "\n", text)
    text = re.sub(r"\n{3,}", "\n\n", text)
    return text.strip()
