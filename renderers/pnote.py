"""P-Note (paper note) renderer."""
import textwrap
from typing import List

from core import Paper, today_iso


def render_pnote(p: Paper, tags: List[str], extracted_sections_md: str, ai_draft_md: str = "",
                 table_md: str = "", math_md: str = "") -> str:
    date_for_note = p.published or today_iso()
    authors_line = ", ".join(p.authors) if p.authors else "Unknown"
    tags_list = ", ".join(tags)

    src_line = f"{p.source.upper()}: {p.uid}"
    abstract_md = ("> **Abstract（原文）**  \n> " + p.abstract) if p.abstract else "_（未获取到 abstract，可手动补充）_"
    ai_block = (f"\n\n---\n\n## AI 自动初稿（待核验）\n\n{ai_draft_md.strip()}\n") if ai_draft_md.strip() else ""
    table_md_section = (f"\n\n---\n\n## 附：PDF 表格（结构化抽取）\n\n{table_md.strip()}\n") if table_md.strip() else ""
    math_md_section = (f"\n\n---\n\n## 附：PDF 公式（结构化抽取）\n\n{math_md.strip()}\n") if math_md.strip() else ""

    sections_block = extracted_sections_md if extracted_sections_md else "_（未能从 PDF 抽取到可用文本）_"

    md = f"""\
type: paper
status: draft
date: {date_for_note}
tags: [{tags_list}]
------------------

# {p.title}

**Source:** {src_line}  
**Authors:** {authors_line}  
**Published:** {p.published or "N/A"}  | **Updated:** {p.updated or "N/A"}  
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

{abstract_md}

---

## 2. 核心问题

---

## 3. 方法结构

### 架构拆解

### 算法逻辑

### 关键组件

---

## 4. 关键创新

---

## 5. 实验分析

### 数据集

### 基线对比

### 消融实验

### 成本分析

---

## 6. 对抗式审稿

* 逻辑漏洞：
* 偏置风险：
* 复现难度：
* 失败模式推测：

---

## 7. 优势

---

## 8. 局限

---

## 9. 本质抽象

---

## 10. 与其他方法对比

* vs A：
* vs B：
* vs C：

---

## 11. Decision（决策）

* 是否使用？
* 使用场景？
* 不适用边界？
* 接下来关注信号？

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

---

## 认知升级

* 长期价值：
* 规模效应：
* 技术护城河：
* 是否范式转移：
* 商业潜力：

---

## 评分量表

* Novelty (1-5):
* Leverage (1-5):
* Evidence (1-5):
* Cost (1-5):
* Moat (1-5):
* Adoption Signal (1-5):

### Overall Judgment
{ai_block}
---

## 附：PDF 章节粗拆（自动抽取 · 供快速定位）

{sections_block}{table_md_section}{math_md_section}
"""
    return textwrap.dedent(md).strip() + "\n"
