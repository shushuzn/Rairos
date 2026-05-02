"""CLI command: citation-chain — Build and visualize citation chains."""
from __future__ import annotations

import argparse

from cli._shared import get_db, print_info, print_error
from llm.citation_chain import CitationChainBuilder


def _build_citation_chain_parser(subparsers) -> argparse.ArgumentParser:
    """Build the citation-chain subcommand parser."""
    p = subparsers.add_parser(
        "citation-chain",
        help="Build citation chains",
        description="Build and visualize citation relationships.",
    )
    p.add_argument("paper_id", nargs="?", help="Starting paper ID")
    p.add_argument("--depth", "-d", type=int, default=2, help="Chain depth (default: 2)")
    p.add_argument("--graphviz", "-g", action="store_true", help="Output Graphviz DOT")
    p.add_argument("--mermaid", "-m", action="store_true", help="Output Mermaid flowchart")
    p.add_argument("--influencers", action="store_true", help="Show papers that influenced this")
    p.add_argument("--impact", action="store_true", help="Show papers influenced by this")
    p.add_argument("--path", help="Find path to another paper ID")
    return p


def _run_citation_chain(args: argparse.Namespace) -> int:
    """Run citation chain command."""
    db = get_db()
    db.init()

    builder = CitationChainBuilder(db=db)

    if args.influencers or args.impact:
        # Build single paper view
        if not args.paper_id:
            print_error("Usage: citation-chain <paper_id> --influencers|--impact")
            return 1

        paper = db.get_paper(args.paper_id) if hasattr(db, 'get_paper') else None
        if not paper:
            print_error(f"Paper [{args.paper_id}] not found")
            return 1

        builder.add_paper(
            paper_id=args.paper_id,
            title=getattr(paper, 'title', args.paper_id),
            year=getattr(paper, 'year', 0) or 0,
        )

        if args.influencers:
            print_info(f"Finding influences for: {args.paper_id}")
            influencers = builder.find_influencers(args.paper_id, depth=args.depth)
            print()
            if influencers:
                print(f"Found {len(influencers)} influencing papers:")
                for i, p in enumerate(influencers[:10], 1):
                    print(f"  {i}. [{p.paper_id[:8]}] {p.title[:50]}")
            else:
                print("No influencers found.")

        if args.impact:
            print_info(f"Finding impact for: {args.paper_id}")
            impact = builder.find_impact(args.paper_id, depth=args.depth)
            print()
            if impact:
                print(f"Found {len(impact)} influenced papers:")
                for i, p in enumerate(impact[:10], 1):
                    print(f"  {i}. [{p.paper_id[:8]}] {p.title[:50]}")
            else:
                print("No impact found.")

        return 0

    if args.paper_id:
        # Build chain
        print_info(f"Building citation chain for: {args.paper_id}")
        chain = builder.build_from_db(args.paper_id, depth=args.depth)

        if args.graphviz:
            print(builder.render_graphviz(chain))
        elif args.mermaid:
            print(builder.render_mermaid(chain))
        else:
            print(builder.render_text(chain))

        # Path finding
        if args.path:
            path = builder.find_path(args.paper_id, args.path)
            if path:
                print()
                print("📍 Path found:")
                for i, p in enumerate(path, 1):
                    print(f"  {i}. {p}")
            else:
                print()
                print("No path found.")

        return 0

    print_error("Usage: citation-chain <paper_id> [options]")
    return 1
