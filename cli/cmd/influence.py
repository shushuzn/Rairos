"""CLI command: influence — Rank papers by citation velocity.

Citation velocity = forward_citations / age_years
A 2024 paper with 10 citations (velocity=10) is "hotter" than
a 2020 paper with 40 citations (velocity=10).
"""
from __future__ import annotations

import argparse
import sys
from dataclasses import dataclass

from cli._shared import get_db
from cli._shared import Colors, colored, print_success, print_error, print_info


@dataclass
class PaperInfluence:
    paper_id: str
    title: str
    year: int
    forward_cites: int
    age_years: float
    velocity: float  # forward_cites / age_years


def _build_influence_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "influence",
        help="Rank papers by citation velocity (citations per year)",
        description="Compute citation velocity = forward_citations / age_years, "
                   "sorting papers by research impact normalized for paper age.",
    )
    p.add_argument(
        "--top", "-n",
        type=int,
        default=20,
        metavar="N",
        help="Number of top papers to show (default: 20)",
    )
    p.add_argument(
        "--paper",
        metavar="PAPER_ID",
        help="Show detailed influence stats for a specific paper",
    )
    p.add_argument(
        "--min-cites",
        type=int,
        default=1,
        metavar="N",
        help="Minimum forward citations to include (default: 1)",
    )
    p.add_argument(
        "--format",
        choices=["text", "csv", "json"],
        default="text",
        help="Output format (default: text)",
    )
    return p


def _compute_influence(db, min_cites: int = 1) -> list[PaperInfluence]:
    """Compute citation velocity for all papers that have forward citations."""
    current_year = 2026  # hardcoded; could use datetime.now().year

    cur = db.conn.execute("""
        SELECT
            p.id,
            p.title,
            p.published,
            COUNT(c.id) AS forward_cites
        FROM papers p
        LEFT JOIN citations c ON c.target_id = p.id
        GROUP BY p.id
        HAVING forward_cites >= ?
    """, (min_cites,))

    results = []
    for row in cur.fetchall():
        paper_id = row[0]
        title = row[1] or ""
        published = row[2] or ""
        forward_cites = row[3]

        try:
            year = int(published[:4])
        except (ValueError, TypeError):
            continue

        if year < 2000 or year > current_year:
            continue

        age_years = current_year - year + 1
        velocity = forward_cites / age_years

        results.append(PaperInfluence(
            paper_id=paper_id,
            title=title,
            year=year,
            forward_cites=forward_cites,
            age_years=age_years,
            velocity=velocity,
        ))

    results.sort(key=lambda x: x.velocity, reverse=True)
    return results


def _run_influence(args: argparse.Namespace) -> int:
    db = get_db()
    db.init()

    # Single paper detail view
    if args.paper:
        return _show_paper_influence(db, args.paper)

    # Top-N ranking
    all_influence = _compute_influence(db, min_cites=args.min_cites)

    if not all_influence:
        print("No papers with sufficient citation data found.")
        return 0

    top_n = all_influence[: args.top]

    if args.format == "csv":
        print("rank,paper_id,title,year,forward_cites,age_years,velocity")
        for i, p in enumerate(top_n, 1):
            title_esc = p.title.replace('"', '""')
            print(f'{i},"{p.paper_id}","{title_esc}",{p.year},{p.forward_cites},{p.age_years:.1f},{p.velocity:.2f}')
        return 0

    if args.format == "json":
        import json
        data = [
            {
                "rank": i + 1,
                "paper_id": p.paper_id,
                "title": p.title,
                "year": p.year,
                "forward_cites": p.forward_cites,
                "age_years": p.age_years,
                "velocity": round(p.velocity, 2),
            }
            for i, p in enumerate(top_n)
        ]
        print(json.dumps(data, ensure_ascii=False, indent=2))
        return 0

    # Text output
    header = f"{'Rank':>4}  {'Velocity':>8}  {'Cites':>5}  {'Age':>3}y  Year  Paper"
    print(colored(header, Colors.HEADER))
    print("-" * len(header))

    for i, p in enumerate(top_n, 1):
        rank_color = Colors.OKGREEN if i <= 5 else (Colors.WARNING if i <= 15 else "")
        vel_bar = "█" * min(int(p.velocity), 20)
        title_short = p.title[:50] + "…" if len(p.title) > 50 else p.title
        line = (f"{i:>4}  "
                 f"{colored(f'{p.velocity:>7.1f}/y', rank_color)}  "
                 f"{p.forward_cites:>5}  "
                 f"{p.age_years:>3.0f}   "
                 f"{p.year}  "
                 f"{title_short}")
        print(line)

    print()
    print(f"Showing {len(top_n)} of {len(all_influence)} papers with >= {args.min_cites} citation(s)")
    print("Formula: velocity = forward_citations / age_years  (age = 2026 - published + 1)")
    return 0


def _show_paper_influence(db, paper_id: str) -> int:
    """Show detailed influence stats for a single paper."""
    title = db.get_paper_title(paper_id)
    counts = db.get_citation_count(paper_id)
    forward = counts["forward"]
    backward = counts["backward"]

    if not title:
        print(f"Error: paper {paper_id!r} not found in database", file=sys.stderr)
        return 1

    # Get published year
    cur = db.conn.execute("SELECT published FROM papers WHERE id = ?", (paper_id,))
    row = cur.fetchone()
    published = row[0] if row else ""
    try:
        year = int(published[:4])
    except (ValueError, TypeError):
        year = 0

    current_year = 2026
    if 2000 < year <= current_year:
        age = current_year - year + 1
        velocity = forward / age
    else:
        age = 0
        velocity = 0.0

    print(colored("=== Paper Influence Profile ===", Colors.HEADER))
    print(f"  Paper ID  : {paper_id}")
    print(f"  Title    : {title}")
    print(f"  Published: {year if year else '(unknown)'}")
    if age > 0:
        print(f"  Age      : {age} years (as of 2026)")
    print()
    print(colored("  Citations", Colors.HEADER))
    print(f"    Cited by (forward) : {forward}  → velocity = {forward}/{age} = {velocity:.2f}/y" if age > 0 else f"    Cited by (forward) : {forward}")
    print(f"    References (backward): {backward}")
    print()
    if age > 0:
        print(colored("  Impact Assessment", Colors.HEADER))
        if velocity >= 10:
            print(colored("    🔥 Extremely high velocity (≥10/y) — field-defining", Colors.SUCCESS))
        elif velocity >= 5:
            print(colored("    📈 High velocity (5-10/y) — very active research", Colors.SUCCESS))
        elif velocity >= 1:
            print(colored("    📊 Moderate velocity (1-5/y) — steady influence", Colors.WARNING))
        else:
            print("    📉 Low velocity — emerging or niche")

    return 0
