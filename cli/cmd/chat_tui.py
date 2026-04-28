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
import json
import os
import re
import sys
from dataclasses import dataclass
from datetime import datetime
from typing import Callable, List

# Load .env from current working directory (unified via cli._shared)
from cli._shared import load_dotenv
load_dotenv()

from textual.app import App, ComposeResult
from textual.binding import Binding
from textual.containers import Container, Horizontal, VerticalScroll
from textual.widgets import Button, Header, Label, Static, Input
from textual.css.query import NoMatches

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
    edited: bool = False


@dataclass
class StreamConfig:
    """Configuration for streaming behavior."""
    batch_size: int = 3        # Characters per batch update
    max_line_width: int = 120  # Auto-wrap at this width
    typing_indicator: bool = True  # Show typing animation


# ─── Markdown Parser ──────────────────────────────────────────────────────────


class SimpleMarkdown:
    """Simple markdown to ANSI-colored text converter for TUI."""

    # ANSI color codes
    COLORS = {
        'bold': '\033[1m',
        'italic': '\033[3m',
        'code': '\033[92m',      # Green
        'code_bg': '\033[40m',   # Dark background
        'link': '\033[94m',      # Blue
        'header': '\033[95m',    # Magenta
        'list': '\033[96m',      # Cyan
        'quote': '\033[90m',     # Gray
        'reset': '\033[0m',
        # Syntax highlighting colors
        'syn_keyword': '\033[38;5;141m',   # Purple
        'syn_string': '\033[38;5;114m',    # Light blue
        'syn_number': '\033[38;5;215m',   # Orange
        'syn_comment': '\033[38;5;242m',   # Gray
        'syn_function': '\033[38;5;79m',   # Cyan
        'syn_class': '\033[38;5;147m',    # Light purple
        'syn_decorator': '\033[38;5;197m',# Pink
        'syn_operator': '\033[38;5;180m', # Yellow-brown
    }

    # Language alias mapping
    LANG_ALIASES = {
        'py': 'python', 'js': 'javascript', 'ts': 'typescript',
        'sh': 'bash', 'shell': 'bash', 'zsh': 'bash',
        'rb': 'ruby', 'rs': 'rust', 'go': 'go',
        'yml': 'yaml', 'tf': 'hcl', 'dockerfile': 'dockerfile',
        'jsonc': 'json', 'jsx': 'javascript', 'tsx': 'typescript',
    }

    # Python keywords
    PY_KEYWORDS = frozenset({
        'and', 'as', 'assert', 'async', 'await', 'break', 'class', 'continue',
        'def', 'del', 'elif', 'else', 'except', 'finally', 'for', 'from',
        'global', 'if', 'import', 'in', 'is', 'lambda', 'nonlocal', 'not',
        'or', 'pass', 'raise', 'return', 'try', 'while', 'with', 'yield',
        'True', 'False', 'None', 'self', 'cls',
    })

    # JavaScript keywords
    JS_KEYWORDS = frozenset({
        'async', 'await', 'break', 'case', 'catch', 'class', 'const', 'continue',
        'debugger', 'default', 'delete', 'do', 'else', 'export', 'extends',
        'finally', 'for', 'function', 'if', 'import', 'in', 'instanceof',
        'let', 'new', 'of', 'return', 'static', 'super', 'switch', 'this',
        'throw', 'try', 'typeof', 'var', 'void', 'while', 'with', 'yield',
        'true', 'false', 'null', 'undefined',
    })

    @classmethod
    def _tokenize_python(cls, code: str) -> str:
        """Highlight Python code with syntax colors."""
        lines = code.split('\n')
        result_lines = []

        for i, line in enumerate(lines):
            if not line.strip() or line.strip().startswith('#'):
                result_lines.append(line)
                continue

            highlighted = []
            pos = 0
            ln = len(line)

            while pos < ln:
                # Decorator
                if line[pos] == '@':
                    end = pos + 1
                    while end < ln and (line[end].isalnum() or line[end] in '_'):
                        end += 1
                    highlighted.append(f"{cls.COLORS['syn_decorator']}{line[pos:end]}{cls.COLORS['reset']}")
                    pos = end
                    continue

                # String (single/double/triple quotes)
                if line[pos] in '"\'':
                    quote = line[pos]
                    if line[pos:pos+3] == quote * 3:
                        quote = quote * 3
                    end = pos + len(quote)
                    while end < ln:
                        if line[end] == '\\' and end + 1 < ln:
                            end += 2
                            continue
                        if line[end:].startswith(quote):
                            end += len(quote)
                            break
                        end += 1
                    highlighted.append(f"{cls.COLORS['syn_string']}{line[pos:end]}{cls.COLORS['reset']}")
                    pos = end
                    continue

                # Comment
                if line[pos] == '#':
                    highlighted.append(f"{cls.COLORS['syn_comment']}{line[pos:]}{cls.COLORS['reset']}")
                    break

                # Word (keyword, function, class, number)
                if line[pos].isalnum() or line[pos] == '_':
                    end = pos
                    while end < ln and (line[end].isalnum() or line[end] in '_'):
                        end += 1
                    word = line[pos:end]

                    # Check if it's a keyword
                    if word in cls.PY_KEYWORDS:
                        highlighted.append(f"{cls.COLORS['syn_keyword']}{word}{cls.COLORS['reset']}")
                    elif end < ln and line[end] == '(':
                        highlighted.append(f"{cls.COLORS['syn_function']}{word}{cls.COLORS['reset']}")
                    elif word[0].isupper() and '_' not in word and len(word) > 1:
                        highlighted.append(f"{cls.COLORS['syn_class']}{word}{cls.COLORS['reset']}")
                    elif word.replace('.', '').isdigit():
                        highlighted.append(f"{cls.COLORS['syn_number']}{word}{cls.COLORS['reset']}")
                    else:
                        highlighted.append(word)
                    pos = end
                    continue

                # Operators
                if line[pos] in '=!<>+-*/%&|^~:':
                    highlighted.append(f"{cls.COLORS['syn_operator']}{line[pos]}{cls.COLORS['reset']}")
                    pos += 1
                    continue

                highlighted.append(line[pos])
                pos += 1

            result_lines.append(''.join(highlighted))

        return '\n'.join(result_lines)

    @classmethod
    def _tokenize_javascript(cls, code: str) -> str:
        """Highlight JavaScript/TypeScript code with syntax colors."""
        lines = code.split('\n')
        result_lines = []

        for line in lines:
            if not line.strip() or line.strip().startswith('//'):
                result_lines.append(line)
                continue

            highlighted = []
            pos = 0
            ln = len(line)

            while pos < ln:
                # String
                if line[pos] in '"\'':
                    quote = line[pos]
                    end = pos + 1
                    while end < ln:
                        if line[end] == '\\':
                            end += 2
                            continue
                        if line[end] == quote:
                            end += 1
                            break
                        end += 1
                    highlighted.append(f"{cls.COLORS['syn_string']}{line[pos:end]}{cls.COLORS['reset']}")
                    pos = end
                    continue

                # Template literal
                if line[pos] == '`':
                    end = pos + 1
                    while end < ln:
                        if line[end] == '\\':
                            end += 2
                            continue
                        if line[end] == '`':
                            end += 1
                            break
                        end += 1
                    highlighted.append(f"{cls.COLORS['syn_string']}{line[pos:end]}{cls.COLORS['reset']}")
                    pos = end
                    continue

                # Comment
                if line[pos:pos+2] == '//':
                    highlighted.append(f"{cls.COLORS['syn_comment']}{line[pos:]}{cls.COLORS['reset']}")
                    break

                # Word
                if line[pos].isalnum() or line[pos] == '_':
                    end = pos
                    while end < ln and (line[end].isalnum() or line[end] in '_$'):
                        end += 1
                    word = line[pos:end]

                    if word in cls.JS_KEYWORDS:
                        highlighted.append(f"{cls.COLORS['syn_keyword']}{word}{cls.COLORS['reset']}")
                    elif end < ln and line[end] == '(':
                        highlighted.append(f"{cls.COLORS['syn_function']}{word}{cls.COLORS['reset']}")
                    elif word[0].isupper() and len(word) > 1:
                        highlighted.append(f"{cls.COLORS['syn_class']}{word}{cls.COLORS['reset']}")
                    elif word.replace('.', '').isdigit():
                        highlighted.append(f"{cls.COLORS['syn_number']}{word}{cls.COLORS['reset']}")
                    else:
                        highlighted.append(word)
                    pos = end
                    continue

                # Operators
                if line[pos] in '=!<>+-*/%&|^~?:':
                    highlighted.append(f"{cls.COLORS['syn_operator']}{line[pos]}{cls.COLORS['reset']}")
                    pos += 1
                    continue

                highlighted.append(line[pos])
                pos += 1

            result_lines.append(''.join(highlighted))

        return '\n'.join(result_lines)

    @classmethod
    def _tokenize_json(cls, code: str) -> str:
        """Highlight JSON with syntax colors."""
        result = []
        i = 0
        ln = len(code)

        while i < ln:
            # String
            if code[i] == '"':
                end = i + 1
                while end < ln:
                    if code[end] == '\\':
                        end += 2
                        continue
                    if code[end] == '"':
                        end += 1
                        break
                    end += 1
                # Check if it's a key (followed by :)
                rest = code[end:].lstrip()
                is_key = rest.startswith(':')
                color = cls.COLORS['syn_keyword'] if is_key else cls.COLORS['syn_string']
                result.append(f"{color}{code[i:end]}{cls.COLORS['reset']}")
                i = end
                continue

            # Number
            if code[i].isdigit() or (code[i] == '-' and i + 1 < ln and code[i+1].isdigit()):
                end = i
                if code[end] == '-':
                    end += 1
                while end < ln and (code[end].isdigit() or code[end] in '.eE+-'):
                    end += 1
                result.append(f"{cls.COLORS['syn_number']}{code[i:end]}{cls.COLORS['reset']}")
                i = end
                continue

            # Keywords
            for kw in ('true', 'false', 'null'):
                if code[i:i+len(kw)] == kw and (i + len(kw) >= ln or not code[i+len(kw)].isalnum()):
                    result.append(f"{cls.COLORS['syn_keyword']}{kw}{cls.COLORS['reset']}")
                    i += len(kw)
                    break
            else:
                result.append(code[i])
                i += 1

        return ''.join(result)

    @classmethod
    def _tokenize_generic(cls, code: str) -> str:
        """Generic highlighting for unknown languages."""
        lines = code.split('\n')
        result_lines = []

        for line in lines:
            # Simple comment detection
            if ' #' in line or line.lstrip().startswith('#'):
                idx = line.find('#')
                if idx >= 0:
                    result_lines.append(
                        f"{line[:idx]}{cls.COLORS['syn_comment']}{line[idx:]}{cls.COLORS['reset']}"
                    )
                    continue
            result_lines.append(line)

        return '\n'.join(result_lines)

    @classmethod
    def _tokenize(cls, lang: str, code: str) -> str:
        """Tokenize code by language."""
        lang = cls.LANG_ALIASES.get(lang.lower(), lang.lower())

        if lang == 'python':
            return cls._tokenize_python(code)
        elif lang in ('javascript', 'typescript', 'jsx', 'tsx'):
            return cls._tokenize_javascript(code)
        elif lang in ('json', 'jsonc'):
            return cls._tokenize_json(code)
        else:
            return cls._tokenize_generic(code)

    @classmethod
    def _render_code_block(cls, lang: str, code: str, width: int = 80) -> str:
        """Render a code block with syntax highlighting and line numbers."""
        lines = code.split('\n')
        highlighted = cls._tokenize(lang, code).split('\n')

        # Calculate line number width
        num_width = len(str(len(lines)))

        # Language display name
        lang_display = lang.upper() if lang else 'CODE'

        # Top border
        top = f"┌─ {lang_display} ─" + "─" * max(0, width - len(lang_display) - 8) + "┐"

        result_lines = [top]
        for i, (orig, hl) in enumerate(zip(lines, highlighted)):
            line_num = str(i + 1).rjust(num_width)
            # Truncate long lines
            max_content = width - num_width - 4  # account for " │ "
            if len(orig) > max_content:
                hl = hl[:max_content] + cls.COLORS['syn_comment'] + ' …' + cls.COLORS['reset']
            content = f"{cls.COLORS['syn_comment']}{line_num}{cls.COLORS['reset']} │ {hl}"
            result_lines.append(f"│ {content}")

        # Bottom border
        result_lines.append("└" + "─" * (width - 2) + "┘")

        return '\n'.join(result_lines)

    @classmethod
    def parse(cls, text: str) -> str:
        """Parse markdown to styled text."""
        if not text:
            return text

        result = text

        # Process code blocks first (before other markdown)
        def replace_code_block(m):
            lang = m.group(1).strip() or 'text'
            code = m.group(2)
            return '\n' + cls._render_code_block(lang, code) + '\n'

        result = re.sub(r'```(\w*)\n?(.*?)```', replace_code_block, result, flags=re.DOTALL)

        # Inline code (`code`)
        result = re.sub(r'`([^`]+)`', r' [\1] ', result)

        # Headers (# ## ###)
        result = re.sub(r'^### (.+)$', r'\n━━━ \1 ━━━\n', result, flags=re.MULTILINE)
        result = re.sub(r'^## (.+)$', r'\n━━ \1 �━━\n', result, flags=re.MULTILINE)
        result = re.sub(r'^# (.+)$', r'\n━ \1 ━\n', result, flags=re.MULTILINE)

        # Bold (**text** or __text__)
        result = re.sub(r'\*\*(.+?)\*\*', r'[\1]', result)
        result = re.sub(r'__(.+?)__', r'[\1]', result)

        # Italic (*text* or _text_)
        result = re.sub(r'\*(.+?)\*', r'/\1/', result)
        result = re.sub(r'_(.+?)_', r'/\1/', result)

        # Lists (- item or * item)
        result = re.sub(r'^[\-\*] (.+)$', r'  • \1', result, flags=re.MULTILINE)

        # Numbered lists (1. item)
        result = re.sub(r'^\d+\. (.+)$', r'  \g<0>', result, flags=re.MULTILINE)

        # Blockquotes (>)
        result = re.sub(r'^> (.+)$', r'  │ \1', result, flags=re.MULTILINE)

        # Horizontal rules (---)
        result = re.sub(r'^---+$', '─' * 50, result, flags=re.MULTILINE)

        return result

    @classmethod
    def wrap_lines(cls, text: str, width: int = 120) -> str:
        """Wrap text to specified width."""
        lines = text.split('\n')
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
        return '\n'.join(wrapped)


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

    FRAMES = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏']

    def __init__(self):
        self.frame = 0

    def next(self) -> str:
        """Get next frame."""
        frame = self.FRAMES[self.frame % len(self.FRAMES)]
        self.frame += 1
        return frame


class TypingCursor:
    """Typing cursor with blink animation."""

    CURSOR_CHARS = ['▌', '█']  # Block cursor variants
    BLINK_FRAMES = 10  # Frames per blink cycle

    def __init__(self):
        self.frame = 0
        self.visible = True

    def next(self) -> str:
        """Get next cursor state."""
        self.frame += 1
        if self.frame % self.BLINK_FRAMES == 0:
            self.visible = not self.visible
        return self.CURSOR_CHARS[0] if self.visible else ' '

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

    def tick(self) -> str:
        """Get next characters (up to batch_size) for typewriter effect."""
        self.frame += 1
        if self.frame % max(1, self.delay) != 0:
            return None  # Skip this frame

        batch = min(3, len(self.text) - self.index)
        if batch <= 0:
            return ""

        result = self.text[self.index:self.index + batch]
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


# ─── Widgets ──────────────────────────────────────────────────────────────────


class ChatBubble(Static):
    """A chat message bubble widget with markdown support and typing effect."""

    # Streaming indicator frames
    STREAM_FRAMES = ['○', '◔', '◑', '◕', '●']  # Expanding circles

    def __init__(self, msg: ChatMessage, config: StreamConfig = None,
                 is_streaming: bool = False, **kwargs):
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
            title = getattr(c, 'paper_title', '')[:50]
            score = getattr(c, 'relevance_score', 0)
            lines.append(f"  {i}. 📖 {title} (score={score:.2f})")
        if len(self.msg.citations) > 3:
            lines.append(f"  [+{len(self.msg.citations)-3} more]")
        return Static(colored('\n'.join(lines), Colors.WARNING), classes="cite-list")


class PaperCard(Static, can_focus=True):
    """An enhanced paper card with click-to-expand and rich metadata."""

    def __init__(self, citation, index: int, expanded: bool = False, on_select=None, **kwargs):
        self.citation = citation
        self.index = index
        self.expanded = expanded
        self.on_select = on_select
        super().__init__(**kwargs)

    def render(self) -> str:
        score = getattr(self.citation, 'relevance_score', 0)
        title = getattr(self.citation, 'paper_title', 'Unknown')
        snippet = getattr(self.citation, 'snippet', '')
        pid = getattr(self.citation, 'paper_id', '')
        authors = getattr(self.citation, 'authors', [])
        published = getattr(self.citation, 'published', '')[:10]
        abstract = getattr(self.citation, 'abstract', '')[:200]
        categories = getattr(self.citation, 'categories', [])[:5]

        # Collapsed view
        expand_icon = "▼" if self.expanded else "▶"
        header = f"{expand_icon} [{self.index+1}] {title[:45]}"
        meta = f"    📄 {pid} | ⭐ {score:.2f} | 📅 {published}"

        if not self.expanded:
            # Compact view with preview
            preview = snippet[:80] + "..." if len(snippet) > 80 else snippet
            return '\n'.join(filter(None, [header, meta, f"    💬 {preview}"]))

        # Expanded view
        lines = [
            header,
            meta,
        ]

        # Authors with avatars
        if authors:
            author_str = f"    👥 {', '.join(authors[:4])}"
            if len(authors) > 4:
                author_str += f" +{len(authors)-4}"
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

        return '\n'.join(lines)

    def on_click(self) -> None:
        """Toggle expanded state."""
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


class SidebarPaperList(VerticalScroll):
    """Scrollable paper list in the sidebar with clickable expansion."""

    def __init__(self, citations: List, **kwargs):
        self._citations = citations
        self._expanded_idx = None
        super().__init__(**kwargs)

    def compose(self) -> ComposeResult:
        yield Static(colored("📚 相关论文", Colors.HEADER + Colors.BOLD),
                     classes="sidebar-title")
        for i, c in enumerate(self._citations[:5]):
            yield PaperCard(c, i, classes="paper-card", id=f"paper-{i}")
        if len(self._citations) > 5:
            yield Static(
                colored(f"  [+{len(self._citations)-5} more papers]", Colors.WARNING),
                classes="more-papers"
            )
        yield Label("")  # spacer


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
            btn.on_click = lambda e, text=s: self.on_select(text)
            yield btn


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

    /* ── Streaming animation ── */
    .ai-msg.streaming {
        color: #8be9fd;
    }

    .streaming::after {
        content: " ▌";
        animation: blink 1s step-end infinite;
        color: #50fa7b;
    }

    @keyframes blink {
        0%, 100% { opacity: 1; }
        50% { opacity: 0; }
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
        concept: str = None,
        limit: int = 5,
        friction_tracker: FrictionTracker = None,
        stream: bool = True,
        session_id: str = None,
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

    # ── App lifecycle ──────────────────────────────────────────────────────

    def compose(self) -> ComposeResult:
        yield Header(show_clock=True)
        with Horizontal(id="main-row"):
            with Container(id="chat-area"):
                with VerticalScroll(id="messages"):
                    yield Static(
                        colored("📚 AI Research OS — RAG Chat\n", Colors.HEADER + Colors.BOLD) +
                        colored("   对你的论文库进行自然语言问答，带引用溯源\n", Colors.OKBLUE) +
                        colored("   Enter 发送 · Ctrl+L 清屏 · Ctrl+E 历史 · Ctrl+S 导出 · q 退出\n", Colors.WARNING),
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

    # ── Input handling ───────────────────────────────────────────────────────

    def on_input_submitted(self, event: Input.Submitted) -> None:
        if self._streaming:
            return
        question = event.value.strip()
        if not question:
            return
        self._handle_submit(question)
        self.query_one("#chat-input").value = ""

    def on_button_pressed(self, event: Button.Pressed) -> None:
        """Handle quick action button clicks."""
        btn_id = event.button.id
        if btn_id == "btn-history":
            self.action_toggle_history()
        elif btn_id == "btn-new":
            self.action_new_session()
        elif btn_id == "btn-export":
            self.action_export_session()

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
            role="user",
            content=question,
            citations=[],
            timestamp=datetime.now().isoformat()
        )
        self.messages.append(user_msg)
        self._render_messages()

        # Set streaming state
        self._streaming = True
        self._update_status("🔍 检索中...", "typing")

        # Build AI placeholder
        ai_msg = ChatMessage(
            role="assistant",
            content="",
            citations=[],
            timestamp=datetime.now().isoformat()
        )
        self.messages.append(ai_msg)
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
            self._chat_history.append({
                "question": rewritten_question,
                "answer": ai_msg.content,
                "citations": ai_msg.citations,
            })

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

        self._update_status("🤖 生成回答中...", "typing")

        # Build prompt
        prompt = self.chat._build_prompt(question, contexts)
        answer = ""

        for delta in stream_llm_chat_completions(
            [],
            model=self.chat.model,
            user_prompt=prompt,
            base_url=self.chat.base_url,
            api_key=self.chat.api_key,
            system_prompt=_RAG_SYSTEM_PROMPT,
        ):
            answer += delta
            # Batch updates for smooth rendering
            if len(answer) % self._stream_config.batch_size < 2:
                ai_msg.content = answer
                self._update_streaming_content(answer)

        ai_msg.content = answer
        ai_msg.citations = self.chat._extract_citations(contexts)

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
                is_streaming = (i == len(self.messages) - 1 and self._streaming)
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
            self.query_one("#progress-bar").update(f"[{'█' * progress}{'░' * (100 - progress)}] {progress}%")
        except NoMatches:
            pass

    def _clear_progress(self) -> None:
        """Clear the progress bar."""
        try:
            self.query_one("#progress-bar").update("")
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
            "/search": lambda: self._search_sessions(arg) if arg else self._update_status("用法: /search <关键词>"),
            "/rename": lambda: self._rename_session(arg) if arg else self._update_status("用法: /rename <新标题>"),
            "/delete": lambda: self._delete_session(arg) if arg else self._update_status("用法: /delete <编号>"),
            "/export": lambda: self._export_session() if self.session_id else self._update_status("⚠️ 当前没有会话"),
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
                sidebar.mount(PaperCard(c, 0, classes="paper-card"))
            if len(citations) > 5:
                sidebar.mount(Static(
                    colored(f"  [+{len(citations)-5} more papers]", Colors.WARNING),
                    classes="more-papers"
                ))
        except NoMatches:
            pass

    def _update_status(self, text: str, cls: str = "") -> None:
        """Update status bar with style class."""
        try:
            status = self.query_one("#status-bar")
            status.update(text)
            status.remove_class("typing", "done", "error")
            if cls:
                status.add_class(cls)
        except NoMatches:
            pass

    # ── Actions ────────────────────────────────────────────────────────────

    def action_quit(self) -> None:
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
            "  /search <k> 搜索\n"
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
            inp = self.query_one("#chat-input")
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
            idx_in_list = ai_msg_indices.index(self._selected_msg_idx) if self._selected_msg_idx in ai_msg_indices else 0
            idx_in_list = max(0, idx_in_list - 1)
            self._selected_msg_idx = ai_msg_indices[idx_in_list]

        self._render_messages_with_selection()
        self._update_nav_hint(f"已选中第 {ai_msg_indices.index(self._selected_msg_idx) + 1}/{len(ai_msg_indices)} 条回复")

    def action_select_next_message(self) -> None:
        """Select next message in history."""
        if not self.messages:
            return

        # If input has content, don't navigate
        try:
            inp = self.query_one("#chat-input")
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
            idx_in_list = ai_msg_indices.index(self._selected_msg_idx) if self._selected_msg_idx in ai_msg_indices else -1
            idx_in_list = min(len(ai_msg_indices) - 1, idx_in_list + 1)
            self._selected_msg_idx = ai_msg_indices[idx_in_list]

        self._render_messages_with_selection()
        self._update_nav_hint(f"已选中第 {ai_msg_indices.index(self._selected_msg_idx) + 1}/{len(ai_msg_indices)} 条回复")

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
            inp = self.query_one("#chat-input")
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
                is_streaming = (i == len(self.messages) - 1 and self._streaming)
                bubble = ChatBubble(msg, self._stream_config, is_streaming=is_streaming)
                if i == self._selected_msg_idx:
                    bubble.add_class("selected")
                container.mount(bubble)

            self.scroll_messages_to_bottom()
        except NoMatches:
            pass

    def _update_nav_hint(self, text: str = None) -> None:
        """Update navigation hint bar."""
        try:
            hint = self.query_one("#nav-hint")
            if text:
                hint.update(colored(f"❯ {text}  |  ↑↓ 选择  c=复制  e=编辑", Colors.OKBLUE))
            else:
                hint.update(colored("❯ 输入问题开始对话  |  ↑↓ 选择消息  c=复制  e=编辑", Colors.WARNING))
        except NoMatches:
            pass

    # ── Command Completion ────────────────────────────────────────────────────

    # Available slash commands
    SLASH_COMMANDS = [
        ("/sessions", "查看所有会话"),
        ("/load", "加载会话 /load <id>"),
        ("/search", "搜索会话 /search <关键词>"),
        ("/rename", "重命名会话 /rename <标题>"),
        ("/delete", "删除会话 /delete <id>"),
        ("/export", "导出会话"),
        ("/clear", "清空对话"),
        ("/help", "显示帮助"),
    ]

    def action_complete_command(self) -> None:
        """Complete slash commands on Tab press."""
        try:
            inp = self.query_one("#chat-input")
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
                        from llm.chat import ChatCitation
                        cites_data = json.loads(msg.get("citations", "[]")) if isinstance(msg.get("citations"), str) else msg.get("citations", [])
                        for c in cites_data:
                            citations.append(ChatCitation(
                                paper_id=c.get("paper_id", ""),
                                paper_title=c.get("title", ""),
                                authors=[],
                                published="",
                                snippet="",
                                relevance_score=c.get("score", 0.0),
                            ))
                    except Exception:
                        pass
                    self.messages.append(ChatMessage(
                        role=role,
                        content=content,
                        citations=citations,
                        timestamp=msg.get("created_at", "")
                    ))
                    if role == "assistant":
                        self._chat_history.append({"question": "", "answer": content, "citations": citations})
                self._render_messages()
                self._update_status(f"📂 已加载会话 {session_id}（{len(prev_messages)} 条消息）")
        except Exception as e:
            self._update_status(f"⚠️ 无法加载会话: {e}")

    def _save_to_session(self, question: str, answer: str, citations) -> None:
        """Save a message pair to the current session."""
        if not self.session_id:
            return
        try:
            citations_data = [
                {"paper_id": c.paper_id, "title": c.paper_title, "score": c.relevance_score}
                for c in citations
            ] if citations else []
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
            lines.extend(["", "命令:", "  /load <编号>  加载会话", "  /search <关键词>  搜索会话",
                         "  /rename <标题>  重命名", "  /delete <编号>  删除", "  /export  导出当前会话"])

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
                ""
            ]

            for msg in self.messages:
                role = "User" if msg.role == "user" else "Assistant"
                ts = Timestamp.format(msg.timestamp)
                lines.append(f"## {role} ({ts})")
                lines.append(msg.content)
                if msg.citations:
                    lines.append("\n**Citations:**")
                    for c in msg.citations:
                        title = getattr(c, 'paper_title', 'Unknown')
                        pid = getattr(c, 'paper_id', '')
                        lines.append(f"- [{pid}] {title}")
                lines.append("")

            # Get download directory or current directory
            export_path = Path.cwd() / filename

            with open(export_path, "w", encoding="utf-8") as f:
                f.write('\n'.join(lines))

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
                self._suggestions = [opt['question'] for opt in options[:4]]

                # Display suggestions in notification
                lines = ["💡 追问建议:", ""]
                for i, opt in enumerate(options[:4], 1):
                    q = opt['question'][:50]
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
    p.add_argument(
        "--session", "-s", metavar="ID",
        help="Continue from a saved chat session",
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
