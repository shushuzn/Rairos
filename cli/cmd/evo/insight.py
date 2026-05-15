"""CLI command: insight — Manage key insight cards."""

# [LEGACY] Insight extractor — depends on llm/insight/

from __future__ import annotations


import argparse

from typing import List, cast


from cli._shared import get_db, print_error, print_success

from llm.insight_cards import InsightCard, InsightManager


def _build_insight_parser(subparsers) -> argparse.ArgumentParser:
    """Build the insight subcommand parser."""

    p = subparsers.add_parser(
        "insight",
        help="Manage key insight cards",
        description="""Extract and manage key insights from papers.

Examples:
  airos insight add --paper p1 --content "BERT improves QA by 15%" --tags nlp,rag
  airos insight search --query attention
  airos insight rate --card i0001 --stars 4
  airos insight like --card i0001
  airos insight top
  airos insight tag-cloud
  airos insight quality-report --json
  airos insight quality-report --watch --interval 60
  airos insight alert --json
  airos insight evolve --query "RAG"
  airos insight recompute-credibility
  airos insight eval-retrieval

Valid --type values: finding, method, limitation, future_work
Valid gap_type values: capability, method_limitation, exploration_gap,
  unexplained_phenomenon, improvement, method_gap, embodied_planning,
  cross_domain, theoretical, empirical""",
    )

    p.add_argument(
        "action",
        choices=[
            "add",
            "list",
            "search",
            "tag-cloud",
            "export",
            "rate",
            "like",
            "dislike",
            "top",
            "bottom",
            "evolve",
            "quality-report",
            "recompute-credibility",
            "archive-trendslop",
            "eval-retrieval",
            "alert",
            "kg-bridge",
            "promote",
        ],
        help="Action to perform",
    )
    p.add_argument("--paper", help="Paper ID")

    p.add_argument("--content", help="Insight content")

    p.add_argument(
        "--type",
        "-t",
        choices=["finding", "method", "limitation", "future_work"],
        default="finding",
        help="Insight type",
    )

    p.add_argument("--tags", help="Comma-separated tags")

    p.add_argument("--evidence", help="Evidence/paper reference")

    p.add_argument("--query", "-q", help="Search query")

    p.add_argument("--markdown", "-m", action="store_true", help="Output as Markdown")

    p.add_argument("--collection", "-c", help="Collection ID to add to")

    p.add_argument("--cite", help="Card ID to reference")

    p.add_argument("--card", help="Card ID to rate/like/dislike")

    p.add_argument("--stars", type=int, choices=[1, 2, 3, 4, 5], help="Star rating 1-5")

    p.add_argument("--top-k", type=int, default=10, help="Number of top/bottom cards to show")

    p.add_argument("--json", action="store_true", help="Output as JSON (for machine parsing)")

    p.add_argument("--watch", action="store_true", help="Watch mode: continuously output report")

    p.add_argument(
        "--interval", type=int, default=30, help="Watch interval in seconds (default: 30)"
    )

    return p  # type: ignore[no-any-return]


def _run_insight(args: argparse.Namespace) -> int:
    """Run insight command."""

    manager = InsightManager()

    if args.action == "add":
        if not args.paper or not args.content:
            print_error("Usage: insight add --paper <pid> --content <text>")

            return 1  # type: ignore[no-any-return]

        tags = [t.strip() for t in args.tags.split(",")] if args.tags else []

        card = manager.add_card(
            paper_id=args.paper,
            paper_title=args.paper,  # Will be updated if paper found
            content=args.content,
            insight_type=args.type,
            tags=tags,
            evidence=args.evidence or "",
        )

        # Try to get paper title

        if args.paper:
            db = get_db()

            db.init()

            paper = db.get_paper(args.paper) if hasattr(db, "get_paper") else None

            if paper:
                manager.update_card(card.card_id, tags=tags)  # Just update for now

        print_success(f"Created insight card: {card.card_id}")

        return 0  # type: ignore[no-any-return]

    elif args.action == "list":
        cards = manager.search_cards(
            query=args.query,
            tags=[t.strip() for t in args.tags.split(",")] if args.tags else None,
            insight_type=args.type if hasattr(args, "type") else None,
        )

        if args.markdown:
            print(manager.render_markdown(cards))

        else:
            print(manager.render_text(cards))

        return 0

    elif args.action == "search":
        cards = manager.search_cards(
            query=args.query,
            tags=[t.strip() for t in args.tags.split(",")] if args.tags else None,
        )

        if args.markdown:
            print(manager.render_markdown(cards))

        else:
            print(manager.render_text(cards))

        return 0

    elif args.action == "tag-cloud":
        tag_cloud = manager.get_tag_cloud()

        if not tag_cloud:
            print("No tags found.")

            return 0

        print("📊 Tag Cloud\n")

        max_count = max(tag_cloud.values()) if tag_cloud else 1

        for tag, count in sorted(tag_cloud.items(), key=lambda x: -x[1])[:20]:
            bar = "█" * int(count / max_count * 20)

            print(f"  {tag:20} {count:3} {bar}")

        return 0

    elif args.action == "export":
        cards = manager.search_cards(
            query=args.query,
            tags=[t.strip() for t in args.tags.split(",")] if args.tags else None,
        )

        if args.collection:
            # Get cards from collection

            collections = manager._load_collections()

            for c in collections:
                if c.get("collection_id") == args.collection:
                    card_ids = c.get("card_ids", [])

                    raw_cards = [manager.get_card(cid) for cid in card_ids]

                    cards = cast(List[InsightCard], [c for c in raw_cards if c])

                    break

        output = manager.export_for_note(cards)

        print(output)

        return 0

    elif args.action == "rate":
        if not args.card or args.stars is None:
            print_error("Usage: insight rate --card <id> --stars <1-5>")

            return 1

        ok = manager.rate_card(args.card, args.stars)

        if ok:
            card = manager.get_card(args.card)  # type: ignore[assignment]

            stars = "★" * args.stars + "☆" * (5 - args.stars)

            print_success(f"Rated {args.card}: {stars} ({args.stars}/5)")

            # Bridge to EvolutionTracker

            try:
                from llm.insight import get_evolution_tracker

                evo = get_evolution_tracker()

                topic = card.paper_title if card else ""

                evo.record_insight_feedback(
                    topic=topic, insight_card_id=args.card, rating=args.stars, paper_title=topic
                )

            except Exception:
                pass  # EvolutionTracker is optional

        else:
            print_error(f"Card not found: {args.card}")
            return 1

        return 0

    elif args.action == "like":
        if not args.card:
            print_error("Usage: insight like --card <id>")

            return 1

        ok = manager.like_card(args.card)

        if ok:
            print_success(f"Liked {args.card} (★★★★★)")

            try:
                from llm.insight import get_evolution_tracker

                evo = get_evolution_tracker()

                card = manager.get_card(args.card)  # type: ignore[assignment]

                topic = card.paper_title if card else ""

                evo.record_insight_feedback(
                    topic=topic, insight_card_id=args.card, rating=5, paper_title=topic
                )

            except Exception:
                pass

        else:
            print_error(f"Card not found: {args.card}")
            return 1

        return 0

    elif args.action == "dislike":
        if not args.card:
            print_error("Usage: insight dislike --card <id>")

            return 1

        ok = manager.dislike_card(args.card)

        if ok:
            print_success(f"Disliked {args.card} (★☆☆☆☆)")

            try:
                from llm.insight import get_evolution_tracker

                evo = get_evolution_tracker()

                card = manager.get_card(args.card)  # type: ignore[assignment]

                topic = card.paper_title if card else ""

                evo.record_insight_feedback(
                    topic=topic, insight_card_id=args.card, rating=1, paper_title=topic
                )

            except Exception:
                pass

        else:
            print_error(f"Card not found: {args.card}")
            return 1

        return 0

    elif args.action == "top":
        cards = manager.get_high_quality_cards(min_rating=4, min_scores=1)  # type: ignore[assignment]

        if not cards:
            print("No highly-rated cards yet. Rate some cards first!")

            return 0

        print(f"Top {min(args.top_k, len(cards))} Highest-Rated Insights:")

        for c in cards[: args.top_k]:  # type: ignore[assignment]
            stars = "★" * c.quality_rating + "☆" * (5 - c.quality_rating)  # type: ignore[attr-defined]

            print(f"  [{c.card_id}] {stars} ({c.usefulness_score:.2f}) {c.content[:60]}")  # type: ignore[attr-defined]

        return 0

    elif args.action == "bottom":
        cards = manager.get_low_quality_cards(max_rating=2, min_scores=1)  # type: ignore[assignment]

        if not cards:
            print("No low-rated cards yet.")

            return 0

        print(f"Lowest-Rated Insights ({len(cards)} total):")

        for c in cards[: args.top_k]:  # type: ignore[assignment]
            stars = "★" * c.quality_rating + "☆" * (5 - c.quality_rating)  # type: ignore[attr-defined]

            print(f"  [{c.card_id}] {stars} ({c.usefulness_score:.2f}) {c.content[:60]}")  # type: ignore[attr-defined]

        return 0

    elif args.action == "evolve":
        topic = args.query or ""
        gap_type = args.tags or None
        if not topic:
            print_error("Usage: insight evolve --query <topic> [--tags <gap_type>]")
            return 1
        from llm.insight.evolution import InsightEvolution
        from llm.insight.tracker import get_evolution_tracker

        evo = InsightEvolution(tracker=get_evolution_tracker())  # type: ignore[assignment]
        result = evo.evolve(topic=topic, gap_type=gap_type)  # type: ignore[attr-defined]
        print(evo.render_summary(result))  # type: ignore[attr-defined]
        return 0

    elif args.action == "quality-report":
        import json as _json

        from llm.insight.tracker import get_evolution_tracker

        tracker = get_evolution_tracker()

        def _render_report(report):
            if args.json:
                print(_json.dumps(report, indent=2))
            else:
                print("=== Gene Pool Quality Report ===")
                print(f"  Total capsules    : {report['total']}")
                print(f"  Avg score        : {report['avg_score']}")
                print(
                    f"  Score dist       : high={report['score_distribution']['high (≥0.7)']} mid={report['score_distribution']['mid (0.4-0.7)']} low={report['score_distribution']['low (<0.4)']}"
                )
                print(f"  Credibility dist : {report['credibility_distribution']}")
                print(
                    f"  Trendslop        : {report['trendslop']['count']} ({report['trendslop']['pct']}) — {report['trendslop']['top_reasons']}"
                )
                print(
                    f"  Feedback use     : high={report['feedback_distribution']['high_use (≥3)']} zero={report['feedback_distribution']['low_use (0)']}"
                )
                print(f"  At-risk (streak≥2): {report['at_risk_capsules']}")
                print(f"  Top gap types    : {report['top_gap_types']}")

        if args.watch:
            import time

            print("Watching GenePool health (Ctrl+C to stop)...", flush=True)
            while True:
                report = tracker.get_gene_pool_quality_report()
                if "error" in report:
                    print_error(f"quality-report: {report['error']}")
                    return 1
                ts = time.strftime("%Y-%m-%d %H:%M:%S")
                if args.json:
                    print(_json.dumps({"timestamp": ts, **report}))
                else:
                    print(f"\n[{ts}] === Gene Pool Quality Report ===")
                    print(f"  Total capsules    : {report['total']}")
                    print(f"  Avg score        : {report['avg_score']}")
                    print(
                        f"  Score dist       : high={report['score_distribution']['high (≥0.7)']} mid={report['score_distribution']['mid (0.4-0.7)']} low={report['score_distribution']['low (<0.4)']}"
                    )
                    print(f"  Credibility dist : {report['credibility_distribution']}")
                    print(
                        f"  Trendslop        : {report['trendslop']['count']} ({report['trendslop']['pct']})"
                    )
                    print(f"  At-risk (streak≥2): {report['at_risk_capsules']}")
                time.sleep(args.interval)
        else:
            report = tracker.get_gene_pool_quality_report()
            if "error" in report:
                print_error(f"quality-report: {report['error']}")
                return 1
            _render_report(report)
        return 0

    elif args.action == "recompute-credibility":
        from llm.insight.tracker import get_evolution_tracker

        tracker = get_evolution_tracker()
        result = tracker.recompute_credibility_all()
        print(
            f"Recomputed credibility: {result['updated']} updated, {result['errors']} errors (archived capsules skipped)"
        )
        return 0

    elif args.action == "archive-trendslop":
        from llm.insight.tracker import get_evolution_tracker

        # type: ignore[assignment]
        tracker = get_evolution_tracker()
        capsules = tracker._load_capsules()
        trendslop_capsules = [c for c in capsules if c.trendslop and c.status == "active"]
        if not trendslop_capsules:
            print("No active trendslop capsules to archive.")
            return 0
        archived = 0
        for c in trendslop_capsules:  # type: ignore[assignment]
            if tracker.archive_capsule(c.capsule_id):  # type: ignore[attr-defined]
                archived += 1
        print(f"Archived {archived} trendslop capsules out of {len(trendslop_capsules)} flagged.")
        return 0

    elif args.action == "eval-retrieval":
        from llm.insight.tracker import get_evolution_tracker

        tracker = get_evolution_tracker()
        limit = getattr(args, "top_k", 50)
        result = tracker.eval_retrieval(limit=limit)
        if "error" in result:
            print_error(f"eval-retrieval: {result['error']}")
            return 1
        print(f"=== Gene Pool Retrieval Eval (n={result['total']}) ===")
        print(f"  recall@3 : {result['recall@3']}")
        print(f"  recall@5 : {result['recall@5']}")
        print(f"  MRR      : {result['mrr']}")
        return 0

    elif args.action == "alert":
        import json as _json

        from llm.insight.tracker import get_evolution_tracker

        tracker = get_evolution_tracker()
        report = tracker.get_gene_pool_quality_report()
        if "error" in report:
            print_error(f"alert: {report['error']}")
            return 1
        alerts = report.get("alerts", [])
        if args.json:
            print(
                _json.dumps(
                    {"total": report["total"], "alert_count": len(alerts), "alerts": alerts}
                )
            )
            return 0
        if not alerts:
            print(f"No alerts (GenePool healthy: {report['total']} capsules)")
            return 0
        level_icons = {"critical": "🔴", "warning": "🟡", "info": "🔵"}
        for alert in alerts:
            icon = level_icons.get(alert["level"], "⚪")
            print(f"{icon} [{alert['level'].upper()}] {alert['code']}: {alert['message']}")
        return 0

    elif args.action == "kg-bridge":
        import json as _json

        from kg.manager import KGManager
        from llm.insight.tracker import sync_gene_pool_to_kg

        kg = KGManager()
        result = sync_gene_pool_to_kg(kg_manager=kg)
        if args.json:
            print(_json.dumps(result))
            return 0
        print("=== GenePool → Knowledge Graph Bridge ===")
        print(f"  Capsules synced  : {result['synced']}")
        print(f"  Eligible total  : {result['eligible']}")
        print(f"  Total in pool   : {result['total_capsules']}")
        print(f"  Errors          : {result['errors']}")
        if result["synced"] > 0:
            stats = kg.stats()
            gp_nodes = stats["nodes_by_type"].get("GenePool-Capsule", 0)
            print(f"  KG nodes now    : GenePool-Capsule={gp_nodes}")
        return 0

    elif args.action == "promote":
        import json as _json

        result = manager.promote_capsules_to_insights()
        if args.json:
            print(_json.dumps(result))
            return 0
        print("=== GenePool → Insight Cards Promotion ===")
        print(f"  Cards promoted  : {result['promoted']}")
        print(f"  Already promoted: {result['skipped_already']}")
        print(f"  No source       : {result['skipped_no_source']}")
        print(f"  Eligible total  : {result['eligible_capsules']}")
        print(f"  Total cards now : {result['total_cards']}")
        if result["promoted"] > 0:
            print("\n  Run 'airos insight list' to see new cards.")
        return 0

    print_error(f"Unknown action: {args.action}")

    return 1
