"""CLI command: ingest — Import + process in one shot.

Provides the full pipeline that airos import intentionally skips:
    import → postprocess → embed → kg-sync

Usage:
    airos ingest 2301.00001                 # single paper, full pipeline
    airos ingest 2301.00001 --skip-embed    # skip semantic embedding
    airos ingest 2301.00001 --skip-postprocess  # import + kg only
    airos ingest --file ids.txt --no-pdf     # batch, no PDF
"""
from __future__ import annotations

import argparse
import time
from pathlib import Path
from typing import List, Optional

from cli._shared import get_db, print_error, print_info, print_success, print_warning
from cli.warp import WarpBlocks


def _resolve_pdf_path(db, paper_id: str, root: Path) -> Optional[Path]:
    """Download PDF to cache if pdf_url is available."""
    rec = db.get_paper(paper_id)
    if not rec:
        return None
    pdf_url = getattr(rec, "pdf_url", "") or ""
    if not pdf_url:
        return None
    cache_dir = root / "cache"
    cache_dir.mkdir(exist_ok=True)
    cached = cache_dir / f"{paper_id}.pdf"
    if cached.exists():
        return cached
    try:
        from pdf.extract import download_pdf
        download_pdf(pdf_url, cached)
        return cached
    except Exception as e:
        print_warning(f"PDF download failed for {paper_id}: {e}")
        return None


def _extract_text(pdf_path: Path, max_pages: int = 30) -> str:
    """Extract text from PDF with graceful fallback."""
    try:
        from pdf.extract import extract_pdf_text_hybrid
        return extract_pdf_text_hybrid(pdf_path, max_pages=max_pages)
    except Exception as e:
        print_warning(f"PDF extraction failed: {e}")
        return ""


def _run_import_phase(paper_ids: list[str], db, source: str) -> tuple[list[str], list[str]]:
    """Import papers by fetching metadata. Returns (added_ids, failed_ids)."""
    from cli.cmd.import_ import _fetch_paper_metadata

    added, failed = [], []
    for pid in paper_ids:
        pid = pid.strip()
        if not pid:
            continue
        try:
            metadata = _fetch_paper_metadata(pid)
            if metadata:
                db.upsert_paper(
                    paper_id=pid,
                    source=source,
                    title=metadata.get("title", ""),
                    authors=metadata.get("authors", []),
                    abstract=metadata.get("abstract", ""),
                    published=metadata.get("published", ""),
                    abs_url=metadata.get("abs_url", ""),
                    pdf_url=metadata.get("pdf_url", ""),
                    primary_category=metadata.get("primary_category", ""),
                    doi=metadata.get("doi", ""),
                )
                added.append(pid)
                print_info(f"  Imported: {pid}")
            else:
                failed.append(pid)
                print_error(f"  Failed to fetch: {pid}")
        except Exception as e:
            failed.append(pid)
            print_error(f"  Error importing {pid}: {e}")
    return added, failed


def _run_embed_phase(paper_ids: list[str], db, delay: float = 0.0) -> tuple[int, int]:
    """Generate semantic embeddings via Ollama. Returns (generated, failed)."""
    from cli.cmd.dedup_semantic import _generate_missing_embeddings
    return _generate_missing_embeddings(db, delay=delay)


def _run_postprocess_phase(
    paper_id: str,
    root: Path,
    db,
    stages: Optional[list[str]] = None,
    skip_llm: bool = False,
) -> bool:
    """Run ResearchDeepDivePipeline on a paper. Returns True on success."""
    from llm.postprocess import ResearchDeepDivePipeline, PostStage, make_llm_config

    rec = db.get_paper(paper_id)
    if not rec:
        print_warning(f"  Paper {paper_id} not found in DB for postprocess")
        return False

    # Resolve tags
    raw_tags = getattr(rec, "tags", "") or ""
    tags = [t.strip() for t in raw_tags.split(",") if t.strip()] if raw_tags else []
    # Fallback: try to infer from primary_category
    if not tags:
        cat = getattr(rec, "primary_category", "") or ""
        if cat:
            tags = [cat]

    # Download + extract PDF
    pdf_path = _resolve_pdf_path(db, paper_id, root)
    extracted_text = _extract_text(pdf_path) if pdf_path else ""

    # Build paper object for the pipeline
    from core import Paper
    paper = Paper(
        source=getattr(rec, "source", "") or "arxiv",
        uid=paper_id,
        title=getattr(rec, "title", "") or "",
        authors=getattr(rec, "authors", []) or [],
        abstract=getattr(rec, "abstract", "") or "",
        published=getattr(rec, "published", "") or "",
        updated=getattr(rec, "updated", "") or "",
        abs_url=getattr(rec, "abs_url", "") or f"https://arxiv.org/abs/{paper_id}",
        pdf_url=getattr(rec, "pdf_url", "") or "",
    )
    # P-note path
    pnote_dir = root / (getattr(rec, "category", "") or "02-Models")
    pnote_dir.mkdir(parents=True, exist_ok=True)
    from core.basics import slugify_title
    year = (getattr(rec, "published", "") or "")[:4] or time.strftime("%Y")
    pnote_path = pnote_dir / f"P - {year} - {slugify_title(paper.title)}.md"
    pnote_path.parent.mkdir(parents=True, exist_ok=True)

    try:
        pipeline = ResearchDeepDivePipeline(db=db, data_dir=root)
        stage_vals = None
        if stages:
            stage_vals = [PostStage(s) for s in stages]
        pl_config = make_llm_config()
        if skip_llm:
            if pl_config is not None:
                pl_config["api_key"] = ""

        result = pipeline.run(
            paper_id=paper_id,
            extracted_text=extracted_text,
            paper=paper,
            tags=tags,
            pnote_path=pnote_path,
            llm_config=pl_config,
            stages=stage_vals,
        )
        if result.stages_completed:
            print_success(f"  Postprocess OK: {', '.join(result.stages_completed)}")
        if result.stages_failed:
            print_warning(f"  Postprocess issues: {', '.join(result.stages_failed)}")
        return bool(result.stages_completed)
    except Exception as e:
        print_error(f"  Postprocess error for {paper_id}: {e}")
        return False


def _run_kg_sync_phase(db) -> bool:
    """Rebuild KG from all papers. Returns True on success."""
    try:
        from kg.integration import KGIntegration
        from kg import KGManager
        kg = KGManager()
        integ = KGIntegration(kg)
        integ.rebuild_from_papers_json("data/papers.json", incremental=True)
        print_success("  KG synced")
        return True
    except Exception as e:
        print_warning(f"  KG sync failed: {e}")
        return False


def _build_ingest_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "ingest",
        help="Import + process: metadata → postprocess → embed → KG sync",
        prog="airos ingest",
        description=(
            "Full import pipeline: fetch metadata, run deep analysis, "
            "generate embeddings, and sync the knowledge graph — in one command."
        ),
        epilog="""\
Examples:
  %(prog)s 2301.00001                         # single paper, all phases
  %(prog)s --file ids.txt                     # batch from file
  %(prog)s 2301.00001 --skip-embed            # skip Ollama embedding
  %(prog)s 2301.00001 --skip-postprocess      # import + KG only
  %(prog)s 2301.00001 --root AI-Research --tags LLM,RAG""",
    )
    p.add_argument("ids", nargs="*", metavar="ID", help="arXiv IDs, DOIs, or paper UIDs")
    p.add_argument("--file", metavar="FILE", help="Read IDs from file (one per line)")
    p.add_argument("--root", default="AI-Research", help="Root folder (default: AI-Research)")
    p.add_argument("--tags", default="", help="Comma-separated tags for all papers")
    p.add_argument("--source", default="ingest", help="Source label (default: ingest)")

    g = p.add_mutually_exclusive_group()
    g.add_argument("--skip-postprocess", action="store_true", help="Skip deep analysis / postprocess")
    g.add_argument("--only-postprocess", action="store_true", help="Skip import, only run postprocess on existing papers")

    p.add_argument("--skip-embed", action="store_true", help="Skip semantic embedding generation")
    p.add_argument("--skip-kg", action="store_true", help="Skip KG sync")
    p.add_argument("--skip-pdf", action="store_true", help="Skip PDF download in postprocess")
    p.add_argument("--stages", nargs="+",
        choices=["paper_analysis", "benchmark", "cross_reference", "insight", "kg_sync", "pnote_update"],
        help="Specific postprocess stages to run (default: all)")
    p.add_argument("--skip-llm", action="store_true", help="Skip LLM calls in postprocess (keyword-only)")
    p.add_argument("--format", choices=["text", "warp"], default="warp", help="Output format")
    return p  # type: ignore[no-any-return]


def _run_ingest(args: argparse.Namespace) -> int:
    from rich.console import Console
    c = Console()

    root = Path(args.root)

    # Collect paper IDs
    paper_ids: list[str] = list(getattr(args, "ids", []) or [])
    if getattr(args, "file", None):
        fpath = Path(args.file)
        if fpath.exists():
            paper_ids += [l.strip() for l in fpath.read_text(encoding="utf-8").splitlines() if l.strip()]
        else:
            print_error(f"File not found: {args.file}")
            return 1  # type: ignore[no-any-return]

    if not paper_ids:
        print_error("No paper IDs provided (use positional args or --file)")
        return 1  # type: ignore[no-any-return]

    paper_ids = list(dict.fromkeys(paper_ids))  # dedup preserve order

    db = get_db()
    db.init()

    # ── Phase 1: Import ───────────────────────────────────────────────────────
    if args.only_postprocess:
        import_ids = [pid for pid in paper_ids if db.paper_exists(pid)]
        skipped_import = [pid for pid in paper_ids if not db.paper_exists(pid)]
        if skipped_import:
            print_warning(f"Skipped (not in DB): {', '.join(skipped_import)}")
        if not import_ids:
            print_error("None of the papers are in the database. Run without --only-postprocess first.")
            return 1
    else:
        print_info("Phase 1/4: Importing metadata...")
        added, failed = _run_import_phase(paper_ids, db, args.source)
        import_ids = added
        if failed:
            print_warning(f"Failed to import: {', '.join(failed)}")

    # ── Phase 2: Embeddings ─────────────────────────────────────────────────
    embed_ids = []
    if not args.skip_embed:
        print_info(f"Phase 2/4: Generating embeddings ({len(import_ids)} papers)...")
        gen, fail = _run_embed_phase(import_ids, db)
        embed_ids = [pid for pid in import_ids if db.paper_exists(pid)]
        print_success(f"  Embeddings: {gen} generated, {fail} failed")
    else:
        print_info("Phase 2/4: Skipped (--skip-embed)")

    # ── Phase 3: Postprocess ────────────────────────────────────────────────
    postprocess_ok: List[str] = []
    postprocess_fail: List[str] = []
    if not args.skip_postprocess:
        print_info(f"Phase 3/4: Deep analysis ({len(import_ids)} papers)...")
        for pid in import_ids:
            pid = pid.strip()
            if not pid:
                continue
            ok = _run_postprocess_phase(pid, root, db, stages=args.stages, skip_llm=args.skip_llm)
            (postprocess_ok if ok else postprocess_fail).append(pid)
    else:
        print_info("Phase 3/4: Skipped (--skip-postprocess)")

    # ── Phase 4: KG sync ───────────────────────────────────────────────────
    if not args.skip_kg:
        print_info("Phase 4/4: Syncing knowledge graph...")
        _run_kg_sync_phase(db)
    else:
        print_info("Phase 4/4: Skipped (--skip-kg)")

    # ── Summary ─────────────────────────────────────────────────────────────
    c.rule("[bold #FF8272]  Ingest Complete  [/]")
    rows = [
        ["[#A5D5FE]Imported[/#A5D5FE]",     f"[#B4FA72]{len(import_ids)}[/]"],
        ["[#A5D5FE]Postprocess OK[/#A5D5FE]", f"[#B4FA72]{len(postprocess_ok)}[/]"],
        ["[#A5D5FE]Postprocess fail[/#A5D5FE]", f"[#FF8272]{len(postprocess_fail)}[/]"],
        ["[#A5D5FE]Embeddings[/#A5D5FE]",    f"[#B4FA72]{len(embed_ids)}[/]"],
    ]
    c.print(WarpBlocks.table(["Phase", "Count"], rows, title="Pipeline Summary"))
    return 0
