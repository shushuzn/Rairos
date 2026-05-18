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

use anyhow::{Context, Result};
use rairos_core::Database;
use std::path::PathBuf;
use crate::handlers::*;

pub fn handle_repl(query: Option<String>) -> Result<()> {
    let db_path = PathBuf::from("rairos.db");
    if !db_path.exists() {
        return Err(anyhow::anyhow!("Database not found. Run 'rairos init' first."));
    }
    let db = Database::open(&db_path).context("Failed to open database")?;

    println!("=== Rairos REPL ===");
    println!("Type 'help' for commands, 'exit' to quit.\n");

    if let Some(q) = query {
        println!("Pre-loading papers matching: {}", q);
        match db.search_papers(&q, 10) {
            Ok(papers) if !papers.is_empty() => {
                println!("Found {} papers:\n", papers.len());
                for (i, p) in papers.iter().enumerate() {
                    let title = if p.title.len() > 60 {
                        format!("{}...", &p.title[..60])
                    } else {
                        p.title.clone()
                    };
                    let arxiv = p.arxiv_id.as_deref().unwrap_or("-");
                    let id_short = if p.id.len() > 8 { &p.id[..8] } else { p.id.as_str() };
                    println!("  {}. [{}] {} — {}", i + 1, id_short, title, arxiv);
                }
                println!();
            }
            _ => println!("No papers found for query: {}\n", q),
        }
    }

    loop {
        print!("rairos> ");
        std::io::Write::flush(&mut std::io::stdout()).ok();

        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() || input.trim().is_empty() {
            continue;
        }
        let input = input.trim();

        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        let cmd = parts[0].to_lowercase();
        let arg = parts.get(1).map(|s| s.trim()).unwrap_or("");

        match cmd.as_str() {
            "exit" | "quit" => {
                println!("Goodbye!");
                break;
            }
            "help" => {
                println!("\nCommands:");
                println!("  help                   Show this help");
                println!("  exit / quit            Exit REPL");
                println!("  search <query>         Search papers");
                println!("  show <id>              Show paper details");
                println!("  list [status]          List papers (pending/done/all)");
                println!("  stats                  Show DB statistics");
                println!("  gap <topic>            Detect research gaps");
                println!("  add <arxiv_id>         Import paper from arXiv");
                println!();
            }
            "search" if arg.is_empty() => {
                println!("Usage: search <query>\n");
            }
            "search" => {
                match db.search_papers(arg, 20) {
                    Ok(papers) if papers.is_empty() => {
                        println!("No papers found for: {}", arg);
                    }
                    Ok(papers) => {
                        println!("Found {} papers:\n", papers.len());
                        for (i, p) in papers.iter().enumerate() {
                            let title = if p.title.len() > 60 {
                                format!("{}...", &p.title[..60])
                            } else {
                                p.title.clone()
                            };
                            let arxiv = p.arxiv_id.as_deref().unwrap_or("-");
                            let id_short = if p.id.len() > 8 { &p.id[..8] } else { p.id.as_str() };
                            println!("  {}. [{}] {} — {}", i + 1, id_short, title, arxiv);
                        }
                        println!();
                    }
                    Err(e) => println!("Error: {}\n", e),
                }
            }
            "show" if arg.is_empty() => {
                println!("Usage: show <id>\n");
            }
            "show" => {
                if let Err(e) = handle_show(&db, arg, "table") {
                    println!("Error: {}\n", e);
                }
            }
            "list" => {
                let status = if arg.is_empty() { None } else { Some(arg.to_string()) };
                if let Err(e) = handle_list(&db, status, None, &[], 20, 0, "published", "desc", "table") {
                    println!("Error: {}\n", e);
                }
            }
            "stats" => {
                if let Err(e) = handle_stats(&db, false, "table") {
                    println!("Error: {}\n", e);
                }
            }
            "gap" if arg.is_empty() => {
                println!("Usage: gap <topic>\n");
            }
            "gap" => {
                if let Err(e) = handle_gap(&db, arg, 5, "table", None) {
                    println!("Error: {}\n", e);
                }
            }
            "add" if arg.is_empty() => {
                println!("Usage: add <arxiv_id>\n");
            }
            "add" => {
                if let Err(e) = handle_add(&db, arg) {
                    println!("Error: {}\n", e);
                }
            }
            _ => {
                println!("Unknown command: {}. Type 'help' for available commands.\n", cmd);
            }
        }
    }
    Ok(())
}
