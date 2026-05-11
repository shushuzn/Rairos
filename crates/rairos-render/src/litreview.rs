//! Literature Review renderer: Generates incremental review documents.

use chrono::Utc;
use std::collections::HashMap;

/// Paper dict for literature review.
pub type PaperDict = HashMap<String, serde_json::Value>;

/// Render a literature review Markdown document.
pub fn render_litreview(
    topic: &str,
    papers: &[PaperDict],
    created_at: Option<&str>,
    updated_at: Option<&str>,
) -> String {
    let now = Utc::now().to_rfc3339();
    let created = created_at.unwrap_or(&now);
    let updated = updated_at.unwrap_or(&now);

    let paper_count = papers.len();
    let date_range = get_date_range(papers);
    let sorted_papers = sorted_by_date(papers);
    let top_papers = sorted_by_score(papers);

    let mut lines = Vec::new();

    lines.push("---".to_string());
    lines.push(format!("type: lit-review"));
    lines.push(format!("topic: {}", topic));
    lines.push(format!("created_at: \"{}\"", created));
    lines.push(format!("last_updated: \"{}\"", updated));
    lines.push("status: evolving".to_string());
    lines.push(format!("paper_count: {}", paper_count));
    lines.push("---".to_string());
    lines.push(String::new());
    lines.push(format!("# {} 文献综述", topic));
    lines.push(String::new());
    lines.push("## 概述".to_string());
    lines.push(String::new());
    lines.push(format!("- **论文数量**: {}", paper_count));
    lines.push(format!("- **时间范围**: {}", date_range));
    lines.push(format!("- **最后更新**: {}", &updated[..10]));
    lines.push(String::new());
    lines.push("本综述随订阅论文自动更新，保持与研究前沿同步。".to_string());
    lines.push(String::new());
    lines.push("## 研究时间线".to_string());
    lines.push(String::new());

    // Timeline table
    if !sorted_papers.is_empty() {
        lines.push("| 日期 | 论文 |".to_string());
        lines.push("|------|------|".to_string());
        for p in sorted_papers.iter().take(20) {
            let date = p.get("published").and_then(|v| v.as_str()).unwrap_or("未知");
            let title = p.get("title").and_then(|v| v.as_str()).unwrap_or("无标题");
            let date_short = &date[..10.min(date.len())];
            let title_short = &title[..50.min(title.len())];
            lines.push(format!("| {} | {} |", date_short, title_short));
        }
    } else {
        lines.push("_暂无论文数据_".to_string());
    }

    lines.push(String::new());
    lines.push("## 方法分类".to_string());
    lines.push(String::new());

    let method_groups = group_by_methodology(papers);
    if !method_groups.is_empty() {
        for (method, group_papers) in &method_groups {
            lines.push(format!("### {} ({} 篇)", method, group_papers.len()));
            lines.push(String::new());
            for p in group_papers.iter().take(5) {
                let title = p.get("title").and_then(|v| v.as_str()).unwrap_or("无标题");
                let score = p.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let title_short = &title[..60.min(title.len())];
                lines.push(format!("- **{}** (score={:.2})", title_short, score));
            }
            if group_papers.len() > 5 {
                lines.push(format!("- _... 还有 {} 篇_", group_papers.len() - 5));
            }
            lines.push(String::new());
        }
    } else {
        lines.push("_暂无分类数据_".to_string());
        lines.push(String::new());
    }

    lines.push("## 代表论文 (Top 10)".to_string());
    lines.push(String::new());

    if !top_papers.is_empty() {
        for (i, p) in top_papers.iter().enumerate().take(10) {
            let title = p.get("title").and_then(|v| v.as_str()).unwrap_or("无标题");
            let score = p.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let arxiv_id = p.get("arxiv_id").and_then(|v| v.as_str()).unwrap_or("");
            lines.push(format!(
                "{}. [{}](https://arxiv.org/abs/{}) _[score: {:.2}]_",
                i + 1,
                title,
                arxiv_id,
                score
            ));
        }
    } else {
        lines.push("_暂无论文数据_".to_string());
    }

    lines.push(String::new());
    lines.push("## 开放问题".to_string());
    lines.push(String::new());

    let open_problems = extract_open_problems(papers);
    if !open_problems.is_empty() {
        for problem in open_problems.iter().take(5) {
            lines.push(format!("- {}", problem));
        }
    } else {
        lines.push("- 持续跟踪最新研究进展".to_string());
    }

    lines.push(String::new());
    lines.push("## 更新日志".to_string());
    lines.push(format!("- {}: 创建综述文档 ({} 篇论文)", &now[..10], paper_count));

    lines.join("\n") + "\n"
}

/// Incrementally update an existing literature review, preserving user annotations.
pub fn update_litreview(
    existing_content: &str,
    new_papers: &[PaperDict],
    all_papers: Option<&[PaperDict]>,
) -> String {
    let now = Utc::now().to_rfc3339();
    let lines: Vec<&str> = existing_content.lines().collect();
    let mut updated_lines: Vec<String> = Vec::new();

    // Find and update frontmatter
    let mut in_frontmatter = false;
    for (i, line) in lines.iter().enumerate() {
        if line.trim() == "---" {
            if !in_frontmatter {
                in_frontmatter = true;
                updated_lines.push(line.to_string());
            } else {
                updated_lines.push(line.to_string());
                let _ = i;
                updated_lines.push(format!("last_updated: \"{}\"", now));
                if let Some(all) = all_papers {
                    updated_lines.push(format!("paper_count: {}", all.len()));
                }
                break;
            }
        } else {
            updated_lines.push(line.to_string());
        }
    }

    // Update overview fields
    if let Some(all) = all_papers {
        let paper_count = all.len();
        let date_range = get_date_range(all);
        for (_i, line) in updated_lines.iter_mut().enumerate() {
            if line.starts_with("**论文数量**") {
                *line = format!("- **论文数量**: {}", paper_count);
            } else if line.starts_with("**时间范围**") {
                *line = format!("- **时间范围**: {}", date_range);
            } else if line.starts_with("**最后更新**") {
                *line = format!("- **最后更新**: {}", &now[..10]);
            }
        }
    }

    // Find changelog section
    let mut changelog_start = 0usize;
    for (i, line) in lines.iter().enumerate() {
        if line.contains("## 更新日志") {
            changelog_start = i;
            break;
        }
    }

    if changelog_start > 0 && !new_papers.is_empty() {
        let mut result: Vec<String> = updated_lines[..changelog_start].to_vec();
        result.push("## 更新日志".to_string());
        result.push(String::new());
        for p in new_papers.iter().take(5) {
            let title = p.get("title").and_then(|v| v.as_str()).unwrap_or("无标题");
            let arxiv_id = p.get("arxiv_id").and_then(|v| v.as_str()).unwrap_or("");
            let title_short = &title[..50.min(title.len())];
            result.push(format!(
                "- {}: 新增 [{}](https://arxiv.org/abs/{})",
                &now[..10], title_short, arxiv_id
            ));
        }
        result.push(String::new());
        for line in &lines[changelog_start + 1..] {
            let skip = new_papers.iter().take(5).any(|p| {
                let title = p.get("title").and_then(|v| v.as_str()).unwrap_or("");
                title[..50.min(title.len())].contains(&line[..line.len().min(50)])
                    && line.contains(&now[..10])
            });
            if !skip {
                result.push(line.to_string());
            }
        }
        return result.join("\n") + "\n";
    }

    updated_lines.join("\n") + "\n"
}

fn get_date_range(papers: &[PaperDict]) -> String {
    let mut dates: Vec<&str> = papers
        .iter()
        .filter_map(|p| p.get("published").and_then(|v| v.as_str()))
        .collect();
    if dates.is_empty() {
        return "未知".to_string();
    }
    dates.sort();
    format!("{} ~ {}", dates.first().unwrap(), dates.last().unwrap())
}

fn sorted_by_date(papers: &[PaperDict]) -> Vec<&PaperDict> {
    let mut papers: Vec<&PaperDict> = papers.iter().collect();
    papers.sort_by(|a, b| {
        let da = a.get("published").and_then(|v| v.as_str()).unwrap_or("");
        let db = b.get("published").and_then(|v| v.as_str()).unwrap_or("");
        db.cmp(da)
    });
    papers
}

fn sorted_by_score(papers: &[PaperDict]) -> Vec<&PaperDict> {
    let mut papers: Vec<&PaperDict> = papers.iter().collect();
    papers.sort_by(|a, b| {
        let sa = a.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let sb = b.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });
    papers
}

fn group_by_methodology(papers: &[PaperDict]) -> HashMap<String, Vec<&PaperDict>> {
    let method_keywords: HashMap<&str, Vec<&str>> = [
        ("Transformer", vec!["transformer", "attention", "self-attention", "bert", "gpt"]),
        ("CNN/卷积", vec!["convolution", "cnn", "convolutional", "resnet", "vgg"]),
        ("图神经网络", vec!["graph", "gnn", "gcn", "gat"]),
        ("强化学习", vec!["reinforcement", "rl", "policy", "q-learning", "ddpg"]),
        ("扩散模型", vec!["diffusion", "ddpm", "score-based", "gan"]),
        ("检索增强", vec!["retrieval", "rag", "retrieval-augmented", "knowledge retrieval"]),
        ("多模态", vec!["multimodal", "vision-language", "image-text", "vqa"]),
        ("大语言模型", vec!["llm", "large language", "foundation model", "gpt-", "claude", "gemini"]),
    ]
    .iter()
    .cloned()
    .collect();

    let mut groups: HashMap<String, Vec<&PaperDict>> = HashMap::new();
    for paper in papers {
        let text = [
            paper.get("title").and_then(|v| v.as_str()).unwrap_or(""),
            paper.get("abstract").and_then(|v| v.as_str()).unwrap_or(""),
        ]
        .join(" ")
        .to_lowercase();

        for (method, keywords) in &method_keywords {
            if keywords.iter().any(|kw| text.contains(*kw)) {
                groups.entry(method.to_string()).or_default().push(paper);
                break;
            }
        }
    }
    groups
}

fn extract_open_problems(papers: &[PaperDict]) -> Vec<String> {
    let signal_phrases = [
        "remain an open problem",
        "future work",
        "future research",
        "left for future",
        "beyond the scope",
        "limitation",
        "challenge",
        "opportunity",
        "potential future",
    ];

    let mut problems: Vec<String> = Vec::new();

    for paper in papers.iter().take(15) {
        let abstract_ = paper
            .get("abstract")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase();

        for phrase in &signal_phrases {
            if let Some(idx) = abstract_.find(phrase) {
                let start = idx.saturating_sub(50);
                let end = (idx + 80).min(abstract_.len());
                let snippet: String = abstract_[start..end].split_whitespace().collect();
                if snippet.len() > 20 {
                    let title = paper
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")[..40.min(
                            paper
                                .get("title")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .len(),
                        )]
                    .to_string();
                    problems.push(format!("_{}..._: {}...", title, snippet));
                    break;
                }
            }
        }
    }

    problems.truncate(5);
    problems
}
