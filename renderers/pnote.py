"""P-Note (paper note) renderer."""
import textwrap
from typing import Any, Dict, List, Optional, Tuple

from core import Paper, today_iso


def render_pnote(
    p: Paper,
    tags: List[str],
    extracted_sections_md: str,
    ai_draft_md: str = "",
    table_md: str = "",
    math_md: str = "",
    parsed_ai: Optional[Tuple[Dict[str, str], Dict[str, Any]]] = None,
) -> str:
    """
    Render a P-note markdown file.

    Args:
        p: Paper dataclass
        tags: List of tag strings
        extracted_sections_md: PDF section snippets markdown
        ai_draft_md: Raw AI draft markdown (used if parsed_ai is None)
        table_md: Extracted table markdown
        math_md: Extracted math markdown
        parsed_ai: Optional (sections_dict, rubric_dict) from parse_ai_pnote_draft.
                   If provided, section content is injected into the template
                   and rubric scores are written to frontmatter.
    """
    date_for_note = p.published or today_iso()
    authors_line = ", ".join(p.authors) if p.authors else "Unknown"
    tags_list = ", ".join(tags)

    src_line = f"{p.source.upper()}: {p.uid}"

    # Build frontmatter
    frontmatter_fields = [
        "type: paper",
        "status: draft",
        f"date: {date_for_note}",
        f"tags: [{tags_list}]",
    ]
    if parsed_ai is not None:
        _, rubric_dict = parsed_ai
        scores = _extract_rubric_scores(rubric_dict)
        if scores:
            frontmatter_fields.append("rubric:")
            for k, v in scores.items():
                frontmatter_fields.append(f"  {k}: {v}")
            overall = rubric_dict.get("overall", "")
            if overall:
                # Escape double quotes in overall
                escaped = str(overall).replace('"', '\\"')
                frontmatter_fields.append(f'  overall: "{escaped}"')
        frontmatter_fields.append("ai_generated: true")
    elif ai_draft_md.strip():
        frontmatter_fields.append("rubric: draft-ai")

    fm = "\n".join(frontmatter_fields)

    # Build AI draft block
    ai_block = _build_ai_block(parsed_ai, ai_draft_md)

    table_md_section = (
        f"\n\n---\n\n## 附：PDF 表格（结构化抽取）\n\n{table_md.strip()}\n"
        if table_md.strip()
        else ""
    )
    math_md_section = (
        f"\n\n---\n\n## 附：PDF 公式（结构化抽取）\n\n{math_md.strip()}\n"
        if math_md.strip()
        else ""
    )
    sections_block = (
        extracted_sections_md
        if extracted_sections_md
        else "_（未能从 PDF 抽取到可用文本）_"
    )

    # Build section content from parsed_ai (for injection into template sections)
    injected_sections_md = _build_injected_sections_md(parsed_ai)

    md = f"""\
{fm}
------------------

# {p.title}

**Source:** {src_line}
**Authors:** {authors_line}
**Published:** {p.published or "N/A"} | **Updated:** {p.updated or "N/A"}
**Landing:** {p.abs_url}
**PDF:** {p.pdf_url or "N/A"}
**Primary Category:** {p.primary_category or "N/A"}

---

## Research Question Card

* 我想解决什么问题？
* 为什么重要？
* 我的先验判断是什么？
* 什么证据会推翻我？

---

## 1. 背景

> **Abstract（原文）**
> {p.abstract or "(未获取到 abstract，可手动补充)"}

{injected_sections_md.get("## 1. 背景", "")}

---

## 2. 核心问题

{injected_sections_md.get("## 2. 核心问题", "")}

---

## 3. 方法结构
### 3.1 架构拆解

{injected_sections_md.get("## 3.1 架构拆解", "")}

### 3.2 算法逻辑

{injected_sections_md.get("## 3.2 算法逻辑", "")}

### 3.3 关键组件

{injected_sections_md.get("## 3.3 关键组件", "")}

---

## 4. 关键创新

{injected_sections_md.get("## 4. 关键创新", "")}

---

## 5. 实验分析
### 5.1 数据集

{injected_sections_md.get("## 5.1 数据集", "")}

### 5.2 基线对比

{injected_sections_md.get("## 5.2 基线对比", "")}

### 5.3 消融实验

{injected_sections_md.get("## 5.3 消融实验", "")}

### 5.4 成本分析

{injected_sections_md.get("## 5.4 成本分析", "")}

---

## 6. 对抗式审稿
* 逻辑漏洞：
* 偏置风险：
* 复现难度：
* 失败模式推测：

{injected_sections_md.get("## 6. 对抗式审稿", "")}

---

## 7. 优势

{injected_sections_md.get("## 7. 优势", "")}

---

## 8. 局限

{injected_sections_md.get("## 8. 局限", "")}

---

## 9. 本质抽象

{injected_sections_md.get("## 9. 本质抽象", "")}

---

## 10. 与其他方法对比
* vs A：
* vs B：
* vs C：

{injected_sections_md.get("## 10. 与其他方法对比", "")}

---

## 11. Decision（决策）
* 是否使用？
* 使用场景？
* 不适用边界？
* 接下来关注信号？

{injected_sections_md.get("## 11. Decision（决策）", "")}

---

## 知识蒸馏
### Facts
1.
2.

### Principles
1.
2.

### Insights
1.
2.

{injected_sections_md.get("## 12. 知识蒸馏", "")}

---

## 认知升级
* 长期价值：
* 规模效应：
* 技术护城河：
* 是否范式转移：
* 商业潜力：

{injected_sections_md.get("## 13. 认知升级", "")}

---

## 评分量表
* Novelty (1-5):
* Leverage (1-5):
* Evidence (1-5):
* Cost (1-5):
* Moat (1-5):
* Adoption Signal (1-5):

### Overall Judgment

{ai_block}---

## 附：PDF 章节粗拆（自动抽取 · 供快速定位）

{sections_block}{table_md_section}{math_md_section}
"""
    return textwrap.dedent(md).strip() + "\n"


def _build_injected_sections_md(
    parsed_ai: Optional[Tuple[Dict[str, str], Dict[str, Any]]],
) -> Dict[str, str]:
    """Extract section content from parsed_ai for template injection."""
    if parsed_ai is None:
        return {}
    sections_dict, _ = parsed_ai
    return sections_dict


def _build_ai_block(
    parsed_ai: Optional[Tuple[Dict[str, str], Dict[str, Any]]],
    ai_draft_md: str,
) -> str:
    """Build the ## AI 自动初稿 block for the bottom of the note."""
    if parsed_ai is not None:
        # Show full raw output at bottom for reference
        sections_dict, rubric_dict = parsed_ai
        raw = sections_dict.get("__raw__", "")
        if raw:
            return f"> AI Draft（可编辑，需人工核验）\n\n{raw.strip()}\n"
        return ""
    elif ai_draft_md.strip():
        return f"> AI Draft（可编辑，需人工核验）\n\n{ai_draft_md.strip()}\n"
    return ""


def _extract_rubric_scores(rubric: Dict[str, Any]) -> Dict[str, int]:
    """Extract valid integer rubric scores (1-5) from rubric dict."""
    score_keys = ["novelty", "leverage", "evidence", "cost", "moat", "adoption"]
    return {
        k: v
        for k in score_keys
        for v in [rubric.get(k)]
        if isinstance(v, int) and 1 <= v <= 5
    }
