"""CLI command: status."""
from __future__ import annotations

import argparse

from cli._shared import get_db


def _build_status_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser("status", help="Show database status")
    return p


def _run_status(args: argparse.Namespace) -> int:
    db = get_db()
    db.init()
    papers = db.get_papers(limit=10000)
    print(f"Total papers: {len(papers)}")
    by_source: dict[str, int] = {}
    by_status: dict[str, int] = {}
    for p in papers:
        by_source[p.source or "?"] = by_source.get(p.source or "?", 0) + 1
        by_status[p.parse_status or "?"] = by_status.get(p.parse_status or "?", 0) + 1
    print("By source:", ", ".join(f"{k}={v}" for k, v in sorted(by_source.items())))
    print("By status:", ", ".join(f"{k}={v}" for k, v in sorted(by_status.items())))
    return 0
