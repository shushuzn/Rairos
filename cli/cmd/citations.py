"""CLI command: citations."""
from __future__ import annotations

import argparse
import sys
from typing import Literal

from cli._shared import get_db
from cli.warp import WarpBlocks


def _build_citations_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "citations",
        help="Show citation relationships for a paper",
        description="Display papers cited by or citing a given paper",
    )
    p.add_argument("--from", dest="citation_from", help="Paper ID to find citations from")
    p.add_argument("--to", dest="citation_to", help="Paper ID to find citations to")
    p.add_argument(
        "--format",
        choices=["text", "csv", "warp"],
        default="text",
        help="Output format (default: text)",
    )
    p.set_defaults(func=lambda args: _run_citations(args))
    return p  # type: ignore[no-any-return]


def _run_citations(args: argparse.Namespace) -> int:
    from rich.console import Console
    from io import StringIO
    c = Console(file=StringIO(), force_terminal=False, width=80)

    if not args.citation_from and not args.citation_to:
        print("Error: must specify --from or --to", file=sys.stderr)
        return 1  # type: ignore[no-any-return]

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
            return 1  # type: ignore[no-any-return]
        if not to_title:
            print(f"Error: paper {paper_to} not found in the database")
            return 1  # type: ignore[no-any-return]

        backward_from = db.get_citations(paper_from, "from")
        forward_to = db.get_citations(paper_to, "to")

        direct = any(c.target_id == paper_to for c in backward_from)
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
            body_lines = [
                f"[#FF8272]{paper_from}[/]  [#8E8E8E]←[/]  [#A5D5FE]{from_title[:50]}[/]",
                f"[#FF8272]{paper_to}[/]    [#8E8E8E]→[/]  [#A5D5FE]{to_title[:50]}[/]",
            ]
            if direct:
                body_lines.append("")
                body_lines.append("[#B4FA72]✅ DIRECT: A cites B[/]")
            if via_papers:
                via_ids = list({c.target_id for c in via_papers})
                paper_map = db.get_papers_bulk(via_ids)
                title_map = {pid: (paper_map[pid].title or '') for pid in via_ids if pid in paper_map}
                body_lines.append("")
                body_lines.append(f"[#FEFDC2]⚡ INDIRECT ({len(via_papers)} connections):[/]")
                for via in via_papers:
                    t = title_map.get(via.target_id, '?')
                    t_short = (t[:50] + "...") if len(t) > 53 else t
                    body_lines.append(f"  [#D0D1FE]{paper_from}[/] → [#FF8272]{via.target_id}[/] → [#FF8272]{paper_to}[/]")
                    body_lines.append(f"    [#8E8E8E]{t_short}[/]")
            if not direct and not via_papers:
                body_lines.append("")
                body_lines.append("[#8E8E8E]No citation path found between these papers[/]")

            c.print(WarpBlocks.panel(
                f"Citation Bridge — [#FF8272]{paper_from}[/] ↔ [#FF8272]{paper_to}[/]",
                "\n".join(body_lines)
))
            print(c.file.getvalue(), end="")  # type: ignore[union-attr,attr-defined]
        return 0

    # Single-direction mode
    paper_id = args.citation_from or args.citation_to
    direction: Literal["from", "to", "both"] = "from" if args.citation_from else "to"

    citations = db.get_citations(paper_id, direction)
    source_title = db.get_paper_title(paper_id)

    if source_title is None:
        print(f"Error: paper {paper_id} not found in the database")
        return 1

    if args.format == "csv":
        print("paper,count")
        print(f"{paper_id},{len(citations)}")
    else:
        label = "Backward Citations" if direction == "from" else "Forward Citations"
        citation_rows = []
        for citation in citations:
            cid = citation.target_id if direction == "from" else citation.source_id
            citation_rows.append([f"[#D0D1FE]{cid}[/]"])

        c.print(WarpBlocks.panel(
            f"{label} — [#FF8272]{paper_id}[/]",
            f"[#A5D5FE]{source_title[:60]}[/]"
        ))
        if citation_rows:
            c.print(WarpBlocks.table(
                ["Paper ID"],
                citation_rows,
                title=f"{len(citation_rows)} Citation(s)"
            ))
        else:
            c.print(WarpBlocks.panel(
                "No Citations",
"[#8E8E8E]This paper has no citations in the database[/]"
            ))
            print(c.file.getvalue(), end="")  # type: ignore[union-attr,attr-defined]

    return 0
