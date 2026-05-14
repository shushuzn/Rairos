"""
MCP tool benchmark — compares Rust vs Python MCP dispatch latency for 5 pure-computation tools.

Each tool is called N times via both the Rust MCP backend (call_tool_rs) and
the Python fallback handle (direct Python function call), with pre-baked input data
(no DB, no API) to measure pure dispatch + algorithm time.
"""

import json
import statistics
import time

N = 100

# ── Python function helpers (defined before TOOLS) ──────────────────────────

def _py_args_impact_score():
    # ImpactScorer.score_paper(paper_id, title, year, raw_citations, citing_papers)
    return ("2301.00001", "Test Paper", 2023, 50, None)

def _py_args_impact_rank():
    # ImpactScorer.rank_papers(papers, top_k)
    papers = [{"paper_id": f"2301.{i:05d}", "title": f"Paper {i}",
               "year": 2020 + (i % 5), "citation_count": i * 10} for i in range(100)]
    return (papers, 10)

# ── Tool definitions ────────────────────────────────────────────────────────

TOOLS = [
    {
        "name": "impact_score_paper",
        "rust_args": {"arxiv_id": "2301.00001", "title": "Test Paper", "citation_count": 50, "year": 2023},
        "python_fn": None,  # set below
        "python_args": _py_args_impact_score,
    },
    {
        "name": "impact_rank",
        "rust_args": {"topic": "deep learning", "top_k": 10,
                      "papers": [{"arxiv_id": f"2301.{i:05d}", "title": f"Paper {i}",
                                   "citation_count": i * 10, "year": 2020 + (i % 5)}
                                  for i in range(100)]},
        "python_fn": None,
        "python_args": _py_args_impact_rank,
    },
    {
        "name": "citation_chain_families",
        "rust_args": {"arxiv_id": "2301.00001"},
        "python_fn": None,
        "python_args": lambda: None,
        "skip_python": True,  # Python version needs a built chain
    },
    {
        "name": "citation_chain_silent",
        "rust_args": {"arxiv_id": "2301.00001"},
        "python_fn": None,
        "python_args": lambda: None,
        "skip_python": True,
    },
    {
        "name": "citation_chain_render",
        "rust_args": {"arxiv_id": "2301.00001", "format": "text"},
        "python_fn": None,
        "python_args": lambda: None,
        "skip_python": True,
    },
]


def load_rust():
    from rairos_mcp_py import call_tool_rs
    return call_tool_rs


def load_python():
    """Lazily import Python versions of the same algorithms."""
    from llm.impact_scorer import ImpactScorer
    scorer = ImpactScorer()
    return {
        "impact_score_paper": scorer.score_paper,
        "impact_rank": scorer.rank_papers,
    }


def bench_rust(call_tool_rs, name, args):
    args_json = json.dumps(args)
    times = []
    errors = 0
    for _ in range(N):
        t0 = time.perf_counter()
        result = call_tool_rs(name, args_json)
        elapsed = time.perf_counter() - t0
        if result is None:
            errors += 1
        else:
            times.append(elapsed)
    return times, errors


def bench_python(fn, args):
    times = []
    for _ in range(N):
        t0 = time.perf_counter()
        fn(*args)
        elapsed = time.perf_counter() - t0
        times.append(elapsed)
    return times


def report(name, rust_times, py_times, rust_errors):
    def stats(t):
        if not t:
            return {"mean": float("nan"), "min": float("nan"), "max": float("nan"),
                    "p50": float("nan"), "p99": float("nan")}
        t.sort()
        return {
            "mean": statistics.mean(t),
            "min": t[0],
            "max": t[-1],
            "p50": t[len(t) // 2],
            "p99": t[int(len(t) * 0.99)],
        }

    rs = stats(rust_times)
    ps = stats(py_times) if py_times else {}

    rust_ms = rs["mean"] * 1000
    py_ms = ps.get("mean", float("nan")) * 1000
    speedup = py_ms / rust_ms if rust_ms > 0 and not (py_ms != py_ms) else float("nan")

    err_flag = f"  [{rust_errors} ERR]" if rust_errors else ""
    print(f"{name:35s}  Rust:{rust_ms:>10.3f}ms  Python:{py_ms:>10.3f}ms  Speedup:{speedup:>8.1f}x{err_flag}")
    if rust_times:
        print(f"  {'':35s}  Rust p50={rs['p50']*1000:.3f}ms  p99={rs['p99']*1000:.3f}ms  "
              f"Python p50={ps.get('p50',float('nan'))*1000:.3f}ms  p99={ps.get('p99',float('nan'))*1000:.3f}ms")


def main():
    print("=" * 80)
    print("MCP Tool Dispatch Benchmark — Rust vs Python")
    print(f"  {N} iterations per tool, pre-baked data (no DB/API)")
    print("=" * 80)
    print()

    call_tool_rs = load_rust()
    py_funcs = load_python()
    UPDATE_MSG_HEIGHT = len(TOOLS) + 4

    # Warmup
    for _ in range(5):
        call_tool_rs("impact_score_paper", json.dumps(TOOLS[0]["rust_args"]))

    print(f"{'Tool':35s}  {'Rust':>12s}  {'Python':>12s}  {'Speedup':>10s}")
    print("-" * 72)

    for tool in TOOLS:
        name = tool["name"]
        rust_times, rust_errors = bench_rust(call_tool_rs, name, tool["rust_args"])

        py_times = None
        if not tool.get("skip_python"):
            fn = py_funcs.get(name)
            if fn:
                args = tool["python_args"]()
                py_times = bench_python(fn, args)

        report(name, rust_times, py_times or [], rust_errors)
        print()

    print("=" * 80)
    print("Note: Rust times include PyO3 FFI + JSON-RPC dispatch overhead (now ~5µs baseline).")
    print("Python times are direct function calls with no dispatch overhead.")
    print("For pure compute comparison, see cargo bench results (23µs for 1000 impact scores).")
    print("=" * 80)


if __name__ == "__main__":
    main()
