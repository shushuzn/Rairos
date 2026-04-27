"""Research Chat: AI research assistant with context awareness."""
from dataclasses import dataclass, field
from typing import List, Optional, Dict, Any
from enum import Enum


class QueryType(Enum):
    """Query classification types."""
    FACTUAL = "factual"  # Factual questions
    INFERENTIAL = "inferential"  # Reasoning questions
    DISCOVERY = "discovery"  # Discovery/gap questions
    COMPARISON = "comparison"  # Comparison questions


@dataclass
class PaperContext:
    """Paper context for chat."""
    uid: str
    title: str
    abstract: str
    authors: List[str]
    year: int
    key_findings: List[str] = field(default_factory=list)


@dataclass
class ResearchContext:
    """User's research context."""
    topic: str
    papers: List[PaperContext] = field(default_factory=list)
    insights: List[Any] = field(default_factory=list)  # InsightCard
    relations: Dict[str, List[str]] = field(default_factory=dict)
    citations: Dict[str, List[str]] = field(default_factory=dict)


class ResearchChat:
    """Research chat engine with context awareness."""

    # Chinese and English stopwords for topic extraction
    STOPWORDS = {
        "的", "是", "什么", "如何", "为什么", "有", "哪些", "哪个",
        "和", "与", "在", "了", "都", "也", "要", "能", "可以",
        "the", "a", "an", "is", "are", "was", "were", "what",
        "how", "why", "which", "this", "that", "these", "those",
    }

    def __init__(self, db=None, insight_manager=None):
        self.db = db
        self.insight_manager = insight_manager
        self._chat_history: List[Dict[str, str]] = []

    # ── Context Building ────────────────────────────────────

    def build_context(
        self,
        query: str,
        topic_hint: Optional[str] = None,
    ) -> ResearchContext:
        """Build research context from query."""
        topic = topic_hint or self._extract_topic(query)

        # 1. Find relevant papers
        papers = self._find_relevant_papers(topic)

        # 2. Find relevant insights
        insights = []
        if self.insight_manager:
            insights = self.insight_manager.search_cards(query=topic)

        # 3. Build relations (placeholder for future KG integration)
        relations, citations = self._build_relations(papers)

        return ResearchContext(
            topic=topic,
            papers=papers,
            insights=insights,
            relations=relations,
            citations=citations,
        )

    def _extract_topic(self, query: str) -> str:
        """Extract topic from query."""
        import re
        # Tokenize: Chinese sequences (2+ chars) and English words as atomic tokens
        words = re.findall(r'[\u4e00-\u9fff]{2,}|[a-zA-Z]+', query)

        # Filter stopwords and short words
        candidates = [
            w for w in words
            if w.lower() not in self.STOPWORDS and len(w) > 1
        ]

        # Return top 3 candidates joined
        if candidates:
            return " ".join(candidates[:3])
        return query[:20]

    def _find_relevant_papers(
        self,
        topic: str,
        limit: int = 10,
    ) -> List[PaperContext]:
        """Find relevant papers from database."""
        if not self.db:
            return []

        rows, _ = self.db.search_papers(topic, limit=limit)
        return [
            PaperContext(
                uid=r.uid,
                title=r.title,
                abstract=getattr(r, "abstract", ""),
                authors=(
                    getattr(r, "authors", "").split(",")
                    if hasattr(r, "authors")
                    else []
                ),
                year=getattr(r, "year", 2020),
            )
            for r in rows
        ]

    def _build_relations(
        self,
        papers: List[PaperContext],
    ) -> tuple:
        """Build paper relations (placeholder for KG integration)."""
        # TODO: Integrate with core/kg/ for real relation building
        return {}, {}

    # ── Query Analysis ──────────────────────────────────────

    def classify_query(self, query: str) -> QueryType:
        """Classify query type."""
        q = query.lower()

        if any(k in q for k in ["对比", "区别", "异同", "哪个更好", "difference", "compare"]):
            return QueryType.COMPARISON
        elif any(k in q for k in ["未解决", "空白", "机会", "还有什么", "gap", "unresolved"]):
            return QueryType.DISCOVERY
        elif any(k in q for k in ["为什么", "原因", "导致", "why", "because", "cause"]):
            return QueryType.INFERENTIAL
        else:
            return QueryType.FACTUAL

    # ── Response Generation ─────────────────────────────────

    def chat(
        self,
        query: str,
        context: Optional[ResearchContext] = None,
    ) -> str:
        """Main chat entry point."""
        ctx = context or self.build_context(query)
        query_type = self.classify_query(query)

        system_prompt = self._build_system_prompt(ctx)
        user_prompt = self._build_user_prompt(query, ctx, query_type)

        response = self._call_llm(system_prompt, user_prompt)

        # Store in history
        self._chat_history.append({"role": "user", "content": query})
        self._chat_history.append({"role": "assistant", "content": response})

        return response

    def _build_system_prompt(self, ctx: ResearchContext) -> str:
        """Build system prompt with context."""
        papers_info = "\n".join([
            f"- {p.title} ({p.year})"
            for p in ctx.papers[:5]
        ]) if ctx.papers else "No relevant papers found"

        insights_info = "\n".join([
            f"- {i.content[:100]}"
            for i in ctx.insights[:3]
        ]) if ctx.insights else "No relevant insights"

        return f"""You are a research assistant. Answer questions based on the user's research library.

Current research focus: {ctx.topic}

Relevant papers from your library:
{papers_info}

User's annotated insights:
{insights_info}

Guidelines:
1. Reference specific papers when making claims
2. Synthesize information from multiple sources when possible
3. Acknowledge gaps in the research library
4. Suggest follow-up questions when appropriate"""

    def _build_user_prompt(
        self,
        query: str,
        ctx: ResearchContext,
        query_type: QueryType,
    ) -> str:
        """Build user prompt based on query type."""
        prompts = {
            QueryType.FACTUAL: f"Answer the following question:\n\n{query}",
            QueryType.INFERENTIAL: f"Analyze and reason about:\n\n{query}",
            QueryType.DISCOVERY: f"Identify research gaps and opportunities in:\n\n{query}",
            QueryType.COMPARISON: f"Compare and contrast:\n\n{query}",
        }
        return prompts.get(query_type, query)

    def _call_llm(
        self,
        system_prompt: str,
        user_prompt: str,
    ) -> str:
        """Call LLM API."""
        from llm.client import call_llm_chat_completions

        messages = [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_prompt},
        ]

        return call_llm_chat_completions(
            messages=messages,
            model="qwen3.5-plus",
            system_prompt=None,
        )

    def get_history(self) -> List[Dict[str, str]]:
        """Get chat history."""
        return self._chat_history.copy()

    def clear_history(self) -> None:
        """Clear chat history."""
        self._chat_history.clear()
