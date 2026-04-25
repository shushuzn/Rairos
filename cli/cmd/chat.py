"""CLI command: chat — RAG Chat with your paper library."""
from __future__ import annotations

import argparse
import os
import sys
import warnings
from typing import List, Optional

from cli._shared import get_db, Colors, colored, print_info, print_error


def _build_chat_parser(subparsers) -> argparse.ArgumentParser:
    """Build the chat subcommand parser."""
    p = subparsers.add_parser(
        "chat",
        help="RAG Chat with your paper library",
        description="Ask questions about papers in your library with source citations.",
    )
    p.add_argument(
        "question", nargs="?", default=None,
        help="Question to ask (omit for interactive mode)",
    )
    p.add_argument(
        "--paper", metavar="ID",
        help="Target specific paper by ID",
    )
    p.add_argument(
        "--concept", "-c", metavar="TAG",
        help="Filter by concept/tag",
    )
    p.add_argument(
        "--limit", "-n", type=int, default=5,
        help="Number of papers to retrieve (default: 5)",
    )
    p.add_argument(
        "--interactive", "-i", action="store_true",
        help="Interactive REPL mode",
    )
    p.add_argument(
        "--no-cite", action="store_true",
        help="Hide citations in output",
    )
    p.add_argument(
        "--model", type=str, default=None,
        help="LLM model to use",
    )
    p.add_argument(
        "--verbose", "-v", action="store_true",
        help="Verbose output (show retrieval debug info)",
    )
    return p


def _run_chat(args: argparse.Namespace) -> int:
    """Run the chat command."""
    db = get_db()
    db.init()

    # Check API key
    api_key = os.getenv("OPENAI_API_KEY")
    if not api_key:
        print_error("OPENAI_API_KEY not set. Please set it to enable chat functionality.")
        print_info("  export OPENAI_API_KEY=sk-...")
        return 1

    # Import LLM chat
    try:
        from llm.chat import RagChat, ChatResult
    except ImportError as e:
        print_error(f"Failed to import RAG chat module: {e}")
        return 1

    # Get model from args or config
    model = args.model
    if not model:
        try:
            from config import DEFAULT_LLM_MODEL_CLI
            model = DEFAULT_LLM_MODEL_CLI
        except Exception:
            model = "gpt-4o-mini"

    # Get base URL
    try:
        from config import DEFAULT_OPENAI_BASE_URL
        base_url = DEFAULT_OPENAI_BASE_URL
    except Exception:
        base_url = "https://api.openai.com/v1"

    # Initialize chat
    chat = RagChat(
        db=db,
        api_key=api_key,
        base_url=base_url,
        model=model,
    )

    # Interactive mode
    if args.interactive or args.question is None:
        return _run_interactive(chat, args)

    # Single question
    return _run_single_question(chat, args)


def _run_single_question(chat, args) -> int:
    """Run a single question."""
    try:
        result = chat.chat(
            question=args.question,
            paper_id=args.paper,
            concept=args.concept,
            limit=args.limit,
            verbose=args.verbose,
        )

        # Print answer
        print(colored("═" * 60, Colors.OKBLUE))
        print(result.answer)
        print(colored("═" * 60, Colors.OKBLUE))

        # Print citations
        if not args.no_cite and result.citations:
            print()
            print(colored("📖 引用来源", Colors.HEADER))
            print("-" * 60)
            for i, cite in enumerate(result.citations, 1):
                print(f"\n[{i}] {cite.paper_title}")
                print(f"    ID: {cite.paper_id}")
                print(f"    相关度: {cite.relevance_score:.2f}")
                print(f"    > {cite.snippet[:150]}...")

        # Show suggested follow-up questions
        _show_suggestions(result, question=args.question)

        return 0

    except Exception as e:
        print_error(f"Chat failed: {e}")
        if args.verbose:
            import traceback
            traceback.print_exc()
        return 1


def _run_interactive(chat, args) -> int:
    """Run interactive REPL mode."""
    print(colored("╔════════════════════════════════════════════════════════╗", Colors.OKBLUE))
    print(colored("║     📚 AI Research OS — RAG Chat                       ║", Colors.OKBLUE))
    print(colored("╠════════════════════════════════════════════════════════╣", Colors.OKBLUE))
    print(colored("║  对你的论文库进行自然语言问答，带引用溯源            ║", Colors.OKBLUE))
    print(colored("╚════════════════════════════════════════════════════════╝", Colors.OKBLUE))
    print()
    print(colored("命令：", Colors.HEADER))
    print("  q / quit / exit    退出")
    print("  clear              清除对话历史")
    print("  help               显示帮助")
    print()
    print(colored("提示：", Colors.WARNING))
    print("  输入问题即可获得基于论文内容的回答")
    print("  使用 --paper 指定论文，--concept 过滤主题")
    print()

    history: List[dict] = []

    while True:
        try:
            question = input(colored("❓ ", Colors.OKGREEN)).strip()
        except (EOFError, KeyboardInterrupt):
            print("\n\n再会！")
            break

        if not question:
            continue

        # Handle commands
        cmd = question.lower()
        if cmd in ("q", "quit", "exit", "exit()"):
            print(colored("\n再会！记得回来继续探索你的论文库 🚀\n", Colors.OKBLUE))
            break

        if cmd == "clear":
            history = []
            print(colored("✓ 对话历史已清除\n", Colors.OKGREEN))
            continue

        if cmd == "help":
            print(colored("\n帮助：", Colors.HEADER))
            print("  直接输入问题即可获得回答")
            print("  示例：")
            print("    self-attention 是如何工作的？")
            print("    这篇论文的贡献是什么？")
            print("    RAG 和 fine-tuning 有什么区别？")
            print()
            continue

        # Process question
        print(colored("\n🔍 检索中...\n", Colors.WARNING))

        try:
            result = chat.chat(
                question=question,
                paper_id=args.paper,
                concept=args.concept,
                limit=args.limit,
                verbose=args.verbose,
            )

            # Print answer
            print(colored("💡 回答：", Colors.HEADER))
            print("─" * 60)
            print(result.answer)
            print("─" * 60)

            # Print citations
            if not args.no_cite and result.citations:
                print()
                print(colored("📖 引用来源", Colors.HEADER))
                for i, cite in enumerate(result.citations, 1):
                    print(f"  [{i}] {cite.paper_title}")
                    print(f"      ID: {cite.paper_id} | 相关度: {cite.relevance_score:.2f}")

            # Record feedback
            _collect_feedback(question, result, chat)

            # Show smart follow-up suggestions
            _show_suggestions(result, question=question)

            print()

            # Save to history
            history.append({
                "question": question,
                "answer": result.answer,
                "citations": result.citations,
            })

        except Exception as e:
            warnings.warn(f"Chat failed: {e}")
            print_error(f"⚠️ 回答失败: {e}")
            if args.verbose:
                import traceback
                traceback.print_exc()
            print()

    return 0


def _collect_feedback(question: str, result, chat) -> None:
    """Collect user feedback after a chat response."""
    try:
        from llm.evolution import get_evolution_memory

        # Get paper IDs from citations
        paper_ids = [c.paper_id for c in result.citations] if result.citations else []

        # Ask for feedback
        print()
        feedback = input(colored("这个回答有帮助吗？(y/n/q跳过): ", Colors.OKBLUE)).strip().lower()

        if feedback == "y":
            print(colored("  ✅ 感谢反馈！系统正在学习...", Colors.OKGREEN))
            evo = get_evolution_memory()
            evo.record_chat_feedback(
                query=question,
                paper_ids=paper_ids,
                is_positive=True,
                outcome="success",
                score=0.8,
            )
        elif feedback == "n":
            print(colored("  📝 记录负面反馈，系统会避免类似回答", Colors.WARNING))
            evo = get_evolution_memory()
            evo.record_chat_feedback(
                query=question,
                paper_ids=paper_ids,
                is_positive=False,
                outcome="partial",
                score=0.3,
            )
        # else: skip (q or anything else)

    except Exception:
        # Silently skip feedback collection on error
        pass


def _show_suggestions(result, question: str = "") -> None:
    """Show smart follow-up questions based on the answer content."""
    if not result.citations and not result.answer:
        return

    try:
        from llm.evolution_report import get_smart_followup
        followup = get_smart_followup()

        options = followup.generate_options(
            question=question,
            answer=result.answer or "",
            citations=result.citations,
        )

        if options:
            print()
            print(colored(followup.render_options(options), Colors.HEADER))
    except Exception:
        # Silently skip suggestions
        pass


def _show_suggestions_legacy(result) -> None:
    """Fallback: show suggested questions from learning history."""
    if not result.citations:
        return

    try:
        from llm.evolution_report import generate_evolution_report
        report = generate_evolution_report(days=30)

        if report.questions_to_explore:
            print()
            print(colored("💡 你可能想问：", Colors.HEADER))
            for q in report.questions_to_explore[:3]:
                print(f"  • {q}")
    except Exception:
        # Silently skip suggestions
        pass
