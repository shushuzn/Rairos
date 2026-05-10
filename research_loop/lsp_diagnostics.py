"""LSP-style diagnostics — progressive ruff (fast) + pyright (background).

Integrates into the paper2code pipeline to surface code quality issues
before pytest runs, inspired by DeepSeek-TUI's LSP diagnostics integration.
"""

from __future__ import annotations

import json
import shutil
import subprocess
import threading
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, List, Optional


@dataclass
class Diagnostic:
    """A single code diagnostic (lint or type error)."""

    file: Path
    line: int
    column: int
    severity: str  # "error" | "warning" | "information"
    code: str  # e.g. "E501", "PTH", " reportUnusedImport"
    message: str

    def __str__(self) -> str:
        loc = f"{self.file}:{self.line}:{self.column}"
        return f"  [{self.severity.upper()}] {loc} {self.code}: {self.message}"


# ─── Ruff (fast synchronous lint) ──────────────────────────────────────────────


def check_ruff(code_path: Path) -> List[Diagnostic]:
    """Run ruff linter synchronously. Returns immediately — typically < 1s."""
    ruff = shutil.which("ruff")
    if not ruff:
        return []
    try:
        result = subprocess.run(
            [ruff, "check", str(code_path), "--output-format=json"],
            capture_output=True,
            text=True,
            timeout=30,
            encoding="utf-8",
            errors="replace",
        )
        return _parse_ruff_json(code_path, result.stdout)
    except Exception:
        return []


def _parse_ruff_json(code_path: Path, stdout: str) -> List[Diagnostic]:
    if not stdout.strip():
        return []
    diagnostics = []
    try:
        for entry in json.loads(stdout):
            location = entry.get("location", {})
            diagnostics.append(
                Diagnostic(
                    file=code_path,
                    line=location.get("line", 1),
                    column=location.get("column", 1),
                    severity=_ruff_to_severity(entry.get("rule")),
                    code=entry.get("rule", entry.get("code", "")),
                    message=entry.get("message", ""),
                )
            )
    except Exception:
        pass
    return diagnostics


def _ruff_to_severity(rule: str) -> str:
    if not rule:
        return "information"
    # Ruff error codes: E=F1xx, W=F2xx, I=F4xx, UP=F5xx, PTH=F6xx
    prefix = rule[0] if rule else ""
    if prefix in ("E", "F", "F8"):
        return "error"
    if prefix in ("W", "F4", "F5", "F6", "F7"):
        return "warning"
    return "information"


# ─── Pyright (slow async type check) ─────────────────────────────────────────


def check_pyright(code_path: Path) -> List[Diagnostic]:
    """Run pyright type checker. May take 10-60s depending on codebase size."""
    pyright = shutil.which("pyright") or shutil.which("pyright.exe")
    if not pyright:
        return []
    try:
        result = subprocess.run(
            [pyright, str(code_path), "--outputjson"],
            capture_output=True,
            text=True,
            timeout=120,
            encoding="utf-8",
            errors="replace",
        )
        return _parse_pyright_json(code_path, result.stdout)
    except Exception:
        return []


def _parse_pyright_json(code_path: Path, stdout: str) -> List[Diagnostic]:
    if not stdout.strip():
        return []
    diagnostics = []
    try:
        report = json.loads(stdout)
        general_diagnostics = report.get("generalDiagnostics", [])
        for diag in general_diagnostics:
            severity = diag.get("severity", "information")
            range_info = diag.get("range", {})
            start = range_info.get("start", {})
            diagnostics.append(
                Diagnostic(
                    file=code_path,
                    line=start.get("line", 1),
                    column=start.get("character", 1),
                    severity=severity,
                    code="pyright",
                    message=diag.get("message", ""),
                )
            )
    except Exception:
        pass
    return diagnostics


# ─── Progressive runner ────────────────────────────────────────────────────────


def run_progressive(
    code_path: Path,
    on_fast: Optional[Callable[[List[Diagnostic]], None]] = None,
    on_complete: Optional[Callable[[List[Diagnostic]], None]] = None,
) -> None:
    """Run diagnostics progressively: ruff first (fast), then pyright (background).

    Args:
        code_path: Path to Python file to check
        on_fast: Called immediately with ruff results (synchronous)
        on_complete: Called after pyright completes (in background thread)
    """
    # Fast path: ruff synchronously
    ruff_results = check_ruff(code_path)
    if on_fast:
        on_fast(ruff_results)

    # Slow path: pyright in background thread
    def background():
        pyright_results = check_pyright(code_path)
        if on_complete:
            on_complete(pyright_results)

    t = threading.Thread(target=background)
    t.start()


def format_diagnostics(diagnostics: List[Diagnostic], header: str = "diagnostics") -> str:
    """Format diagnostics for terminal display."""
    if not diagnostics:
        return ""
    lines = [f"\n{header.upper()} ({len(diagnostics)} issues):"]
    for d in diagnostics:
        lines.append(str(d))
    return "\n".join(lines)
