"""CLI command: cite-stats."""
from __future__ import annotations

import argparse
import sys

from cli._shared import get_db
from cli._shared import (
    Colors, colored,
)
from cli.warp import WarpBlocks


def _build_cite_stats_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "cite-stats",
        help="Show citation statistics for papers in the database",
    )
    p.add_argument("--paper", metavar="PAPER_ID", dest="stats_paper", help="Show stats for a specific paper")
    p.add_argument("--top", type=int, metavar="N", help="Show top N most-cited papers")
    p.add_argument("--format", choices=["text", "csv", "warp"], default="text", help="Output format (default: text)")
    return p  # type: ignore[no-any-return]


def _run_cite_stats(args: argparse.Namespace) -> int:
    db = get_db()
    db.init()

    if args.stats_paper:
        paper_id = args.stats_paper
        title = db.get_paper_title(paper_id)
        if db.paper_exists(paper_id) is False:
            print(f"Error: paper {paper_id!r} not found in database", file=sys.stderr)
            return 1  # type: ignore[no-any-return]
        counts = db.get_citation_count(paper_id)
        print(f"Paper: {paper_id}")
        print(f"Title : {title or '(no title)'}")
        print(f"Cites  (backward): {counts['backward']} papers")
        print(f"CitedBy (forward) : {counts['forward']} papers")
        return 0  # type: ignore[no-any-return]

    cur = db.conn.cursor()
    cur.execute("SELECT COUNT(*) FROM citations")
    total_citations = cur.fetchone()[0]
    cur.execute("SELECT COUNT(DISTINCT source_id) FROM citations")
    citing_papers = cur.fetchone()[0]
    cur.execute("SELECT COUNT(DISTINCT target_id) FROM citations")
    cited_papers = cur.fetchone()[0]
    cur.execute("SELECT COUNT(*) FROM papers")
    total_papers = cur.fetchone()[0]
    cur.execute("SELECT COUNT(DISTINCT id) FROM papers WHERE id IN (SELECT source_id FROM citations UNION SELECT target_id FROM citations)")
    papers_with_any = cur.fetchone()[0]
    orphan_papers = total_papers - papers_with_any

    if args.format == "csv":
        print("metric,value")
        print(f"total_citations,{total_citations}")
        print(f"papers_with_any_citation,{papers_with_any}")
        print(f"orphan_papers,{orphan_papers}")
        print(f"citing_papers,{citing_papers}")
        print(f"cited_papers,{cited_papers}")
        return 0  # type: ignore[no-any-return]

    if args.top:
        cur.execute("""
            SELECT target_id, COUNT(*) as cnt
            FROM citations
            GROUP BY target_id
            ORDER BY cnt DESC
            LIMIT ?
        """, (args.top,))
        top_rows = cur.fetchall()
    else:
        top_rows = []

    if args.format == "warp":
        _run_cite_stats_warp(total_citations, papers_with_any, orphan_papers,
                              citing_papers, cited_papers, total_papers, top_rows, db)
        return 0

    # text format
    print("=== Citation Statistics ===")
    print(f"Total citation pairs   : {total_citations}")
    print(f"Papers with any citation data: {papers_with_any} / {total_papers}")
    print(f"Orphan papers (no citation data): {orphan_papers}")
    print(f"Papers that cite others: {citing_papers}")
    print(f"Papers cited by others: {cited_papers}")
    if top_rows:
        print(f"\nTop {args.top or len(top_rows)} most-cited papers:")
        for row in top_rows:
            title = db.get_paper_title(row[0]) or ""
            print(f"  [{row[1]:4d}] {row[0]}  {title[:60]}")
    return 0


def _run_cite_stats_warp(total_citations, papers_with_any, orphan_papers,
                          citing_papers, cited_papers, total_papers, top_rows, db) -> None:
    """Render citation stats using Warp-style blocks."""
    blocks = []

    summary_lines = [
        f"Citation pairs  : {colored(str(total_citations), Colors.BOLD)}",
        f"Papers w/ cites: {colored(str(papers_with_any), Colors.OKGREEN)} / {total_papers}",
        f"Orphan papers   : {colored(str(orphan_papers), Colors.WARNING)}",
        f"Cite others    : {citing_papers}",
        f"Cited by others: {cited_papers}",
    ]
    blocks.append(WarpBlocks.panel("Citation Statistics", "\n".join(summary_lines)))

    if top_rows:
        rows = []
        for row in top_rows:
            title = db.get_paper_title(row[0]) or ""
            rows.append([str(row[1]), row[0], title[:50]])
        blocks.append(WarpBlocks.table(["Cites", "Paper ID", "Title"], rows))

    print("\n\n".join(blocks))

