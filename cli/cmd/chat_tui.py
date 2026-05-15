     1|"""
     2|Hermes-style TUI Chat: Full-screen RAG chat with paper context sidebar.
     3|
     4|Layout:
     5|┌──────────────────────────────────────────────────────────────────┐
     6|│  AI Research OS Chat                               [?] [q]       │
     7|├────────────────────────────────────────┬─────────────────────────┤
     8|│                                        │ 📚 相关论文             │
     9|│  [User message]                       │                         │
    10|│                                        │ ▶ Paper Title 1        │
    11|│  [AI response]                        │   Score: 0.92          │
    12|│  [streaming...]                       │   BERT: Pre-training... │
    13|│                                        │                         │
    14|│                                        │ ▶ Paper Title 2        │
    15|│                                        │   Score: 0.87          │
    16|│                                        │   ...                   │
    17|│                                        │                         │
    18|│                                        │ [+3 more] (if >3)      │
    19|├────────────────────────────────────────┴─────────────────────────┤
    20|│  ❯ [Type your question...                          ] [Enter ⏎]  │
    21|└──────────────────────────────────────────────────────────────────┘
    22|"""
# [LEGACY] Full TUI chat interface — Python-specific, ~2800 lines

    23|
    24|from __future__ import annotations
    25|
    26|import argparse
    27|import asyncio
    28|import json
    29|import os
    30|import re
    31|import sys
    32|from dataclasses import dataclass
    33|from datetime import datetime
    34|from typing import Any, Callable, Dict, List, Optional, Set, cast
    35|
    36|# Load .env from current working directory (unified via cli._shared)
    37|from cli._shared import load_dotenv
    38|
    39|load_dotenv()
    40|
    41|from textual.app import App, ComposeResult
    42|from textual.binding import Binding
    43|from textual.containers import Container, Horizontal, VerticalScroll
    44|from textual.screen import Screen
    45|from textual.widgets import Button, Header, Label, Static, Input
    46|from textual.css.query import NoMatches
    47|
    48|from cli._shared import get_db, Colors, colored
    49|from cli.warp import WarpBlocks
    50|from llm.chat import RagChat
    51|from llm.reasoning import ReasoningBlock
    52|from llm.friction_tracker import FrictionTracker
    53|
    54|
    55|# ─── Message data ──────────────────────────────────────────────────────────────
    56|
    57|
    58|@dataclass
    59|class ChatMessage:
    60|    """A single chat message."""
    61|
    62|    role: str  # "user" or "assistant"
    63|    content: str
    64|    citations: list  # List[Citation]
    65|    timestamp: str = ""
    66|    edited: bool = False
    67|
    68|
    69|@dataclass
    70|class StreamConfig:
    71|    """Configuration for streaming behavior."""
    72|
    73|    batch_size: int = 3  # Characters per batch update
    74|    max_line_width: int = 120  # Auto-wrap at this width
    75|    typing_indicator: bool = True  # Show typing animation
    76|
    77|
    78|# ─── Message Indexer ─────────────────────────────────────────────────────────
    79|
    80|
    81|def tokenize(text: str) -> Set[str]:
    82|    """Extract searchable tokens from text (simple word tokenization)."""
    83|    # Lowercase, keep alphanumeric+Chinese chars, split by whitespace/punctuation
    84|    tokens = re.findall(r"[\w\u4e00-\u9fff]+", text.lower())
    85|    # Filter out very short tokens (1-2 chars often noise)
    86|    return {t for t in tokens if len(t) >= 2}
    87|
    88|
    89|def _index_message(tokens: Set[str], msg_idx: int, index: Dict[str, Set[int]]) -> None:
    90|    """Add message tokens to the inverted index."""
    91|    for token in tokens:
    92|        if token not in index:
    93|            index[token] = set()
    94|        index[token].add(msg_idx)
    95|
    96|
    97|# ─── Markdown Parser ──────────────────────────────────────────────────────────
    98|
    99|
   100|class SimpleMarkdown:
   101|    """Simple markdown to ANSI-colored text converter for TUI.
   102|
   103|    Delegates code-block rendering to shared WarpBlocks to avoid duplication.
   104|    """
   105|
   106|    # Re-use WarpBlocks colors for compatibility with existing CSS/text patterns
   107|    COLORS = WarpBlocks.C
   108|    LANG_ALIASES = WarpBlocks.LANG_ALIASES if hasattr(WarpBlocks, "LANG_ALIASES") else {}
   109|    PY_KEYWORDS = WarpBlocks.PY_KEYWORDS if hasattr(WarpBlocks, "PY_KEYWORDS") else frozenset()
   110|    JS_KEYWORDS = WarpBlocks.JS_KEYWORDS if hasattr(WarpBlocks, "JS_KEYWORDS") else frozenset()
   111|
   112|    # Delegate tokenizers to shared module
   113|    _tokenize_python = (
   114|        staticmethod(WarpBlocks._tokenize_python)
   115|        if hasattr(WarpBlocks, "_tokenize_python")
   116|        else None
   117|    )
   118|    _tokenize_javascript = (
   119|        staticmethod(WarpBlocks._tokenize_javascript)
   120|        if hasattr(WarpBlocks, "_tokenize_javascript")
   121|        else None
   122|    )
   123|    _tokenize_json = (
   124|        staticmethod(WarpBlocks._tokenize_json) if hasattr(WarpBlocks, "_tokenize_json") else None
   125|    )
   126|    _tokenize_generic = (
   127|        staticmethod(WarpBlocks._tokenize_generic)
   128|        if hasattr(WarpBlocks, "_tokenize_generic")
   129|        else None
   130|    )
   131|    _tokenize = staticmethod(WarpBlocks._tokenize) if hasattr(WarpBlocks, "_tokenize") else None
   132|
   133|    @classmethod
   134|    def _render_code_block(cls, lang: str, code: str, width: int = 80) -> str:
   135|        """Render a code block with syntax highlighting and line numbers.
   136|
   137|        Delegates to the shared WarpBlocks implementation.
   138|        """
   139|        return WarpBlocks.code_block(lang, code, width=width)
   140|
   141|    @classmethod
   142|    def parse(cls, text: str) -> str:
   143|        """Parse markdown to styled text."""
   144|        if not text:
   145|            return text
   146|
   147|        result = text
   148|
   149|        # Process code blocks first (before other markdown)
   150|        def replace_code_block(m):
   151|            lang = m.group(1).strip() or "text"
   152|            code = m.group(2)
   153|            return "\n" + cls._render_code_block(lang, code) + "\n"
   154|
   155|        result = re.sub(r"```(\w*)\n?(.*?)```", replace_code_block, result, flags=re.DOTALL)
   156|
   157|        # Inline code (`code`)
   158|        result = re.sub(r"`([^`]+)`", r" [\1] ", result)
   159|
   160|        # Headers (# ## ###)
   161|        result = re.sub(r"^### (.+)$", r"\n━━━ \1 ━━━\n", result, flags=re.MULTILINE)
   162|        result = re.sub(r"^## (.+)$", r"\n━━ \1 �━━\n", result, flags=re.MULTILINE)
   163|        result = re.sub(r"^# (.+)$", r"\n━ \1 ━\n", result, flags=re.MULTILINE)
   164|
   165|        # Bold (**text** or __text__)
   166|        result = re.sub(r"\*\*(.+?)\*\*", r"[\1]", result)
   167|        result = re.sub(r"__(.+?)__", r"[\1]", result)
   168|
   169|        # Italic (*text* or _text_)
   170|        result = re.sub(r"\*(.+?)\*", r"/\1/", result)
   171|        result = re.sub(r"_(.+?)_", r"/\1/", result)
   172|
   173|        # Lists (- item or * item)
   174|        result = re.sub(r"^[\-\*] (.+)$", r"  • \1", result, flags=re.MULTILINE)
   175|
   176|        # Numbered lists (1. item)
   177|        result = re.sub(r"^\d+\. (.+)$", r"  \g<0>", result, flags=re.MULTILINE)
   178|
   179|        # Blockquotes (>)
   180|        result = re.sub(r"^> (.+)$", r"  │ \1", result, flags=re.MULTILINE)
   181|
   182|        # Horizontal rules (---)
   183|        result = re.sub(r"^---+$", "─" * 50, result, flags=re.MULTILINE)
   184|
   185|        return result
   186|
   187|    @classmethod
   188|    def wrap_lines(cls, text: str, width: int = 120) -> str:
   189|        """Wrap text to specified width."""
   190|        lines = text.split("\n")
   191|        wrapped = []
   192|        for line in lines:
   193|            if len(line) <= width:
   194|                wrapped.append(line)
   195|            else:
   196|                # Break at word boundaries
   197|                words = line.split()
   198|                current = ""
   199|                for word in words:
   200|                    if len(current) + len(word) + 1 <= width:
   201|                        current += (" " if current else "") + word
   202|                    else:
   203|                        if current:
   204|                            wrapped.append(current)
   205|                        current = word
   206|                if current:
   207|                    wrapped.append(current)
   208|        return "\n".join(wrapped)
   209|
   210|
   211|# ─── Timestamp Formatter ──────────────────────────────────────────────────────
   212|
   213|
   214|class Timestamp:
   215|    """Format timestamps for display."""
   216|
   217|    @staticmethod
   218|    def now() -> str:
   219|        """Get current timestamp."""
   220|        return datetime.now().strftime("%H:%M")
   221|
   222|    @staticmethod
   223|    def format(ts: str) -> str:
   224|        """Format a timestamp string."""
   225|        if not ts:
   226|            return ""
   227|        try:
   228|            dt = datetime.fromisoformat(ts)
   229|            return dt.strftime("%H:%M")
   230|        except Exception:
   231|            return ts[:5] if len(ts) > 5 else ts
   232|
   233|
   234|# ─── Loading Animation ──────────────────────────────────────────────────────────
   235|
   236|
   237|class LoadingDots:
   238|    """Animated loading indicator with braille dots."""
   239|
   240|    FRAMES = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
   241|
   242|    def __init__(self):
   243|        self.frame = 0
   244|
   245|    def next(self) -> str:
   246|        """Get next frame."""
   247|        frame = self.FRAMES[self.frame % len(self.FRAMES)]
   248|        self.frame += 1
   249|        return frame  # type: ignore[no-any-return]
   250|
   251|
   252|class TypingCursor:
   253|    """Typing cursor with blink animation."""
   254|
   255|    CURSOR_CHARS = ["▌", "█"]  # Block cursor variants
   256|    BLINK_FRAMES = 10  # Frames per blink cycle
   257|
   258|    def __init__(self):
   259|        self.frame = 0
   260|        self.visible = True
   261|
   262|    def next(self) -> str:
   263|        """Get next cursor state."""
   264|        self.frame += 1
   265|        if self.frame % self.BLINK_FRAMES == 0:
   266|            self.visible = not self.visible
   267|        return self.CURSOR_CHARS[0] if self.visible else " "
   268|
   269|    def reset(self) -> None:
   270|        """Reset cursor state."""
   271|        self.frame = 0
   272|        self.visible = True
   273|
   274|
   275|class Typewriter:
   276|    """Character-by-character typing effect for streaming text."""
   277|
   278|    def __init__(self, text: str = "", delay: int = 0):
   279|        self.text = text
   280|        self.index = 0
   281|        self.delay = delay
   282|        self.frame = 0
   283|
   284|    def tick(self) -> str | None:
   285|        """Get next characters (up to batch_size) for typewriter effect."""
   286|        self.frame += 1
   287|        if self.frame % max(1, self.delay) != 0:
   288|            return None  # Skip this frame
   289|
   290|        batch = min(3, len(self.text) - self.index)
   291|        if batch <= 0:
   292|            return ""
   293|
   294|        result = self.text[self.index : self.index + batch]
   295|        self.index += batch
   296|        return result
   297|
   298|    def is_done(self) -> bool:
   299|        """Check if typing is complete."""
   300|        return self.index >= len(self.text)
   301|
   302|    def progress(self) -> float:
   303|        """Get progress 0.0 to 1.0."""
   304|        if not self.text:
   305|            return 1.0
   306|        return self.index / len(self.text)
   307|
   308|
   309|# ─── KG Visualizer (utility class, not a Textual widget) ─────────────────────
   310|
   311|
   312|class KGVisualizer:
   313|    """Render a KG neighborhood as an ASCII directed graph.
   314|
   315|    Input: list of (node_id, node_type, label, connections) tuples and
   316|           list of (source_id, target_id, relation_type) edges.
   317|    Output: multi-line ASCII art string.
   318|    """
   319|
   320|    MAX_LABEL = 12
   321|
   322|    @classmethod
   323|    def _trunc(cls, label: str) -> str:
   324|        if len(label) > cls.MAX_LABEL:
   325|            return label[: cls.MAX_LABEL - 1] + "…"
   326|        return label
   327|
   328|    @classmethod
   329|    def _node_str(cls, node_type: str, label: str) -> str:
   330|        short_type = (node_type[:4] + ":") if node_type else "Node:"
   331|        return f"[{short_type}{cls._trunc(label)}]"
   332|
   333|    @classmethod
   334|    def render(
   335|        cls,
   336|        nodes: list[tuple],
   337|        edges: list[tuple],
   338|        root_id: str = "",
   339|        max_width: int = 40,
   340|        max_depth: int = 2,
   341|    ) -> str:
   342|        """Render KG as ASCII art using BFS layering."""
   343|        if not nodes:
   344|            return "  (no KG data)"
   345|
   346|        # node_id -> (node_type, label)
   347|        node_info: Dict[str, tuple[str, str]] = {n[0]: (n[1], n[2]) for n in nodes}
   348|
   349|        # Build adjacency: node_id -> list of (neighbor_id, relation)
   350|        adj: Dict[str, list[tuple[str, str]]] = {n[0]: [] for n in nodes}
   351|        for src, tgt, rel in edges:
   352|            if src in adj and tgt in node_info:
   353|                adj[src].append((tgt, rel))
   354|            if tgt in adj and src in node_info:
   355|                adj[tgt].append((src, rel))
   356|
   357|        # BFS from root to collect reachable nodes within max_depth
   358|        if not root_id:
   359|            root_id = nodes[0][0]
   360|
   361|        visited: Dict[str, int] = {}
   362|        queue = [(root_id, 0)]
   363|        while queue:
   364|            nid, d = queue.pop(0)
   365|            if nid in visited or d > max_depth:
   366|                continue
   367|            visited[nid] = d
   368|            for neighbor, _ in adj.get(nid, []):
   369|                if neighbor not in visited:
   370|                    queue.append((neighbor, d + 1))
   371|
   372|        if not visited:
   373|            return "  (no reachable KG data)"
   374|
   375|        # Group by depth level
   376|        levels: Dict[int, list[tuple[str, str]]] = {}
   377|        for nid, depth in visited.items():
   378|            if nid in node_info:
   379|                ntype, label = node_info[nid]
   380|                levels.setdefault(depth, []).append((nid, cls._node_str(ntype, label)))
   381|
   382|        lines: list[str] = []
   383|
   384|        for depth in sorted(levels.keys()):
   385|            level_nodes = levels[depth]
   386|            if depth == 0:
   387|                lines.append("  " + "  ".join(lbl for _, lbl in level_nodes))
   388|            else:
   389|                n = len(level_nodes)
   390|                # Build connector row
   391|                if n == 1:
   392|                    connector = "  │ " + "─" * (len(level_nodes[0][1]) + 2)
   393|                else:
   394|                    seg = "─┬─"
   395|                    connector = (
   396|                        "  │ "
   397|                        + ("─" * len(level_nodes[0][1]) + seg) * (n - 1)
   398|                        + "─" * len(level_nodes[0][1])
   399|                    )
   400|                lines.append(connector)
   401|                # Node row
   402|                row = "  │ " + "  │ ".join(lbl for _, lbl in level_nodes)
   403|                lines.append(row)
   404|
   405|        return "\n".join(lines) if lines else "  (no KG data)"
   406|
   407|
   408|# ─── SidebarKGView ───────────────────────────────────────────────────────────────
   409|
   410|
   411|class SidebarKGView(Static):
   412|    """A toggleable KG ASCII view shown in the sidebar below the paper cards."""
   413|
   414|    MAX_LINES = 20
   415|
   416|    def __init__(self, on_close=None, **kwargs):
   417|        self.on_close = on_close
   418|        super().__init__(**kwargs)
   419|
   420|    def show_kg(self, ascii_graph: str, truncated: int = 0) -> None:
   421|        """Display the ASCII KG graph in this widget."""
   422|        content = ascii_graph
   423|        if truncated > 0:
   424|            content += f"\n  … {truncated} more lines"
   425|        self.update(colored(content, "#8be9fd"))
   426|        self.remove_class("hidden")
   427|
   428|    def hide_kg(self) -> None:
   429|        """Hide the KG view."""
   430|        self.add_class("hidden")
   431|
   432|    def show_empty(self, message: str) -> None:
   433|        """Show an empty or unavailable state."""
   434|        self.update(colored(f"  {message}", Colors.WARNING))
   435|        self.remove_class("hidden")
   436|
   437|
   438|# ─── Widgets ──────────────────────────────────────────────────────────────────
   439|
   440|
   441|class ChatBubble(Static):
   442|    """A chat message bubble widget with markdown support and typing effect."""
   443|
   444|    # Streaming indicator frames
   445|    STREAM_FRAMES = ["○", "◔", "◑", "◕", "●"]  # Expanding circles
   446|
   447|    def __init__(
   448|        self,
   449|        msg: ChatMessage,
   450|        config: StreamConfig | None = None,
   451|        is_streaming: bool = False,
   452|        **kwargs,
   453|    ):
   454|        self.msg = msg
   455|        self.config = config or StreamConfig()
   456|        self.is_streaming = is_streaming
   457|        self.stream_frame = 0
   458|        super().__init__(**kwargs)
   459|
   460|    def get_streaming_indicator(self) -> str:
   461|        """Get animated streaming indicator."""
   462|        if not self.is_streaming:
   463|            return ""
   464|        self.stream_frame = (self.stream_frame + 1) % len(self.STREAM_FRAMES)
   465|        return colored(f" {self.STREAM_FRAMES[self.stream_frame]}", Colors.WARNING)
   466|
   467|    def compose(self) -> ComposeResult:
   468|        ts = Timestamp.format(self.msg.timestamp) if self.msg.timestamp else Timestamp.now()
   469|
   470|        if self.msg.role == "user":
   471|            content = SimpleMarkdown.wrap_lines(self.msg.content, self.config.max_line_width - 20)
   472|            yield Static(
   473|                colored(f"❯ {content}", Colors.OKGREEN),
   474|                classes="user-msg",
   475|            )
   476|            yield Static(
   477|                colored(f"  {ts}", Colors.OKBLUE + " dim"),
   478|                classes="timestamp",
   479|            )
   480|        else:
   481|            # AI response with markdown
   482|            parsed = SimpleMarkdown.parse(self.msg.content)
   483|            wrapped = SimpleMarkdown.wrap_lines(parsed, self.config.max_line_width - 10)
   484|
   485|            # Add streaming indicator
   486|            streaming_suffix = self.get_streaming_indicator() if self.is_streaming else ""
   487|
   488|            yield Static(
   489|                colored(f"🤖 {wrapped}{streaming_suffix}", Colors.OKBLUE),
   490|                classes="ai-msg" + (" streaming" if self.is_streaming else ""),
   491|            )
   492|            yield Static(
   493|                colored(f"  {ts}", Colors.OKBLUE + " dim"),
   494|                classes="timestamp",
   495|            )
   496|            # Citations
   497|            if self.msg.citations:
   498|                yield self._render_citations()
   499|
   500|    def _render_citations(self) -> Static:
   501|
