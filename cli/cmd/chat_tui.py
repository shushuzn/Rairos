"""
Hermes-style TUI Chat: Full-screen RAG chat with paper context sidebar.

Layout:
┌──────────────────────────────────────────────────────────────────┐
│  AI Research OS Chat                               [?] [q]       │
├────────────────────────────────────────┬─────────────────────────┤
│                                        │ 📚 相关论文             │
│  [User message]                       │                         │
│                                        │ ▶ Paper Title 1        │
│  [AI response]                        │   Score: 0.92          │
│  [streaming...]                       │   BERT: Pre-training... │
│                                        │                         │
│                                        │ ▶ Paper Title 2        │
│                                        │   Score: 0.87          │
│                                        │   ...                   │
│                                        │                         │
│                                        │ [+3 more] (if >3)       │
├────────────────────────────────────────┴─────────────────────────┤
│  ❯ [Type your question...                          ] [Enter ⏎]  │
└──────────────────────────────────────────────────────────────────┘
"""
from __future__ import annotations

import argparse
import os
import sys
from dataclasses import dataclass
from typing import List, Optional

# Load .env from current working directory (unified via cli._shared)
from cli._shared import load_dotenv
load_dotenv()

from textual.app import App, ComposeResult
from textual.binding import Binding
from textual.containers import Container, Horizontal, VerticalScroll
from textual.events import Key
from textual.widgets import Button, Header, Label, Static, Input

from cli._shared import get_db, Colors, colored
from llm.chat import RagChat
from llm.friction_tracker import FrictionTracker


# ─── Message data ──────────────────────────────────────────────────────────────


@dataclass
class ChatMessage:
    """A single chat message."""
    role: str          # "user" or "assistant"
    content: str
    citations: list    # List[Citation]
    timestamp: str = ""


# ─── Widgets ──────────────────────────────────────────────────────────────────


class ChatMessageView(Static):
    """A single chat message bubble."""

    def __init__(self, msg: ChatMessage, **kwargs):
        self.msg = msg
        super().__init__(**kwargs)

    def render(self) -> str:
        if self.msg.role == "user":
            prefix = colored("❯ ", Colors.OKGREEN)
            border = colored("─" * 40, Colors.OKGREEN)
            return f"{border}\n{prefix}{self.msg.content}"
        else:
            prefix = colored("🤖 ", Colors.OKBLUE)
            border = colored("─" * 40, Colors.OKBLUE)
            content = self.msg.content or colored("[ 流式响应中... ]", Colors.WARNING)
            return f"{border}\n{prefix}{content}"


class PaperCard(Static):
    """A paper card in the sidebar."""

    def __init__(self, citation, index: int, **kwargs):
        self.citation = citation
        self.index = index
        super().__init__(**kwargs)

    def render(self) -> str:
        score = getattr(self.citation, 'relevance_score', 0)
        title = getattr(self.citation, 'paper_title', 'Unknown')
        snippet = getattr(self.citation, 'snippet', '')[:80]
        pid = getattr(self.citation, 'paper_id', '')
        return (
            f"  ▶ {title}\n"
            f"    [{pid}] score={score:.2f}\n"
            f"    {snippet}..."
        )


class SidebarPaperList(VerticalScroll):
    """Scrollable paper list in the sidebar."""

    def __init__(self, citations: List, **kwargs):
        self._citations = citations[:3]
        self._extra = len(citations) - 3
        super().__init__(**kwargs)

    def compose(self) -> ComposeResult:
        yield Static(colored("📚 相关论文", Colors.HEADER + Colors.BOLD),
                     classes="sidebar-title")
        for i, c in enumerate(self._citations):
            yield PaperCard(c, i, classes="paper-card")
        if self._extra > 0:
            yield Static(
                colored(f"  [+{self._extra} more papers]", Colors.WARNING),
                classes="more-papers"
            )
        yield Label("")  # spacer


# ─── Main TUI App ────────────────────────────────────────────────────────────


class TUIChatApp(App):
    """Full-screen RAG chat with paper context sidebar."""

    TITLE = "AI Research OS Chat"
    SUB_TITLE = "RAG Chat with your paper library"

    CSS = """
    Screen {
        background: $surface;
    }

    /* ── Header ── */
    Header {
        background: #1a1a2e;
    }

    /* ── Main layout ── */
    #chat-area {
        width: 65%;
        height: 100%;
        background: #0f0f1a;
        border: solid #2a2a4a;
        padding: 0 0 0 0;
    }

    #sidebar {
        width: 35%;
        height: 100%;
        background: #12122a;
        border: solid #2a2a4a;
        padding: 0 0 0 0;
    }

    #sidebar-title {
        color: #a0a0ff;
        text-style: bold;
        padding: 1 2;
        background: #1a1a3a;
    }

    .paper-card {
        color: #c0c0e0;
        padding: 1 2;
        border: solid #3a3a6a;
        margin: 0 0 1 0;
        background: #16163a;
    }

    .more-papers {
        color: #8080cc;
        padding: 1 2;
    }

    /* ── Message area ── */
    #messages {
        width: 100%;
        height: 100%;
        padding: 1 2;
    }

    /* ── Input area ── */
    #input-area {
        height: 6;
        background: #1a1a2e;
        border-top: solid #3a3a6a;
        padding: 1 2;
    }

    Input {
        margin: 0 1;
    }

    #send-btn {
        color: #00cc88;
    }

    /* ── Status ── */
    #status-bar {
        background: #0a0a1a;
        color: #6060a0;
        padding: 0 2;
    }

    /* ── Suggestions ── */
    .suggestions {
        color: #80a0ff;
        padding: 1 2;
        background: #0a1525;
        border: solid #2040a0;
        margin: 1 0;
    }
    """

    BINDINGS = [
        Binding("q", "quit", "Quit", show=True),
        Binding("ctrl+c", "quit", "", show=False),
        Binding("ctrl+l", "clear", "Clear", show=False),
        Binding("f1", "help", "Help", show=False),
    ]

    def __init__(
        self,
        chat: RagChat,
        concept: str = None,
        limit: int = 5,
        friction_tracker: FrictionTracker = None,
        stream: bool = True,
        **kwargs,
    ):
        super().__init__(**kwargs)
        self.chat = chat
        self.concept = concept
        self.limit = limit
        self.stream = stream
        self.friction = friction_tracker or FrictionTracker()
        self.messages: List[ChatMessage] = []
        self.pending_citations: List = []
        self._streaming = False
        self._chat_history: List[dict] = []  # For follow-up question context

    # ── App lifecycle ──────────────────────────────────────────────────────

    def compose(self) -> ComposeResult:
        yield Header(show_clock=False)
        with Horizontal(id="main-row"):
            with Container(id="chat-area"):
                with VerticalScroll(id="messages"):
                    yield Static(
                        colored("📚 AI Research OS — RAG Chat\n", Colors.HEADER + Colors.BOLD) +
                        colored("   对你的论文库进行自然语言问答，带引用溯源\n", Colors.OKBLUE) +
                        colored("   Enter 发送 · Shift+Enter 换行 · q 退出 · Ctrl+L 清屏\n", Colors.WARNING),
                        id="welcome"
                    )
            with Container(id="sidebar"):
                yield Static(colored("📚 相关论文", Colors.HEADER + Colors.BOLD),
                            id="sidebar-title")
                yield SidebarPaperList([], id="paper-list")
        with Container(id="input-area"):
            yield Input(
                placeholder="输入问题后按 Enter 发送...",
                id="chat-input",
                classes="chat-input",
            )
            yield Button("Send", id="send-btn", variant="primary")
        yield Static("❯ 输入问题开始对话", id="status-bar")

    def on_mount(self) -> None:
        self.query_one("#chat-input").focus()

    # ── Input handling ───────────────────────────────────────────────────────

    def on_input_submitted(self, event: Input.Submitted) -> None:
        if self._streaming:
            return
        question = event.value.strip()
        if not question:
            return
        self._handle_submit(question)
        self.query_one("#chat-input").value = ""

    def _handle_submit(self, question: str) -> None:
        """Process a user question."""
        # Rewrite follow-up questions using LLM
        rewritten_question = question
        if self._chat_history:
            try:
                rewritten_question = self.chat._rewrite_followup(question, self._chat_history)
            except Exception:
                pass  # Fall back to original question

        # Add user message
        user_msg = ChatMessage(role="user", content=question, citations=[])
        self.messages.append(user_msg)
        self._render_messages()

        # Set streaming state
        self._streaming = True
        self._update_status("🔍 检索中...")

        # Build AI placeholder
        ai_msg = ChatMessage(role="assistant", content="", citations=[])
        self.messages.append(ai_msg)
        self._render_messages()

        try:
            if self.stream:
                # Streaming mode: accumulate response incrementally
                self._update_status("🤖 生成回答中...")
                # Import streaming function
                from llm.client import stream_llm_chat_completions
                from llm.chat import _RAG_SYSTEM_PROMPT

                # First retrieve contexts using rewritten question
                contexts = self.chat._retrieve(rewritten_question, None, self.concept, self.limit)
                if not contexts:
                    # Fallback: use general LLM without paper context
                    if self.chat.api_key:
                        self._update_status("🤖 生成回答中...")
                        answer = ""
                        for delta in stream_llm_chat_completions(
                            [],
                            model=self.chat.model,
                            user_prompt=rewritten_question,
                            base_url=self.chat.base_url,
                            api_key=self.chat.api_key,
                            system_prompt="你是一个有帮助的 AI 助手。用中文简洁回答问题。",
                        ):
                            answer += delta
                            ai_msg.content = answer
                            self._render_messages()
                        ai_msg.content = answer
                    else:
                        ai_msg.content = "⚠️ 未找到相关论文，且未配置 API Key"
                        self._update_status("⚠️ 无相关论文")
                    return

                # Build prompt
                prompt = self.chat._build_prompt(rewritten_question, contexts)
                answer = ""

                # Stream the response
                for delta in stream_llm_chat_completions(
                    [],
                    model=self.chat.model,
                    user_prompt=prompt,
                    base_url=self.chat.base_url,
                    api_key=self.chat.api_key,
                    system_prompt=_RAG_SYSTEM_PROMPT,
                ):
                    answer += delta
                    # Update display incrementally
                    ai_msg.content = answer
                    self._render_messages()

                ai_msg.content = answer
                ai_msg.citations = self.chat._extract_citations(contexts)
            # Save to history for follow-up context (use rewritten question)
            self._chat_history.append({
                "question": rewritten_question,
                "answer": ai_msg.content,
                "citations": ai_msg.citations,
            })

            # Also show suggestions if available
            self._show_suggestions(ai_msg.citations)

            self.pending_citations = ai_msg.citations
            self._update_sidebar(ai_msg.citations)
            self._update_status(f"✅ 回复完成 · {len(ai_msg.citations)} 篇引用")

        except Exception as e:
            ai_msg.content = colored(f"⚠️ 出错了: {e}", Colors.FAIL)
            self._update_status(colored(f"⚠️ 错误: {e}", Colors.FAIL))
            # Record friction
            self.friction.record_retrieval_failure("chat_tui", question, notes=str(e))

        finally:
            self._streaming = False
            self._render_messages()
            self.scroll_messages_to_bottom()

    def _render_messages(self) -> None:
        """Re-render all messages in the messages container."""
        container = self.query_one("#messages")
        # Remove old message widgets (keep welcome)
        for w in container.query("ChatBubble"):
            w.remove()

        for msg in self.messages:
            bubble = ChatBubble(msg)
            container.mount(bubble)

    def scroll_messages_to_bottom(self) -> None:
        container = self.query_one("#messages")
        container.scroll_end(animate=True)

    def _update_sidebar(self, citations) -> None:
        """Refresh the paper sidebar."""
        sidebar = self.query_one("#paper-list")
        for child in sidebar.query("*"):
            child.remove()
        for c in citations[:3]:
            sidebar.mount(PaperCard(c, 0, classes="paper-card"))
        if len(citations) > 3:
            sidebar.mount(Static(
                colored(f"  [+{len(citations)-3} more papers]", Colors.WARNING),
                classes="more-papers"
            ))

    def _update_status(self, text: str) -> None:
        status = self.query_one("#status-bar")
        status.update(text)

    # ── Actions ────────────────────────────────────────────────────────────

    def action_quit(self) -> None:
        self.exit(0)

    def action_clear(self) -> None:
        self.messages.clear()
        self._chat_history.clear()
        self._render_messages()
        welcome = self.query_one("#welcome")
        self.query_one("#messages").mount(welcome)
        self._update_status("对话已清除")

    def action_help(self) -> None:
        self.notify(
            "Enter: 发送消息  |  Shift+Enter: 换行\n"
            "Ctrl+L: 清屏  |  q: 退出",
            title="快捷键",
            timeout=5,
        )

    def _show_suggestions(self, citations) -> None:
        """Show follow-up question suggestions."""
        if not citations:
            return
        try:
            from llm.evolution_report import get_smart_followup
            followup = get_smart_followup()
            last_q = self._chat_history[-1]["question"] if self._chat_history else ""

            # Convert citations to context
            ctx_list = [
                type('Ctx', (), {
                    'paper_id': c.paper_id,
                    'paper_title': c.paper_title,
                    'authors': getattr(c, 'authors', []),
                    'published': getattr(c, 'published', ''),
                    'snippet': getattr(c, 'snippet', ''),
                    'relevance_score': getattr(c, 'relevance_score', 0)
                }) for c in citations
            ]

            options = followup.generate_options(
                question=last_q,
                answer="",
                citations=ctx_list,
            )
            if options:
                # Add suggestion widget below last message
                container = self.query_one("#messages")
                container.mount(Static(
                    colored("💡 追问建议：", Colors.WARNING) +
                    colored(followup.render_options(options), Colors.OKBLUE),
                    classes="suggestions",
                ))
        except Exception:
            pass


class ChatBubble(Static):
    """A chat message bubble widget."""

    def __init__(self, msg: ChatMessage, **kwargs):
        self.msg = msg
        super().__init__(**kwargs)

    def compose(self) -> ComposeResult:
        if self.msg.role == "user":
            yield Static(
                colored(f"❯ {self.msg.content}", Colors.OKGREEN),
                classes="user-msg",
            )
        else:
            yield Static(
                colored("🤖 AI", Colors.OKBLUE) + "\n" + self.msg.content,
                classes="ai-msg",
            )
            if self.msg.citations:
                c = self.msg.citations[0]
                title = getattr(c, 'paper_title', '')[:60]
                yield Static(
                    colored(f"  📖 引用: {title}", Colors.WARNING),
                    classes="cite-hint",
                )


# ─── CLI command ──────────────────────────────────────────────────────────────


def _build_chat_tui_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "chat-tui",
        help="Full-screen TUI RAG chat with paper context sidebar (Hermes-style)",
        description="Launch a full-screen terminal chat interface for RAG-powered Q&A.",
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
        "--model", type=str, default=None,
        help="LLM model to use",
    )
    return p


def run(args: argparse.Namespace) -> int:
    """Run the TUI chat."""
    db = get_db()
    db.init()

    # Check API key
    api_key = os.getenv("OPENAI_API_KEY")
    if not api_key:
        print(colored("OPENAI_API_KEY not set.", Colors.FAIL), file=sys.stderr)
        print("  export OPENAI_API_KEY=sk-...", file=sys.stderr)
        return 1

    # Model
    model = args.model
    if not model:
        try:
            from config import DEFAULT_LLM_MODEL_CLI as model
        except Exception:
            model = "gpt-4o-mini"

    # Base URL
    try:
        from config import DEFAULT_OPENAI_BASE_URL as base_url
    except Exception:
        base_url = "https://api.openai.com/v1"

    # Init chat
    chat = RagChat(db=db, api_key=api_key, base_url=base_url, model=model)

    # Launch TUI
    app = TUIChatApp(chat=chat, concept=args.concept, limit=args.limit, stream=True)
    app.run()
    return 0
