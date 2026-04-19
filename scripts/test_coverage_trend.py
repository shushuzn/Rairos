#!/usr/bin/env python3
"""Track test coverage over time. Run after each coverage report."""
import subprocess
import json
import os
from pathlib import Path
from datetime import datetime

HISTORY_FILE = Path(__file__).parent.parent / "docs" / "coverage_history.md"


def run_coverage():
    result = subprocess.run(
        ["coverage", "json", "-o", ".coverage_history.json"],
        capture_output=True, text=True
    )
    if result.returncode != 0:
        print(f"Coverage failed: {result.stderr}")
        return None

    with open(".coverage_history.json") as f:
        data = json.load(f)

    totals = data["totals"]
    return {
        "date": datetime.now().strftime("%Y-%m-%d"),
        "total_percent": round(totals["percent_covered"] or 0, 2),
        "covered": totals["covered_lines"],
        "missed": totals["missing_lines"],
        "total": totals["num_statements"],
    }


def update_markdown(history):
    HISTORY_FILE.parent.mkdir(parents=True, exist_ok=True)

    new_entry = (
        f"| {history['date']} | {history['total_percent']}% "
        f"| {history['covered']} | {history['missed']} | {history['total']} |\n"
    )

    if HISTORY_FILE.exists():
        content = HISTORY_FILE.read_text(encoding="utf-8")
        # Replace the last data row or append
        lines = content.splitlines()
        for i, line in enumerate(lines):
            if line.startswith("| 20") and "---|---|" not in line:
                lines[i] = new_entry.strip()
                new_content = "\n".join(lines)
                break
        else:
            new_content = content.rstrip() + "\n" + new_entry
    else:
        new_content = (
            "# Coverage History\n\n"
            "| Date | Coverage | Covered | Missed | Total |\n"
            "|------|----------|---------|--------|-------|\n"
            + new_entry
        )

    HISTORY_FILE.write_text(new_content, encoding="utf-8")
    print(f"Updated {HISTORY_FILE}")
    print(
        f"  {history['date']}: {history['total_percent']}% "
        f"({history['covered']}/{history['total']})"
    )


if __name__ == "__main__":
    history = run_coverage()
    if history:
        update_markdown(history)
