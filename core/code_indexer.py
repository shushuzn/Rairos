"""Code base indexer using Ollama embeddings for semantic code search."""
from __future__ import annotations

import json
import math
import re
import threading
import urllib.request
from pathlib import Path
from typing import Any, Dict, List, Optional

import jieba

# Ollama embedding model
DEFAULT_EMBED_MODEL = "nomic-embed-text"

# Code-specific stopwords
_STOPWORDS = {
    "def", "class", "return", "if", "else", "elif", "for", "while", "try", "except",
    "finally", "with", "as", "import", "from", "pass", "break", "continue", "and", "or",
    "not", "in", "is", "None", "True", "False", "lambda", "yield", "async", "await",
    "self", "cls", "global", "nonlocal", "assert", "raise", "del",
    "print", "len", "range", "list", "dict", "set", "tuple", "str", "int", "float",
    "bool", "type", "open", "read", "write", "close", "get", "set", "add", "update",
}


class CodeChunk:
    """A chunk of code with its embedding."""
    def __init__(self, chunk_id: str, file: str, line: int, content: str, embedding: Optional[List[float]] = None):
        self.id = chunk_id
        self.file = file
        self.line = line
        self.content = content
        self.embedding = embedding


class CodeIndexer:
    """
    Semantic code indexer using Ollama embeddings.

    Chunks code into functions/classes and creates embeddings
    for semantic search via cosine similarity.
    """

    def __init__(self, embed_model: str = DEFAULT_EMBED_MODEL):
        self.embed_model = embed_model
        self._chunks: List[CodeChunk] = []
        self._file_chunks: Dict[str, List[int]] = {}  # file → chunk indices
        self._embeddings_cache: Dict[str, List[float]] = {}  # chunk_id → embedding
        self._lock = threading.Lock()
        self._initialized = False

    def _get_embedding(self, text: str) -> Optional[List[float]]:
        """Get embedding from Ollama."""
        cache_key = text[:200]  # Truncate for cache key
        if cache_key in self._embeddings_cache:
            return self._embeddings_cache[cache_key]

        try:
            req = urllib.request.Request(
                "http://localhost:11434/api/embeddings",
                data=json.dumps({"model": self.embed_model, "prompt": text}).encode(),
                headers={"Content-Type": "application/json"},
                method="POST",
            )
            with urllib.request.urlopen(req, timeout=60) as resp:
                data = json.loads(resp.read())
                embedding = data.get("embedding")
                if embedding:
                    self._embeddings_cache[cache_key] = embedding
                return embedding
        except Exception:
            return None

    def _cosine_sim(self, a: List[float], b: List[float]) -> float:
        """Compute cosine similarity between two vectors."""
        dot = sum(x * y for x, y in zip(a, b))
        norm_a = math.sqrt(sum(x * x for x in a))
        norm_b = math.sqrt(sum(y * y for y in b))
        if norm_a == 0 or norm_b == 0:
            return 0.0
        return dot / (norm_a * norm_b)

    def initialize(self, project_root: Optional[Path] = None) -> None:
        """Index all Python files in the project."""
        if self._initialized:
            return

        if project_root is None:
            project_root = Path(__file__).parent.parent

        skip_dirs = {'.venv', '__pycache__', '.git', 'build', 'dist',
                     '.pytest_cache', 'node_modules', 'data', 'cache'}

        for py_file in project_root.rglob('*.py'):
            if any(skip in py_file.parts for skip in skip_dirs):
                continue
            try:
                self.add_file(str(py_file.relative_to(project_root)), py_file.read_text(encoding='utf-8', errors='ignore'))
            except Exception:
                pass

        self._initialized = True

    def add_file(self, file_path: str, content: str) -> int:
        """Add or update a file in the index. Returns chunk count."""
        # Remove old chunks for this file
        with self._lock:
            if file_path in self._file_chunks:
                old_indices = self._file_chunks[file_path]
                self._chunks = [c for i, c in enumerate(self._chunks) if i not in old_indices]
                del self._file_chunks[file_path]

        # Extract code chunks
        chunks = self._extract_chunks(file_path, content)
        chunk_start = len(self._chunks)

        with self._lock:
            self._chunks.extend(chunks)
            self._file_chunks[file_path] = list(range(chunk_start, len(self._chunks)))

        return len(chunks)

    def _extract_chunks(self, file: str, content: str) -> List[CodeChunk]:
        """Extract code chunks (functions, classes, comments)."""
        chunks = []
        lines = content.split('\n')
        current_chunk_lines: List[str] = []
        current_line = 1

        for line_num, line in enumerate(lines, 1):
            stripped = line.strip()

            # Start new chunk on function/class definition
            if re.match(r'^(def |class |async def )', stripped):
                if current_chunk_lines:
                    text = '\n'.join(current_chunk_lines).strip()
                    if text:
                        chunks.append(CodeChunk(f"{file}:{current_line}", file, current_line, text))
                        current_chunk_lines = []
                current_line = line_num

            current_chunk_lines.append(line)

        # Don't forget the last chunk
        if current_chunk_lines:
            text = '\n'.join(current_chunk_lines).strip()
            if text:
                chunks.append(CodeChunk(f"{file}:{current_line}", file, current_line, text))

        return chunks

    def search(self, query: str, limit: int = 10, use_semantic: bool = True, top_k: int = 50) -> List[CodeChunk]:
        """Search for query with hybrid approach: keyword filter + semantic rerank."""
        if not self._chunks:
            self.initialize()

        if not use_semantic:
            return self._keyword_search(query, limit)

        # Step 1: Keyword filter - get top_k candidates
        candidates = self._keyword_search(query, top_k)
        if not candidates:
            return []

        # Step 2: Semantic rerank on candidates only
        query_emb = self._get_embedding(query)
        if not query_emb:
            return candidates[:limit]

        scored = []
        for chunk in candidates:
            # Embed on-demand (lazy)
            if chunk.embedding is None:
                chunk.embedding = self._get_embedding(chunk.content[:500])  # Truncate for speed
            if chunk.embedding:
                sim = self._cosine_sim(query_emb, chunk.embedding)
                scored.append((sim, chunk))

        scored.sort(reverse=True)
        return [c for _, c in scored[:limit]]

    def _keyword_search(self, query: str, limit: int) -> List[CodeChunk]:
        """Fallback keyword search using jieba."""
        query_tokens = set(jieba.cut(query.lower()))
        query_tokens = {t for t in query_tokens if len(t) >= 2 and t not in _STOPWORDS}

        scored = []
        with self._lock:
            for chunk in self._chunks:
                content_lower = chunk.content.lower()
                matches = sum(1 for t in query_tokens if t in content_lower)
                if matches > 0:
                    scored.append((matches, chunk))

        scored.sort(reverse=True, key=lambda x: (x[0], x[1].file, x[1].line))
        return [c for _, c in scored[:limit]]

    @property
    def size(self) -> int:
        return len(self._chunks)

    def stats(self) -> Dict[str, Any]:
        with self._lock:
            return {
                "chunks": len(self._chunks),
                "files": len(self._file_chunks),
                "embeddings_cached": len(self._embeddings_cache),
            }


# Global index instance
_code_indexer: Optional[CodeIndexer] = None


def get_code_indexer() -> CodeIndexer:
    """Get the global code indexer instance."""
    global _code_indexer
    if _code_indexer is None:
        _code_indexer = CodeIndexer()
        _code_indexer.initialize()
    return _code_indexer
