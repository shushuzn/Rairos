"""CLI command: dedup-semantic."""

from __future__ import annotations

import argparse
import json
import sys
import time
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from typing import TYPE_CHECKING, List, Optional, Tuple

if TYPE_CHECKING:
    from db.database import Database

from cli._shared import get_db
from cli.warp import WarpBlocks


def _build_dedup_semantic_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "dedup-semantic",
        help="Find near-duplicate papers using semantic embeddings",
    )
    p.add_argument(
        "--threshold",
        type=float,
        default=0.85,
        help="Minimum cosine similarity threshold (default: 0.85)",
    )
    p.add_argument(
        "--limit",
        type=int,
        default=20,
        help="Maximum similar papers to find per query (default: 20)",
    )
    p.add_argument(
        "--paper",
        metavar="PAPER_ID",
        help="Check similarity for a specific paper only",
    )
    p.add_argument(
        "--generate",
        action="store_true",
        help="Generate embeddings for papers that don't have them yet",
    )
    p.add_argument(
        "--dry-run",
        action="store_true",
        help="Show what would happen without making changes",
    )
    p.add_argument(
        "--stats",
        action="store_true",
        help="Show embedding coverage statistics",
    )
    p.add_argument(
        "--format",
        choices=["warp", "csv"],
        default="warp",
        help="Output format: 'warp' (WarpBlocks-styled, default) or 'csv'",
    )
    return p  # type: ignore[no-any-return]


def _get_ollama_embedding(text: str, model: str = "nomic-embed-text") -> Optional[List[float]]:
    """Fetch embedding from local Ollama. Returns None on failure."""
    try:
        req = urllib.request.Request(
            "http://localhost:11434/api/embeddings",
            data=json.dumps({"model": model, "prompt": text}).encode(),
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=60) as resp:
            data = json.loads(resp.read())
            return data.get("embedding")  # type: ignore[no-any-return]
    except Exception as e:
        print(f"  [WARN] Ollama embedding failed: {e}", file=sys.stderr)
        return None


def _get_ollama_embedding_batch(
    texts: List[str],
    model: str = "nomic-embed-text",
    batch_size: int = 32,
) -> List[Optional[List[float]]]:
    """Fetch embeddings for multiple texts in one Ollama API call.

    Falls back to individual /api/embeddings calls if the batch endpoint
    returns a non-JSON response (e.g. 502 Bad Gateway from an older model
    that does not support multi-prompt batching).
    Returns list parallel to input; None for failed items.
    """
    results: List[Optional[List[float]]] = [None] * len(texts)
    for i in range(0, len(texts), batch_size):
        batch = texts[i : i + batch_size]
        try:
            req = urllib.request.Request(
                "http://localhost:11434/api/embed",
                data=json.dumps({"model": model, "prompt": batch}).encode(),
                headers={"Content-Type": "application/json"},
                method="POST",
            )
            with urllib.request.urlopen(req, timeout=120) as resp:
                raw = resp.read().decode()
                try:
                    data = json.loads(raw)
                except json.JSONDecodeError:
                    # Batch endpoint not supported (old model or 502) — fall back to single
                    for j, text in enumerate(batch):
                        single = _get_ollama_embedding(text, model)
                        results[i + j] = single
                    continue
                embeddings = data.get("embeddings") or []
                for j, emb in enumerate(embeddings):
                    results[i + j] = emb
        except Exception:
            # Network or server error — fall back to single calls
            for j, text in enumerate(batch):
                single = _get_ollama_embedding(text, model)
                results[i + j] = single
    return results


def _generate_missing_embeddings(
    db: "Database",
    delay: float = 0.0,
    batch_size: int = 32,
    max_workers: int = 8,
) -> Tuple[int, int]:
    """Generate embeddings for papers missing them using batch Ollama API.

    Returns (generated, failed).
    """
    papers = db.get_papers_without_embeddings(limit=1000)
    if not papers:
        return 0, 0

    # Prepare texts
    paper_ids = []
    texts = []
    for paper in papers:
        text = (paper.title or "") + ("\n\n" + paper.abstract if paper.abstract else "")
        if text.strip():
            paper_ids.append(paper.id)
            texts.append(text)

    if not texts:
        return 0, 0

    # Batch Ollama calls with thread pool for I/O parallelism
    generated, failed = 0, 0
    lock = __import__("threading").Lock()

    def store_batch(batch_texts: List[str], batch_ids: List[str]):
        nonlocal generated, failed
        embeddings = _get_ollama_embedding_batch(batch_texts, batch_size=batch_size)
        local_gen, local_fail = 0, 0
        for pid, emb in zip(batch_ids, embeddings):
            if emb is not None:
                try:
                    db.set_embedding(pid, emb)
                    local_gen += 1
                except Exception:
                    local_fail += 1
            else:
                local_fail += 1
        with lock:
            generated += local_gen
            failed += local_fail

    # Split into batches for Ollama, process batches in parallel
    for i in range(0, len(texts), batch_size * max_workers):
        chunk_ids = paper_ids[i : i + batch_size * max_workers]
        chunk_texts = texts[i : i + batch_size * max_workers]
        batches = [
            (chunk_texts[j : j + batch_size], chunk_ids[j : j + batch_size])
            for j in range(0, len(chunk_texts), batch_size)
        ]
        with ThreadPoolExecutor(max_workers=min(len(batches), max_workers)) as ex:
            for batch_texts, batch_ids in batches:
                ex.submit(store_batch, batch_texts, batch_ids)
        if delay > 0:
            time.sleep(delay)

    return generated, failed


def _run_dedup_semantic(args: argparse.Namespace) -> int:
    from rich.console import Console

    c = Console()

    db = get_db()
    db.init()

    if args.stats:
        s = db.get_embedding_stats()
        total = s["total_with_text"]
        with_emb = s["with_embedding"]
        pct = (with_emb / total * 100) if total > 0 else 0
        c.print(
            WarpBlocks.panel(
                "Embedding Coverage",
                f"[#A5D5FE]Papers with embedding:[/]  [#B4FA72]{with_emb}[/]\n"
                f"[#A5D5FE]Papers with text:[/]      [#FF8272]{total}[/]\n"
                f"[#A5D5FE]Coverage:[/]              [#FEFDC2]{pct:.1f}%[/]",
            )
        )
        return 0

    if args.generate:
        c.print("[#A5D5FE]Generating missing embeddings...[/]")
        gen, fail = _generate_missing_embeddings(db)
        c.print(
            WarpBlocks.panel(
                "Embedding Generation", f"[#B4FA72]Generated: {gen}[/]  [#FF5555]Failed: {fail}[/]"
            )
        )
        return 0

    if args.paper:
        if not db.paper_exists(args.paper):
            print(f"Paper '{args.paper}' not found")
            return 1
        paper = db.get_paper(args.paper)
        sims = db.find_similar(args.paper, threshold=args.threshold, limit=args.limit)
        if not sims:
            c.print(
                WarpBlocks.panel(
                    f"Similar Papers — {args.paper}",
                    f"[#8E8E8E]No similar papers above threshold=[#A5D5FE]{args.threshold}[/][/]",
                )
            )
            return 0
        if args.format == "csv":
            print("paper_a,paper_b,similarity,title_a,title_b")
            for sim_paper, score in sims:
                t1 = paper.title.replace('"', '""')
                t2 = sim_paper.title.replace('"', '""')
                print(f'{args.paper},{sim_paper.id},{score:.4f},"{t1}","{t2}"')
        elif args.format == "text":
            for sim_paper, score in sims:
                print(f"  {score:.4f}  {sim_paper.id}  {sim_paper.title}")
        elif args.format == "warp":
            rows = []
            for sim_paper, score in sims:
                sim_short = (
                    (sim_paper.title[:55] + "...") if len(sim_paper.title) > 58 else sim_paper.title
                )
                score_color = (
                    "[#B4FA72]" if score >= 0.95 else "[#FEFDC2]" if score >= 0.85 else "[#FF8272]"
                )
                rows.append(
                    [
                        f"{score_color}{score:.4f}[/]",
                        f"[#D0D1FE]{sim_paper.id}[/]",
                        f"[#A5D5FE]{sim_short}[/]",
                    ]
                )
            c.print(
                WarpBlocks.panel(
                    f"Similar Papers — [#FF8272]{args.paper}[/] (threshold=[#FF8272]{args.threshold}[/])",
                    f"[#A5D5FE]{len(sims)} similar papers found[/]",
                )
            )
            if rows:
                c.print(
                    WarpBlocks.table(
                        ["Similarity", "Paper ID", "Title"],
                        rows,
                        title=f"Similar to {args.paper} ({len(rows)})",
                    )
                )
        return 0

    # Global: check all papers
    papers, _total = db.list_papers(limit=10000)
    found = 0
    seen: set = set()
    pair_rows = []
    text_pairs: list = []

    if args.format == "csv":
        print("paper_a,paper_b,similarity,title_a,title_b")

    for paper in papers:
        if paper.id in seen or not paper.title:
            continue
        sims = db.find_similar(paper.id, threshold=args.threshold, limit=5)
        for sim_paper, score in sims:
            pair_key = tuple(sorted([paper.id, sim_paper.id]))
            if pair_key in seen:
                continue
            seen.add(pair_key)
            if args.format == "csv":
                t1 = paper.title.replace('"', '""')
                t2 = sim_paper.title.replace('"', '""')
                print(f'{paper.id},{sim_paper.id},{score:.4f},"{t1}","{t2}"')
            else:
                a_short = (paper.title[:50] + "...") if len(paper.title) > 53 else paper.title
                b_short = (
                    (sim_paper.title[:50] + "...") if len(sim_paper.title) > 53 else sim_paper.title
                )
                score_color = (
                    "[#B4FA72]" if score >= 0.95 else "[#FEFDC2]" if score >= 0.85 else "[#FF8272]"
                )
                pair_rows.append(
                    [
                        f"{score_color}{score:.4f}[/]",
                        f"[#FF8272]{paper.id}[/]",
                        f"[#A5D5FE]{a_short}[/]",
                        f"[#D0D1FE]{sim_paper.id}[/]",
                        f"[#A5D5FE]{b_short}[/]",
                    ]
                )
                text_pairs.append((score, paper.id, a_short, sim_paper.id, b_short))
            found += 1

    if found == 0 and args.format == "text":
        print("No Duplicates Found")
    elif found > 0 and args.format == "text":
        print(f"Found {found} duplicate pair(s):")
        for score, pid_a, title_a, pid_b, title_b in text_pairs:
            print(f"  {score:.4f}  {pid_a}  {title_a}  ↔  {pid_b}  {title_b}")

    if args.format == "warp":
        if found == 0:
            c.print(
                WarpBlocks.panel(
                    "No Duplicates Found",
                    f"[#8E8E8E]No semantic duplicates above threshold=[#A5D5FE]{args.threshold}[/][/]",
                )
            )
        else:
            c.print(
                WarpBlocks.panel(
                    f"Semantic Duplicates — [#FF8272]{found}[/] pairs (threshold=[#FF8272]{args.threshold}[/])",
                    f"[#A5D5FE]{len(papers)} papers scanned[/]",
                )
            )
            if pair_rows:
                c.print(
                    WarpBlocks.table(
                        ["Score", "Paper A", "Title A", "Paper B", "Title B"],
                        pair_rows,
                        title=f"Duplicate Pairs ({len(pair_rows)})",
                    )
                )

    return 0
