"""CLI command: demo — import seed data for a quick-start demo.

Usage:
    airos demo
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from cli._shared import Colors, colored, print_error, print_success

DEMO_DIR = Path(__file__).parent.parent.parent / "data" / "demo"


def _build_demo_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "demo",
        help="Import seed papers + Gene Pool capsules for a quick-start demo",
        description="Load sample papers and pre-computed Gene Pool capsules "
        "so you can explore Rairos features immediately.",
    )
    p.add_argument(
        "--papers",
        type=str,
        default=str(DEMO_DIR / "papers.json"),
        help="Path to seed papers JSON (default: data/demo/papers.json)",
    )
    p.add_argument(
        "--capsules",
        type=str,
        default=str(DEMO_DIR / "capsules.json"),
        help="Path to seed Gene Pool capsules JSON (default: data/demo/capsules.json)",
    )
    p.set_defaults(func=_run_demo)
    return p  # type: ignore[no-any-return]


def _run_demo(args) -> None:
    papers_path = Path(getattr(args, "papers", DEMO_DIR / "papers.json"))
    capsules_path = Path(getattr(args, "capsules", DEMO_DIR / "capsules.json"))

    print()
    print(f"  {Colors.CYAN}╔══════════════════════════════════════╗{Colors.END}")
    print(f"  {Colors.CYAN}║     Rairos Demo Setup                 ║{Colors.END}")
    print(f"  {Colors.CYAN}╚══════════════════════════════════════╝{Colors.END}")
    print()

    # ── Import Papers ────────────────────────────────────────────────
    from db.database import Database

    db = Database()
    db.init()

    if not papers_path.exists():
        print_error(f"Papers file not found: {papers_path}")
        sys.exit(1)

    with open(papers_path, encoding="utf-8") as f:
        data = json.load(f)

    imported = 0
    for paper in data.get("papers", []):
        try:
            db.upsert_paper(
                paper_id=paper["id"],
                title=paper["title"],
                authors=paper.get("authors", []),
                abstract=paper.get("abstract", ""),
                source=paper.get("source", "arxiv"),
                primary_category=paper.get("primary_category", ""),
                published=paper.get("published", ""),
            )
            imported += 1
            print(f"  {Colors.GREEN}✓{Colors.END} Imported: {paper['title'][:55]}")
        except Exception as e:
            print(f"  {Colors.YELLOW}⚠{Colors.END} Skipped {paper['id']}: {e}")

    print(f"  {Colors.GREEN}→ {imported} papers imported{Colors.END}")
    print()

    # ── Import Gene Pool Capsules ───────────────────────────────────
    from llm.insight.tracker import EvolutionTracker

    tracker = EvolutionTracker()

    if not capsules_path.exists():
        print_error(f"Capsules file not found: {capsules_path}")
    else:
        with open(capsules_path, encoding="utf-8") as f:
            cap_data = json.load(f)

        caps_imported = 0
        for c in cap_data.get("capsules", []):
            try:
                from llm.insight.gene import CapsuleGene

                capsule = CapsuleGene.from_dict(c)
                tracker._save_capsules(tracker._load_capsules() + [capsule])
                caps_imported += 1
                print(
                    f"  {Colors.GREEN}✓{Colors.END} Capsule: {c.get('action_gap_title', '')[:55]}"
                )
            except Exception as e:
                print(f"  {Colors.YELLOW}⚠{Colors.END} Skipped capsule: {e}")

        print(f"  {Colors.GREEN}→ {caps_imported} Gene Pool capsules loaded{Colors.END}")
        print()

    print()
    print(f"  {Colors.GREEN}╔══════════════════════════════════════╗{Colors.END}")
    print(f"  {Colors.GREEN}║  Demo ready!                         ║{Colors.END}")
    print(f"  {Colors.GREEN}╚══════════════════════════════════════╝{Colors.END}")
    print()
    print("  Try these commands:")
    print(f"    {Colors.CYAN}rairos gap list{Colors.END}        — View Gene Pool capsules")
    print(f"    {Colors.CYAN}rairos daemon status{Colors.END}   — Check daemon")
    print(f"    {Colors.CYAN}rairos search{Colors.END}           — Search papers")
    print(f"    {Colors.CYAN}uvicorn web.app:app --port 8501{Colors.END}  — Open Web UI")
    print()
