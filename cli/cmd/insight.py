"""CLI command: insight — Manage key insight cards."""

from __future__ import annotations

import argparse
from typing import List, Optional, cast

from cli._shared import get_db, print_error, print_success
from llm.insight_cards import InsightCard, InsightManager


def _build_insight_parser(subparsers) -> argparse.ArgumentParser:
    """Build the insight subcommand parser."""
    p = subparsers.add_parser(
        "insight",
        help="Manage key insight cards",
        description="Extract and manage key insights from papers.",
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
            "eval-retrieval",
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
            card = manager.get_card(args.card)
            stars = "★" * args.stars + "☆" * (5 - args.stars)
            print_success(f"Rated {args.card}: {stars} ({args.stars}/5)")
            # Bridge to EvolutionTracker
            try:
                from llm.insight_evolution import get_evolution_tracker

                evo = get_evolution_tracker()
                topic = card.paper_title if card else ""
                evo.record_insight_feedback(
                    topic=topic, insight_card_id=args.card, rating=args.stars, paper_title=topic
                )
            except Exception:
                pass  # EvolutionTracker is optional
        else:
            print_error(f"Card not found: {args.card}")
        return 0

    elif args.action == "like":
        if not args.card:
            print_error("Usage: insight like --card <id>")
            return 1
        ok = manager.like_card(args.card)
        if ok:
            print_success(f"Liked {args.card} (★★★★★)")
            try:
                from llm.insight_evolution import get_evolution_tracker

                evo = get_evolution_tracker()
                card = manager.get_card(args.card)
                topic = card.paper_title if card else ""
                evo.record_insight_feedback(
                    topic=topic, insight_card_id=args.card, rating=5, paper_title=topic
                )
            except Exception:
                pass
        else:
            print_error(f"Card not found: {args.card}")
        return 0

    elif args.action == "dislike":
        if not args.card:
            print_error("Usage: insight dislike --card <id>")
            return 1
        ok = manager.dislike_card(args.card)
        if ok:
            print_success(f"Disliked {args.card} (★☆☆☆☆)")
            try:
                from llm.insight_evolution import get_evolution_tracker

                evo = get_evolution_tracker()
                card = manager.get_card(args.card)
                topic = card.paper_title if card else ""
                evo.record_insight_feedback(
                    topic=topic, insight_card_id=args.card, rating=1, paper_title=topic
                )
            except Exception:
                pass
        else:
            print_error(f"Card not found: {args.card}")
        return 0

    elif args.action == "top":
        cards = manager.get_high_quality_cards(min_rating=4, min_scores=1)
        if not cards:
            print("No highly-rated cards yet. Rate some cards first!")
            return 0
        print(f"Top {min(args.top_k, len(cards))} Highest-Rated Insights:")
        for c in cards[: args.top_k]:
            stars = "★" * c.quality_rating + "☆" * (5 - c.quality_rating)
            print(f"  [{c.card_id}] {stars} ({c.usefulness_score:.2f}) {c.content[:60]}")
        return 0

    elif args.action == "bottom":
        cards = manager.get_low_quality_cards(max_rating=2, min_scores=1)
        if not cards:
            print("No low-rated cards yet.")
            return 0
        print(f"Lowest-Rated Insights ({len(cards)} total):")
        for c in cards[: args.top_k]:
            stars = "★" * c.quality_rating + "☆" * (5 - c.quality_rating)
            print(f"  [{c.card_id}] {stars} ({c.usefulness_score:.2f}) {c.content[:60]}")
        return 0


    elif args.action == "eval-retrieval":
        from llm.insight.tracker import get_evolution_tracker
        tracker = get_evolution_tracker()
        limit = getattr(args, "top_k", 50)
        result = tracker.eval_retrieval(limit=limit)
        if "error" in result:
            print_error(f"eval-retrieval: {result["error"]}")
            return 1
        print(f"=== Gene Pool Retrieval Eval (n={result["total"]}) ===")
        print(f"  recall@3 : {result["recall@3"]}")
        print(f"  recall@5 : {result["recall@5"]}")
        print(f"  MRR      : {result["mrr"]}")
        return 0

    print_error(f"Unknown action: {args.action}")
    return 1
