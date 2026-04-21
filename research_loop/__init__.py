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
import datetime as dt
import os
import sys
import time  # noqa: F401  # tests mock research_loop.time.sleep
import warnings
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from typing import List, Optional, Tuple

from core import Paper
from core.basics import ensure_research_tree, safe_uid, slugify_title
from llm.generate import ai_generate_pnote_draft
from llm.parse import parse_ai_pnote_draft, extract_rubric_scores
from notes.cnote import ensure_cnote, update_cnote_links
from parsers.arxiv_search import search_arxiv
from pdf.extract import download_pdf as _download_pdf, extract_pdf_text
from updaters.radar import update_radar
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
                    pdf_path = Path(f"/tmp/{safe_uid(paper.uid)}.pdf")
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
        rubric = {}

        if a_key and extracted_text:
            try:
                if verbose:
                    print("  [llm] Generating draft...")
                draft = ai_generate_pnote_draft(
                    paper=paper,
                    tags=note_tags,
                    extracted_text=extracted_text,
                    base_url=b_url,
                    api_key=a_key,
                    model=mod,
                )
                sections, rubric, _ = parse_ai_pnote_draft(draft)
                if verbose:
                    print(f"  [llm] Draft generated ({len(draft)} chars)")
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
        ensure_research_tree(root)

        # Timeline: append under the paper's year
        year = paper.published[:4] if paper.published else dt.date.today().isoformat()[:4]
        update_timeline(root, year, note_path, paper.title)

        # For each tag: create/verify C-note and link it
        for tag in note_tags:
            concept_dir = root / "01-Foundations"
            cpath = ensure_cnote(concept_dir, tag)
            update_cnote_links(cpath, note_path)

        # Radar: bump the topic rows
        if note_tags:
            update_radar(root, note_tags, year)

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
                note_path, _, err = future.result()
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
