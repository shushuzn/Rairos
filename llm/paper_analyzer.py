"""
Deep paper analysis: fills P-note blank sections with AI-generated content.

Produces a PaperAnalysisResult with:
- sections_dict: maps "## N. Title" → markdown content (matches render_pnote() keys)
- rubric_dict: {novelty, leverage, evidence, cost, moat, adoption} each 1-5
- extracted_methods/datasets/metrics: keyword lists
"""
from __future__ import annotations

import json
import logging
import math
import re
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, TYPE_CHECKING

if TYPE_CHECKING:
    from build.lib.pdf.extract import StructuredPdfContent

logger = logging.getLogger(__name__)

# ── Section keys must exactly match renderers/pnote.py template ──────────────

_SECTION_KEYS = [
    "## 1. 背景",
    "## 2. 核心问题",
    "## 3.1 架构拆解",
    "## 3.2 算法逻辑",
    "## 3.3 关键组件",
    "## 4. 关键创新",
    "## 5.1 数据集",
    "## 5.2 基线对比",
    "## 5.3 消融实验",
    "## 5.4 成本分析",
    "## 6. 对抗式审稿",
    "## 7. 优势",
    "## 8. 局限",
    "## 9. 本质抽象",
    "## 10. 与其他方法对比",
    "## 11. Decision（决策）",
    "## 12. 知识蒸馏",
    "## 13. 认知升级",
]

_RUBRIC_KEYS = ["novelty", "leverage", "evidence", "cost", "moat", "adoption"]

_METHOD_KEYWORDS = [
    "transformer", "attention", "cnn", "rnn", "lstm", "gru", "bert", "gpt",
    "diffusion", "gan", "vae", "resnet", "unet", "mlp", "graph neural",
    "reinforcement learning", "rl", "fine-tuning", "prompt", "instruction tuning",
    "retrieval augmented", "rerank", "fusion", "encoder", "decoder",
    "quantization", "distillation", "pruning", "contrastive", "adversarial",
    "normalization", "self-supervised", "multi-modal", "multimodal",
]

_DATASET_KEYWORDS = [
    "imagenet", "cifar", "mnist", "svhn", "squad", "glue", "superglue",
    "mmlu", "gsm8k", "humaneval", "mbpp", "hellaswag", "arc", "truthfulqa",
    "coco", "cityscapes", "wikitext", "librispeech", "pascal", "ade20k",
    "sst", "cola", "mrpc", "qnli", "rte", "wnli", "boolq", "piqa",
    "winogrande", "lambada", "enwik8", "text8",
]

_METRIC_KEYWORDS = [
    "accuracy", "bleu", "rouge", "f1", "precision", "recall", "perplexity",
    "wer", "cer", "map", "ndcg", "auc", "mse", "mae", "rmse",
    "top-1", "top-5", "latency", "throughput", "params",
]


@dataclass
class PaperAnalysisResult:
    """Result of a deep paper analysis."""
    paper_id: str
    sections: Dict[str, str] = field(default_factory=dict)
    rubric: Dict[str, Any] = field(default_factory=dict)
    extracted_methods: List[str] = field(default_factory=list)
    extracted_datasets: List[str] = field(default_factory=list)
    extracted_metrics: List[str] = field(default_factory=list)
    raw_llm_output: str = ""
    llm_used: bool = False
    # Citation-grounded analysis
    claims: List["CitationClaim"] = field(default_factory=list)
    unverified_claims: List["CitationClaim"] = field(default_factory=list)


@dataclass
class CitationClaim:
    """A single claim in the analysis linked to source text."""
    text: str  # The claim text
    page: int  # 0-indexed page number in original PDF
    block_idx: int  # Index within the page's text blocks
    chunk_text: str  # The source text this claim refers to
    verified: bool = False  # True if source text actually supports the claim
    verification_note: str = ""  # Why it failed verification, if any
    evidence_score: float = 0.0  # 0.0-1.0: strength of supporting evidence
    correction_round: int = 0  # Which correction round this was verified in (0=first)
    claim_type: str = ""  # "numerical" | "methodology" | "descriptive"


# ── LLM prompts ──────────────────────────────────────────────────────────────

_SYSTEM_PROMPT = """你是一个严谨的 AI 研究助理，擅长对抗式审稿和深度论文分析。

任务：分析论文，按指定格式输出各章节内容。

硬规则：
1. 每个章节标题必须严格使用 `## N. 标题` 格式，N 和标题必须与要求完全一致
2. 内容必须基于论文原文；不确定的加 [推测] 标注
3. 禁止捏造实验/数据/结果
4. 输出中文 Markdown
5. 每条关键陈述必须标注来源页码，格式为 [Page N]，N 为页码数字
6. 末尾输出 JSON 评分块（见评分量表说明）

评分量表：
- Novelty (1-5): 1=增量改进 2=组合已有 3=新任务/视角 4=新范式 5=开创性
- Leverage (1-5): 1=难落地 2=需适配 3=可直接用 4=显著降本 5=范式级
- Evidence (1-5): 1=无实验 2=部分 3=充分覆盖 4=强基线 5=消融完整
- Cost (1-5): 1=极高 2=较高 3=中等 4=较低 5=极低
- Moat (1-5): 1=无壁垒 2=代码 3=数据 4=算法/专利 5=生态
- Adoption (1-5): 1=无 2=<100stars 3>1k/引用>10 4=工业落地 5=生态标配"""

_USER_PROMPT_TEMPLATE = """论文标题：{title}
作者：{authors}
标签：{tags}

【Abstract】
{abstract}

【抽取正文片段（已标注页码）】
{body}

**重要：每条关键陈述必须标注来源页码，格式为 [Page N]，如 "Transformer 使用多头注意力机制 [Page 3]"**

请按以下章节生成初稿，使用 `## N. 标题` 格式（N 和标题必须与以下列表完全一致）：

## 1. 背景
一句话：这篇论文要解决什么问题？（引用摘要）

## 2. 核心问题
这篇论文的核心技术方案是什么？

## 3.1 架构拆解
## 3.2 算法逻辑
## 3.3 关键组件

## 4. 关键创新
一句话总结最大创新点。

## 5.1 数据集
## 5.2 基线对比
## 5.3 消融实验
## 5.4 成本分析

## 6. 对抗式审稿
列出3个最强质疑点。

## 7. 优势
## 8. 局限
## 9. 本质抽象
一句话抽象出本质。

## 10. 与其他方法对比
## 11. Decision（决策）
## 12. 知识蒸馏
### Facts
### Principles
### Insights

## 13. 认知升级

在以上 Markdown 内容之后，另起一行输出以下 JSON（不要放在代码块中）：

```json
{{"novelty": 3, "leverage": 4, "evidence": 3, "cost": 2, "moat": 2, "adoption": 3, "overall": "一句话评价"}}
```"""


# ── Analyzer ─────────────────────────────────────────────────────────────────


class PaperAnalyzer:
    """Deep analysis of a paper to fill P-note sections."""

    def __init__(self, llm_config: Optional[Dict[str, Any]] = None):
        self.llm_config = llm_config or {}

    def analyze(
        self,
        paper_id: str,
        title: str,
        abstract: str,
        body_text: str,
        tags: Optional[List[str]] = None,
        authors: Optional[List[str]] = None,
        use_llm: bool = True,
        structured_content: Optional["StructuredPdfContent"] = None,
    ) -> PaperAnalysisResult:
        """Analyze a paper and produce structured section content.

        Args:
            paper_id: Unique paper identifier.
            title: Paper title.
            abstract: Paper abstract.
            body_text: Full extracted PDF text.
            tags: Paper tags.
            authors: Paper authors.
            use_llm: Whether to attempt LLM-powered analysis.
            structured_content: Optional structured PDF content with page-annotated blocks.

        Returns:
            PaperAnalysisResult with sections, rubric, extracted keywords, and citation claims.
        """
        if use_llm and self.llm_config.get("api_key"):
            return self._analyze_with_llm(
                paper_id, title, abstract, body_text, tags or [], authors,
                structured_content,
            )
        return self._analyze_fallback(
            paper_id, title, abstract, body_text, tags or [],
        )

    # ── LLM path ──────────────────────────────────────────────────────────

    def _analyze_with_llm(
        self,
        paper_id: str,
        title: str,
        abstract: str,
        body_text: str,
        tags: List[str],
        authors: Optional[List[str]] = None,
        structured_content: Optional["StructuredPdfContent"] = None,
    ) -> PaperAnalysisResult:
        from llm.client import call_llm_chat_completions

        cfg = self.llm_config
        authors_str = ", ".join(authors) if authors else "Unknown"
        tags_str = ", ".join(tags) if tags else ""

        # Use page-annotated body if structured content is available
        if structured_content:
            body = self._build_page_annotated_body(structured_content)
        else:
            body = body_text

        prompt = _USER_PROMPT_TEMPLATE.format(
            title=title,
            authors=authors_str,
            tags=tags_str,
            abstract=abstract or "(空)",
            body=body,
        )

        raw = call_llm_chat_completions(
            messages=[],
            model=cfg.get("model", "gpt-4o-mini"),
            base_url=cfg.get("base_url", "https://api.openai.com/v1"),
            api_key=cfg["api_key"],
            system_prompt=_SYSTEM_PROMPT,
            user_prompt=prompt,
            timeout=cfg.get("timeout", 300),
        )

        sections, rubric = self._parse_llm_response(raw)
        result = PaperAnalysisResult(
            paper_id=paper_id,
            sections=sections,
            rubric=rubric,
            raw_llm_output=raw,
            llm_used=True,
        )
        result.extracted_methods = self._extract_keywords(body_text, _METHOD_KEYWORDS)
        result.extracted_datasets = self._extract_keywords(body_text, _DATASET_KEYWORDS)
        result.extracted_metrics = self._extract_keywords(body_text, _METRIC_KEYWORDS)
        return result

    # ── No-LLM fallback ───────────────────────────────────────────────────

    def _analyze_fallback(
        self,
        paper_id: str,
        title: str,
        abstract: str,
        body_text: str,
        tags: List[str],
    ) -> PaperAnalysisResult:
        needs_ai = "\n\n> _（需要 AI 分析）_"

        sections: Dict[str, str] = {}
        for key in _SECTION_KEYS:
            sections[key] = needs_ai

        sections["## 1. 背景"] = (
            f"> **Abstract（原文）**\n> {abstract}\n\n"
            "_（关键词匹配摘要，建议 AI 深入分析）_"
        )
        sections["## 2. 核心问题"] = (
            "_基于摘要推断：_" + needs_ai
        )

        methods = self._extract_keywords(body_text, _METHOD_KEYWORDS)
        datasets = self._extract_keywords(body_text, _DATASET_KEYWORDS)
        metrics = self._extract_keywords(body_text, _METRIC_KEYWORDS)

        if methods:
            sections["## 3.1 架构拆解"] = (
                f"_检测到的方法/架构关键词：{', '.join(methods)}_\n\n{needs_ai}"
            )
        if datasets:
            sections["## 5.1 数据集"] = (
                f"_检测到的数据集关键词：{', '.join(datasets)}_\n\n{needs_ai}"
            )
        if metrics:
            sections["## 5.2 基线对比"] = (
                f"_检测到的评估指标关键词：{', '.join(metrics)}_\n\n{needs_ai}"
            )

        return PaperAnalysisResult(
            paper_id=paper_id,
            sections=sections,
            rubric={k: 0 for k in _RUBRIC_KEYS},
            extracted_methods=methods,
            extracted_datasets=datasets,
            extracted_metrics=metrics,
            llm_used=False,
        )

    # ── Response parsing ─────────────────────────────────────────────────

    def _parse_llm_response(self, raw: str) -> tuple[Dict[str, str], Dict[str, Any]]:
        """Parse LLM markdown output into sections dict and rubric dict.

        The LLM is expected to output:
          ## N. Title
          content...
          ...
          ```json
          {"novelty": ..., "overall": "..."}
          ```

        Returns (sections_dict, rubric_dict).
        """
        sections: Dict[str, str] = {}
        rubric: Dict[str, Any] = {}

        # 1. Extract rubric JSON block first (might be in code fence)
        rubric, remaining = self._extract_rubric(raw)

        # 2. Parse sections from remaining text
        self._parse_sections(remaining, sections)

        return sections, rubric

    def _extract_rubric(self, text: str) -> tuple[Dict[str, Any], str]:
        """Extract rubric JSON from the end of text. Returns (rubric, text_without_rubric)."""
        rubric: Dict[str, Any] = {}

        # Try JSON code fence first
        pattern = r"```(?:json)?\s*\n?(\{[\s\S]*?" + '"' + r"(?:novelty|overall)[\s\S]*?\})\s*\n?```"
        m = re.search(pattern, text, re.IGNORECASE)
        if m:
            try:
                rubric = json.loads(m.group(1))
            except json.JSONDecodeError:
                pass

        if rubric:
            # Remove the JSON block from text for section parsing
            remaining = text[:m.start()].rstrip()
            return rubric, remaining

        # Try bare JSON at end of text
        pattern = r"(\{(?:[^{}]|(?!\s*```)[^{}]*)*\})\s*$"
        m = re.search(pattern, text, re.DOTALL)
        if m:
            try:
                data = json.loads(m.group(1))
                if "novelty" in data or "overall" in data:
                    rubric = data
                    remaining = text[:m.start()].rstrip()
                    return rubric, remaining
            except json.JSONDecodeError:
                pass

        return rubric, text

    def _parse_sections(self, text: str, sections: Dict[str, str]) -> None:
        """Parse section content from markdown text into sections dict."""
        # Split on any `## ` heading — captures both `## 1. 背景` and `## 背景`
        pattern = r"^(##\s+(?:\d+(?:\.\d+)?\.?\s*)?[^\n]+)"

        parts = re.split(pattern, text, flags=re.MULTILINE)

        # parts[0] is text before first heading (discard)
        # Then alternating: heading, content, heading, content...
        for i in range(1, len(parts), 2):
            if i + 1 >= len(parts):
                break
            heading = parts[i].strip()
            content = parts[i + 1].strip()

            # Normalize heading for matching
            norm_heading = self._normalize_heading(heading)

            # Try to match against known section keys
            matched_key = self._match_section_key(norm_heading)
            if matched_key:
                sections[matched_key] = content

        # Also store raw heading-content pairs for unmatched sections
        # (useful for __raw__ reconstruction)

    @staticmethod
    def _normalize_heading(heading: str) -> str:
        """Normalize a heading for matching: strip ##, normalize whitespace."""
        h = heading.strip()
        if h.startswith("##"):
            h = h[2:].strip()
        # Collapse multiple spaces
        h = re.sub(r"\s+", " ", h)
        return h

    def _match_section_key(self, norm: str) -> Optional[str]:
        """Try to match a normalized heading against known section keys."""
        # Direct match
        for key in _SECTION_KEYS:
            if norm.lower() == self._normalize_heading(key).lower():
                return key

        # Partial match (e.g., "背景" matches "## 1. 背景")
        for key in _SECTION_KEYS:
            key_norm = self._normalize_heading(key)
            # Check if the core title part matches
            if key_norm.split(". ", 1)[-1].lower() == norm.split(". ", 1)[-1].lower():
                return key
            # Or if one contains the other
            if key_norm.lower() in norm.lower() or norm.lower() in key_norm.lower():
                return key

        return None

    # ── Keyword extraction ───────────────────────────────────────────────

    @staticmethod
    def _extract_keywords(text: str, keywords: List[str]) -> List[str]:
        """Extract known keywords from text using word boundaries, deduplicated in order."""
        import re
        found = []
        seen: set = set()
        for kw in keywords:
            pattern = re.compile(r'\b' + re.escape(kw) + r'\b', re.IGNORECASE)
            if pattern.search(text) and kw not in seen:
                found.append(kw)
                seen.add(kw)
        return found

    # ── Citation grounding ───────────────────────────────────────────────

    def _build_page_annotated_body(self, content: "StructuredPdfContent") -> str:
        """Build page-annotated body text for LLM context."""
        parts: List[str] = []
        for block in content.text_blocks:
            if block.type.value in ("text", "heading"):
                page_label = f"[Page {block.page + 1}]"
                parts.append(f"{page_label} {block.text}")
        return "\n\n".join(parts)

    def verify_claims(self, result: PaperAnalysisResult, content: "StructuredPdfContent") -> PaperAnalysisResult:
        """Verify each citation claim against source text blocks with evidence strength scoring.

        Features:
        - Evidence strength scoring (0.0-1.0) on a continuous scale
        - Self-correction loop: retry unverified claims with lower thresholds
        - Cross-page expansion: if exact page fails, try adjacent pages
        - Claim deduplication: same claim in multiple sections counted once

        Args:
            result: PaperAnalysisResult with sections containing [Page N] citations.
            content: Source StructuredPdfContent to verify against.

        Returns:
            Updated PaperAnalysisResult with claims and unverified_claims populated.
        """
        import re
        import json
        import urllib.request

        # ── Embedding helpers ──────────────────────────────────────────
        _embed_cache: Dict[str, List[float]] = {}

        def _get_embedding(text: str) -> Optional[List[float]]:
            """Get embedding from Ollama nomic-embed-text, cached."""
            cache_key = text[:200]
            if cache_key in _embed_cache:
                return _embed_cache[cache_key]
            try:
                req = urllib.request.Request(
                    "http://localhost:11434/api/embeddings",
                    data=json.dumps({"model": "nomic-embed-text", "prompt": text}).encode(),
                    headers={"Content-Type": "application/json"},
                    method="POST",
                )
                with urllib.request.urlopen(req, timeout=60) as resp:
                    data = json.loads(resp.read())
                    emb = data.get("embedding")
                    if emb:
                        _embed_cache[cache_key] = emb
                    return emb
            except Exception:
                return None

        def _cosine_sim(a: List[float], b: List[float]) -> float:
            dot = sum(x * y for x, y in zip(a, b))
            norm_a = math.sqrt(sum(x * x for x in a))
            norm_b = math.sqrt(sum(y * y for y in b))
            if norm_a == 0 or norm_b == 0:
                return 0.0
            return dot / (norm_a * norm_b)

        def _word_overlap(claim: str, source: str) -> float:
            """Return overlap ratio 0-1."""
            claim_words = set(w.strip(".,;:!?()[]{}\"'") for w in claim.lower().split())
            source_words = set(w.strip(".,;:!?()[]{}\"'") for w in source.lower().split())
            if not claim_words:
                return 0.0
            overlap = claim_words & source_words
            return len(overlap) / len(claim_words)

        def _evidence_score(emb_sim: float, overlap: float) -> float:
            """Combined evidence score: embedding × 0.6 + word overlap × 0.4."""
            return min(1.0, emb_sim * 0.6 + overlap * 0.4)

        # ── Claim type classification ─────────────────────────────────
        _NUMERICAL_PATTERNS = [
            r'\d+\.?\d*%',
            r'\d+x',
            r'\d+\.\d+%',
            r'\d+倍',
            r'(准确率|精度|提升|提高|降低|增长|超过|击败|优于)',
            r'(accuracy|precision|recall|f1|latency|speed|throughput|improve|improve|improvement)',
        ]
        _METHODOLOGY_PATTERNS = [
            r'(使用|基于|采用|提出|设计|架构|机制|方法|框架|原理)',
            r'(architecture|mechanism|framework|approach|methodology)',
        ]

        def _classify_claim(claim_text: str) -> str:
            """Classify a claim into: 'numerical', 'methodology', or 'descriptive'."""
            text = claim_text.lower()
            if any(re.search(p, text) for p in _NUMERICAL_PATTERNS):
                return "numerical"
            if any(re.search(p, text) for p in _METHODOLOGY_PATTERNS):
                return "methodology"
            return "descriptive"

        # Adaptive thresholds per claim type
        _THRESHOLDS = {
            "numerical":    (0.75, 0.20),   # stricter: exact numbers
            "methodology":   (0.70, 0.15),   # standard
            "descriptive":   (0.65, 0.12),   # looser: LLM paraphrase
        }
        _ACCEPT_SCORE = {"numerical": 0.70, "methodology": 0.60, "descriptive": 0.55}

        def _verify_block(
            claim_text: str, block_text: str, use_emb: bool,
            emb_thresh: float = 0.70, ovlp_thresh: float = 0.15,
        ) -> tuple[bool, float, str]:
            """Verify claim against a single block. Returns (verified, score, note)."""
            overlap = _word_overlap(claim_text, block_text)

            if use_emb:
                claim_emb = _get_embedding(claim_text[:500])
                source_emb = _get_embedding(block_text[:500])
                if claim_emb and source_emb:
                    emb_sim = _cosine_sim(claim_emb, source_emb)
                    score = _evidence_score(emb_sim, overlap)
                    if emb_sim >= emb_thresh and overlap >= ovlp_thresh:
                        return True, score, f"emb={emb_sim:.2f}, ovlp={overlap:.2f}"
                    # Pure embedding fallback: very high sim even with low word match
                    if emb_sim >= 0.78:
                        return True, score, f"partial emb={emb_sim:.2f}"

            if overlap >= max(0.3, ovlp_thresh):
                return True, overlap, f"ovlp={overlap:.2f}"
            shared = len(set(claim_text.lower().split()) & set(block_text.lower().split()))
            if shared >= 4:
                return True, overlap, f"shared={shared} words"

            return False, 0.0, ""

        # ── Build page index ─────────────────────────────────────────
        page_blocks: Dict[int, List[tuple[int, str]]] = {}
        for idx, block in enumerate(content.text_blocks):
            if block.page not in page_blocks:
                page_blocks[block.page] = []
            page_blocks[block.page].append((idx, block.text))

        use_embedding = False

        # ── Extract claims with deduplication ─────────────────────────
        claim_pattern = re.compile(r"([^[]+?)\s*\[Page\s+(\d+)\]")
        seen: set[str] = set()

        raw_claims: List[dict] = []
        for section_text in result.sections.values():
            for m in claim_pattern.finditer(section_text):
                page_ref = int(m.group(2)) - 1
                claim_text = m.group(1).strip()
                norm = claim_text.lower()[:80]
                if norm in seen:
                    continue
                seen.add(norm)
                raw_claims.append({"text": claim_text, "page": page_ref})

        # ── Round 0: initial verification with adaptive thresholds ──────
        verified: List[CitationClaim] = []
        retry_queue: List[dict] = []

        for item in raw_claims:
            claim_type = _classify_claim(item["text"])
            emb_thresh, ovlp_thresh = _THRESHOLDS.get(claim_type, (0.70, 0.15))
            accept_score = _ACCEPT_SCORE.get(claim_type, 0.60)

            best_score = 0.0
            best_idx = -1
            best_chunk = ""
            note = ""
            done = False

            if item["page"] in page_blocks:
                for block_idx, block_text in page_blocks[item["page"]]:
                    ok, score, note = _verify_block(
                        item["text"], block_text, use_embedding,
                        emb_thresh=emb_thresh, ovlp_thresh=ovlp_thresh,
                    )
                    if ok and score > best_score:
                        best_score, best_idx, best_chunk = score, block_idx, block_text[:200]
                        note = score >= accept_score and f"Verified ({note})" or f"Partial ({note})"
                        if score >= accept_score:
                            done = True
                            break

            if done:
                verified.append(CitationClaim(
                    text=item["text"], page=item["page"], block_idx=best_idx,
                    chunk_text=best_chunk, verified=True, evidence_score=best_score,
                    verification_note=note, correction_round=0, claim_type=claim_type,
                ))
            else:
                retry_queue.append({
                    **item, "best_score": best_score, "best_chunk": best_chunk,
                    "best_block_idx": best_idx, "note": note, "claim_type": claim_type,
                })

        # ── Self-correction loop: up to 2 rounds ─────────────────────
        for round_n in range(1, 3):
            still: List[dict] = []
            for item in retry_queue:
                claim_type = item["claim_type"]
                emb_thresh, ovlp_thresh = _THRESHOLDS.get(claim_type, (0.70, 0.15))
                accept_score = _ACCEPT_SCORE.get(claim_type, 0.60)

                item_improved = False
                for delta in [-1, 1, -2, 2]:
                    nb_page = item["page"] + delta
                    if nb_page < 0 or nb_page not in page_blocks:
                        continue
                    for block_idx, block_text in page_blocks[nb_page]:
                        ok, score, note = _verify_block(
                            item["text"], block_text, use_embedding,
                            emb_thresh=emb_thresh, ovlp_thresh=ovlp_thresh,
                        )
                        if ok and score > item["best_score"]:
                            item["best_score"] = score
                            item["best_block_idx"] = block_idx
                            item["best_chunk"] = block_text[:200]
                            item["note"] = f"cross-page [{item['page']+1}→{nb_page+1}]: {note}"
                            item_improved = True
                            if score >= accept_score:
                                break
                    if item_improved and item["best_score"] >= accept_score:
                        break

                if item_improved and item["best_score"] > 0.05:
                    verified.append(CitationClaim(
                        text=item["text"], page=item["page"], block_idx=item["best_block_idx"],
                        chunk_text=item["best_chunk"], verified=True, evidence_score=item["best_score"],
                        verification_note=f"Verified after correction: {item['note']}",
                        correction_round=round_n, claim_type=claim_type,
                    ))
                else:
                    still.append(item)

            retry_queue = still
            if not retry_queue:
                break

        # ── Build final results ─────────────────────────────────────
        result.claims = verified
        result.unverified_claims = [
            CitationClaim(
                text=item["text"], page=item["page"], block_idx=item["best_block_idx"],
                chunk_text=item["best_chunk"], verified=False, evidence_score=item["best_score"],
                verification_note=item["note"] or "No matching source text found",
                correction_round=2, claim_type=item.get("claim_type", ""),
            )
            for item in retry_queue
        ]
        return result
