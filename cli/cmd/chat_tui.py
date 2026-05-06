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
│                                        │ [+3 more] (if >3)      │
├────────────────────────────────────────┴─────────────────────────┤
│  ❯ [Type your question...                          ] [Enter ⏎]  │
└──────────────────────────────────────────────────────────────────┘
"""

from __future__ import annotations

import argparse
import asyncio
import json
import os
import re
import sys
from dataclasses import dataclass
from datetime import datetime
from typing import Any, Callable, Dict, List, Optional, Set, cast

# Load .env from current working directory (unified via cli._shared)
from cli._shared import load_dotenv

load_dotenv()

from textual.app import App, ComposeResult
from textual.binding import Binding
from textual.containers import Container, Horizontal, VerticalScroll
from textual.screen import Screen
from textual.widgets import Button, Header, Label, Static, Input
from textual.css.query import NoMatches

from cli._shared import get_db, Colors, colored
from cli.warp import WarpBlocks
from llm.chat import RagChat
from llm.friction_tracker import FrictionTracker


# ─── Message data ──────────────────────────────────────────────────────────────


@dataclass
class ChatMessage:
    """A single chat message."""

    role: str  # "user" or "assistant"
    content: str
    citations: list  # List[Citation]
    timestamp: str = ""
    edited: bool = False


@dataclass
class StreamConfig:
    """Configuration for streaming behavior."""

    batch_size: int = 3  # Characters per batch update
    max_line_width: int = 120  # Auto-wrap at this width
    typing_indicator: bool = True  # Show typing animation


# ─── Message Indexer ─────────────────────────────────────────────────────────


def tokenize(text: str) -> Set[str]:
    """Extract searchable tokens from text (simple word tokenization)."""
    # Lowercase, keep alphanumeric+Chinese chars, split by whitespace/punctuation
    tokens = re.findall(r"[\w\u4e00-\u9fff]+", text.lower())
    # Filter out very short tokens (1-2 chars often noise)
    return {t for t in tokens if len(t) >= 2}


def _index_message(tokens: Set[str], msg_idx: int, index: Dict[str, Set[int]]) -> None:
    """Add message tokens to the inverted index."""
    for token in tokens:
        if token not in index:
            index[token] = set()
        index[token].add(msg_idx)


# ─── Markdown Parser ──────────────────────────────────────────────────────────


class SimpleMarkdown:
    """Simple markdown to ANSI-colored text converter for TUI.

    Delegates code-block rendering to shared WarpBlocks to avoid duplication.
    """

    # Re-use WarpBlocks colors for compatibility with existing CSS/text patterns
    COLORS = WarpBlocks.C
    LANG_ALIASES = WarpBlocks.LANG_ALIASES if hasattr(WarpBlocks, "LANG_ALIASES") else {}
    PY_KEYWORDS = WarpBlocks.PY_KEYWORDS if hasattr(WarpBlocks, "PY_KEYWORDS") else frozenset()
    JS_KEYWORDS = WarpBlocks.JS_KEYWORDS if hasattr(WarpBlocks, "JS_KEYWORDS") else frozenset()

    # Delegate tokenizers to shared module
    _tokenize_python = (
        staticmethod(WarpBlocks._tokenize_python)
        if hasattr(WarpBlocks, "_tokenize_python")
        else None
    )
    _tokenize_javascript = (
        staticmethod(WarpBlocks._tokenize_javascript)
        if hasattr(WarpBlocks, "_tokenize_javascript")
        else None
    )
    _tokenize_json = (
        staticmethod(WarpBlocks._tokenize_json) if hasattr(WarpBlocks, "_tokenize_json") else None
    )
    _tokenize_generic = (
        staticmethod(WarpBlocks._tokenize_generic)
        if hasattr(WarpBlocks, "_tokenize_generic")
        else None
    )
    _tokenize = staticmethod(WarpBlocks._tokenize) if hasattr(WarpBlocks, "_tokenize") else None

    @classmethod
    def _render_code_block(cls, lang: str, code: str, width: int = 80) -> str:
        """Render a code block with syntax highlighting and line numbers.

        Delegates to the shared WarpBlocks implementation.
        """
        return WarpBlocks.code_block(lang, code, width=width)

    @classmethod
    def parse(cls, text: str) -> str:
        """Parse markdown to styled text."""
        if not text:
            return text

        result = text

        # Process code blocks first (before other markdown)
        def replace_code_block(m):
            lang = m.group(1).strip() or "text"
            code = m.group(2)
            return "\n" + cls._render_code_block(lang, code) + "\n"

        result = re.sub(r"```(\w*)\n?(.*?)```", replace_code_block, result, flags=re.DOTALL)

        # Inline code (`code`)
        result = re.sub(r"`([^`]+)`", r" [\1] ", result)

        # Headers (# ## ###)
        result = re.sub(r"^### (.+)$", r"\n━━━ \1 ━━━\n", result, flags=re.MULTILINE)
        result = re.sub(r"^## (.+)$", r"\n━━ \1 �━━\n", result, flags=re.MULTILINE)
        result = re.sub(r"^# (.+)$", r"\n━ \1 ━\n", result, flags=re.MULTILINE)

        # Bold (**text** or __text__)
        result = re.sub(r"\*\*(.+?)\*\*", r"[\1]", result)
        result = re.sub(r"__(.+?)__", r"[\1]", result)

        # Italic (*text* or _text_)
        result = re.sub(r"\*(.+?)\*", r"/\1/", result)
        result = re.sub(r"_(.+?)_", r"/\1/", result)

        # Lists (- item or * item)
        result = re.sub(r"^[\-\*] (.+)$", r"  • \1", result, flags=re.MULTILINE)

        # Numbered lists (1. item)
        result = re.sub(r"^\d+\. (.+)$", r"  \g<0>", result, flags=re.MULTILINE)

        # Blockquotes (>)
        result = re.sub(r"^> (.+)$", r"  │ \1", result, flags=re.MULTILINE)

        # Horizontal rules (---)
        result = re.sub(r"^---+$", "─" * 50, result, flags=re.MULTILINE)

        return result

    @classmethod
    def wrap_lines(cls, text: str, width: int = 120) -> str:
        """Wrap text to specified width."""
        lines = text.split("\n")
        wrapped = []
        for line in lines:
            if len(line) <= width:
                wrapped.append(line)
            else:
                # Break at word boundaries
                words = line.split()
                current = ""
                for word in words:
                    if len(current) + len(word) + 1 <= width:
                        current += (" " if current else "") + word
                    else:
                        if current:
                            wrapped.append(current)
                        current = word
                if current:
                    wrapped.append(current)
        return "\n".join(wrapped)


# ─── Timestamp Formatter ──────────────────────────────────────────────────────


class Timestamp:
    """Format timestamps for display."""

    @staticmethod
    def now() -> str:
        """Get current timestamp."""
        return datetime.now().strftime("%H:%M")

    @staticmethod
    def format(ts: str) -> str:
        """Format a timestamp string."""
        if not ts:
            return ""
        try:
            dt = datetime.fromisoformat(ts)
            return dt.strftime("%H:%M")
        except Exception:
            return ts[:5] if len(ts) > 5 else ts


# ─── Loading Animation ──────────────────────────────────────────────────────────


class LoadingDots:
    """Animated loading indicator with braille dots."""

    FRAMES = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]

    def __init__(self):
        self.frame = 0

    def next(self) -> str:
        """Get next frame."""
        frame = self.FRAMES[self.frame % len(self.FRAMES)]
        self.frame += 1
        return frame  # type: ignore[no-any-return]


class TypingCursor:
    """Typing cursor with blink animation."""

    CURSOR_CHARS = ["▌", "█"]  # Block cursor variants
    BLINK_FRAMES = 10  # Frames per blink cycle

    def __init__(self):
        self.frame = 0
        self.visible = True

    def next(self) -> str:
        """Get next cursor state."""
        self.frame += 1
        if self.frame % self.BLINK_FRAMES == 0:
            self.visible = not self.visible
        return self.CURSOR_CHARS[0] if self.visible else " "

    def reset(self) -> None:
        """Reset cursor state."""
        self.frame = 0
        self.visible = True


class Typewriter:
    """Character-by-character typing effect for streaming text."""

    def __init__(self, text: str = "", delay: int = 0):
        self.text = text
        self.index = 0
        self.delay = delay
        self.frame = 0

    def tick(self) -> str | None:
        """Get next characters (up to batch_size) for typewriter effect."""
        self.frame += 1
        if self.frame % max(1, self.delay) != 0:
            return None  # Skip this frame

        batch = min(3, len(self.text) - self.index)
        if batch <= 0:
            return ""

        result = self.text[self.index : self.index + batch]
        self.index += batch
        return result

    def is_done(self) -> bool:
        """Check if typing is complete."""
        return self.index >= len(self.text)

    def progress(self) -> float:
        """Get progress 0.0 to 1.0."""
        if not self.text:
            return 1.0
        return self.index / len(self.text)


# ─── KG Visualizer (utility class, not a Textual widget) ─────────────────────


class KGVisualizer:
    """Render a KG neighborhood as an ASCII directed graph.

    Input: list of (node_id, node_type, label, connections) tuples and
           list of (source_id, target_id, relation_type) edges.
    Output: multi-line ASCII art string.
    """

    MAX_LABEL = 12

    @classmethod
    def _trunc(cls, label: str) -> str:
        if len(label) > cls.MAX_LABEL:
            return label[: cls.MAX_LABEL - 1] + "…"
        return label

    @classmethod
    def _node_str(cls, node_type: str, label: str) -> str:
        short_type = (node_type[:4] + ":") if node_type else "Node:"
        return f"[{short_type}{cls._trunc(label)}]"

    @classmethod
    def render(
        cls,
        nodes: list[tuple],
        edges: list[tuple],
        root_id: str = "",
        max_width: int = 40,
        max_depth: int = 2,
    ) -> str:
        """Render KG as ASCII art using BFS layering."""
        if not nodes:
            return "  (no KG data)"

        # node_id -> (node_type, label)
        node_info: Dict[str, tuple[str, str]] = {n[0]: (n[1], n[2]) for n in nodes}

        # Build adjacency: node_id -> list of (neighbor_id, relation)
        adj: Dict[str, list[tuple[str, str]]] = {n[0]: [] for n in nodes}
        for src, tgt, rel in edges:
            if src in adj and tgt in node_info:
                adj[src].append((tgt, rel))
            if tgt in adj and src in node_info:
                adj[tgt].append((src, rel))

        # BFS from root to collect reachable nodes within max_depth
        if not root_id:
            root_id = nodes[0][0]

        visited: Dict[str, int] = {}
        queue = [(root_id, 0)]
        while queue:
            nid, d = queue.pop(0)
            if nid in visited or d > max_depth:
                continue
            visited[nid] = d
            for neighbor, _ in adj.get(nid, []):
                if neighbor not in visited:
                    queue.append((neighbor, d + 1))

        if not visited:
            return "  (no reachable KG data)"

        # Group by depth level
        levels: Dict[int, list[tuple[str, str]]] = {}
        for nid, depth in visited.items():
            if nid in node_info:
                ntype, label = node_info[nid]
                levels.setdefault(depth, []).append((nid, cls._node_str(ntype, label)))

        lines: list[str] = []

        for depth in sorted(levels.keys()):
            level_nodes = levels[depth]
            if depth == 0:
                lines.append("  " + "  ".join(lbl for _, lbl in level_nodes))
            else:
                n = len(level_nodes)
                # Build connector row
                if n == 1:
                    connector = "  │ " + "─" * (len(level_nodes[0][1]) + 2)
                else:
                    seg = "─┬─"
                    connector = "  │ " + ("─" * len(level_nodes[0][1]) + seg) * (n - 1) + "─" * len(level_nodes[0][1])
                lines.append(connector)
                # Node row
                row = "  │ " + "  │ ".join(lbl for _, lbl in level_nodes)
                lines.append(row)

        return "\n".join(lines) if lines else "  (no KG data)"


# ─── SidebarKGView ───────────────────────────────────────────────────────────────


class SidebarKGView(Static):
    """A toggleable KG ASCII view shown in the sidebar below the paper cards."""

    MAX_LINES = 20

    def __init__(self, on_close=None, **kwargs):
        self.on_close = on_close
        super().__init__(**kwargs)

    def show_kg(self, ascii_graph: str, truncated: int = 0) -> None:
        """Display the ASCII KG graph in this widget."""
        content = ascii_graph
        if truncated > 0:
            content += f"\n  … {truncated} more lines"
        self.update(colored(content, "#8be9fd"))
        self.remove_class("hidden")

    def hide_kg(self) -> None:
        """Hide the KG view."""
        self.add_class("hidden")

    def show_empty(self, message: str) -> None:
        """Show an empty or unavailable state."""
        self.update(colored(f"  {message}", Colors.WARNING))
        self.remove_class("hidden")


# ─── Widgets ──────────────────────────────────────────────────────────────────


class ChatBubble(Static):
    """A chat message bubble widget with markdown support and typing effect."""

    # Streaming indicator frames
    STREAM_FRAMES = ["○", "◔", "◑", "◕", "●"]  # Expanding circles

    def __init__(
        self,
        msg: ChatMessage,
        config: StreamConfig | None = None,
        is_streaming: bool = False,
        **kwargs,
    ):
        self.msg = msg
        self.config = config or StreamConfig()
        self.is_streaming = is_streaming
        self.stream_frame = 0
        super().__init__(**kwargs)

    def get_streaming_indicator(self) -> str:
        """Get animated streaming indicator."""
        if not self.is_streaming:
            return ""
        self.stream_frame = (self.stream_frame + 1) % len(self.STREAM_FRAMES)
        return colored(f" {self.STREAM_FRAMES[self.stream_frame]}", Colors.WARNING)

    def compose(self) -> ComposeResult:
        ts = Timestamp.format(self.msg.timestamp) if self.msg.timestamp else Timestamp.now()

        if self.msg.role == "user":
            content = SimpleMarkdown.wrap_lines(self.msg.content, self.config.max_line_width - 20)
            yield Static(
                colored(f"❯ {content}", Colors.OKGREEN),
                classes="user-msg",
            )
            yield Static(
                colored(f"  {ts}", Colors.OKBLUE + " dim"),
                classes="timestamp",
            )
        else:
            # AI response with markdown
            parsed = SimpleMarkdown.parse(self.msg.content)
            wrapped = SimpleMarkdown.wrap_lines(parsed, self.config.max_line_width - 10)

            # Add streaming indicator
            streaming_suffix = self.get_streaming_indicator() if self.is_streaming else ""

            yield Static(
                colored(f"🤖 {wrapped}{streaming_suffix}", Colors.OKBLUE),
                classes="ai-msg" + (" streaming" if self.is_streaming else ""),
            )
            yield Static(
                colored(f"  {ts}", Colors.OKBLUE + " dim"),
                classes="timestamp",
            )
            # Citations
            if self.msg.citations:
                yield self._render_citations()

    def _render_citations(self) -> Static:
        """Render citation hints."""
        lines = []
        for i, c in enumerate(self.msg.citations[:3], 1):
            title = getattr(c, "paper_title", "")[:50]
            score = getattr(c, "relevance_score", 0)
            lines.append(f"  {i}. 📖 {title} (score={score:.2f})")
        if len(self.msg.citations) > 3:
            lines.append(f"  [+{len(self.msg.citations) - 3} more]")
        return Static(colored("\n".join(lines), Colors.WARNING), classes="cite-list")


class PaperCard(Static, can_focus=True):
    """An enhanced paper card with click-to-expand and rich metadata."""

    def __init__(self, citation, index: int, expanded: bool = False, on_select=None, on_compare_select=None, on_kg_toggle=None, **kwargs):
        self.citation = citation
        self.index = index
        self.expanded = expanded
        self.on_select = on_select
        self.on_compare_select = on_compare_select
        self.on_kg_toggle = on_kg_toggle
        super().__init__(**kwargs)

    def render(self) -> str:
        score = getattr(self.citation, "relevance_score", 0)
        title = getattr(self.citation, "paper_title", "Unknown")
        snippet = getattr(self.citation, "snippet", "")
        pid = getattr(self.citation, "paper_id", "")
        authors = getattr(self.citation, "authors", [])
        published = getattr(self.citation, "published", "")[:10]
        abstract = getattr(self.citation, "abstract", "")[:200]
        categories = getattr(self.citation, "categories", [])[:5]

        # Collapsed view
        expand_icon = "▼" if self.expanded else "▶"
        header = f"{expand_icon} [{self.index + 1}] {title[:45]}"
        meta = f"    📄 {pid} | ⭐ {score:.2f} | 📅 {published}"

        if not self.expanded:
            # Compact view with preview
            preview = snippet[:80] + "..." if len(snippet) > 80 else snippet
            return "\n".join(filter(None, [header, meta, f"    💬 {preview}"]))

        # Expanded view
        lines = [
            header,
            meta,
        ]

        # Authors with avatars
        if authors:
            author_str = f"    👥 {', '.join(authors[:4])}"
            if len(authors) > 4:
                author_str += f" +{len(authors) - 4}"
            lines.append(author_str)

        # Categories/Tags
        if categories:
            tags_str = "  ".join(f"[{c[:8]}]" for c in categories[:5])
            lines.append(f"    🏷️ {tags_str}")

        # Relevance score bar
        bar_len = int(score * 10)
        bar = "█" * bar_len + "░" * (10 - bar_len)
        lines.append(f"    📊 相关度: [{bar}] {score:.0%}")

        # Abstract
        if abstract:
            lines.append("")
            lines.append("  📝 摘要:")
            # Wrap abstract
            import textwrap

            wrapped = textwrap.wrap(abstract, width=45)
            for w in wrapped[:4]:
                lines.append(f"     {w}")
            if len(abstract) > 180:
                lines.append("     ...")

        # Snippet/Context
        if snippet and snippet != abstract:
            lines.append("")
            lines.append("  💬 相关片段:")
            wrapped = textwrap.wrap(snippet, width=45)
            for w in wrapped[:3]:
                lines.append(f"     {w}")

        # Click hint
        lines.append("")
        lines.append("  ▸ 点击收起")
        if self.on_kg_toggle:
            lines.append("  [KG] 查看知识图谱")

        return "\n".join(lines)

    def on_click(self) -> None:
        """Toggle expanded state, or signal compare-select on Shift/Ctrl+click."""
        # Check for Shift or Ctrl modifier for compare selection
        from textual.events import Click
        # Shift or Ctrl click triggers compare selection instead of toggle
        try:
            # Try to get modifiers from event if available
            ev = self.app._last_event
            if isinstance(ev, Click) and (ev.modifiers.shift or ev.modifiers.ctrl):
                if self.on_compare_select:
                    self.on_compare_select(self.citation, self.index)
                    return
        except Exception:
            pass
        # Normal click: toggle expanded state
        self.expanded = not self.expanded
        self.refresh()
        if self.on_select:
            self.on_select(self.citation, self.expanded)


class SessionCard(Static):
    """A session card for session list."""

    def __init__(self, session: dict, index: int, is_active: bool = False, **kwargs):
        self.session = session
        self.index = index
        self.is_active = is_active
        super().__init__(**kwargs)

    def render(self) -> str:
        sid = self.session.get("id", "")[:8]
        title = self.session.get("title", "无标题")[:40]
        updated = self.session.get("updated_at", "")[:16]
        active = " ◉" if self.is_active else ""
        return f"  {self.index}. [{sid}] {title}{active}\n      📅 {updated}"


class CompareScreen(Screen):
    """Full-screen modal overlay for side-by-side paper comparison."""

    CSS = """
    CompareScreen {
        align: center middle;
        background: #0d0d14;
    }

    #compare-container {
        width: 100%;
        height: 100%;
        layout: horizontal;
    }

    .paper-pane {
        width: 50%;
        height: 100%;
        padding: 2 3;
        background: #13131f;
    }

    #paper-a-pane {
        border-right: solid #3a3a55;
    }

    #paper-b-pane {
        border-left: solid #3a3a55;
    }

    .paper-header {
        color: #bd93f9;
        text-style: bold;
        margin-bottom: 1;
    }

    .paper-title {
        color: #f8f8f2;
        text-style: bold;
        font-size: 120%;
        margin-bottom: 1;
    }

    .paper-meta {
        color: #6272a4;
        margin-bottom: 1;
    }

    .section-header {
        color: #ff79c6;
        text-style: bold;
        margin-top: 1;
        margin-bottom: 0;
    }

    .paper-abstract {
        color: #c0c0d0;
        margin-top: 0;
        margin-bottom: 1;
    }

    .paper-contrib {
        color: #8be9fd;
        margin-top: 0;
        margin-bottom: 1;
    }

    .paper-tags {
        color: #50fa7b;
        margin-top: 0;
    }

    #divider {
        width: 1;
        height: 100%;
        background: #6272a4;
    }

    #compare-header {
        width: 100%;
        color: #f1fa8c;
        text-style: bold;
        padding: 1 4;
        background: #1e1e2e;
        dock: top;
    }

    #compare-footer {
        width: 100%;
        color: #6272a4;
        padding: 0 4;
        dock: bottom;
        background: #1a1a28;
    }
    """

    BINDINGS = [
        Binding("q", "app.pop_screen", "Close", show=True),
        Binding("escape", "app.pop_screen", "Close", show=True),
        Binding("t", "swap_papers", "Swap", show=True),
    ]

    def __init__(self, paper_a: dict, paper_b: dict, **kwargs):
        super().__init__(**kwargs)
        self._paper_a = paper_a
        self._paper_b = paper_b

    def compose(self) -> ComposeResult:
        yield Static("Paper A                                │  Paper B", id="compare-header")
        with Horizontal(id="compare-container"):
            yield Static(self._render_paper(self._paper_a), classes="paper-pane", id="paper-a-pane")
            yield Static("", classes="divider", id="divider")
            yield Static(self._render_paper(self._paper_b), classes="paper-pane", id="paper-b-pane")
        yield Static(
            "Esc / q: Close    t: Swap left/right",
            id="compare-footer",
        )

    def _render_paper(self, paper: dict) -> str:
        """Render a paper dict as a formatted string."""
        title = paper.get("title", "Unknown Title")
        authors = paper.get("authors", [])
        year = paper.get("published", "")[:4] or "N/A"
        venue = paper.get("journal") or paper.get("primary_category", "") or "N/A"
        abstract = paper.get("abstract", "")
        categories = paper.get("categories", [])
        tags = paper.get("tags", [])
        citation_count = paper.get("citation_count", 0)

        # Abstract: first 300 chars with ellipsis
        abstract_short = abstract[:300] + ("…" if len(abstract) > 300 else "")

        # Key contributions: use field if present, else first 3 sentences of abstract
        key_contribs = paper.get("key_contributions", [])
        if key_contribs:
            contrib_lines = key_contribs[:3]
        else:
            # Extract first 3 sentences from abstract as fallback
            sentences = re.split(r"[.!?]+", abstract)
            contrib_lines = [s.strip() for s in sentences if s.strip()][:3]

        author_str = ", ".join(authors[:4]) + (" +" if len(authors) > 4 else "")
        categories_str = ", ".join(categories[:5])
        tags_str = "  ".join(f"[{t[:8]}]" for t in tags[:5]) if tags else categories_str

        lines = []

        # Title
        lines.append(colored(title, Colors.HEADER + Colors.BOLD))
        lines.append("")

        # Meta: authors, year, venue, citations
        if citation_count:
            lines.append(colored(f"Authors: {author_str}", Colors.OKBLUE))
            lines.append(colored(f"Year: {year}    Venue: {venue}    Citations: {citation_count:,}+", Colors.OKBLUE))
        else:
            lines.append(colored(f"Authors: {author_str}", Colors.OKBLUE))
            lines.append(colored(f"Year: {year}    Venue: {venue}", Colors.OKBLUE))
        lines.append("")

        # Abstract section
        lines.append(colored("Abstract:", Colors.HEADER + Colors.BOLD))
        import textwrap
        for chunk in textwrap.wrap(abstract_short, width=55):
            lines.append("  " + chunk)
        lines.append("")

        # Key Contributions
        lines.append(colored("Key Contributions:", Colors.HEADER + Colors.BOLD))
        for contrib in contrib_lines:
            contrib_clean = contrib.strip()[:80]
            lines.append(f"  • {contrib_clean}")
        lines.append("")

        # Tags
        if tags_str:
            lines.append(colored("Tags:", Colors.HEADER + Colors.BOLD))
            lines.append(f"  {tags_str}")

        return "\n".join(lines)

    def action_swap_papers(self) -> None:
        """Swap the left and right papers."""
        self._paper_a, self._paper_b = self._paper_b, self._paper_a
        self.query_one("#paper-a-pane").update(self._render_paper(self._paper_a))
        self.query_one("#paper-b-pane").update(self._render_paper(self._paper_b))
        self.query_one("#compare-header").update(
            "Paper A                                │  Paper B"
        )


class SidebarPaperList(VerticalScroll):

    def __init__(self, citations: List, compare_select_callback: Callable[[Any, int], None] | None = None, on_kg_toggle: Callable[[Any], None] | None = None, **kwargs):
        self._citations = citations
        self._expanded_idx = None
        self._compare_select_callback = compare_select_callback
        self._on_kg_toggle = on_kg_toggle
        super().__init__(**kwargs)

    def compose(self) -> ComposeResult:
        yield Static(colored("📚 相关论文", Colors.HEADER + Colors.BOLD), classes="sidebar-title")
        for i, c in enumerate(self._citations[:5]):
            yield PaperCard(c, i, classes="paper-card", id=f"paper-{i}", on_compare_select=self._compare_select_callback, on_kg_toggle=self._on_kg_toggle)
        if len(self._citations) > 5:
            yield Static(
                colored(f"  [+{len(self._citations) - 5} more papers]", Colors.WARNING),
                classes="more-papers",
            )
        yield Label("")  # spacer
        # KG view widget mounted in sidebar (toggled on KG button click)
        yield SidebarKGView(id="kg-view", classes="kg-view hidden")


# ─── Suggestion Chips ─────────────────────────────────────────────────────────


class SuggestionChips(Horizontal):
    """Clickable suggestion chips for follow-up questions."""

    def __init__(self, suggestions: List[str], on_select: Callable[[str], None], **kwargs):
        self.suggestions = suggestions
        self.on_select = on_select
        super().__init__(**kwargs)

    def compose(self) -> ComposeResult:
        for i, s in enumerate(self.suggestions[:4]):
            btn = Button(
                f"💡 {s[:40]}",
                id=f"suggestion-{i}",
                classes="suggestion-btn",
            )
            btn.on_click = lambda e, text=s: self.on_select(text)  # type: ignore[attr-defined]
            yield btn


class ReasoningBuffer(Static):
    """A collapsible reasoning steps display widget.

    Shows each reasoning phase as a progress bar:
    [Reasoning: decomposition ████████░░░ 67%]
    [Reasoning: search        ██████░░░░░ 50%]
    [Reasoning: synthesis     ░░░░░░░░░░  0%]

    Each phase shows a progress bar based on `done` flag.
    Collapsed by default, expanded via click.
    Color: `Colors.WARNING` (orange/yellow).
    Uses `▓` (filled) and `░` (empty) for the bar.
    """

    BAR_LEN = 10

    def __init__(self, **kwargs):
        self._phases: Dict[str, tuple[str, bool]] = {}  # phase -> (content, done)
        self._expanded = False
        super().__init__(**kwargs)

    def update_phase(self, phase: str, content: str, done: bool) -> None:
        """Update or add a reasoning phase."""
        self._phases[phase] = (content, done)
        self._render()

    def clear(self) -> None:
        """Clear all phases."""
        self._phases = {}
        self._render()

    def _render(self) -> None:
        """Render the reasoning buffer."""
        if not self._phases:
            self.update("")
            return

        lines = []
        for phase, (content, done) in self._phases.items():
            bar = "▓" * self.BAR_LEN if done else "░" * self.BAR_LEN
            label = phase or "reasoning"
            lines.append(
                colored(f"🤖 [Reasoning: {label:<14} {bar} {'done' if done else '...'} ]", Colors.WARNING)
            )

        text = "\n".join(lines)
        if not self._expanded:
            text = colored("▸ [Reasoning: ", Colors.WARNING) + colored(f"{len(self._phases)} phase(s) hidden, click to expand]", Colors.OKBLUE)
        self.update(text)

    def on_click(self) -> None:
        """Toggle expanded state."""
        self._expanded = not self._expanded
        self._render()

    def on_click(self) -> None:
        """Toggle expanded state."""
        self._expanded = not self._expanded
        self._render()


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
        background: #1e1e2e;
    }

    /* ── Main layout ── */
    #chat-area {
        width: 65%;
        height: 100%;
        background: #0d0d14;
        border: solid #2d2d44;
    }

    #sidebar {
        width: 35%;
        height: 100%;
        background: #13131f;
        border: solid #2d2d44;
    }

    #sidebar-title {
        color: #c0c0ff;
        text-style: bold;
        padding: 1 2;
        background: #1a1a2f;
    }

    .paper-card {
        color: #b0b0d0;
        padding: 1 2;
        border: solid #3a3a55;
        margin: 0 0 1 0;
        background: #18182a;
    }

    .paper-card:hover, .paper-card:focus {
        background: #202035;
        border: solid #4a4a70;
    }

    .paper-card:focus {
        border: solid #bd93f9;
    }

    .paper-card.expanded {
        background: #1a1a30;
        border: solid #5a5a80;
    }

    .more-papers {
        color: #7070aa;
        padding: 1 2;
    }

    .kg-view {
        color: #8be9fd;
        padding: 1 2;
        background: #1a1a30;
    }

    .kg-view.hidden {
        display: none;
    }

    /* ── Message area ── */
    #messages {
        width: 100%;
        height: 100%;
        padding: 1 2;
    }

    .user-msg {
        color: #50fa7b;
        padding: 0 0;
    }

    .ai-msg {
        color: #8be9fd;
        padding: 0 0;
    }

    .timestamp {
        color: #444466;
        padding: 0 0;
    }

    .cite-list {
        color: #ffb86c;
        padding: 1 0;
    }

    /* ── Input area ── */
    #input-area {
        height: 6;
        background: #1a1a28;
        border-top: solid #3a3a55;
        padding: 1 2;
    }

    Input {
        margin: 0 1;
    }

    .action-btn {
        margin: 0 1;
        color: #bd93f9;
    }

    .action-btn:hover {
        color: #ff79c6;
    }

    #btn-history {
        color: #8be9fd;
    }

    #btn-new {
        color: #50fa7b;
    }

    #btn-export {
        color: #ffb86c;
    }

    /* ── Status ── */
    #status-bar {
        background: #0a0a12;
        color: #6060a0;
        padding: 0 2;
    }

    #status-bar.typing {
        color: #f1fa8c;
    }

    #status-bar.done {
        color: #50fa7b;
    }

    #status-bar.error {
        color: #ff5555;
    }

    #status-bar.vim-normal {
        color: #50fa7b;
    }

    #status-bar.vim-search {
        color: #f1fa8c;
    }

    #status-bar.vim-command {
        color: #8be9fd;
    }

    #status-bar.vim-insert {
        color: #ff79c6;
    }

    /* ── Suggestions ── */
    .suggestion-btn {
        margin: 0 1;
        color: #bd93f9;
    }

    .suggestion-btn:hover {
        color: #ff79c6;
    }

    /* ── Welcome ── */
    #welcome {
        color: #6272a4;
    }

    /* ── Loading animation ── */
    .loading-dots {
        color: #f1fa8c;
    }

    /* ── Selected message ── */
    .selected {
        background: #2a2a44;
        border: solid #bd93f9;
    }

    /* ── Nav hint ── */
    #nav-hint {
        color: #6272a4;
        padding: 0 2;
    }

    /* ── Vim mode indicator ── */
    #nav-hint.vim-normal {
        color: #50fa7b;
    }

    #nav-hint.vim-search {
        color: #f1fa8c;
    }

    #nav-hint.vim-command {
        color: #8be9fd;
    }

    #nav-hint.vim-insert {
        color: #ff79c6;
    }

    /* ── Progress indicator ── */
    #progress-bar {
        width: 100%;
        height: 2;
        background: #1a1a2e;
        dock: bottom;
    }

    #progress-fill {
        width: 0%;
        height: 100%;
        background: #50fa7b;
    }
    """

    BINDINGS = [
        Binding("q", "quit", "Quit", show=True),
        Binding("ctrl+c", "quit", "", show=False),
        Binding("ctrl+l", "clear", "Clear", show=False),
        Binding("ctrl+e", "toggle_history", "History", show=False),
        Binding("ctrl+s", "export_session", "Export", show=False),
        Binding("ctrl+f", "search_messages", "Search", show=False),
        Binding("f1", "help", "Help", show=False),
        Binding("ctrl+n", "new_session", "New", show=False),
        Binding("ctrl+k", "command_palette", "Cmd", show=False),
        Binding("tab", "complete_command", "Tab", show=False),
        Binding("up", "select_prev_message", "↑", show=False),
        Binding("down", "select_next_message", "↓", show=False),
        Binding("enter", "activate_message", "Enter", show=False),
        Binding("c", "copy_selected", "Copy", show=False),
        Binding("e", "edit_selected", "Edit", show=False),
    ]

    def __init__(
        self,
        chat: RagChat,
        concept: str | None = None,
        limit: int = 5,
        friction_tracker: FrictionTracker | None = None,
        stream: bool = True,
        session_id: str | None = None,
        **kwargs,
    ):
        super().__init__(**kwargs)
        self.chat = chat
        self.concept = concept
        self.limit = limit
        self.stream = stream
        self.friction = friction_tracker or FrictionTracker()
        self.session_id = session_id
        self.messages: List[ChatMessage] = []
        self.pending_citations: List = []
        self._streaming = False
        self._chat_history: List[dict] = []
        self._loading = LoadingDots()
        self._stream_config = StreamConfig()
        self._suggestions: List[str] = []
        self._selected_msg_idx: int = -1  # For keyboard navigation
        self._msg_index: Dict[str, Set[int]] = {}  # Token → message indices
        self._reasoning_buffer: Optional["ReasoningBuffer"] = None  # Streaming reasoning display

        # Vim mode state
        self._vim_mode: str = "normal"  # normal | search | command | insert
        self._pending_g: bool = False  # For g/G two-key combos
        self._g_timeout_task: asyncio.Task | None = None  # Pending g/G timeout

        # Compare mode state
        self._compare_mode: bool = False  # True when selecting papers to compare
        self._compare_selected: List[Any] = []  # List of selected papers for comparison

    # ── App lifecycle ──────────────────────────────────────────────────────

    def compose(self) -> ComposeResult:
        yield Header(show_clock=True)
        with Horizontal(id="main-row"):
            with Container(id="chat-area"):
                with VerticalScroll(id="messages"):
                    yield Static(
                        colored("📚 AI Research OS — RAG Chat\n", Colors.HEADER + Colors.BOLD)
                        + colored("   对你的论文库进行自然语言问答，带引用溯源\n", Colors.OKBLUE)
                        + colored(
                            "   Enter 发送 · Ctrl+L 清屏 · Ctrl+E 历史 · Ctrl+S 导出 · q 退出\n",
                            Colors.WARNING,
                        ),
                        id="welcome",
                    )
            with Container(id="sidebar"):
                yield Static(
                    colored("📚 相关论文", Colors.HEADER + Colors.BOLD), id="sidebar-title"
                )
                yield SidebarPaperList([], id="paper-list", compare_select_callback=self._on_paper_compare_select, on_kg_toggle=self._on_paper_kg_toggle)
                yield Button("⚖️ 对比", id="btn-compare", variant="primary", classes="action-btn")
        with Container(id="input-area"):
            yield Input(
                placeholder="输入问题后按 Enter 发送...",
                id="chat-input",
                classes="chat-input",
            )
            yield Button("📜 历史", id="btn-history", variant="primary", classes="action-btn")
            yield Button("🆕 新建", id="btn-new", variant="primary", classes="action-btn")
            yield Button("💾 导出", id="btn-export", variant="primary", classes="action-btn")
        yield Static("❯ 输入问题开始对话  |  ↑↓ 选择消息  c=复制  e=编辑", id="nav-hint")
        yield Static("❯ 输入问题开始对话", id="status-bar")
        # Progress bar for streaming
        yield Static("", id="progress-bar")

    def on_mount(self) -> None:
        self.query_one("#chat-input").focus()
        if self.session_id:
            self._load_session(self.session_id)
        self._update_vim_mode_indicators()

    # ── Vim Mode ─────────────────────────────────────────────────────────────

    def _set_vim_mode(self, mode: str) -> None:
        """Set Vim mode and update indicators."""
        self._vim_mode = mode
        self._update_vim_mode_indicators()

    def _update_vim_mode_indicators(self) -> None:
        """Update status bar and nav hint with current Vim mode."""
        try:
            status = cast(Static, self.query_one("#status-bar"))
            hint = cast(Static, self.query_one("#nav-hint"))

            # Remove all vim classes
            for cls in ["vim-normal", "vim-search", "vim-command", "vim-insert"]:
                status.remove_class(cls)
                hint.remove_class(cls)

            # Add current mode class
            status.add_class(f"vim-{self._vim_mode}")
            hint.add_class(f"vim-{self._vim_mode}")

            # Update status bar text with mode indicator
            mode_labels = {
                "normal": "-- NORMAL --",
                "search": "-- SEARCH --",
                "command": "-- COMMAND --",
                "insert": "-- INSERT --",
            }
            current_text = status.renderable if hasattr(status, "renderable") else ""
            if isinstance(current_text, str):
                # Prepend mode indicator to status text
                mode_label = mode_labels.get(self._vim_mode, "-- NORMAL --")
                # Keep existing status content but add mode prefix
                pass  # Status keeps its own text

            # Update nav hint based on mode
            nav_hints = {
                "normal": "❯ j/k↑↓  /搜索  :命令  g顶  G底  q退出",
                "search": "❯ 输入搜索词后 Enter  Esc取消",
                "command": "❯ 输入命令后 Enter  :w保存 :q退出 :e编辑 :goto N  Esc取消",
                "insert": "❯ 输入问题开始对话  |  ↑↓ 选择  c=复制  e=编辑",
            }
            hint.update(colored(nav_hints.get(self._vim_mode, nav_hints["normal"]), Colors.OKBLUE))

            # Add message counter if message is selected
            self._update_message_counter()

        except NoMatches:
            pass

    def _update_message_counter(self) -> None:
        """Update status bar with message counter (selected/total)."""
        if self._selected_msg_idx < 0 or not self.messages:
            return
        try:
            status = cast(Static, self.query_one("#status-bar"))
            ai_msg_indices = [i for i, m in enumerate(self.messages) if m.role == "assistant"]
            if self._selected_msg_idx in ai_msg_indices:
                idx = ai_msg_indices.index(self._selected_msg_idx) + 1
                total = len(ai_msg_indices)
                current = status.renderable if hasattr(status, "renderable") else ""
                if isinstance(current, str) and f" {idx}/{total} " not in current:
                    status.update(f"{current}  [{idx}/{total}]")
        except NoMatches:
            pass

    def on_key(self, event) -> None:
        """Handle Vim mode key bindings before default Textual handling."""
        # Cancel pending g timeout if any key is pressed
        if self._pending_g and event.key not in ("g", "G"):
            self._pending_g = False
            if self._g_timeout_task:
                self._g_timeout_task.cancel()
                self._g_timeout_task = None

        # Handle based on current mode
        if self._vim_mode == "normal":
            self._handle_vim_normal_key(event)
        elif self._vim_mode == "search":
            self._handle_vim_search_key(event)
        elif self._vim_mode == "command":
            self._handle_vim_command_key(event)

        # Handle Escape in any mode to return to normal
        if event.key == "escape":
            self._set_vim_mode("normal")
            try:
                self.query_one("#chat-input").focus()
            except NoMatches:
                pass

    def _handle_vim_normal_key(self, event) -> None:
        """Handle keypresses in Vim Normal mode."""
        key = event.key

        # j → down (select next message)
        if key == "j":
            self.action_select_next_message()
            return

        # k → up (select previous message)
        if key == "k":
            self.action_select_prev_message()
            return

        # / → enter Search mode
        if key == "/":
            self._set_vim_mode("search")
            try:
                inp = cast(Input, self.query_one("#chat-input"))
                inp.value = "/"
                inp.cursor_position = len(inp.value)
                inp.focus()
            except NoMatches:
                pass
            return

        # : → enter Command mode
        if key == ":":
            self._set_vim_mode("command")
            try:
                inp = cast(Input, self.query_one("#chat-input"))
                inp.value = ":"
                inp.cursor_position = len(inp.value)
                inp.focus()
            except NoMatches:
                pass
            return

        # g → start two-key combo for go-to-top
        if key == "g":
            self._pending_g = True
            if self._g_timeout_task:
                self._g_timeout_task.cancel()

            async def clear_pending():
                await asyncio.sleep(0.3)
                self._pending_g = False

            self._g_timeout_task = asyncio.create_task(clear_pending())
            return

        # G → go to bottom (last message) - only if not preceded by g
        if key == "G" and not self._pending_g:
            self._go_to_last_message()
            return

        # Handle gG combo (g followed by G = go to top)
        # This is handled when we get the second 'g' and _pending_g is True
        if key == "g" and self._pending_g:
            # This is the second 'g', meaning gg = go to top
            self._pending_g = False
            if self._g_timeout_task:
                self._g_timeout_task.cancel()
                self._g_timeout_task = None
            self._go_to_first_message()
            return

        # q → quit (already handled by binding, but make sure mode is normal)
        # Let Textual handle 'q' via BINDINGS

    def _handle_vim_search_key(self, event) -> None:
        """Handle keypresses in Vim Search mode."""
        key = event.key

        # Enter → execute search
        if key == "enter":
            try:
                inp = cast(Input, self.query_one("#chat-input"))
                query = inp.value.strip()
                if query.startswith("/"):
                    query = query[1:]
                if query:
                    self._search_current_messages(query)
                inp.value = ""
                self._set_vim_mode("normal")
                self.query_one("#chat-input").focus()
            except NoMatches:
                pass
            return

        # Escape → cancel and return to normal mode
        if key == "escape":
            try:
                inp = cast(Input, self.query_one("#chat-input"))
                inp.value = ""
            except NoMatches:
                pass
            self._set_vim_mode("normal")
            return

        # Let other keys (letters, backspace, etc.) pass to input via Textual default

    def _handle_vim_command_key(self, event) -> None:
        """Handle keypresses in Vim Command mode."""
        key = event.key

        # Enter → execute command
        if key == "enter":
            try:
                inp = cast(Input, self.query_one("#chat-input"))
                cmd = inp.value.strip()
                if cmd.startswith(":"):
                    cmd = cmd[1:]
                inp.value = ""
                self._execute_vim_command(cmd)
                self._set_vim_mode("normal")
                self.query_one("#chat-input").focus()
            except NoMatches:
                pass
            return

        # Escape → cancel and return to normal mode
        if key == "escape":
            try:
                inp = cast(Input, self.query_one("#chat-input"))
                inp.value = ""
            except NoMatches:
                pass
            self._set_vim_mode("normal")
            return

        # Let other keys pass to input via Textual default

    def _execute_vim_command(self, cmd: str) -> None:
        """Execute a Vim-style command."""
        if not cmd:
            return

        parts = cmd.split(maxsplit=1)
        command = parts[0].lower()
        arg = parts[1] if len(parts) > 1 else ""

        if command == "w":
            # Save / export
            self._export_session()
        elif command == "q":
            # Quit
            self.action_quit()
        elif command == "e":
            # Edit selected message
            self.action_edit_selected()
        elif command == "goto" or command == "g":
            # Go to message number
            if arg:
                self._goto_message(arg)
        elif command == "wq":
            # Write and quit
            self._export_session()
            self.action_quit()
        else:
            self._update_status(f"⚠️ 未知命令: {command}")

    def _go_to_first_message(self) -> None:
        """Go to first AI message."""
        ai_msg_indices = [i for i, m in enumerate(self.messages) if m.role == "assistant"]
        if ai_msg_indices:
            self._selected_msg_idx = ai_msg_indices[0]
            self._render_messages_with_selection()
            self._update_nav_hint(f"已跳转到第1/{len(ai_msg_indices)}条")

    def _go_to_last_message(self) -> None:
        """Go to last AI message."""
        ai_msg_indices = [i for i, m in enumerate(self.messages) if m.role == "assistant"]
        if ai_msg_indices:
            self._selected_msg_idx = ai_msg_indices[-1]
            self._render_messages_with_selection()
            self._update_nav_hint(f"已跳转到第{len(ai_msg_indices)}/{len(ai_msg_indices)}条")

    # ── Input handling ───────────────────────────────────────────────────────

    def on_input_submitted(self, event: Input.Submitted) -> None:
        if self._streaming:
            return
        question = event.value.strip()
        if not question:
            return
        self._handle_submit(question)
        cast(Input, self.query_one("#chat-input")).value = ""

    def on_button_pressed(self, event: Button.Pressed) -> None:
        """Handle quick action button clicks."""
        btn_id = event.button.id
        if btn_id == "btn-history":
            self.action_toggle_history()
        elif btn_id == "btn-new":
            self.action_new_session()
        elif btn_id == "btn-export":
            self.action_export_session()
        elif btn_id == "btn-compare":
            self._start_compare_mode()

    def _handle_submit(self, question: str) -> None:
        """Process a user question."""
        # Command handling
        cmd = question.strip()
        if cmd.startswith("/"):
            self._handle_command(cmd)
            return

        # Create session if not exists
        if not self.session_id:
            import uuid

            self.session_id = str(uuid.uuid4())[:8]
            try:
                self.chat.db.create_chat_session(self.session_id, "TUI对话")
            except Exception:
                pass

        # Rewrite follow-up questions
        rewritten_question = question
        if self._chat_history:
            try:
                rewritten_question = self.chat._rewrite_followup(question, self._chat_history)
            except Exception:
                pass

        # Add user message
        user_msg = ChatMessage(
            role="user", content=question, citations=[], timestamp=datetime.now().isoformat()
        )
        self.messages.append(user_msg)
        _index_message(tokenize(user_msg.content), len(self.messages) - 1, self._msg_index)
        self._render_messages()

        # Set streaming state
        self._streaming = True
        self._update_status("🔍 检索中...", "typing")

        # Build AI placeholder
        ai_msg = ChatMessage(
            role="assistant", content="", citations=[], timestamp=datetime.now().isoformat()
        )
        self.messages.append(ai_msg)
        _index_message(tokenize(ai_msg.content), len(self.messages) - 1, self._msg_index)
        self._render_messages()

        try:
            if self.stream:
                # Retrieve contexts
                contexts = self.chat._retrieve(rewritten_question, None, self.concept, self.limit)

                if not contexts:
                    # Fallback without context
                    if self.chat.api_key:
                        self._update_status("🤖 生成回答中...", "typing")
                        answer = self._stream_no_context(rewritten_question)
                        ai_msg.content = answer
                    else:
                        ai_msg.content = "⚠️ 未找到相关论文，且未配置 API Key"
                        self._update_status("⚠️ 无相关论文", "error")
                else:
                    # RAG response
                    self._stream_with_context(ai_msg, rewritten_question, contexts)

            # Save to history
            self._chat_history.append(
                {
                    "question": rewritten_question,
                    "answer": ai_msg.content,
                    "citations": ai_msg.citations,
                }
            )

            # Persist to database
            self._save_to_session(question, ai_msg.content, ai_msg.citations)

            # Update sidebar and show suggestions
            self.pending_citations = ai_msg.citations
            self._update_sidebar(ai_msg.citations)
            self._update_status(f"✅ 回复完成 · {len(ai_msg.citations)} 篇引用", "done")

            # Generate suggestions
            self._show_suggestions(ai_msg.citations)

        except Exception as e:
            ai_msg.content = colored(f"⚠️ 出错了: {e}", Colors.FAIL)
            self._update_status(colored(f"⚠️ 错误: {e}", Colors.FAIL), "error")
            self.friction.record_retrieval_failure("chat_tui", question, notes=str(e))

        finally:
            self._streaming = False
            # Clear progress bar
            self._clear_progress()
            # Final render without streaming indicator
            self._render_messages()
            self.scroll_messages_to_bottom()

    def _stream_no_context(self, question: str) -> str:
        """Stream response without paper context."""
        from llm.client import stream_llm_chat_completions

        answer = ""
        for delta in stream_llm_chat_completions(
            [],
            model=self.chat.model,
            user_prompt=question,
            base_url=self.chat.base_url,
            api_key=self.chat.api_key,
            system_prompt="你是一个有帮助的 AI 助手，擅长回答各种问题。用中文简洁回答。",
        ):
            answer += delta
            if len(answer) % 10 < 3:
                self._update_streaming_content(answer)

        return answer

    def _stream_with_context(self, ai_msg: ChatMessage, question: str, contexts) -> None:
        """Stream response with paper context."""
        from llm.client import stream_llm_chat_completions
        from llm.chat import _RAG_SYSTEM_PROMPT
        from llm.reasoning import StreamingReasoner, ReasoningBlock

        self._update_status("🤖 生成回答中...", "typing")

        # Build prompt
        prompt = self.chat._build_prompt(question, contexts)

        # Prepare messages for streaming
        messages: List[dict] = []
        if _RAG_SYSTEM_PROMPT:
            messages.append({"role": "system", "content": _RAG_SYSTEM_PROMPT})
        messages.append({"role": "user", "content": prompt})

        # Create and mount reasoning buffer above AI message
        self._mount_reasoning_buffer()

        answer = ""

        def on_chunk(content_delta: str) -> None:
            nonlocal answer
            answer += content_delta
            if len(answer) % self._stream_config.batch_size < 2:
                ai_msg.content = answer
                self._update_streaming_content(answer)

        def on_reasoning(block: ReasoningBlock) -> None:
            self._update_reasoning_buffer(block)

        # Try streaming with reasoning first, fall back to plain streaming
        try:
            reasoner = StreamingReasoner(
                model=self.chat.model,
                extended_thinking=True,
            )
            # Suppress the generator result - callbacks do the work
            for _ in reasoner.stream_messages(
                messages,
                on_chunk=on_chunk,
                on_reasoning=on_reasoning,
                base_url=self.chat.base_url,
                api_key=self.chat.api_key,
            ):
                pass
        except Exception:
            # Fall back to plain streaming
            self._remove_reasoning_buffer()
            for delta in stream_llm_chat_completions(
                [],
                model=self.chat.model,
                user_prompt=prompt,
                base_url=self.chat.base_url,
                api_key=self.chat.api_key,
                system_prompt=_RAG_SYSTEM_PROMPT,
            ):
                answer += delta
                if len(answer) % self._stream_config.batch_size < 2:
                    ai_msg.content = answer
                    self._update_streaming_content(answer)

        ai_msg.content = answer
        ai_msg.citations = self.chat._extract_citations(contexts)
        # Rebuild index for assistant message (streaming content was not indexed)
        msg_idx = self.messages.index(ai_msg)
        _index_message(tokenize(ai_msg.content), msg_idx, self._msg_index)
        self._remove_reasoning_buffer()

    def _mount_reasoning_buffer(self) -> None:
        """Mount the reasoning buffer widget above the AI message bubble."""
        try:
            container = self.query_one("#messages")
            # Remove old reasoning buffer if any
            self._remove_reasoning_buffer()
            # Create new buffer
            self._reasoning_buffer = ReasoningBuffer(id="reasoning-buffer")
            # Insert before the last message (AI bubble) if messages exist
            if self.messages and self.messages[-1].role == "assistant":
                # Find the ChatBubble for the last message and insert before it
                bubbles = list(container.query("ChatBubble"))
                if bubbles:
                    bubbles[-1].mount(self._reasoning_buffer, before=bubbles[-1])
                    return
            # Fallback: just append
            container.mount(self._reasoning_buffer)
        except NoMatches:
            pass

    def _update_reasoning_buffer(self, block: "ReasoningBlock") -> None:
        """Update the reasoning buffer with a new block."""
        if self._reasoning_buffer is None:
            return
        self._reasoning_buffer.update_phase(block.phase, block.content, block.done)

    def _remove_reasoning_buffer(self) -> None:
        """Remove the reasoning buffer widget."""
        if self._reasoning_buffer is not None:
            try:
                self._reasoning_buffer.remove()
            except Exception:
                pass
            self._reasoning_buffer = None

    def _update_streaming_content(self, content: str) -> None:
        """Update streaming content with animation."""
        if self.messages and self.messages[-1].role == "assistant":
            self.messages[-1].content = content
            # Render with streaming indicator
            self._render_streaming_message()
            # Update progress bar
            self._update_progress(len(content))
            self.scroll_messages_to_bottom()

    def _render_streaming_message(self) -> None:
        """Render current message with streaming animation."""
        try:
            container = self.query_one("#messages")
            # Remove old message widgets
            for w in container.query("ChatBubble"):
                w.remove()

            for i, msg in enumerate(self.messages):
                is_streaming = i == len(self.messages) - 1 and self._streaming
                bubble = ChatBubble(msg, self._stream_config, is_streaming=is_streaming)
                container.mount(bubble)
        except NoMatches:
            pass

    def _update_progress(self, char_count: int) -> None:
        """Update progress bar based on character count."""
        try:
            # Estimate total based on typical response size (2000 chars)
            estimated_total = max(char_count * 3, 100)
            progress = min(int(char_count / estimated_total * 100), 95)
            cast(Static, self.query_one("#progress-bar")).update(
                f"[{'█' * progress}{'░' * (100 - progress)}] {progress}%"
            )
        except NoMatches:
            pass

    def _clear_progress(self) -> None:
        """Clear the progress bar."""
        try:
            cast(Static, self.query_one("#progress-bar")).update("")
        except NoMatches:
            pass

    def _handle_command(self, cmd: str) -> None:
        """Handle slash commands."""
        parts = cmd.split(maxsplit=1)
        command = parts[0].lower()
        arg = parts[1] if len(parts) > 1 else ""

        handlers = {
            "/sessions": self._show_sessions,
            "/load": lambda: self._load_session_by_index(arg) if arg else self._show_sessions(),
            "/search": lambda: (
                self._search_sessions(arg) if arg else self._update_status("用法: /search <关键词>")
            ),
            "/msg": lambda: (
                self._search_current_messages(arg)
                if arg
                else self._update_status("用法: /msg <关键词>")
            ),
            "/code": lambda: (
                self._search_code(arg) if arg else self._update_status("用法: /code <关键词>")
            ),
            "/goto": lambda: (
                self._goto_message(arg) if arg else self._update_status("用法: /goto <编号>")
            ),
            "/rename": lambda: (
                self._rename_session(arg) if arg else self._update_status("用法: /rename <新标题>")
            ),
            "/delete": lambda: (
                self._delete_session(arg) if arg else self._update_status("用法: /delete <编号>")
            ),
            "/export": lambda: (
                self._export_session() if self.session_id else self._update_status("⚠️ 当前没有会话")
            ),
            "/clear": self.action_clear,
            "/help": self._show_help,
        }

        handler = handlers.get(command)
        if handler:
            handler()
        else:
            self._update_status(f"⚠️ 未知命令: {command}")

    def _render_messages(self) -> None:
        """Re-render all messages efficiently."""
        try:
            container = self.query_one("#messages")
            # Remove old message widgets (keep welcome)
            for w in container.query("ChatBubble"):
                w.remove()

            for msg in self.messages:
                bubble = ChatBubble(msg, self._stream_config)
                container.mount(bubble)

            # Reset selection when messages change
            self._selected_msg_idx = -1
            self._update_nav_hint()
        except NoMatches:
            pass

    def scroll_messages_to_bottom(self) -> None:
        try:
            container = self.query_one("#messages")
            container.scroll_end(animate=True)
        except NoMatches:
            pass

    def _update_sidebar(self, citations) -> None:
        """Refresh the paper sidebar."""
        try:
            sidebar = self.query_one("#paper-list")
            for child in sidebar.query("*"):
                child.remove()
            for c in citations[:5]:
                sidebar.mount(PaperCard(c, 0, classes="paper-card", on_compare_select=self._on_paper_compare_select))
            if len(citations) > 5:
                sidebar.mount(
                    Static(
                        colored(f"  [+{len(citations) - 5} more papers]", Colors.WARNING),
                        classes="more-papers",
                    )
                )
        except NoMatches:
            pass

    # ── Compare Mode ────────────────────────────────────────────────────────────

    def _start_compare_mode(self) -> None:
        """Enter compare mode: clear selection and prompt user to select papers."""
        self._compare_selected.clear()
        self._compare_mode = True
        self._update_status("⚖️ 对比模式: 点击两篇论文进行对比 (Shift/Ctrl+点击)", "done")
        self._update_nav_hint("⚖️ 对比模式: 点击两篇论文进行对比")

    def _on_paper_compare_select(self, citation, index: int) -> None:
        """Handle paper selection for comparison."""
        if not self._compare_mode:
            # If not in compare mode, enter compare mode on first click
            self._compare_selected.clear()
            self._compare_mode = True

        self._compare_selected.append(citation)

        if len(self._compare_selected) == 1:
            self._update_status(f"⚖️ 已选择第1篇: {getattr(citation, 'paper_title', 'Unknown')[:40]}...  再点击第二篇", "done")
        elif len(self._compare_selected) >= 2:
            # Two papers selected - show compare screen
            self._compare_mode = False
            self._show_compare_screen(self._compare_selected[0], self._compare_selected[1])
            self._compare_selected.clear()
            self._update_status("✅ 对比完成", "done")
            self._update_nav_hint()

    def _on_paper_kg_toggle(self, citation) -> None:
        """Handle [KG] button click on an expanded paper card."""
        paper_id = getattr(citation, "paper_id", "")
        if not paper_id:
            self._update_status("⚠️ 该论文没有 paper_id", "error")
            return

        # Try to get KG data via KGManager
        try:
            from kg import KGManager
        except Exception:
            try:
                from kg.manager import KGManager
            except Exception:
                kg_view = self.query_one("#kg-view", SidebarKGView)
                kg_view.show_empty("KG not available (kg package not found)")
                return

        try:
            kg = KGManager()
            paper_node = kg.get_node_by_entity("Paper", paper_id)
            if paper_node is None:
                kg_view = self.query_one("#kg-view", SidebarKGView)
                kg_view.show_empty(f"KG not available for this paper")
                return

            neighbors = kg.find_neighbors(paper_node["id"], depth=2)

            # Build node list: (node_id, type, label, connections)
            nodes = [(paper_node["id"], paper_node["type"], paper_node["label"], len(neighbors))]
            edges = []
            for neighbor_node, edge, depth in neighbors:
                nodes.append((neighbor_node["id"], neighbor_node["type"], neighbor_node["label"], 0))
                edges.append((paper_node["id"], neighbor_node["id"], edge["relation_type"]))

            ascii_graph = KGVisualizer.render(nodes, edges, root_id=paper_node["id"])
            kg_view = self.query_one("#kg-view", SidebarKGView)

            graph_lines = ascii_graph.split("\n")
            truncated = 0
            if len(graph_lines) > SidebarKGView.MAX_LINES:
                truncated = len(graph_lines) - SidebarKGView.MAX_LINES
                ascii_graph = "\n".join(graph_lines[: SidebarKGView.MAX_LINES])

            kg_view.show_kg(ascii_graph, truncated=truncated)
            self._update_status(f"📊 KG图谱 · {len(nodes)} 节点 · {len(edges)} 边", "done")
        except Exception as e:
            try:
                kg_view = self.query_one("#kg-view", SidebarKGView)
                kg_view.show_empty(f"KG unavailable: {e}")
            except Exception:
                pass
            self._update_status(f"⚠️ KG加载失败: {e}", "error")

    def _show_compare_screen(self, paper_a, paper_b) -> None:
        """Show the compare screen with two papers."""
        # Convert Citation objects to dicts for CompareScreen
        def paper_to_dict(citation) -> dict:
            return {
                "title": getattr(citation, "paper_title", "Unknown"),
                "authors": getattr(citation, "authors", []),
                "published": getattr(citation, "published", ""),
                "abstract": getattr(citation, "abstract", ""),
                "categories": getattr(citation, "categories", []),
                "tags": getattr(citation, "tags", []),
                "citation_count": getattr(citation, "citation_count", 0),
                "key_contributions": getattr(citation, "key_contributions", []),
                "journal": getattr(citation, "journal", ""),
                "primary_category": getattr(citation, "primary_category", ""),
            }

        paper_dict_a = paper_to_dict(paper_a)
        paper_dict_b = paper_to_dict(paper_b)
        self.push_screen(CompareScreen(paper_dict_a, paper_dict_b))

    def _update_status(self, text: str, cls: str = "") -> None:
        """Update status bar with style class."""
        try:
            status = cast(Static, self.query_one("#status-bar"))
            status.update(text)
            status.remove_class("typing", "done", "error")
            if cls:
                status.add_class(cls)
        except NoMatches:
            pass

    # ── Actions ────────────────────────────────────────────────────────────

    async def action_quit(self) -> None:
        self.exit(0)

    def action_clear(self) -> None:
        """Clear all messages."""
        self.messages.clear()
        self._chat_history.clear()
        self._render_messages()
        try:
            welcome = self.query_one("#welcome")
            self.query_one("#messages").mount(welcome)
        except NoMatches:
            pass
        self._update_status("对话已清除")

    def action_toggle_history(self) -> None:
        """Toggle session history panel."""
        self._show_sessions()

    def action_export_session(self) -> None:
        """Export current session to file."""
        self._export_session()

    def action_help(self) -> None:
        """Show help dialog."""
        self._show_help()

    def action_new_session(self) -> None:
        """Create a new chat session."""
        import uuid

        self.session_id = str(uuid.uuid4())[:8]
        self.messages.clear()
        self._chat_history.clear()
        try:
            self.chat.db.create_chat_session(self.session_id, "TUI对话")
        except Exception:
            pass
        self._render_messages()
        try:
            welcome = self.query_one("#welcome")
            self.query_one("#messages").mount(welcome)
        except NoMatches:
            pass
        self._update_status(f"✅ 新建会话 [{self.session_id}]")

    def action_command_palette(self) -> None:
        """Show command palette."""
        self.notify(
            "🎯 命令面板\n\n"
            "💬 发送: Enter\n"
            "📜 历史: Ctrl+E\n"
            "🔍 搜索: Ctrl+F\n"
            "🆕 新建: Ctrl+N\n"
            "💾 导出: Ctrl+S\n"
            "🗑️ 清屏: Ctrl+L\n"
            "↹ Tab   补全命令\n"
            "↑↓      选择消息\n"
            "c       复制选中\n"
            "e       编辑选中\n"
            "🚪 退出: q\n\n"
            "🔧 斜杠命令:\n"
            "  /sessions  查看会话\n"
            "  /load <id>  加载会话\n"
            "  /search <k> 搜索会话\n"
            "  /msg <k>     搜索消息\n"
            "  /goto <n>    跳转消息\n"
            "  /rename <t> 重命名\n"
            "  /export    导出\n"
            "  /help      帮助",
            title="快捷操作",
            timeout=8,
        )

    # ── Keyboard Navigation ─────────────────────────────────────────────────

    def action_select_prev_message(self) -> None:
        """Select previous message in history."""
        if not self.messages:
            return

        # If input has content, don't navigate (normal up arrow behavior)
        try:
            inp = cast(Input, self.query_one("#chat-input"))
            if inp.value:
                return
        except NoMatches:
            pass

        # Navigate through messages (AI messages only)
        ai_msg_indices = [i for i, m in enumerate(self.messages) if m.role == "assistant"]

        if not ai_msg_indices:
            return

        # Move selection
        if self._selected_msg_idx < 0:
            self._selected_msg_idx = len(ai_msg_indices) - 1
        else:
            idx_in_list = (
                ai_msg_indices.index(self._selected_msg_idx)
                if self._selected_msg_idx in ai_msg_indices
                else 0
            )
            idx_in_list = max(0, idx_in_list - 1)
            self._selected_msg_idx = ai_msg_indices[idx_in_list]

        self._render_messages_with_selection()
        idx = ai_msg_indices.index(self._selected_msg_idx) + 1
        total = len(ai_msg_indices)
        self._update_nav_hint(f"已选中 {idx}/{total}")
        self._update_message_counter()

    def action_select_next_message(self) -> None:
        """Select next message in history."""
        if not self.messages:
            return

        # If input has content, don't navigate
        try:
            inp = cast(Input, self.query_one("#chat-input"))
            if inp.value:
                return
        except NoMatches:
            pass

        ai_msg_indices = [i for i, m in enumerate(self.messages) if m.role == "assistant"]

        if not ai_msg_indices:
            return

        if self._selected_msg_idx < 0:
            self._selected_msg_idx = ai_msg_indices[0]
        else:
            idx_in_list = (
                ai_msg_indices.index(self._selected_msg_idx)
                if self._selected_msg_idx in ai_msg_indices
                else -1
            )
            idx_in_list = min(len(ai_msg_indices) - 1, idx_in_list + 1)
            self._selected_msg_idx = ai_msg_indices[idx_in_list]

        self._render_messages_with_selection()
        idx = ai_msg_indices.index(self._selected_msg_idx) + 1
        total = len(ai_msg_indices)
        self._update_nav_hint(f"已选中 {idx}/{total}")
        self._update_message_counter()

    def action_activate_message(self) -> None:
        """Handle Enter - deselect when no selection, otherwise focus input."""
        if self._selected_msg_idx >= 0:
            self._selected_msg_idx = -1
            self._render_messages()
            self._update_nav_hint("已取消选择")
            try:
                self.query_one("#chat-input").focus()
            except NoMatches:
                pass

    def action_copy_selected(self) -> None:
        """Copy selected message content to clipboard."""
        if self._selected_msg_idx < 0 or self._selected_msg_idx >= len(self.messages):
            self._update_status("⚠️ 没有选中的消息 (用↑↓选择)")
            return

        msg = self.messages[self._selected_msg_idx]
        content = msg.content[:500] + "..." if len(msg.content) > 500 else msg.content

        try:
            import pyperclip

            pyperclip.copy(content)
            self._update_status(f"✅ 已复制到剪贴板 ({len(content)} 字符)")
        except ImportError:
            # Fallback: show content in notification
            self.notify(f"📋 消息内容:\n\n{content[:300]}...", title="复制内容", timeout=5)
            self._update_status("📋 已显示消息内容（pyperclip未安装）")

    def action_edit_selected(self) -> None:
        """Edit selected message by copying to input."""
        if self._selected_msg_idx < 0 or self._selected_msg_idx >= len(self.messages):
            self._update_status("⚠️ 没有选中的消息 (用↑↓选择)")
            return

        msg = self.messages[self._selected_msg_idx]
        if msg.role == "assistant":
            self._update_status("⚠️ 只能编辑用户消息")
            return

        try:
            inp = cast(Input, self.query_one("#chat-input"))
            inp.value = msg.content
            inp.focus()
            self._selected_msg_idx = -1
            self._render_messages()
            self._update_status("✏️ 已复制到输入框，可编辑后发送")
        except NoMatches:
            pass

    def _render_messages_with_selection(self) -> None:
        """Render messages with selection highlight."""
        try:
            container = self.query_one("#messages")
            for w in container.query("ChatBubble"):
                w.remove()

            for i, msg in enumerate(self.messages):
                is_streaming = i == len(self.messages) - 1 and self._streaming
                bubble = ChatBubble(msg, self._stream_config, is_streaming=is_streaming)
                if i == self._selected_msg_idx:
                    bubble.add_class("selected")
                container.mount(bubble)

            self.scroll_messages_to_bottom()
        except NoMatches:
            pass

    def _update_nav_hint(self, text: str | None = None) -> None:
        """Update navigation hint bar with optional custom text."""
        try:
            hint = cast(Static, self.query_one("#nav-hint"))
            if text:
                hint.update(
                    colored(f"❯ {text}  |  j/k↑↓  /搜索  :命令  g顶  G底  q退出", Colors.OKBLUE)
                )
            else:
                # Use mode-specific hint in normal mode
                if self._vim_mode == "normal":
                    hint.update(
                        colored(
                            "❯ 输入问题开始对话  |  j/k↑↓  /搜索  :命令  g顶  G底  q退出",
                            Colors.WARNING,
                        )
                    )
                # For other modes, _update_vim_mode_indicators handles it
        except NoMatches:
            pass

    # ── Message Search ───────────────────────────────────────────────────

    def action_search_messages(self) -> None:
        """Search through current session messages."""
        if not self.messages:
            self._update_status("⚠️ 当前没有消息可搜索")
            return

        # Show search prompt
        self.notify(
            "🔍 搜索消息\n\n输入 /msg <关键词> 在当前会话中搜索\n示例: /msg transformer",
            title="消息搜索",
            timeout=5,
        )
        self._update_status("📝 输入 /msg <关键词> 搜索当前会话")

    def _search_current_messages(self, query: str) -> None:
        """Search through current session messages using tokenized index."""
        if not query or not self.messages:
            return

        # Tokenize query and find candidate messages via inverted index
        query_tokens = tokenize(query)
        if not query_tokens:
            self._update_status(f"🔍 未找到包含 '{query}' 的消息")
            return

        # Find candidate message indices using inverted index
        candidate_indices: Set[int] = set()
        for token in query_tokens:
            if token in self._msg_index:
                if not candidate_indices:
                    candidate_indices = self._msg_index[token].copy()
                else:
                    candidate_indices &= self._msg_index[token]

        # Also do exact substring search on candidates for phrase matching
        query_lower = query.lower()
        results: List[Dict[str, Any]] = []
        indices_to_check = candidate_indices if candidate_indices else range(len(self.messages))

        for i in indices_to_check:
            msg = self.messages[i]
            content_lower = msg.content.lower()
            if query_lower in content_lower:
                idx = content_lower.find(query_lower)
                start = max(0, idx - 30)
                end = min(len(msg.content), idx + len(query) + 50)
                snippet = msg.content[start:end].strip()

                results.append(
                    {
                        "index": i,
                        "role": msg.role,
                        "timestamp": msg.timestamp,
                        "snippet": snippet,
                        "full_content": msg.content,
                    }
                )

        if not results:
            self._update_status(f"🔍 未找到包含 '{query}' 的消息")
            return

        # Show results
        lines = [f"🔍 搜索结果 ({len(results)} 条):", ""]
        for i, r in enumerate(results[:10], 1):
            role_icon = "❯" if r["role"] == "user" else "🤖"
            ts = Timestamp.format(r["timestamp"])
            preview = r["snippet"][:60] + "..." if len(r["snippet"]) > 60 else r["snippet"]
            lines.append(f"  {i}. {role_icon} [{ts}] {preview}")

        lines.extend(["", "输入 /goto <编号> 跳转到对应消息"])
        self.notify("\n".join(lines), title=f"搜索: {query}", timeout=15)

    def _search_code(self, query: str) -> None:
        """Search project code using jieba tokenizer."""
        if not query:
            return

        try:
            from core.code_indexer import get_code_indexer

            idx = get_code_indexer()
            results = idx.search(query, limit=10)

            if not results:
                self._update_status(f"🔍 代码中未找到 '{query}'")
                return

            lines = [f"📂 代码搜索结果 ({len(results)} 条):", ""]
            for i, r in enumerate(results[:10], 1):
                lines.append(f"  {i}. {r.file}:{r.line}")
                lines.append(f"      {r.content[:70]}...")
            lines.append("")
            lines.append(f"共索引 {idx.size} 个token，{idx.stats()['indexed_files']} 个文件")

            self.notify("\n".join(lines), title=f"代码: {query}", timeout=15)
        except Exception as e:
            self._update_status(f"⚠️ 代码搜索失败: {e}")

    def _goto_message(self, idx_str: str) -> None:
        """Go to a specific message by index from search results."""
        try:
            idx = int(idx_str) - 1  # Convert to 0-based
            if idx < 0 or idx >= len(self.messages):
                self._update_status(f"⚠️ 无效的编号 (1-{len(self.messages)})")
                return

            msg = self.messages[idx]
            # Select and show the message
            self._selected_msg_idx = idx
            self._render_messages_with_selection()

            # Show content in notification for quick view
            content = msg.content[:300] + "..." if len(msg.content) > 300 else msg.content
            role_icon = "❯" if msg.role == "user" else "🤖"
            ts = Timestamp.format(msg.timestamp)
            self.notify(
                f"{role_icon} [{ts}]\n\n{content}",
                title=f"消息 #{idx + 1}",
                timeout=8,
            )
        except ValueError:
            self._update_status("⚠️ 无效的编号格式")

    # ── Command Completion ────────────────────────────────────────────────────

    # Available slash commands
    SLASH_COMMANDS = [
        ("/sessions", "查看所有会话"),
        ("/load", "加载会话 /load <id>"),
        ("/search", "搜索会话 /search <关键词>"),
        ("/msg", "搜索消息 /msg <关键词>"),
        ("/code", "搜索代码 /code <关键词>"),
        ("/goto", "跳转到消息 /goto <编号>"),
        ("/rename", "重命名会话 /rename <标题>"),
        ("/delete", "删除会话 /delete <id>"),
        ("/export", "导出会话"),
        ("/clear", "清空对话"),
        ("/help", "显示帮助"),
    ]

    def action_complete_command(self) -> None:
        """Complete slash commands on Tab press."""
        try:
            inp = cast(Input, self.query_one("#chat-input"))
            current = inp.value

            # Only complete if starts with /
            if not current.startswith("/"):
                return

            # Find matching commands
            matches = [cmd for cmd, desc in self.SLASH_COMMANDS if cmd.startswith(current)]

            if not matches:
                return

            if len(matches) == 1:
                # Single match - complete it
                inp.value = matches[0] + " "
                inp.cursor_position = len(inp.value)
            else:
                # Multiple matches - show suggestions
                lines = ["↹ 候选命令:"]
                for cmd, desc in self.SLASH_COMMANDS:
                    if cmd.startswith(current):
                        lines.append(f"  {cmd} - {desc}")
                self.notify("\n".join(lines), title="命令补全", timeout=3)
        except NoMatches:
            pass

    # ── Session Management ─────────────────────────────────────────────────

    def _load_session(self, session_id: str) -> None:
        """Load a chat session."""
        try:
            prev_messages = self.chat.db.get_chat_messages(session_id)
            if prev_messages:
                for msg in prev_messages:
                    role = "user" if msg["role"] == "user" else "assistant"
                    content = msg["content"]
                    citations = []
                    try:
                        from llm.chat import Citation

                        cites_data = (
                            json.loads(msg.get("citations", "[]"))
                            if isinstance(msg.get("citations"), str)
                            else msg.get("citations", [])
                        )
                        for c in cites_data:
                            citations.append(
                                Citation(
                                    paper_id=c.get("paper_id", ""),
                                    paper_title=c.get("title", ""),
                                    authors=[],
                                    published="",
                                    snippet="",
                                    relevance_score=c.get("score", 0.0),
                                )
                            )
                    except Exception:
                        pass
                    self.messages.append(
                        ChatMessage(
                            role=role,
                            content=content,
                            citations=citations,
                            timestamp=msg.get("created_at", ""),
                        )
                    )
                    if role == "assistant":
                        self._chat_history.append(
                            {"question": "", "answer": content, "citations": citations}
                        )
                # Rebuild search index for loaded messages
                self._msg_index = {}
                for i, msg in enumerate(self.messages):
                    _index_message(tokenize(msg.content), i, self._msg_index)
                self._render_messages()
                self._update_status(f"📂 已加载会话 {session_id}（{len(prev_messages)} 条消息）")
        except Exception as e:
            self._update_status(f"⚠️ 无法加载会话: {e}")

    def _save_to_session(self, question: str, answer: str, citations) -> None:
        """Save a message pair to the current session."""
        if not self.session_id:
            return
        try:
            citations_data = (
                [
                    {"paper_id": c.paper_id, "title": c.paper_title, "score": c.relevance_score}
                    for c in citations
                ]
                if citations
                else []
            )
            self.chat.db.add_chat_message(self.session_id, "user", question, [])
            self.chat.db.add_chat_message(self.session_id, "assistant", answer, citations_data)
        except Exception:
            pass

    def _show_sessions(self) -> None:
        """Show available chat sessions in a notification."""
        try:
            sessions = self.chat.db.get_chat_sessions(limit=20)
            if not sessions:
                self._update_status("📂 没有保存的会话")
                return

            lines = ["📂 可用会话:", ""]
            for i, s in enumerate(sessions, 1):
                sid = s.get("id", "")[:8]
                title = s.get("title", "无标题")[:35]
                updated = s.get("updated_at", "")[:16]
                active = " ◉" if s.get("id") == self.session_id else ""
                lines.append(f"  {i:2}. [{sid}] {title}{active}")
                lines.append(f"      📅 {updated}")
            lines.extend(
                [
                    "",
                    "命令:",
                    "  /load <编号>  加载会话",
                    "  /search <关键词>  搜索会话",
                    "  /rename <标题>  重命名",
                    "  /delete <编号>  删除",
                    "  /export  导出当前会话",
                ]
            )

            self.notify("\n".join(lines), title="会话管理", timeout=20)
        except Exception as e:
            self._update_status(f"⚠️ 无法获取会话列表: {e}")

    def _load_session_by_index(self, idx: str) -> None:
        """Load a session by index number or session ID."""
        try:
            sessions = self.chat.db.get_chat_sessions(limit=50)
            if not sessions:
                self._update_status("📂 没有保存的会话")
                return

            # Try number first
            try:
                num = int(idx)
                if 1 <= num <= len(sessions):
                    self.session_id = sessions[num - 1]["id"]
                    self._load_session(self.session_id)
                    return
            except ValueError:
                pass

            # Try as partial session ID
            for s in sessions:
                if s["id"].startswith(idx):
                    self.session_id = s["id"]
                    self._load_session(self.session_id)
                    return

            self._update_status(f"⚠️ 未找到会话: {idx}")
        except Exception as e:
            self._update_status(f"⚠️ 加载失败: {e}")

    def _search_sessions(self, query: str) -> None:
        """Search chat sessions by keyword."""
        try:
            results = self.chat.db.search_chat_sessions(query, limit=15)
            if not results:
                self._update_status(f"🔍 未找到包含 '{query}' 的会话")
                return

            lines = [f"🔍 搜索结果 ({len(results)}):", ""]
            for i, s in enumerate(results, 1):
                sid = s.get("id", "")[:8]
                title = s.get("title", "无标题")[:35]
                updated = s.get("updated_at", "")[:16]
                lines.append(f"  {i:2}. [{sid}] {title}")
                lines.append(f"      📅 {updated}")
            lines.extend(["", "输入 /load <编号> 加载"])

            self.notify("\n".join(lines), title=f"搜索: {query}", timeout=15)
        except Exception as e:
            self._update_status(f"⚠️ 搜索失败: {e}")

    def _rename_session(self, new_title: str) -> None:
        """Rename the current session."""
        if not self.session_id:
            self._update_status("⚠️ 当前没有活动的会话")
            return
        try:
            self.chat.db.update_chat_session_title(self.session_id, new_title)
            self._update_status(f"✅ 已重命名为: {new_title}")
        except Exception as e:
            self._update_status(f"⚠️ 重命名失败: {e}")

    def _delete_session(self, idx: str) -> None:
        """Delete a session by index or ID."""
        try:
            sessions = self.chat.db.get_chat_sessions(limit=50)
            if not sessions:
                self._update_status("📂 没有可删除的会话")
                return

            session_id = None
            try:
                num = int(idx)
                if 1 <= num <= len(sessions):
                    session_id = sessions[num - 1]["id"]
            except ValueError:
                for s in sessions:
                    if s["id"].startswith(idx):
                        session_id = s["id"]
                        break

            if not session_id:
                self._update_status(f"⚠️ 未找到会话: {idx}")
                return

            self.chat.db.delete_chat_session(session_id)
            self._update_status("✅ 已删除会话")

            if self.session_id == session_id:
                self.session_id = None
                self.messages.clear()
                self._chat_history.clear()
                self._render_messages()
        except Exception as e:
            self._update_status(f"⚠️ 删除失败: {e}")

    def _export_session(self) -> None:
        """Export current session to a markdown file."""
        if not self.messages:
            self._update_status("⚠️ 当前没有会话内容")
            return

        try:
            from pathlib import Path

            timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
            filename = f"chat_export_{timestamp}.md"

            lines = [
                "# Chat Export",
                f"Exported: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}",
                f"Session: {self.session_id or 'N/A'}",
                "---",
                "",
            ]

            for msg in self.messages:
                role = "User" if msg.role == "user" else "Assistant"
                ts = Timestamp.format(msg.timestamp)
                lines.append(f"## {role} ({ts})")
                lines.append(msg.content)
                if msg.citations:
                    lines.append("\n**Citations:**")
                    for c in msg.citations:
                        title = getattr(c, "paper_title", "Unknown")
                        pid = getattr(c, "paper_id", "")
                        lines.append(f"- [{pid}] {title}")
                lines.append("")

            # Get download directory or current directory
            export_path = Path.cwd() / filename

            with open(export_path, "w", encoding="utf-8") as f:
                f.write("\n".join(lines))

            self._update_status(f"✅ 已导出到: {filename}")
        except Exception as e:
            self._update_status(f"⚠️ 导出失败: {e}")

    def _show_help(self) -> None:
        """Show help dialog."""
        self.notify(
            "📖 AI Research OS Chat 帮助\n\n"
            "📝 输入: Enter 发送消息\n"
            "🔄 换行: Shift+Enter\n"
            "🗑️ 清屏: Ctrl+L\n"
            "📜 历史: Ctrl+E\n"
            "💾 导出: Ctrl+S\n"
            "🚪 退出: q\n\n"
            "💡 追问建议会在回复后显示\n"
            "📚 相关论文显示在右侧面板\n\n"
            "🔧 命令:\n"
            "/sessions  - 查看所有会话\n"
            "/load <id>  - 加载会话\n"
            "/search <kw>- 搜索会话\n"
            "/rename <t> - 重命名会话\n"
            "/export     - 导出当前会话\n"
            "/clear      - 清空对话",
            title="帮助",
            timeout=15,
        )

    # ── Suggestions ─────────────────────────────────────────────────────────

    def _show_suggestions(self, citations) -> None:
        """Generate and display follow-up question suggestions."""
        if not citations:
            return

        try:
            from llm.evolution_report import get_smart_followup

            followup = get_smart_followup()
            last_q = self._chat_history[-1]["question"] if self._chat_history else ""

            # Convert citations to context
            ctx_list = [
                type(
                    "Ctx",
                    (),
                    {
                        "paper_id": c.paper_id,
                        "paper_title": c.paper_title,
                        "authors": getattr(c, "authors", []),
                        "published": getattr(c, "published", ""),
                        "snippet": getattr(c, "snippet", ""),
                        "relevance_score": getattr(c, "relevance_score", 0),
                    },
                )
                for c in citations
            ]

            options = followup.generate_options(
                question=last_q,
                answer="",
                citations=ctx_list,
            )

            if options:
                self._suggestions = [opt.query for opt in options[:4]]

                # Display suggestions in notification
                lines = ["💡 追问建议:", ""]
                for i, opt in enumerate(options[:4], 1):
                    q = opt.query[:50]
                    lines.append(f"  {i}. {q}")
                lines.append("")
                lines.append("点击编号复制，或直接在输入框输入")

                self.notify("\n".join(lines), title="追问建议", timeout=10)
        except Exception:
            pass


# ─── CLI command ──────────────────────────────────────────────────────────────


def _build_chat_tui_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "chat-tui",
        help="Full-screen TUI RAG chat with paper context sidebar (Hermes-style)",
        description="Launch a full-screen terminal chat interface for RAG-powered Q&A.",
    )
    p.add_argument(
        "--concept",
        "-c",
        metavar="TAG",
        help="Filter by concept/tag",
    )
    p.add_argument(
        "--limit",
        "-n",
        type=int,
        default=5,
        help="Number of papers to retrieve (default: 5)",
    )
    p.add_argument(
        "--model",
        type=str,
        default=None,
        help="LLM model to use",
    )
    p.add_argument(
        "--session",
        "-s",
        metavar="ID",
        help="Continue from a saved chat session",
    )
    return p  # type: ignore[no-any-return]


def run(args: argparse.Namespace) -> int:
    """Run the TUI chat."""
    db = get_db()
    db.init()

    # Check API key
    api_key = os.getenv("OPENAI_API_KEY")
    if not api_key:
        print(colored("OPENAI_API_KEY not set.", Colors.FAIL), file=sys.stderr)
        print("  export OPENAI_API_KEY=sk-...", file=sys.stderr)
        return 1  # type: ignore[no-any-return]

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

    # Launch TUI with optional session
    app = TUIChatApp(
        chat=chat,
        concept=args.concept,
        limit=args.limit,
        stream=True,
        session_id=args.session,
    )
    app.run()
    return 0
