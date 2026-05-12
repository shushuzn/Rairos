//! P-Note (paper note) renderer.
//!
//! Renders a full P-Note markdown document from paper data, tags,
//! extracted sections, AI drafts, tables, math, and rubric scores.

use std::collections::HashMap;

/// Paper data for rendering a P-Note.
#[derive(Debug, Clone)]
pub struct Paper {
    pub uid: String,
    pub title: String,
    pub authors: Vec<String>,
    pub abstract_: String,
    pub published: Option<String>,
    pub updated: Option<String>,
    pub abs_url: Option<String>,
    pub pdf_url: Option<String>,
    pub source: String,
    pub primary_category: Option<String>,
}

/// Render a P-Note markdown document.
///
/// # Arguments
/// * `p` - Paper data
/// * `tags` - List of tag strings
/// * `extracted_sections_md` - PDF section snippets markdown
/// * `ai_draft_md` - Raw AI draft markdown
/// * `table_md` - Extracted table markdown
/// * `math_md` - Extracted math markdown
/// * `rubric_scores` - Optional HashMap with keys: novelty, leverage, evidence, cost, moat, adoption (1-5)
/// * `ai_overall` - Optional overall judgment text
pub fn render_pnote(
    p: &Paper,
    tags: &[String],
    extracted_sections_md: &str,
    ai_draft_md: &str,
    table_md: &str,
    math_md: &str,
    rubric_scores: Option<HashMap<String, i32>>,
    ai_overall: Option<&str>,
) -> String {
    let date_for_note = p.published.clone().unwrap_or_else(today_iso);
    let authors_line = if p.authors.is_empty() {
        "Unknown".to_string()
    } else {
        p.authors.join(", ")
    };
    let tags_list = tags.join(", ");
    let src_line = format!("{}: {}", p.source.to_uppercase(), p.uid);

    let mut frontmatter_fields = vec![
        "type: paper".to_string(),
        "status: draft".to_string(),
        format!("date: {}", date_for_note),
        format!("tags: [{}]", tags_list),
    ];

    let radar_svg: String;
    if let Some(ref scores) = rubric_scores {
        if !scores.is_empty() {
            frontmatter_fields.push("rubric:".to_string());
            for k in [
                "novelty", "leverage", "evidence", "cost", "moat", "adoption",
            ] {
                if let Some(&v) = scores.get(k) {
                    frontmatter_fields.push(format!("  {}: {}", k, v));
                }
            }
            if let Some(overall) = ai_overall {
                frontmatter_fields.push(format!("  overall: \"{}\"", overall.replace('"', "\\\"")));
            }
            frontmatter_fields.push("ai_generated: true".to_string());
            radar_svg = crate::radar_chart::render_radar_chart(
                scores.iter().map(|(k, v)| (k.clone(), *v)).collect(),
                280,
            );
        } else {
            radar_svg = String::new();
        }
    } else {
        radar_svg = String::new();
    }

    if ai_draft_md.trim().is_empty() && rubric_scores.is_none() {
        frontmatter_fields.push("rubric: draft-ai".to_string());
    }

    let fm = frontmatter_fields.join("\n");

    let table_md_section = if !table_md.trim().is_empty() {
        format!(
            "\n\n---\n\n## 附：PDF 表格（结构化抽取）\n\n{}\n",
            table_md.trim()
        )
    } else {
        String::new()
    };

    let math_md_section = if !math_md.trim().is_empty() {
        format!(
            "\n\n---\n\n## 附：PDF 公式（结构化抽取）\n\n{}\n",
            math_md.trim()
        )
    } else {
        String::new()
    };

    let sections_block = if !extracted_sections_md.is_empty() {
        extracted_sections_md.to_string()
    } else {
        "_（未能从 PDF 抽取到可用文本）_".to_string()
    };

    let ai_block = if !ai_draft_md.trim().is_empty() {
        format!(
            "> AI Draft（可编辑，需人工核验）\n\n{}\n",
            ai_draft_md.trim()
        )
    } else {
        String::new()
    };

    format!(
        r#"{fm}
------------------

# {title}

**Source:** {src_line}
**Authors:** {authors_line}
**Published:** {published} | **Updated:** {updated}
**Landing:** {abs_url}
**PDF:** {pdf_url}
**Primary Category:** {primary_category}

---

## Research Question Card

* 我想解决什么问题？
* 为什么重要？
* 我的先验判断是什么？
* 什么证据会推翻我？

---

## 1. 背景

> **Abstract（原文）**
> {abstract_}

---

## 2. 核心问题

---

## 3. 方法结构
### 3.1 架构拆解

### 3.2 算法逻辑

### 3.3 关键组件

---

## 4. 关键创新

---

## 5. 实验分析
### 5.1 数据集

### 5.2 基线对比

### 5.3 消融实验

### 5.4 成本分析

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

{radar_svg}

* Novelty (1-5):
* Leverage (1-5):
* Evidence (1-5):
* Cost (1-5):
* Moat (1-5):
* Adoption Signal (1-5):

### Overall Judgment

{ai_block}---
## 附：PDF 章节粗拆（自动抽取 · 供快速定位）

{sections_block}{table_md_section}{math_md_section}"#,
        fm = fm,
        title = p.title,
        src_line = src_line,
        authors_line = authors_line,
        published = p.published.as_deref().unwrap_or("N/A"),
        updated = p.updated.as_deref().unwrap_or("N/A"),
        abs_url = p.abs_url.as_deref().unwrap_or("N/A"),
        pdf_url = p.pdf_url.as_deref().unwrap_or("N/A"),
        primary_category = p.primary_category.as_deref().unwrap_or("N/A"),
        abstract_ = if p.abstract_.is_empty() {
            "(未获取到 abstract，可手动补充)"
        } else {
            &p.abstract_
        },
        radar_svg = if radar_svg.is_empty() {
            String::new()
        } else {
            format!("\n{}\n", radar_svg)
        },
        ai_block = ai_block,
        sections_block = sections_block,
        table_md_section = table_md_section,
        math_md_section = math_md_section,
    )
}

fn today_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}
