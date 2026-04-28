"""CLI command: subscribe — Smart arXiv subscription management."""
from __future__ import annotations

import argparse

from cli._shared import get_db, print_info, print_error


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
    p_add.add_argument("--keywords", "-k", type=str,
                        help="Additional keywords (comma-separated)")
    p_add.add_argument("--min-score", "-s", type=float, default=0.5,
                        help="Minimum relevance score (0.0-1.0, default: 0.5)")
    p_add.add_argument("--max-results", "-n", type=int, default=10,
                        help="Max papers per check (default: 10)")

    # list
    sub.add_parser("list", help="List all subscriptions")

    # check
    p_check = sub.add_parser("check", help="Check subscriptions for new papers")
    p_check.add_argument("id", nargs="?", help="Subscription ID (optional, checks all if omitted)")

    # recommendations
    p_rec = sub.add_parser("recommendations", help="Show recommended papers")
    p_rec.add_argument("id", help="Subscription ID")
    p_rec.add_argument("--limit", "-n", type=int, default=20, help="Max papers to show")

    # delete
    p_delete = sub.add_parser("delete", help="Delete a subscription")
    p_delete.add_argument("id", help="Subscription ID")

    return p


def _run_subscribe(args: argparse.Namespace) -> int:
    """Run subscribe command."""
    if not args.action:
        print_error("Usage: subscribe <add|list|check|recommendations|delete>")
        return 1

    db = get_db()
    db.init()

    if args.action == "add":
        keywords = []
        if getattr(args, 'keywords', None):
            keywords = [k.strip() for k in args.keywords.split(",") if k.strip()]

        sub_id = db.add_arxiv_subscription(
            topic=args.topic,
            keywords=keywords,
            min_score=getattr(args, 'min_score', 0.5),
            max_results=getattr(args, 'max_results', 10),
        )
        print_info(f"Added subscription [{sub_id}]: {args.topic}")
        if keywords:
            print_info(f"  Keywords: {', '.join(keywords)}")
        return 0

    if args.action == "list":
        subs = db.list_arxiv_subscriptions()
        if not subs:
            print_info("No subscriptions. Use 'subscribe add <topic>' to create one.")
            return 0

        print_info(f"Found {len(subs)} subscription(s):")
        for s in subs:
            keywords = s.get('keywords', []) or []
            last_checked = s.get('last_checked') or "never"
            print(f"  [{s['id']}] {s['topic']}")
            print(f"        min_score={s['min_score']}, max_results={s['max_results']}")
            if keywords:
                print(f"        keywords: {', '.join(keywords)}")
            print(f"        last checked: {last_checked}")
        return 0

    if args.action == "check":
        from llm.subscription_monitor import SubscriptionMonitor
        from llm.subscription_scorer import SubscriptionScorer
        from llm.litreview_analyzer import LitReviewAnalyzer

        monitor = SubscriptionMonitor(db, SubscriptionScorer(db))
        analyzer = LitReviewAnalyzer(db)

        if args.id:
            results = monitor.check_subscription(args.id)
            if not results:
                print_info("No new papers above threshold.")
            else:
                print_info(f"Found {len(results)} paper(s):")
                for r in results:
                    print(f"  [{r['arxiv_id']}] {r['title'][:60]}...")
                    print(f"        score={r['score']:.2f}")

                # Auto-update literature review if one exists
                updated_file = analyzer.update_for_subscription(args.id, results)
                if updated_file:
                    print_info(f"  Updated litreview: {updated_file}")
        else:
            all_results = monitor.check_all()
            for sub_id, papers in all_results.items():
                print_info(f"\nSubscription [{sub_id}]: {len(papers)} new paper(s)")
                for r in papers[:3]:  # Show top 3
                    print(f"  [{r['arxiv_id']}] {r['title'][:50]}... (score={r['score']:.2f})")

                # Auto-update literature reviews
                if papers:
                    updated_file = analyzer.update_for_subscription(sub_id, papers)
                    if updated_file:
                        print_info(f"  Updated litreview: {updated_file}")
        return 0

    if args.action == "recommendations":
        papers = db.get_subscription_papers(args.id, limit=getattr(args, 'limit', 20))
        if not papers:
            print_info("No recommendations yet. Run 'subscribe check' first.")
            return 0

        print_info(f"Found {len(papers)} recommendation(s):")
        for p in papers:
            print(f"  [{p['arxiv_id']}]")
            print(f"    Title: {p.get('title', 'N/A')[:60]}")
            print(f"    Score: {p.get('score', 0):.2f}")
            print(f"    Published: {p.get('published', 'N/A')}")
        return 0

    if args.action == "delete":
        deleted = db.delete_arxiv_subscription(args.id)
        if deleted:
            print_info(f"Deleted subscription [{args.id}]")
        else:
            print_error(f"Subscription [{args.id}] not found")
        return 0

    print_error(f"Unknown action: {args.action}")
    return 1
