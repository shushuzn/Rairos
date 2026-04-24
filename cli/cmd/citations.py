"""CLI command: citations."""
from __future__ import annotations

import argparse
import sys
from typing import Literal

from cli._shared import get_db
from cli._shared import (
    Colors, colored, print_success, print_error, print_warning, print_info, print_header,
)


def _build_citations_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "citations",
        help="Show citation relationships for a paper",
    )
    p.add_argument(
        "--from",
        metavar="PAPER_ID",
        dest="citation_from",
        help="Show papers cited by PAPER_ID (backward citations)",
    )
    p.add_argument(
        "--to",
        metavar="PAPER_ID",
        dest="citation_to",
        help="Show papers that cite PAPER_ID (forward citations)",
    )
    p.add_argument(
        "--format",
        choices=["text", "csv"],
        default="text",
        help="Output format (default: text)",
    )
    return p


def _run_citations(args: argparse.Namespace) -> int:
    if not args.citation_from and not args.citation_to:
        print("Error: must specify --from or --to", file=sys.stderr)
        return 1

    db = get_db()
    db.init()

    # Bidirectional filter: --from A --to B shows papers connecting A and B
    if args.citation_from and args.citation_to:
        paper_from = args.citation_from
        paper_to = args.citation_to
        from_title = db.get_paper_title(paper_from)
        to_title = db.get_paper_title(paper_to)

        if not from_title:
            print(f"Error: paper {paper_from} not found in the database")
            return 1
        if not to_title:
            print(f"Error: paper {paper_to} not found in the database")
            return 1

        backward_from = db.get_citations(paper_from, "from")
        forward_to = db.get_citations(paper_to, "to")

        # Check for direct edge: A -> B
        direct = any(c.target_id == paper_to for c in backward_from)

        # Papers that A cites AND that also have forward citations to B
        forward_to_sources = {c.source_id for c in forward_to}
        via_papers = [c for c in backward_from if c.target_id in forward_to_sources]

        if args.format == "csv":
            print("from_id,from_title,to_id,to_title,type")
            if direct:
                print(f"{paper_from},{from_title},{paper_to},{to_title},direct")
            if via_papers:
                via_ids = list({c.target_id for c in via_papers})
                paper_map = db.get_papers_bulk(via_ids)
                title_map = {pid: (paper_map[pid].title or '') for pid in via_ids if pid in paper_map}
                for c in via_papers:
                    t = title_map.get(c.target_id, '')
                    print(f"{paper_from},{from_title},{c.target_id},{t},via")
        else:
            print(f"CITATION BRIDGE: {paper_from} <-> {paper_to}")
            print(f"  {paper_from}: {from_title}")
            print(f"  {paper_to}: {to_title}")
            print()
            if direct:
                print(f"  DIRECT: {paper_from} cites {paper_to}")
            if via_papers:
                via_ids = list({c.target_id for c in via_papers})
                paper_map = db.get_papers_bulk(via_ids)
                title_map = {pid: (paper_map[pid].title or '') for pid in via_ids if pid in paper_map}
                print(f"  INDIRECT ({len(via_papers)} connections):")
                for c in via_papers:
                    t = title_map.get(c.target_id, '?')
                    print(f"    {paper_from} -> {c.target_id} ({t}) -> {paper_to}")
            if not direct and not via_papers:
                print("  No citation path found between these papers.")
        return 0

    # Single-direction mode
    paper_id = args.citation_from or args.citation_to
    direction: Literal["from", "to", "both"] = "from" if args.citation_from else "to"

    citations = db.get_citations(paper_id, direction)
    source_title = db.get_paper_title(paper_id)

    if args.format == "csv":
        print("paper,count")
        print(f"{paper_id},{len(citations)}")
    else:
        print(f"{'Cited by' if direction == 'to' else 'References'} for {paper_id}: {source_title}")
        for c in citations:
            print(f"  {c.target_id if direction == 'from' else c.source_id}")

    return 0
