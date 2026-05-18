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
use crate::NarrativeAction;
use crate::handlers::*;

pub fn handle_narrative(action: &NarrativeAction) -> Result<()> {
    use rairos_narratives::{compute_phase, compute_readiness, render_dashboard, render_thread};
    use rairos_narratives::{NarrativePhase, ResearchThread};

    let mut tracker = rairos_narratives::ResearchThreadTracker::new()?;

    match action {
        NarrativeAction::List => {
            let threads = tracker.list_threads();
            if threads.is_empty() {
                println!("没有找到研究线索。");
            } else {
                for t in &threads {
                    let icon = match t.phase {
                        NarrativePhase::Exploration => "🔍",
                        NarrativePhase::Hypothesis => "💡",
                        NarrativePhase::Validation => "🔬",
                        NarrativePhase::Publication => "📄",
                    };
                    let created = if t.created_at.len() >= 10 {
                        &t.created_at[..10]
                    } else {
                        &t.created_at
                    };
                    println!(
                        "{} [{}] {} — {} (创造: {})",
                        icon, t.id, t.topic, t.phase.as_str(), created
                    );
                }
            }
        }

        NarrativeAction::Show { id } => match tracker.get_thread(id) {
            Some(t) => {
                println!("{}", render_thread(t));
            }
            None => {
                eprintln!("❌ 线索 [{}] 不存在", id);
            }
        },

        NarrativeAction::Track { topic } => {
            let existing = tracker.get_by_topic(topic);
            let mut thread = if let Some(existing) = existing {
                existing.clone()
            } else {
                // Try to aggregate from tracker files
                match rairos_narratives::aggregate_by_topic(topic) {
                    Ok(aggregated) => aggregated,
                    Err(_) => ResearchThread::new(topic),
                }
            };

            // Recompute phase and scores
            let new_phase = compute_phase(&thread);
            if new_phase != thread.phase {
                thread.phase_updated_at = chrono::Utc::now()
                    .format("%Y-%m-%dT%H:%M:%S")
                    .to_string();
            }
            thread.phase = new_phase;
            let (c, e, n) = compute_readiness(&thread);
            thread.contribution_score = c;
            thread.experiment_score = e;
            thread.narrative_score = n;

            tracker.upsert(&mut thread);
            tracker.save()?;
            println!("✓ 线索已更新: [{}] {}", thread.id, thread.topic);
            println!("  阶段: {} | 贡献: {:.0}% | 实验: {:.0}% | 叙述: {:.0}%",
                thread.phase.as_str(),
                thread.contribution_score * 100.0,
                thread.experiment_score * 100.0,
                thread.narrative_score * 100.0,
            );
        }

        NarrativeAction::Update { id, topic, notes } => {
            let mut thread = match tracker.get_thread(id) {
                Some(t) => t.clone(),
                None => {
                    eprintln!("❌ 线索 [{}] 不存在", id);
                    return Ok(());
                }
            };
            if let Some(t) = topic {
                thread.topic = t.clone();
            }
            if let Some(n) = notes {
                thread.notes = n.clone();
            }
            tracker.upsert(&mut thread);
            tracker.save()?;
            println!("✓ 已更新线索 [{}]", id);
        }

        NarrativeAction::Note { id, text } => {
            let mut thread = match tracker.get_thread(id) {
                Some(t) => t.clone(),
                None => {
                    eprintln!("❌ 线索 [{}] 不存在", id);
                    return Ok(());
                }
            };
            if thread.notes.is_empty() {
                thread.notes = text.clone();
            } else {
                thread.notes = format!("{}\n{}", thread.notes, text);
            }
            tracker.upsert(&mut thread);
            tracker.save()?;
            println!("✓ 笔记已添加到线索 [{}]", id);
        }

        NarrativeAction::Dashboard => {
            let threads = tracker.list_threads();
            let refs: Vec<&rairos_narratives::ResearchThread> = threads.to_vec();
            println!("{}", render_dashboard(&refs));
        }
    }

    Ok(())
}
