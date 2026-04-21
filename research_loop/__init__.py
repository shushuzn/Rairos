"""Autonomous research loop: search → download → extract → summarize → save.

This module orchestrates the core research workflow:
1. Search arXiv for papers matching a keyword query
2. Download PDFs
3. Extract text
4. Generate structured research notes via LLM
5. Save markdown report to disk

Inspired by karpathy/autoresearch but adapted for the paper-management workflow
already present in ai_research_os.
"""
from __future__ import annotations

import datetime as dt
import os
import sys
import tempfile
import time  # tests mock research_loop.time.sleep
import logging
import warnings

logger = logging.getLogger(__name__)
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple, cast

from core import Paper
from core.basics import ensure_research_tree, get_default_concept_dir, safe_uid, slugify_title
from llm.generate import ai_generate_pnote_draft, estimate_cost
from llm.parse import parse_ai_pnote_draft, extract_rubric_scores
from parsers.arxiv_search import search_arxiv
from pdf.extract import download_pdf as _download_pdf, extract_pdf_text
from updaters.radar import flush_radar, update_radar
from updaters.timeline import update_timeline


# Default: max papers to process in one run
DEFAULT_LIMIT = 5


def run_research(
    query: str,
    limit: int = DEFAULT_LIMIT,
    output_dir: Optional[Path] = None,
    max_text_len: int = 8000,
    base_url: Optional[str] = None,
    api_key: Optional[str] = None,
    model: str = "gpt-4o-mini",
    tags: Optional[List[str]] = None,
    download_pdfs: bool = True,
    skip_existing: bool = True,
    verbose: bool = False,
    root: Optional[Path] = None,
) -> List[Path]:
    """
    Run the autonomous research loop for a keyword query.

    Args:
        query: Search keyword/phrase (passed to arXiv API as `all:` query)
        limit: Max papers to process (default 5)
        output_dir: Where to save reports. Defaults to ~/ai_research/query-slug/
        max_text_len: Max characters of extracted text to send to LLM (default 8000)
        base_url: LLM API base URL (e.g. https://api.openai.com/v1).
                  If None, reads from OPENAI_BASE_URL env var.
        api_key: LLM API key. If None, reads from OPENAI_API_KEY env var.
        model: Model name (default gpt-4o-mini)
        tags: Tags to assign to each paper note (auto-inferred if empty)
        download_pdfs: Whether to download PDFs (default True)
        skip_existing: Skip papers whose note already exists (default True)
        verbose: Print progress (default False)
        root: If provided, research OS root is used to integrate notes into
              the research tree (C-notes, Radar, Timeline). Default None (standalone).

    Returns:
        List of output markdown file paths created
    """
    # Resolve API credentials
    api_key = api_key or os.environ.get("OPENAI_API_KEY", "")
    base_url = base_url or os.environ.get("OPENAI_BASE_URL", "https://api.openai.com/v1")

    if not api_key:
        warnings.warn(
            "OPENAI_API_KEY not set. Skipping AI draft generation "
            "(notes will be created with metadata only).",
            stacklevel=2,
        )

    # Resolve output directory
    if output_dir is None:
        home = Path.home()
        slug = slugify_title(query)[:50]
        output_dir = home / "ai_research" / slug

    output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    # --- Step 1: Search arXiv ---
    if verbose:
        print(f"[research] Searching arXiv for: {query}")

    try:
        papers = search_arxiv(query, max_results=limit)
    except RuntimeError as e:
        print(f"[research] Search failed: {e}", file=sys.stderr)
        return []

    if not papers:
        print(f"[research] No papers found for query: {query}")
        return []

    if verbose:
        print(f"[research] Found {len(papers)} papers")

    output_paths: List[Path] = []

    # --- Step 2: Process each paper in parallel ---
    def _process_one_paper(
        args: Tuple[Paper, int, int, Path, bool, bool, int, str, str, str, List[str], Optional[Path]],
    ) -> Tuple[Optional[Path], Optional[Path], Optional[str]]:
        """Worker: download → extract → LLM → write note. Returns (note_path, pdf_path, error_reason)."""
        paper, i, total, out_dir, do_download, do_skip, max_txt, b_url, a_key, mod, tgs, rl_root = args
        title_short = paper.title[:60] + ("..." if len(paper.title) > 60 else "")
        if verbose:
            print(f"[research] [{i}/{total}] {paper.uid}: {title_short}")

        slug = slugify_title(paper.title)[:60]
        note_path = out_dir / f"{safe_uid(paper.uid)}_{slug}.md"

        if do_skip and note_path.exists():
            if verbose:
                print(f"  [skip] Already exists: {note_path.name}")
            return note_path, None, None  # skip

        pdf_path: Optional[Path] = None
        extracted_text = ""
        err: Optional[str] = None

        if do_download and paper.pdf_url:
            extracted_text = ""
            for attempt in range(2):
                try:
                    pdf_path = Path(tempfile.gettempdir()) / f"{safe_uid(paper.uid)}.pdf"
                    _download_pdf(paper.pdf_url, pdf_path, timeout=60)
                    if verbose:
                        print(f"  [pdf] Downloaded: {pdf_path.name} ({pdf_path.stat().st_size / 1024:.0f} KB)")
                    extracted_text = extract_pdf_text(pdf_path, max_pages=15)
                    if len(extracted_text) > max_txt:
                        extracted_text = extracted_text[:max_txt] + "\n\n[... truncated ...]"
                    if verbose and extracted_text:
                        print(f"  [text] Extracted {len(extracted_text)} chars")
                    break  # success
                except Exception as e:
                    extracted_text = ""
                    if attempt == 0:
                        time.sleep(1)  # brief pause before retry
                        continue  # retry once
                    err = "pdf_failed"
                    warnings.warn(f"PDF download/extract failed for {paper.uid} after retry: {e}", stacklevel=2)

        note_tags = tgs or []
        draft = ""
        rubric: Dict[str, Any] = {}
        cost_info: Dict[str, Any] = {}

        if a_key and extracted_text:
            try:
                if verbose:
                    print("  [llm] Generating draft (streaming)...")
                input_for_llm = f"Title: {paper.title}\nAbstract: {paper.abstract or ''}\n\nExtracted text:\n{extracted_text[:8000]}"
                draft = ai_generate_pnote_draft(
                    paper=paper,
                    tags=note_tags,
                    extracted_text=extracted_text,
                    base_url=b_url,
                    api_key=a_key,
                    model=mod,
                    stream=True,
                    verbose=verbose,
                )
                sections, rubric, _ = parse_ai_pnote_draft(draft)
                cost_info = estimate_cost(mod, input_for_llm, draft)
                if verbose:
                    print(
                        f"  [llm] Draft generated ({len(draft)} chars, "
                        f"≈{cost_info.get('total_tokens', 0)} tokens, "
                        f"${cost_info.get('total_cost_usd', 0):.4f})"
                    )
            except Exception as e:
                warnings.warn(f"LLM draft generation failed for {paper.uid}: {e}", stacklevel=2)
                draft = ""
                err = err or "llm_failed"
        elif not a_key:
            if verbose:
                print("  [skip] No API key — metadata-only note")
        else:
            if verbose:
                print("  [skip] No extracted text — metadata-only note")

        note_content = _build_research_note(paper, draft, rubric, note_tags)
        note_path.write_text(note_content, encoding="utf-8")

        # --- Integrate into research tree ---
        if rl_root:
            try:
                _integrate_into_tree(rl_root, note_path, paper, note_tags)
            except Exception as e:
                warnings.warn(f"Failed to integrate into research tree: {e}", stacklevel=2)

        if verbose:
            rubric_scores = extract_rubric_scores(rubric)
            print(
                f"  [saved] {note_path.name}"
                + (f" [novelty={rubric_scores.get('novelty', '?')}]" if rubric_scores else "")
            )

        return note_path, pdf_path, err

    def _integrate_into_tree(root: Path, note_path: Path, paper: Paper, note_tags: List[str]) -> None:
        """Add a newly-created research note into the research OS tree."""
        # Lazy import to avoid circular import
        from notes.cnote import ensure_cnote, update_cnote_links

        ensure_research_tree(root)

        # Timeline: append under the paper's year
        year = paper.published[:4] if paper.published else dt.date.today().isoformat()[:4]
        update_timeline(root, year, note_path, paper.title)

        # For each tag: create/verify C-note and link it
        for tag in note_tags:
            concept_dir = root / get_default_concept_dir()
            cpath = ensure_cnote(concept_dir, tag)
            update_cnote_links(cpath, note_path)

        # Radar: accumulate bumps in memory, caller flushes after all papers
        if note_tags:
            update_radar(root, note_tags, year, flush=False)

    # Build work items (filter skipped before spawning threads)
    work_items: List[Tuple[Paper, int, int, Path, bool, bool, int, str, str, str, List[str], Optional[Path]]] = []
    for i, paper in enumerate(papers, 1):
        slug = slugify_title(paper.title)[:60]
        note_path = output_dir / f"{safe_uid(paper.uid)}_{slug}.md"
        if skip_existing and note_path.exists():
            output_paths.append(note_path)
            continue
        work_items.append((paper, i, len(papers), output_dir, download_pdfs, skip_existing, max_text_len, base_url, api_key, model, tags or [], root))

    if work_items:
        processed = 0
        failed = 0
        error_reasons: dict[str, int] = {}
        skipped = len(papers) - len(work_items)
        # Parallel: up to 3 papers simultaneously (PDF downloads are the bottleneck)
        with ThreadPoolExecutor(max_workers=3) as ex:
            futures = {ex.submit(_process_one_paper, item): item for item in work_items}
            for future in as_completed(futures):
                note_path, _, err = future.result()  # type: ignore[assignment]
                if note_path:
                    output_paths.append(note_path)
                    if err:
                        failed += 1
                        error_reasons[err] = error_reasons.get(err, 0) + 1
                    else:
                        processed += 1
        # Summary report
        total = len(papers)
        print(f"\n[research] Done: {processed}/{total} processed, {failed} failed, {skipped} skipped")
        for reason, count in error_reasons.items():
            print(f"  [{reason}] {count} paper(s)")

    # Flush accumulated radar updates in one write
    if root:
        flush_radar(root)

    return output_paths


def _build_research_note(
    paper: Paper,
    draft: str,
    rubric: dict,
    tags: List[str],
) -> str:
    """Build a research note markdown file from paper metadata + AI draft."""
    lines = [
        f"# {paper.title}",
        "",
        f"**UID:** `{paper.uid}`",
        f"**Source:** [{paper.source}]({paper.abs_url})",
        f"**PDF:** [{paper.pdf_url}]({paper.pdf_url})",
        f"**Published:** {paper.published or 'N/A'}",
        f"**Updated:** {paper.updated or 'N/A'}",
        "",
    ]

    if paper.authors:
        author_str = ", ".join(paper.authors[:5])
        if len(paper.authors) > 5:
            author_str += f" et al. (+{len(paper.authors) - 5} more)"
        lines.append(f"**Authors:** {author_str}")
        lines.append("")

    if paper.categories:
        lines.append(f"**Categories:** {paper.categories}")
        lines.append("")

    if tags:
        lines.append(f"**Tags:** {', '.join(tags)}")
        lines.append("")

    lines.append("## Abstract")
    lines.append("")
    lines.append(paper.abstract or "_No abstract available._")
    lines.append("")

    if paper.comment:
        lines.append(f"**arXiv Comment:** {paper.comment}")
        lines.append("")

    if paper.journal_ref:
        lines.append(f"**Journal Ref:** {paper.journal_ref}")
        lines.append("")

    if draft:
        lines.append("---")
        lines.append("")
        lines.append("## AI Research Note")
        lines.append("")
        lines.append(draft)
        lines.append("")

        rubric_scores = extract_rubric_scores(rubric)
        if rubric_scores:
            lines.append("---")
            lines.append("")
            lines.append("### Rubric Scores")
            for k, v in rubric_scores.items():
                bar = "★" * v + "☆" * (5 - v)
                lines.append(f"- **{k.capitalize()}:** {bar} ({v}/5)")
            if rubric.get("overall"):
                lines.append(f"- **Overall:** {rubric['overall']}")
            lines.append("")
    else:
        lines.append("---")
        lines.append("")
        lines.append("_Note: Set `OPENAI_API_KEY` to enable AI draft generation._")
        lines.append("")

    lines.append("---")
    lines.append(f"_Generated by ai-research-os on {dt.date.today().isoformat()}_")

    return "\n".join(lines)


# ─── Async research loop ────────────────────────────────────────────────────────


async def arun_research(
    query: str,
    limit: int = DEFAULT_LIMIT,
    output_dir: Optional[Path] = None,
    max_text_len: int = 8000,
    base_url: Optional[str] = None,
    api_key: Optional[str] = None,
    model: str = "gpt-4o-mini",
    tags: Optional[List[str]] = None,
    download_pdfs: bool = True,
    skip_existing: bool = True,
    verbose: bool = False,
    root: Optional[Path] = None,
    progress_callback: Optional[Callable[..., Any]] = None,
) -> List[Path]:
    """
    Async version of run_research. Runs PDF download, text extraction,
    LLM draft, and note writing concurrently with a semaphore cap of 3.

    Returns the same output as run_research (list of created note paths).
    """
    import asyncio
    import tempfile as _tempfile

    from llm.client_async import call_llm_chat_completions_async
    from pdf.extract_async import download_pdf_async

    # Resolve API credentials
    api_key = api_key or os.environ.get("OPENAI_API_KEY", "")
    base_url = base_url or os.environ.get("OPENAI_BASE_URL", "https://api.openai.com/v1")

    if not api_key:
        warnings.warn(
            "OPENAI_API_KEY not set. Skipping AI draft generation "
            "(notes will be created with metadata only).",
            stacklevel=2,
        )

    # Resolve output directory
    if output_dir is None:
        home = Path.home()
        slug = slugify_title(query)[:50]
        output_dir = home / "ai_research" / slug

    output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    # --- Step 1: Search arXiv ---
    if verbose:
        print(f"[research] Searching arXiv for: {query}")

    try:
        papers = search_arxiv(query, max_results=limit)
    except RuntimeError as e:
        print(f"[research] Search failed: {e}", file=sys.stderr)
        return []

    if not papers:
        print(f"[research] No papers found for query: {query}")
        return []

    if verbose:
        print(f"[research] Found {len(papers)} papers")

    # Filter skipped papers before scheduling any work
    work_items: List[Paper] = []
    output_paths: List[Path] = []
    for paper in papers:
        slug = slugify_title(paper.title)[:60]
        note_path = output_dir / f"{safe_uid(paper.uid)}_{slug}.md"
        if skip_existing and note_path.exists():
            output_paths.append(note_path)
        else:
            work_items.append(paper)

    if not work_items:
        return output_paths

    sem = asyncio.Semaphore(3)
    loop = asyncio.get_running_loop()

    async def _process_one(
        paper: Paper,
        i: int,
        total: int,
    ) -> Tuple[Optional[Path], Optional[Path], Optional[str]]:
        async with sem:
            title_short = paper.title[:60] + ("..." if len(paper.title) > 60 else "")
            if verbose:
                print(f"[research] [{i}/{total}] {paper.uid}: {title_short}")

            slug = slugify_title(paper.title)[:60]
            note_path = output_dir / f"{safe_uid(paper.uid)}_{slug}.md"
            pdf_path: Optional[Path] = None
            extracted_text = ""
            err: Optional[str] = None

            if download_pdfs and paper.pdf_url:
                for attempt in range(2):
                    try:
                        pdf_path = Path(_tempfile.gettempdir()) / f"{safe_uid(paper.uid)}.pdf"
                        await download_pdf_async(paper.pdf_url, pdf_path, timeout=60)
                        if verbose:
                            print(f"  [pdf] Downloaded: {pdf_path.name} ({pdf_path.stat().st_size / 1024:.0f} KB)")
                        # CPU-bound: run in executor to avoid blocking the event loop
                        extracted_text = await loop.run_in_executor(
                            None, lambda: extract_pdf_text(pdf_path, max_pages=15)
                        )
                        if len(extracted_text) > max_text_len:
                            extracted_text = extracted_text[:max_text_len] + "\n\n[... truncated ...]"
                        if verbose and extracted_text:
                            print(f"  [text] Extracted {len(extracted_text)} chars")
                        break
                    except Exception as e:
                        extracted_text = ""
                        if attempt == 0:
                            await asyncio.sleep(1)
                            continue
                        err = "pdf_failed"
                        warnings.warn(f"PDF download/extract failed for {paper.uid} after retry: {e}", stacklevel=2)

            note_tags = list(tags) if tags else []
            draft = ""
            rubric: dict = {}
            cost_info: dict = {}

            if api_key and extracted_text:
                try:
                    if verbose:
                        print("  [llm] Generating draft (streaming)...")
                    input_for_llm = f"Title: {paper.title}\nAbstract: {paper.abstract or ''}\n\nExtracted text:\n{extracted_text[:8000]}"

                    # CPU-bound LLM call: run in executor for true streaming
                    async def _generate() -> str:
                        return cast(str, await call_llm_chat_completions_async(
                            messages=[],
                            model=model,
                            user_prompt=None,
                            base_url=base_url,
                            api_key=api_key,
                            system_prompt=None,
                            stream=True,
                            progress_callback=progress_callback,
                        ))

                    draft = await _generate()
                    sections, rubric, _ = parse_ai_pnote_draft(draft)
                    cost_info = estimate_cost(model, input_for_llm, draft)
                    if verbose:
                        print(
                            f"  [llm] Draft generated ({len(draft)} chars, "
                            f"≈{cost_info.get('total_tokens', 0)} tokens, "
                            f"${cost_info.get('total_cost_usd', 0):.4f})"
                        )
                except Exception as e:
                    warnings.warn(f"LLM draft generation failed for {paper.uid}: {e}", stacklevel=2)
                    draft = ""
                    err = err or "llm_failed"
            elif not api_key:
                if verbose:
                    print("  [skip] No API key — metadata-only note")
            else:
                if verbose:
                    print("  [skip] No extracted text — metadata-only note")

            note_content = _build_research_note(paper, draft, rubric, note_tags)
            note_path.write_text(note_content, encoding="utf-8")

            if root:
                try:
                    _integrate_into_tree(root, note_path, paper, note_tags)  # type: ignore[name-defined]
                except Exception as e:
                    warnings.warn(f"Failed to integrate into research tree: {e}", stacklevel=2)

            if verbose:
                rubric_scores = extract_rubric_scores(rubric)
                print(
                    f"  [saved] {note_path.name}"
                    + (f" [novelty={rubric_scores.get('novelty', '?')}]" if rubric_scores else "")
                )

            return note_path, pdf_path, err

    # Schedule all paper-processing tasks concurrently
    tasks = [
        _process_one(paper, i, len(work_items))
        for i, paper in enumerate(work_items, 1)
    ]
    results = await asyncio.gather(*tasks, return_exceptions=False)

    processed = 0
    failed = 0
    error_reasons: dict[str, int] = {}
    skipped = len(papers) - len(work_items)

    for note_path, _, err in results:  # type: ignore[assignment]
        if note_path:
            output_paths.append(note_path)
            if err:
                failed += 1
                error_reasons[err] = error_reasons.get(err, 0) + 1
            else:
                processed += 1

    total = len(papers)
    print(f"\n[research] Done: {processed}/{total} processed, {failed} failed, {skipped} skipped")
    for reason, count in error_reasons.items():
        print(f"  [{reason}] {count} paper(s)")

    if root:
        flush_radar(root)

    return output_paths


# ─── Metrics ───────────────────────────────────────────────────────────────────


class Metrics:
    """Tracks research loop execution statistics."""

    def __init__(self):
        self.papers_processed = 0
        self.papers_failed = 0
        self.papers_skipped = 0
        self.llm_calls = 0
        self.llm_cost_usd = 0.0

    def snapshot(self) -> dict:
        return {
            "papers_processed": self.papers_processed,
            "papers_failed": self.papers_failed,
            "papers_skipped": self.papers_skipped,
            "llm_calls": self.llm_calls,
            "llm_cost_usd": self.llm_cost_usd,
        }
