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
    detect_p.add_argument("--verbose", "-v", action="store_true",
                          help="Show table contents")

    # list — list papers with benchmark-like tables
    list_p = sub.add_parser("list", help="List papers with benchmark tables")
    list_p.add_argument("--limit", type=int, default=20,
                        help="Maximum results (default: 20)")

    # compare — cross-paper benchmark comparison
    compare_p = sub.add_parser("compare", help="Compare benchmarks across papers")
    compare_p.add_argument("paper_ids", nargs="+", help="Paper IDs to compare")
    compare_p.add_argument("--format", "-f", default="text",
                           choices=["text", "markdown", "json"],
                           help="Output format (default: text)")
    compare_p.add_argument("--metric", "-m", default=None,
                           help="Filter by metric name (e.g., 'Accuracy')")

    # viz — benchmark comparison visualization
    viz_p = sub.add_parser("viz", help="Visualize benchmark comparison as charts")
    viz_p.add_argument("paper_ids", nargs="+", help="Paper IDs to compare")
    viz_p.add_argument("--output", "-o", default="benchmark_chart.html",
                       help="Output file path (default: benchmark_chart.html)")
    viz_p.add_argument("--format", "-f", default="html",
                       choices=["html", "svg", "json"],
                       help="Output format (default: html)")
    viz_p.add_argument("--metric", "-m", default=None,
                       help="Filter by metric name (e.g., 'Accuracy')")

    return p


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
        return 1


def _run_detect(args: argparse.Namespace, comparator: BenchmarkComparator) -> int:
    """Detect benchmark tables in a single paper."""
    pid = args.paper_id
    tables = comparator.detect_tables(pid)

    if not tables:
        print_info(f"No benchmark-like tables found in paper: {pid}")
        return 1

    print_success(f"Found {len(tables)} benchmark table(s) in {pid}:\n")

    for i, t in enumerate(tables):
        print(f"  [{i + 1}] {t.benchmark_name}")
        print(f"      Caption: {t.caption[:100]}")
        print(f"      Page: {t.page + 1}")
        print(f"      Metrics: {', '.join(t.metrics[:8])}")

        if args.verbose and t.metrics and t.rows:
            # Show mini table
            metric = t.metrics[0] if t.metrics else ""
            print(f"\n      {metric} summary:")
            for row in t.rows[:10]:  # limit to 10 rows
                model = str(row[0].raw_value) if hasattr(row[0], 'raw_value') else str(row[0])
                if len(row) > 1:
                    cell = row[1]
                    numeric = cell.numeric if hasattr(cell, 'numeric') else None
                    if numeric is not None:
                        print(f"        {model[:25]:<26} {numeric:.4f}")
        print()

    return 0


def _run_list(args: argparse.Namespace, comparator: BenchmarkComparator) -> int:
    """List papers with benchmark-like tables."""
    db = comparator.db
    tables = db.get_all_experiment_tables()

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
        print_info("No papers with stored tables found.")
        return 0

    # Sort by benchmark table count
    ranked = sorted(paper_stats.items(), key=lambda x: x[1]["benchmark"], reverse=True)
    ranked = ranked[:args.limit]

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
        result.matches = [
            m for m in result.matches
            if metric_lower in m.metric_name.lower()
        ]

    if not result.matches:
        print_info("No matching benchmarks found across papers.")
        # Show what each paper has
        for pid, tables in result.tables_found.items():
            print_info(f"\n  {pid}: {len(tables)} benchmark table(s)")
            for t in tables:
                print_info(f"    - {t.benchmark_name}: {', '.join(t.metrics[:3])}")
        return 0

    if args.format == "json":
        print(comparator.render_json(result))
    elif args.format == "markdown":
        print(comparator.render_markdown(result))
    else:
        print(comparator.render_text(result))

    return 0


def _run_viz(args: argparse.Namespace, comparator: BenchmarkComparator) -> int:
    """Generate benchmark comparison visualization."""
    paper_ids = args.paper_ids
    output_path = args.output

    print_info(f"Comparing benchmarks across {len(paper_ids)} papers for visualization...")

    result = comparator.compare(paper_ids)

    # Filter by metric if specified
    if args.metric:
        metric_lower = args.metric.lower()
        result.matches = [
            m for m in result.matches
            if metric_lower in m.metric_name.lower()
        ]

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
