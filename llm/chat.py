"""RAG Chat - Intelligent Q&A with your paper library.

Provides natural language Q&A over your paper corpus with source citation.
"""
from __future__ import annotations

import os
import re
from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Callable, List, Optional, Tuple

from llm.client import call_llm_chat_completions
from llm.evolution_report import get_adaptive_retrieval


class QueryType(Enum):
    """Query type classification for adaptive routing."""
    FACTUAL = "factual"      # Who, when, what (exact facts)
    CONCEPTUAL = "conceptual"  # Explain, how, why (understanding)
    COMPARATIVE = "comparative"  # vs, compared, difference (analysis)
    TEMPORAL = "temporal"    # recent, latest, 2024, new (time-sensitive)
    GENERAL = "general"       # Default fallback


# ─── Data Structures ──────────────────────────────────────────────────────────────


@dataclass
class Citation:
    """A citation extracted from a paper."""
    paper_id: str
    paper_title: str
    snippet: str
    relevance_score: float


@dataclass
class ChatContext:
    """A retrieved context from a paper."""
    paper_id: str
    paper_title: str
    authors: List[str]
    published: str
    snippet: str
    relevance_score: float


@dataclass
class ChatResult:
    """Result of a RAG chat interaction."""
    answer: str
    citations: List[Citation] = field(default_factory=list)
    papers_used: List[str] = field(default_factory=list)


# ─── System Prompt ─────────────────────────────────────────────────────────────


_RAG_SYSTEM_PROMPT = """你是一个严谨的 AI 研究助手，精通论文阅读和学术分析。

核心原则：
1. 基于原文回答，不要捏造或推测未提及的内容
2. 不确定的信息必须加 [推测] 标注
3. 使用 > 块引用格式引用原文片段
4. 区分"原文明确说"和"可推断"
5. 回答使用中文，但引用原文时保留英文原句

输出格式：
- 开头总结回答要点（1-2句话）
- 详细解释部分引用原文片段
- 结尾标注信息来源
"""


# ─── RAG Chat Implementation ──────────────────────────────────────────────────────


class RagChat:
    """RAG Chat for paper Q&A.

    Provides natural language question answering over a paper corpus
    with full source citation tracking.
    """

    def __init__(
        self,
        db,
        api_key: Optional[str] = None,
        base_url: Optional[str] = None,
        model: Optional[str] = None,
        call_llm: Optional[Callable[..., Any]] = None,
    ):
        """
        Initialize RAG Chat.

        Args:
            db: Database instance with paper storage
            api_key: LLM API key (falls back to OPENAI_API_KEY env)
            base_url: LLM API base URL
            model: Model name
            call_llm: Override LLM call function (for testing)
        """
        self.db = db
        self.api_key = api_key or os.getenv("OPENAI_API_KEY", "")
        self.base_url = base_url or os.getenv("OPENAI_BASE_URL", "https://api.openai.com/v1")
        self.model = model or os.getenv("DEFAULT_LLM_MODEL", "gpt-4o-mini")
        self._call_llm = call_llm or call_llm_chat_completions
        # Lazy-load adaptive retrieval to avoid circular imports
        self._adaptive = None
        # Cache compiled patterns for query classification
        self._query_patterns = self._compile_query_patterns()

    def _compile_query_patterns(self) -> dict:
        """Pre-compile regex patterns for query classification."""
        return {
            QueryType.FACTUAL: [
                re.compile(r'\b(who|whom|whose|who\'s)\b', re.I),
                re.compile(r'\b(when|what year|what date)\b', re.I),
                re.compile(r'\b(which (paper|author|model))\b', re.I),
                re.compile(r'\b(who proposed|who introduced|who published)\b', re.I),
                # 中文模式
                re.compile(r'(是谁|谁提出|谁发明|谁提出|谁创建|谁发明|哪篇|哪个作者|哪篇论文)'),
            ],
            QueryType.CONCEPTUAL: [
                re.compile(r'\b(what is|what are|explain|describe|how does|how do|why does|why do|understand|definition|meaning)\b', re.I),
                re.compile(r'(原理|机制|概念|解释|是什么|如何|为什么|理解|定义|工作原理)'),
            ],
            QueryType.COMPARATIVE: [
                re.compile(r'\b(vs|versus|compared to|compared with|difference between| versus | vs\. )\b', re.I),
                re.compile(r'\b(和.*比较|区别|差异|对比|优于|劣于)\b'),
            ],
            QueryType.TEMPORAL: [
                re.compile(r'\b(recent|latest|newest|202[0-9]|20[2-9]\d)\b', re.I),
                re.compile(r'\b(最近|最新|新的|202[0-9]|今年|去年|明年)\b'),
                re.compile(r'\b(published in|released in|presented in|from 20)\b', re.I),
            ],
        }

    def classify_query(self, query: str) -> QueryType:
        """
        Classify query type for adaptive retrieval strategy.

        Uses keyword/pattern matching for fast classification without LLM call.
        """
        scores = {qt: 0 for qt in QueryType}

        for qtype, patterns in self._query_patterns.items():
            for pattern in patterns:
                if pattern.search(query):
                    scores[qtype] += 1

        # Return type with highest score, fallback to GENERAL
        max_score = max(scores.values())
        if max_score == 0:
            return QueryType.GENERAL

        for qtype, score in scores.items():
            if score == max_score:
                return qtype

        return QueryType.GENERAL

    def chat(
        self,
        question: str,
        paper_id: Optional[str] = None,
        concept: Optional[str] = None,
        limit: int = 5,
        verbose: bool = False,
    ) -> ChatResult:
        """
        Answer a question using the paper corpus.

        Args:
            question: Natural language question
            paper_id: Limit to specific paper
            concept: Filter by concept/tag
            limit: Max papers to retrieve
            verbose: Print debug info

        Returns:
            ChatResult with answer and citations
        """
        # Classify query for adaptive routing (used in verbose mode)
        query_type = self.classify_query(question)

        # 1. Retrieve relevant contexts
        contexts = self._retrieve(question, paper_id, concept, limit)

        if not contexts:
            return ChatResult(
                answer="⚠️ 未找到相关论文。请确保：\n"
                       "1. 论文已添加到数据库\n"
                       "2. 论文已解析（包含全文）\n"
                       "3. 使用 --limit 增加检索范围",
                citations=[],
                papers_used=[],
            )

        if verbose:
            qtype_name = query_type.value
            print(f"[{qtype_name}] Retrieved {len(contexts)} contexts from {len(set(c.paper_id for c in contexts))} papers")

        # 2. Build prompt
        prompt = self._build_prompt(question, contexts)

        # 3. Generate answer
        answer = self._call_llm(
            base_url=self.base_url,
            api_key=self.api_key,
            model=self.model,
            system_prompt=_RAG_SYSTEM_PROMPT,
            user_prompt=prompt,
        )

        # 4. Extract citations
        citations = self._extract_citations(contexts)

        return ChatResult(
            answer=answer,
            citations=citations,
            papers_used=list(set(c.paper_id for c in contexts)),
        )

    def _retrieve(
        self,
        query: str,
        paper_id: Optional[str],
        concept: Optional[str],
        limit: int,
    ) -> List[ChatContext]:
        """
        Retrieve relevant contexts from papers using adaptive routing.

        Query type determines retrieval strategy:
        - FACTUAL: BM25 exact match (keywords)
        - CONCEPTUAL: Semantic similarity (embedding)
        - COMPARATIVE: Hybrid with balanced weights
        - TEMPORAL: BM25 + temporal boost (newer papers)
        - GENERAL: Default BM25 + adaptive boost
        """
        # Classify query for adaptive routing
        query_type = self.classify_query(query)

        contexts: List[ChatContext] = []
        seen_papers = set()

        # Strategy based on query type
        if paper_id:
            # If specific paper, search within that paper's content
            paper = self.db.get_paper(paper_id)
            if paper and paper.plain_text:
                snippet = self._extract_snippet(paper.plain_text, query)
                if snippet:
                    contexts.append(ChatContext(
                        paper_id=paper.id,
                        paper_title=paper.title or "Unknown",
                        authors=paper.authors or [],
                        published=paper.published or "",
                        snippet=snippet,
                        relevance_score=1.0,
                    ))
                    seen_papers.add(paper.id)
        else:
            # Adaptive retrieval based on query type
            results = self._adaptive_retrieve(query, query_type, concept, limit * 2)

            for result in results:
                if result.paper_id in seen_papers:
                    continue

                # Get full paper for content
                paper = self.db.get_paper(result.paper_id)
                if paper and paper.plain_text:
                    snippet = self._extract_snippet(paper.plain_text, query)
                    if snippet:
                        contexts.append(ChatContext(
                            paper_id=paper.id,
                            paper_title=paper.title or result.title,
                            authors=paper.authors or result.authors,
                            published=paper.published or result.published,
                            snippet=snippet,
                            relevance_score=abs(result.score) if result.score else 0.5,
                        ))
                        seen_papers.add(paper.id)

                if len(contexts) >= limit:
                    break

        # Sort by relevance and deduplicate
        contexts.sort(key=lambda x: x.relevance_score, reverse=True)

        return contexts[:limit]

    def _adaptive_retrieve(
        self,
        query: str,
        query_type: QueryType,
        concept: Optional[str],
        limit: int,
    ):
        """
        Adaptive retrieval with query-type-specific strategy.

        Args:
            query: User query
            query_type: Classified query type
            concept: Optional concept filter
            limit: Max results

        Returns:
            List of search results with scores
        """
        # Base BM25 search
        results, _ = self.db.search_papers(
            query=query,
            limit=limit,
            parse_status="parsed",
        )

        # Apply query-type-specific strategies
        if query_type == QueryType.TEMPORAL:
            # Boost newer papers for temporal queries
            results = self._temporal_boost(results)
        elif query_type == QueryType.CONCEPTUAL:
            # For conceptual queries, try semantic similarity if available
            results = self._semantic_rerank(query, results)
        elif query_type == QueryType.COMPARATIVE:
            # For comparisons, include papers from different time periods
            results = self._diversity_boost(results)

        # Always apply adaptive boost from feedback history
        results = self._apply_adaptive_boost(results)

        return results

    def _temporal_boost(self, results: list) -> list:
        """Boost newer papers for temporal queries."""
        for result in results:
            # Extract year from published date
            score = abs(result.score) if result.score else 0.5
            if result.published:
                year_match = re.search(r'(20[2-9]\d|19[9]\d)', result.published)
                if year_match:
                    year = int(year_match.group(1))
                    current_year = 2026
                    # Exponential decay: newer = higher boost
                    years_ago = current_year - year
                    if years_ago <= 2:
                        score *= 1.5  # Recent papers get 50% boost
                    elif years_ago <= 5:
                        score *= 1.2
                    result.score = score
        return results

    def _semantic_rerank(self, query: str, results: list) -> list:
        """
        Rerank using semantic similarity for conceptual queries.
        Falls back to BM25 if embeddings not available.
        """
        # Check if we have embedding support
        if hasattr(self.db, 'find_similar') and results:
            # Use first result as anchor for similarity
            anchor_id = results[0].paper_id
            try:
                similar = self.db.find_similar(anchor_id, limit=len(results))
                # Create score map
                sim_scores = {s['paper_id']: s.get('score', 0.5) for s in similar}
                # Blend BM25 and semantic scores
                for r in results:
                    bm25_score = abs(r.score) if r.score else 0.5
                    sem_score = sim_scores.get(r.paper_id, 0.5)
                    r.score = 0.4 * bm25_score + 0.6 * sem_score
            except Exception:
                pass  # Fall back to BM25
        return results

    def _diversity_boost(self, results: list) -> list:
        """Boost diversity for comparative queries (different time periods)."""
        seen_years = set()
        adjusted_results = []

        for result in results:
            score = abs(result.score) if result.score else 0.5

            # Extract year
            year = None
            if result.published:
                year_match = re.search(r'(20[2-9]\d|19[9]\d)', result.published)
                if year_match:
                    year = int(year_match.group(1))

            # Diversity boost: first paper from each era gets boost
            if year:
                era = year // 5  # 5-year buckets
                if era not in seen_years:
                    score *= 1.3
                    seen_years.add(era)

            result.score = score
            adjusted_results.append(result)

        return adjusted_results

    def _extract_snippet(self, text: str, query: str, context_chars: int = 500) -> str:
        """
        Extract a relevant snippet from text around query keywords.

        Args:
            text: Full text to search
            query: Query string
            context_chars: Characters of context around match

        Returns:
            Extracted snippet with surrounding context
        """
        if not text or not query:
            return text[:context_chars] if text else ""

        # Find query terms in text (case-insensitive)
        query_terms = query.lower().split()
        text_lower = text.lower()

        best_pos = -1
        for term in query_terms:
            if len(term) < 3:
                continue
            pos = text_lower.find(term)
            if pos != -1:
                best_pos = pos
                break

        if best_pos == -1:
            # No exact match, return beginning
            return text[:context_chars]

        # Extract context around match
        start = max(0, best_pos - context_chars // 2)
        end = min(len(text), best_pos + context_chars // 2)

        snippet = text[start:end].strip()

        # Add ellipsis if truncated
        prefix = "..." if start > 0 else ""
        suffix = "..." if end < len(text) else ""

        return f"{prefix}{snippet}{suffix}"

    def _build_prompt(self, question: str, contexts: List[ChatContext]) -> str:
        """Build RAG prompt with retrieved contexts."""
        # Group contexts by paper
        paper_contexts: dict[str, List[ChatContext]] = {}
        for ctx in contexts:
            if ctx.paper_id not in paper_contexts:
                paper_contexts[ctx.paper_id] = []
            paper_contexts[ctx.paper_id].append(ctx)

        # Build context text
        context_parts = []
        for paper_id, ctxs in paper_contexts.items():
            ctx = ctxs[0]  # Use most relevant snippet
            snippets = "\n\n".join([f"> {c.snippet}" for c in ctxs[:2]])
            context_parts.append(
                f"【论文】{ctx.paper_title}\n"
                f"作者：{', '.join(ctx.authors[:3]) if ctx.authors else 'Unknown'}\n"
                f"年份：{ctx.published[:4] if ctx.published else 'N/A'}\n\n"
                f"{snippets}"
            )

        context_text = "\n\n---\n\n".join(context_parts)

        prompt = f"""基于以下论文内容回答问题。如果信息不足以回答，请明确说明。

【论文内容】
{context_text}

【问题】
{question}

请按以下格式回答：
1. 首先用1-2句话总结回答要点
2. 然后详细解释，引用原文片段（用 > 块引用格式）
3. 对于不确定的信息，加 [推测] 标注
4. 最后列出参考论文列表
"""
        return prompt

    def _extract_citations(self, contexts: List[ChatContext]) -> List[Citation]:
        """Extract citations from retrieved contexts."""
        citations = []
        seen = set()

        for ctx in contexts:
            if ctx.paper_id in seen:
                continue
            seen.add(ctx.paper_id)

            citations.append(Citation(
                paper_id=ctx.paper_id,
                paper_title=ctx.paper_title,
                snippet=ctx.snippet[:200] + "..." if len(ctx.snippet) > 200 else ctx.snippet,
                relevance_score=ctx.relevance_score,
            ))

        return citations

    def _apply_adaptive_boost(self, results: List) -> List:
        """Apply adaptive boost to search results based on feedback history."""
        try:
            # Lazy initialization to avoid circular import
            if self._adaptive is None:
                from llm.evolution_report import get_adaptive_retrieval
                self._adaptive = get_adaptive_retrieval()
            # Convert to dict format for apply_boost
            result_dicts = [
                {"paper_id": r.paper_id, "score": abs(r.score) if r.score else 0.5}
                for r in results
            ]
            boosted = self._adaptive.apply_boost(result_dicts, decay=0.1)
            # Re-sort results by boosted score
            boosted_ids = [b["paper_id"] for b in boosted]
            boosted_scores = {b["paper_id"]: b["score"] for b in boosted}
            # Return original results re-sorted by boosted score
            return sorted(results, key=lambda r: boosted_scores.get(r.paper_id, 0), reverse=True)
        except Exception:
            # Fallback to original order on error
            return results

    def format_result(self, result: ChatResult, show_citations: bool = True) -> str:
        """Format ChatResult for terminal output."""
        output = []
        output.append("─" * 60)
        output.append(result.answer)
        output.append("─" * 60)

        if show_citations and result.citations:
            output.append("\n📖 引用来源：")
            for i, cite in enumerate(result.citations, 1):
                authors = ""
                output.append(f"\n[{i}] {cite.paper_title} ({cite.paper_id})")
                output.append(f"    相关度: {cite.relevance_score:.2f}")
                output.append(f"    > {cite.snippet[:150]}...")

        return "\n".join(output)


# ─── Convenience Functions ──────────────────────────────────────────────────────


def rag_chat(
    question: str,
    paper_id: Optional[str] = None,
    concept: Optional[str] = None,
    limit: int = 5,
    api_key: Optional[str] = None,
    base_url: Optional[str] = None,
    model: Optional[str] = None,
) -> ChatResult:
    """
    One-shot RAG chat.

    Args:
        question: Question to answer
        paper_id: Limit to specific paper
        concept: Filter by concept/tag
        limit: Max papers to retrieve
        api_key: LLM API key
        base_url: LLM API base URL
        model: Model name

    Returns:
        ChatResult with answer and citations
    """
    from cli._shared import get_db

    db = get_db()
    db.init()

    chat = RagChat(db, api_key, base_url, model)
    return chat.chat(question, paper_id, concept, limit)
