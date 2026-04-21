"""PDF download and text extraction."""
import re
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import List, Optional

import requests


def download_pdf(pdf_url: str, out_path: Path, timeout: int = 60) -> None:
    """Download PDF with resume support. Overwrites out_path on success."""
    out_path.parent.mkdir(parents=True, exist_ok=True)
    resume_path = out_path.with_suffix(".part")
    existing_size = 0
    headers = {}
    if resume_path.exists():
        existing_size = resume_path.stat().st_size
        if existing_size > 0:
            headers["Range"] = f"bytes={existing_size}-"
    try:
        with requests.get(pdf_url, headers=headers, stream=True, timeout=timeout, allow_redirects=True) as r:
            # Check if server supports Range
            supports_range = r.status_code == 206 or (
                existing_size > 0
                and r.headers.get("Accept-Ranges", "none") != "none"
            )
            if supports_range and existing_size > 0:
                # Resume: append to existing partial file
                with open(resume_path, "ab") as f:
                    for chunk in r.iter_content(chunk_size=1024 * 1024):
                        if chunk:
                            f.write(chunk)
            else:
                # No resume support or no partial file: overwrite
                r.raise_for_status()
                resume_path.unlink(missing_ok=True)
                with open(out_path, "wb") as f:
                    for chunk in r.iter_content(chunk_size=1024 * 1024):
                        if chunk:
                            f.write(chunk)
                return
    except requests.exceptions.HTTPError as e:
        if e.response is not None and e.response.status_code == 416:  # Range not satisfiable
            resume_path.unlink(missing_ok=True)
            raise RuntimeError(f"Requested range not satisfiable for {pdf_url}") from e
        raise
    # Finalize: rename .part → target
    if resume_path.exists() and resume_path.stat().st_size > 0:
        resume_path.rename(out_path)
    elif not out_path.exists():
        raise RuntimeError(f"Download failed for {pdf_url}: no content received")


def extract_pdf_text(pdf_path: Path, max_pages: Optional[int] = None) -> str:
    """Fast text-layer extraction (PyMuPDF)."""
    try:
        import fitz
    except Exception:
        fitz = None
    if fitz is None:
        raise RuntimeError("PyMuPDF not installed. Install with: pip install pymupdf")

    try:
        doc = fitz.open(str(pdf_path))
    except (FileNotFoundError, OSError, getattr(fitz, "FileNotFoundError", FileNotFoundError)):
        return ""

    pages = doc.page_count
    if max_pages is not None:
        pages = min(pages, max_pages)

    chunks = []
    for i in range(pages):
        page = doc.load_page(i)
        txt = page.get_text("text")
        if txt:
            chunks.append(txt)

    raw = "\n".join(chunks)
    raw = raw.replace("\r", "\n")
    raw = re.sub(r"[ \t]+\n", "\n", raw)
    raw = re.sub(r"\n{3,}", "\n\n", raw)
    return raw.strip()


def _is_gibberish_or_too_short(s: str) -> bool:
    s = (s or "").strip()
    if len(s) < 120:
        return True
    printable = sum(1 for ch in s if ch.isprintable())
    if printable / max(1, len(s)) < 0.9:
        return True
    bad = sum(1 for ch in s if (ord(ch) < 9) or (0xE000 <= ord(ch) <= 0xF8FF))
    if bad / max(1, len(s)) > 0.02:
        return True
    return False


def _ocr_page(page, ocr_lang: str = "chi_sim+eng", zoom: float = 2.0) -> str:
    try:
        import fitz
    except Exception:
        fitz = None
    try:
        import pytesseract
        from PIL import Image
    except Exception:
        pytesseract = None
        Image = None

    if pytesseract is None or Image is None or fitz is None:
        raise RuntimeError("OCR deps missing. Install with: pip install pytesseract pillow")

    mat = fitz.Matrix(zoom, zoom)
    pix = page.get_pixmap(matrix=mat, alpha=False)
    img = Image.frombytes("RGB", [pix.width, pix.height], pix.samples)
    img = img.convert("L")
    txt = pytesseract.image_to_string(img, lang=ocr_lang) or ""
    return txt.replace("\r", "\n").strip()


def extract_pdf_text_hybrid(
    pdf_path: Path,
    max_pages: Optional[int] = None,
    ocr: bool = False,
    ocr_lang: str = "chi_sim+eng",
    ocr_zoom: float = 2.0,
    use_pdfminer_fallback: bool = True,
) -> str:
    """
    Best-effort text extraction for non-encrypted PDFs:
    - Per-page PyMuPDF text extraction
    - OCR per page if too short / gibberish (optional)
    - Optional pdfminer fallback (whole doc) for weird encodings
    """
    try:
        import fitz
    except Exception:
        fitz = None
    if fitz is None:
        raise RuntimeError("PyMuPDF not installed. Install with: pip install pymupdf")

    # pdfminer fallback (optional)
    pdfminer_extract_text = None
    if use_pdfminer_fallback:
        try:
            from pdfminer.high_level import extract_text as _pdfminer_extract_text
            pdfminer_extract_text = _pdfminer_extract_text
        except Exception:
            pass

    # pdfminer tries whole doc once
    miner_text = ""
    if pdfminer_extract_text is not None:
        try:
            miner_text = (pdfminer_extract_text(str(pdf_path)) or "").strip()
        except Exception:
            miner_text = ""

    doc = fitz.open(str(pdf_path))
    pages = doc.page_count
    if max_pages is not None:
        pages = min(pages, max_pages)

    out = []
    for i in range(pages):
        page = doc.load_page(i)
        txt = (page.get_text("text") or "").replace("\r", "\n").strip()

        if ocr and _is_gibberish_or_too_short(txt):
            try:
                txt_ocr = _ocr_page(page, ocr_lang=ocr_lang, zoom=ocr_zoom)
                if len(txt_ocr) > len(txt):
                    txt = txt_ocr
            except Exception:
                pass

        if txt:
            out.append(txt)

    fitz_text = "\n\n".join(out).strip()

    # Choose better one
    best = fitz_text
    if miner_text and len(miner_text) > len(best) * 1.2:
        best = miner_text

    best = re.sub(r"[ \t]+\n", "\n", best)
    best = re.sub(r"\n{3,}", "\n\n", best)
    return best.strip()


# ─── Structured Extraction ────────────────────────────────────────────────────


class BlockType(Enum):
    HEADING = "heading"
    BODY = "body"
    CAPTION = "caption"
    LIST_ITEM = "list_item"
    FOOTNOTE = "footnote"


@dataclass
class TextBlock:
    type: BlockType
    text: str
    page: int  # 0-indexed


@dataclass
class TableBlock:
    text: str  # markdown-like table
    page: int
    bbox: tuple  # (x0, y0, x1, y1)


@dataclass
class MathBlock:
    text: str  # LaTeX source or Unicode math
    is_display: bool  # True = standalone display equation
    page: int


@dataclass
class StructuredPdfContent:
    text_blocks: List[TextBlock] = field(default_factory=list)
    tables: List[TableBlock] = field(default_factory=list)
    math_blocks: List[MathBlock] = field(default_factory=list)


def _detect_block_type(line: str, prev_type: BlockType, page_idx: int) -> BlockType:
    """Heuristic classification of a text block."""
    s = line.strip()
    if not s:
        return BlockType.BODY

    # Markdown heading
    if re.match(r"^#{1,6}\s+", s):
        return BlockType.HEADING

    # All-caps short line (likely a section header)
    if s.isupper() and 3 <= len(s) <= 60 and len(s.split()) <= 10:
        return BlockType.HEADING

    # Numbered section heading: "1. Introduction" or "I. Background"
    if re.match(r"^(\d+(\.\d+)*\.?|I{1,3}|IV|V|VI{0,3})\s+[A-Z][A-Za-z ]{2,40}$", s):
        return BlockType.HEADING

    # Figure / Table caption pattern
    if re.match(r"^(Figure|Fig\.|Table|Alg\.?|Algorithm|Listing|Plate)\s+\d", s, re.I):
        return BlockType.CAPTION
    if re.match(r"^(Figure|Fig\.|Table)\s+\d.*:$", s):
        return BlockType.CAPTION

    # Footnote / reference mark: "[1]" or "^*" at end of line
    if re.match(r"^\[\d+\]$", s) or re.match(r"^\^\d+$", s):
        return BlockType.FOOTNOTE

    # List item
    if re.match(r"^[-*+]\s", s) or re.match(r"^\d+\.\s", s):
        return BlockType.LIST_ITEM

    return BlockType.BODY


# Math patterns: display math (standalone equations) vs inline
_DISPLAY_MATH_PATTERNS = [
    # LaTeX display: \[ ... \] or $$ ... $$
    re.compile(r"^\s*\$\$[\s\S]+?\$\$\s*$"),
    re.compile(r"^\s*\\\[\s*[\s\S]+?\s*\\\]\s*$"),
    # AMS align environment
    re.compile(r"^\s*\\begin\{(align|align\*|gather|gather\*|eqnarray)\}[\s\S]+?\\end\{\1\}\s*$", re.M),
    # Unicode math operators often appear as standalone lines
    re.compile(r"^\s*[\u2200-\u22FF\u2A00-\u2BFF]\s*[\u2200-\u22FF\u2A00-\u2BFF\s]+\s*=\s*[\u2200-\u22FF\u2A00-\u2BFF\s]+\s*$"),
]
_INLINE_MATH_PAT = re.compile(r"\$([^\$\n]+?)\$|\\\([^)]+\\\)")


def _is_display_math(line: str) -> bool:
    s = line.strip()
    for pat in _DISPLAY_MATH_PATTERNS:
        if pat.match(s):
            return True
    return False


def _extract_inline_math(line: str) -> List[MathBlock]:
    """Extract inline math spans from a line."""
    blocks = []
    for m in _INLINE_MATH_PAT.finditer(line):
        blocks.append(MathBlock(text=m.group(0), is_display=False, page=-1))
    return blocks


def extract_pdf_structured(
    pdf_path: Path,
    max_pages: Optional[int] = None,
) -> StructuredPdfContent:
    """
    Extract structured PDF content: text blocks (with type), tables, math.

    Uses PyMuPDF block-level extraction + table detection.
    Falls back gracefully if fitz is unavailable.
    """
    try:
        import fitz
    except Exception:
        fitz = None

    if fitz is None:
        raise RuntimeError("PyMuPDF not installed. Install with: pip install pymupdf")

    try:
        doc = fitz.open(str(pdf_path))
    except (FileNotFoundError, OSError, getattr(fitz, "FileNotFoundError", FileNotFoundError)):
        return StructuredPdfContent()

    pages = min(doc.page_count, max_pages or doc.page_count)

    content = StructuredPdfContent()
    prev_block_type = BlockType.BODY

    for page_idx in range(pages):
        page = doc.load_page(page_idx)

        # ── Table detection ──────────────────────────────────────────────────
        try:
            table_browse = page.find_tables()
            for tbl in table_browse:
                # tbl is a Table object; convert to markdown-like text
                rows = []
                for row in tbl.rows:
                    cells = []
                    for cell in row:
                        cell_text = (cell.text or "").strip()
                        cells.append(cell_text)
                    rows.append(cells)

                if rows:
                    # Build markdown table efficiently using str.join instead of per-cell concatenation
                    col_count = max(len(r) for r in rows) if rows else 0
                    md_lines = []
                    header = rows[0] if rows else []
                    md_lines.append("| " + " | ".join(header[i] if i < len(header) else "" for i in range(col_count)) + " |")
                    md_lines.append("|" + "|".join(" --- " for _ in range(col_count)) + "|")
                    for row in rows[1:]:
                        md_lines.append("| " + " | ".join(row[i] if i < len(row) else "" for i in range(col_count)) + " |")

                    table_text = "\n".join(md_lines)
                    bbox = tbl.bbox if hasattr(tbl, "bbox") else (0, 0, 0, 0)
                    content.tables.append(TableBlock(text=table_text, page=page_idx, bbox=bbox))
        except Exception:
            pass  # table detection is best-effort

        # ── Block-level text extraction ───────────────────────────────────────
        # Cache full text once to avoid calling get_text twice per page
        try:
            _raw = page.get_text("text") or ""
            page_text_full = _raw if isinstance(_raw, str) else ""
        except Exception:
            page_text_full = ""

        try:
            page_dict = page.get_text("dict", flags=fitz.TEXTFLAGS_BLOCKS)
        except Exception:
            page_dict = {}

        for block in page_dict.get("blocks", []):
            if block.get("type") != 0:  # only care about text blocks
                continue

            _block_bbox = block.get("bbox", (0, 0, 0, 0))

            for line in block.get("lines", []):
                line_text_parts = []
                for span in line.get("spans", []):
                    line_text_parts.append(span.get("text", ""))

                line_text = "".join(line_text_parts)
                if not line_text.strip():
                    continue

                # Classify block type
                block_type = _detect_block_type(line_text, prev_block_type, page_idx)
                prev_block_type = block_type

                # Filter: skip very short noise blocks
                if block_type == BlockType.BODY and len(line_text.strip()) < 3:
                    continue

                content.text_blocks.append(TextBlock(
                    type=block_type,
                    text=line_text,
                    page=page_idx,
                ))

                # Extract inline math from this line
                inline_maths = _extract_inline_math(line_text)
                for im in inline_maths:
                    im.page = page_idx
                    content.math_blocks.append(im)

        # ── Check for display math lines missed by block approach ─────────────
        # Reuse page_text_full already fetched above (avoids second get_text call)
        for raw_line in (page_text_full or "").splitlines():
            if _is_display_math(raw_line):
                content.math_blocks.append(MathBlock(
                    text=raw_line.strip(),
                    is_display=True,
                    page=page_idx,
                ))

    return content
