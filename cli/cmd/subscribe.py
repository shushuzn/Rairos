"""CLI command: subscribe — Smart arXiv subscription management with autopilot watch."""

from __future__ import annotations

import argparse
import json
import logging
import sys
import threading
import time
from pathlib import Path

from cli._shared import get_db, print_info, print_error, print_success

logger = logging.getLogger(__name__)

# ─── Persistent watch state ────────────────────────────────────────────────────


def _get_watch_state_path() -> Path:
    state_dir = Path.home() / ".ai_research_os" / "subscriptions"
    state_dir.mkdir(parents=True, exist_ok=True)
    return state_dir / "watch_state.json"


def _load_watch_state() -> dict:
    path = _get_watch_state_path()
    if path.exists():
        try:
            return json.loads(path.read_text(encoding="utf-8"))
        except Exception:
            pass
    return {"running": False, "interval_minutes": 60, "last_check": ""}


def _save_watch_state(state: dict) -> None:
    path = _get_watch_state_path()
    path.write_text(json.dumps(state, indent=2, ensure_ascii=False), encoding="utf-8")


# ─── Watch loop ────────────────────────────────────────────────────────────────


def _run_watch_loop(interval_minutes: int, stop_event: threading.Event) -> None:
    """Background watch loop. Runs until stop_event is set."""
    from core.notifications import get_webhook_notifier
    from core.notifications import configure_webhook
    from llm.subscription_monitor import SubscriptionMonitor
    from llm.subscription_scorer import SubscriptionScorer

    db = get_db()
    db.init()
    monitor = SubscriptionMonitor(db, SubscriptionScorer(db))
    webhook = get_webhook_notifier()

    print_success(f"[Autopilot] Watch started — checking every {interval_minutes} min")
    state = _load_watch_state()
    state["running"] = True
    state["interval_minutes"] = interval_minutes
    _save_watch_state(state)

    while not stop_event.is_set():
        try:
            all_results = monitor.check_all()
            total = sum(len(papers) for papers in all_results.values())

            if total > 0:
                print_info(
                    f"[Autopilot] Found {total} new paper(s) across {len(all_results)} subscription(s)"
                )

                # Send webhook notifications
                for sub_id, papers in all_results.items():
                    if papers:
                        sub = db.get_arxiv_subscription(sub_id)
                        topic = sub.get("topic", sub_id) if sub else sub_id
                        webhook.notify_papers_found(topic, papers, min_score=0.5)
            else:
                print_info("[Autopilot] No new papers found")

            state["last_check"] = time.strftime("%Y-%m-%dT%H:%M:%S")
            _save_watch_state(state)

        except Exception as e:
            logger.error(f"[Autopilot] Check failed: {e}")
            print_error(f"[Autopilot] Error: {e}")

        # Wait for interval or stop signal
        stop_event.wait(timeout=interval_minutes * 60)

    state["running"] = False
    _save_watch_state(state)
    print_info("[Autopilot] Watch stopped")


# ─── Parser ───────────────────────────────────────────────────────────────────


def _build_subscribe_parser(subparsers) -> argparse.ArgumentParser:
    """Build the subscribe subcommand parser."""
    p = subparsers.add_parser(
        "subscribe",
        help="Smart arXiv subscriptions",
        description="Subscribe to research topics and get AI-scored paper recommendations.",
    )

    sub = p.add_subparsers(dest="action", help="Subscription actions")

    # add
    p_add = sub.add_parser("add", help="Add a new subscription")
    p_add.add_argument("topic", help="Research topic keywords (e.g., 'transformer attention')")
    p_add.add_argument("--keywords", "-k", type=str, help="Additional keywords (comma-separated)")
    p_add.add_argument(
        "--min-score",
        "-s",
        type=float,
        default=0.5,
        help="Minimum relevance score (0.0-1.0, default: 0.5)",
    )
    p_add.add_argument(
        "--max-results", "-n", type=int, default=10, help="Max papers per check (default: 10)"
    )

    # list
    sub.add_parser("list", help="List all subscriptions")

    # check
    p_check = sub.add_parser("check", help="Check subscriptions for new papers")
    p_check.add_argument("id", nargs="?", help="Subscription ID (optional, checks all if omitted)")
    p_check.add_argument("--discord", type=str, default="", help="Discord webhook URL to test")
    p_check.add_argument("--feishu", type=str, default="", help="Feishu webhook URL to test")

    # recommendations
    p_rec = sub.add_parser("recommendations", help="Show recommended papers")
    p_rec.add_argument("id", help="Subscription ID")
    p_rec.add_argument("--limit", "-n", type=int, default=20, help="Max papers to show")

    # delete
    p_delete = sub.add_parser("delete", help="Delete a subscription")
    p_delete.add_argument("id", help="Subscription ID")

    # watch
    p_watch = sub.add_parser("watch", help="Start autopilot watch (background monitoring)")
    p_watch.add_argument(
        "--interval", "-i", type=int, default=60, help="Check interval in minutes (default: 60)"
    )
    p_watch.add_argument("--discord", type=str, default="", help="Discord webhook URL")
    p_watch.add_argument("--feishu", type=str, default="", help="Feishu webhook URL")
    p_watch.add_argument("--daemon", action="store_true", help="Run as daemon (background process)")

    # stop watch
    sub.add_parser("stop", help="Stop the autopilot watch")

    return p  # type: ignore[no-any-return]


# ─── Runner ───────────────────────────────────────────────────────────────────


# Module-level state for the watch thread (survives across CLI invocations)
_watch_thread: threading.Thread | None = None
_watch_stop_event: threading.Event | None = None


def _run_subscribe(args: argparse.Namespace) -> int:
    """Run subscribe command."""
    global _watch_thread, _watch_stop_event

    if not args.action:
        print_error("Usage: subscribe <add|list|check|recommendations|delete|watch|stop>")
        return 1  # type: ignore[no-any-return]

    db = get_db()
    db.init()

    # ── add ──────────────────────────────────────────────────────────────────
    if args.action == "add":
        keywords = []
        if getattr(args, "keywords", None):
            keywords = [k.strip() for k in args.keywords.split(",") if k.strip()]

        sub_id = db.add_arxiv_subscription(
            topic=args.topic,
            keywords=keywords,
            min_score=getattr(args, "min_score", 0.5),
            max_results=getattr(args, "max_results", 10),
        )
        print_success(f"Added subscription [{sub_id}]: {args.topic}")
        if keywords:
            print_info(f"  Keywords: {', '.join(keywords)}")
        return 0  # type: ignore[no-any-return]

    # ── list ─────────────────────────────────────────────────────────────────
    if args.action == "list":
        subs = db.list_arxiv_subscriptions()
        if not subs:
            print_info("No subscriptions. Use 'subscribe add <topic>' to create one.")
            return 0  # type: ignore[no-any-return]

        state = _load_watch_state()
        watch_status = "running" if state.get("running") else "stopped"

        print_info(f"Found {len(subs)} subscription(s) [watch: {watch_status}]:")
        for s in subs:
            keywords = s.get("keywords", []) or []
            last_checked = s.get("last_checked") or "never"
            print(f"  [{s['id']}] {s['topic']}")
            print(f"        min_score={s['min_score']}, max_results={s['max_results']}")
            if keywords:
                print(f"        keywords: {', '.join(keywords)}")
            print(f"        last checked: {last_checked}")
        return 0  # type: ignore[no-any-return]

    # ── check ────────────────────────────────────────────────────────────────
    if args.action == "check":
        from core.notifications import get_webhook_notifier, configure_webhook
        from llm.subscription_monitor import SubscriptionMonitor
        from llm.subscription_scorer import SubscriptionScorer
        from llm.litreview_analyzer import LitReviewAnalyzer

        # Configure webhook if provided
        webhook_url = getattr(args, "discord", "") or getattr(args, "feishu", "") or ""
        if webhook_url:
            configure_webhook(webhook_url)
            print_success(
                "Webhook configured: " + ("Discord" if getattr(args, "discord", "") else "Feishu")
            )

        monitor = SubscriptionMonitor(db, SubscriptionScorer(db))
        analyzer = LitReviewAnalyzer(db)
        webhook = get_webhook_notifier()

        if args.id:
            results = monitor.check_subscription(args.id)
            if not results:
                print_info("No new papers above threshold.")
            else:
                print_success(f"Found {len(results)} paper(s):")
                for r in results:
                    print(f"  [{r['arxiv_id']}] {r['title'][:60]}...")
                    print(f"        score={r['score']:.2f}")

                # Notify via webhook
                sub = db.get_arxiv_subscription(args.id)
                topic = sub.get("topic", args.id) if sub else args.id
                webhook.notify_papers_found(topic, results, min_score=0.5)

                # Auto-update literature review
                updated_file = analyzer.update_for_subscription(args.id, results)
                if updated_file:
                    print_info(f"  Updated litreview: {updated_file}")
        else:
            all_results = monitor.check_all()
            for sub_id, papers in all_results.items():
                sub = db.get_arxiv_subscription(sub_id)
                topic = sub.get("topic", sub_id) if sub else sub_id
                print_info(f"\nSubscription [{sub_id}] ({topic}): {len(papers)} new paper(s)")
                for r in papers[:3]:
                    print(f"  [{r['arxiv_id']}] {r['title'][:50]}... (score={r['score']:.2f})")

                if papers:
                    webhook.notify_papers_found(topic, papers, min_score=0.5)
                    updated_file = analyzer.update_for_subscription(sub_id, papers)
                    if updated_file:
                        print_info(f"  Updated litreview: {updated_file}")
        return 0

    # ── recommendations ─────────────────────────────────────────────────────
    if args.action == "recommendations":
        papers = db.get_subscription_papers(args.id, limit=getattr(args, "limit", 20))
        if not papers:
            print_info("No recommendations yet. Run 'subscribe check' first.")
            return 0

        print_success(f"Found {len(papers)} recommendation(s):")
        for p in papers:
            print(f"  [{p['arxiv_id']}]")
            print(f"    Title: {p.get('title', 'N/A')[:60]}")
            print(f"    Score: {p.get('score', 0):.2f}")
            print(f"    Published: {p.get('published', 'N/A')}")
        return 0

    # ── delete ──────────────────────────────────────────────────────────────
    if args.action == "delete":
        deleted = db.delete_arxiv_subscription(args.id)
        if deleted:
            print_success(f"Deleted subscription [{args.id}]")
        else:
            print_error(f"Subscription [{args.id}] not found")
        return 0

    # ── watch ───────────────────────────────────────────────────────────────
    if args.action == "watch":
        state = _load_watch_state()
        if state.get("running") and _watch_thread and _watch_thread.is_alive():
            print_error("[Autopilot] Watch is already running. Use 'subscribe stop' first.")
            return 1

        interval = getattr(args, "interval", 60)
        if interval < 5:
            print_error("Interval must be at least 5 minutes.")
            return 1

        # Configure webhook
        from core.notifications import configure_webhook

        discord_url = getattr(args, "discord", "") or ""
        feishu_url = getattr(args, "feishu", "") or ""

        # Prefer Discord if set, else Feishu
        webhook_url = discord_url or feishu_url
        if webhook_url:
            configure_webhook(webhook_url)
            platform = "Discord" if discord_url else "Feishu"
            print_success(f"[Autopilot] Webhook configured: {platform}")

        if getattr(args, "daemon", False):
            # Daemon mode: spawn background thread and exit
            stop_event = threading.Event()
            thread = threading.Thread(
                target=_run_watch_loop,
                args=(interval, stop_event),
                name="autopilot-watch",
                daemon=True,
            )
            thread.start()
            print_success(f"[Autopilot] Started in background (PID={thread.native_id})")
            return 0
        else:
            # Foreground mode: run in current thread (for cron/CI)
            print_info("[Autopilot] Running one cycle...")
            stop_event = threading.Event()
            _run_watch_loop(interval, stop_event)
            return 0

    # ── stop ────────────────────────────────────────────────────────────────
    if args.action == "stop":
        state = _load_watch_state()
        if not state.get("running"):
            print_info("Watch is not running.")
            return 0

        # Signal the watch to stop via state file (cross-process)
        state["running"] = False
        state["stop_requested"] = True
        _save_watch_state(state)

        if _watch_stop_event:
            _watch_stop_event.set()

        print_success("[Autopilot] Stop signal sent.")
        return 0

    print_error(f"Unknown action: {args.action}")
    return 1
