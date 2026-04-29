"""P-Note (paper note) renderer."""
import math
import textwrap
from typing import Any, Dict, List, Optional, Tuple

from core import Paper, today_iso


# ── Radar chart ────────────────────────────────────────────────────────────────

def render_radar_chart(scores: Dict[str, int], size: int = 280) -> str:
    """Render a 6-axis radar chart SVG from rubric scores.

    Axes: Novelty · Leverage · Evidence · Cost · Moat · Adoption
    Each score is 1-5; larger is better on all axes.
    Cost is inverted so higher = cheaper (better).

    Args:
        scores: Dict with keys novelty/leverage/evidence/cost/moat/adoption (1-5)
        size: SVG viewBox width/height (default 280)

    Returns:
        SVG markup string, or empty string if insufficient data.
    """
    AXES = [
        ("Novelty", "创新性"),
        ("Leverage", "杠杆效应"),
        ("Evidence", "实验证据"),
        ("Cost", "成本"),
        ("Moat", "护城河"),
        ("Adoption", "采纳信号"),
    ]
    n = len(AXES)
    if n < 3:
        return ""

    # Only include axes that have valid scores
    valid_axes = [(AXES[i][0], AXES[i][1], scores.get(AXES[i][0].lower(), 0))
                  for i in range(n)]
    valid_axes = [(a, b, s) for a, b, s in valid_axes if isinstance(s, (int, float)) and 1 <= s <= 5]
    if len(valid_axes) < 3:
        return ""

    # Adjust n for valid axes only
    n = len(valid_axes)
    angle_step = 2 * math.pi / n

    cx = cy = size / 2
    max_radius = size / 2 - 42  # leave room for labels
    rings = 5  # 1-5 scale

    # Colour palette (professional, accessible)
    fill_colour   = "#3b82f6"   # blue-500, ~60% opacity
    stroke_colour = "#1d4ed8"   # blue-700
    grid_colour   = "#94a3b8"  # slate-400
    label_colour  = "#334155"   # slate-700
    bg_colour     = "#f8fafc"   # slate-50

    parts = [
        f'<svg viewBox="0 0 {size} {size}" xmlns="http://www.w3.org/2000/svg" '
        f'role="img" aria-label="论文评分雷达图">',
        f'  <title>论文评分雷达图</title>',
        f'  <rect width="{size}" height="{size}" fill="{bg_colour}" rx="8"/>',
    ]

    # ── Grid rings ──────────────────────────────────────────────────────────
    for ring in range(1, rings + 1):
        r = max_radius * ring / rings
        ring_pts = " ".join(
            f"{cx + r * math.sin(i * angle_step - math.pi / 2):.1f},"
            f"{cy + r * math.cos(i * angle_step - math.pi / 2):.1f}"
            for i in range(n)
        )
        parts.append(f'  <polygon points="{ring_pts}" fill="none" stroke="{grid_colour}" '
                     f'stroke-width="0.6" stroke-dasharray="2,2"/>')
        # Ring label (only on outermost)
        if ring == rings:
            parts.append(
                f'  <text x="{cx}" y="{cy - r - 3}" text-anchor="middle" '
                f'font-size="9" fill="{grid_colour}">5</text>'
            )
            parts.append(
                f'  <text x="{cx}" y="{cy - r // 2 - 3}" text-anchor="middle" '
                f'font-size="9" fill="{grid_colour}">{rings // 2}</text>'
            )
            parts.append(
                f'  <text x="{cx}" y="{cy - 3}" text-anchor="middle" '
                f'font-size="9" fill="{grid_colour}">1</text>'
            )

    # ── Axes (spokes) ─────────────────────────────────────────────────────
    for i, (en, zh, _) in enumerate(valid_axes):
        angle = i * angle_step - math.pi / 2
        x2 = cx + max_radius * math.sin(angle)
        y2 = cy + max_radius * math.cos(angle)
        parts.append(f'  <line x1="{cx:.1f}" y1="{cy:.1f}" x2="{x2:.1f}" y2="{y2:.1f}" '
                     f'stroke="{grid_colour}" stroke-width="0.8"/>')

    # ── Data polygon ────────────────────────────────────────────────────────
    data_pts = " ".join(
        f"{cx + max_radius * (s / rings) * math.sin(i * angle_step - math.pi / 2):.1f},"
        f"{cy + max_radius * (s / rings) * math.cos(i * angle_step - math.pi / 2):.1f}"
        for i, (_, _, s) in enumerate(valid_axes)
    )
    parts.append(
        f'  <polygon points="{data_pts}" fill="{fill_colour}" fill-opacity="0.35" '
        f'stroke="{stroke_colour}" stroke-width="1.5" stroke-linejoin="round"/>'
    )

    # ── Data point dots ─────────────────────────────────────────────────────
    for i, (_, _, s) in enumerate(valid_axes):
        angle = i * angle_step - math.pi / 2
        x = cx + max_radius * (s / rings) * math.sin(angle)
        y = cy + max_radius * (s / rings) * math.cos(angle)
        parts.append(
            f'  <circle cx="{x:.1f}" cy="{y:.1f}" r="3" '
            f'fill="{stroke_colour}" stroke="{bg_colour}" stroke-width="1"/>'
        )

    # ── Axis labels ─────────────────────────────────────────────────────────
    for i, (en, zh, s) in enumerate(valid_axes):
        angle = i * angle_step - math.pi / 2
        label_r = max_radius + 20
        lx = cx + label_r * math.sin(angle)
        ly = cy + label_r * math.cos(angle)
        anchor = "start" if lx > cx + 10 else "end" if lx < cx - 10 else "middle"
        parts.append(
            f'  <text x="{lx:.1f}" y="{ly:.1f}" text-anchor="{anchor}" '
            f'dominant-baseline="middle" font-size="10.5" font-weight="600" '
            f'fill="{label_colour}">{zh}</text>'
        )
        parts.append(
            f'  <text x="{lx:.1f}" y="{ly + 13:.1f}" text-anchor="{anchor}" '
            f'dominant-baseline="middle" font-size="9" fill="{grid_colour}">'
            f'{en}={s}</text>'
        )

    # ── Summary badge ───────────────────────────────────────────────────────
    total = sum(s for _, _, s in valid_axes)
    avg = total / n
    badge_r = 18
    bx, by = cx + max_radius * 0.6, cy - max_radius * 0.55
    parts.append(
        f'  <circle cx="{bx:.1f}" cy="{by:.1f}" r="{badge_r}" '
        f'fill="{stroke_colour}" opacity="0.9"/>'
    )
    parts.append(
        f'  <text x="{bx:.1f}" y="{by - 3}" text-anchor="middle" '
        f'dominant-baseline="middle" font-size="11" font-weight="bold" fill="white">'
        f'{avg:.1f}</text>'
    )
    parts.append(
        f'  <text x="{bx:.1f}" y="{by + 9}" text-anchor="middle" '
        f'dominant-baseline="middle" font-size="8" fill="white">avg</text>'
    )

    parts.append("</svg>")
    return "\n".join(parts)


def render_pnote(
    p: Paper,
    tags: List[str],
    extracted_sections_md: str,
    ai_draft_md: str = "",
    table_md: str = "",
    math_md: str = "",
    parsed_ai: Optional[Tuple[Dict[str, str], Dict[str, Any]]] = None,
    claims_data: Optional[Dict[str, Any]] = None,
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
        claims_data: Optional dict with 'claims' and 'unverified_claims' lists
                     from PaperAnalysisResult for citation verification display.
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

    # Build radar chart if rubric scores are available
    radar_svg = ""
    if parsed_ai is not None:
        _, rubric_dict = parsed_ai
        scores = _extract_rubric_scores(rubric_dict)
        if scores:
            radar_svg = render_radar_chart(scores)

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

    # Build citation verification claims section
    claims_section = _build_claims_section(claims_data)

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

{radar_svg}

* Novelty (1-5):
* Leverage (1-5):
* Evidence (1-5):
* Cost (1-5):
* Moat (1-5):
* Adoption Signal (1-5):

### Overall Judgment

{ai_block}

{claims_section}---

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


def _build_claims_section(claims_data: Optional[Dict[str, Any]]) -> str:
    """Build citation verification claims section for P-note.

    Renders verified and unverified claims from PaperAnalysisResult
    with clear visual differentiation.
    """
    if not claims_data:
        return ""

    claims = claims_data.get("claims", [])
    unverified = claims_data.get("unverified_claims", [])

    if not claims and not unverified:
        return ""

    parts = []

    # Summary stats
    total = len(claims) + len(unverified)
    verified_count = len(claims)
    unverified_count = len(unverified)
    rate = (verified_count / total * 100) if total > 0 else 0

    parts.append("## 引用验证摘要\n")
    parts.append(f"| 状态 | 数量 | 验证率 |")
    parts.append(f"|------|------|--------|")
    parts.append(f"| ✅ 已验证 | {verified_count} | {rate:.0f}% |")
    parts.append(f"| ⚠️ 未验证 | {unverified_count} | — |")
    parts.append("")

    _TYPE_ICONS = {
        "numerical":   "📊",
        "methodology": "🔧",
        "descriptive": "📝",
    }

    def _type_label(claim_type: str) -> str:
        icon = _TYPE_ICONS.get(claim_type, "📝")
        label = {"numerical": "数字类", "methodology": "方法论", "descriptive": "描述性"}.get(claim_type, "描述性")
        return f"{icon} {label}"

    # Verified claims
    if claims:
        parts.append("### ✅ 已验证 Claims\n")
        for i, c in enumerate(claims, 1):
            page = c.get("page", "?")
            chunk = c.get("chunk_text", "")
            score = c.get("evidence_score", 0.0)
            ctype = c.get("claim_type", "")
            type_str = _type_label(ctype)
            parts.append(f"{i}. {type_str} · **[Page {page}]** 证据强度 {score:.0%}  {chunk}\n")

    # Unverified claims - these need attention
    if unverified:
        parts.append("### ⚠️ 未验证 Claims（需人工核查）\n")
        for i, c in enumerate(unverified, 1):
            page = c.get("page", "?")
            chunk = c.get("chunk_text", "")
            score = c.get("evidence_score", 0.0)
            note = c.get("verification_note", "无法在原文找到支撑文本")
            ctype = c.get("claim_type", "")
            type_str = _type_label(ctype)
            parts.append(
                f"{i}. {type_str} · **[Page {page}]** 证据强度 {score:.0%}  {chunk}\n"
                f"   > 🔍 未验证原因：{note}\n"
            )

    return "\n".join(parts)

