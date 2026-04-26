"""CLI command: insight — Manage key insight cards."""
from __future__ import annotations

import argparse

from cli._shared import get_db, print_info, print_error, print_success
from llm.insight_cards import InsightManager


def _build_insight_parser(subparsers) -> argparse.ArgumentParser:
    """Build the insight subcommand parser."""
    p = subparsers.add_parser(
        "insight",
        help="Manage key insight cards",
        description="Extract and manage key insights from papers.",
    )
    p.add_argument("action", choices=["add", "list", "search", "tag-cloud", "export"],
                   help="Action to perform")
    p.add_argument("--paper", help="Paper ID")
    p.add_argument("--content", help="Insight content")
    p.add_argument("--type", "-t", choices=["finding", "method", "limitation", "future_work"],
                   default="finding", help="Insight type")
    p.add_argument("--tags", help="Comma-separated tags")
    p.add_argument("--evidence", help="Evidence/paper reference")
    p.add_argument("--query", "-q", help="Search query")
    p.add_argument("--markdown", "-m", action="store_true", help="Output as Markdown")
    p.add_argument("--collection", "-c", help="Collection ID to add to")
    p.add_argument("--cite", help="Card ID to reference")
    return p


def _run_insight(args: argparse.Namespace) -> int:
    """Run insight command."""
    manager = InsightManager()

    if args.action == "add":
        if not args.paper or not args.content:
            print_error("Usage: insight add --paper <pid> --content <text>")
            return 1

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
            paper = db.get_paper(args.paper) if hasattr(db, 'get_paper') else None
            if paper:
                manager.update_card(card.card_id, tags=tags)  # Just update for now

        print_success(f"Created insight card: {card.card_id}")
        return 0

    elif args.action == "list":
        cards = manager.search_cards(
            query=args.query,
            tags=[t.strip() for t in args.tags.split(",")] if args.tags else None,
            insight_type=args.type if hasattr(args, 'type') else None,
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
        tags = manager.get_tag_cloud()
        if not tags:
            print("No tags found.")
            return 0

        print("📊 Tag Cloud\n")
        max_count = max(tags.values()) if tags else 1

        for tag, count in sorted(tags.items(), key=lambda x: -x[1])[:20]:
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
            from llm.insight_cards import InsightCollection
            collections = manager._load_collections()
            for c in collections:
                if c.get("collection_id") == args.collection:
                    card_ids = c.get("card_ids", [])
                    cards = [manager.get_card(cid) for cid in card_ids]
                    cards = [c for c in cards if c]
                    break

        output = manager.export_for_note(cards)
        print(output)
        return 0

    print_error(f"Unknown action: {args.action}")
    return 1
