"""
CLI command: benchmark — Cross-paper benchmark result comparison.

Usage:
    airos benchmark detect 2604.22754
    airos benchmark compare 2604.22754 2302.00763
    airos benchmark list --limit 20
"""

from __future__ import annotations

import argparse

from cli._shared import get_db, print_info, print_error, print_success
from cli.warp import WarpBlocks
from llm.benchmark import BenchmarkComparator


def _build_benchmark_parser(subparsers) -> argparse.ArgumentParser:
    """Build the benchmark subcommand parser."""
    p = subparsers.add_parser(
        "benchmark",
        help="Cross-paper benchmark comparison",
        description="Detect and compare benchmark results across papers.",
    )
    sub = p.add_subparsers(dest="benchmark_cmd", help="Benchmark commands")

    # detect — identify benchmark tables in a paper
    detect_p = sub.add_parser("detect", help="Detect benchmark tables in a paper")
    detect_p.add_argument("paper_id", help="Paper ID to analyze")
    detect_p.add_argument("--verbose", "-v", action="store_true", help="Show table contents")
    detect_p.add_argument(
        "--format",
        "-f",
        default="text",
        choices=["text", "warp"],
        help="Output format (default: text)",
    )

    # list — list papers with benchmark-like tables
    list_p = sub.add_parser("list", help="List papers with benchmark tables")
    list_p.add_argument("--limit", type=int, default=20, help="Maximum results (default: 20)")
    list_p.add_argument(
        "--format",
        "-f",
        default="text",
        choices=["text", "warp"],
        help="Output format (default: text)",
    )

    # compare — cross-paper benchmark comparison
    compare_p = sub.add_parser("compare", help="Compare benchmarks across papers")
    compare_p.add_argument("paper_ids", nargs="+", help="Paper IDs to compare")
    compare_p.add_argument(
        "--format",
        "-f",
        default="text",
        choices=["text", "markdown", "json", "warp"],
        help="Output format (default: text)",
    )
    compare_p.add_argument(
        "--metric", "-m", default=None, help="Filter by metric name (e.g., 'Accuracy')"
    )

    # viz — benchmark comparison visualization
    viz_p = sub.add_parser("viz", help="Visualize benchmark comparison as charts")
    viz_p.add_argument("paper_ids", nargs="+", help="Paper IDs to compare")
    viz_p.add_argument(
        "--output",
        "-o",
        default="benchmark_chart.html",
        help="Output file path (default: benchmark_chart.html)",
    )
    viz_p.add_argument(
        "--format",
        "-f",
        default="html",
        choices=["html", "svg", "json"],
        help="Output format (default: html)",
    )
    viz_p.add_argument(
        "--metric", "-m", default=None, help="Filter by metric name (e.g., 'Accuracy')"
    )

    return p  # type: ignore[no-any-return]


def _run_benchmark(args: argparse.Namespace) -> int:
    """Run benchmark command."""
    db = get_db()
    db.init()

    comparator = BenchmarkComparator(db=db)

    if args.benchmark_cmd == "detect":
        return _run_detect(args, comparator)

    elif args.benchmark_cmd == "list":
        return _run_list(args, comparator)

    elif args.benchmark_cmd == "compare":
        return _run_compare(args, comparator)

    elif args.benchmark_cmd == "viz":
        return _run_viz(args, comparator)

    else:
        print_error("Usage: airos benchmark {detect|list|compare|viz} [...]")
        return 1  # type: ignore[no-any-return]


def _run_detect(args: argparse.Namespace, comparator: BenchmarkComparator) -> int:
    """Detect benchmark tables in a single paper."""
    pid = args.paper_id
    tables = comparator.detect_tables(pid)
    use_warp = getattr(args, "format", "text") == "warp"

    if not tables:
        if use_warp:
            print(
                WarpBlocks.panel(
                    "Benchmark Detection", f"[#8E8E8E]No benchmark tables found in {pid}[/]"
                )
            )
        else:
            print_info(f"No benchmark-like tables found in paper: {pid}")
        return 1  # type: ignore[no-any-return]

    if use_warp:
        rows = []
        for i, t in enumerate(tables):
            t.caption[:40] + "..." if len(t.caption) > 40 else t.caption
            metrics_str = ", ".join(t.metrics[:5]) if t.metrics else "—"
            rows.append([str(i + 1), t.benchmark_name, f"[#A5D5FE]p{t.page + 1}[/]", metrics_str])

        print(
            WarpBlocks.table(
                ["#", "Benchmark", "Page", "Metrics"], rows, title=f"Benchmark Tables in {pid}"
            )
        )

        if args.verbose and t.metrics and t.rows:
            from rich.console import Console

            c = Console()
            metric = t.metrics[0] if t.metrics else ""
            rows_data = []
            for row in t.rows[:10]:
                model = str(row[0].raw_value) if hasattr(row[0], "raw_value") else str(row[0])
                if len(row) > 1:
                    cell = row[1]
                    numeric = cell.numeric if hasattr(cell, "numeric") else None
                    if numeric is not None:
                        rows_data.append([model[:28], f"[#B4FA72]{numeric:.4f}[/]"])
            if rows_data:
                c.print(
                    WarpBlocks.table(
                        ["Model", metric or "Score"], rows_data, title=f"  {metric} Detail"
                    )
                )
    else:
        print_success(f"Found {len(tables)} benchmark table(s) in {pid}:\n")
        for i, t in enumerate(tables):
            print(f"  [{i + 1}] {t.benchmark_name}")
            print(f"      Caption: {t.caption[:100]}")
            print(f"      Page: {t.page + 1}")
            print(f"      Metrics: {', '.join(t.metrics[:8])}")

            if args.verbose and t.metrics and t.rows:
                metric = t.metrics[0] if t.metrics else ""
                print(f"\n      {metric} summary:")
                for row in t.rows[:10]:
                    model = str(row[0].raw_value) if hasattr(row[0], "raw_value") else str(row[0])
                    if len(row) > 1:
                        cell = row[1]
                        numeric = cell.numeric if hasattr(cell, "numeric") else None
                        if numeric is not None:
                            print(f"        {model[:25]:<26} {numeric:.4f}")
            print()

    return 0


def _run_list(args: argparse.Namespace, comparator: BenchmarkComparator) -> int:
    """List papers with benchmark-like tables."""
    db = comparator.db
    tables = db.get_all_experiment_tables()
    use_warp = getattr(args, "format", "text") == "warp"

    # Group by paper, count benchmark-like
    from collections import defaultdict

    paper_stats: dict = defaultdict(lambda: {"total": 0, "benchmark": 0, "benchmarks": []})
    for t in tables:
        stats = paper_stats[t.paper_id]
        stats["total"] += 1
        if comparator._is_benchmark_like(t):
            stats["benchmark"] += 1
            from llm.benchmark import _guess_benchmark_name

            name = _guess_benchmark_name(t.table_caption, t.headers)
            if name not in stats["benchmarks"]:
                stats["benchmarks"].append(name)

    if not paper_stats:
        if use_warp:
            print(
                WarpBlocks.panel("Benchmark List", "[#8E8E8E]No papers with stored tables found[/]")
            )
        else:
            print_info("No papers with stored tables found.")
        return 0

    # Sort by benchmark table count
    ranked = sorted(paper_stats.items(), key=lambda x: x[1]["benchmark"], reverse=True)
    ranked = ranked[: args.limit]

    if use_warp:
        rows = []
        for pid, stats in ranked:
            benchmarks = ", ".join(stats["benchmarks"][:3])
            bench = stats["benchmark"]
            badge = "[#B4FA72]●[/]" if bench > 0 else "[#8E8E8E]○[/]"
            rows.append([badge, pid, str(stats["total"]), str(bench), benchmarks[:40]])
        print(
            WarpBlocks.table(
                ["", "Paper ID", "Total", "Bench", "Benchmarks"],
                rows,
                title=f"Benchmark Tables ({len(ranked)} papers)",
            )
        )
    else:
        print_success(f"Papers with benchmark tables ({len(ranked)} results):\n")
        print(f"{'Paper ID':<16} {'Total':<7} {'Bench':<7} {'Benchmarks'}")
        print("-" * 60)
        for pid, stats in ranked:
            benchmarks = ", ".join(stats["benchmarks"][:3])
            print(f"{pid:<16} {stats['total']:<7} {stats['benchmark']:<7} {benchmarks}")

    return 0


def _run_compare(args: argparse.Namespace, comparator: BenchmarkComparator) -> int:
    """Compare benchmarks across multiple papers."""
    paper_ids = args.paper_ids

    if len(paper_ids) < 1:
        print_error("Need at least 1 paper ID to compare")
        return 1

    print_info(f"Comparing benchmarks across {len(paper_ids)} papers...")

    result = comparator.compare(paper_ids)

    # Filter by metric if specified
    if args.metric:
        metric_lower = args.metric.lower()
        result.matches = [m for m in result.matches if metric_lower in m.metric_name.lower()]

    if not result.matches:
        if args.format == "warp":
            print(WarpBlocks.panel("Benchmark Compare", "[#8E8E8E]No matching benchmarks found[/]"))
        else:
            print_info("No matching benchmarks found across papers.")
            for pid, tables in result.tables_found.items():
                print_info(f"\n  {pid}: {len(tables)} benchmark table(s)")
                for t in tables:
                    print_info(f"    - {t.benchmark_name}: {', '.join(t.metrics[:3])}")
        return 0

    if args.format == "json":
        print(comparator.render_json(result))
    elif args.format == "markdown":
        print(comparator.render_markdown(result))
    elif args.format == "warp":
        _render_compare_warp(result, comparator)
    else:
        print(comparator.render_text(result))

    return 0


def _render_compare_warp(result, comparator: BenchmarkComparator) -> None:
    """Render benchmark comparison in Warp style."""
    from rich.console import Console

    c = Console()

    c.rule("[bold #FF8272]  Benchmark Comparison  [/]")
    c.print()

    # Group by benchmark + metric
    from collections import defaultdict

    groups: dict = defaultdict(list)
    for m in result.matches:
        groups[(m.benchmark_name, m.metric_name)].extend(m.entries)

    for (bench_name, metric_name), entries in groups.items():
        direction = "↑" if comparator._is_higher_better(metric_name) else "↓"  # type: ignore[attr-defined]
        direction_label = (
            "[#B4FA72]higher better[/]"
            if comparator._is_higher_better(metric_name)
            else "[#FF5555]lower better[/]"
        )  # type: ignore[attr-defined]

        # Build table rows
        rows = []
        for paper_id, value, model in sorted(entries, key=lambda x: -x[1]):
            value_str = (
                f"[#B4FA72]{value:.4f}[/]"
                if value >= 0.8
                else f"[#FEFDC2]{value:.4f}[/]"
                if value >= 0.5
                else f"[#FF5555]{value:.4f}[/]"
            )
            rows.append([paper_id[:18], model[:28], value_str])

        c.print(
            WarpBlocks.table(
                ["Paper", "Model", f"Score {direction}"],
                rows,
                title=f"  {bench_name} — {metric_name} ({direction_label})",
            )
        )
        c.print()


def _run_viz(args: argparse.Namespace, comparator: BenchmarkComparator) -> int:
    """Generate benchmark comparison visualization."""
    paper_ids = args.paper_ids
    output_path = args.output

    print_info(f"Comparing benchmarks across {len(paper_ids)} papers for visualization...")

    result = comparator.compare(paper_ids)

    # Filter by metric if specified
    if args.metric:
        metric_lower = args.metric.lower()
        result.matches = [m for m in result.matches if metric_lower in m.metric_name.lower()]

    if not result.matches:
        print_info("No matching benchmarks found to visualize.")
        for pid, tables in result.tables_found.items():
            print_info(f"\n  {pid}: {len(tables)} benchmark table(s)")
            for t in tables:
                print_info(f"    - {t.benchmark_name}: {', '.join(t.metrics[:3])}")
        return 0

    from viz.benchmark_viz import BenchmarkViz

    viz = BenchmarkViz()

    if args.format == "json":
        import json as _json

        print(_json.dumps(viz.to_json(result), indent=2, ensure_ascii=False))
        return 0
    elif args.format == "svg":
        print(viz.render_svg(result))
        return 0
    else:
        out = viz.render_html(result, output_path)
        print_success(f"Chart saved to: {out}")
        return 0
