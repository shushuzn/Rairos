"""doctor command — diagnose environment and report issues."""

from __future__ import annotations

import argparse
import os
import shutil
import sqlite3
import sys
import typing
from pathlib import Path


def _banner(msg: str) -> str:
    return f"[bold cyan]✓[/] {msg}"


def _warn(msg: str) -> str:
    return f"[yellow]⚠[/] {msg}"


def _fail(msg: str) -> str:
    return f"[red]✗[/] {msg}"


def _section(title: str) -> str:
    return f"\n[bold]{title}[/]"


def _run_doctor(args) -> int:
    """Run environment diagnostics."""
    from rich.console import Console
    from rich.table import Table

    console = Console()
    issues: list[str] = []
    warnings: list[str] = []
    ok: list[str] = []

    # ── Python ──────────────────────────────────────────────────────────────
    console.print(_section("Python"))
    ver = sys.version_info
    python_ok = ver >= (3, 10)
    if python_ok:
        ok.append(f"Python {ver.major}.{ver.minor}.{ver.micro}")
    else:
        issues.append(f"Python {ver.major}.{ver.minor} — requires 3.10+")

    # ── Platform ────────────────────────────────────────────────────────────
    console.print(_section("Platform"))
    ok.append(f"{sys.platform} | {os.name}")

    # ── Required executables ────────────────────────────────────────────────
    console.print(_section("Executables"))
    for exe, name in [("uv", "uv"), ("git", "Git"), ("python", "Python")]:
        path = shutil.which(exe)
        if path:
            ok.append(f"{name}: {path}")
        else:
            issues.append(f"{name}: not found in PATH")

    # ── Directories ─────────────────────────────────────────────────────────
    console.print(_section("Directories"))
    _dirs: typing.List[typing.Tuple[str, Path]] = [("HOME", Path.home()), ("CWD", Path.cwd())]
    for name, path in _dirs:
        if path.exists():
            ok.append(f"{name}: {path}")
        else:
            issues.append(f"{name}: does not exist ({path})")

    # ── Database ────────────────────────────────────────────────────────────
    console.print(_section("Database"))
    db_paths: list[Path] = [
        Path(os.environ.get("AIROS_DB", "") or ""),
        Path.home() / ".ai_research_os" / "research.db",
        Path("~/.ai_research_os/research.db").expanduser(),
    ]
    found_db = None
    for dp in db_paths:
        if dp.exists():
            found_db = dp
            break
    if found_db:
        ok.append(f"Database: {found_db}")
        try:
            conn = sqlite3.connect(str(found_db))
            cur = conn.execute("SELECT COUNT(*) FROM papers")
            count = cur.fetchone()[0]
            conn.close()
            ok.append(f"  papers: {count}")
        except Exception as e:
            warnings.append(f"Database readable but query failed: {e}")
    else:
        warnings.append("No database found (run 'rairos init' or ingest first)")

    # ── Config files ────────────────────────────────────────────────────────
    console.print(_section("Config Files"))
    for name in [".env", ".env.local", ".env.example"]:
        p = Path(name)
        if p.exists():
            ok.append(f"{name}: exists")
        else:
            warnings.append(f"{name}: not found")

    # ── Pre-commit ──────────────────────────────────────────────────────────
    console.print(_section("Git Hooks"))
    hook_dir = Path(".git/hooks")
    if hook_dir.exists():
        precommit = hook_dir / "pre-commit"
        if precommit.exists() and precommit.stat().st_size > 0:
            ok.append("pre-commit hook: installed")
        else:
            warnings.append("pre-commit hook: not installed (run: pre-commit install)")
    else:
        issues.append("Not a git repository")

    # ── Dependencies ─────────────────────────────────────────────────────────
    console.print(_section("Key Dependencies"))
    # Executables (via shutil.which — more reliable than import for CLI tools)
    for exe in ["pytest", "ruff", "mypy", "git", "uv"]:
        if shutil.which(exe):
            ok.append(f"{exe}: installed")
        else:
            issues.append(f"{exe}: not found in PATH")

    # ── API Keys (check .env) ───────────────────────────────────────────────
    console.print(_section("Environment Variables"))
    env_checks = [
        ("OPENAI_API_KEY", "OpenAI", False),
        ("ANTHROPIC_API_KEY", "Anthropic", False),
        ("GITHUB_TOKEN", "GitHub", False),
        ("ZILLIZ_URI", "Zilliz", False),
        ("ZILLIZ_TOKEN", "Zilliz token", False),
    ]
    for var, name, required in env_checks:
        val = os.environ.get(var, "")
        if val:
            masked = val[:4] + "****" if len(val) > 4 else "****"
            ok.append(f"{name} ({var}): {masked}")
        elif required:
            issues.append(f"{name} ({var}): not set (required)")
        else:
            warnings.append(f"{name} ({var}): not set")

    # ── Rairos version ──────────────────────────────────────────────────────
    console.print(_section("Rairos"))
    try:
        from core import __version__  # type: ignore[attr-defined]

        ok.append(f"Version: {__version__}")
    except ImportError:
        ok.append("Version: unknown (core/__init__.py missing __version__)")

    # ── Summary ─────────────────────────────────────────────────────────────
    console.print(_section("Summary"))
    table = Table(show_header=False, box=None)
    table.add_column("Status", style="bold", width=4)
    table.add_column("Count")
    table.add_row("[green]OK[/]", str(len(ok)))
    table.add_row("[yellow]Warnings[/]", str(len(warnings)))
    table.add_row("[red]Issues[/]", str(len(issues)))
    console.print(table)

    if issues:
        console.print(f"\n[red bold]Issues found ({len(issues)}):[/]")
        for issue in issues:
            console.print("  " + _fail(issue))
    if warnings:
        console.print(f"\n[yellow]Warnings ({len(warnings)}):[/]")
        for w in warnings:
            console.print("  " + _warn(w))
    if not issues:
        console.print("\n[green]All checks passed.[/]")

    return 1 if issues else 0


def _build_doctor_parser(subparsers: argparse._SubParsersAction) -> argparse.ArgumentParser:
    """Build doctor subcommand parser."""
    p = subparsers.add_parser("doctor", help="Diagnose environment and report issues")
    p.set_defaults(func=_run_doctor)
    return p  # type: ignore[no-any-return]
