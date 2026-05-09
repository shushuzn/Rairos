"""CLI command: daemon — autonomous research autopilot + event watch.

Usage:
    airos daemon start              Start background research watch + evolution
    airos daemon stop               Stop background watch
    airos daemon status             Current daemon status
    airos daemon run-cycle          Run one orchestrator cycle
    airos daemon evolve             Run one evolution cycle on Gene Pool
    airos daemon log                Recent alerts and activity
    airos daemon watch              Start continuous event monitoring (Jin10 + auto-capsule)
    airos daemon watch-stop         Stop event monitoring
"""

from __future__ import annotations

import argparse
import sys
import time

from cli._shared import Colors, print_error, print_success


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
        "--topic", type=str, default="", help="Topic to focus evolution on (default: auto-detect)"
    )
    evolve.set_defaults(func=_run_daemon_evolve)

    # log
    log = sub.add_parser("log", help="Show recent alerts and activity")
    log.add_argument("--limit", type=int, default=20, help="Max entries to show")
    log.set_defaults(func=_run_daemon_log)

    # watch (event monitoring)
    watch = sub.add_parser("watch", help="Start continuous event monitoring (Jin10)")
    watch.add_argument(
        "--interval", type=int, default=300, help="Check interval in seconds (default: 300)"
    )
    watch.set_defaults(func=_run_daemon_watch)

    watch_stop = sub.add_parser("watch-stop", help="Stop event monitoring")
    watch_stop.set_defaults(func=_run_daemon_watch_stop)

    watch_status = sub.add_parser("watch-status", help="Event monitor status")
    watch_status.set_defaults(func=_run_daemon_watch_status)

    # sse — stream daemon events via SSE to stdout
    sse = sub.add_parser("sse", help="Stream daemon events via SSE to stdout")
    sse.add_argument("--port", "-p", type=int, default=8765, help="SSE port (default 8765)")
    sse.set_defaults(func=_run_daemon_sse)

    # events — list recent events from EventBus
    events = sub.add_parser("events", help="List recent events from the EventBus")
    events.add_argument(
        "--type",
        "-t",
        type=str,
        default=None,
        help="Filter by event type (e.g. alert_found, cycle_complete)",
    )
    events.add_argument("--limit", "-n", type=int, default=20, help="Max events to show")
    events.set_defaults(func=_run_daemon_events)

    return p  # type: ignore[no-any-return]


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
    running_str = (
        f"{Colors.GREEN}RUNNING{Colors.END}" if running else f"{Colors.FAIL}STOPPED{Colors.END}"
    )

    print(f"\n  {'Daemon Status':<20} {running_str}")
    print(f"  {'Interval':<20} {status.get('interval_minutes', 30)}min")
    print(f"  {'Last Check':<20} {status.get('last_check', 'never')}")
    print(f"  {'Alerts Stored':<20} {status.get('alerts_count', 0)}")

    gp = status.get("gene_pool", {})
    print(
        f"  {'Gene Pool':<20} {gp.get('total_capsules', 0)} capsules, avg {gp.get('avg_score', 0):.3f}"
    )

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
            color = Colors.FAIL if sev == "HIGH" else Colors.YELLOW
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
    print(
        f"  Audit:    {result.get('audit', {}).get('total', 0)} capsules, "
        f"avg quality {result.get('audit', {}).get('avg_quality', 0):.3f}"
    )
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
        color = Colors.FAIL if sev == "HIGH" else Colors.YELLOW if sev == "MEDIUM" else Colors.CYAN
        ts = time.strftime("%H:%M", time.localtime(a.created_at)) if a.created_at else "?"
        gp = getattr(a, "gene_pool_score", 0)
        boost = " ✅" if getattr(a, "preference_boost", False) else ""
        print(f"  {ts} {color}[{sev}]{Colors.END} {a.top_gap_title[:55]:<55} gp={gp:.2f}{boost}")
    print()


def _run_daemon_watch(args) -> None:
    from llm.watch import WatchDaemon

    interval = getattr(args, "interval", 300)
    daemon = WatchDaemon(interval=interval)
    daemon.start()
    print_success(f"Event watch started (interval={interval}s)")
    print("  Monitoring Jin10 for geopolitical + financial events")
    print("  Auto-encoding high-impact events as Gene Pool capsules")
    print("  Use 'airos daemon watch-status' to check")


def _run_daemon_watch_stop(args) -> None:
    from llm.watch import WatchDaemon

    d = WatchDaemon()
    d.stop()
    print_success("Event watch stopped.")


def _run_daemon_watch_status(args) -> None:
    from llm.watch import WatchDaemon

    d = WatchDaemon()
    status = d.get_status()
    print("\n  Event Watch Status:")
    print(f"    Running:     {'Yes' if status['running'] else 'No'}")
    print(f"    Interval:    {status['interval']}s")
    print(f"    Last check:  {status.get('last_check', 'never')[:19]}")
    print(f"    Events:      {status['total_events']} total")
    print(f"    Gene Pool:   {status['gene_pool_size']} capsules")
    print()


def _run_daemon_sse(args) -> None:
    """Stream daemon events to stdout via SSE."""
    import asyncio
    import threading
    from research_loop.daemon import EventBus, SSEServer

    port = getattr(args, "port", 8765)

    # Start SSE server in a background thread
    server = SSEServer(port=port)

    def run_server():
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        loop.run_until_complete(server._serve())

    server_thread = threading.Thread(target=run_server, name="sse-server", daemon=True)
    server_thread.start()

    # Give server a moment to start
    print_success(f"SSE server running on http://localhost:{port}/events")
    print("  Streaming events to stdout (Ctrl-C to stop)...")

    # Subscribe and print events to stdout
    eb = EventBus()

    def print_event(event) -> None:
        import json as _json

        d = event.to_dict()
        d["_printed_at"] = time.strftime("%H:%M:%S")
        print(
            f"[{d['_printed_at']}] {event.event_type}: {_json.dumps(d['data'], ensure_ascii=False)[:120]}"
        )

    eb.subscribe("*", print_event)

    try:
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        eb.unsubscribe("*", print_event)
        print("\nSSE stream stopped.")


def _run_daemon_events(args) -> None:
    """List recent events from the EventBus."""
    from research_loop.daemon import EventBus

    eb = EventBus()
    ftype = getattr(args, "type", None)
    limit = getattr(args, "limit", 20)

    events = eb.get_history(event_type=ftype, limit=limit)

    if not events:
        print("No events found." + (f" (type={ftype})" if ftype else ""))
        return

    print(f"\n  Recent Events ({len(events)}, type={ftype or 'all'}):\n")
    for ev in events:
        ts = time.strftime("%H:%M:%S", time.localtime(ev.timestamp))
        import json as _json

        data_str = _json.dumps(ev.data, ensure_ascii=False)[:80]
        print(f"  {ts}  [{ev.event_type}]  {data_str}")
    print()
