"""
Benchmark Leaderboard — ranked paper2code implementations.

Persists benchmark results keyed by arxiv_id, ranks by combined score:
  combined_score = pass_rate × 0.7 + coverage_ratio × 0.3

闭环:
  paper2code pipeline (run_benchmark) → upsert_leaderboard_entry
  → ranked leaderboard → MCP tool for status/rankings/render_html
"""

from __future__ import annotations

import json
from dataclasses import asdict, dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional

GP_DIR = Path.home() / ".ai_research_os" / "evolution"
LEADERBOARD_FILE = GP_DIR / "leaderboard.json"

# Weights for combined score
PASS_RATE_WEIGHT = 0.7
COVERAGE_WEIGHT = 0.3


@dataclass
class LeaderboardEntry:
    """A single benchmark result for one paper implementation."""

    arxiv_id: str
    title: str = ""
    passed: int = 0
    failed: int = 0
    skipped: int = 0
    duration_seconds: float = 0.0
    pass_rate: float = 0.0        # passed / (passed + failed)
    coverage_ratio: float = 0.0    # from BenchmarkResult
    combined_score: float = 0.0   # weighted composite (raw)
    calibrated_score: float = 0.0  # combined_score × (1 - difficulty_penalty)
    difficulty_penalty: float = 0.0  # 0.0–0.5 based on stub rate
    stub_rate: float = 0.0        # skipped / total tests
    framework: str = "pytorch"
    capsule_id: str = ""
    last_updated: str = ""         # ISO timestamp
    numerical_claims_total: int = 0
    numerical_claims_covered: int = 0

    # Difficulty thresholds
    STUB_RATE_HIGH = 0.70   # >70% stubs → easy paper, big penalty
    STUB_RATE_MEDIUM = 0.40  # >40% stubs → moderate penalty
    PENALTY_HIGH = 0.40
    PENALTY_MEDIUM = 0.20
    PENALTY_LOW = 0.05

    def compute_score(self) -> float:
        """Compute both raw combined_score and calibrated_score with difficulty penalty."""
        total = self.passed + self.failed + self.skipped
        # stub_rate: how many tests are stubs (skipped)
        self.stub_rate = round(self.skipped / total, 4) if total > 0 else 0.0

        # Difficulty penalty based on stub rate
        if self.stub_rate >= self.STUB_RATE_HIGH:
            self.difficulty_penalty = self.PENALTY_HIGH
        elif self.stub_rate >= self.STUB_RATE_MEDIUM:
            self.difficulty_penalty = self.PENALTY_MEDIUM
        else:
            self.difficulty_penalty = self.PENALTY_LOW

        # Raw combined score
        pr = self.pass_rate
        cr = self.coverage_ratio
        self.combined_score = round(pr * PASS_RATE_WEIGHT + cr * COVERAGE_WEIGHT, 4)

        # Calibrated: penalize easy papers
        self.calibrated_score = round(self.combined_score * (1 - self.difficulty_penalty), 4)

        return self.calibrated_score

    def to_dict(self) -> dict:
        result = asdict(self)
        return result

    @classmethod
    def from_dict(cls, d: dict) -> "LeaderboardEntry":
        # Accept both old entries (without calibrated_score) and new
        known = cls.__dataclass_fields__
        return cls(**{k: v for k, v in d.items() if k in known})


class Leaderboard:
    """Collection of ranked benchmark entries."""

    def __init__(self):
        self.entries: Dict[str, LeaderboardEntry] = {}  # arxiv_id → entry
        self._load()

    def _load(self) -> None:
        if not LEADERBOARD_FILE.exists():
            return
        try:
            data = json.loads(LEADERBOARD_FILE.read_text(encoding="utf-8"))
            for d in data.get("entries", []):
                entry = LeaderboardEntry.from_dict(d)
                self.entries[entry.arxiv_id] = entry
        except Exception:
            pass

    def _save(self) -> None:
        GP_DIR.mkdir(parents=True, exist_ok=True)
        data = {
            "version": "1.0",
            "updated_at": _now_iso(),
            "entries": [e.to_dict() for e in self.entries.values()],
        }
        LEADERBOARD_FILE.write_text(json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")

    def upsert(self, entry: LeaderboardEntry) -> None:
        entry.last_updated = _now_iso()
        entry.compute_score()
        self.entries[entry.arxiv_id] = entry
        self._save()

    def get(self, arxiv_id: str) -> Optional[LeaderboardEntry]:
        return self.entries.get(arxiv_id)

    def rankings(self, limit: int = 20) -> List[LeaderboardEntry]:
        """Return entries sorted by calibrated_score (difficulty-adjusted) descending."""
        ranked = sorted(self.entries.values(), key=lambda e: e.calibrated_score, reverse=True)
        return ranked[:limit]

    def rankings_by_pass_rate(self, limit: int = 20) -> List[LeaderboardEntry]:
        ranked = sorted(self.entries.values(), key=lambda e: e.pass_rate, reverse=True)
        return ranked[:limit]

    def rankings_by_coverage(self, limit: int = 20) -> List[LeaderboardEntry]:
        ranked = sorted(self.entries.values(), key=lambda e: e.coverage_ratio, reverse=True)
        return ranked[:limit]

    def total_count(self) -> int:
        return len(self.entries)

    def avg_pass_rate(self) -> float:
        if not self.entries:
            return 0.0
        return round(sum(e.pass_rate for e in self.entries.values()) / len(self.entries), 3)

    def avg_coverage(self) -> float:
        with_cov = [e.coverage_ratio for e in self.entries.values() if e.coverage_ratio > 0]
        if not with_cov:
            return 0.0
        return round(sum(with_cov) / len(with_cov), 3)


# ─── Upsert from BenchmarkResult ─────────────────────────────────────────────────


def upsert_from_benchmark(
    arxiv_id: str,
    benchmark_result: Any,
    paper_title: str = "",
    framework: str = "pytorch",
    capsule_id: str = "",
) -> LeaderboardEntry:
    """Create or update a leaderboard entry from a BenchmarkResult object."""
    total = benchmark_result.passed + benchmark_result.failed
    pass_rate = benchmark_result.passed / total if total > 0 else 0.0

    entry = LeaderboardEntry(
        arxiv_id=arxiv_id,
        title=paper_title[:100] if paper_title else arxiv_id,
        passed=benchmark_result.passed,
        failed=benchmark_result.failed,
        skipped=benchmark_result.skipped,
        duration_seconds=benchmark_result.duration_seconds,
        pass_rate=round(pass_rate, 4),
        coverage_ratio=round(getattr(benchmark_result, "coverage_ratio", 0.0), 4),
        framework=framework,
        capsule_id=capsule_id,
        numerical_claims_total=getattr(benchmark_result, "numerical_claims_total", 0),
        numerical_claims_covered=getattr(benchmark_result, "numerical_claims_covered", 0),
    )
    entry.compute_score()

    board = Leaderboard()
    board.upsert(entry)
    return entry


# ─── HTML rendering ─────────────────────────────────────────────────────────────


def render_leaderboard_html(
    sort_by: str = "combined",
    limit: int = 20,
) -> str:
    """Render the leaderboard as an HTML table."""
    board = Leaderboard()

    if sort_by == "pass_rate":
        entries = board.rankings_by_pass_rate(limit)
    elif sort_by == "coverage":
        entries = board.rankings_by_coverage(limit)
    else:
        entries = board.rankings(limit)

    board_json = json.dumps([e.to_dict() for e in entries], ensure_ascii=False)

    avg_pr = board.avg_pass_rate()
    avg_cov = board.avg_coverage()

    rows_html = ""
    for rank, e in enumerate(entries, 1):
        pr_pct = f"{e.pass_rate * 100:.1f}%"
        cov_pct = f"{e.coverage_ratio * 100:.1f}%"
        score_color = "#3fb950" if e.combined_score > 0.7 else "#f0883e" if e.combined_score > 0.4 else "#8b949e"
        rows_html += f"""<tr>
          <td style="text-align:center;color:#8b949e">{rank}</td>
          <td><a href="https://arxiv.org/{e.arxiv_id}" target="_blank" style="color:#58a6ff">{e.arxiv_id}</a></td>
          <td style="color:#e6edf3">{e.title[:50]}</td>
          <td style="text-align:center;color:#3fb950">{e.passed}</td>
          <td style="text-align:center;color:#f85149">{e.failed}</td>
          <td style="text-align:center;color:#8b949e">{e.skipped}</td>
          <td style="text-align:center">{pr_pct}</td>
          <td style="text-align:center">{cov_pct}</td>
          <td style="text-align:center;font-weight:bold;color:{score_color}">{e.combined_score:.3f}</td>
          <td style="text-align:center;color:#8b949e">{e.framework}</td>
        </tr>"""

    html = f"""<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>Paper2code Leaderboard</title>
<style>
  body {{ font-family: -apple-system, BlinkMacSystemFont, sans-serif; background: #0d1117; color: #e6edf3; margin: 20px; }}
  h2 {{ color: #58a6ff; }}
  table {{ border-collapse: collapse; width: 100%; max-width: 900px; }}
  th {{ background: #161b22; color: #8b949e; padding: 8px 12px; text-align:left; font-size: 12px; text-transform: uppercase; }}
  td {{ padding: 8px 12px; border-bottom: 1px solid #21262d; font-size: 13px; }}
  tr:hover {{ background: #161b22; }}
  .sort-links {{ margin-bottom: 16px; }}
  .sort-links a {{ color: #58a6ff; margin-right: 16px; text-decoration: none; }}
  .sort-links a.active {{ color: #3fb950; font-weight: bold; }}
  .summary {{ color: #8b949e; font-size: 13px; margin-bottom: 16px; }}
</style>
</head>
<body>
<h2>📊 Paper2code Benchmark Leaderboard</h2>
<div class="summary">
  {board.total_count()} implementations · avg pass rate: {avg_pr*100:.1f}% · avg coverage: {avg_cov*100:.1f}%
</div>
<div class="sort-links">
  <a href="?sort=combined" class="{'active' if sort_by=='combined' else ''}">Combined Score</a>
  <a href="?sort=pass_rate" class="{'active' if sort_by=='pass_rate' else ''}">Pass Rate</a>
  <a href="?sort=coverage" class="{'active' if sort_by=='coverage' else ''}">Coverage</a>
</div>
<table>
<thead>
<tr>
  <th>#</th><th>arXiv ID</th><th>Title</th>
  <th style="text-align:center">✓ Pass</th>
  <th style="text-align:center">✗ Fail</th>
  <th style="text-align:center">⊘ Skip</th>
  <th style="text-align:center">Pass Rate</th>
  <th style="text-align:center">Coverage</th>
  <th style="text-align:center">Score</th>
  <th style="text-align:center">Framework</th>
</tr>
</thead>
<tbody>
{rows_html}
</tbody>
</table>
</body>
</html>"""
    return html


# ─── Utility ───────────────────────────────────────────────────────────────────


def _now_iso() -> str:
    return datetime.utcnow().isoformat()


# ─── MCP tool actions ───────────────────────────────────────────────────────────


def leaderboard_action(
    action: str = "status",
    arxiv_id: Optional[str] = None,
    sort_by: str = "combined",
    limit: int = 20,
) -> dict:
    """MCP tool dispatcher for Benchmark Leaderboard.

    Actions:
      status     — summary stats (count, avg scores)
      rankings   — top N entries by combined/pass_rate/coverage
      upsert     — add/update a single entry (manual)
      render     — full HTML leaderboard
      entry      — get a single entry by arxiv_id
    """
    board = Leaderboard()

    if action == "status":
        return {
            "total_implementations": board.total_count(),
            "avg_pass_rate": board.avg_pass_rate(),
            "avg_coverage_ratio": board.avg_coverage(),
            "file": str(LEADERBOARD_FILE),
        }

    elif action == "rankings":
        entries = board.rankings(limit) if sort_by == "combined" else (
            board.rankings_by_pass_rate(limit) if sort_by == "pass_rate"
            else board.rankings_by_coverage(limit)
        )
        return {
            "rankings": [
                {
                    "rank": idx + 1,
                    "arxiv_id": e.arxiv_id,
                    "title": e.title,
                    "passed": e.passed,
                    "failed": e.failed,
                    "skipped": e.skipped,
                    "pass_rate": e.pass_rate,
                    "coverage_ratio": e.coverage_ratio,
                    "combined_score": e.combined_score,
                    "calibrated_score": e.calibrated_score,
                    "difficulty_penalty": e.difficulty_penalty,
                    "stub_rate": e.stub_rate,
                    "framework": e.framework,
                    "last_updated": e.last_updated,
                }
                for idx, e in enumerate(entries)
            ],
            "total": board.total_count(),
            "sort_by": sort_by,
            "note": "rankings sorted by calibrated_score (difficulty-adjusted)",
        }

    elif action == "upsert":
        if not arxiv_id:
            return {"error": "arxiv_id required for upsert"}
        # Return current entry state for confirmation
        existing = board.get(arxiv_id)
        return {
            "arxiv_id": arxiv_id,
            "existing": existing.to_dict() if existing else None,
            "message": "Use upsert_from_benchmark() after run_benchmark() to auto-populate",
        }

    elif action == "render":
        html = render_leaderboard_html(sort_by=sort_by, limit=limit)
        return {"html": html, "size_kb": round(len(html) / 1024, 1)}

    elif action == "entry":
        if not arxiv_id:
            return {"error": "arxiv_id required for entry"}
        e = board.get(arxiv_id)
        if e:
            return e.to_dict()
        return {"error": f"No entry found for {arxiv_id}"}

    else:
        return {"error": f"Unknown action: {action}"}
