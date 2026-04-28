"""CLI command: chat — RAG Chat with your paper library."""
from __future__ import annotations

import argparse
import datetime
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
    p.add_argument(
        "--stream", action="store_true",
        help="Stream the response as it generates (for interactive mode)",
    )
    p.add_argument(
        "--export", "-e", metavar="FILE",
        help="Export chat history to Markdown file",
    )
    p.add_argument(
        "--session", "-s", metavar="ID",
        help="Continue from a saved chat session",
    )
    return p


def _run_chat(args: argparse.Namespace) -> int:
    """Run the chat command."""
    from cli._shared import load_dotenv
    load_dotenv()

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
        if args.stream:
            # Streaming mode
            print(colored("═" * 60, Colors.OKBLUE))
            print(colored("🔄 流式输出中...\n", Colors.WARNING))

            # Retrieve contexts first
            contexts = chat._retrieve(args.question, args.paper, args.concept, args.limit)
            if not contexts:
                print_error("未找到相关论文")
                return 1

            from llm.client import stream_llm_chat_completions
            from llm.chat import _RAG_SYSTEM_PROMPT
            from llm.constants import LLM_BASE_URL

            prompt = chat._build_prompt(args.question, contexts)
            answer = ""

            for delta in stream_llm_chat_completions(
                [],
                model=chat.model,
                user_prompt=prompt,
                base_url=chat.base_url or LLM_BASE_URL,
                api_key=chat.api_key,
                system_prompt=_RAG_SYSTEM_PROMPT,
            ):
                print(delta, end="", flush=True)
                answer += delta
            print()
            print(colored("═" * 60, Colors.OKBLUE))

            # Extract citations
            citations = chat._extract_citations(contexts)

            # Print citations
            if not args.no_cite and citations:
                print()
                print(colored("📖 引用来源", Colors.HEADER))
                print("-" * 60)
                for i, cite in enumerate(citations, 1):
                    print(f"\n[{i}] {cite.paper_title}")
                    print(f"    ID: {cite.paper_id}")
                    print(f"    相关度: {cite.relevance_score:.2f}")
                    print(f"    > {cite.snippet[:150]}...")

            _show_suggestions_by_context(args.question, citations)
            return 0

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

        # Export if requested
        if args.export:
            history = [{
                "question": args.question,
                "answer": result.answer,
                "citations": result.citations or [],
            }]
            export_chat_to_markdown(history, args.export)
            print(colored(f"✓ 已导出到 {args.export}", Colors.OKGREEN))

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
    session_id = args.session

    # Load previous session if specified
    if session_id:
        try:
            prev_messages = chat.db.get_chat_messages(session_id)
            if prev_messages:
                print(colored(f"📂 已加载会话 {session_id}（{len(prev_messages)} 条消息）\n", Colors.OKBLUE))
                for msg in prev_messages:
                    role = "❓" if msg["role"] == "user" else "🤖"
                    print(colored(f"{role} {msg['content'][:80]}...", Colors.OKBLUE))
                    history.append({"question": msg["content"], "answer": "", "citations": []})
                print()
        except Exception:
            print(colored("⚠️ 无法加载指定会话，将创建新会话\n", Colors.WARNING))

    # Create session if not loading
    if not session_id:
        import uuid
        session_id = str(uuid.uuid4())[:8]
        chat.db.create_chat_session(session_id, "新对话")

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
            # 被动反馈：用户退出，记录中性信号
            if history:
                last = history[-1]
                _record_passive_feedback(
                    last["question"],
                    [c.paper_id for c in last.get("citations", [])],
                    "exited"
                )
                # Auto-export if file specified
                if args.export:
                    export_chat_to_markdown(history, args.export)
                    print(colored(f"✓ 对话已导出到 {args.export}\n", Colors.OKGREEN))
                else:
                    # Prompt to export
                    try:
                        export_choice = input(
                            colored("导出对话到 Markdown？(y/n): ", Colors.OKBLUE)
                        ).strip().lower()
                        if export_choice == "y":
                            default_path = f"chat_export_{datetime.now().strftime('%Y%m%d_%H%M%S')}.md"
                            export_chat_to_markdown(history, default_path)
                            print(colored(f"✓ 对话已导出到 {default_path}\n", Colors.OKGREEN))
                    except (EOFError, KeyboardInterrupt):
                        pass
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
            if args.stream:
                # Streaming mode
                contexts = chat._retrieve(question, args.paper, args.concept, args.limit)
                if not contexts:
                    print_error("未找到相关论文")
                    return

                from llm.client import stream_llm_chat_completions
                from llm.chat import _RAG_SYSTEM_PROMPT
                from llm.constants import LLM_BASE_URL

                print(colored("💡 回答：", Colors.HEADER))
                print("─" * 60)

                prompt = chat._build_prompt(question, contexts)
                answer = ""

                for delta in stream_llm_chat_completions(
                    [],
                    model=chat.model,
                    user_prompt=prompt,
                    base_url=chat.base_url or LLM_BASE_URL,
                    api_key=chat.api_key,
                    system_prompt=_RAG_SYSTEM_PROMPT,
                ):
                    print(delta, end="", flush=True)
                    answer += delta
                print()
                print("─" * 60)

                citations = chat._extract_citations(contexts)
                _record_passive_feedback(question, [c.paper_id for c in citations] if citations else [], "continued")
                _show_suggestions_by_context(question, citations)
                print()
                history.append({"question": question, "answer": answer, "citations": citations})
                continue

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

            # Record feedback - 用户继续追问说明回答有帮助
            _record_passive_feedback(
                question,
                [c.paper_id for c in result.citations] if result.citations else [],
                "continued"
            )

            # Show smart follow-up suggestions
            _show_suggestions(result, question=question)

            print()

            # Save to history
            history.append({
                "question": question,
                "answer": result.answer,
                "citations": result.citations,
            })

            # Persist to database
            try:
                citations_data = [
                    {"paper_id": c.paper_id, "title": c.paper_title, "score": c.relevance_score}
                    for c in result.citations
                ] if result.citations else []
                chat.db.add_chat_message(session_id, "user", question, [])
                chat.db.add_chat_message(session_id, "assistant", result.answer, citations_data)
            except Exception:
                pass  # Non-critical, don't fail on DB errors

        except Exception as e:
            warnings.warn(f"Chat failed: {e}")
            print_error(f"⚠️ 回答失败: {e}")
            if args.verbose:
                import traceback
                traceback.print_exc()
            print()

    return 0


def _collect_feedback(question: str, result, chat, auto_mode: bool = True) -> None:
    """Collect user feedback after a chat response.

    Args:
        auto_mode: 如果 True，不询问用户，自动推断反馈
    """
    try:
        from llm.evolution import get_evolution_memory
        evo = get_evolution_memory()

        # Get paper IDs from citations
        paper_ids = [c.paper_id for c in result.citations] if result.citations else []

        if auto_mode:
            # 自动模式：推断用户是否继续追问来判断满意度
            # 由调用方在用户继续追问时传入 "continued"
            return

        # 手动模式：询问用户
        print()
        feedback = input(colored("这个回答有帮助吗？(y/n/q跳过): ", Colors.OKBLUE)).strip().lower()

        if feedback == "y":
            print(colored("  ✅ 感谢反馈！系统正在学习...", Colors.OKGREEN))
            evo.record_chat_feedback(
                query=question,
                paper_ids=paper_ids,
                is_positive=True,
                outcome="success",
                score=0.8,
            )
        elif feedback == "n":
            print(colored("  📝 记录负面反馈，系统会避免类似回答", Colors.WARNING))
            evo.record_chat_feedback(
                query=question,
                paper_ids=paper_ids,
                is_positive=False,
                outcome="partial",
                score=0.3,
            )

    except Exception:
        # Silently skip feedback collection on error
        pass


def _record_passive_feedback(query: str, paper_ids: List[str], action: str) -> None:
    """Record passive feedback based on user behavior.

    用户行为推断：
    - "continued": 用户继续追问 → 正面
    - "exited": 用户退出 → 中性
    """
    try:
        from llm.evolution import get_evolution_memory
        evo = get_evolution_memory()
        evo.infer_passive_feedback(query, paper_ids, action)
    except Exception:
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


def _show_suggestions_by_context(question: str, citations) -> None:
    """Show suggestions from citations (for streaming mode)."""
    if not citations:
        return
    try:
        from llm.evolution_report import get_smart_followup
        followup = get_smart_followup()
        # Convert citations to ChatContext-like objects
        ctx_list = [
            type('Ctx', (), {
                'paper_id': c.paper_id,
                'paper_title': c.paper_title,
                'authors': c.authors,
                'published': c.published,
                'snippet': c.snippet,
                'relevance_score': c.relevance_score
            }) for c in citations
        ]
        options = followup.generate_options(
            question=question,
            answer="",
            citations=ctx_list,
        )
        if options:
            print()
            print(colored(followup.render_options(options), Colors.HEADER))
    except Exception:
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


def export_chat_to_markdown(history: List[dict], filepath: str) -> bool:
    """Export chat history to a Markdown file.

    Args:
        history: List of chat messages with question, answer, citations
        filepath: Output file path

    Returns:
        True if export succeeded
    """
    import datetime

    try:
        with open(filepath, "w", encoding="utf-8") as f:
            # Header
            f.write("# AI Research OS — Chat Export\n\n")
            f.write(f"**导出时间**: {datetime.datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n\n")
            f.write("---\n\n")

            # Each Q&A pair
            for i, entry in enumerate(history, 1):
                f.write(f"## Q{i}: {entry.get('question', '')}\n\n")

                answer = entry.get('answer', '')
                if answer:
                    f.write(f"**A**: {answer}\n\n")

                citations = entry.get('citations', [])
                if citations:
                    f.write("### 引用来源\n\n")
                    for j, cite in enumerate(citations, 1):
                        title = getattr(cite, 'paper_title', 'Unknown')
                        pid = getattr(cite, 'paper_id', '')
                        score = getattr(cite, 'relevance_score', 0)
                        snippet = getattr(cite, 'snippet', '')[:200]
                        f.write(f"**[{j}] {title}**  \n")
                        f.write(f"ID: `{pid}` | 相关度: {score:.2f}\n\n")
                        if snippet:
                            f.write(f"> {snippet}...\n\n")

                f.write("---\n\n")

        return True
    except Exception as e:
        print_error(f"导出失败: {e}")
        return False
