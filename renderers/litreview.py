"""Literature Review renderer: Generates incremental review documents."""
from __future__ import annotations

import datetime
import textwrap
from typing import List, Dict, Any


def render_litreview(
    topic: str,
    papers: List[Dict[str, Any]],
    created_at: str = None,
    updated_at: str = None,
) -> str:
    """Generate a literature review Markdown document.

    Args:
        topic: Research topic for this review
        papers: List of paper dicts with keys: arxiv_id, title, score, published, abstract
        created_at: ISO timestamp when review was created
        updated_at: ISO timestamp of last update

    Returns:
        Markdown string for the literature review
    """
    now = datetime.datetime.now().isoformat()
    created = created_at or now
    updated = updated_at or now

    paper_count = len(papers)
    date_range = _get_date_range(papers)

    # Sort papers by date (newest first)
    sorted_papers = sorted(
        papers,
        key=lambda p: p.get("published", ""),
        reverse=True
    )

    # Sort by score for top papers
    top_papers = sorted(
        papers,
        key=lambda p: p.get("score", 0),
        reverse=True
    )[:10]

    # Build markdown
    lines = [
        "---",
        f"type: lit-review",
        f"topic: {topic}",
        f'created_at: "{created}"',
        f'last_updated: "{updated}"',
        "status: evolving",
        f"paper_count: {paper_count}",
        "---",
        "",
        f"# {topic} 文献综述",
        "",
        "## 概述",
        "",
        f"- **论文数量**: {paper_count}",
        f"- **时间范围**: {date_range}",
        f"- **最后更新**: {updated[:10]}",
        "",
        "本综述随订阅论文自动更新，保持与研究前沿同步。",
        "",
        "## 研究时间线",
        "",
    ]

    # Add timeline
    if sorted_papers:
        lines.append("| 日期 | 论文 |")
        lines.append("|------|------|")
        for p in sorted_papers[:20]:
            date = p.get("published", "未知")[:10]
            title = p.get("title", "无标题")[:50]
            lines.append(f"| {date} | {title} |")
    else:
        lines.append("_暂无论文数据_")

    lines.extend(["", "## 方法分类", ""])

    # Group papers by methodology keywords
    method_groups = _group_by_methodology(papers)
    if method_groups:
        for method, group_papers in method_groups.items():
            lines.append(f"### {method} ({len(group_papers)} 篇)")
            lines.append("")
            for p in group_papers[:5]:
                title = p.get("title", "无标题")
                score = p.get("score", 0)
                lines.append(f"- **{title[:60]}** (score={score:.2f})")
            if len(group_papers) > 5:
                lines.append(f"- _... 还有 {len(group_papers) - 5} 篇_")
            lines.append("")
    else:
        lines.append("_暂无分类数据_")
        lines.append("")

    lines.extend(["## 代表论文 (Top 10)", ""])

    if top_papers:
        for i, p in enumerate(top_papers, 1):
            title = p.get("title", "无标题")
            score = p.get("score", 0)
            arxiv_id = p.get("arxiv_id", "")
            lines.append(f"{i}. [{title}](https://arxiv.org/abs/{arxiv_id}) _[score: {score:.2f}]_")
    else:
        lines.append("_暂无论文数据_")

    lines.extend(["", "## 开放问题", ""])

    # Extract potential open problems from abstracts
    open_problems = _extract_open_problems(papers)
    if open_problems:
        for problem in open_problems[:5]:
            lines.append(f"- {problem}")
    else:
        lines.append("- 持续跟踪最新研究进展")

    lines.extend(["", "## 更新日志", ""])
    lines.append(f"- {now[:10]}: 创建综述文档 ({paper_count} 篇论文)")

    return "\n".join(lines) + "\n"


def update_litreview(
    existing_content: str,
    new_papers: List[Dict[str, Any]],
    all_papers: List[Dict[str, Any]] = None,
) -> str:
    """Incrementally update an existing literature review.

    Preserves user annotations in existing sections.

    Args:
        existing_content: Current review Markdown content
        new_papers: List of newly added papers
        all_papers: Full list of papers (if None, will be reconstructed)

    Returns:
        Updated Markdown string
    """
    now = datetime.datetime.now().isoformat()

    lines = existing_content.split("\n")
    updated_lines = []

    # Find and update the frontmatter
    in_frontmatter = False
    frontmatter_end = -1
    for i, line in enumerate(lines):
        if line.strip() == "---":
            if not in_frontmatter:
                in_frontmatter = True
                updated_lines.append(line)
            else:
                frontmatter_end = i
                updated_lines.append(line)
                # Add updated fields
                updated_lines.append(f'last_updated: "{now}"')
                if all_papers:
                    updated_lines.append(f"paper_count: {len(all_papers)}")
                break
        else:
            updated_lines.append(line)

    # Update overview section if we have all papers
    if all_papers:
        paper_count = len(all_papers)
        date_range = _get_date_range(all_papers)
        for i, line in enumerate(updated_lines):
            if line.startswith("**论文数量**"):
                updated_lines[i] = f"- **论文数量**: {paper_count}"
            elif line.startswith("**时间范围**"):
                updated_lines[i] = f"- **时间范围**: {date_range}"
            elif line.startswith("**最后更新**"):
                updated_lines[i] = f"- **最后更新**: {now[:10]}"

    # Find changelog section
    changelog_start = -1
    for i, line in enumerate(lines):
        if "## 更新日志" in line:
            changelog_start = i
            break

    if changelog_start >= 0 and new_papers:
        # Insert new entries before the changelog section
        result = updated_lines[:changelog_start]
        result.extend(["## 更新日志", ""])
        for p in new_papers[:5]:
            title = p.get("title", "无标题")[:50]
            arxiv_id = p.get("arxiv_id", "")
            result.append(f"- {now[:10]}: 新增 [{title}](https://arxiv.org/abs/{arxiv_id})")
        result.append("")
        # Add the rest
        for line in lines[changelog_start + 1:]:
            # Skip duplicate entries we just added
            skip = False
            for p in new_papers[:5]:
                if p.get("title", "")[:50] in line and now[:10] in line:
                    skip = True
                    break
            if not skip:
                result.append(line)
        return "\n".join(result)

    return "\n".join(updated_lines)


def _get_date_range(papers: List[Dict[str, Any]]) -> str:
    """Get date range string from papers list."""
    dates = [p.get("published", "")[:10] for p in papers if p.get("published")]
    if not dates:
        return "未知"
    dates.sort()
    return f"{dates[0]} ~ {dates[-1]}"


def _group_by_methodology(papers: List[Dict[str, Any]]) -> Dict[str, List[Dict]]:
    """Group papers by detected methodology keywords."""
    method_keywords = {
        "Transformer": ["transformer", "attention", "self-attention", "bert", "gpt"],
        "CNN/卷积": ["convolution", "cnn", "convolutional", "resnet", "vgg"],
        "图神经网络": ["graph", "gnn", "gcn", "gat"],
        "强化学习": ["reinforcement", "rl", "policy", "q-learning", "ddpg"],
        "扩散模型": ["diffusion", "ddpm", "score-based", "gan"],
        "检索增强": ["retrieval", "rag", "retrieval-augmented", "knowledge retrieval"],
        "多模态": ["multimodal", "vision-language", "image-text", "vqa"],
        "大语言模型": ["llm", "large language", "foundation model", "gpt-", "claude", "gemini"],
    }

    groups: Dict[str, List[Dict]] = {}
    for paper in papers:
        text = (
            paper.get("title", "") + " " + paper.get("abstract", "")
        ).lower()

        for method, keywords in method_keywords.items():
            if any(kw in text for kw in keywords):
                if method not in groups:
                    groups[method] = []
                groups[method].append(paper)
                break

    return groups


def _extract_open_problems(papers: List[Dict[str, Any]]) -> List[str]:
    """Extract potential open problems from paper abstracts."""
    problems = []

    signal_phrases = [
        "remain an open problem",
        "future work",
        "future research",
        "left for future",
        "beyond the scope",
        "limitation",
        "challenge",
        "opportunity",
        "potential future",
    ]

    for paper in papers[:15]:  # Check top 15 papers
        abstract = paper.get("abstract", "").lower()
        for phrase in signal_phrases:
            idx = abstract.find(phrase)
            if idx > 0:
                # Extract context around the phrase
                start = max(0, idx - 50)
                end = min(len(abstract), idx + 80)
                snippet = abstract[start:end].strip()
                # Clean up
                snippet = " ".join(snippet.split())[:100]
                if snippet and len(snippet) > 20:
                    title = paper.get("title", "")[:40]
                    problems.append(f"_{title}..._: {snippet}...")
                    break

    return problems[:5]  # Return top 5 unique problems
