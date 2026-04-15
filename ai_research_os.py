#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""
AI Research OS (Full Flow) — AI 自动初稿版（P + C + M + Radar + Timeline）
+ 支持本地 PDF (--pdf) + OCR 兜底 (--ocr) + 中英混合 (chi_sim+eng) + pdfminer 兜底

依赖：
  pip install requests feedparser pymupdf
  # OCR 可选：
  pip install pytesseract pillow
  # pdfminer 兜底可选：
  pip install pdfminer.six

运行示例：
  py ./ai_research_os.py https://arxiv.org/abs/2601.00155 --tags LLM,RAG
  py ./ai_research_os.py https://arxiv.org/abs/2601.00155 --tags LLM,RAG --ai

  # 付费/订阅论文：先人工下载 PDF，再喂给脚本
  py ./ai_research_os.py "10.test/test" --tags LLM --pdf "D:/papers/paper.pdf" --ocr --ai
"""

import argparse
import datetime as dt
import json
import os
import re
import textwrap
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

# ============ 代理配置（可选） ============
# 取消注释并修改为你自己的代理地址，或通过环境变量传入。
# 本地 arXiv / Crossref API 访问不需要代理，直接注释掉即可。
# 如果确实需要代理（访问受限网络），取消下面三行并填入地址。
# ==========================================
# import os
# PROXY_ADDR = "http://127.0.0.1:7897"
# os.environ["HTTP_PROXY"] = PROXY_ADDR
# os.environ["HTTPS_PROXY"] = PROXY_ADDR
# ==========================================

import requests
import feedparser

try:
    import fitz  # PyMuPDF
except Exception:
    fitz = None

# OCR (optional)
try:
    import pytesseract
    from PIL import Image
except Exception:
    pytesseract = None
    Image = None

# pdfminer fallback (optional)
try:
    from pdfminer.high_level import extract_text as pdfminer_extract_text
except Exception:
    pdfminer_extract_text = None


# -----------------------------
# Data structures
# -----------------------------

@dataclass
class Paper:
    source: str              # "arxiv" or "doi"
    uid: str                 # arXiv id or DOI
    title: str
    authors: List[str]
    abstract: str
    published: str           # YYYY-MM-DD best-effort
    updated: str             # YYYY-MM-DD best-effort
    abs_url: str             # landing page
    pdf_url: str             # direct pdf when known
    primary_category: str = ""


# -----------------------------
# Constants
# -----------------------------

ARXIV_API = "https://export.arxiv.org/api/query?id_list={arxiv_id}"
CROSSREF_WORKS = "https://api.crossref.org/works/{doi}"
DOI_RESOLVER = "https://doi.org/"

RADAR_PATH = Path("00-Radar") / "Radar.md"
TIMELINE_PATH = Path("00-Radar") / "Timeline.md"


# -----------------------------
# Basics
# -----------------------------

def today_iso() -> str:
    return dt.date.today().isoformat()


def ensure_research_tree(root: Path) -> None:
    dirs = [
        "00-Radar",
        "01-Foundations",
        "02-Models",
        "03-Training",
        "04-Scaling",
        "05-Alignment",
        "06-Agents",
        "07-Infrastructure",
        "08-Optimization",
        "09-Evaluation",
        "10-Applications",
        "11-Future-Directions",
    ]
    root.mkdir(parents=True, exist_ok=True)
    for d in dirs:
        (root / d).mkdir(parents=True, exist_ok=True)


def slugify_title(title: str, max_len: int = 80) -> str:
    t = title.strip()
    t = re.sub(r"\s+", " ", t)
    t = re.sub(r"[^\w\s\-]", "", t, flags=re.UNICODE)
    t = t.replace(" ", "-")
    t = re.sub(r"-{2,}", "-", t).strip("-_")
    if len(t) > max_len:
        t = t[:max_len].rstrip("-_")
    return t or "Paper"


def safe_uid(s: str) -> str:
    return re.sub(r"[^\w\.-]+", "_", s.strip())


def read_text(p: Path) -> str:
    return p.read_text(encoding="utf-8") if p.exists() else ""


def write_text(p: Path, content: str) -> None:
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(content, encoding="utf-8")


# -----------------------------
# Input detection
# -----------------------------

def is_probably_doi(s: str) -> bool:
    s = s.strip()
    return bool(re.search(r"(https?://(dx\.)?doi\.org/)?10\.\d{4,9}/\S+", s, flags=re.I))


def normalize_doi(s: str) -> str:
    s = s.strip()
    s = re.sub(r"^https?://(dx\.)?doi\.org/", "", s, flags=re.I)
    return s.strip().rstrip(".")


def normalize_arxiv_id(s: str) -> Optional[str]:
    s = s.strip()

    # arXiv URL formats
    m = re.search(r"(?:arxiv\.org/(?:abs|pdf)/)(\d{4}\.\d{4,5})(v\d+)?", s, flags=re.I)
    if m:
        return (m.group(1) + (m.group(2) or "")).strip()

    # New-style id directly
    if re.fullmatch(r"\d{4}\.\d{4,5}(v\d+)?", s):
        return s

    # Old-style id
    if re.fullmatch(r"[a-zA-Z\-]+/\d{7}(v\d+)?", s):
        return s

    # arXiv DOI format
    m2 = re.search(r"10\.48550/arXiv\.(\d{4}\.\d{4,5})(v\d+)?", s, flags=re.I)
    if m2:
        return (m2.group(1) + (m2.group(2) or "")).strip()

    return None


# -----------------------------
# Fetch metadata
# -----------------------------

def fetch_arxiv_metadata(arxiv_id: str, timeout: int = 30) -> Paper:
    url = ARXIV_API.format(arxiv_id=arxiv_id)
    r = requests.get(url, timeout=timeout)
    r.raise_for_status()
    feed = feedparser.parse(r.text)
    if not feed.entries:
        raise ValueError(f"arXiv API returned no entries for id: {arxiv_id}")

    e = feed.entries[0]
    title = (getattr(e, "title", "") or "").replace("\n", " ").strip()
    abstract = (getattr(e, "summary", "") or "").replace("\n", " ").strip()

    authors = []
    for a in getattr(e, "authors", []) or []:
        name = getattr(a, "name", "").strip()
        if name:
            authors.append(name)

    published = (getattr(e, "published", "") or "")[:10]
    updated = (getattr(e, "updated", "") or "")[:10]
    abs_url = getattr(e, "link", "") or f"https://arxiv.org/abs/{arxiv_id}"

    pdf_url = ""
    for l in getattr(e, "links", []) or []:
        if getattr(l, "type", "") == "application/pdf":
            pdf_url = l.href
            break
    if not pdf_url:
        pdf_url = f"https://arxiv.org/pdf/{arxiv_id}.pdf"

    primary_cat = ""
    try:
        primary_cat = getattr(e, "arxiv_primary_category", {}).get("term", "")  # type: ignore
    except Exception:
        primary_cat = ""

    return Paper(
        source="arxiv",
        uid=arxiv_id,
        title=title,
        authors=authors,
        abstract=abstract,
        published=published or "",
        updated=updated or "",
        abs_url=abs_url,
        pdf_url=pdf_url,
        primary_category=primary_cat or "",
    )


def _best_effort_date_from_crossref(item: dict) -> str:
    for key in ["published-print", "published-online", "published", "issued", "created", "deposited"]:
        obj = item.get(key)
        if not obj:
            continue
        dp = obj.get("date-parts")
        if isinstance(dp, list) and dp and isinstance(dp[0], list) and dp[0]:
            parts = dp[0]
            y = parts[0]
            m = parts[1] if len(parts) > 1 else 1
            d = parts[2] if len(parts) > 2 else 1
            try:
                return dt.date(int(y), int(m), int(d)).isoformat()
            except Exception:
                continue
    return ""


def _authors_from_crossref(item: dict) -> List[str]:
    out = []
    for a in item.get("author", []) or []:
        given = (a.get("given") or "").strip()
        family = (a.get("family") or "").strip()
        name = (given + " " + family).strip()
        if name:
            out.append(name)
    return out


def _title_from_crossref(item: dict) -> str:
    t = item.get("title") or []
    if isinstance(t, list) and t:
        return str(t[0]).strip()
    if isinstance(t, str):
        return t.strip()
    return ""


def _abstract_from_crossref(item: dict) -> str:
    ab = item.get("abstract") or ""
    if not ab:
        return ""
    ab = re.sub(r"<[^>]+>", "", ab)
    ab = re.sub(r"\s+", " ", ab).strip()
    return ab


def _try_find_arxiv_id_in_crossref(item: dict, doi: str) -> Optional[str]:
    arx = normalize_arxiv_id(doi)
    if arx:
        return arx

    rel = item.get("relation") or {}
    blob = str(rel)
    m = re.search(r"arxiv\.org/(?:abs|pdf)/(\d{4}\.\d{4,5})(v\d+)?", blob, flags=re.I)
    if m:
        return (m.group(1) + (m.group(2) or "")).strip()

    for k in ["alternative-id", "archive", "URL", "link"]:
        v = item.get(k)
        if not v:
            continue
        blob2 = str(v)
        m2 = re.search(r"arxiv\.org/(?:abs|pdf)/(\d{4}\.\d{4,5})(v\d+)?", blob2, flags=re.I)
        if m2:
            return (m2.group(1) + (m2.group(2) or "")).strip()

        m3 = re.search(r"(\d{4}\.\d{4,5})(v\d+)?", blob2)
        if m3 and "arxiv" in blob2.lower():
            return (m3.group(1) + (m3.group(2) or "")).strip()

    return None


def fetch_crossref_metadata(doi: str, timeout: int = 30) -> Tuple[Paper, Optional[str]]:
    """
    Crossref failures (404/network) will NOT crash.
    Returns (Paper, maybe_arxiv_id).
    """
    url = CROSSREF_WORKS.format(doi=doi)

    try:
        r = requests.get(
            url,
            timeout=timeout,
            headers={"User-Agent": "AI-Research-OS/1.0"},
        )
        if r.status_code == 404:
            raise ValueError("Crossref 404 (DOI not found in Crossref)")
        r.raise_for_status()
        data = r.json()
        item = data.get("message") or {}

        title = _title_from_crossref(item) or doi
        authors = _authors_from_crossref(item)
        abstract = _abstract_from_crossref(item)
        published = _best_effort_date_from_crossref(item)

        abs_url = (item.get("URL") or "").strip() or (DOI_RESOLVER + doi)

        pdf_url = ""
        for l in item.get("link", []) or []:
            if (l.get("content-type") or "").lower() == "application/pdf" and l.get("URL"):
                pdf_url = l["URL"].strip()
                break

        maybe_arxiv = _try_find_arxiv_id_in_crossref(item, doi)

        p = Paper(
            source="doi",
            uid=doi,
            title=title,
            authors=authors,
            abstract=abstract,
            published=published or "",
            updated="",
            abs_url=abs_url,
            pdf_url=pdf_url,
            primary_category="",
        )
        return p, maybe_arxiv

    except Exception:
        # Graceful downgrade: DOI-only metadata (minimal), still try to parse arXiv id from DOI string.
        maybe_arxiv = normalize_arxiv_id(doi)
        p = Paper(
            source="doi",
            uid=doi,
            title=doi,
            authors=[],
            abstract="",
            published="",
            updated="",
            abs_url=DOI_RESOLVER + doi,
            pdf_url="",
            primary_category="",
        )
        return p, maybe_arxiv


# -----------------------------
# PDF download + extraction
# -----------------------------

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
    # very rough "bad char" heuristic
    bad = sum(1 for ch in s if (ord(ch) < 9) or (0xE000 <= ord(ch) <= 0xF8FF))
    if bad / max(1, len(s)) > 0.02:
        return True
    return False


def _ocr_page(page, ocr_lang: str = "chi_sim+eng", zoom: float = 2.0) -> str:
    if pytesseract is None or Image is None:
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
    if fitz is None:
        raise RuntimeError("PyMuPDF not installed. Install with: pip install pymupdf")

    # pdfminer tries whole doc once (optional)
    miner_text = ""
    if use_pdfminer_fallback and pdfminer_extract_text is not None:
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


def looks_like_heading(line: str) -> bool:
    s = line.strip()
    if len(s) < 3 or len(s) > 120:
        return False

    if re.match(r"^(\d+(\.\d+)*)\s+[A-Za-z].{2,}$", s):
        return True
    if re.match(r"^(I|II|III|IV|V|VI|VII|VIII|IX|X)\.?\s+[A-Za-z].{2,}$", s):
        return True

    keywords = [
        "abstract", "introduction", "background", "related work", "method",
        "approach", "preliminaries", "experiments", "evaluation", "results",
        "discussion", "limitations", "conclusion", "future work", "references",
        "appendix", "acknowledgments", "ablation"
    ]
    low = s.lower()
    if any(low == k for k in keywords):
        return True
    if any(low.startswith(k + " ") for k in keywords):
        return True

    if s.isupper() and 4 <= len(s) <= 40:
        return True

    return False


def segment_into_sections(text: str, max_sections: int = 18) -> List[Tuple[str, str]]:
    lines = text.splitlines()
    sections: List[Tuple[str, List[str]]] = []
    cur_title = "BODY"
    cur_buf: List[str] = []

    for line in lines:
        stripped = line.strip()
        # Detect markdown headings (# Title, ## Title, etc.)
        md_heading_match = re.match(r"^(#{1,6})\s+(.+)$", stripped)
        if md_heading_match:
            if cur_buf:
                sections.append((cur_title, cur_buf))
            cur_title = md_heading_match.group(2).strip()
            cur_buf = []
        elif looks_like_heading(line):
            if cur_buf:
                sections.append((cur_title, cur_buf))
            cur_title = line.strip()
            cur_buf = []
        else:
            cur_buf.append(line)

    if cur_buf:
        sections.append((cur_title, cur_buf))

    merged: List[Tuple[str, str]] = []
    for title, buf in sections:
        content = "\n".join(buf).strip()
        if not content:
            continue
        if False and merged and len(content) < 400 and title != "BODY":  # disabled: merging breaks section separation
            pt, pc = merged[-1]
            merged[-1] = (pt, (pc + "\n\n" + title + "\n" + content).strip())
        else:
            merged.append((title, content))

    if len(merged) > max_sections:
        merged = merged[:max_sections] + [("TRUNCATED", "…(text truncated)…")]

    return merged


def format_section_snippets(sections: List[Tuple[str, str]], max_chars_each: int = 1800) -> str:
    out = []
    for title, content in sections:
        snippet = content.strip()
        if len(snippet) > max_chars_each:
            snippet = snippet[:max_chars_each].rstrip() + "\n…"
        out.append(f"### {title}\n\n" + "\n".join(["> " + ln for ln in snippet.splitlines() if ln.strip()]))
        out.append("")
    return "\n".join(out).strip()


# -----------------------------
# LLM (OpenAI-compatible) for AI draft
# -----------------------------

def call_llm_chat_completions(
    messages: List[Dict[str, str]],
    model: str,
    user_prompt: Optional[str] = None,
    base_url: str = "https://api.openai.com/v1",
    api_key: Optional[str] = None,
    timeout: int = 180,
    system_prompt: Optional[str] = None,
) -> str:
    api_key = api_key or os.getenv("OPENAI_API_KEY", "")
    if not api_key:
        raise ValueError("Missing API key. Provide --api-key or set OPENAI_API_KEY.")

    url = base_url.rstrip("/") + "/chat/completions"
    headers = {
        "Authorization": f"Bearer {api_key}",
        "Content-Type": "application/json",
    }
    msgs = messages[:]
    if system_prompt:
        msgs = [{"role": "system", "content": system_prompt}] + msgs
    payload = {
        "model": model,
        "temperature": 0.2,
        "messages": msgs,
    }
    if user_prompt:
        payload["messages"] = msgs + [{"role": "user", "content": user_prompt}]

    r = requests.post(url, headers=headers, data=json.dumps(payload), timeout=timeout)
    r.raise_for_status()
    data = r.json()
    return data["choices"][0]["message"]["content"]


def ai_generate_pnote_draft(
    paper: Paper,
    tags: List[str],
    extracted_text: str,
    base_url: str,
    api_key: str,
    model: str,
) -> str:
    system_prompt = """你是严谨的研究助理 + 对抗式审稿人。
目标：基于给定论文信息与抽取的正文片段，为用户的 Research OS P-Note 生成“可编辑初稿”。
硬性要求：
- 明确区分：事实（可在文中/摘要中找到） vs 推断（你的判断）
- 不要编造不存在的实验/数据集/结果；若不确定，写“未在当前片段中找到”
- 输出必须是中文、Markdown
- 每个栏目开头必须加：> AI Draft（可编辑，需人工核验）
- 只输出指定栏目，不要输出额外解释
"""

    user_prompt = f"""
论文标题：{paper.title}
作者：{", ".join(paper.authors) if paper.authors else "Unknown"}
来源：{paper.source}:{paper.uid}
发布日期：{paper.published or "N/A"}
标签：{", ".join(tags)}

【Abstract】
{paper.abstract or "(空)"}

【抽取正文片段（可能不完整）】
{extracted_text}

现在请按以下栏目生成初稿（每栏用 '## ' 二级标题输出，标题必须严格匹配）：

## 1. 背景
## 2. 核心问题
## 3. 方法结构
- 需要包含：架构拆解 / 算法逻辑 / 关键组件（用小标题或列表）
## 4. 关键创新
## 5. 实验分析
- 需要包含：数据集 / 基线对比 / 消融实验 / 成本分析（找不到就标注未找到）
## 6. 对抗式审稿
- 需要包含：逻辑漏洞 / 偏置风险 / 复现难度 / 失败模式推测
## 7. 优势
## 8. 局限
## 9. 本质抽象
## 10. 与其他方法对比
- 给出 vs A / vs B / vs C 的对比建议（A/B/C 用你认为合理的同类方法名；不确定就写“待定”）
## 11. Decision（决策）
- 是否使用/使用场景/不适用边界/接下来关注信号
## 知识蒸馏
- Facts/Principles/Insights
## 认知升级
- 长期价值/规模效应/技术护城河/是否范式转移/商业潜力
## 评分量表
- Novelty/Leverage/Evidence/Cost/Moat/Adoption Signal + Overall Judgment
"""

    return call_llm_chat_completions(
        base_url=base_url,
        api_key=api_key,
        model=model,
        system_prompt=system_prompt,
        user_prompt=user_prompt,
    )


# -----------------------------
# Markdown templates (pure MD)
# -----------------------------

def render_pnote(p: Paper, tags: List[str], extracted_sections_md: str, ai_draft_md: str = "") -> str:
    date_for_note = p.published or today_iso()
    authors_line = ", ".join(p.authors) if p.authors else "Unknown"
    tags_list = ", ".join(tags)

    src_line = f"{p.source.upper()}: {p.uid}"
    abstract_md = ("> **Abstract（原文）**  \n> " + p.abstract) if p.abstract else "_（未获取到 abstract，可手动补充）_"
    ai_block = f"\n\n---\n\n## AI 自动初稿（待核验）\n\n{ai_draft_md.strip()}\n" if ai_draft_md.strip() else ""

    md = f"""\
type: paper
status: draft
date: {date_for_note}
tags: [{tags_list}]
------------------

# {p.title}

**Source:** {src_line}  
**Authors:** {authors_line}  
**Published:** {p.published or "N/A"}  | **Updated:** {p.updated or "N/A"}  
**Landing:** {p.abs_url}  
**PDF:** {p.pdf_url or "N/A"}  
**Primary Category:** {p.primary_category or "N/A"}

---

## Research Question Card

* 我想解决什么问题？
* 为什么重要？
* 我的先验判断是什么？
* 什么证据会推翻我？

---

## 1. 背景

{abstract_md}

---

## 2. 核心问题

---

## 3. 方法结构

### 架构拆解

### 算法逻辑

### 关键组件

---

## 4. 关键创新

---

## 5. 实验分析

### 数据集

### 基线对比

### 消融实验

### 成本分析

---

## 6. 对抗式审稿

* 逻辑漏洞：
* 偏置风险：
* 复现难度：
* 失败模式推测：

---

## 7. 优势

---

## 8. 局限

---

## 9. 本质抽象

---

## 10. 与其他方法对比

* vs A：
* vs B：
* vs C：

---

## 11. Decision（决策）

* 是否使用？
* 使用场景？
* 不适用边界？
* 接下来关注信号？

---

## 知识蒸馏

### Facts

1.
2.

### Principles

1.
2.

### Insights

1.
2.

---

## 认知升级

* 长期价值：
* 规模效应：
* 技术护城河：
* 是否范式转移：
* 商业潜力：

---

## 评分量表

* Novelty (1-5):
* Leverage (1-5):
* Evidence (1-5):
* Cost (1-5):
* Moat (1-5):
* Adoption Signal (1-5):

### Overall Judgment
{ai_block}
---

## 附：PDF 章节粗拆（自动抽取 · 供快速定位）

{extracted_sections_md if extracted_sections_md else "_（未能从 PDF 抽取到可用文本）_"}
"""
    return textwrap.dedent(md).strip() + "\n"


def render_cnote(concept: str) -> str:
    md = f"""\
type: concept
status: evergreen
-----------------

# {concept}

## 核心定义

## 产生背景

## 技术本质

## 常见实现路径

## 优势

## 局限

## 与其他思想的关系

## 代表论文

## 演化时间线

## 未来趋势

## 关联笔记

"""
    return textwrap.dedent(md).strip() + "\n"


def render_mnote(title: str, a: str, b: str, c: str) -> str:
    today = today_iso()
    md = f"""\
type: comparison
status: evolving
----------------

# {title}

## 对比维度

| 维度   | A | B | C |
| ---- | - | - | - |
| 核心思想 |   |   |   |
| 成本结构 |   |   |   |
| 性能   |   |   |   |
| 扩展性  |   |   |   |
| 适用场景 |   |   |   |

---

## 当前 A/B/C

- A: {a}
- B: {b}
- C: {c}

---

## 结构性差异

---

## 成本演进分析

---

## 演进方向

---

## 当前判断

---

## View Evolution Log

* {today}

  * 旧观点：
  * 新证据：
  * 更新结论：

"""
    return textwrap.dedent(md).strip() + "\n"


# -----------------------------
# Parsing P-Notes for tags / dates
# -----------------------------

def parse_frontmatter(md: str) -> Dict[str, Any]:
    out: Dict[str, Any] = {}
    lines = md.splitlines()
    i = 0
    while i < len(lines):
        line = lines[i]
        if line.strip() == "------------------":
            break
        m = re.match(r"^\s*([A-Za-z0-9_\-]+)\s*:\s*(.*)\s*$", line)
        if m:
            key = m.group(1).strip()
            val = m.group(2).strip()
            # Check if next lines are YAML list items
            if val == "" and i + 1 < len(lines) and re.match(r"^\s+-\s+", lines[i + 1]):
                items = []
                j = i + 1
                while j < len(lines):
                    item_line = lines[j]
                    item_m = re.match(r"^\s+-\s+(.*)\s*$", item_line)
                    if not item_m:
                        break
                    items.append(item_m.group(1).strip())
                    j += 1
                out[key] = items
                i = j - 1
            else:
                out[key] = val
        i += 1
    return out


def parse_tags_from_frontmatter(fm: Dict[str, Any]) -> List[str]:
    raw = fm.get("tags", "")
    # Handle Python list directly (e.g., ["LLM", "RAG"])
    if isinstance(raw, list):
        return [str(t).strip() for t in raw if str(t).strip()]
    raw = str(raw).strip()
    if not raw:
        return []
    # Handle comma-separated string (e.g. "LLM,Agent,RAG")
    if "," in raw and not raw.startswith("["):
        return [t.strip() for t in raw.split(",") if t.strip()]
    m = re.match(r"^\[(.*)\]$", raw)
    if not m:
        return []
    inner = m.group(1).strip()
    if not inner:
        return []
    return [t.strip() for t in inner.split(",") if t.strip()]


def parse_date_from_frontmatter(fm: Dict[str, str]) -> str:
    d = fm.get("date", "").strip()
    if re.fullmatch(r"\d{4}-\d{2}-\d{2}", d):
        return d
    return ""


def collect_pnotes(root: Path) -> List[Path]:
    return sorted([p for p in root.rglob("*.md") if p.is_file() and p.parent.name in ("02-Papers", "Papers", "papers")])


def pnotes_by_tag(root: Path) -> Dict[str, List[Tuple[str, Path]]]:
    mapping: Dict[str, List[Tuple[str, Path]]] = {}
    for p in collect_pnotes(root):
        md = read_text(p)
        fm = parse_frontmatter(md)
        tags = parse_tags_from_frontmatter(fm)
        if not tags:
            continue
        d = parse_date_from_frontmatter(fm)
        if not d:
            d = dt.date.fromtimestamp(p.stat().st_mtime).isoformat()

        for t in tags:
            mapping.setdefault(t, []).append((d, p))

    for t in mapping:
        mapping[t].sort(key=lambda x: x[0], reverse=True)
    return mapping


def wikilink_for_pnote(pnote_path: Path) -> str:
    stem = Path(pnote_path).stem
    return f"[[{stem}]]"


# -----------------------------
# C-Note update
# -----------------------------

def ensure_cnote(concept_dir: Path, concept: str) -> Path:
    path = concept_dir / f"C - {concept}.md"
    if not path.exists():
        write_text(path, render_cnote(concept))
    return path


def upsert_link_under_heading(md: str, heading: str, link_line: str) -> str:
    # Strip leading ##/ heading prefix so we match against just the text
    clean_heading = re.sub(r"^#+\s+", "", heading).strip()
    # Look for the heading (## prefix already in pattern)
    pattern = rf"(^##\s+{re.escape(clean_heading)}(?:\s*|\s+.*)$)"
    m = re.search(pattern, md, flags=re.M)
    if not m:
        return md.rstrip() + f"\n\n## {clean_heading}\n\n{link_line}\n"

    match_line = m.group(0).split('\n')[0]  # just the heading line without trailing content
    start = m.start() + len(match_line)  # end of heading line in the full string
    after = md[start:]
    m2 = re.match(r"(\s*\n)*", after)  # skip blank lines
    insert_pos = start + (m2.end() if m2 else 0)

    # Find section end: next ## heading or end of file
    rest = after[m2.end() if m2 else 0:]
    m3 = re.search(r"\n##\s+", rest)
    section_end = insert_pos + m3.start() if m3 else len(md)

    # Extract current section content (skip the leading \n from after the heading)
    section_content = md[insert_pos:section_end].lstrip("\n")

    # Remove ALL link lines under this heading (any format: wikilinks or plain)
    # Match: line starting with -, optional space, any content
    cleaned = re.sub(r"^-\s*\S.*$", "", section_content, flags=re.M).strip("\n")
    section_content = cleaned.strip("\n")
    new_section = link_line.rstrip() + "\n" + section_content if section_content.strip() else link_line.rstrip()

    return md[:insert_pos] + new_section + md[section_end:]


def update_cnote_links(cnote_path: Path, pnote_path: Path) -> None:
    md = read_text(cnote_path)
    md2 = upsert_link_under_heading(md, "关联笔记", wikilink_for_pnote(pnote_path))
    write_text(cnote_path, md2)


# -----------------------------
# M-Note (>=3)
# -----------------------------

def pick_top3_pnotes_for_tag(tag: str, tag_map: Dict[str, List[Tuple[str, Path]]]) -> Optional[List[Path]]:
    items = tag_map.get(tag, [])
    if len(items) < 3:
        return None
    return [items[0][1], items[1][1], items[2][1]]


def mnote_filename(tag: str, a: Path, b: Path, c: Path) -> str:
    def short(stem: str, n: int = 24) -> str:
        s = stem
        s = re.sub(r"^P\s*-\s*\d{4}\s*-\s*", "", s).strip()
        if len(s) > n:
            s = s[:n].rstrip("-_ ")
        return s

    A = short(a.stem)
    B = short(b.stem)
    C = short(c.stem)
    return f"M - {tag} - {A} vs {B} vs {C}.md"


def parse_current_abc(md: str) -> Tuple[Optional[str], Optional[str], Optional[str]]:
    def find(label: str) -> Optional[str]:
        m = re.search(rf"^\-\s*{label}:\s*(.+)\s*$", md, flags=re.M)
        return m.group(1).strip() if m else None

    return find("A"), find("B"), find("C")


def append_view_evolution_log(md: str, old_abc: Tuple[str, str, str], new_abc: Tuple[str, str, str]) -> str:
    today = today_iso()
    block = f"""
* {today}

  * 旧观点：A/B/C = {old_abc[0]} / {old_abc[1]} / {old_abc[2]}
  * 新证据：新增/更新同主题论文，A/B/C 刷新为 {new_abc[0]} / {new_abc[1]} / {new_abc[2]}
  * 更新结论：

"""
    m = re.search(r"^##\s+View Evolution Log\s*$", md, flags=re.M)
    if not m:
        return md.rstrip() + "\n\n## View Evolution Log\n" + block

    insert_pos = m.end()
    return md[:insert_pos] + "\n" + block + md[insert_pos:]


def ensure_or_update_mnote(mnote_dir: Path, tag: str, top3: List[Path]) -> Optional[Path]:
    mnote_dir.mkdir(parents=True, exist_ok=True)
    if len(top3) < 3:
        return None

    existing = sorted([p for p in mnote_dir.glob(f"M - {tag} - *.md") if p.is_file()])
    a, b, c = top3
    newA, newB, newC = a.stem, b.stem, c.stem

    if not existing:
        fname = mnote_filename(tag, a, b, c)
        path = mnote_dir / fname
        title = f"{tag}: {newA} vs {newB} vs {newC}"
        write_text(path, render_mnote(title, newA, newB, newC))
        return path

    path = existing[0]
    md = read_text(path)
    curA, curB, curC = parse_current_abc(md)

    if not (curA and curB and curC):
        md2 = md.rstrip() + f"\n\n---\n\n## 当前 A/B/C（自动补齐）\n\n- A: {newA}\n- B: {newB}\n- C: {newC}\n"
        write_text(path, md2)
        return path

    if (curA, curB, curC) != (newA, newB, newC):
        md2 = re.sub(r"^\-\s*A:\s*.*$", f"- A: {newA}", md, flags=re.M)
        md2 = re.sub(r"^\-\s*B:\s*.*$", f"- B: {newB}", md2, flags=re.M)
        md2 = re.sub(r"^\-\s*C:\s*.*$", f"- C: {newC}", md2, flags=re.M)
        md2 = append_view_evolution_log(md2, (curA, curB, curC), (newA, newB, newC))
        write_text(path, md2)

    return path


# -----------------------------
# Radar + Timeline
# -----------------------------

def ensure_radar(root: Path) -> Path:
    p = root / RADAR_PATH
    if p.exists():
        return p

    md = """\
# Radar（长期跟踪页）

| 主题 | 热度 | 证据质量 | 成本变化 | 我的信心 | 最近更新 |
| -- | -- | ---- | ---- | ---- | ---- |
"""
    write_text(p, textwrap.dedent(md).strip() + "\n")
    return p


def parse_radar_table(md: str) -> Tuple[str, List[Dict[str, str]]]:
    lines = md.splitlines()
    start = None
    for i, ln in enumerate(lines):
        if ln.strip().startswith("| 主题 |"):
            start = i
            break
    if start is None:
        return md.rstrip() + "\n", []

    header = "\n".join(lines[:start]).rstrip() + "\n"
    rows: List[Dict[str, str]] = []
    for ln in lines[start + 2:]:
        if not ln.strip().startswith("|"):
            continue
        cols = [c.strip() for c in ln.strip().strip("|").split("|")]
        if len(cols) < 6:
            continue
        rows.append({
            "主题": cols[0],
            "热度": cols[1],
            "证据质量": cols[2],
            "成本变化": cols[3],
            "我的信心": cols[4],
            "最近更新": cols[5],
        })
    return header, rows


def render_radar(header: str, rows: List[Dict[str, str]]) -> str:
    out = [
        header.rstrip(),
        "",
        "| 主题 | 热度 | 证据质量 | 成本变化 | 我的信心 | 最近更新 |",
        "| -- | -- | ---- | ---- | ---- | ---- |",
    ]
    for r in rows:
        out.append(f"| {r['主题']} | {r['热度']} | {r['证据质量']} | {r['成本变化']} | {r['我的信心']} | {r['最近更新']} |")
    return "\n".join(out).strip() + "\n"


def update_radar(root: Path, tags: List[str], note_date: str) -> Path:
    p = ensure_radar(root)
    md = read_text(p)
    header, rows = parse_radar_table(md)

    row_map = {r["主题"]: r for r in rows}
    for t in tags:
        if t not in row_map:
            row_map[t] = {"主题": t, "热度": "1", "证据质量": "", "成本变化": "", "我的信心": "", "最近更新": note_date}
        else:
            try:
                row_map[t]["热度"] = str(int(row_map[t]["热度"] or "0") + 1)
            except Exception:
                row_map[t]["热度"] = "1"
            row_map[t]["最近更新"] = note_date

    rows2 = list(row_map.values())

    def heat(r: Dict[str, str]) -> int:
        try:
            return int(r["热度"])
        except Exception:
            return 0

    rows2.sort(key=lambda r: (-heat(r), r["主题"].lower()))
    write_text(p, render_radar(header, rows2))
    return p


def ensure_timeline(root: Path) -> Path:
    p = root / TIMELINE_PATH
    if p.exists():
        return p
    md = """\
# Timeline（技术演进）

按年份记录关键论文与技术拐点。

"""
    write_text(p, textwrap.dedent(md).strip() + "\n")
    return p


def update_timeline(root: Path, year: str, pnote_path: Path, title: str) -> Path:
    p = ensure_timeline(root)
    md = read_text(p)

    section = f"## {year}"
    bullet = f"- {wikilink_for_pnote(pnote_path)} — {title}"

    if section not in md:
        md = md.rstrip() + f"\n\n{section}\n\n{bullet}\n"
        write_text(p, md.strip() + "\n")
        return p

    if bullet in md:
        return p

    pattern = rf"^##\s+{re.escape(year)}\s*$"
    m = re.search(pattern, md, flags=re.M)
    if not m:
        return p

    start = m.end()
    rest = md[start:]
    m2 = re.search(r"^\s*##\s+", rest, flags=re.M)
    end = start + (m2.start() if m2 else len(rest))

    block = md[start:end].rstrip() + "\n" + bullet + "\n"
    md2 = md[:start] + block + md[end:]
    write_text(p, md2.strip() + "\n")
    return p


# -----------------------------
# Tag inference (fallback)
# -----------------------------

KEYWORD_TAGS = [
    (r"\bagent(s)?\b|tool\s*use|function\s*calling", "Agent"),
    (r"\brag\b|retrieval\-augmented|retrieval augmented", "RAG"),
    (r"\bmoe\b|mixture of experts", "MoE"),
    (r"\brlhf\b|preference optimization|dpo\b", "Alignment"),
    (r"\bevaluation\b|benchmark", "Evaluation"),
    (r"\bcompiler\b|kernel|cuda|inference", "Infrastructure"),
    (r"\bmultimodal\b|vision|audio", "Multimodal"),
    (r"\bcompression\b|quantization|distillation", "Optimization"),
    (r"\blong context\b|context length", "LongContext"),
    (r"\bsafety\b|jailbreak|red teaming", "Safety"),
]


def infer_tags_if_empty(tags: List[str], paper: Paper) -> List[str]:
    if tags:
        return tags
    text = f"{paper.title}\n{paper.abstract}".lower()
    out = []
    for pat, tg in KEYWORD_TAGS:
        if re.search(pat, text, flags=re.I):
            out.append(tg)
    return out if out else ["Unsorted"]


# -----------------------------
# Main flow
# -----------------------------

def main(argv: Optional[List[str]] = None) -> int:
    import sys
    parser = argparse.ArgumentParser(description="AI Research OS - Full Flow (P+C+M+Radar+Timeline + optional AI draft)")
    parser.add_argument("input", help="arXiv id/URL or DOI/doi.org URL")
    parser.add_argument("--root", default="AI-Research", help="Root folder for your research OS")
    parser.add_argument("--category", default="02-Models", help="Folder under root to place P-Note")
    parser.add_argument("--tags", default="", help="Comma-separated tags (recommended), e.g. LLM,Agent,RAG")
    parser.add_argument("--concept-dir", default="01-Foundations", help="Folder under root to place C-Notes")
    parser.add_argument("--comparison-dir", default="00-Radar", help="Folder under root to place M-Notes")
    parser.add_argument("--max-pages", type=int, default=None, help="Max PDF pages to extract")

    # ✅ 新增：本地 PDF（付费墙/订阅论文）
    parser.add_argument("--pdf", default="", help="Path to a local PDF (manual download). If set, skip PDF download.")

    # ✅ 新增：OCR（扫描版/图片版 PDF）
    parser.add_argument("--ocr", action="store_true", help="Enable OCR fallback per page (scanned PDFs).")
    parser.add_argument("--ocr-lang", default="chi_sim+eng", help="Tesseract language (default: chi_sim+eng).")
    parser.add_argument("--ocr-zoom", type=float, default=2.0, help="OCR render zoom (default: 2.0).")
    parser.add_argument("--no-pdfminer", action="store_true", help="Disable pdfminer fallback.")

    # AI draft options
    parser.add_argument("--ai", action="store_true", help="Use AI to draft-fill P-Note sections (adds an AI draft block)")
    parser.add_argument("--api-key", default="", help="LLM API key (or set OPENAI_API_KEY env)")
    parser.add_argument("--model", default="qwen3.5-plus", help="LLM model name (OpenAI-compatible)")
    parser.add_argument("--base-url", default="https://dashscope.aliyuncs.com/compatible-mode/v1", help="OpenAI-compatible base url")
    parser.add_argument("--ai-max-chars", type=int, default=24000, help="Max chars of extracted text sent to AI")
    args = parser.parse_args(argv if argv is not None else sys.argv[1:])

    raw_in = args.input.strip()
    root = Path(args.root).resolve()
    ensure_research_tree(root)

    category_dir = root / args.category
    category_dir.mkdir(parents=True, exist_ok=True)

    concept_dir = root / args.concept_dir
    concept_dir.mkdir(parents=True, exist_ok=True)

    comparison_dir = root / args.comparison_dir
    comparison_dir.mkdir(parents=True, exist_ok=True)

    paper: Optional[Paper] = None
    arxiv_id = normalize_arxiv_id(raw_in)

    # DOI flow: prioritize arXiv DOI before Crossref
    if is_probably_doi(raw_in):
        doi = normalize_doi(raw_in)
        arxiv_from_doi = normalize_arxiv_id(doi)
        if arxiv_from_doi:
            paper = fetch_arxiv_metadata(arxiv_from_doi)
        else:
            doi_paper, maybe_arxiv = fetch_crossref_metadata(doi)
            paper = fetch_arxiv_metadata(maybe_arxiv) if maybe_arxiv else doi_paper

    elif arxiv_id:
        paper = fetch_arxiv_metadata(arxiv_id)

    else:
        m = re.search(r"(\d{4}\.\d{4,5}(v\d+)?)", raw_in)
        if m:
            paper = fetch_arxiv_metadata(m.group(1))
        else:
            # 允许占位 input（例如 "test"）配合 --pdf
            paper = Paper(
                source="doi" if is_probably_doi(raw_in) else "doi",
                uid=raw_in,
                title=raw_in,
                authors=[],
                abstract="",
                published="",
                updated="",
                abs_url=DOI_RESOLVER + raw_in,
                pdf_url="",
                primary_category="",
            )

    tags = [t.strip() for t in args.tags.split(",") if t.strip()]
    tags = infer_tags_if_empty(tags, paper)

    note_date = paper.published or today_iso()
    year = (note_date[:4] if len(note_date) >= 4 else str(dt.date.today().year))
    title_slug = slugify_title(paper.title)

    pnote_name = f"P - {year} - {title_slug}.md"
    pnote_path = category_dir / pnote_name

    # 默认：把下载 PDF 放到 _assets/{uid}/ 下面
    assets_dir = category_dir / "_assets" / safe_uid(paper.uid)
    default_pdf_path = assets_dir / (safe_uid(paper.uid) + ".pdf")

    extracted_sections_md = ""
    extracted_text_for_ai = ""
    pdf_downloaded = False

    # ✅ 关键改动：如果给了 --pdf，就跳过网络下载，直接解析本地 PDF
    if args.pdf:
        pdf_path = Path(args.pdf).expanduser().resolve()
        if not pdf_path.exists():
            raise FileNotFoundError(f"--pdf not found: {pdf_path}")
        pdf_downloaded = True
        # 如果用户是本地 PDF，PDF URL 也记一下（可选）
        paper.pdf_url = str(pdf_path)
    else:
        pdf_path = default_pdf_path
        if paper.pdf_url:
            try:
                download_pdf(paper.pdf_url, pdf_path)
                pdf_downloaded = True
            except Exception as e:
                extracted_sections_md = f"_PDF 下载失败：{e}_"
        else:
            extracted_sections_md = "_未提供可直接下载的 PDF 链接（常见于 DOI-only 元数据），已跳过 PDF 抽取。_"

    if pdf_downloaded:
        try:
            txt = extract_pdf_text_hybrid(
                pdf_path,
                max_pages=args.max_pages,
                ocr=args.ocr,
                ocr_lang=args.ocr_lang,
                ocr_zoom=args.ocr_zoom,
                use_pdfminer_fallback=(not args.no_pdfminer),
            )
            extracted_text_for_ai = txt
            sections = segment_into_sections(txt)
            extracted_sections_md = format_section_snippets(sections)
        except Exception as e:
            extracted_sections_md = f"_PDF 抽取失败：{e}_"

    # AI draft generation (optional)
    ai_draft_md = ""
    if args.ai:
        ctx = extracted_text_for_ai.strip() or (paper.abstract or "")
        ctx = ctx[: max(1000, args.ai_max_chars)]
        try:
            ai_draft_md = ai_generate_pnote_draft(
                paper=paper,
                tags=tags,
                extracted_text=ctx[: args.ai_max_chars],
                base_url=args.base_url,
                api_key=args.api_key,
                model=args.model,
            )
        except Exception as e:
            ai_draft_md = (
                "> AI Draft（生成失败，需人工核验）\n\n"
                f"- 错误：{e}\n"
                "- 建议：检查 OPENAI_API_KEY / --api-key / --base-url / --model\n"
            )

    # Write P-Note
    write_text(pnote_path, render_pnote(paper, tags, extracted_sections_md, ai_draft_md=ai_draft_md))

    # C-Notes create/update + link P-Note
    cnote_paths: List[Path] = []
    for t in tags:
        cpath = ensure_cnote(concept_dir, t)
        update_cnote_links(cpath, pnote_path)
        cnote_paths.append(cpath)

    # Radar update
    radar_path = update_radar(root, tags, note_date)

    # Timeline update
    timeline_path = update_timeline(root, year, pnote_path, paper.title)

    # M-Note trigger/update
    tag_map = pnotes_by_tag(root)
    mnote_paths: List[Path] = []
    for t in tags:
        top3 = pick_top3_pnotes_for_tag(t, tag_map)
        if top3:
            mpath = ensure_or_update_mnote(comparison_dir, t, top3)
            if mpath:
                cpath = concept_dir / f"C - {t}.md"
                cmd = read_text(cpath)
                mlink = f"[[{mpath.stem}]]"
                cmd2 = upsert_link_under_heading(cmd, "关联笔记", mlink)
                write_text(cpath, cmd2)
                mnote_paths.append(mpath)

    # Print summary
    print("OK: AI Research OS Flow Done")
    print(f"- P-Note: {pnote_path}")
    if pdf_downloaded:
        print(f"- PDF   : {pdf_path}")
    else:
        print("- PDF   : (not downloaded)")
    print(f"- Radar : {radar_path}")
    print(f"- Timeline: {timeline_path}")

    if cnote_paths:
        print("- C-Notes:")
        for p in cnote_paths:
            print(f"  - {p}")

    if mnote_paths:
        print("- M-Notes:")
        for p in mnote_paths:
            print(f"  - {p}")
    else:
        print("- M-Notes: (no tag reached 3 P-Notes yet)")

    if args.ai:
        print("- AI Draft: ENABLED (see P-Note section: 'AI 自动初稿（待核验）')")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())