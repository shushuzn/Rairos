"""Semantic Router — natural-language CLI command routing with graceful degradation."""

from __future__ import annotations

import argparse
import json
import logging
import math
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

from llm.client import call_llm_chat_completions

logger = logging.getLogger(__name__)


# ─── QueryType taxonomy ────────────────────────────────────────────────────────


class QueryType(Enum):
    GAP_ANALYSIS = "gap_analysis"
    HYPOTHESIS_GENERATION = "hypothesis_generation"
    EXPERIMENT = "experiment"
    INSIGHT = "insight"
    NARRATIVE = "narrative"
    PAPER_SEARCH = "paper_search"
    QUESTION_ANSWER = "question_answer"
    GENERAL = "general"


# ─── QueryType → CLI subcommand map ─────────────────────────────────────────

_QUERY_TYPE_TO_COMMAND: Dict[QueryType, str] = {
    QueryType.GAP_ANALYSIS: "gap",
    QueryType.HYPOTHESIS_GENERATION: "hypothesize",
    QueryType.EXPERIMENT: "experiment",
    QueryType.INSIGHT: "insight",
    QueryType.NARRATIVE: "narrative",
    QueryType.PAPER_SEARCH: "search",
    QueryType.QUESTION_ANSWER: "ask",
    QueryType.GENERAL: "chat",
}

# CLI subcommand → (module_path, parser_builder_name) lookup for programmatic execution
# NOTE: This dict is intentionally empty. Previous entries (gap, hypothesize, etc.)
# were removed as their modules were ported to Rust. The _dispatch_command function
# handles fallback to chat for any missing handlers.
_SUBCOMMAND_TABLE_LOOKUP: Dict[str, Tuple[str, str]] = {}


# ─── Route dataclass ──────────────────────────────────────────────────────────


@dataclass
class Route:
    query_type: QueryType
    confidence: float  # 0.0–1.0
    primary_command: str  # e.g. "gap"
    reasoning: str = ""
    multi_intent: bool = False
    secondary_query_type: Optional[QueryType] = None
    sub_commands: List[str] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "query_type": self.query_type.value,
            "confidence": self.confidence,
            "primary_command": self.primary_command,
            "reasoning": self.reasoning,
            "multi_intent": self.multi_intent,
            "secondary_query_type": (
                self.secondary_query_type.value if self.secondary_query_type else None
            ),
            "sub_commands": self.sub_commands,
        }


# ─── Tool capability index ────────────────────────────────────────────────────

_INDEX: Optional[Dict[str, Any]] = None

_CAPABILITY_DESCRIPTIONS: Dict[QueryType, str] = {
    QueryType.GAP_ANALYSIS: "Identify research gaps, unanswered questions, or underexplored areas in a topic",
    QueryType.HYPOTHESIS_GENERATION: "Generate testable research hypotheses or conjectures based on gaps",
    QueryType.EXPERIMENT: "Design, run, or track experiments to validate hypotheses",
    QueryType.INSIGHT: "Extract key insights, patterns, or synthesis from research papers",
    QueryType.NARRATIVE: "Track research narrative threads, phases, or story arcs across the research",
    QueryType.PAPER_SEARCH: "Search for papers or research publications by keywords or topic",
    QueryType.QUESTION_ANSWER: "Ask a research question that can be answered from the paper library",
    QueryType.GENERAL: "General research conversation or open-ended discussion",
}


def _load_index() -> Dict[str, Any]:
    global _INDEX
    if _INDEX is None:
        path = Path(__file__).parent.parent / "data" / "semantic_router_index.json"
        if path.exists():
            _INDEX = json.loads(path.read_text(encoding="utf-8"))
        else:
            _INDEX = {}
    return _INDEX


# ─── Keyword scoring ──────────────────────────────────────────────────────────

_QUERY_TYPE_KEYWORDS: Dict[QueryType, List[str]] = {
    QueryType.GAP_ANALYSIS: [
        "gap",
        "gaps",
        "空白",
        "未解决",
        "missing",
        "unresolved",
        "opportunity",
        "差距",
        "limitation",
        "limitations",
        "不足",
        "untouched",
        "overlooked",
        "open problem",
        "open question",
    ],
    QueryType.HYPOTHESIS_GENERATION: [
        "hypothesis",
        "假设",
        "假设生成",
        "conjecture",
        "predict",
        "预测",
        "实验设计",
        "hypothesize",
        "if-then",
    ],
    QueryType.EXPERIMENT: [
        "experiment",
        "实验",
        "ab test",
        "evaluate",
        "评估",
        "validate",
        "验证",
        "trial",
        "跑实验",
        "实验结果",
        "benchmark",
        "benchmarking",
    ],
    QueryType.INSIGHT: [
        "insight",
        "insights",
        "发现",
        "洞察",
        "pattern",
        "patterns",
        "发现",
        "key finding",
        "takeaway",
        "synthesis",
    ],
    QueryType.NARRATIVE: [
        "narrative",
        "story",
        "线程",
        "progress",
        "phase",
        "跟踪",
        "跟踪",
        "进展",
        "状态",
        "story arc",
    ],
    QueryType.PAPER_SEARCH: [
        "paper",
        "papers",
        "search",
        "find",
        "论文",
        "搜索",
        "arxiv",
        "找论文",
        "文献",
        "publication",
    ],
    QueryType.QUESTION_ANSWER: [
        "what",
        "who",
        "how",
        "why",
        "explain",
        "什么",
        "如何",
        "为什么",
        "请问",
        "回答",
        "answer",
        "can you",
    ],
    QueryType.GENERAL: [
        "chat",
        "talk",
        "discuss",
        "对话",
        "聊聊",
        "tell me",
        "about",
        "introduction",
        "介绍",
    ],
}


# ─── LLM-based routing ───────────────────────────────────────────────────────


def _route_by_llm(query: str, model: Optional[str] = None) -> Route:
    """Primary router: classify via LLM."""
    model = model or "qwen3.5-plus"

    capability_lines = "\n".join(
        f"  - {qt.value}: {desc}" for qt, desc in _CAPABILITY_DESCRIPTIONS.items()
    )

    system_prompt = (
        "You are a CLI research-command classifier. "
        "Given a user's natural-language research query, classify it into exactly one type.\n\n"
        f"Available types:\n{capability_lines}\n\n"
        "Return ONLY valid JSON with this exact shape:\n"
        '  {"query_type": "...", "confidence": 0.0-1.0, '
        '"reasoning": "...", "multi_intent": false}\n\n'
        "Rules:\n"
        "  - confidence < 0.5 means you are uncertain\n"
        "  - multi_intent=true only when query clearly contains TWO distinct intents "
        "(e.g. '分析gap并提出假设' = gap_analysis + hypothesis_generation)\n"
        "  - If multi_intent=true, also include: "
        '"secondary_query_type": "..."'
    )

    messages: List[Dict[str, str]] = [
        {"role": "user", "content": query},
    ]

    try:
        raw = call_llm_chat_completions(
            messages=messages,
            model=model,
            system_prompt=system_prompt,
        )
    except (ValueError, Exception):
        # No API key or other LLM error — let route() fall back
        raise

    parsed = json.loads(raw)
    qt = QueryType(parsed["query_type"])

    route = Route(
        query_type=qt,
        confidence=float(parsed["confidence"]),
        primary_command=_QUERY_TYPE_TO_COMMAND[qt],
        reasoning=parsed.get("reasoning", ""),
        multi_intent=parsed.get("multi_intent", False),
    )

    if route.multi_intent:
        secondary_str = parsed.get("secondary_query_type")
        if secondary_str:
            route.secondary_query_type = QueryType(secondary_str)
            route.sub_commands = [
                route.primary_command,
                _QUERY_TYPE_TO_COMMAND[route.secondary_query_type],
            ]
        else:
            route.multi_intent = False

    return route


# ─── Embedding-based fallback ────────────────────────────────────────────────


def cosine_sim(a: List[float], b: List[float]) -> float:
    dot = sum(x * y for x, y in zip(a, b, strict=True))
    norm_a = math.sqrt(sum(x * x for x in a))
    norm_b = math.sqrt(sum(x * x for x in b))
    if norm_a == 0 or norm_b == 0:
        return 0.0
    return dot / (norm_a * norm_b)


def _route_by_embedding(query: str) -> Route:
    """Cosine-similarity fallback: compare query embedding against capability descriptions."""
    try:
        from cli.cmd.dedup_semantic import _get_ollama_embedding_batch

        texts = [query] + [_CAPABILITY_DESCRIPTIONS[qt] for qt in QueryType]
        embeddings = _get_ollama_embedding_batch(texts)
        if not embeddings or embeddings[0] is None:
            raise RuntimeError("Embedding service unavailable")
        query_emb = embeddings[0]
        cap_embs = [e for e in embeddings[1:] if e is not None]

        scores: List[float] = [cosine_sim(query_emb, emb) for emb in cap_embs]

        best_idx = int(max(range(len(scores)), key=lambda i: scores[i]))
        best_qt = list(QueryType)[best_idx]

        return Route(
            query_type=best_qt,
            confidence=float(scores[best_idx]),
            primary_command=_QUERY_TYPE_TO_COMMAND[best_qt],
            reasoning="[embedding fallback]",
        )
    except Exception as e:
        logger.debug("Embedding routing failed: %s", e)
        raise


# ─── Keyword-based fallback ───────────────────────────────────────────────────


def _route_by_keyword(query: str) -> Route:
    """Last-resort: score by keyword overlap."""
    q_lower = query.lower()
    best_score = 0.0
    best_qt = QueryType.GENERAL

    for qt, keywords in _QUERY_TYPE_KEYWORDS.items():
        score = sum(1.0 for kw in keywords if kw in q_lower)
        if score > best_score:
            best_score = score
            best_qt = qt

    return Route(
        query_type=best_qt,
        confidence=min(best_score / 3.0, 1.0),  # 3+ keyword hits → 100%
        primary_command=_QUERY_TYPE_TO_COMMAND[best_qt],
        reasoning=f"[keyword fallback: score={best_score:.1f}]",
    )


# ─── SemanticRouter ───────────────────────────────────────────────────────────


class SemanticRouter:
    """Route a natural-language query to the appropriate CLI subcommand.

    Degradation chain: LLM classification → embedding similarity → keyword heuristic.
    """

    def __init__(self, model: Optional[str] = None):
        self.model = model or "qwen3.5-plus"

    def route(self, query: str) -> Route:
        # Primary: LLM
        try:
            return _route_by_llm(query, self.model)
        except Exception as e:
            logger.debug("LLM routing failed, trying embedding: %s", e)

        # Fallback 1: embedding similarity
        try:
            return _route_by_embedding(query)
        except Exception as e:
            logger.debug("Embedding routing failed, using keyword: %s", e)

        # Fallback 2: keyword heuristic
        return _route_by_keyword(query)

    def execute(self, route: Route, query: str, exec_all: bool = False) -> Dict[str, str]:
        """Execute routed command(s), capture stdout. Returns {command: output}."""
        commands = (
            route.sub_commands if (route.multi_intent and exec_all) else [route.primary_command]
        )
        outputs: Dict[str, str] = {}

        for cmd in commands:
            try:
                output = _run_command_by_name(cmd, query)
                outputs[cmd] = output
            except (Exception, SystemExit) as exc:
                logger.warning("Command %s failed: %s", cmd, exc)
                outputs[cmd] = f"[ERROR in {cmd}: {exc}]"

        return outputs


# ─── Internal command runner ───────────────────────────────────────────────────


def _run_command_by_name(subcmd: str, query: str) -> str:
    """Run a CLI subcommand by name, capturing stdout as a string.

    Bypasses argparse entirely — directly constructs Namespace objects from the
    natural-language query string.  This avoids the fundamental mismatch between
    free-form query text and argparse's token-based positional argument parsing.
    """
    import argparse
    import contextlib
    from io import StringIO

    # ── Build the args namespace directly from the query string ──────────────
    #
    # Each command has a primary positional (topic / query / question) that
    # receives the full query text.  Optional flags are left at their defaults.
    # Commands with required sub-actions (e.g. insight) use sensible defaults.
    #
    # Map: subcmd → primary positional attribute name
    _POSITIONAL_MAP: Dict[str, str] = {
        "gap": "topic",
        "hypothesize": "topic",
        "experiment": "query",
        "insight": "action",  # required; set to "list" (see below)
        "narrative": "question",
        "ask": "query",
        "search": "topic",
        "chat": "topic",
    }

    attr = _POSITIONAL_MAP.get(subcmd, "topic")

    # insight uses a required "action" positional; all other fields use --query
    if subcmd == "insight":
        args = argparse.Namespace(
            subcmd=subcmd,
            action="list",
            query=query or "",
            tags=None,
            markdown=False,
            type="finding",
        )
    elif subcmd == "gap":
        # All optional flags default to False/None so the command uses its own defaults
        args = argparse.Namespace(
            subcmd=subcmd,
            topic=query,
            no_llm=False,
            json=False,
            min_papers=3,
            model=None,
            interactive=False,
            enhanced=False,
            no_insights=False,
            hypothesis=False,
            profile=False,
            history=False,
            stats=False,
            prefs_history=False,
        )
    elif subcmd == "ask":
        args = argparse.Namespace(
            subcmd=subcmd,
            query=query,
            context=None,
            verbose=False,
            no_insights=False,
            max_papers=10,
            route=False,
        )
    else:
        args = argparse.Namespace(subcmd=subcmd, **{attr: query})

    # ── Capture stdout and dispatch ──────────────────────────────────────────
    buf = StringIO()
    with contextlib.redirect_stdout(buf):
        _dispatch_command(subcmd, args)

    return buf.getvalue()


def _dispatch_command(subcmd: str, args: argparse.Namespace) -> None:
    """Dispatch to the appropriate _run_<subcmd> function.

    Uses deferred imports per subcommand to avoid hard failures on
    modules that have been ported to Rust and removed.
    Falls back to generic chat when handler is unavailable.
    """
    import logging
    import importlib

    # Handlers: subcommand → cli attribute name
    _HANDLERS: Dict[str, str] = {
        "narrative": "_run_narrative",
        "chat": "_run_chat",
    }

    handler_name = _HANDLERS.get(subcmd)
    if handler_name is not None:
        mod = importlib.import_module("cli")
        fn = getattr(mod, handler_name, None)
        if fn is not None:
            fn(args)
            return

    # Fallback: log the gap and route to chat
    logging.warning(
        "semantic_router: handler for '%s' not available "
        "(module not yet ported to Rust), falling back to chat",
        subcmd,
    )
    from cli import _run_chat

    args.subcmd = "chat"
    args.topic = getattr(args, "query", None) or getattr(args, "topic", None) or getattr(args, "question", "")
    _run_chat(args)
