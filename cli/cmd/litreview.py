"""CLI command: litreview — Incremental Literature Review management."""
from __future__ import annotations

import argparse
import os
from pathlib import Path

from cli._shared import get_db, print_info, print_error


def _build_litreview_parser(subparsers) -> argparse.ArgumentParser:
    """Build the litreview subcommand parser."""
    p = subparsers.add_parser(
        "litreview",
        help="Incremental literature review management",
        description="Generate and maintain living literature reviews from subscription papers.",
    )

    sub = p.add_subparsers(dest="action", help="Review actions")

    # generate
    p_gen = sub.add_parser("generate", help="Generate or update a literature review")
    p_gen.add_argument("topic", help="Research topic for the review")
    p_gen.add_argument("--subscription", "-s", type=str,
                        help="Subscription ID to link to")
    p_gen.add_argument("--output", "-o", type=str,
                        help="Output file path (default: LitReview-{topic}.md)")

    # view
    p_view = sub.add_parser("view", help="View a literature review")
    p_view.add_argument("id", help="Review ID or topic")
    p_view.add_argument("--file", "-f", type=str,
                        help="Direct file path to view")

    # list
    sub.add_parser("list", help="List all literature reviews")

    # delete
    p_del = sub.add_parser("delete", help="Delete a literature review")
    p_del.add_argument("id", help="Review ID")

    return p


def _run_litreview(args: argparse.Namespace) -> int:
    """Run litreview command."""
    from renderers.litreview import render_litreview, update_litreview

    if not args.action:
        print_error("Usage: litreview <generate|view|list|delete>")
        return 1

    db = get_db()
    db.init()

    if args.action == "generate":
        topic = args.topic
        subscription_id = getattr(args, 'subscription', None)
        output_path = getattr(args, 'output', None)

        # Get papers
        papers = []
        if subscription_id:
            papers = db.get_subscription_papers(subscription_id, limit=100)
        else:
            # Get all papers from all subscriptions for this topic
            subs = db.list_arxiv_subscriptions()
            topic_lower = topic.lower()
            for sub in subs:
                if topic_lower in sub.get('topic', '').lower():
                    sub_papers = db.get_subscription_papers(sub['id'], limit=100)
                    papers.extend(sub_papers)

        # Generate review content
        review_md = render_litreview(topic, papers)

        # Determine output path
        if not output_path:
            safe_topic = "".join(c if c.isalnum() else "-" for c in topic)[:30]
            output_path = f"LitReview-{safe_topic}.md"

        # Write file
        try:
            with open(output_path, "w", encoding="utf-8") as f:
                f.write(review_md)
            print_info(f"Generated: {output_path}")
        except IOError as e:
            print_error(f"Failed to write file: {e}")
            return 1

        # Record in database
        review_id = db.add_literature_review(
            topic=topic,
            subscription_id=subscription_id,
            file_path=str(Path(output_path).resolve()),
        )
        db.update_literature_review(review_id, paper_count=len(papers))

        print_info(f"Created review [{review_id}] with {len(papers)} papers")
        return 0

    if args.action == "view":
        review_id = getattr(args, 'id', '')
        file_path = getattr(args, 'file', None)

        content = None
        if file_path:
            try:
                with open(file_path, "r", encoding="utf-8") as f:
                    content = f.read()
                print_info(f"=== {file_path} ===")
            except IOError as e:
                print_error(f"Failed to read file: {e}")
                return 1
        else:
            # Try to find by ID or topic
            review = db.get_literature_review(review_id)
            if review:
                fp = review.get('file_path')
                if fp and os.path.exists(fp):
                    with open(fp, "r", encoding="utf-8") as f:
                        content = f.read()
                    print_info(f"=== {review['topic']} (ID: {review_id}) ===")
                else:
                    print_error(f"Review file not found: {fp}")
                    return 1
            else:
                # Try as topic
                reviews = db.list_literature_reviews()
                for r in reviews:
                    if review_id.lower() in r.get('topic', '').lower():
                        fp = r.get('file_path')
                        if fp and os.path.exists(fp):
                            with open(fp, "r", encoding="utf-8") as f:
                                content = f.read()
                            print_info(f"=== {r['topic']} (ID: {r['id']}) ===")
                            break

        if content:
            print(content)
        else:
            print_error(f"Review not found: {review_id}")
            return 1
        return 0

    if args.action == "list":
        reviews = db.list_literature_reviews()
        if not reviews:
            print_info("No literature reviews. Use 'litreview generate <topic>' to create one.")
            return 0

        print_info(f"Found {len(reviews)} literature review(s):")
        for r in reviews:
            print(f"  [{r['id']}] {r['topic']}")
            print(f"        papers: {r.get('paper_count', 0)}")
            print(f"        last updated: {r.get('last_updated', 'never')}")
            if r.get('file_path'):
                print(f"        file: {r['file_path']}")
        return 0

    if args.action == "delete":
        deleted = db.delete_literature_review(args.id)
        if deleted:
            print_info(f"Deleted review [{args.id}]")
        else:
            print_error(f"Review [{args.id}] not found")
        return 0

    print_error(f"Unknown action: {args.action}")
    return 1
