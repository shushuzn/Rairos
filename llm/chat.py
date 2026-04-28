"""RAG Chat - Intelligent Q&A with your paper library.

Provides natural language Q&A over your paper corpus with source citation.
"""
from __future__ import annotations

import hashlib
import os
import re
from dataclasses import dataclass, field
from enum import Enum
from functools import lru_cache
from typing import Any, Callable, List, Optional, Tuple

import json
import urllib.request

from llm.client import call_llm_chat_completions, stream_llm_chat_completions
from llm.constants import LLM_BASE_URL, LLM_MODEL
from llm.research_session import get_session_tracker


# LRU cache for embedding lookups (avoids redundant Ollama API calls)
_EMBEDDING_CACHE: dict[str, Optional[List[float]]] = {}
_EMBEDDING_CACHE_MAX = 1000

# Cache for retrieval results (avoids redundant DB queries)
_RETRIEVAL_CACHE: dict[str, Tuple[float, List]] = {}  # key -> (timestamp, contexts)
_RETRIEVAL_CACHE_TTL = 3600  # 1 hour TTL
_RETRIEVAL_CACHE_MAX = 500


def _get_retrieval_cache_key(query: str, concept: Optional[str], limit: int) -> str:
    """Generate cache key for retrieval results."""
    import time
    return hashlib.md5(f"{query.strip()}:{concept}:{limit}".encode()).hexdigest()


def _get_cached_retrieval(cache_key: str) -> Optional[List]:
    """Get cached retrieval results if valid."""
    import time
    if cache_key in _RETRIEVAL_CACHE:
        timestamp, contexts = _RETRIEVAL_CACHE[cache_key]
        if time.time() - timestamp < _RETRIEVAL_CACHE_TTL:
            return contexts
        else:
            del _RETRIEVAL_CACHE[cache_key]
    return None


def _cache_retrieval(cache_key: str, contexts: List) -> None:
    """Cache retrieval results with LRU eviction."""
    import time
    while len(_RETRIEVAL_CACHE) >= _RETRIEVAL_CACHE_MAX:
        oldest_key = min(_RETRIEVAL_CACHE, key=lambda k: _RETRIEVAL_CACHE[k][0])
        del _RETRIEVAL_CACHE[oldest_key]
    _RETRIEVAL_CACHE[cache_key] = (time.time(), contexts)


def _get_ollama_embedding(text: str, model: str = "nomic-embed-text") -> Optional[List[float]]:
    """Fetch embedding from local Ollama with LRU caching. Returns None on failure."""
    # Normalize text: lowercase, strip whitespace for consistent cache keys
    cache_key = hashlib.md5(f"{model}:{text.strip()}".encode()).hexdigest()

    # Check cache first
    if cache_key in _EMBEDDING_CACHE:
        return _EMBEDDING_CACHE[cache_key]

    try:
        req = urllib.request.Request(
            "http://localhost:11434/api/embeddings",
            data=json.dumps({"model": model, "prompt": text}).encode(),
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=60) as resp:
            data = json.loads(resp.read())
            embedding = data.get("embedding")
            # Cache the result (with LRU eviction)
            if len(_EMBEDDING_CACHE) >= _EMBEDDING_CACHE_MAX:
                # Remove oldest entry (first item in dict - insertion order maintained)
                _EMBEDDING_CACHE.pop(next(iter(_EMBEDDING_CACHE)))
            _EMBEDDING_CACHE[cache_key] = embedding
            return embedding
    except Exception:
        _EMBEDDING_CACHE[cache_key] = None  # Cache failures too
        return None


def clear_embedding_cache() -> int:
    """Clear the embedding cache. Returns number of entries cleared."""
    count = len(_EMBEDDING_CACHE)
    _EMBEDDING_CACHE.clear()
    return count


# ------------------------------------------------------------------
# Prompt constants for cross-paper analysis
# ------------------------------------------------------------------
_CROSS_PAPER_SYSTEM_PROMPT = """你是一个研究综述助手，擅长发现论文之间的关联。

分析多篇论文，找出：
1. 共同点 (connection): 讨论相似主题或互补方法
2. 对比 (comparison): 同一问题的不同解决方法
3. 矛盾 (contradiction): 结论或方法冲突
4. 演进 (evolution): 后人如何在前人基础上改进

输出格式（最多3个洞察）：
- 类型: 一句话总结 [论文1] [论文2]
例如：
- comparison: BERT vs GPT的预训练目标不同 [BERT] [GPT-2]
- evolution: LoRA基于Adapter思想提出低秩更新 [Adapter] [LoRA]"""

_CROSS_PAPER_USER_PROMPT_TEMPLATE = """请分析以下论文之间的关联：

{context_text}

找出最重要的关联（最多3个）："""


class QueryType(Enum):
    """Query type classification for adaptive routing."""
    FACTUAL = "factual"      # Who, when, what (exact facts)
    CONCEPTUAL = "conceptual"  # Explain, how, why (understanding)
    COMPARATIVE = "comparative"  # vs, compared, difference (analysis)
    TEMPORAL = "temporal"    # recent, latest, 2024, new (time-sensitive)
    GENERAL = "general"       # Default fallback


# 查询类型 → BM25权重（语义权重 = 1 - BM25权重）
_QUERY_WEIGHTS = {
    QueryType.FACTUAL:     0.65,  # 精确匹配权威
    QueryType.CONCEPTUAL:  0.20,  # 语义理解主导
    QueryType.COMPARATIVE: 0.50,  # 平衡
    QueryType.TEMPORAL:    0.55,  # BM25 + 时效性boost
    QueryType.GENERAL:     0.40,  # 默认
}

# 查询类型 → MMR lambda（0.7=偏重相关度，0.5=平衡，0.3=偏重多样性）
_MMR_LAMBDA = {
    QueryType.FACTUAL:     0.8,   # 事实查询：相关度优先
    QueryType.CONCEPTUAL:  0.6,   # 概念查询：适度多样性
    QueryType.COMPARATIVE: 0.5,   # 比较查询：平衡相关度与多样性
    QueryType.TEMPORAL:    0.7,   # 时序查询：相关度优先
    QueryType.GENERAL:     0.6,   # 默认：适度多样性
}


# ─── Data Structures ──────────────────────────────────────────────────────────────


@dataclass
class Citation:
    """A citation extracted from a paper with source tracing."""
    paper_id: str
    paper_title: str
    authors: List[str]
    published: str
    snippet: str
    relevance_score: float
    section: str = ""           # 论文章节 (abstract, intro, method, etc.)
    char_start: int = 0          # 在原文中的起始位置
    char_end: int = 0            # 在原文中的结束位置
    quote: str = ""              # 精确引用语句


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
class ConfidenceScore:
    """Confidence score for RAG answer quality."""
    score: float              # 0-100 置信度
    papers_count: int         # 引用的论文数
    coverage: str            # 覆盖描述 (e.g., "3篇论文，覆盖Method章节")
    warnings: List[str]      # 低置信度警告
    sources: List[str]      # 主要来源章节

    @property
    def level(self) -> str:
        """Return confidence level label."""
        if self.score >= 80:
            return "高"
        elif self.score >= 50:
            return "中"
        else:
            return "低"


@dataclass
class CrossPaperInsight:
    """Cross-paper synthesis insight."""
    insight_type: str  # "comparison", "connection", "contradiction", "evolution"
    summary: str  # 一句话总结
    papers: List[str]  # 涉及的论文
    detail: str  # 详细说明


@dataclass
class ChatResult:
    """Result of a RAG chat interaction."""
    answer: str
    citations: List[Citation] = field(default_factory=list)
    papers_used: List[str] = field(default_factory=list)
    session_id: Optional[str] = None  # 会话ID for continuity
    resolved_context: Optional[dict] = None  # 解析的上下文信息
    probing_questions: List[str] = field(default_factory=list)  # 智能追问建议
    confidence: Optional[ConfidenceScore] = None  # 答案可信度评分
    cross_paper_insights: List[CrossPaperInsight] = field(default_factory=list)  # 跨论文洞察


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
        self.base_url = base_url or LLM_BASE_URL
        self.model = model or LLM_MODEL
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
                re.compile(r'\b(who proposed|who introduced|who published|who wrote|who created)\b', re.I),
                re.compile(r'\b(where (published|presented|introduced|released))\b', re.I),
                re.compile(r'\b(what organization|what institution|what company)\b', re.I),
                # 中文：事实查询
                re.compile(r'(是谁|谁提出|谁创建|谁发布|哪篇|哪个作者|哪篇论文|何时|何时发表)'),
                re.compile(r'(哪年|哪月|哪里|哪个机构|哪家|哪个团队|谁的工作)'),
            ],
            QueryType.CONCEPTUAL: [
                re.compile(r'\b(what is|what are|explain|describe|how does|how do|why does|why do|understand|definition|meaning)\b', re.I),
                re.compile(r'(原理|机制|概念|解释|是什么|如何|为什么|理解|定义|工作原理)'),
                re.compile(r'(什么意思|含义|理论基础|基本思想|核心思想|本质)'),
                re.compile(r'(怎么做|如何实现|如何工作|是怎样|怎样)'),
            ],
            QueryType.COMPARATIVE: [
                re.compile(r'\b(vs|versus|compared to|compared with)\b', re.I),
                re.compile(r'\b(difference between|differences between)\b', re.I),
                re.compile(r'\b(compare|comparison)\b', re.I),
                re.compile(r'\b(which is better|which is worse|which is stronger)\b', re.I),
                re.compile(r'\b(pros and cons|pros/cons|strengths? and weaknesses?)\b', re.I),
                # 中文：比较查询
                re.compile(r'(和.*比较|比较.*和|对比|区别|差异)'),
                re.compile(r'(哪个更好|哪个更差|哪个更强|孰优孰劣)'),
                re.compile(r'(优于|劣于|胜于|强于|优势|劣势)'),
            ],
            QueryType.TEMPORAL: [
                re.compile(r'\b(recent|latest|newest|recently)\b', re.I),
                re.compile(r'\b(202[0-9]|20[2-9]\d)\b', re.I),
                re.compile(r'\b(最近|最新|新的|202[0-9]|今年|去年|明年)\b'),
                re.compile(r'\b(published in|released in|presented in|from 20)\b', re.I),
                re.compile(r'\b(evolution|development|history|progress)\b', re.I),
                re.compile(r'\b(before|after|since|until|past|future)\b', re.I),
                # 中文：时间查询
                re.compile(r'(最近 最新 新的|今年|去年|明年|近年)'),
                re.compile(r'(何时|什么时候|多会儿|早期|后期)'),
                re.compile(r'(演变|发展|演进|历史|进展|进步)'),
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

    # ─── Context-Aware Multi-Turn Chat ─────────────────────────────────────────

    def _resolve_pronouns(self, question: str, session: Any) -> str:
        """
        Resolve pronouns and references from conversation history.

        Replaces pronouns like '它', '这个', '有哪些变体' with context from history.
        """
        if not session or not session.queries:
            return question

        resolved = question
        last_query = session.queries[-1] if session.queries else None

        if not last_query:
            return resolved

        # Extract key entities from last question
        last_q = last_query.question

        # Check if this is a follow-up question
        is_followup = any(
            pattern.search(question.lower())
            for pattern in [
                re.compile(r'^(它|它们|这个|有哪些|有什么|哪个|哪些|怎么|如何|为什么|有什么不同)'),
                re.compile(r'^(what about|and how|what are the|which ones|how about)'),
            ]
        )

        if is_followup and last_q:
            # Extract the main topic from last question
            # Simple heuristic: take significant words
            topic = self._extract_topic(last_q)
            if topic:
                # Add context to prompt
                resolved = f"[上文讨论: {topic}] {question}"

        return resolved

    def _extract_topic(self, text: str) -> Optional[str]:
        """Extract the main topic/entity from a question."""
        # Remove common question patterns
        patterns = [
            r'是什么|什么是|请问|帮我|找找|解释|说明|介绍',
            r'what is|what are|explain|describe|introduce',
        ]
        cleaned = text
        for p in patterns:
            cleaned = re.sub(p, '', cleaned, flags=re.I)

        # Take first meaningful phrase (3-10 chars)
        words = cleaned.split()
        for w in words:
            if 2 <= len(w) <= 15 and w not in {'的', '了', '是', '在', '和', 'the', 'a', 'an', 'is', 'are'}:
                return w[:20]

        return None

    def _rewrite_followup(self, question: str, history: List[dict]) -> str:
        """
        Rewrite follow-up questions using LLM for better context understanding.

        Args:
            question: Original follow-up question
            history: List of previous Q&A pairs

        Returns:
            Rewritten question with explicit context
        """
        if len(history) < 1 or not question:
            return question

        # Only rewrite if this looks like a follow-up
        followup_patterns = [
            r'^(它|它们|这个|那|这些|那些|有哪些|有什么|哪个|哪些|怎么|如何|为什么|有啥)',
            r'^(what about|and how|what are|which ones|how about|and the|also|but|what if)',
        ]
        is_followup = any(
            re.match(p, question.lower())
            for p in followup_patterns
        )
        if not is_followup:
            return question

        # Build context from history
        context_parts = []
        for i, entry in enumerate(history[-3:], 1):  # Last 3 exchanges
            q = entry.get('question', '')
            a = entry.get('answer', '')
            # Truncate answer to first 200 chars
            a_short = a[:200] + "..." if len(a) > 200 else a
            context_parts.append(f"Q{i}: {q}\nA{i}: {a_short}")

        history_text = "\n\n".join(context_parts)

        prompt = f"""Rewrite the follow-up question as a standalone question that includes all necessary context from the conversation history.

Conversation History:
{history_text}

Follow-up Question: {question}

Rewrite as a standalone question (in the same language as the original question):"""

        try:
            rewritten = call_llm_chat_completions(
                messages=[],
                model=self.model,
                user_prompt=prompt,
                base_url=self.base_url or LLM_BASE_URL,
                api_key=self.api_key,
                system_prompt="You are a helpful assistant that rewrites questions. Output ONLY the rewritten question, nothing else.",
                timeout=30,
            )
            # Clean up response
            rewritten = rewritten.strip()
            # Remove quotes if present
            if rewritten.startswith('"') and rewritten.endswith('"'):
                rewritten = rewritten[1:-1]
            if rewritten.startswith("'") and rewritten.endswith("'"):
                rewritten = rewritten[1:-1]
            return rewritten if rewritten else question
        except Exception:
            # Fallback to simple resolution on error
            return self._resolve_pronouns_simple(question, history)

    def _resolve_pronouns_simple(self, question: str, history: List[dict]) -> str:
        """Simple fallback resolution without LLM."""
        if not history or not question:
            return question

        last = history[-1]
        topic = self._extract_topic(last.get('question', ''))
        if topic:
            return f"[上文讨论: {topic}] {question}"
        return question

    def chat(
        self,
        question: str,
        paper_id: Optional[str] = None,
        concept: Optional[str] = None,
        limit: int = 5,
        verbose: bool = False,
        session_id: Optional[str] = None,
        stream: bool = False,
    ) -> ChatResult:
        """
        Answer a question using the paper corpus.

        Args:
            question: Natural language question
            paper_id: Limit to specific paper
            concept: Filter by concept/tag
            limit: Max papers to retrieve
            verbose: Print debug info
            session_id: Optional session ID for multi-turn conversation
            stream: If True, print answer incrementally (for TUI)

        Returns:
            ChatResult with answer and citations
        """
        # Get session tracker for context-aware retrieval
        session_tracker = get_session_tracker()
        session = session_tracker.get_current_session()

        # Classify query for adaptive routing (used in verbose mode)
        query_type = self.classify_query(question)

        # Rewrite follow-up questions using LLM for better context understanding
        # Get recent history for context
        history = []
        if session and session.queries:
            history = [
                {"question": q.question, "answer": getattr(q, 'answer', '')}
                for q in session.queries[-3:]
            ]
        resolved_question = self._rewrite_followup(question, history) if history else question

        # Retrieve relevant contexts
        contexts = self._retrieve(resolved_question, paper_id, concept, limit)

        if not contexts:
            # Fallback: use general LLM without paper context
            if self.api_key:
                fallback_prompt = f"""用户问题：{question}

请用中文回答这个关于 AI/机器学习/深度学习相关的问题。如果问题与这些领域无关，请直接回答。
回答要简洁、有帮助，标注信息来源（如"根据我的知识"）。"""
                answer = self._call_llm(
                    [],
                    model=self.model,
                    user_prompt=question,
                    base_url=self.base_url,
                    api_key=self.api_key,
                    system_prompt="你是一个有帮助的 AI 助手。用中文简洁回答问题。",
                )
                return ChatResult(
                    answer=answer,
                    citations=[],
                    papers_used=[],
                )
            else:
                return ChatResult(
                    answer="⚠️ 未找到相关论文，且未配置 API Key。\n"
                           "请确保：\n"
                           "1. 论文已添加到数据库\n"
                           "2. 已设置 OPENAI_API_KEY 环境变量",
                    citations=[],
                    papers_used=[],
                )

        if verbose:
            qtype_name = query_type.value
            print(f"[{qtype_name}] Retrieved {len(contexts)} contexts from {len(set(c.paper_id for c in contexts))} papers")

        # 2. Build prompt with resolved context
        prompt = self._build_prompt(resolved_question, contexts)

        # 3. Generate answer (streaming or non-streaming)
        if stream:
            # For streaming, we accumulate into answer variable
            answer = ""
            for delta in stream_llm_chat_completions(
                [],
                model=self.model,
                user_prompt=prompt,
                base_url=self.base_url,
                api_key=self.api_key,
                system_prompt=_RAG_SYSTEM_PROMPT,
            ):
                answer += delta
                # Print incrementally for TUI
                print(delta, end="", flush=True)
            print()  # Newline after streaming
        else:
            answer = self._call_llm(
                [],
                model=self.model,
                user_prompt=prompt,
                base_url=self.base_url,
                api_key=self.api_key,
                system_prompt=_RAG_SYSTEM_PROMPT,
            )

        # 4. Extract citations
        citations = self._extract_citations(contexts)

        # 5. Generate confidence score
        confidence = self._calculate_confidence(answer, contexts)

        # 6. Generate cross-paper insights (if multiple papers)
        cross_paper_insights = []
        unique_papers = list(set(c.paper_id for c in contexts))
        if len(unique_papers) >= 2 and self.api_key:
            cross_paper_insights = self._synthesize_cross_paper_insights(contexts, query_type)

        # 7. Generate probing questions (LLM-driven if available)
        probing_questions = []
        if session:
            session_tracker.add_query(
                question=question,
                answer=answer,
                paper_ids=list(set(c.paper_id for c in contexts)),
                paper_titles=[c.paper_title for c in contexts],
            )
            # Get LLM-driven probing questions
            probing_questions = session_tracker.get_probing_questions(
                use_llm=True,
                api_key=self.api_key,
                base_url=self.base_url,
                model=self.model,
            )

        return ChatResult(
            answer=answer,
            citations=citations,
            papers_used=unique_papers,
            session_id=session.id if session else None,
            resolved_context={"original": question, "resolved": resolved_question, "type": query_type.value},
            probing_questions=probing_questions,
            confidence=confidence,
            cross_paper_insights=cross_paper_insights,
        )

    def _synthesize_cross_paper_insights(
        self,
        contexts: List[ChatContext],
        query_type: QueryType,
    ) -> List[CrossPaperInsight]:
        """
        Synthesize insights from multiple papers.

        Identifies:
        - Connections between papers (same topic, complementary methods)
        - Comparisons (different approaches to the same problem)
        - Contradictions (conflicting findings)
        - Evolution (building on each other)
        """
        if not self.api_key:
            return []

        # Build paper summaries
        paper_summaries = {}
        for ctx in contexts:
            if ctx.paper_id not in paper_summaries:
                paper_summaries[ctx.paper_id] = {
                    "title": ctx.paper_title,
                    "authors": ctx.authors,
                    "year": ctx.published[:4] if ctx.published else "N/A",
                    "snippets": [],
                }
            paper_summaries[ctx.paper_id]["snippets"].append(ctx.snippet[:300])

        # Build LLM prompt for synthesis
        papers_text = []
        for pid, info in paper_summaries.items():
            snippets = "\n".join([f"- {s}" for s in info["snippets"][:2]])
            papers_text.append(f"【{info['title']}】({info['year']})\n{snippets}")

        context_text = "\n\n".join(papers_text)

        user_prompt = _CROSS_PAPER_USER_PROMPT_TEMPLATE.format(
            context_text=context_text,
        )

        try:
            response = self._call_llm(
                [],
                model=self.model,
                user_prompt=user_prompt,
                base_url=self.base_url,
                api_key=self.api_key,
                system_prompt=_CROSS_PAPER_SYSTEM_PROMPT,
            )

            if not response:
                return []

            # Parse response into CrossPaperInsight objects
            insights = []
            for line in response.strip().split('\n'):
                line = line.strip()
                if not line or line.startswith('#'):
                    continue

                # Parse line like "- type: summary [Paper1] [Paper2]"
                match = re.match(r'[-*]\s*(\w+):\s*(.+?)\s*\[([^\]]+)\]\s*\[([^\]]+)\]', line)
                if match:
                    insights.append(CrossPaperInsight(
                        insight_type=match.group(1),
                        summary=match.group(2).strip(),
                        papers=[match.group(3), match.group(4)],
                        detail="",
                    ))

            return insights[:3]  # Max 3 insights

        except Exception:
            # Cross-paper synthesis is best-effort — return empty insights without crashing.
            return []

    def _calculate_confidence(self, answer: str, contexts: List[ChatContext]) -> Optional[ConfidenceScore]:
        """
        Calculate confidence score for the answer based on retrieved contexts.

        Factors:
        - Number of cited papers
        - Relevance scores of papers
        - Section coverage (Abstract < Method < Experiments < All)
        - Answer length vs context coverage
        """
        if not contexts:
            return ConfidenceScore(
                score=0,
                papers_count=0,
                coverage="无相关论文",
                warnings=["未找到相关论文，无法验证回答准确性"],
                sources=[],
            )

        papers_count = len(set(ctx.paper_id for ctx in contexts))
        avg_relevance = sum(ctx.relevance_score for ctx in contexts) / len(contexts)

        # Detect section coverage
        sections = set()
        for ctx in contexts:
            if ctx.snippet:
                # Heuristic: check snippet position in paper
                if 'abstract' in ctx.snippet.lower()[:100]:
                    sections.add('Abstract')
                if 'introduction' in ctx.snippet.lower()[:200]:
                    sections.add('Introduction')
                if any(kw in ctx.snippet.lower()[:100] for kw in ['method', 'approach', 'model', 'architecture']):
                    sections.add('Method')
                if any(kw in ctx.snippet.lower()[:100] for kw in ['experiment', 'result', 'evaluation', 'benchmark']):
                    sections.add('Experiments')

        if not sections:
            sections.add('General')

        # Calculate score
        score = 50.0  # Base score

        # Paper count factor (max +20)
        if papers_count >= 3:
            score += 20
        elif papers_count >= 2:
            score += 15
        elif papers_count == 1:
            score += 10

        # Relevance factor (max +20)
        score += avg_relevance * 20

        # Section coverage factor (max +10)
        if len(sections) >= 3:
            score += 10
        elif len(sections) >= 2:
            score += 7
        else:
            score += 3

        score = min(100, max(0, int(score)))

        # Generate warnings for low confidence
        warnings = []
        if papers_count == 1:
            warnings.append("仅基于单篇论文，建议补充更多证据")
        if avg_relevance < 0.6:
            warnings.append("部分检索结果相关性较低")
        if len(sections) == 1 and 'General' not in sections:
            warnings.append(f"仅覆盖{sections.pop()}章节，缺少其他视角")
        elif len(sections) == 1:
            warnings.append("检索覆盖范围有限")

        coverage = f"{papers_count}篇论文，覆盖{', '.join(list(sections)[:3])}"

        return ConfidenceScore(
            score=score,
            papers_count=papers_count,
            coverage=coverage,
            warnings=warnings,
            sources=list(sections),
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
        # Check cache first (only for general queries, not specific paper)
        if not paper_id:
            cache_key = _get_retrieval_cache_key(query, concept, limit)
            cached = _get_cached_retrieval(cache_key)
            if cached is not None:
                return cached

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

                # Get full paper for content — prefer plain_text, fall back to abstract
                paper = self.db.get_paper(result.paper_id)
                text = (paper.plain_text or paper.abstract or "") if paper else ""
                if text:
                    snippet, section, char_start, char_end = self._extract_snippet(text, query)
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
        result = contexts[:limit]

        # Cache results (only for general queries, not specific paper)
        if not paper_id:
            cache_key = _get_retrieval_cache_key(query, concept, limit)
            _cache_retrieval(cache_key, result)

        return result

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
        # Base BM25 search — include all parse statuses (abstract alone is valuable context)
        results, _ = self.db.search_papers(
            query=query,
            limit=limit,
        )

        # Apply query-type-specific strategies
        if query_type == QueryType.TEMPORAL:
            # Boost newer papers for temporal queries
            results = self._temporal_boost(results)
        elif query_type == QueryType.COMPARATIVE:
            # For comparisons, include papers from different time periods
            results = self._diversity_boost(results)

        # Apply semantic reranking with query-type-adaptive weights
        results = self._semantic_rerank(query, results, query_type)

        # Apply MMR diversity reranking (query-type-specific lambda)
        mmr_lambda = _MMR_LAMBDA.get(query_type, 0.7)
        results = self._mmr_rerank(results, limit, lambda_param=mmr_lambda)

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

    def _semantic_rerank(self, query: str, results: list, query_type: QueryType) -> list:
        """
        Rerank using semantic similarity for ALL candidates.
        Computes query embedding once, then compares against all candidate embeddings.
        Falls back to BM25 if embeddings not available.
        """
        # Get weights for this query type
        bm25_weight = _QUERY_WEIGHTS.get(query_type, 0.4)
        sem_weight = 1.0 - bm25_weight

        if not results:
            return results

        try:
            # Step 1: Get query embedding (compute once)
            query_emb = _get_ollama_embedding(query)
            if query_emb is None:
                return results  # No embedding support, keep BM25 ranking

            # Step 2: Fetch all candidate embeddings in one DB call
            paper_ids = [r.paper_id for r in results]
            embeddings = self.db.get_embeddings_bulk(paper_ids)

            # Step 3: Precompute query norm for cosine similarity
            query_norm = sum(x * x for x in query_emb) ** 0.5
            if query_norm == 0:
                return results

            # Step 4: Compute cosine similarity for ALL candidates
            for r in results:
                bm25_score = abs(r.score) if r.score else 0.5
                paper_emb = embeddings.get(r.paper_id)

                if paper_emb is not None:
                    # Cosine similarity: dot / (norm_a * norm_b)
                    norm = sum(x * x for x in paper_emb) ** 0.5
                    if norm > 0:
                        dot = sum(a * b for a, b in zip(query_emb, paper_emb))
                        sem_score = dot / (query_norm * norm)
                    else:
                        sem_score = 0.5
                else:
                    sem_score = 0.5  # No embedding = neutral

                r.score = bm25_weight * bm25_score + sem_weight * sem_score

        except Exception:
            # Semantic reranking is best-effort — fall back to BM25 ranking without crashing.
            pass

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

    def _mmr_rerank(self, results: list, limit: int, lambda_param: float = 0.7) -> list:
        """
        MMR (Maximum Marginal Relevance) reranking for diversity.

        Balances relevance vs. diversity by penalizing results that are
        too similar to already-selected results.

        Args:
            results: List of ranked results
            limit: Max results to return
            lambda_param: Trade-off (0.7 = relevance, 0.3 = diversity)

        Returns:
            Re-ranked list with diversity
        """
        if not results or len(results) <= 1:
            return results[:limit]

        try:
            # Get embeddings for all candidates
            paper_ids = [r.paper_id for r in results]
            embeddings = self.db.get_embeddings_bulk(paper_ids)

            # Filter to only papers with embeddings
            valid_indices = [i for i, r in enumerate(results) if embeddings.get(r.paper_id)]
            if len(valid_indices) < 2:
                return results[:limit]

            # Precompute norms
            emb_norms = {}
            for pid, emb in embeddings.items():
                if emb:
                    n = sum(x * x for x in emb) ** 0.5
                    emb_norms[pid] = n if n > 0 else 1.0

            selected = []
            remaining = list(valid_indices)

            # Greedy selection: pick best MMR score at each step
            while remaining and len(selected) < limit:
                best_idx = None
                best_mmr = -float('inf')

                for idx in remaining:
                    r = results[idx]
                    paper_emb = embeddings.get(r.paper_id)
                    if paper_emb is None:
                        continue

                    # Relevance score
                    relevance = abs(r.score) if r.score else 0.5

                    # Max similarity to already selected (diversity penalty)
                    max_sim = 0.0
                    if selected and emb_norms.get(r.paper_id):
                        norm_r = emb_norms[r.paper_id]
                        for sel_idx in selected:
                            sel_r = results[sel_idx]
                            sel_emb = embeddings.get(sel_r.paper_id)
                            if sel_emb and emb_norms.get(sel_r.paper_id):
                                norm_s = emb_norms[sel_r.paper_id]
                                dot = sum(a * b for a, b in zip(paper_emb, sel_emb))
                                sim = dot / (norm_r * norm_s)
                                max_sim = max(max_sim, sim)

                    # MMR = lambda * relevance - (1 - lambda) * max_sim
                    mmr = lambda_param * relevance - (1 - lambda_param) * max_sim

                    if mmr > best_mmr:
                        best_mmr = mmr
                        best_idx = idx

                if best_idx is not None:
                    selected.append(best_idx)
                    remaining.remove(best_idx)

            # Return selected results in original order
            return [results[i] for i in selected] if selected else results[:limit]

        except Exception:
            # Fallback to original order on error
            return results[:limit]

    def _extract_snippet(self, text: str, query: str, context_chars: int = 500) -> Tuple[str, str, int, int]:
        """
        Extract a relevant snippet from text around query keywords with source tracing.

        Args:
            text: Full text to search
            query: Query string
            context_chars: Characters of context around match

        Returns:
            Tuple of (snippet, section, char_start, char_end)
        """
        if not text or not query:
            return text[:context_chars] if text else "", self._detect_section(text, 0), 0, min(context_chars, len(text) if text else 0)

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
            end = min(context_chars, len(text) if text else 0)
            return text[:end], self._detect_section(text, 0), 0, end

        # Extract context around match
        start = max(0, best_pos - context_chars // 2)
        end = min(len(text), best_pos + context_chars // 2)

        snippet = text[start:end].strip()
        section = self._detect_section(text, start)

        # Add ellipsis if truncated
        prefix = "..." if start > 0 else ""
        suffix = "..." if end < len(text) else ""

        return f"{prefix}{snippet}{suffix}", section, start, end

    def _detect_section(self, text: str, pos: int) -> str:
        """
        Detect which section of the paper the position falls in.

        Returns section name like 'abstract', 'introduction', 'method', etc.
        """
        if not text or pos < 0:
            return ""

        # Section headers patterns (case-insensitive)
        sections = [
            (r'\babstract\b', 'Abstract'),
            (r'\bintroduction\b', 'Introduction'),
            (r'\brelated work\b', 'Related Work'),
            (r'\bbackground\b', 'Background'),
            (r'\bpreliminaries\b', 'Preliminaries'),
            (r'\bmethod\b', 'Method'),
            (r'\bmethodology\b', 'Methodology'),
            (r'\bmodel\b', 'Model'),
            (r'\bexperiments?\b', 'Experiments'),
            (r'\bresults?\b', 'Results'),
            (r'\bevaluation\b', 'Evaluation'),
            (r'\bdiscussion\b', 'Discussion'),
            (r'\bconclusion\b', 'Conclusion'),
            (r'\breferences?\b', 'References'),
        ]

        text_lower = text[:pos].lower()

        for pattern, name in sections:
            if re.search(pattern, text_lower, re.I):
                return name

        return ""

    def _compress_snippet(self, text: str, max_chars: int = 400) -> str:
        """
        Compress snippet to reduce token count while preserving key information.

        Strategy:
        - Remove redundant whitespace and line breaks
        - Truncate at sentence boundaries when possible
        - Preserve the first sentence (usually contains the key claim)
        """
        if not text:
            return ""

        # Strip and normalize whitespace
        text = re.sub(r'\s+', ' ', text).strip()

        # If already short enough, return as-is
        if len(text) <= max_chars:
            return text

        # Try to truncate at sentence boundary
        sentences = re.split(r'(?<=[。！？.!?])', text)
        result = ""
        for sent in sentences:
            if len(result) + len(sent) <= max_chars:
                result += sent
            else:
                break

        # If no sentences fit, truncate with ellipsis
        if not result:
            result = text[:max_chars - 3] + "..."

        return result

    def _build_prompt(self, question: str, contexts: List[ChatContext]) -> str:
        """Build RAG prompt with retrieved contexts."""
        # Group contexts by paper
        paper_contexts: dict[str, List[ChatContext]] = {}
        for ctx in contexts:
            if ctx.paper_id not in paper_contexts:
                paper_contexts[ctx.paper_id] = []
            paper_contexts[ctx.paper_id].append(ctx)

        # Build context text with compression
        context_parts = []
        for paper_id, ctxs in paper_contexts.items():
            ctx = ctxs[0]  # Use most relevant snippet
            # Compress snippets to reduce token count
            compressed = self._compress_snippet(ctx.snippet, max_chars=400)
            snippets = "\n\n".join([f"> {self._compress_snippet(c.snippet, max_chars=300)}" for c in ctxs[:2]])
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
        """
        Extract enhanced citations from retrieved contexts with source tracing.

        Each citation includes:
        - Paper metadata (title, authors, year)
        - Precise location (section, character positions)
        - Exact quote from the original text
        """
        citations = []
        seen = set()

        for ctx in contexts:
            if ctx.paper_id in seen:
                continue
            seen.add(ctx.paper_id)

            # Extract a precise quote (first sentence or key phrase)
            quote = self._extract_quote(ctx.snippet)

            citations.append(Citation(
                paper_id=ctx.paper_id,
                paper_title=ctx.paper_title,
                authors=ctx.authors,
                published=ctx.published,
                snippet=ctx.snippet[:300] + "..." if len(ctx.snippet) > 300 else ctx.snippet,
                relevance_score=ctx.relevance_score,
                section="",  # Section detection done in _extract_snippet
                char_start=0,
                char_end=0,
                quote=quote,
            ))

        return citations

    def _extract_quote(self, snippet: str) -> str:
        """
        Extract a precise quote from snippet (first sentence or definition).

        Returns a short, impactful quote that can be used for citation.
        """
        if not snippet:
            return ""

        # Clean up the snippet
        clean = snippet.strip().replace("\n", " ").replace("  ", " ")

        # Try to find first sentence ending with . ! or ?
        import re as regex_module
        sentence_end = regex_module.search(r'[.!?]\s', clean)

        if sentence_end:
            quote = clean[:sentence_end.end()].strip()
        else:
            # Fall back to first 150 chars
            quote = clean[:150].strip()

        # Clean up quote markers
        quote = quote.strip('"...»""')
        if len(quote) > 150:
            quote = quote[:147] + "..."

        return quote if quote else ""

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
            boosted_scores = {b["paper_id"]: b["score"] for b in boosted}
            # Return original results re-sorted by boosted score
            return sorted(results, key=lambda r: boosted_scores.get(r.paper_id, 0), reverse=True)
        except Exception:
            # Adaptive boost is best-effort — fall back to original order without crashing.
            return results

    def format_result(self, result: ChatResult, show_citations: bool = True, show_probing: bool = True, show_confidence: bool = True, show_insights: bool = True) -> str:
        """Format ChatResult for terminal output with all enhancements."""
        output = []
        output.append("─" * 60)
        output.append(result.answer)
        output.append("─" * 60)

        # Show cross-paper insights
        if show_insights and result.cross_paper_insights:
            output.append("\n🔗 跨论文洞察：")
            type_emoji = {
                "comparison": "⚖️",
                "connection": "🔗",
                "contradiction": "⚡",
                "evolution": "📈",
            }
            for insight in result.cross_paper_insights:
                emoji = type_emoji.get(insight.insight_type, "💡")
                output.append(f"   {emoji} {insight.summary}")
                output.append(f"       [{insight.papers[0]}] vs [{insight.papers[1]}]")

        # Show confidence score if available
        if show_confidence and result.confidence:
            conf = result.confidence
            level_emoji = {"高": "🟢", "中": "🟡", "低": "🔴"}.get(conf.level, "⚪")
            output.append(f"\n{level_emoji} 置信度: {conf.score}% ({conf.level})")
            output.append(f"   {conf.coverage}")
            if conf.warnings:
                for w in conf.warnings[:2]:
                    output.append(f"   ⚠️ {w}")

        # Show probing questions if available
        if show_probing and result.probing_questions:
            output.append("\n💭 深入探索：")
            for i, q in enumerate(result.probing_questions, 1):
                output.append(f"   {i}. {q}")

        if show_citations and result.citations:
            output.append("\n📖 引用来源：")
            for i, cite in enumerate(result.citations, 1):
                authors = ', '.join(cite.authors[:3]) if cite.authors else "Unknown"
                year = cite.published[:4] if cite.published else "N/A"

                output.append(f"\n[{i}] {cite.paper_title}")
                output.append(f"    {authors} ({year})")

                if cite.section:
                    output.append(f"    📑 来源章节: {cite.section}")

                output.append(f"    相关度: {cite.relevance_score:.2f}")

                if cite.quote:
                    output.append(f"    💬 \"{cite.quote}\"")

                output.append(f"    🔗 arXiv: {cite.paper_id}")

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
