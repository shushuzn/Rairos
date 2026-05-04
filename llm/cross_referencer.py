"""
Cross-paper contradiction/synergy detection.

Detects relationships between a paper and existing papers in the database:
- contradiction: claims that conflict with existing evidence
- alignment: claims that reinforce or extend existing work
- extension: fills a gap or introduces a new dimension
"""

from __future__ import annotations

import logging
import re
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional

logger = logging.getLogger(__name__)


@dataclass
class CrossReferenceItem:
    """A single cross-reference relationship."""

    relation: str  # "contradiction", "alignment", "extension", "unrelated"
    target_paper_id: str
    target_title: str
    description: str
    confidence: float = 0.5  # 0-1
    evidence: str = ""


@dataclass
class CrossReferenceResult:
    """Result of cross-referencing a paper against the database."""

    paper_id: str
    related_papers_found: int = 0
    items: List[CrossReferenceItem] = field(default_factory=list)
    used_fallback: bool = False
    error: str = ""


# ── LLM prompts ──────────────────────────────────────────────────────────────

_CROSS_REF_SYSTEM_PROMPT = """你是一个严谨的 AI 研究助理，擅长检测论文之间的矛盾、协同和扩展关系。

任务：将目标论文与已存在的论文进行对比分析。

硬规则：
1. 只分析你确实能判断的关系，不确定的说 "不确定"
2. 每个判断需要引用原文支撑
3. 禁止捏造
4. 输出中文"""

_CROSS_REF_USER_TEMPLATE = """## 目标论文
标题：{target_title}
标签：{target_tags}
摘要：{target_abstract}

## 对比论文
{reference_papers}

请逐篇分析以上对比论文与目标论文的关系。
对每篇论文输出以下格式：

```
[论文ID] (relation)
描述：...
证据：...
置信度：高/中/低
```

relation 必须是以下之一：
- contradiction: 目标论文的claims与对比论文矛盾
- alignment: 目标论文支持/延伸对比论文的结论
- extension: 目标论文填补了对比论文的空白或在新维度上拓展
- unrelated: 两者无关

最后输出 JSON 总结：
```json
{{"total_related": N, "contradictions": N, "alignments": N, "extensions": N, "unrelated": N}}
```"""


# ── CrossReferencer ──────────────────────────────────────────────────────────


class CrossReferencer:
    """Detect contradiction/alignment/extension relationships across papers."""

    def __init__(self, db=None, llm_config: Optional[Dict[str, Any]] = None):
        self.db = db
        self.llm_config = llm_config or {}

    def analyze(
        self,
        paper_id: str,
        title: str,
        abstract: str,
        body_text: str,
        tags: Optional[List[str]] = None,
        use_llm: bool = True,
    ) -> CrossReferenceResult:
        """Cross-reference a paper against the database.

        Finds papers with overlapping tags, then analyzes their relationship.

        Args:
            paper_id: Target paper ID.
            title: Target paper title.
            abstract: Target paper abstract.
            body_text: Target paper body text.
            tags: Target paper tags.
            use_llm: Whether to use LLM for semantic analysis.

        Returns:
            CrossReferenceResult with relationship items.
        """
        if not self.db:
            return CrossReferenceResult(
                paper_id=paper_id,
                error="No database available for cross-referencing",
                used_fallback=True,
            )

        # Find candidate papers (same tags)
        candidates = self._find_candidates(paper_id, tags or [])
        if not candidates:
            return CrossReferenceResult(
                paper_id=paper_id,
                related_papers_found=0,
                used_fallback=False,
            )

        if use_llm and self.llm_config.get("api_key"):
            return self._analyze_with_llm(
                paper_id,
                title,
                abstract,
                body_text,
                tags or [],
                candidates,
            )
        return self._analyze_fallback(paper_id, candidates)

    # ── Candidate retrieval ──────────────────────────────────────────────

    def _find_candidates(
        self,
        paper_id: str,
        tags: List[str],
        max_candidates: int = 10,
    ) -> List[Dict[str, Any]]:
        """Find existing papers with overlapping tags."""
        if not self.db or not tags:
            return []

        candidates: List[Dict[str, Any]] = []
        seen_ids: set = set()

        for tag in tags:
            try:
                records = self.db.get_papers_by_tag(tag, limit=5)
            except Exception:
                continue

            for rec in records:
                pid = getattr(rec, "id", "") or getattr(rec, "paper_id", "")
                if pid and pid != paper_id and pid not in seen_ids:
                    seen_ids.add(pid)
                    candidates.append(
                        {
                            "id": pid,
                            "title": getattr(rec, "title", ""),
                            "abstract": getattr(rec, "abstract", ""),
                            "tags": getattr(rec, "tags", []),
                        }
                    )

            if len(candidates) >= max_candidates:
                break

        return candidates[:max_candidates]

    # ── LLM path ─────────────────────────────────────────────────────────

    def _analyze_with_llm(
        self,
        paper_id: str,
        title: str,
        abstract: str,
        body_text: str,
        tags: List[str],
        candidates: List[Dict[str, Any]],
    ) -> CrossReferenceResult:
        from llm.client import call_llm_chat_completions

        cfg = self.llm_config

        # Build reference papers section
        ref_lines = []
        for i, c in enumerate(candidates, 1):
            ref_lines.append(
                f"---\n论文 {i}:\nID: {c['id']}\n"
                f"标题：{c['title']}\n"
                f"摘要：{c.get('abstract', '(无)')}\n"
            )
        ref_text = "\n".join(ref_lines)

        prompt = _CROSS_REF_USER_TEMPLATE.format(
            target_title=title,
            target_tags=", ".join(tags),
            target_abstract=abstract or "(无)",
            reference_papers=ref_text,
        )

        raw = call_llm_chat_completions(
            messages=[],
            model=cfg.get("model", "gpt-4o-mini"),
            base_url=cfg.get("base_url", "https://api.openai.com/v1"),
            api_key=cfg["api_key"],
            system_prompt=_CROSS_REF_SYSTEM_PROMPT,
            user_prompt=prompt,
            timeout=cfg.get("timeout", 180),
        )

        items = self._parse_response(raw, candidates)
        return CrossReferenceResult(
            paper_id=paper_id,
            related_papers_found=len(candidates),
            items=items,
            used_fallback=False,
        )

    # ── No-LLM fallback ──────────────────────────────────────────────────

    def _analyze_fallback(
        self,
        paper_id: str,
        candidates: List[Dict[str, Any]],
    ) -> CrossReferenceResult:
        """Without LLM: keyword overlap → alignment, citation presence → extension."""
        items: List[CrossReferenceItem] = []

        for c in candidates[:5]:  # Limit to 5 for fallback
            items.append(
                CrossReferenceItem(
                    relation="alignment",
                    target_paper_id=c["id"],
                    target_title=c["title"],
                    description="Same tag overlap — suggest manual review for relationship",
                    confidence=0.3,
                )
            )

        return CrossReferenceResult(
            paper_id=paper_id,
            related_papers_found=len(candidates),
            items=items,
            used_fallback=True,
        )

    # ── Response parsing ─────────────────────────────────────────────────

    def _parse_response(
        self,
        raw: str,
        candidates: List[Dict[str, Any]],
    ) -> List[CrossReferenceItem]:
        """Parse LLM response into cross-reference items."""
        items: List[CrossReferenceItem] = []

        # Try to extract per-paper blocks
        # Pattern: [paper_id] (relation) or paper_id (relation)
        pattern = r"\[?(\S+?)\]?\s*\((\w+)\)"
        for m in re.finditer(pattern, raw):
            pid = m.group(1).strip("[]")
            relation = m.group(2).lower()

            if relation not in ("contradiction", "alignment", "extension", "unrelated"):
                continue

            # Find the corresponding candidate
            title = pid
            for c in candidates:
                if c["id"] == pid:
                    title = c["title"]
                    break

            items.append(
                CrossReferenceItem(
                    relation=relation,
                    target_paper_id=pid,
                    target_title=title,
                    description="(parsed from LLM analysis)",
                    confidence=0.5,
                )
            )

        # If no structured items found, create generic ones
        if not items:
            for c in candidates[:3]:
                items.append(
                    CrossReferenceItem(
                        relation="alignment",
                        target_paper_id=c["id"],
                        target_title=c["title"],
                        description="Related by shared tags",
                        confidence=0.3,
                    )
                )

        return items
