"""PDF download and text extraction."""
import re
from pathlib import Path
from typing import Optional

import requests


def download_pdf(pdf_url: str, out_path: Path, timeout: int = 60) -> None:
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with requests.get(pdf_url, stream=True, timeout=timeout, allow_redirects=True) as r:
        r.raise_for_status()
        with open(out_path, "wb") as f:
            for chunk in r.iter_content(chunk_size=1024 * 1024):
                if chunk:
                    f.write(chunk)


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
