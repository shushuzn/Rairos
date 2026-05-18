#![allow(
    clippy::too_many_arguments,
    clippy::needless_borrow,
    clippy::print_literal,
    clippy::unwrap_or_default,
    clippy::unnecessary_sort_by,
    clippy::format_in_format_args,
    clippy::map_identity,
    clippy::unused_enumerate_index,
    clippy::needless_borrows_for_generic_args,
    clippy::unnecessary_to_owned,
    clippy::manual_range_contains
)]

use anyhow::Result;
use crate::QuestionAction;

pub fn handle_question(action: &QuestionAction) -> Result<()> {
    use rairos_questions::{QuestionSource, QuestionStatus};

    let mut tracker = rairos_questions::QuestionTracker::new()?;

    match action {
        QuestionAction::List {
            status,
            topic,
            source,
            verbose,
        } => {
            let status_enum = status.as_ref().and_then(|s| match s.as_str() {
                "open" => Some(QuestionStatus::Open),
                "in_progress" => Some(QuestionStatus::InProgress),
                "resolved" => Some(QuestionStatus::Resolved),
                "wontfix" => Some(QuestionStatus::Wontfix),
                _ => None,
            });
            let source_enum = source.as_ref().and_then(|s| match s.as_str() {
                "manual" => Some(QuestionSource::Manual),
                "gap_detection" => Some(QuestionSource::GapDetection),
                "hypothesis" => Some(QuestionSource::Hypothesis),
                "literature_review" => Some(QuestionSource::LiteratureReview),
                _ => None,
            });
            let questions = tracker.list(topic.as_deref(), status_enum.as_ref(), source_enum.as_ref());
            if questions.is_empty() {
                println!("没有找到研究问题。");
            } else {
                for (i, q) in questions.iter().enumerate() {
                    let icon = match q.status {
                        QuestionStatus::Open => "○",
                        QuestionStatus::InProgress => "◐",
                        QuestionStatus::Resolved => "●",
                        QuestionStatus::Wontfix => "✗",
                    };
                    println!("{}. [{}] {}", i + 1, icon, q.question);
                    println!(
                        "   ID: {} | 来源: {} | 优先级: {}/10",
                        q.id,
                        q.source.as_str(),
                        q.priority
                    );
                    if !q.topic.is_empty() {
                        println!("   主题: {}", q.topic);
                    }
                    if !q.related_papers.is_empty() {
                        println!("   关联论文: {} 篇", q.related_papers.len());
                    }
                    if *verbose && !q.notes.is_empty() {
                        println!("   备注: {}", q.notes);
                    }
                    println!();
                }
            }
        }

        QuestionAction::Add {
            question,
            topic,
            priority,
            notes,
        } => {
            let q = tracker.add(
                question.clone(),
                QuestionSource::Manual,
                topic.clone().unwrap_or_default(),
                *priority,
                notes.clone().unwrap_or_default(),
            );
            tracker.save()?;
            println!("✓ 添加问题 [{}]: {}", q.id, q.question);
            println!("  来源: {} | 优先级: {}/10", q.source.as_str(), q.priority);
        }

        QuestionAction::Get { id } => {
            match tracker.get(id) {
                Some(q) => {
                    let icon = match q.status {
                        QuestionStatus::Open => "○",
                        QuestionStatus::InProgress => "◐",
                        QuestionStatus::Resolved => "●",
                        QuestionStatus::Wontfix => "✗",
                    };
                    println!("问题: {}", q.question);
                    println!("ID: {}", q.id);
                    println!("状态: {} {}", icon, q.status.as_str());
                    println!("来源: {}", q.source.as_str());
                    println!("优先级: {}/10", q.priority);
                    if !q.topic.is_empty() {
                        println!("主题: {}", q.topic);
                    }
                    println!("创建: {}", q.created_at);
                    println!("更新: {}", q.updated_at);
                    if !q.related_papers.is_empty() {
                        println!("关联论文: {}", q.related_papers.join(", "));
                    }
                    if !q.notes.is_empty() {
                        println!("备注: {}", q.notes);
                    }
                }
                None => {
                    eprintln!("❌ 问题 [{}] 不存在", id);
                }
            }
        }

        QuestionAction::Update {
            id,
            status,
            notes,
            priority,
        } => {
            let status_enum = status.as_ref().and_then(|s| match s.as_str() {
                "open" => Some(QuestionStatus::Open),
                "in_progress" => Some(QuestionStatus::InProgress),
                "resolved" => Some(QuestionStatus::Resolved),
                "wontfix" => Some(QuestionStatus::Wontfix),
                _ => None,
            });
            match tracker.update(id, status_enum, notes.clone(), *priority) {
                Ok(()) => {
                    tracker.save()?;
                    if let Some(q) = tracker.get(id) {
                        println!("✓ 更新问题 [{}]: {}", q.id, q.question);
                    }
                }
                Err(e) => {
                    eprintln!("❌ {}", e);
                }
            }
        }

        QuestionAction::Link { id, paper_id } => {
            match tracker.link_paper(id, paper_id) {
                Ok(()) => {
                    tracker.save()?;
                    println!("✓ 关联论文 [{}] → 问题 [{}]", paper_id, id);
                }
                Err(e) => {
                    eprintln!("❌ {}", e);
                }
            }
        }

        QuestionAction::Unlink { id, paper_id } => {
            match tracker.unlink_paper(id, paper_id) {
                Ok(()) => {
                    tracker.save()?;
                    println!("✓ 取消关联 [{}] ← 问题 [{}]", paper_id, id);
                }
                Err(e) => {
                    eprintln!("❌ {}", e);
                }
            }
        }

        QuestionAction::Delete { id } => {
            match tracker.delete(id) {
                Ok(()) => {
                    tracker.save()?;
                    println!("✓ 删除问题 [{}]", id);
                }
                Err(e) => {
                    eprintln!("❌ {}", e);
                }
            }
        }

        QuestionAction::Sync { topic, priority } => {
            // Sync from gap detection (sample gaps matching Python behaviour)
            let gaps = vec![
                "长文档场景下的检索效率问题".to_string(),
                "检索结果与生成质量的一致性保证".to_string(),
                "跨领域知识迁移的有效性评估".to_string(),
            ];
            let new_questions = tracker.sync_from_gaps(
                &gaps,
                topic.as_deref().unwrap_or("general"),
                *priority,
            );
            tracker.save()?;
            if new_questions.is_empty() {
                println!("没有新的问题需要同步");
            } else {
                println!("✓ 同步了 {} 个新问题:", new_questions.len());
                for q in &new_questions {
                    println!("  - [{}] {}", q.id, q.question);
                }
            }
        }

        QuestionAction::Stats => {
            let stats = tracker.stats();
            println!("📊 研究问题统计");
            let total = stats.open + stats.in_progress + stats.resolved + stats.wontfix;
            println!("总计: {} 个问题", total);
            println!();
            println!("按状态:");
            println!("  open: {}", stats.open);
            println!("  in_progress: {}", stats.in_progress);
            println!("  resolved: {}", stats.resolved);
            println!("  wontfix: {}", stats.wontfix);
            println!();
            println!("按来源:");
            println!("  manual: {}", stats.manual);
            println!("  gap_detection: {}", stats.gap_detection);
            println!("  hypothesis: {}", stats.hypothesis);
            println!("  literature_review: {}", stats.literature_review);
        }
    }

    Ok(())
}
