"""
Warp-style terminal block renderer.

Provides box-drawing character blocks, syntax highlighting, and structured
output primitives for CLI commands. Inspired by Warp terminal's block model.

Color palette aligned with Warp terminal's dark theme:
  Accent:  #FF8272 (coral red)
  Success: #B4FA72 (lime green)
  Info:    #A5D5FE (sky blue)
  Purple:  #D0D1FE (lavender)
"""

from __future__ import annotations

import re
from typing import List, Tuple, Optional, cast

try:
    from rich.console import Console
    from rich.table import Table as RichTable
    from rich.panel import Panel as RichPanel

    _RICH = True
except ImportError:
    _RICH = False

# ─── Colors ────────────────────────────────────────────────────────────────────


class _Colors:
    """ANSI color codes — supports both attribute (e.g. .SYN_COMMENT) and
    dict-style (e.g. ['syn_comment']) access for backward compatibility."""

    def __init__(self) -> None:
        # Basic styles
        self.BOLD = "\033[1m"
        self.ITALIC = "\033[3m"
        self.CODE = "\033[92m"
        self.LINK = "\033[94m"
        self.HEADER = "\033[95m"
        self.LIST = "\033[96m"
        self.QUOTE = "\033[90m"
        self.RESET = "\033[0m"

        # Syntax highlighting (256-color palette)
        self.SYN_KEYWORD = "\033[38;5;141m"  # Purple
        self.SYN_STRING = "\033[38;5;114m"  # Light blue
        self.SYN_NUMBER = "\033[38;5;215m"  # Orange
        self.SYN_COMMENT = "\033[38;5;242m"  # Gray
        self.SYN_FUNCTION = "\033[38;5;79m"  # Cyan
        self.SYN_CLASS = "\033[38;5;147m"  # Light purple
        self.SYN_DECORATOR = "\033[38;5;197m"  # Pink
        self.SYN_OPERATOR = "\033[38;5;180m"  # Yellow-brown

        # Warp dark-theme colors (#RRGGBB hex, for Rich markup)
        self.ACCENT = "#FF8272"  # Coral red (Warp primary accent)
        self.SUCCESS = "#B4FA72"  # Lime green
        self.INFO = "#A5D5FE"  # Sky blue
        self.PURPLE = "#D0D1FE"  # Lavender
        self.WARNING = "#FEFDC2"  # Pale yellow
        self.ERROR = "#FF5555"  # Red
        self.TEXT = "#F1F1F1"  # White text
        self.MUTED = "#8E8E8E"  # Dimmed text
        self.BG_HEADER = "#232323"  # Dark panel header
        self.BG_ROW_ALT = "#232323"  # Alternating row bg

        # Fallback ANSI for non-Rich mode
        self.TITLE_FG = "\033[38;5;141m"
        self.HEADER_BG = "\033[48;5;235m"
        self.ROW_ALT = "\033[48;5;236m"
        self.RESET_BG = "\033[49m"

        # Dict-key aliases (backward compat with SimpleMarkdown.COLORS dict)
        self._aliases = {
            "bold": "BOLD",
            "italic": "ITALIC",
            "code": "CODE",
            "code_bg": "RESET_BG",
            "link": "LINK",
            "header": "HEADER",
            "list": "LIST",
            "quote": "QUOTE",
            "reset": "RESET",
            "syn_keyword": "SYN_KEYWORD",
            "syn_string": "SYN_STRING",
            "syn_number": "SYN_NUMBER",
            "syn_comment": "SYN_COMMENT",
            "syn_function": "SYN_FUNCTION",
            "syn_class": "SYN_CLASS",
            "syn_decorator": "SYN_DECORATOR",
            "syn_operator": "SYN_OPERATOR",
            # Warp palette
            "accent": "ACCENT",
            "success": "SUCCESS",
            "info": "INFO",
            "purple": "PURPLE",
            "warning": "WARNING",
            "error": "ERROR",
            "text": "TEXT",
            "muted": "MUTED",
        }

    def __getitem__(self, key: str) -> str:
        """Dict-style access for backward compatibility."""
        attr = self._aliases.get(key, key)
        return cast(str, getattr(self, attr))


# Module-level singleton instance
_C = _Colors()


# ─── Language definitions ───────────────────────────────────────────────────────


LANG_ALIASES = {
    "py": "python",
    "js": "javascript",
    "ts": "typescript",
    "sh": "bash",
    "shell": "bash",
    "zsh": "bash",
    "rb": "ruby",
    "rs": "rust",
    "go": "go",
    "yml": "yaml",
    "tf": "hcl",
    "dockerfile": "dockerfile",
    "jsonc": "json",
    "jsx": "javascript",
    "tsx": "typescript",
}

PY_KEYWORDS = frozenset(
    {
        "and",
        "as",
        "assert",
        "async",
        "await",
        "break",
        "class",
        "continue",
        "def",
        "del",
        "elif",
        "else",
        "except",
        "finally",
        "for",
        "from",
        "global",
        "if",
        "import",
        "in",
        "is",
        "lambda",
        "nonlocal",
        "not",
        "or",
        "pass",
        "raise",
        "return",
        "try",
        "while",
        "with",
        "yield",
        "True",
        "False",
        "None",
        "self",
        "cls",
    }
)

JS_KEYWORDS = frozenset(
    {
        "async",
        "await",
        "break",
        "case",
        "catch",
        "class",
        "const",
        "continue",
        "debugger",
        "default",
        "delete",
        "do",
        "else",
        "export",
        "extends",
        "finally",
        "for",
        "function",
        "if",
        "import",
        "in",
        "instanceof",
        "let",
        "new",
        "of",
        "return",
        "static",
        "super",
        "switch",
        "this",
        "throw",
        "try",
        "typeof",
        "var",
        "void",
        "while",
        "with",
        "yield",
        "true",
        "false",
        "null",
        "undefined",
    }
)


# ─── Syntax highlighting ───────────────────────────────────────────────────────


def _tokenize_python(code: str) -> str:
    """Highlight Python code with syntax colors."""
    lines = code.split("\n")
    result_lines = []

    for line in lines:
        if not line.strip() or line.strip().startswith("#"):
            result_lines.append(line)
            continue

        highlighted = []
        pos = 0
        ln = len(line)

        while pos < ln:
            if line[pos] == "@":
                end = pos + 1
                while end < ln and (line[end].isalnum() or line[end] in "_"):
                    end += 1
                highlighted.append(f"{_C.SYN_DECORATOR}{line[pos:end]}{_C.RESET}")
                pos = end
                continue

            if line[pos] in "\"'":
                quote = line[pos]
                if line[pos : pos + 3] == quote * 3:
                    quote = quote * 3
                end = pos + len(quote)
                while end < ln:
                    if line[end] == "\\" and end + 1 < ln:
                        end += 2
                        continue
                    if line[end:].startswith(quote):
                        end += len(quote)
                        break
                    end += 1
                highlighted.append(f"{_C.SYN_STRING}{line[pos:end]}{_C.RESET}")
                pos = end
                continue

            if line[pos] == "#":
                highlighted.append(f"{_C.SYN_COMMENT}{line[pos:]}{_C.RESET}")
                break

            if line[pos].isalnum() or line[pos] == "_":
                end = pos
                while end < ln and (line[end].isalnum() or line[end] in "_"):
                    end += 1
                word = line[pos:end]

                if word in PY_KEYWORDS:
                    highlighted.append(f"{_C.SYN_KEYWORD}{word}{_C.RESET}")
                elif end < ln and line[end] == "(":
                    highlighted.append(f"{_C.SYN_FUNCTION}{word}{_C.RESET}")
                elif word[0].isupper() and "_" not in word and len(word) > 1:
                    highlighted.append(f"{_C.SYN_CLASS}{word}{_C.RESET}")
                elif word.replace(".", "").isdigit():
                    highlighted.append(f"{_C.SYN_NUMBER}{word}{_C.RESET}")
                else:
                    highlighted.append(word)
                pos = end
                continue

            if line[pos] in "=!<>+-*/%&|^~:":
                highlighted.append(f"{_C.SYN_OPERATOR}{line[pos]}{_C.RESET}")
                pos += 1
                continue

            highlighted.append(line[pos])
            pos += 1

        result_lines.append("".join(highlighted))

    return "\n".join(result_lines)


def _tokenize_javascript(code: str) -> str:
    """Highlight JavaScript/TypeScript code with syntax colors."""
    lines = code.split("\n")
    result_lines = []

    for line in lines:
        if not line.strip() or line.strip().startswith("//"):
            result_lines.append(line)
            continue

        highlighted = []
        pos = 0
        ln = len(line)

        while pos < ln:
            if line[pos] in "\"'":
                quote = line[pos]
                end = pos + 1
                while end < ln:
                    if line[end] == "\\":
                        end += 2
                        continue
                    if line[end] == quote:
                        end += 1
                        break
                    end += 1
                highlighted.append(f"{_C.SYN_STRING}{line[pos:end]}{_C.RESET}")
                pos = end
                continue

            if line[pos] == "`":
                end = pos + 1
                while end < ln:
                    if line[end] == "\\":
                        end += 2
                        continue
                    if line[end] == "`":
                        end += 1
                        break
                    end += 1
                highlighted.append(f"{_C.SYN_STRING}{line[pos:end]}{_C.RESET}")
                pos = end
                continue

            if line[pos : pos + 2] == "//":
                highlighted.append(f"{_C.SYN_COMMENT}{line[pos:]}{_C.RESET}")
                break

            if line[pos].isalnum() or line[pos] in "_$":
                end = pos
                while end < ln and (line[end].isalnum() or line[end] in "_$"):
                    end += 1
                word = line[pos:end]

                if word in JS_KEYWORDS:
                    highlighted.append(f"{_C.SYN_KEYWORD}{word}{_C.RESET}")
                elif end < ln and line[end] == "(":
                    highlighted.append(f"{_C.SYN_FUNCTION}{word}{_C.RESET}")
                elif word[0].isupper() and len(word) > 1:
                    highlighted.append(f"{_C.SYN_CLASS}{word}{_C.RESET}")
                elif word.replace(".", "").isdigit():
                    highlighted.append(f"{_C.SYN_NUMBER}{word}{_C.RESET}")
                else:
                    highlighted.append(word)
                pos = end
                continue

            if line[pos] in "=!<>+-*/%&|^~?:":
                highlighted.append(f"{_C.SYN_OPERATOR}{line[pos]}{_C.RESET}")
                pos += 1
                continue

            highlighted.append(line[pos])
            pos += 1

        result_lines.append("".join(highlighted))

    return "\n".join(result_lines)


def _tokenize_json(code: str) -> str:
    """Highlight JSON with syntax colors."""
    result = []
    i = 0
    ln = len(code)

    while i < ln:
        if code[i] == '"':
            end = i + 1
            while end < ln:
                if code[end] == "\\":
                    end += 2
                    continue
                if code[end] == '"':
                    end += 1
                    break
                end += 1
            rest = code[end:].lstrip()
            is_key = rest.startswith(":")
            color = _C.SYN_KEYWORD if is_key else _C.SYN_STRING
            result.append(f"{color}{code[i:end]}{_C.RESET}")
            i = end
            continue

        if code[i].isdigit() or (code[i] == "-" and i + 1 < ln and code[i + 1].isdigit()):
            end = i
            if code[end] == "-":
                end += 1
            while end < ln and (code[end].isdigit() or code[end] in ".eE+-"):
                end += 1
            result.append(f"{_C.SYN_NUMBER}{code[i:end]}{_C.RESET}")
            i = end
            continue

        for kw in ("true", "false", "null"):
            if code[i : i + len(kw)] == kw and (
                i + len(kw) >= ln or not code[i + len(kw)].isalnum()
            ):
                result.append(f"{_C.SYN_KEYWORD}{kw}{_C.RESET}")
                i += len(kw)
                break
        else:
            result.append(code[i])
            i += 1

    return "".join(result)


def _tokenize_generic(code: str) -> str:
    """Generic highlighting for unknown languages."""
    lines = code.split("\n")
    result_lines = []

    for line in lines:
        if " #" in line or line.lstrip().startswith("#"):
            idx = line.find("#")
            if idx >= 0:
                result_lines.append(f"{line[:idx]}{_C.SYN_COMMENT}{line[idx:]}{_C.RESET}")
                continue
        result_lines.append(line)

    return "\n".join(result_lines)


def _tokenize(lang: str, code: str) -> str:
    """Tokenize code by language."""
    lang = LANG_ALIASES.get(lang.lower(), lang.lower())

    if lang == "python":
        return _tokenize_python(code)
    elif lang in ("javascript", "typescript", "jsx", "tsx"):
        return _tokenize_javascript(code)
    elif lang in ("json", "jsonc"):
        return _tokenize_json(code)
    else:
        return _tokenize_generic(code)


# ─── WarpBlocks ───────────────────────────────────────────────────────────────


class WarpBlocks:
    """Warp-style terminal block renderer.

    Provides structured terminal output using box-drawing characters (┌─┐│└┘).

    Usage:
        print(WarpBlocks.panel("Stats", "42 papers loaded"))
        print(WarpBlocks.code_block("python", "print('hello')"))
        print(WarpBlocks.table(["Name", "Count"], [["Papers", "42"]]))
    """

    # Re-export colors for consumers
    C = _C
    # Dict-style access (matches SimpleMarkdown.COLORS dict pattern)
    COLORS = _C  # instance with __getitem__

    # Re-export language/tokenizer utilities (for SimpleMarkdown compatibility)
    LANG_ALIASES = LANG_ALIASES
    PY_KEYWORDS = PY_KEYWORDS
    JS_KEYWORDS = JS_KEYWORDS
    _tokenize_python = staticmethod(_tokenize_python)
    _tokenize_javascript = staticmethod(_tokenize_javascript)
    _tokenize_json = staticmethod(_tokenize_json)
    _tokenize_generic = staticmethod(_tokenize_generic)
    _tokenize = staticmethod(_tokenize)

    @classmethod
    def code_block(
        cls,
        lang: str,
        code: str,
        title: Optional[str] = None,
        width: int = 80,
        show_line_numbers: bool = True,
    ) -> str:
        """Render a code block with syntax highlighting and box-drawing border.

        Args:
            lang: Language identifier (python, javascript, json, etc.)
            code: Source code string
            title: Optional title shown in the top border (default: lang.upper())
            width: Total block width in characters
            show_line_numbers: Whether to show line numbers on the left
        """
        if not code:
            return ""

        lines = code.split("\n")
        highlighted = _tokenize(lang, code).split("\n")
        display_title = title or (lang.upper() if lang else "CODE")

        # Top border: "┌─ TITLE ────────────────┐"
        top = f"┌─ {display_title} ─" + "─" * max(0, width - len(display_title) - 8) + "┐"

        result_lines = [top]
        if show_line_numbers:
            num_width = len(str(len(lines)))
            for i, (orig, hl) in enumerate(zip(lines, highlighted)):
                line_num = str(i + 1).rjust(num_width)
                max_content = width - num_width - 4  # " │ "
                if len(orig) > max_content:
                    hl = hl[:max_content] + _C.SYN_COMMENT + " …" + _C.RESET
                content = f"{_C.SYN_COMMENT}{line_num}{_C.RESET} │ {hl}"
                result_lines.append(f"│ {content}")
        else:
            for hl in highlighted:
                max_content = width - 4
                if len(hl) > max_content:
                    hl = hl[:max_content] + _C.SYN_COMMENT + " …" + _C.RESET
                result_lines.append(f"│ {hl}")

        result_lines.append("└" + "─" * (width - 2) + "┘")
        return "\n".join(result_lines)

    @classmethod
    @classmethod
    def panel(cls, title: str, body: str, width: int = 80) -> str:
        """Render a panel with Warp-style colors using Rich (falls back to ASCII).

        Args:
            title: Title shown in the top border
            body: Body content (supports \\n for multiple lines)
            width: Total block width
        """
        if _RICH:
            from io import StringIO

            buf = StringIO()
            console = Console(file=buf, width=width, force_terminal=True, no_color=False)
            panel = RichPanel(
                body,
                title=f"[bold #FF8272]  {title}  [/]",
                border_style="#FF8272",
                style="#F1F1F1",
                padding=(1, 2),
            )
            console.print(panel)
            return buf.getvalue().rstrip("\n")

        # Fallback: original ASCII box-drawing
        top = f"┌─ {title} " + "─" * max(0, width - len(title) - 7) + "┐"
        lines = body.split("\n") if body else [""]
        result_lines = [top]
        for line in lines:
            if len(line) > width - 4:
                line = line[: width - 7] + "…"
            result_lines.append(f"│ {line}" + " " * max(0, width - len(line) - 4) + "│")
        result_lines.append("└" + "─" * (width - 2) + "┘")
        return "\n".join(result_lines)

    @classmethod
    def section(cls, title: str, *body_lines: str, width: int = 80) -> str:
        """Render a titled section block with Warp-style colors.

        Args:
            title: Section title (shown in a header row)
            *body_lines: Body content lines
            width: Total block width
        """
        body = "\n".join(body_lines)
        if _RICH:
            from io import StringIO

            buf = StringIO()
            console = Console(file=buf, width=width, force_terminal=True, no_color=False)
            panel = RichPanel(
                body,
                title=f"[bold #FF8272]  {title}  [/]",
                border_style="#FF8272",
                style="#F1F1F1",
                padding=(1, 2),
            )
            console.print(panel)
            return buf.getvalue().rstrip("\n")

        # Fallback
        top = f"┌─ {title} " + "─" * max(0, width - len(title) - 7) + "┐"
        result_lines = [top]
        for line in body_lines:
            wrapped = cls.wrap(line, width - 4)
            for wl in wrapped.split("\n"):
                result_lines.append(f"│ {wl}" + " " * max(0, width - len(wl) - 4) + "│")
        result_lines.append("└" + "─" * (width - 2) + "┘")
        return "\n".join(result_lines)

    @classmethod
    def table(
        cls,
        headers: List[str],
        rows: List[List[str]],
        width: int = 80,
        title: Optional[str] = None,
    ) -> str:
        """Render a table with Warp-style colors using Rich (falls back to ASCII).

        Args:
            headers: Column header names
            rows: List of row data (one list per row)
            width: Total table width
            title: Optional table title
        """
        if not headers:
            return ""

        if _RICH:
            from io import StringIO

            buf = StringIO()
            console = Console(file=buf, width=width, force_terminal=True, no_color=False)

            t = RichTable(
                title=title,
                style="bold #FF8272",
                header_style="bold #F1F1F1 on #232323",
                border_style="#3A3A3C",
                row_styles=["", "#232323"],  # alternating row backgrounds
                show_lines=True,
            )
            for h in headers:
                t.add_column(f"  {h}  ", style="#A5D5FE", no_wrap=False)

            for row in rows:
                cells = [str(c) for c in row]
                # Color-code status columns heuristically
                styled = []
                for _i, c in enumerate(cells):
                    lc = c.lower()
                    if any(kw in lc for kw in ("ready", "done", "✓", "pass", "ok", "success")):
                        styled.append(f"[#B4FA72]{c}[/]")
                    elif any(kw in lc for kw in ("pending", "warn", "⚠", "loading", "progress")):
                        styled.append(f"[#FEFDC2]{c}[/]")
                    elif any(kw in lc for kw in ("error", "fail", "✗", "✘", "dead")):
                        styled.append(f"[#FF5555]{c}[/]")
                    else:
                        styled.append(c)
                t.add_row(*styled)

            console.print(t)
            return buf.getvalue().rstrip("\n")

        # Fallback: original ASCII box-drawing
        n_cols = len(headers)
        col_widths = [
            max(len(h), max((len(str(r[i])) for r in rows), default=0))
            for i, h in enumerate(headers)
        ]
        total_weighted = sum(col_widths) + (n_cols - 1) * 3
        scale = (
            min(1.0, (width - n_cols - 1) / max(1, total_weighted)) if total_weighted > 0 else 1.0
        )
        col_widths = [max(3, int(w * scale)) for w in col_widths]

        header_sep = "├".join("─" * w for w in col_widths)

        def fmt_row(cells: List[str]) -> str:
            padded = [str(c).ljust(w) for c, w in zip(cells, col_widths)]
            return "│ " + " │ ".join(padded) + " │"

        lines = []
        lines.append("┌" + header_sep + "┐")
        lines.append(fmt_row(headers))
        lines.append("├" + header_sep + "┤")
        for i, row in enumerate(rows):
            alt = i % 2 == 1
            cells = [str(c) for c in row] + [""] * (n_cols - len(row))
            row_str = fmt_row(cells)
            if alt:
                row_str = _C.ROW_ALT + row_str + _C.RESET_BG
            lines.append(row_str)
        lines.append("└" + header_sep + "┘")
        return "\n".join(lines)

    @classmethod
    def tree(cls, root: str, children: List[Tuple[str, str]], width: int = 80) -> str:
        """Render a tree view with a root and child nodes.

        Args:
            root: Root node label
            children: List of (prefix, label) tuples for child nodes
            width: Max width
        """
        lines = [f"📂 {_C.BOLD}{root}{_C.RESET}"]
        for i, (_prefix, label) in enumerate(children):
            is_last = i == len(children) - 1
            branch = "└── " if is_last else "├── "
            wrapped = cls.wrap(label, width - 6)
            for j, wl in enumerate(wrapped.split("\n")):
                indent = "    " if is_last else "│   "
                lines.append(f"{indent}{branch if j == 0 else ''}{wl}")
        return "\n".join(lines)

    @classmethod
    def progress(cls, label: str, current: int, total: int, width: int = 40) -> str:
        """Render a progress bar.

        Args:
            label: Label shown before the bar
            current: Current progress value
            total: Total value
            width: Width of the bar (not counting label)
        """
        if total <= 0:
            pct = 0
            filled = 0
        else:
            pct = min(100, max(0, int(current / total * 100)))
            filled = int(pct / 100 * width)

        bar = "█" * filled + "░" * (width - filled)
        return f"{label} [{bar}] {pct}%"

    @classmethod
    def parse_markdown(cls, text: str) -> str:
        """Parse markdown to styled text with Warp-style headers.

        Mirrors SimpleMarkdown.parse() for compatibility.
        """
        if not text:
            return text

        result = text

        # Process code blocks first
        def replace_code_block(m):
            lang = m.group(1).strip() or "text"
            code = m.group(2)
            return "\n" + cls.code_block(lang, code) + "\n"

        result = re.sub(r"```(\w*)\n?(.*?)```", replace_code_block, result, flags=re.DOTALL)

        # Inline code
        result = re.sub(r"`([^`]+)`", r" [\1] ", result)

        # Headers
        result = re.sub(r"^### (.+)$", r"\n━━━ \1 ━━━\n", result, flags=re.MULTILINE)
        result = re.sub(r"^## (.+)$", r"\n━━ \1 �━━\n", result, flags=re.MULTILINE)
        result = re.sub(r"^# (.+)$", r"\n━ \1 ━\n", result, flags=re.MULTILINE)

        # Bold / Italic
        result = re.sub(r"\*\*(.+?)\*\*", r"[\1]", result)
        result = re.sub(r"__(.+?)__", r"[\1]", result)
        result = re.sub(r"\*(.+?)\*", r"/\1/", result)
        result = re.sub(r"_(.+?)_", r"/\1/", result)

        # Lists
        result = re.sub(r"^[\-\*] (.+)$", r"  • \1", result, flags=re.MULTILINE)
        result = re.sub(r"^\d+\. (.+)$", r"  \g<0>", result, flags=re.MULTILINE)

        # Blockquotes
        result = re.sub(r"^> (.+)$", r"  │ \1", result, flags=re.MULTILINE)

        # Horizontal rules
        result = re.sub(r"^---+$", "─" * 50, result, flags=re.MULTILINE)

        return result

    @classmethod
    def wrap(cls, text: str, width: int = 80) -> str:
        """Wrap text to specified width at word boundaries."""
        if not text:
            return text
        lines = text.split("\n")
        wrapped = []
        for line in lines:
            if len(line) <= width:
                wrapped.append(line)
            else:
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
