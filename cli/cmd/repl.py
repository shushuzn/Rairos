"""CLI command: repl — interactive REPL for AI Research OS."""
from __future__ import annotations

import argparse
import sys
from typing import Optional

# readline is Unix-only; on Windows fall back gracefully
try:
    import readline
except ImportError:
    readline = None  # type: ignore

from cli._shared import (
    Colors, colored, print_success, print_error, print_warning, print_info, print_header,
    get_db,
)
from kg import KGManager

DEFAULT_LIMIT = 20


class AIROSRepl:
    """Stateful interactive REPL for AI Research OS.

    Holds context (current paper/tag) and exposes chained exploration
    of the paper database and knowledge graph.
    """

    def __init__(self, limit: int = DEFAULT_LIMIT):
        self.limit = limit
        self.db = get_db()
        self.kg = KGManager()
        self.current_paper: Optional[str] = None
        self.current_tag: Optional[str] = None
        self._setup_readline()

    # ── Public API ───────────────────────────────────────────────────────────────

    def run(self) -> int:
        """Run the REPL loop. Returns exit code."""
        print_header("AI Research OS — Interactive REPL")
        print_info("Type `help` for commands. Ctrl-C or `exit` to quit.\n")

        while True:
            try:
                line = self._readline()
                if not line.strip():
                    continue
                stop = self._execute(line)
                if stop:  # exit signal
                    break
            except KeyboardInterrupt:
                print(" (use `exit` to quit)")
            except EOFError:
                break

        print_info("Goodbye.")
        return 0

    # ── Command Dispatch ───────────────────────────────────────────────────────

    def _execute(self, line: str) -> bool:
        """Parse and run one line. Returns True to exit."""
        tokens = line.strip().split(maxsplit=1)
        verb = tokens[0].lower()
        args = tokens[1] if len(tokens) > 1 else ""

        handler = _COMMAND_HANDLERS.get(verb)
        if handler is None:
            print_error(f"Unknown command: {verb}. Try `help`.")
            return False

        try:
            return handler(self, args)
        except Exception as exc:
            print_error(f"Error: {exc}")
            return False

    # ── Readline Setup ────────────────────────────────────────────────────────

    def _setup_readline(self):
        """Configure readline tab completion and history (Unix-only)."""
        if readline is None:
            return
        try:
            readline.parse_and_bind("tab: complete")
            readline.set_completer(self._completer)
            try:
                readline.set_completer_delims(" \t\n")
            except Exception:
                pass
        except Exception:
            pass

    def _completer(self, text: str, state: int) -> Optional[str]:
        """readline completer — command names on first token."""
        if readline is None:
            return None
        if not hasattr(self, "_completing_pos"):
            self._completing_pos = 0
        try:
            buf = readline.get_line_buffer()
            tokens = buf.split()
            self._completing_pos = len(tokens)
        except Exception:
            self._completing_pos = 0

        if self._completing_pos <= 1:
            options = [c for c in _ALL_COMMANDS if c.startswith(text)]
            try:
                return options[state]
            except IndexError:
                return None
        return None

    # ── Line Reading ──────────────────────────────────────────────────────────

    def _readline(self) -> str:
        prompt = self._prompt()
        return input(prompt)

    def _prompt(self) -> str:
        """Build prompt showing current context."""
        base = colored("airos", Colors.HEADER)
        parts = []
        if self.current_paper:
            short = self.current_paper[:12]
            parts.append(colored(f"[{short}]", Colors.OKBLUE))
        if self.current_tag:
            parts.append(colored(f"#{self.current_tag}", Colors.OKGREEN))
        suffix = " ".join(parts)
        return f"{base}{' ' + suffix if suffix else ''}> "

    # ── Command Handlers ───────────────────────────────────────────────────────

    def _cmd_search(self, args: str) -> bool:
        """search <query>"""
        query = args.strip()
        if not query:
            print_warning("Usage: search <query>")
            return False
        results, total = self.db.search_papers(query, limit=self.limit)
        print_info(f"[{len(results)} of {total} results]")
        for r in results:
            flag = colored("*", Colors.WARNING) if r.parse_status != "parsed" else " "
            print(f"  {flag} {r.paper_id}  {r.title[:65]}")
            if r.snippet:
                print(f"         {r.snippet[:80]}")
        return False

    def _cmd_kg(self, args: str) -> bool:
        """kg <subcommand> [args]"""
        tokens = args.strip().split(maxsplit=1)
        sub = tokens[0] if tokens else ""
        sub_args = tokens[1] if len(tokens) > 1 else ""

        sub_handler = _KG_SUB_HANDLERS.get(sub)
        if sub_handler is None:
            print_error(f"Unknown kg subcommand: {sub}. Options: {', '.join(_KG_SUB_HANDLERS)}")
            return False
        try:
            return sub_handler(self, sub_args)
        except Exception as exc:
            print_error(f"Error: {exc}")
            return False

    def _kg_stats(self, _args: str) -> bool:
        s = self.kg.stats()
        print_info(f"Nodes: {s['total_nodes']}  Edges: {s['total_edges']}")
        for t, c in sorted(s["nodes_by_type"].items()):
            print(f"  {t:12s}: {c:6d}")
        return False

    def _kg_neighbors(self, args: str) -> bool:
        paper_id = args.strip() or self.current_paper
        if not paper_id:
            print_warning("Usage: kg neighbors [paper_id]  (or just `neighbors`)")
            return False
        neighbors = self.kg.find_neighbors(paper_id, depth=2)
        print_info(f"[{len(neighbors)} neighbors (depth ≤ 2)]")
        for node, edge, depth in sorted(neighbors, key=lambda x: x[2])[: self.limit]:
            print(
                f"  [{depth}] {node['type']:8s} | "
                f"{edge['relation_type']:12s} | {node['label'][:55]}"
            )
        self.current_paper = paper_id
        return False

    def _kg_path(self, args: str) -> bool:
        tokens = args.strip().split()
        if len(tokens) < 2:
            print_warning("Usage: kg path <idA> <idB>  (or just `path <idA> <idB>`)")
            return False
        idA, idB = tokens[0], tokens[1]
        # Try to resolve paper UIDs to node IDs
        nodeA = self.kg.get_node_by_entity("Paper", idA) or self.kg.get_node(idA)
        nodeB = self.kg.get_node_by_entity("Paper", idB) or self.kg.get_node(idB)
        if not nodeA:
            print_error(f"Node A ('{idA}') not found.")
            return False
        if not nodeB:
            print_error(f"Node B ('{idB}') not found.")
            return False
        path = self.kg.find_shortest_path(nodeA["id"], nodeB["id"])
        if not path:
            print_error(f"No path found between '{idA}' and '{idB}'.")
            return False
        print_success(f"Path ({len(path)} hops):")
        for i, nid in enumerate(path):
            node = self.kg.get_node(nid)
            label = (node["label"][:50] if node else nid) if node else nid
            print(f"  {i+1}. [{node['type'] if node else '?'}] {label}")
        return False

    def _kg_tag(self, args: str) -> bool:
        tag = args.strip() or self.current_tag
        if not tag:
            print_warning("Usage: kg tag <tag_name>  (or just `tag <name>`)")
            return False
        papers = self.kg.find_papers_by_tag(tag)
        print_info(f"[Tag '{tag}': {len(papers)} papers]")
        for p in papers[: self.limit]:
            print(f"  {p['entity_id']}  {p['label'][:60]}")
        self.current_tag = tag
        return False

    def _kg_graph(self, args: str) -> bool:
        tokens = args.strip().split(maxsplit=1)
        paper_id = tokens[0] if tokens else self.current_paper
        if not paper_id:
            print_warning("Usage: kg graph <paper_id>  (or just `graph <paper_id>`)")
            return False
        paper_node = self.kg.get_node_by_entity("Paper", paper_id)
        if not paper_node:
            print_error(f"Paper '{paper_id}' not found in KG.")
            return False
        neighbors = self.kg.find_neighbors(paper_node["id"], depth=2)
        print_info(f"[Graph for '{paper_id}': {len(neighbors)} neighbors]")
        for node, edge, depth in sorted(neighbors, key=lambda x: x[2])[: self.limit]:
            print(f"  [{depth}] {node['type']:8s} | {edge['relation_type']:12s} | {node['label'][:50]}")
        self.current_paper = paper_id
        return False

    def _cmd_neighbors(self, args: str) -> bool:
        """Shortcut for kg neighbors"""
        paper_id = args.strip() or self.current_paper
        if not paper_id:
            print_warning("Usage: neighbors [paper_id]")
            return False
        return self._kg_neighbors(paper_id)

    def _cmd_path(self, args: str) -> bool:
        """Shortcut for kg path"""
        return self._kg_path(args)

    def _cmd_stats(self, args: str) -> bool:
        """Shortcut for kg stats"""
        return self._kg_stats(args)

    def _cmd_graph(self, args: str) -> bool:
        """Shortcut for kg graph"""
        return self._kg_graph(args)

    def _cmd_paper(self, args: str) -> bool:
        """paper <uid>"""
        uid = args.strip()
        if not uid:
            print_warning("Usage: paper <uid>")
            return False
        self.current_paper = uid
        print_success(f"Current paper: {uid}")
        return False

    def _cmd_tag(self, args: str) -> bool:
        """tag <name>"""
        tag = args.strip()
        if not tag:
            print_warning("Usage: tag <name>")
            return False
        self.current_tag = tag
        print_success(f"Current tag: {tag}")
        return False

    def _cmd_cd(self, args: str) -> bool:
        """cd paper <uid> | cd tag <name>"""
        tokens = args.strip().split(maxsplit=1)
        if len(tokens) < 2:
            print_warning("Usage: cd paper <uid> | cd tag <name>")
            return False
        kind, val = tokens[0], tokens[1]
        if kind == "paper":
            self.current_paper = val
            print_success(f"Current paper: {val}")
        elif kind == "tag":
            self.current_tag = val
            print_success(f"Current tag: {val}")
        else:
            print_error(f"Unknown context type: {kind}. Use `paper` or `tag`.")
        return False

    def _cmd_pwd(self, _args: str) -> bool:
        """Print current context."""
        if self.current_paper:
            print(f"paper: {self.current_paper}")
        if self.current_tag:
            print(f"tag:   {self.current_tag}")
        if not self.current_paper and not self.current_tag:
            print("(no context set)")
        return False

    def _cmd_help(self, args: str) -> bool:
        """Show help."""
        cmd = args.strip().lower()
        if cmd and cmd in _HELP_MAP:
            print(_HELP_MAP[cmd])
        elif cmd:
            print_error(f"Unknown command: {cmd}")
        else:
            self._print_help_all()
        return False

    def _print_help_all(self):
        print_header("AI Research OS REPL — Commands")
        for line in _HELP_ALL.strip("\n").split("\n"):
            print(line)

    def _cmd_exit(self, _args: str) -> bool:
        """Exit the REPL."""
        return True


# ── Command Registry ────────────────────────────────────────────────────────────

def _make_handler(name: str):
    def handler(self: AIROSRepl, args: str) -> bool:
        return getattr(self, f"_cmd_{name}")(args)
    return handler


_COMMAND_HANDLERS: dict = {
    "search":    AIROSRepl._cmd_search,
    "kg":        AIROSRepl._cmd_kg,
    "neighbors": AIROSRepl._cmd_neighbors,
    "path":      AIROSRepl._cmd_path,
    "stats":     AIROSRepl._cmd_stats,
    "graph":     AIROSRepl._cmd_graph,
    "paper":     AIROSRepl._cmd_paper,
    "tag":       AIROSRepl._cmd_tag,
    "cd":        AIROSRepl._cmd_cd,
    "pwd":       AIROSRepl._cmd_pwd,
    "help":      AIROSRepl._cmd_help,
    "exit":      AIROSRepl._cmd_exit,
    "quit":      AIROSRepl._cmd_exit,
    "?":         AIROSRepl._cmd_help,
}

_KG_SUB_HANDLERS: dict = {
    "stats":     AIROSRepl._kg_stats,
    "neighbors": AIROSRepl._kg_neighbors,
    "path":      AIROSRepl._kg_path,
    "tag":       AIROSRepl._kg_tag,
    "graph":     AIROSRepl._kg_graph,
}

_ALL_COMMANDS = list(_COMMAND_HANDLERS.keys()) + ["kg"]

_HELP_ALL = """
  search <query>         Full-text search indexed papers (DB FTS5)
  kg stats              KG statistics (nodes/edges by type)
  kg neighbors [uid]     BFS neighbors of paper (default: current paper)
  kg path <idA> <idB>  Shortest path between two KG nodes
  kg tag <name>         Papers + notes for a tag
  kg graph [uid]        Ego subgraph for a paper
  paper <uid>           Set current paper context
  tag <name>            Set current tag context
  cd paper <uid>        Set current paper (alias)
  cd tag <name>         Set current tag (alias)
  pwd                   Show current context
  stats                 KG statistics (shortcut: kg stats)
  neighbors [uid]        Neighbors of paper (shortcut: kg neighbors)
  graph [uid]           Ego subgraph (shortcut: kg graph)
  path <a> <b>          Shortest path (shortcut: kg path)
  help [cmd]            Show help
  exit / quit           Exit REPL
"""

_HELP_MAP = {
    "search":    "search <query> — Full-text search indexed papers using SQLite FTS5",
    "kg":        "kg <sub> — Knowledge graph: stats | neighbors | path | tag | graph",
    "neighbors": "neighbors [uid] — Show KG BFS neighbors (uses current paper if omitted)",
    "path":      "path <idA> <idB> — Shortest path between two KG nodes",
    "stats":     "stats — KG statistics (node/edge counts by type)",
    "graph":     "graph [uid] — Ego subgraph for a paper (shortcut: kg graph)",
    "paper":     "paper <uid> — Set current paper context",
    "tag":       "tag <name> — Set current tag context",
    "cd":        "cd paper <uid> | cd tag <name> — Change current context",
    "pwd":       "pwd — Print current context (paper/tag)",
    "help":      "help [command] — Show help",
    "exit":      "exit — Exit the REPL",
    "quit":      "quit — Alias for exit",
}


# ── CLI Entry Point ─────────────────────────────────────────────────────────────


def _build_repl_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "repl",
        help="Enter interactive REPL",
        description="AI Research OS interactive REPL for chained KG/paper exploration.",
        epilog="""\
Examples:
  %(prog)s                    # start REPL
  %(prog)s --limit 50         # increase result limit per page
""",
    )
    p.add_argument(
        "--limit", type=int, default=DEFAULT_LIMIT,
        help=f"Result limit per command (default {DEFAULT_LIMIT})",
    )
    return p


def _run_repl(args: argparse.Namespace) -> int:
    repl = AIROSRepl(limit=args.limit)
    return repl.run()
