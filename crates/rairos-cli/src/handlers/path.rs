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
use rairos_core::Database;
use crate::handlers::*;

pub fn handle_path(
    db: &rairos_core::Database,
    topic: Option<&str>,
    level: &str,
    max: usize,
    min_year: Option<i32>,
    max_year: Option<i32>,
    mermaid: bool,
    interactive: bool,
) -> Result<()> {
    let level_enum = rairos_pathfinder::ReadingLevel::from_string(level)
        .unwrap_or(rairos_pathfinder::ReadingLevel::Intermediate);

    // Interactive mode
    if interactive || topic.is_none() {
        return handle_path_interactive(db, level_enum, max, min_year, max_year, mermaid);
    }

    let topic = topic.unwrap();
    println!("📊 Planning reading path for: {topic}");
    println!("   Level: {level} | Max papers: {max}");

    // Get KG if available
    let kg = try_get_kg();

    let planner = rairos_pathfinder::ResearchPathPlanner::new(kg.as_ref(), Some(db));
    let path = planner.plan_path(topic, level_enum, max, min_year, max_year);

    if mermaid {
        println!("{}", rairos_pathfinder::render_mermaid(&path));
    } else {
        println!();
        println!("{}", rairos_pathfinder::render_path(&path));
    }

    Ok(())
}

pub fn handle_path_interactive(
    db: &rairos_core::Database,
    mut level: rairos_pathfinder::ReadingLevel,
    mut max: usize,
    min_year: Option<i32>,
    max_year: Option<i32>,
    mut mermaid: bool,
) -> Result<()> {
    println!("📚 Research Path Planner");
    println!("  输入 topic 开始规划阅读路径");
    println!("  输入 level [intro|intermediate|advanced] 设置难度");
    println!("  输入 max [N] 设置最大论文数");
    println!("  输入 mermaid 显示图");
    println!("  输入 q/quit 退出");
    println!();

    loop {
        let user_input = match std::io::stdin().lines().next() {
            Some(Ok(line)) => line.trim().to_string(),
            _ => break,
        };

        if user_input.is_empty() {
            continue;
        }

        let cmd = user_input.to_lowercase();

        match cmd.as_str() {
            "q" | "quit" | "exit" => break,
            "mermaid" => {
                mermaid = !mermaid;
                let status = if mermaid { "启用" } else { "禁用" };
                println!("  ✓ Mermaid 输出已{status}");
                continue;
            }
            _ => {}
        }

        if cmd.starts_with("level ") {
            let level_str = cmd.split_once(' ').map(|(_, rest)| rest).unwrap_or("");
            if let Some(l) = rairos_pathfinder::ReadingLevel::from_string(level_str) {
                level = l;
                println!("  ✓ 难度设置为: {level_str}");
            } else {
                println!("  ✗ 未知难度，可选: intro, intermediate, advanced");
            }
            continue;
        }

        if cmd.starts_with("max ") {
            if let Some(rest) = cmd.split_once(' ').map(|(_, r)| r) {
                if let Ok(n) = rest.parse::<usize>() {
                    max = n;
                    println!("  ✓ 最大论文数设置为: {max}");
                } else {
                    println!("  ✗ 无效数字");
                }
            }
            continue;
        }

        // Treat as topic
        let topic = &user_input;
        println!();
        println!("📊 Planning: {topic}");

        let kg = try_get_kg();
        let planner = rairos_pathfinder::ResearchPathPlanner::new(kg.as_ref(), Some(db));
        let path = planner.plan_path(topic, level, max, min_year, max_year);

        if mermaid {
            println!("{}", rairos_pathfinder::render_mermaid(&path));
        } else {
            println!();
            println!("{}", rairos_pathfinder::render_path(&path));
        }
        println!();
    }

    Ok(())
}
