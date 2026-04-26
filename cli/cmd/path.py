"""CLI command: path — Research reading path planner."""
from __future__ import annotations

import argparse

from cli._shared import get_db, Colors, colored, print_info, print_success, print_error
from llm.research_path import ResearchPathPlanner, ReadingLevel


def _build_path_parser(subparsers) -> argparse.ArgumentParser:
    """Build the path subcommand parser."""
    p = subparsers.add_parser(
        "path",
        help="Generate optimal reading path from your paper library",
        description="Plan research reading order based on citation graph and relevance.",
    )
    p.add_argument(
        "topic",
        nargs="?",
        default=None,
        help="Research topic to explore",
    )
    p.add_argument(
        "--level", "-l",
        choices=["intro", "intermediate", "advanced"],
        default="intermediate",
        help="Reading level (default: intermediate)",
    )
    p.add_argument(
        "--max", "-n",
        type=int,
        default=8,
        help="Maximum papers to recommend (default: 8)",
    )
    p.add_argument(
        "--min-year", type=int, default=None,
        help="Minimum publication year",
    )
    p.add_argument(
        "--max-year", type=int, default=None,
        help="Maximum publication year",
    )
    p.add_argument(
        "--mermaid", "-m",
        action="store_true",
        help="Output as Mermaid diagram",
    )
    p.add_argument(
        "--interactive", "-i",
        action="store_true",
        help="Interactive exploration mode",
    )
    return p


def _run_path(args: argparse.Namespace) -> int:
    """Run path planning command."""
    from kg.manager import KGManager

    db = get_db()
    db.init()

    kg = KGManager()

    planner = ResearchPathPlanner(kg_manager=kg, db=db)

    # Interactive mode
    if args.interactive or not args.topic:
        return _run_interactive(planner, args)

    # Single topic query
    level_map = {
        "intro": ReadingLevel.INTRO,
        "intermediate": ReadingLevel.INTERMEDIATE,
        "advanced": ReadingLevel.ADVANCED,
    }
    level = level_map.get(args.level, ReadingLevel.INTERMEDIATE)

    print_info(f"📊 Planning reading path for: {args.topic}")
    print_info(f"   Level: {args.level} | Max papers: {args.max}")

    path = planner.plan_path(
        topic=args.topic,
        level=level,
        max_papers=args.max,
        min_year=args.min_year,
        max_year=args.max_year,
    )

    if args.mermaid:
        print(planner.render_mermaid(path))
    else:
        print()
        print(planner.render_path(path))

    return 0


def _run_interactive(planner: ResearchPathPlanner, args: argparse.Namespace) -> int:
    """Interactive path exploration mode."""
    print(f"{Colors.CYAN}📚 Research Path Planner{Colors.END}")
    print("  输入 topic 开始规划阅读路径")
    print("  输入 level [intro|intermediate|advanced] 设置难度")
    print("  输入 max [N] 设置最大论文数")
    print("  输入 mermaid 显示图")
    print("  输入 q/quit 退出")
    print()

    current_level = ReadingLevel.INTERMEDIATE
    current_max = 8

    while True:
        try:
            user_input = input(f"{Colors.GREEN}❯ {Colors.END}").strip()
        except (EOFError, KeyboardInterrupt):
            print()
            break

        if not user_input:
            continue

        cmd = user_input.lower()

        if cmd in ("q", "quit", "exit"):
            break

        if cmd.startswith("level "):
            level_str = cmd.split(maxsplit=1)[1]
            level_map = {
                "intro": ReadingLevel.INTRO,
                "intermediate": ReadingLevel.INTERMEDIATE,
                "advanced": ReadingLevel.ADVANCED,
            }
            if level_str in level_map:
                current_level = level_map[level_str]
                print(f"  ✓ 难度设置为: {level_str}")
            else:
                print(f"  ✗ 未知难度，可选: intro, intermediate, advanced")
            continue

        if cmd.startswith("max "):
            try:
                current_max = int(cmd.split(maxsplit=1)[1])
                print(f"  ✓ 最大论文数设置为: {current_max}")
            except ValueError:
                print(f"  ✗ 无效数字")
            continue

        if cmd == "mermaid":
            print("  请先输入 topic 进行规划")
            continue

        # Treat as topic
        topic = user_input
        print()
        print(f"{Colors.CYAN}📊 Planning: {topic}{Colors.END}")

        path = planner.plan_path(
            topic=topic,
            level=current_level,
            max_papers=current_max,
        )

        if not path.steps:
            print(f"{Colors.YELLOW}  未找到相关论文{Colors.END}")
        else:
            print()
            print(planner.render_path(path))
        print()

    return 0
