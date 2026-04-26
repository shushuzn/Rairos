"""CLI command: validate — Research question validation."""
from __future__ import annotations

import argparse

from cli._shared import get_db, print_info
from llm.question_validator import QuestionValidator


def _build_validate_parser(subparsers) -> argparse.ArgumentParser:
    """Build the validate subcommand parser."""
    p = subparsers.add_parser(
        "validate",
        help="Validate novelty of research questions",
        description="Analyze a research question's novelty and feasibility.",
    )
    p.add_argument(
        "question",
        nargs="?",
        default=None,
        help="Research question to validate",
    )
    p.add_argument(
        "--no-llm",
        action="store_true",
        help="Disable LLM analysis (rule-based only)",
    )
    p.add_argument(
        "--json", "-j",
        action="store_true",
        help="Output as JSON",
    )
    p.add_argument(
        "--depth", "-d",
        choices=["quick", "full"],
        default="quick",
        help="Analysis depth (default: quick)",
    )
    p.add_argument(
        "--model", "-m",
        type=str,
        default=None,
        help="LLM model to use",
    )
    p.add_argument(
        "--interactive", "-i",
        action="store_true",
        help="Interactive exploration mode",
    )
    return p


def _run_validate(args: argparse.Namespace) -> int:
    """Run question validation command."""
    db = get_db()
    db.init()

    validator = QuestionValidator(db=db)

    # Interactive mode
    if args.interactive or not args.question:
        return _run_interactive(validator, args)

    # Single question validation
    print_info(f"🔬 Validating: {args.question}")

    result = validator.validate(
        question=args.question,
        use_llm=not args.no_llm,
        model=args.model,
        depth=args.depth,
    )

    if args.json:
        print(validator.render_json(result))
    else:
        print()
        print(validator.render_result(result))

    return 0


def _run_interactive(validator: QuestionValidator, args: argparse.Namespace) -> int:
    """Interactive validation mode."""
    print("🔬 Research Question Validator")
    print("  输入研究问题开始验证")
    print("  输入 no-llm 切换 LLM 分析")
    print("  输入 depth quick/full 切换分析深度")
    print("  输入 json 切换 JSON 输出")
    print("  输入 q/quit 退出")
    print()

    use_llm = True
    depth = "quick"
    use_json = False

    while True:
        try:
            user_input = input("❯ ").strip()
        except (EOFError, KeyboardInterrupt):
            print()
            break

        if not user_input:
            continue

        cmd = user_input.lower()

        if cmd in ("q", "quit", "exit"):
            break

        if cmd == "no-llm":
            use_llm = not use_llm
            status = "禁用" if not use_llm else "启用"
            print(f"  ✓ LLM 分析已{status}")
            continue

        if cmd == "json":
            use_json = not use_json
            status = "启用" if use_json else "禁用"
            print(f"  ✓ JSON 输出已{status}")
            continue

        if cmd in ("depth quick", "quick"):
            depth = "quick"
            print("  ✓ 分析深度: quick")
            continue

        if cmd in ("depth full", "full"):
            depth = "full"
            print("  ✓ 分析深度: full")
            continue

        # Treat as question
        question = user_input
        print()
        print(f"🔬 Validating: {question[:60]}...")
        print(f"   LLM: {'启用' if use_llm else '禁用'} | 深度: {depth}")

        result = validator.validate(
            question=question,
            use_llm=use_llm,
            depth=depth,
        )

        if use_json:
            print(validator.render_json(result))
        else:
            print(validator.render_result(result))

        print()

    return 0
