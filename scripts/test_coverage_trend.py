#!/usr/bin/env python3
"""Track test coverage over time. Run after each coverage report."""
import subprocess
import json
import os
from pathlib import Path
from datetime import datetime

HISTORY_FILE = Path(__file__).parent.parent / "docs" / "coverage_history.md"
REPO_ROOT = Path(__file__).parent.parent


def run_coverage_json():
    result = subprocess.run(
        ["coverage", "json", "-q"],
        capture_output=True,
        text=True,
        env={**os.environ, "PYTHONPATH": str(REPO_ROOT)},
        cwd=str(REPO_ROOT),
    )
    if result.returncode != 0:
        raise RuntimeError(f"coverage json failed: {result.stderr}")
    with open(REPO_ROOT / "coverage.json") as f:
        return json.load(f)


def format_entry(totals, timestamp):
    cov = totals.get("percent_covered", 0)
    date_str = datetime.fromtimestamp(timestamp).strftime("%Y-%m-%d %H:%M")
    bar = "=" * int(cov / 5)
    return f"## {date_str}  |  {cov:.2f}%  |  {bar}"


def main():
    data = run_coverage_json()
    totals = data["totals"]
    timestamp = data.get("time", datetime.now().timestamp())

    entry = format_entry(totals, timestamp)

    history_lines = []
    if HISTORY_FILE.exists():
        content = HISTORY_FILE.read_text()
        history_lines = content.strip().split("\n")
        if history_lines and history_lines[0].startswith("# Test Coverage History"):
            history_lines = history_lines[1:]

    header = "# Test Coverage History\n"
    new_content = header + entry + "\n" + "\n".join(history_lines) + "\n"
    HISTORY_FILE.write_text(new_content)

    cov = totals.get("percent_covered", 0)
    print(f"Updated {HISTORY_FILE} — {cov:.2f}%")


if __name__ == "__main__":
    main()
