#!/usr/bin/env python3
"""Generate CLI API reference HTML from source code."""

import ast
import json
import re
from pathlib import Path

ROOT = Path(__file__).parent.parent
CMD_DIR = ROOT / "cli" / "cmd"
OUTPUT = ROOT / "docs" / "cli_reference.html"


def extract_commands():
    """Parse all cli/cmd/*.py files and extract command metadata."""
    commands = []

    for py_file in sorted(CMD_DIR.glob("*.py")):
        if py_file.name.startswith("_"):
            continue

        module_name = py_file.stem
        if module_name in ("__init__",):
            continue

        source = py_file.read_text(encoding="utf-8")
        tree = ast.parse(source, filename=str(py_file))

        # Find the _build_*_parser function
        builder_name = f"_build_{module_name}_parser"
        func = None
        for node in ast.walk(tree):
            if isinstance(node, ast.FunctionDef) and node.name == builder_name:
                func = node
                break

        if not func:
            # Try without module prefix (some files have different naming)
            for node in ast.walk(tree):
                if isinstance(node, ast.FunctionDef) and node.name.startswith("_build_") and node.name.endswith("_parser"):
                    builder_name = node.name
                    func = node
                    break

        if not func:
            continue

        # Extract help/description from add_parser call
        help_text = ""
        desc_text = ""
        cmd_name = module_name.replace("_", "-")

        for node in ast.walk(func):
            if isinstance(node, ast.Call):
                call_name = ""
                if isinstance(node.func, ast.Name):
                    call_name = node.func.id
                elif isinstance(node.func, ast.Attribute):
                    call_name = node.func.attr

                if call_name == "add_parser":
                    for kw in node.keywords:
                        if kw.arg == "help" and isinstance(kw.value, ast.Constant):
                            help_text = kw.value.value or ""
                        if kw.arg == "description" and isinstance(kw.value, ast.Constant):
                            desc_text = kw.value.value or ""

                    # Get positional args (command name)
                    if node.args:
                        first_arg = node.args[0]
                        if isinstance(first_arg, ast.Constant):
                            cmd_name = first_arg.value

        # Extract subcommand patterns (e.g., gap list, gap extract)
        subcommands = []
        for node in ast.walk(func):
            if isinstance(node, ast.Call):
                call_name = ""
                if isinstance(node.func, ast.Attribute):
                    call_name = node.func.attr
                if call_name == "add_parser":
                    sub_name = ""
                    sub_help = ""
                    for kw in node.keywords:
                        if kw.arg == "dest":
                            sub_name = getattr(kw.value, "value", "")
                        if kw.arg == "help" and isinstance(kw.value, ast.Constant):
                            sub_help = kw.value.value or ""
                    if sub_name:
                        subcommands.append({"name": sub_name, "help": sub_help})

        commands.append({
            "name": cmd_name,
            "module": module_name,
            "help": help_text or desc_text,
            "subcommands": subcommands,
            "file": f"cli/cmd/{module_name}.py",
        })

    return commands


def escape(s):
    return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;").replace('"', "&quot;")


def generate_html(commands):
    groups = {
        "Core": ["import", "search", "list", "stats", "status", "export", "dedup", "dedup-semantic", "similar"],
        "Papers": ["cite-fetch", "cite-graph", "cite-import", "cite-stats", "cite-backfill", "citation-chain", "citations"],
        "Research": ["research", "gap", "trend", "influence", "path", "hypothesize", "story", "argue", "narrative"],
        "Insight": ["insight", "evolution", "journal", "digest", "lean", "benchmark", "question", "review", "analyze"],
        "Agents": ["agent", "chat", "chat-tui", "route", "friction", "subscribe", "repl"],
        "Knowledge": ["kg", "rag", "pipeline", "experiment", "postprocess", "validate", "ingest"],
        "Tools": ["cache", "queue", "read-queue", "slides", "session", "dashboard", "roadmap", "merge", "litreview", "compare", "replicate", "paper2code", "evoskill", "visual", "ask"],
    }

    html = []
    html.append("<!DOCTYPE html>")
    html.append("<html lang='en'>")
    html.append("<head>")
    html.append("<meta charset='UTF-8'>")
    html.append("<meta name='viewport' content='width=device-width, initial-scale=1.0'>")
    html.append("<title>CLI API Reference — AI Research OS</title>")
    html.append("<style>")
    html.append("""
    :root {
      --bg: #faf8f5;
      --bg-card: #ffffff;
      --text: #2d2a24;
      --text-muted: #7a7570;
      --accent: #c44;
      --border: #e8e4de;
      --code-bg: #f3f0eb;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body { font-family: 'Segoe UI', system-ui, sans-serif; background: var(--bg); color: var(--text); line-height: 1.6; }
    header { background: var(--text); color: var(--bg); padding: 2rem 2rem 1.5rem; }
    header h1 { font-size: 1.8rem; font-weight: 700; margin-bottom: 0.3rem; }
    header p { color: #b0aaa0; font-size: 0.9rem; }
    .container { max-width: 900px; margin: 0 auto; padding: 2rem 1rem; }
    .search-bar { width: 100%; padding: 0.7rem 1rem; font-size: 1rem; border: 2px solid var(--border); border-radius: 8px; margin-bottom: 1.5rem; background: var(--bg-card); }
    .group { margin-bottom: 2.5rem; }
    .group-title { font-size: 0.75rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.1em; color: var(--accent); margin-bottom: 0.8rem; padding-bottom: 0.3rem; border-bottom: 2px solid var(--accent); }
    .cmd-card { background: var(--bg-card); border: 1px solid var(--border); border-radius: 8px; padding: 1rem 1.2rem; margin-bottom: 0.6rem; }
    .cmd-name { font-family: 'Cascadia Code', 'Fira Code', monospace; font-size: 1rem; font-weight: 700; color: var(--text); }
    .cmd-name span { color: var(--accent); }
    .cmd-help { font-size: 0.875rem; color: var(--text-muted); margin-top: 0.2rem; }
    .cmd-subs { margin-top: 0.5rem; }
    .cmd-sub { display: inline-block; background: var(--code-bg); border: 1px solid var(--border); border-radius: 4px; padding: 0.1rem 0.5rem; font-size: 0.78rem; font-family: 'Cascadia Code', monospace; margin-right: 0.4rem; margin-bottom: 0.3rem; }
    .cmd-sub-name { color: var(--text); }
    .cmd-sub-help { color: var(--text-muted); }
    .cmd-file { font-size: 0.72rem; color: var(--text-muted); margin-top: 0.3rem; font-family: monospace; }
    footer { text-align: center; padding: 2rem; color: var(--text-muted); font-size: 0.8rem; }
    """ + f"\n    // {len(commands)} commands loaded\n")
    html.append("</style>")
    html.append("</head>")
    html.append("<body>")
    html.append("<header>")
    html.append("<h1>CLI API Reference</h1>")
    html.append("<p>AI Research OS · Auto-generated from source · Run <code>airos-cli &lt;command&gt; --help</code> for full docs</p>")
    html.append("</header>")
    html.append("<div class='container'>")
    html.append("<input class='search-bar' id='search' placeholder='Search commands...' oninput='filter()'/>")

    # Sort commands by group
    name_to_cmd = {c["name"]: c for c in commands}

    for group_name, names in groups.items():
        group_cmds = [name_to_cmd[n] for n in names if n in name_to_cmd]
        if not group_cmds:
            continue
        html.append(f"<div class='group' data-group='{group_name}'>")
        html.append(f"<div class='group-title'>{group_name}</div>")
        for cmd in group_cmds:
            subs = cmd.get("subcommands", [])
            sub_html = ""
            if subs:
                sub_html = "<div class='cmd-subs'>" + "".join(
                    f"<span class='cmd-sub'><span class='cmd-sub-name'>{escape(s['name'])}</span> <span class='cmd-sub-help'>{escape(s['help'][:40])}</span></span>"
                    for s in subs
                ) + "</div>"
            html.append(f"<div class='cmd-card' data-name='{escape(cmd['name'])}' data-help='{escape(cmd['help'])}'>")
            html.append(f"<div class='cmd-name'>airos-cli <span>{escape(cmd['name'])}</span></div>")
            html.append(f"<div class='cmd-help'>{escape(cmd['help'])}</div>")
            html.append(sub_html)
            html.append(f"<div class='cmd-file'>{escape(cmd['file'])}</div>")
            html.append("</div>")
        html.append("</div>")

    # Uncategorized commands
    used = set()
    for names in groups.values():
        used.update(names)
    uncategorized = [c for c in commands if c["name"] not in used]
    if uncategorized:
        html.append("<div class='group' data-group='Other'>")
        html.append("<div class='group-title'>Other</div>")
        for cmd in uncategorized:
            subs = cmd.get("subcommands", [])
            sub_html = ""
            if subs:
                sub_html = "<div class='cmd-subs'>" + "".join(
                    f"<span class='cmd-sub'><span class='cmd-sub-name'>{escape(s['name'])}</span> <span class='cmd-sub-help'>{escape(s['help'][:40])}</span></span>"
                    for s in subs
                ) + "</div>"
            html.append(f"<div class='cmd-card' data-name='{escape(cmd['name'])}' data-help='{escape(cmd['help'])}'>")
            html.append(f"<div class='cmd-name'>airos-cli <span>{escape(cmd['name'])}</span></div>")
            html.append(f"<div class='cmd-help'>{escape(cmd['help'])}</div>")
            html.append(sub_html)
            html.append(f"<div class='cmd-file'>{escape(cmd['file'])}</div>")
            html.append("</div>")
        html.append("</div>")

    html.append("</div>")
    html.append("<footer>")
    html.append(f"Auto-generated · {len(commands)} commands · AI Research OS<br>")
    html.append("<a href='https://github.com/shushuzn/Rairos'>GitHub</a> · <a href='./architecture.html'>Architecture</a>")
    html.append("</footer>")
    html.append("<script>")
    html.append("""
    function filter() {
      var q = document.getElementById('search').value.toLowerCase();
      document.querySelectorAll('.cmd-card').forEach(function(card) {
        var name = card.getAttribute('data-name') || '';
        var help = card.getAttribute('data-help') || '';
        card.style.display = (name.includes(q) || help.includes(q)) ? '' : 'none';
      });
      document.querySelectorAll('.group').forEach(function(g) {
        var visible = [].slice.call(g.querySelectorAll('.cmd-card')).some(function(c) { return c.style.display !== 'none'; });
        g.style.display = visible ? '' : 'none';
      });
    }
    """)
    html.append("</script>")
    html.append("</body>")
    html.append("</html>")
    return "\n".join(html)


def main():
    commands = extract_commands()
    print(f"Found {len(commands)} commands")
    html = generate_html(commands)
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(html, encoding="utf-8")
    print(f"Written: {OUTPUT}")


if __name__ == "__main__":
    main()
