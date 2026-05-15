     1|"""
     2|paper trace CLI Command — query paper-code lineage.
     3|
     4|Usage:
     5|    airos paper trace 2106.09685          # show all traces for a paper
     6|    airos paper trace --list            # show recent traces across all papers
     7|    airos paper trace 2106.09685 --refs # show paper_section_refs details
     8|"""
# [LEGACY] Paper-code lineage tracker — depends on llm/trace/

     9|
    10|from __future__ import annotations
    11|
    12|import sys
    13|from 

from cli._shared import print_info, print_success, print_error


def _build_paper_trace_parser(subparsers):
    """Register paper trace subcommand."""
    p = subparsers.add_parser("trace", help="Query paper-code lineage")
    p.add_argument(
        "arxiv_id",
        nargs="?",
        default=None,
        help="arXiv ID (omit to list all recent traces)",
    )
    p.add_argument(
        "--list",
        "-l",
        action="store_true",
        help="List recent traces across all papers",
    )
    p.add_argument(
        "--refs",
        "-r",
        action="store_true",
        help="Show detailed paper_section_refs for each trace",
    )
    p.add_argument(
        "--limit",
        "-n",
        type=int,
        default=20,
        help="Max traces to show (default: 20)",
    )
    p.set_defaults(
        func=lambda a: paper_trace.callback(
            arxiv_id=a.arxiv_id,
            list_all=a.list,
            show_refs=a.refs,
            limit=a.limit,
        )
    )


@click.command("trace")
@click.argument("arxiv_id", required=False, default=None)
@click.option("--list", "-l", is_flag=True, help="List recent traces")
@click.option("--refs", "-r", is_flag=True, help="Show paper_section_refs details")
@click.option("--limit", "-n", type=int, default=20, help="Max traces to show")
def paper_trace(arxiv_id: str | None, list: bool, refs: bool, limit: int):
    """Query paper-code lineage traces.

    Examples:
        paper trace 2106.09685        — traces for one paper
        paper trace --list            — recent traces
        paper trace 2106.09685 --refs  — with provenance refs
    """
    from db.database import Database

    db = Database()
    db.init()

    if list or arxiv_id is None:
        traces = db.list_paper_code_traces(limit=limit)
        if not traces:
            print_info("No traces found.")
            return

        print_success(f"Recent paper-code traces ({len(traces)}):")
        print()
        for t in traces:
            title = t.get("paper_title") or t["paper_id"]
            coverage = (
                f"{t['tagged_lines']}/{t['total_code_lines']}"
                if t["total_code_lines"] > 0
                else "N/A"
            )
            pr = f"{t['benchmark_pass_rate']:.0%}" if t["benchmark_pass_rate"] else "—"
            print(
                f"  [\033[36m{t['paper_id']}\033[0m] {title[:50]}\n"
                f"    module={t['module_name']}  framework={t['framework']}\n"
                f"    coverage={coverage} lines  pass_rate={pr}  created={t['created_at'][:10]}"
            )
            if refs and t.get("paper_section_refs"):
                for ref in t["paper_section_refs"][:5]:
                    print(
                        f"    {ref['source_ref']} → line {ref['code_range']}: "
                        f"{ref.get('paper_text', '')[:60]}"
                    )
            print()
        return

    # Single paper trace
    traces = db.get_paper_code_trace(arxiv_id)
    if not traces:
        print_error(f"No traces found for paper {arxiv_id}.")
        return

    print_success(f"Traces for \033[36m{arxiv_id}\033[0m ({len(traces)}):")
    print()
    for i, t in enumerate(traces):
        coverage = (
            f"{t['tagged_lines']}/{t['total_code_lines']}" if t["total_code_lines"] > 0 else "N/A"
        )
        pr = f"{t['benchmark_pass_rate']:.0%}" if t["benchmark_pass_rate"] else "—"
        untagged = t.get("untagged_ranges") or []
        unreferenced = t.get("unreferenced_sources") or []

        print(
            f"Trace #{i + 1}  module={t['module_name']}  framework={t['framework']}\n"
            f"  code_path: {t['code_path']}\n"
            f"  coverage: {coverage} lines tagged\n"
            f"  pass_rate: {pr}  |  untagged ranges: {len(untagged)}  |  unreferenced: {len(unreferenced)}\n"
            f"  created: {t['created_at']}"
        )

        if refs and t.get("paper_section_refs"):
            print(f"  Provenance refs ({len(t['paper_section_refs'])}):")
            for ref in t["paper_section_refs"]:
                text = ref.get("paper_text", "")[:55]
                rng = ref.get("code_range", ())
                rng_str = f"L{rng[0]}" if rng else "?"
                print(f"    {ref['source_ref']} → {rng_str}: {text}")
        elif refs:
            print("  No provenance refs (code may not have # source: comments)")

        print()

    # Summary stats
    total_lines = sum(t["total_code_lines"] for t in traces)
    total_tagged = sum(t["tagged_lines"] for t in traces)
    if total_lines > 0:
        avg_cov = total_tagged / total_lines * 100
        print_info(
            f"Summary: {total_tagged}/{total_lines} lines traced ({avg_cov:.1f}%) "
            f"across {len(traces)} trace(s)"
        )
