"""CLI command: question — Manage research questions."""
from __future__ import annotations

import argparse
from pathlib import Path

from cli._shared import get_db, print_info, print_error
from llm.question_tracker import QuestionTracker, QuestionStatus, QuestionSource


def _build_question_parser(subparsers) -> argparse.ArgumentParser:
    """Build the question subcommand parser."""
    p = subparsers.add_parser(
        "question",
        help="Manage research questions",
        description="Track and manage research questions from gap detection and manual entry.",
    )

    sub = p.add_subparsers(dest="action", help="Question actions")

    # list
    p_list = sub.add_parser("list", help="List all questions")
    p_list.add_argument("--status", choices=["open", "in_progress", "resolved", "wontfix"])
    p_list.add_argument("--topic", type=str, help="Filter by topic")
    p_list.add_argument("--source", type=str, help="Filter by source")
    p_list.add_argument("-v", "--verbose", action="store_true", help="Verbose output")

    # add
    p_add = sub.add_parser("add", help="Add a new question")
    p_add.add_argument("question", help="Research question text")
    p_add.add_argument("--topic", "-t", type=str, help="Research topic")
    p_add.add_argument("--priority", "-p", type=int, default=5, help="Priority 1-10")
    p_add.add_argument("--notes", "-n", type=str, help="Additional notes")

    # get
    p_get = sub.add_parser("get", help="Get a question by ID")
    p_get.add_argument("id", help="Question ID")

    # update
    p_update = sub.add_parser("update", help="Update a question")
    p_update.add_argument("id", help="Question ID")
    p_update.add_argument("--status", "-s", choices=["open", "in_progress", "resolved", "wontfix"])
    p_update.add_argument("--notes", "-n", type=str, help="Update notes")
    p_update.add_argument("--priority", "-p", type=int, help="Update priority 1-10")

    # link
    p_link = sub.add_parser("link", help="Link a paper to a question")
    p_link.add_argument("id", help="Question ID")
    p_link.add_argument("paper_id", help="Paper ID (arxiv ID or UID)")

    # unlink
    p_unlink = sub.add_parser("unlink", help="Unlink a paper from a question")
    p_unlink.add_argument("id", help="Question ID")
    p_unlink.add_argument("paper_id", help="Paper ID to unlink")

    # delete
    p_delete = sub.add_parser("delete", help="Delete a question")
    p_delete.add_argument("id", help="Question ID")

    # sync
    p_sync = sub.add_parser("sync", help="Sync questions from gap detection")
    p_sync.add_argument("--topic", "-t", type=str, help="Research topic")
    p_sync.add_argument("--priority", "-p", type=int, default=7, help="Priority 1-10")

    # stats
    p_stats = sub.add_parser("stats", help="Show question statistics")

    return p


def _run_question(args: argparse.Namespace) -> int:
    """Run question management command."""
    tracker = QuestionTracker()

    if args.action == "list":
        questions = tracker.list_questions(
            status=args.status,
            topic=args.topic,
            source=args.source,
        )
        print(tracker.render_list(questions, verbose=args.verbose))

    elif args.action == "add":
        q = tracker.add(
            question=args.question,
            source=QuestionSource.MANUAL.value,
            topic=args.topic or "",
            priority=args.priority,
            notes=args.notes or "",
        )
        print(f"✓ 添加问题 [{q.id}]: {q.question}")
        print(f"  来源: {q.source} | 优先级: {q.priority}")

    elif args.action == "get":
        q = tracker.get(args.id)
        if q:
            status_icon = {"open": "○", "in_progress": "◐", "resolved": "●", "wontfix": "✗"}
            print(f"问题: {q.question}")
            print(f"ID: {q.id}")
            print(f"状态: {status_icon.get(q.status, '?')} {q.status}")
            print(f"来源: {q.source}")
            print(f"优先级: {q.priority}/10")
            if q.topic:
                print(f"主题: {q.topic}")
            print(f"创建: {q.created_at}")
            print(f"更新: {q.updated_at}")
            if q.related_papers:
                print(f"关联论文: {', '.join(q.related_papers)}")
            if q.notes:
                print(f"备注: {q.notes}")
        else:
            print_error(f"问题 [{args.id}] 不存在")

    elif args.action == "update":
        q = tracker.update(
            args.id,
            status=args.status,
            notes=args.notes,
            priority=args.priority,
        )
        if q:
            print(f"✓ 更新问题 [{q.id}]: {q.question}")
        else:
            print_error(f"问题 [{args.id}] 不存在")

    elif args.action == "link":
        q = tracker.link_paper(args.id, args.paper_id)
        if q:
            print(f"✓ 关联论文 [{args.paper_id}] → 问题 [{q.id}]")
        else:
            print_error(f"问题 [{args.id}] 不存在")

    elif args.action == "unlink":
        q = tracker.unlink_paper(args.id, args.paper_id)
        if q:
            print(f"✓ 取消关联 [{args.paper_id}] ← 问题 [{q.id}]")
        else:
            print_error(f"问题 [{args.id}] 不存在")

    elif args.action == "delete":
        if tracker.delete(args.id):
            print(f"✓ 删除问题 [{args.id}]")
        else:
            print_error(f"问题 [{args.id}] 不存在")

    elif args.action == "sync":
        # Sync from gap detection if db is available
        db = get_db()
        db.init()

        topic = args.topic or "general"
        gaps = [
            "长文档场景下的检索效率问题",
            "检索结果与生成质量的一致性保证",
            "跨领域知识迁移的有效性评估",
        ]

        new_questions = tracker.sync_from_gaps(
            gaps=gaps,
            topic=topic,
            priority=args.priority,
        )

        if new_questions:
            print(f"✓ 同步了 {len(new_questions)} 个新问题:")
            for q in new_questions:
                print(f"  - [{q.id}] {q.question}")
        else:
            print("没有新的问题需要同步")

    elif args.action == "stats":
        stats = tracker.get_stats()
        print("📊 研究问题统计")
        print(f"总计: {stats['total']} 个问题")
        print("")
        print("按状态:")
        for status, count in stats['by_status'].items():
            print(f"  {status}: {count}")
        print("")
        print("按来源:")
        for source, count in stats['by_source'].items():
            print(f"  {source}: {count}")
        if stats['by_topic']:
            print("")
            print("按主题:")
            for topic, count in stats['by_topic'].items():
                print(f"  {topic}: {count}")

    else:
        print_error("未知操作，请使用 --help 查看帮助")
        return 1

    return 0
