"""
CLI command: postprocess — Run 6-stage research deep dive pipeline.

Usage:
    airos postprocess 2604.22755
    airos postprocess 2604.22755 --stages analysis insight
    airos postprocess 2604.22755 --skip-llm
    airos postprocess 2604.22755 --root AI-Research --tags LLM,RAG
"""
from __future__ import annotations

import argparse
from pathlib import Path
from typing import Optional, cast

from cli._shared import get_db, print_info, print_error, print_success, print_warning
from llm.postprocess import ResearchDeepDivePipeline, PostStage, make_llm_config


def _resolve_pdf_path(rec, root: Path) -> Optional[Path]:
    """Resolve PDF path from paper record or cache."""
    # Try explicit pdf_path field
    pdf_path_str = getattr(rec, "pdf_path", "") or ""
    if pdf_path_str and Path(pdf_path_str).exists():
        return Path(pdf_path_str)
    # Try pdf_url — download if needed
    pdf_url = getattr(rec, "pdf_url", "") or ""
    if pdf_url:
        cache_dir = root / "cache"
        cache_dir.mkdir(exist_ok=True)
        uid = getattr(rec, "id", "")
        cached = cache_dir / f"{uid}.pdf"
        if cached.exists():
            return cached
        try:
            from pdf.extract import download_pdf
            download_pdf(pdf_url, cached)
            return cached
        except Exception:
            pass
    return None


def _build_postprocess_parser(subparsers) -> argparse.ArgumentParser:
    """Build the postprocess subcommand parser."""
    p = subparsers.add_parser(
        "postprocess",
        help="Run deep analysis pipeline on a paper",
        description=(
            "Run the 6-stage research deep dive pipeline on a paper: "
            "PAPER_ANALYSIS → BENCHMARK → CROSS_REFERENCE → INSIGHT → KG_SYNC → PNODE_UPDATE. "
            "Gracefully degrades when no LLM key is available."
        ),
    )
    p.add_argument("paper_id", help="Paper ID to analyze")
    p.add_argument(
        "--root", default="AI-Research",
        help="Root folder for your research OS (default: AI-Research)",
    )
    p.add_argument(
        "--category", default="02-Models",
        help="Folder under root where P-Note lives (default: 02-Models)",
    )
    p.add_argument(
        "--stages", nargs="+", default=None,
        choices=[s.value for s in PostStage],
        help="Specific stages to run (default: all six)",
    )
    p.add_argument(
        "--skip-llm", action="store_true",
        help="Skip LLM usage — keyword-only analysis",
    )
    p.add_argument(
        "--tags", default="",
        help="Comma-separated tags override (default: from DB record)",
    )
    p.add_argument(
        "--structured", action="store_true",
        help="Use structured PDF extraction (tables/math separated) for citation-grounded analysis",
    )
    return p  # type: ignore[no-any-return]


def _run_postprocess(args: argparse.Namespace) -> int:
    """Run postprocess command."""
    paper_id = args.paper_id
    root = Path(args.root)

    # Resolve LLM config
    llm_config: Optional[dict] = None if args.skip_llm else make_llm_config()
    if llm_config:
        print_info(f"LLM mode: {llm_config.get('model', 'unknown')}")
    else:
        print_warning("No LLM key — running in degraded mode (keyword-only)")

    # Resolve stages
    stages = None
    if args.stages:
        stages = [PostStage(s) for s in args.stages]

    # Get DB and paper data
    db = get_db()
    db.init()

    paper = None
    extracted_text = ""
    tags: list[str] = []
    pnote_path: Optional[Path] = None

    try:
        rec = db.get_paper(paper_id)
        if rec:
            extracted_text = getattr(rec, "extracted_text", "") or ""
            tags = (
                [t.strip() for t in args.tags.split(",") if t.strip()]
                if args.tags
                else (getattr(rec, "tags", []) or [])
            )

            from core import Paper
            paper = Paper(
                source=getattr(rec, "source", "arxiv"),
                uid=getattr(rec, "id", paper_id),
                title=getattr(rec, "title", paper_id),
                authors=getattr(rec, "authors", []),
                abstract=getattr(rec, "abstract", "") or "",
                published=getattr(rec, "published", "") or "",
                updated=getattr(rec, "updated", "") or "",
                abs_url=getattr(rec, "abs_url", ""),
                pdf_url=getattr(rec, "pdf_url", ""),
                primary_category=getattr(rec, "primary_category", ""),
            )

            # Guess P-note path from title
            from core.basics import slugify_title
            year = (
                (getattr(rec, "published", "") or "")[:4]
                or "0000"
            )
            guessed = (
                root
                / (args.category or "02-Models")
                / f"P - {year} - {slugify_title(paper.title)}.md"
            )
            if guessed.exists():
                pnote_path = guessed
    except Exception as e:
        print_warning(f"Paper {paper_id} not in DB: {e}")
        print_info("Some stages will degrade (no DB data)")

    # Structured PDF extraction (requires PDF and LLM)
    structured_content = None
    if args.structured:
        if not llm_config:
            print_warning("--structured requires LLM; falling back to plain text")
        else:
            pdf_path = _resolve_pdf_path(rec, root) if rec else None
            if pdf_path and pdf_path.exists():
                try:
                    from pdf.extract import extract_pdf_structured
                    print_info(f"Extracting structured PDF: {pdf_path.name}")
                    structured_content = extract_pdf_structured(pdf_path)
                    extracted_text = "\n".join(
                        b.text for b in structured_content.text_blocks
                    )
                    print_success(f"Extracted {len(structured_content.text_blocks)} text blocks")
                except Exception as e:
                    print_warning(f"Structured extraction failed: {e}")
                    print_info("Falling back to plain text from DB")
            else:
                print_warning(f"PDF not found for {paper_id}; cannot use --structured")
                print_info("Hint: re-ingest the paper with --structured first")

    # Run pipeline
    pipeline = ResearchDeepDivePipeline(db=db, data_dir=root)
    result = pipeline.run(
        paper_id=paper_id,
        extracted_text=extracted_text,
        paper=paper,
        tags=tags,
        pnote_path=pnote_path,
        stages=stages,
        llm_config=llm_config,
        structured_content=structured_content if args.structured else None,
    )

    # Report
    print()
    print_success(f"Pipeline complete — {result.summary}")

    if result.stages_completed:
        print_info(f"  + {', '.join(result.stages_completed)}")
    if result.stages_failed:
        last_errors = [
            (cast(dict, result.stage_results).get(s, {}) or {}).get("error", "")[:80]
            for s in result.stages_failed
            if (cast(dict, result.stage_results).get(s, {}) or {}).get("error")
        ]
        for stage, err in zip(result.stages_failed, last_errors):
            print_error(f"  x {stage}: {err}")
    if result.pnote_updated and pnote_path:
        print_info(f"  -> P-note: {pnote_path.name}")

    return 0 if result.all_succeeded else 1
