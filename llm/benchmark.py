"""
Cross-paper benchmark comparison.

Reads experiment_tables from the database, identifies benchmark-like tables,
and compares numeric results across papers to produce leaderboard-style output.

Usage:
    from llm.benchmark import BenchmarkComparator
    bc = BenchmarkComparator(db)
    results = bc.compare(["2604.22754", "2302.00763"])
    print(bc.render_leaderboard(results))
"""
from __future__ import annotations

import re
from collections import defaultdict
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Tuple

from db.database import Database, ExperimentTableRecord

# ── Patterns for detecting benchmark-like content ─────────────────────────

_METRIC_KEYWORDS = [
    "accuracy", "bleu", "rouge", "f1", "f-score", "f-measure", "precision",
    "recall", "perplexity", "ppl", "wer", "cer", "map", "ndcg", "auc",
    "mse", "mae", "rmse", "r2", "psnr", "ssim", "iou", "miou",
    "top-1", "top-5", "top1", "top5", "error", "err", "loss",
    "latency", "throughput", "params", "flops", "macs", "gflops",
    "win rate", "winrate", "elo", "score", "score", "pass@",
    "humaneval", "mbpp", "gsm8k", "mmlu", "hellaswag", "arc",
    "task", "dataset", "model", "method", "result",
]

_BENCHMARK_NAMES = [
    "imagenet", "cifar", "mnist", "svhn", "imagenet",
    "coco", "pascal voc", "pascal", "cityscapes", "ade20k",
    "squad", "glue", "superglue", "xnli", "wmt",
    "multinli", "sst", "sst-2", "cola", "mrpc", "qnli", "rte", "wnli",
    "wikitext", "ptb", "penn treebank", "enwik8", "text8",
    "librispeech", "wsj", "tedlium", "voxceleb",
    "halalbench", "halal", "openai", "truthfulqa",
    "gsm8k", "math", "humaneval", "mbpp", "mmlu",
    "arc-e", "arc-c", "arc-easy", "arc-challenge",
    "hellaswag", "piqa", "winogrande", "boolq",
    "siqa", "openbookqa", "anli", "storycloze",
    "lambada", "wikitext-103",
]


# ── Data structures ──────────────────────────────────────────────────────


@dataclass
class NormalizedMetric:
    """A single metric value normalized for comparison."""
    raw_value: str          # original text
    numeric: Optional[float] = None  # parsed float
    is_higher_better: bool = True
    confidence: float = 1.0  # how confident we are in parsing (0-1)


@dataclass
class BenchmarkTable:
    """A table identified as containing benchmark results."""
    paper_id: str
    table_id: int
    caption: str
    page: int
    headers: List[str]
    rows: List[List[NormalizedMetric]]
    # Extracted benchmark info
    benchmark_name: str = ""
    models: List[str] = field(default_factory=list)
    metrics: List[str] = field(default_factory=list)


@dataclass
class BenchmarkMatch:
    """A set of matching benchmark tables across papers."""
    benchmark_name: str
    metric_name: str
    entries: List[Tuple[str, float, str]] = field(default_factory=list)
    # (paper_id, value, model_name)


@dataclass
class BenchmarkResult:
    """Full result of cross-paper benchmark comparison."""
    paper_ids: List[str]
    tables_found: Dict[str, List[BenchmarkTable]] = field(default_factory=dict)
    matches: List[BenchmarkMatch] = field(default_factory=list)
    unmatched: List[BenchmarkTable] = field(default_factory=list)


# ── Helper functions ─────────────────────────────────────────────────────


def _contains_numeric(cell: str) -> bool:
    """Check if a cell contains a numeric value."""
    cell = cell.strip()
    if not cell:
        return False
    # Match: 92.5, 92.5%, 0.925, 92.5±0.3, 92.5/100, 1e-3
    return bool(re.search(r'\d+\.?\d*', cell))


def _parse_numeric(value: str) -> Optional[float]:
    """Parse a string into a normalized float.

    Handles: "92.5%", "0.925", "92.5 ± 0.3", "92.5/100", "1e-3", "1.2B"
    """
    value = value.strip().replace(" ", "").replace(",", "")

    if not value:
        return None

    # Percentage: "92.5%"
    m = re.match(r'^([\d.]+)\s*%$', value)
    if m:
        return float(m.group(1))

    # Range with ±: "92.5±0.3" → take the main value
    m = re.match(r'^([\d.]+)±', value)
    if m:
        return float(m.group(1))

    # Fraction: "92.5/100"
    m = re.match(r'^([\d.]+)/([\d.]+)$', value)
    if m:
        return float(m.group(1)) / float(m.group(2))

    # Suffixes: "1.2B", "350M"
    m = re.match(r'^([\d.]+)([BKMG])$', value.upper())
    if m:
        multipliers = {"B": 1e9, "M": 1e6, "K": 1e3, "G": 1e9}
        return float(m.group(1)) * multipliers.get(m.group(2), 1)

    # Simple number
    try:
        return float(value)
    except ValueError:
        pass

    # Scientific notation
    try:
        m = re.match(r'^([\d.]+)[eE]([+-]?\d+)$', value)
        if m:
            return float(value)
    except ValueError:
        pass

    return None


def _is_higher_better(metric_name: str) -> bool:
    """Determine if higher values are better for a given metric."""
    lower_better = {
        "perplexity", "ppl", "wer", "cer", "mse", "mae", "rmse",
        "loss", "error", "err", "latency", "flops", "macs", "gflops",
        "params", "token", "time", "runtime", "cost",
        "top-1 error", "top-5 error", "word error",
    }
    name_lower = metric_name.lower().strip()
    for lb in lower_better:
        if lb in name_lower:
            return False
    return True


def _fuzzy_match_name(name1: str, name2: str) -> float:
    """Compute fuzzy match score between two benchmark/model names.

    Returns a score between 0 and 1.
    """
    n1 = name1.lower().strip().replace("-", " ").replace("_", " ")
    n2 = name2.lower().strip().replace("-", " ").replace("_", " ")

    if n1 == n2:
        return 1.0

    # One contains the other
    if n1 in n2 or n2 in n1:
        return 0.9

    # Word overlap / Jaccard
    w1 = set(n1.split())
    w2 = set(n2.split())
    if not w1 or not w2:
        return 0.0

    intersection = w1 & w2
    union = w1 | w2
    return len(intersection) / len(union)


def _guess_benchmark_name(caption: str, headers: List[str]) -> str:
    """Guess the benchmark/dataset name from caption and headers."""
    text = caption.lower()
    for name in _BENCHMARK_NAMES:
        if name.lower() in text:
            return name
    # Check headers too
    for h in headers:
        h_lower = h.lower()
        for name in _BENCHMARK_NAMES:
            if name.lower() in h_lower:
                return name
    return caption[:80] if caption else "Unknown"


# ── Core comparator ──────────────────────────────────────────────────────


class BenchmarkComparator:
    """Detect and compare benchmark results across papers."""

    def __init__(self, db: Database):
        self.db = db

    def detect_tables(self, paper_id: str) -> List[BenchmarkTable]:
        """Detect benchmark-like tables from a paper's experiment_tables."""
        tables = self.db.get_experiment_tables(paper_id)
        result = []

        for tbl in tables:
            if not self._is_benchmark_like(tbl):
                continue

            headers = tbl.headers
            normalized_rows = []
            for row in tbl.rows:
                norm_row = []
                for cell in row:
                    cell_str = str(cell)
                    numeric = _parse_numeric(cell_str)
                    nm = NormalizedMetric(
                        raw_value=cell_str,
                        numeric=numeric,
                        is_higher_better=True,  # will be overridden per-metric
                        confidence=0.9 if numeric is not None else 0.0,
                    )
                    norm_row.append(nm)
                normalized_rows.append(norm_row)

            bench_name = _guess_benchmark_name(tbl.table_caption, headers)
            models = self._extract_models(tbl)
            metrics = self._extract_metrics(headers, tbl.table_caption)

            result.append(BenchmarkTable(
                paper_id=paper_id,
                table_id=tbl.id,
                caption=tbl.table_caption,
                page=tbl.page,
                headers=headers,
                rows=normalized_rows,
                benchmark_name=bench_name,
                models=models,
                metrics=metrics,
            ))

        return result

    def _is_benchmark_like(self, tbl: ExperimentTableRecord) -> bool:
        """Heuristic: is this table likely containing benchmark results?

        Criteria:
        - Has headers that suggest metrics (Accuracy, BLEU, etc.)
        - Contains numeric values in most cells
        """
        if not tbl.headers or len(tbl.headers) < 2:
            return False
        if not tbl.rows or len(tbl.rows) < 1:
            return False

        # Check headers for metric keywords
        header_text = " ".join(tbl.headers).lower()
        if any(kw in header_text for kw in _METRIC_KEYWORDS):
            return True

        # Check caption for benchmark names
        cap_lower = tbl.table_caption.lower()
        if any(bn in cap_lower for bn in _BENCHMARK_NAMES):
            return True

        return False

    def _extract_models(self, tbl: BenchmarkTable | ExperimentTableRecord) -> List[str]:
        """Extract model/method names from the first column."""
        models = []
        for row in tbl.rows:
            if isinstance(row, list) and row:
                first = str(row[0]) if not isinstance(row[0], NormalizedMetric) else str(row[0].raw_value)
                if first.strip() and not _contains_numeric(first) and len(first) < 60:
                    models.append(first.strip())
                elif isinstance(row[0], NormalizedMetric):
                    models.append("")
        return models

    def _extract_metrics(self, headers: List[str], caption: str) -> List[str]:
        """Extract metric names from headers."""
        metrics = []
        for h in headers[1:]:  # skip first column (usually model name)
            if h.strip() and len(h) < 50:
                metrics.append(h.strip())
        return metrics

    def compare(self, paper_ids: List[str]) -> BenchmarkResult:
        """Compare benchmarks across multiple papers.

        Finds matching benchmark tables and normalizes results.
        """
        result = BenchmarkResult(paper_ids=paper_ids)

        # Detect tables for each paper
        for pid in paper_ids:
            result.tables_found[pid] = self.detect_tables(pid)

        # Find cross-paper matches by benchmark name
        matches = self._match_benchmarks(result.tables_found)
        result.matches = matches

        # Collect unmatched tables
        matched_ids = set()
        for m in matches:
            for pid, _, _ in m.entries:
                matched_ids.add(pid)
        # (unmatched tracking is best-effort; skip for now)

        return result

    def _match_benchmarks(
        self, tables_by_paper: Dict[str, List[BenchmarkTable]]
    ) -> List[BenchmarkMatch]:
        """Find common benchmarks across papers.

        Strategy:
        1. Group all tables by fuzzy-matched benchmark name
        2. For each group, find common metrics across papers
        3. Extract values for each metric per paper
        """
        # Step 1: Group tables by benchmark name
        all_tables: List[BenchmarkTable] = []
        for pid, tables in tables_by_paper.items():
            all_tables.extend(tables)

        name_groups: Dict[str, List[BenchmarkTable]] = defaultdict(list)
        seen_names: Dict[str, str] = {}  # raw_name → canonical_name

        for t in all_tables:
            matched = False
            for canonical in seen_names:
                score = _fuzzy_match_name(t.benchmark_name, canonical)
                if score > 0.5:
                    name_groups[canonical].append(t)
                    matched = True
                    break
            if not matched:
                canonical = t.benchmark_name
                seen_names[canonical] = canonical
                name_groups[canonical].append(t)

        # Step 2: For each group with 2+ papers, find common metrics
        matches: List[BenchmarkMatch] = []
        for bench_name, tables in name_groups.items():
            paper_count = len({t.paper_id for t in tables})
            if paper_count < 1:
                continue

            # Find common metrics across tables
            all_metrics: Dict[str, List[Tuple[str, str, float, int]]] = defaultdict(list)
            # metric_name → [(paper_id, model, value, table_id)]

            for t in tables:
                metric_cols = t.headers[1:]
                for col_idx, metric_name in enumerate(metric_cols):
                    if not metric_name.strip():
                        continue
                    for row in sorted(t.rows, key=lambda r: len(r) if isinstance(r, list) else 0, reverse=True):
                        if col_idx + 1 < len(row):
                            cell = row[col_idx + 1]
                            if isinstance(cell, NormalizedMetric) and cell.numeric is not None:
                                model = str(row[0].raw_value) if isinstance(row[0], NormalizedMetric) else str(row[0])
                                all_metrics[metric_name].append(
                                    (t.paper_id, model, cell.numeric, t.table_id)
                                )

            for metric_name, entries in all_metrics.items():
                if len(entries) < 1:
                    continue
                direction = _is_higher_better(metric_name)
                # Take best entry per paper (best = max if higher-better, min if lower-better)
                paper_best: Dict[str, Tuple[str, float]] = {}
                for pid, model, val, _ in entries:
                    if pid not in paper_best:
                        paper_best[pid] = (model, val)
                    else:
                        _, existing = paper_best[pid]
                        better = (val > existing) if direction else (val < existing)
                        if better:
                            paper_best[pid] = (model, val)

                if len(paper_best) >= 1:
                    match_entries = [(pid, val, model) for pid, (model, val) in paper_best.items()]
                    # Sort: higher-better = descending, lower-better = ascending
                    match_entries.sort(key=lambda x: x[1], reverse=direction)
                    matches.append(BenchmarkMatch(
                        benchmark_name=bench_name,
                        metric_name=metric_name,
                        entries=match_entries,
                    ))

        return matches

    # ── Rendering ────────────────────────────────────────────────────────

    def render_leaderboard(self, result: BenchmarkResult) -> str:
        """Render benchmark comparison as a formatted leaderboard."""
        lines: List[str] = []
        lines.append(f"\n{'=' * 70}")
        lines.append(f"  Cross-Paper Benchmark Comparison")
        lines.append(f"  Papers: {', '.join(result.paper_ids)}")
        lines.append(f"{'=' * 70}")

        if not result.matches:
            lines.append("\n  No matching benchmarks found across papers.")
            return "\n".join(lines)

        for match in result.matches:
            lines.append(f"\n{'─' * 70}")
            lines.append(f"  {match.benchmark_name} → {match.metric_name}")
            direction = "↑ higher is better" if _is_higher_better(match.metric_name) else "↓ lower is better"
            lines.append(f"  ({direction})")
            lines.append(f"{'─' * 70}")

            # Table header
            lines.append(f"  {'Rank':<6} {'Paper ID':<16} {'Model':<22} {'Score':<10}")
            lines.append(f"  {'─' * 6} {'─' * 16} {'─' * 22} {'─' * 10}")

            for rank, (pid, val, model) in enumerate(match.entries, 1):
                medal = {1: "🥇", 2: "🥈", 3: "🥉"}.get(rank, f"  {rank}.")
                lines.append(f"  {medal:<6} {pid:<16} {model[:20]:<22} {val:<10.4f}")

        lines.append(f"\n{'=' * 70}")
        return "\n".join(lines)

    def render_text(self, result: BenchmarkResult) -> str:
        """Render as plain text table."""
        return self.render_leaderboard(result)

    def render_markdown(self, result: BenchmarkResult) -> str:
        """Render as Markdown."""
        lines: List[str] = []
        lines.append(f"# Benchmark Comparison")
        lines.append(f"")
        lines.append(f"**Papers**: {', '.join(result.paper_ids)}")
        lines.append(f"")

        for match in result.matches:
            lines.append(f"## {match.benchmark_name} — {match.metric_name}")
            direction = "↑" if _is_higher_better(match.metric_name) else "↓"
            lines.append(f"")
            lines.append(f"| Rank | Paper ID | Model | Score |")
            lines.append(f"|------|----------|-------|-------|")
            for rank, (pid, val, model) in enumerate(match.entries, 1):
                lines.append(f"| {rank} | `{pid}` | {model} | {val:.4f} |")
            lines.append(f"")

        return "\n".join(lines)

    def render_json(self, result: BenchmarkResult) -> str:
        """Render as JSON string."""
        import json
        output = {}
        for match in result.matches:
            output[f"{match.benchmark_name}/{match.metric_name}"] = [
                {"rank": rank, "paper_id": pid, "model": model, "score": val}
                for rank, (pid, val, model) in enumerate(match.entries, 1)
            ]
        return json.dumps(output, indent=2, ensure_ascii=False)
