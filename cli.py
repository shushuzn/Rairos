#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""AI Research OS CLI entry point."""

import argparse
import os
import sys
from pathlib import Path
from typing import List, Optional

from core import DOI_RESOLVER, Paper, today_iso
from core.basics import ensure_research_tree, safe_uid, slugify_title
from llm.generate import ai_generate_pnote_draft
from notes.cnote import auto_fill_cnotes_with_ai, ensure_cnote, update_cnote_links
from notes.frontmatter import parse_date_from_frontmatter, parse_frontmatter, parse_tags_from_frontmatter
from notes.mnote import ensure_or_update_mnote, pick_top3_pnotes_for_tag
from notes.pnotes import pnotes_by_tag
from parsers.arxiv import fetch_arxiv_metadata
from parsers.crossref import fetch_crossref_metadata
from parsers.input_detection import is_probably_doi, normalize_arxiv_id, normalize_doi
from pdf.extract import download_pdf, extract_pdf_text_hybrid, extract_pdf_structured
from renderers.pnote import render_pnote
from sections.segment import format_section_snippets, segment_into_sections, segment_structured, format_tables_markdown, format_math_markdown
from updaters.radar import update_radar
from updaters.timeline import update_timeline

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

import re


def infer_tags_if_empty(tags: List[str], paper: Paper) -> List[str]:
    if tags:
        return tags
    text = f"{paper.title}\n{paper.abstract}".lower()
    out = []
    for pat, tg in KEYWORD_TAGS:
        if re.search(pat, text, flags=re.I):
            out.append(tg)
    return out if out else ["Unsorted"]


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="AI Research OS - Full Flow (P+C+M+Radar+Timeline + optional AI draft)")
    parser.add_argument("input", help="arXiv id/URL or DOI/doi.org URL")
    parser.add_argument("--root", default="AI-Research", help="Root folder for your research OS")
    parser.add_argument("--category", default="02-Models", help="Folder under root to place P-Note")
    parser.add_argument("--tags", default="", help="Comma-separated tags (recommended), e.g. LLM,Agent,RAG")
    parser.add_argument("--concept-dir", default="01-Foundations", help="Folder under root to place C-Notes")
    parser.add_argument("--comparison-dir", default="00-Radar", help="Folder under root to place M-Notes")
    parser.add_argument("--max-pages", type=int, default=None, help="Max PDF pages to extract")

    # Local PDF (paywalled/subscription papers)
    parser.add_argument("--pdf", default="", help="Path to a local PDF (manual download). If set, skip PDF download.")

    # OCR (scanned/image PDFs)
    parser.add_argument("--ocr", action="store_true", help="Enable OCR fallback per page (scanned PDFs).")
    parser.add_argument("--ocr-lang", default="chi_sim+eng", help="Tesseract language (default: chi_sim+eng).")
    parser.add_argument("--ocr-zoom", type=float, default=2.0, help="OCR render zoom (default: 2.0).")
    parser.add_argument("--no-pdfminer", action="store_true", help="Disable pdfminer fallback.")

    # Structured PDF extraction
    parser.add_argument("--structured", action="store_true", help="Use structured PDF extraction (tables/math separated).")

    # AI draft options
    parser.add_argument("--ai", action="store_true", help="Use AI to draft-fill P-Note sections (adds an AI draft block)")
    parser.add_argument("--ai-cnote", action="store_true", help="AI-fill all C-Notes from existing P-Notes (standalone mode; skips paper processing)")
    parser.add_argument("--ai-max-papers", type=int, default=10, help="Max P-notes to feed per C-note (default: 10)")
    parser.add_argument("--api-key", default="", help="LLM API key (or set OPENAI_API_KEY env)")
    parser.add_argument("--model", default="qwen3.5-plus", help="LLM model name (OpenAI-compatible)")
    parser.add_argument("--base-url", default="https://dashscope.aliyuncs.com/compatible-mode/v1", help="OpenAI-compatible base url")
    parser.add_argument("--ai-max-chars", type=int, default=24000, help="Max chars of extracted text sent to AI")
    args = parser.parse_args(argv if argv is not None else sys.argv[1:])

    # Standalone C-note AI fill mode (no paper processing needed)
    if args.ai_cnote:
        api_key = args.api_key or os.environ.get("OPENAI_API_KEY", "")
        if not api_key:
            print("ERROR: --api-key or OPENAI_API_KEY required for --ai-cnote")
            return 1
        root = Path(args.root).resolve()
        results = auto_fill_cnotes_with_ai(
            root=root,
            api_key=api_key,
            base_url=args.base_url,
            model=args.model,
            min_papers=1,
        )
        print(f"OK: C-note AI fill done ({len(results)} concepts)")
        for concept, status in results:
            print(f"  - {concept}: {status}")
        return 0

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
            # Allow placeholder input (e.g. "test") with --pdf
            paper = Paper(
                source="doi",
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
    year = (note_date[:4] if len(note_date) >= 4 else str(__import__('datetime').date.today().year))
    title_slug = slugify_title(paper.title)

    pnote_name = f"P - {year} - {title_slug}.md"
    pnote_path = category_dir / pnote_name

    # Default: download PDF to _assets/{uid}/
    assets_dir = category_dir / "_assets" / safe_uid(paper.uid)
    default_pdf_path = assets_dir / (safe_uid(paper.uid) + ".pdf")

    extracted_sections_md = ""
    extracted_text_for_ai = ""
    pdf_downloaded = False
    table_md = ""
    math_md = ""

    if args.pdf:
        pdf_path = Path(args.pdf).expanduser().resolve()
        if not pdf_path.exists():
            raise FileNotFoundError(f"--pdf not found: {pdf_path}")
        pdf_downloaded = True
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
            if args.structured:
                sdoc = extract_pdf_structured(
                    pdf_path,
                    max_pages=args.max_pages,
                )
                sections = segment_structured(sdoc)
                extracted_sections_md = format_section_snippets(sections)
                table_md = format_tables_markdown(sdoc)
                math_md = format_math_markdown(sdoc)
                # Provide text to AI draft
                extracted_text_for_ai = "\n".join(b.text for b in sdoc.text_blocks)
            else:
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
                table_md = ""
                math_md = ""
        except Exception as e:
            extracted_sections_md = f"_PDF 抽取失败：{e}_"
            table_md = ""
            math_md = ""

    # AI draft generation (optional)
    ai_draft_md = ""
    parsed_ai = None
    if args.ai:
        ctx = extracted_text_for_ai.strip() or (paper.abstract or "")
        ctx = ctx[: max(1000, args.ai_max_chars)]
        try:
            from llm.parse import parse_ai_pnote_draft
            raw_draft = ai_generate_pnote_draft(
                paper=paper,
                tags=tags,
                extracted_text=ctx[: args.ai_max_chars],
                base_url=args.base_url,
                api_key=args.api_key,
                model=args.model,
            )
            sections_dict, rubric_dict, raw_draft = parse_ai_pnote_draft(raw_draft)
            ai_draft_md = raw_draft  # full raw output for reference
            parsed_ai = (sections_dict, rubric_dict)
        except Exception as e:
            ai_draft_md = (
                "> AI Draft（生成失败，需人工核验）\n\n"
                f"- 错误：{e}\n"
                "- 建议：检查 OPENAI_API_KEY / --api-key / --base-url / --model\n"
            )

    from core.basics import write_text
    write_text(pnote_path, render_pnote(paper, tags, extracted_sections_md, ai_draft_md=ai_draft_md, table_md=table_md, math_md=math_md, parsed_ai=parsed_ai))

    # C-Notes create/update + link P-Note
    cnote_paths = []
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
    mnote_paths = []
    for t in tags:
        top3 = pick_top3_pnotes_for_tag(t, tag_map)
        if top3:
            mpath = ensure_or_update_mnote(comparison_dir, t, top3)
            if mpath:
                from core.basics import read_text as _read_text
                cpath = concept_dir / f"C - {t}.md"
                cmd = _read_text(cpath)
                mlink = f"[[{mpath.stem}]]"
                from notes.cnote import upsert_link_under_heading
                cmd2 = upsert_link_under_heading(cmd, "关联笔记", mlink)
                from core.basics import write_text as _write_text
                _write_text(cpath, cmd2)
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
    sys.exit(main())
