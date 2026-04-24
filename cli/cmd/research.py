"""CLI command: research."""
from __future__ import annotations

import argparse
import asyncio
import sys
from pathlib import Path

from core.i18n import set_lang
from research_loop import arun_research, run_research

from cli._shared import (
    Colors,
    colored,
    print_error,
    print_success,
)


def _build_research_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "research",
        help="Autonomous research loop: search arXiv → download → extract → AI summarize",
    )

    p.add_argument("query", nargs="?", default="", help="Research topic or keyword")

    p.add_argument("--limit", type=int, default=5, help="Max papers to process (default 5)")

    p.add_argument(
        "--output",
        type=str,
        default="",
        help="Output directory (default: ~/ai_research/<query-slug>/)",
    )

    p.add_argument(
        "--no-ai",
        dest="no_ai",
        action="store_true",
        default=False,
        help="Skip AI draft generation (metadata only)",
    )

    p.add_argument(
        "--no-pdf",
        dest="no_pdf",
        action="store_true",
        default=False,
        help="Skip PDF download (use abstract only)",
    )

    p.add_argument("--tag", dest="tags", action="append", default=[], help="Tags to assign (repeatable)")

    p.add_argument(
        "--model",
        type=str,
        default="gpt-4o-mini",
        help="LLM model for AI drafts (default gpt-4o-mini)",
    )

    p.add_argument(
        "--base-url",
        type=str,
        default="",
        help="LLM API base URL (default: OPENAI_BASE_URL env var)",
    )

    p.add_argument(
        "--api-key",
        type=str,
        default="",
        help="LLM API key (default: OPENAI_API_KEY env var)",
    )

    p.add_argument(
        "--no-skip",
        dest="no_skip",
        action="store_true",
        default=False,
        help="Re-generate even if note already exists",
    )

    p.add_argument("-v", "--verbose", action="store_true", default=False, help="Verbose output")

    p.add_argument("--lang", type=str, default="zh", choices=["en", "zh", "e", "z"], help="Output language (default: zh)")

    p.add_argument(
        "--sync",
        dest="sync_mode",
        action="store_true",
        default=False,
        help="Use synchronous I/O (async is the default)",
    )

    p.add_argument(
        "--progress",
        dest="progress",
        action="store_true",
        default=False,
        help="Print LLM streaming tokens in real time (implies --async)",
    )

    return p


def _run_research_cmd(args: argparse.Namespace) -> int:
    set_lang(args.lang)

    # --progress implies --async (progress only works with async mode)
    if args.progress:
        args.sync_mode = False

    output_dir = Path(args.output) if args.output else None
    tags = args.tags if args.tags else []
    skip_existing = not args.no_skip

    # progress_callback prints each streaming chunk to stdout (only works with async)
    progress_cb = sys.stdout.write if args.progress and not args.sync_mode else None

    if args.sync_mode:
        paths = run_research(
            query=args.query,
            limit=args.limit,
            output_dir=output_dir,
            download_pdfs=not args.no_pdf,
            skip_existing=skip_existing,
            tags=tags,
            model=args.model,
            base_url=args.base_url or None,
            api_key=(args.api_key or None) if not args.no_ai else "",
            verbose=args.verbose,
        )
    else:
        paths = asyncio.run(
            arun_research(
                query=args.query,
                limit=args.limit,
                output_dir=output_dir,
                download_pdfs=not args.no_pdf,
                skip_existing=skip_existing,
                tags=tags,
                model=args.model,
                base_url=args.base_url or None,
                api_key=(args.api_key or None) if not args.no_ai else "",
                verbose=args.verbose,
                progress_callback=progress_cb,
            )
        )

    if not paths:
        print_error("No notes generated.")
        return 1

    print_success(f"\nGenerated {len(paths)} note(s):")
    for p in paths:
        print(f"  {colored(p, Colors.OKBLUE)}")
    return 0
