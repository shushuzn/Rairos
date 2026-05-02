"""CLI command: narrative — Research Narrative Tracker."""
from __future__ import annotations

import argparse

from cli._shared import print_info, print_error, print_success
from llm.research_narrative_tracker import (
    ResearchNarrativeTracker,
    ResearchNarrativeService,
    NarrativePhase,
    render_thread,
    render_dashboard,
)


def _build_narrative_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "narrative",
        help="Research Narrative Tracker — unified view across gaps, hypotheses, experiments",
        description="Aggregate all research state into narrative threads with phase tracking and publication readiness scoring.",
    )
    p.add_argument(
        "action",
        nargs="?",
        default="list",
        choices=["list", "show", "track", "update", "note", "dashboard"],
        help="Action to perform",
    )
    p.add_argument(
        "target",
        nargs="?",
        default=None,
        help="Topic or thread ID depending on action",
    )
    p.add_argument(
        "--phase", "-p",
        type=str,
        choices=["exploration", "hypothesis", "validation", "publication"],
        help="Set thread phase (for 'update' action)",
    )
    p.add_argument(
        "--note", "-n",
        type=str,
        help="Set narrative note (for 'note' action)",
    )
    p.add_argument(
        "--json", "-j",
        action="store_true",
        help="Output as JSON",
    )
    p.add_argument(
        "--force", "-f",
        action="store_true",
        help="Force refresh even if thread exists (for 'track' action)",
    )
    return p


def _run_narrative(args: argparse.Namespace) -> int:
    tracker = ResearchNarrativeTracker()
    service = ResearchNarrativeService(tracker)

    action = args.action

    if action == "list":
        return _run_list(tracker)

    if action == "dashboard":
        return _run_dashboard(tracker)

    if action == "track":
        if not args.target:
            print_error("请提供 topic: airos narrative track <topic>")
            return 1
        return _run_track(args.target, service, tracker, force=args.force)

    if action == "show":
        if not args.target:
            print_error("请提供 topic: airos narrative show <topic>")
            return 1
        return _run_show(args.target, service)

    if action == "update":
        if not args.target:
            print_error("请提供 thread ID: airos narrative update <id> --phase <phase>")
            return 1
        if not args.phase:
            print_error("请提供 --phase: exploration | hypothesis | validation | publication")
            return 1
        return _run_update(args.target, args.phase, tracker)

    if action == "note":
        if not args.target:
            print_error("请提供 thread ID: airos narrative note <id> --note 'text'")
            return 1
        if not args.note:
            print_error("请提供 --note '内容'")
            return 1
        return _run_note(args.target, args.note, tracker)

    return 0


def _run_list(tracker: ResearchNarrativeTracker) -> int:
    threads = tracker.list_threads()
    if not threads:
        print_info("没有研究线程. 运行 airos narrative track <topic> 创建第一个.")
        return 0
    print(render_dashboard(threads))
    return 0


def _run_dashboard(tracker: ResearchNarrativeTracker) -> int:
    threads = tracker.list_threads()
    print(render_dashboard(threads))
    return 0


def _run_track(topic: str, service: ResearchNarrativeService, tracker: ResearchNarrativeTracker, force: bool = False) -> int:
    existing = tracker.get_by_topic(topic) if not force else None
    if existing and not force:
        print_info(f"Thread 已存在 ({existing.id}). 使用 --force 刷新.")
        print()
        print(render_thread(existing, service))
        return 0

    print_info(f"🔄 Aggregating research state for: {topic}")
    thread = service.aggregate(topic)
    service.save(thread)
    print_success(f"✅ Thread created/updated: {thread.id}")
    print()
    print(render_thread(thread, service))
    return 0


def _run_show(topic: str, service: ResearchNarrativeService) -> int:
    from llm.research_narrative_tracker import ResearchNarrativeTracker as RNT
    tracker = RNT()
    thread = tracker.get_by_topic(topic)
    if not thread:
        print_error(f"Thread '{topic}' 不存在. 运行: airos narrative track '{topic}'")
        return 1

    if thread.phase != NarrativePhase.EXPLORATION:
        # Re-aggregate to refresh scores
        thread = service.aggregate(topic)
        service.save(thread)

    print(render_thread(thread, service))
    return 0


def _run_update(thread_id: str, phase_str: str, tracker: ResearchNarrativeTracker) -> int:
    thread = tracker.get_thread(thread_id)
    if not thread:
        print_error(f"Thread [{thread_id}] 不存在")
        return 1

    old_phase = thread.phase
    thread.phase = NarrativePhase(phase_str)
    thread.phase_updated_at = __import__("datetime").datetime.now().isoformat()
    tracker.upsert(thread)
    print_success(f"Phase updated: {old_phase.value} → {thread.phase.value}")
    return 0


def _run_note(thread_id: str, note: str, tracker: ResearchNarrativeTracker) -> int:
    thread = tracker.get_thread(thread_id)
    if not thread:
        print_error(f"Thread [{thread_id}] 不存在")
        return 1

    thread.notes = note
    tracker.upsert(thread)
    print_success(f"Note saved for [{thread_id}]")
    return 0
