"""CLI command: daemon — autonomous research autopilot.

Usage:
    airos daemon start        Start background research watch + evolution
    airos daemon stop         Stop background watch
    airos daemon status       Current daemon status
    airos daemon run-cycle    Run one orchestrator cycle (subscriptions + gaps)
    airos daemon evolve       Run one evolution cycle on Gene Pool
    airos daemon log          Recent alerts and activity
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path

from cli._shared import Colors, colored, print_error, print_success


def _build_daemon_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "daemon",
        help="Autonomous research autopilot (watch + evolve)",
        prog="airos daemon",
        description="Start/stop the autonomous research daemon that watches arXiv, "
        "runs gap detection, and evolves the Gene Pool automatically.",
    )

    sub = p.add_subparsers(dest="daemon_command", metavar=" COMMAND")

    # start
    start = sub.add_parser("start", help="Start background research watch + evolution")
    start.add_argument(
        "--interval",
        "-i",
        type=int,
        default=30,
        help="Poll interval in minutes (default 30)",
    )
    start.add_argument(
        "--no-webhook",
        action="store_true",
        help="Disable Discord/Feishu webhook notifications",
    )
    start.set_defaults(func=_run_daemon_start)

    # stop
    stop = sub.add_parser("stop", help="Stop background watch")
    stop.set_defaults(func=_run_daemon_stop)

    # status
    status = sub.add_parser("status", help="Show daemon status")
    status.set_defaults(func=_run_daemon_status)

    # run-cycle
    cycle = sub.add_parser("run-cycle", help="Run one orchestrator cycle")
    cycle.set_defaults(func=_run_daemon_cycle)

    # evolve
    evolve = sub.add_parser("evolve", help="Run one evolution cycle on Gene Pool")
    evolve.add_argument(
        "--topic",
        type=str,
        default="",
        help="Topic to focus evolution on (default: auto-detect from history)",
    )
    evolve.set_defaults(func=_run_daemon_evolve)

    # log
    log = sub.add_parser("log", help="Show recent alerts and activity")
    log.add_argument("--limit", type=int, default=20, help="Max entries to show")
    log.set_defaults(func=_run_daemon_log)

    return p


def _get_orchestrator():
    from research_loop.orchestrator import AutonomousOrchestrator

    return AutonomousOrchestrator(webhook_enabled=True)


def _run_daemon_start(args) -> None:
    orch = _get_orchestrator()
    webhook = not getattr(args, "no_webhook", False)
    if webhook is False:
        orch.webhook_enabled = False

    status = orch.get_status()
    if status.get("running"):
        print_error("Daemon is already running.")
        sys.exit(1)

    interval = getattr(args, "interval", 30)
    orch.start_watch(interval_minutes=interval)
    print_success(f"Daemon started (interval={interval}min, webhook={webhook})")
    print("  Use 'airos daemon status' to check, 'airos daemon stop' to stop.")


def _run_daemon_stop(args) -> None:
    orch = _get_orchestrator()
    orch.stop_watch()
    print_success("Daemon stopped.")


def _run_daemon_status(args) -> None:
    orch = _get_orchestrator()
    status = orch.get_status()

    running = status.get("running", False)
    running_str = f"{Colors.GREEN}RUNNING{Colors.END}" if running else f"{Colors.RED}STOPPED{Colors.END}"

    print(f"\n  {'Daemon Status':<20} {running_str}")
    print(f"  {'Interval':<20} {status.get('interval_minutes', 30)}min")
    print(f"  {'Last Check':<20} {status.get('last_check', 'never')}")
    print(f"  {'Alerts Stored':<20} {status.get('alerts_count', 0)}")

    gp = status.get("gene_pool", {})
    print(f"  {'Gene Pool':<20} {gp.get('total_capsules', 0)} capsules, avg {gp.get('avg_score', 0):.3f}")

    by_type = gp.get("by_gap_type", {})
    if by_type:
        print(f"  {'Gap Types':<20} {', '.join(f'{k}={v}' for k, v in by_type.items())}")
    print()


def _run_daemon_cycle(args) -> None:
    print("Running orchestrator cycle...")
    orch = _get_orchestrator()
    try:
        alerts = orch.run_cycle()
        print_success(f"Cycle complete: {len(alerts)} alert(s) generated")
        for alert in alerts[:5]:
            sev = alert.severity
            color = Colors.RED if sev == "HIGH" else Colors.YELLOW
            print(f"  {color}[{sev}]{Colors.END} {alert.top_gap_title[:60]}")
        if len(alerts) > 5:
            print(f"  ... and {len(alerts) - 5} more")
    except Exception as e:
        print_error(f"Cycle failed: {e}")
        sys.exit(1)


def _run_daemon_evolve(args) -> None:
    from research_loop.orchestrator import AutonomousOrchestrator

    orch = AutonomousOrchestrator(webhook_enabled=False)
    orch._init_components()
    topic = getattr(args, "topic", "") or ""

    print(f"Running evolution cycle (topic={topic or 'auto'}...)")
    result = orch.run_evolution_cycle(topic=topic)
    if "error" in result:
        print_error(f"Evolution failed: {result['error']}")
        sys.exit(1)

    print_success("Evolution cycle complete:")
    print(f"  Audit:    {result.get('audit', {}).get('total', 0)} capsules, "
          f"avg quality {result.get('audit', {}).get('avg_quality', 0):.3f}")
    print(f"  Proposed: {result.get('proposed', 0)} candidates")
    print(f"  Added:    {result.get('result', {}).get('added', 0)} new capsules")
    print(f"  Retired:  {result.get('result', {}).get('retired', 0)} capsules")
    print(f"  Pool:     {result.get('result', {}).get('total_capsules', 0)} total")

    # Show credibility report
    from llm.insight.evolution import InsightEvolution
    from llm.insight.tracker import EvolutionTracker

    tracker = EvolutionTracker()
    evolver = InsightEvolution(tracker=tracker)
    print(evolver.credibility_report())


def _run_daemon_log(args) -> None:
    orch = _get_orchestrator()
    limit = getattr(args, "limit", 20)
    alerts = orch.get_recent_alerts(limit=limit)

    if not alerts:
        print("No alerts yet. Run 'airos daemon run-cycle' to start.")
        return

    print(f"\n  Recent Alerts ({len(alerts)}):\n")
    for a in alerts:
        sev = a.severity
        color = Colors.RED if sev == "HIGH" else Colors.YELLOW if sev == "MEDIUM" else Colors.CYAN
        ts = time.strftime("%H:%M", time.localtime(a.created_at)) if a.created_at else "?"
        gp = getattr(a, "gene_pool_score", 0)
        boost = " ✅" if getattr(a, "preference_boost", False) else ""
        print(f"  {ts} {color}[{sev}]{Colors.END} {a.top_gap_title[:55]:<55} "
              f"gp={gp:.2f}{boost}")
    print()
