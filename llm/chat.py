"""RAG Chat - Intelligent Q&A with your paper library.

Provides natural language Q&A over your paper corpus with source citation.
"""
from __future__ import annotations

import os
from dataclasses import dataclass, field
from typing import Any, Callable, List, Optional, Tuple

from llm.client import call_llm_chat_completions


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
            print(f"Retrieved {len(contexts)} contexts from {len(set(c.paper_id for c in contexts))} papers")

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
        Retrieve relevant contexts from papers.

        Uses BM25 full-text search to find relevant papers,
        then extracts snippets from their content.
        """
        contexts: List[ChatContext] = []
        seen_papers = set()

        # 1. BM25 search for relevant papers
        if paper_id:
            # If specific paper, search within that paper's content
            paper = self.db.get_paper(paper_id)
            if paper and paper.plain_text:
                # Simple keyword match within paper
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
            # Full corpus search
            results, _ = self.db.search_papers(
                query=query,
                limit=limit * 2,
                parse_status="parsed",  # Only fully parsed papers
            )

            for result in results:
                if result.paper_id in seen_papers:
                    continue
                if concept:
                    # Filter by concept/tag
                    tags = self.db.get_tags(result.paper_id)
                    if concept.lower() not in [t.lower() for t in tags]:
                        continue

                # Get full paper for content
                paper = self.db.get_paper(result.paper_id)
                if paper and paper.plain_text:
                    # Extract relevant snippet
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

        # 2. Sort by relevance and deduplicate
        contexts.sort(key=lambda x: x.relevance_score, reverse=True)

        return contexts[:limit]

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
