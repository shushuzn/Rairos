"""Code base indexer using jieba word segmentation for fast code search."""
from __future__ import annotations

import os
import re
from pathlib import Path
from typing import Dict, List, Set, Tuple
from dataclasses import dataclass
import threading

import jieba

# Lock for thread-safe jieba operations
_jieba_lock = threading.Lock()

# Code-specific stopwords (common Python keywords that add no search value)
_STOPWORDS = {
    # Python keywords
    "def", "class", "return", "if", "else", "elif", "for", "while", "try", "except",
    "finally", "with", "as", "import", "from", "pass", "break", "continue", "and", "or",
    "not", "in", "is", "None", "True", "False", "lambda", "yield", "async", "await",
    "self", "cls", "global", "nonlocal", "assert", "raise", "del",
    # Common patterns
    "print", "len", "range", "list", "dict", "set", "tuple", "str", "int", "float",
    "bool", "type", "open", "read", "write", "close", "get", "set", "add", "update",
}


@dataclass
class CodeToken:
    """A token extracted from code."""
    token: str
    file: str
    line: int
    context: str  # Surrounding code snippet


class CodeIndexer:
    """
    Indexer for Python code using jieba segmentation.

    Extracts tokens from:
    - Python docstrings and comments
    - Function/class names (split by underscores)
    - Variable names (split by case)
    - String literals
    """

    def __init__(self):
        self._index: Dict[str, Set[Tuple[str, int, str]]] = {}  # token → {(file, line, context)}
        self._files: Dict[str, float] = {}  # file → mtime
        self._lock = threading.Lock()

    def add_file(self, file_path: str, content: str) -> int:
        """Add or update a file in the index. Returns token count."""
        tokens = self._extract_tokens(content)
        count = 0

        with self._lock:
            # Remove old entries for this file
            for token in list(self._index.keys()):
                self._index[token] = {
                    entry for entry in self._index[token]
                    if entry[0] != file_path
                }
                if not self._index[token]:
                    del self._index[token]

            # Add new entries
            for token_info in tokens:
                if token_info.token not in self._index:
                    self._index[token_info.token] = set()
                self._index[token_info.token].add((token_info.file, token_info.line, token_info.context))
                count += 1

        return count

    def remove_file(self, file_path: str) -> None:
        """Remove a file from the index."""
        with self._lock:
            for token in list(self._index.keys()):
                self._index[token] = {
                    entry for entry in self._index[token]
                    if entry[0] != file_path
                }
                if not self._index[token]:
                    del self._index[token]

    def search(self, query: str, limit: int = 20) -> List[CodeToken]:
        """Search for query and return matching tokens with context."""
        # Tokenize query with jieba
        query_tokens = list(jieba.cut(query.lower()))
        query_tokens = [t.strip() for t in query_tokens if t.strip() and len(t) >= 2]

        if not query_tokens:
            return []

        with self._lock:
            # Find files containing all query tokens
            candidate_files: Dict[str, int] = {}
            for q_token in query_tokens:
                for token, entries in self._index.items():
                    if q_token in token.lower():
                        for file_path, line, context in entries:
                            candidate_files[file_path] = candidate_files.get(file_path, 0) + 1

            # Score and rank by match count
            scored = [
                (count, file_path)
                for file_path, count in candidate_files.items()
            ]
            scored.sort(reverse=True)

            results = []
            for _, file_path in scored[:limit]:
                # Get best match context
                for token, entries in self._index.items():
                    for fp, line, context in entries:
                        if fp == file_path and any(q in token.lower() for q in query_tokens):
                            results.append(CodeToken(
                                token=token,
                                file=fp,
                                line=line,
                                context=context[:100]
                            ))
                            break
                    if len(results) >= limit:
                        break

            return results[:limit]

    def _extract_tokens(self, content: str) -> List[CodeToken]:
        """Extract tokens from file content."""
        tokens = []
        lines = content.split('\n')

        for line_num, line in enumerate(lines, 1):
            # Skip empty lines and pure whitespace
            if not line.strip():
                continue

            # Extract from comments and docstrings
            comment_match = re.search(r'#.*$|"[^"]*"|\'[^\']*\'', line)
            if comment_match:
                comment = comment_match.group(0).strip('# "\'')
                if comment:
                    for word in jieba.cut(comment):
                        word = word.strip().lower()
                        if len(word) >= 2 and word not in _STOPWORDS:
                            tokens.append(CodeToken(
                                token=word,
                                file="",
                                line=line_num,
                                context=line.strip()[:100]
                            ))

            # Extract from function/class names (split by underscore/camelCase)
            name_match = re.search(r'(def|class)\s+(\w+)', line)
            if name_match:
                name = name_match.group(2)
                # Split by underscore
                parts = name.split('_')
                # Split camelCase
                camel_parts = re.findall(r'[A-Z]?[a-z]+|[A-Z]+(?=[A-Z]|$)', name)

                for part in parts + camel_parts:
                    part = part.lower()
                    if len(part) >= 2 and part not in _STOPWORDS:
                        tokens.append(CodeToken(
                            token=part,
                            file="",
                            line=line_num,
                            context=line.strip()[:100]
                        ))

        return tokens

    @property
    def size(self) -> int:
        """Return number of indexed tokens."""
        with self._lock:
            return len(self._index)

    def stats(self) -> Dict[str, int]:
        """Return index statistics."""
        with self._lock:
            return {
                "unique_tokens": len(self._index),
                "indexed_files": len(set(f for entries in self._index.values() for f, _, _ in entries)),
                "total_entries": sum(len(e) for e in self._index.values())
            }


# Global index instance
_code_indexer: CodeIndexer | None = None


def get_code_indexer() -> CodeIndexer:
    """Get the global code indexer instance."""
    global _code_indexer
    if _code_indexer is None:
        _code_indexer = CodeIndexer()
        # Load existing code on first access
        _load_project_code(_code_indexer)
    return _code_indexer


def _load_project_code(indexer: CodeIndexer) -> None:
    """Load all Python files from the project into the indexer."""
    project_root = Path(__file__).parent.parent

    # Common directories to skip
    skip_dirs = {'.venv', '__pycache__', '.git', 'build', 'dist', '.pytest_cache',
                 'node_modules', 'data', 'cache'}

    for py_file in project_root.rglob('*.py'):
        # Skip if in excluded directory
        if any(skip in py_file.parts for skip in skip_dirs):
            continue

        try:
            content = py_file.read_text(encoding='utf-8', errors='ignore')
            rel_path = str(py_file.relative_to(project_root))
            indexer.add_file(rel_path, content)
        except Exception:
            pass
